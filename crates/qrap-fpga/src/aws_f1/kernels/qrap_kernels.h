#ifndef QRAP_KERNELS_H
#define QRAP_KERNELS_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* 256-bit field element: 4×64 = 256 bit (little-endian limb0..limb3) */
typedef struct { uint64_t d[4]; } qrap_fe256_t;
/* Affine point (x,y) */
typedef struct { qrap_fe256_t x, y; } qrap_affine_t;
/* Projective point (x,y,z) */
typedef struct { qrap_fe256_t x, y, z; } qrap_proj_t;

/* NTT: in-place, twiddles precomputed, log2(N) = log_n */
void qrap_ntt(qrap_fe256_t* inout, const qrap_fe256_t* twiddles, uint32_t log_n);

/* MSM: Pippenger bucket method
 * points  — affine array [n]
 * scalars — 256-bit scalars [n]
 * result  — projective output (single point)
 * n       — number of points
 */
void qrap_msm(const qrap_affine_t* points, const qrap_fe256_t* scalars,
              qrap_proj_t* result, uint32_t n);

/* Field arithmetic — Montgomery domain */
void qrap_fe_mul(const qrap_fe256_t* a, const qrap_fe256_t* b,
                 qrap_fe256_t* out, const qrap_fe256_t* mod);
void qrap_fe_inv(const qrap_fe256_t* a, qrap_fe256_t* out,
                 const qrap_fe256_t* mod);
void qrap_fe_add(const qrap_fe256_t* a, const qrap_fe256_t* b,
                 qrap_fe256_t* out, const qrap_fe256_t* mod);
void qrap_fe_sub(const qrap_fe256_t* a, const qrap_fe256_t* b,
                 qrap_fe256_t* out, const qrap_fe256_t* mod);

#ifdef __cplusplus
}
#endif

#endif
