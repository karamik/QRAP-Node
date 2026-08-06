/* Iterative in-place NTT (Cooley-Tukey, decimation-in-time)
 * Assumes input already in bit-reversed order
 * twiddles[stage][i] precomputed as Montgomery form elements */

typedef struct { ulong4 v; } fe_t;

/* 256-bit add with modular reduction (using fe_reduce from field.cl) */
static inline void fe_mod_add(const ulong4 a, const ulong4 b, ulong4* out);
static inline void fe_mod_sub(const ulong4 a, const ulong4 b, ulong4* out);
static inline void fe_mont_mul(const ulong4 a, const ulong4 b, ulong4* out);

__kernel void ntt_radix2(__global fe_t* data,
                         __global const fe_t* twiddles,
                         const uint log_n,
                         const uint stage) {
    uint n = 1u << log_n;
    uint stride = 1u << stage;
    uint half_stride = stride >> 1;
    uint gid = get_global_id(0);

    /* Each work-item processes one butterfly pair */
    uint block = gid / half_stride;
    uint offset = gid % half_stride;
    uint i = block * stride + offset;
    uint j = i + half_stride;

    if (j >= n) return;

    fe_t u = data[i];
    fe_t v = data[j];

    /* v = v * twiddle */
    fe_mont_mul(v.v, twiddles[gid].v, &v.v);

    /* Butterfly */
    fe_mod_add(u.v, v.v, &data[i].v);
    fe_mod_sub(u.v, v.v, &data[j].v);
}
