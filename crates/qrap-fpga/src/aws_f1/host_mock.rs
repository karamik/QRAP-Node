//! Mock F1 host — pure Rust, no OpenCL, for Termux testing
use super::{Fe256, ProjPoint, AffinePoint};

pub struct MockHost;

impl MockHost {
    pub fn new() -> Self { Self }
}

impl Default for MockHost {
    fn default() -> Self { Self::new() }
}

impl MockHost {
    pub fn init(&self, _xclbin: &str) -> Result<(), i32> { Ok(()) }

    pub fn fe_mul_batch(&self, a: &[Fe256], b: &[Fe256]) -> Vec<Fe256> {
        a.iter().zip(b.iter()).map(|(x, y)| {
            Fe256::mont_mul(x, y)
        }).collect()
    }

    pub fn ntt(&self, _data: &mut [Fe256], _twiddles: &[Fe256], _log_n: u32) {
        // Mock: no-op
    }

    pub fn msm(&self, points: &[AffinePoint], scalars: &[Fe256]) -> ProjPoint {
        let mut res = ProjPoint::default();
        for (p, s) in points.iter().zip(scalars.iter()) {
            if !s.is_zero() {
                super::field_cpu::ec_add_mixed(&mut res, p);
            }
        }
        res
    }
}
