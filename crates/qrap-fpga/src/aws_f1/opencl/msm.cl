/* Pippenger bucket method — windowed decomposition
 * Window size = 16 bits => 65536 buckets per window
 * Scalars split into 16 windows (256/16 = 16) */

typedef struct { ulong4 v; } fe_t;
typedef struct { fe_t x, y; } affine_t;
typedef struct { fe_t x, y, z; } proj_t;

#define WINDOW_BITS 16
#define NUM_BUCKETS (1 << WINDOW_BITS)   /* 65536 */
#define NUM_WINDOWS 16

/* EC point addition (projective + affine -> projective)
 * Using Jacobian coordinates for BN254
 * Simplified: full formulas require field mul/add/sub */
static inline void ec_add_mixed(proj_t* R, const affine_t* P) {
    /* Placeholder: real impl needs:
     * U1 = X1*Z2^2, U2 = X2*Z1^2
     * S1 = Y1*Z2^3, S2 = Y2*Z1^3
     * H = U2-U1, R = S2-S1
     * X3 = R^2 - H^3 - 2*U1*H^2
     * Y3 = R*(U1*H^2 - X3) - S1*H^3
     * Z3 = H*Z1*Z2
     */
}

/* Accumulate points into buckets for one window */
__kernel void msm_bucket_accum(__global const affine_t* points,
                               __global const fe_t* scalars,
                               __global proj_t* buckets,
                               const uint n,
                               const uint window_idx) {
    uint gid = get_global_id(0);
    if (gid >= n) return;

    /* Extract 16-bit window from 256-bit scalar */
    /* scalar is 4 ulongs: limb0 lowest */
    uint limb_idx = (window_idx * WINDOW_BITS) / 64;
    uint bit_offset = (window_idx * WINDOW_BITS) % 64;
    ulong scalar limb = scalars[gid].v.s[limb_idx];
    uint bucket_idx;

    if (bit_offset + WINDOW_BITS <= 64) {
        bucket_idx = (uint)((scalar_limb >> bit_offset) & 0xFFFFUL);
    } else {
        uint low_bits = 64 - bit_offset;
        ulong next_limb = (limb_idx < 3) ? scalars[gid].v.s[limb_idx+1] : 0UL;
        bucket_idx = (uint)(
            ((scalar_limb >> bit_offset) & ((1UL << low_bits) - 1)) |
            ((next_limb & ((1UL << (WINDOW_BITS - low_bits)) - 1)) << low_bits)
        );
    }

    if (bucket_idx == 0) return;

    /* Atomically add point to bucket[bucket_idx]
     * On FPGA we use private accumulation then reduction */
    ec_add_mixed(&buckets[bucket_idx], &points[gid]);
}

/* Reduce buckets: sum_{i=1}^{65535} i * B_i via double-and-add chain */
__kernel void msm_bucket_reduce(__global const proj_t* buckets,
                                __global proj_t* window_sums,
                                const uint num_windows) {
    uint wid = get_global_id(0);
    if (wid >= num_windows) return;

    /* Placeholder: accumulate buckets in reverse order
     * total = 0
     * for i = 65535 .. 1:
     *     total += bucket[i]
     *     window_sum += total
     * Final: window_sums[wid] = window_sum
     */
}
