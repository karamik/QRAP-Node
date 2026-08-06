#pragma OPENCL EXTENSION cl_khr_int64_extended_atomics : enable

typedef struct { ulong4 v; } fe_t;

/* BN254 prime p = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
 * Stored as little-endian: limb0 lowest */
constant ulong4 P = (ulong4)(
    0x3c208c16d87cfd47UL,
    0x97816a916871ca8dUL,
    0xb85045b68181585dUL,
    0x30644e72e131a029UL
);

/* p' = -p^{-1} mod 2^64 (Montgomery constant) */
constant ulong NP = 0x87d20782e4866389UL;

/* 128-bit add: returns carry (0 or 1) */
static inline uint add128(ulong a, ulong b, ulong* lo) {
    *lo = a + b;
    return (*lo < a) ? 1 : 0;
}

/* 256-bit add: c = a + b; return carry */
static inline uint fe_add_c(const ulong4 a, const ulong4 b, ulong4* c) {
    uint carry = 0;
    ulong t;
    carry = add128(a.s0, b.s0, &t); c->s0 = t;
    carry = add128(a.s1, b.s1 + carry, &t); c->s1 = t;
    carry = add128(a.s2, b.s2 + carry, &t); c->s2 = t;
    carry = add128(a.s3, b.s3 + carry, &t); c->s3 = t;
    return carry;
}

/* 256-bit sub: c = a - b; return borrow */
static inline uint fe_sub_c(const ulong4 a, const ulong4 b, ulong4* c) {
    uint borrow = 0;
    ulong t;
    borrow = (a.s0 < b.s0) ? 1 : 0; c->s0 = a.s0 - b.s0;
    t = b.s1 + borrow; borrow = (a.s1 < t) ? 1 : 0; c->s1 = a.s1 - t;
    t = b.s2 + borrow; borrow = (a.s2 < t) ? 1 : 0; c->s2 = a.s2 - t;
    t = b.s3 + borrow; borrow = (a.s3 < t) ? 1 : 0; c->s3 = a.s3 - t;
    return borrow;
}

/* Conditional subtract: if a >= p then a -= p */
static inline void fe_reduce(ulong4* a) {
    ulong4 t;
    uint borrow = fe_sub_c(*a, P, &t);
    if (!borrow) *a = t;
}

/* Montgomery multiplication: CIOS method
 * out = a * b * R^{-1} mod p   (inputs/outputs in Montgomery form) */
static inline void fe_mont_mul(const ulong4 a, const ulong4 b, ulong4* out) {
    ulong t[8] = {0};
    ulong carry, hi, lo;
    uint c;

    /* 1. Schoolbook multiply a * b -> t[0..7] */
    for (int i = 0; i < 4; i++) {
        carry = 0;
        for (int j = 0; j < 4; j++) {
            hi = mul_hi(a.s[i], b.s[j]);
            lo = a.s[i] * b.s[j];
            c = add128(t[i+j], lo, &t[i+j]);
            c += add128(t[i+j], carry, &t[i+j]);
            carry = hi + c;
        }
        t[i+4] += carry;
    }

    /* 2. Montgomery reduction */
    for (int i = 0; i < 4; i++) {
        ulong m = t[i] * NP;  /* m = t[i] * p' mod 2^64 */
        carry = 0;
        for (int j = 0; j < 4; j++) {
            hi = mul_hi(m, P.s[j]);
            lo = m * P.s[j];
            c = add128(t[i+j], lo, &t[i+j]);
            c += add128(t[i+j], carry, &t[i+j]);
            carry = hi + c;
        }
        c = add128(t[i+4], carry, &t[i+4]);
        if (i+5 < 8) t[i+5] += c;
    }

    /* 3. Result in t[4..7], conditionally subtract p */
    ulong4 res = (ulong4)(t[4], t[5], t[6], t[7]);
    fe_reduce(&res);
    *out = res;
}

/* OpenCL kernel: batch Montgomery multiplication */
__kernel void fe_mul_batch(__global const fe_t* A,
                           __global const fe_t* B,
                           __global fe_t* C,
                           const uint n) {
    uint gid = get_global_id(0);
    if (gid >= n) return;
    fe_mont_mul(A[gid].v, B[gid].v, &C[gid].v);
}

/* OpenCL kernel: batch field addition */
__kernel void fe_add_batch(__global const fe_t* A,
                           __global const fe_t* B,
                           __global fe_t* C,
                           const uint n) {
    uint gid = get_global_id(0);
    if (gid >= n) return;
    ulong4 r;
    fe_add_c(A[gid].v, B[gid].v, &r);
    fe_reduce(&r);
    C[gid].v = r;
}
