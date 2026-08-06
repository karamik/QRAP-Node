//! AWS F1 VU9P FPGA acceleration for PLONK proof generation
#![cfg(feature = "aws-f1")]

pub mod field_cpu;
pub mod host_mock;
pub mod prover;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fe256 {
    pub d: [u64; 4],
}

impl Fe256 {
    pub fn is_zero(&self) -> bool {
        self.d.iter().all(|&x| x == 0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AffinePoint {
    pub x: Fe256,
    pub y: Fe256,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjPoint {
    pub x: Fe256,
    pub y: Fe256,
    pub z: Fe256,
}

// Real C++ host only on x86_64 with OpenCL
#[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
mod real_host {
    use super::*;
    use std::os::raw::{c_void, c_char, c_int};

    extern "C" {
        pub fn qrap_f1_create() -> *mut c_void;
        pub fn qrap_f1_destroy(handle: *mut c_void);
        pub fn qrap_f1_init(handle: *mut c_void, xclbin: *const c_char) -> c_int;
        pub fn qrap_f1_fe_mul(handle: *mut c_void, a: *const Fe256, b: *const Fe256,
                              c: *mut Fe256, n: u32) -> c_int;
        pub fn qrap_f1_ntt(handle: *mut c_void, inout: *mut Fe256,
                           twiddles: *const Fe256, log_n: u32) -> c_int;
    }
}

pub struct F1Accelerator {
    #[cfg(any(target_arch = "aarch64", target_os = "android"))]
    mock: host_mock::MockHost,
    #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
    handle: *mut std::os::raw::c_void,
}

impl Default for F1Accelerator {
    fn default() -> Self { Self::new() }
}

impl F1Accelerator {
    pub fn new() -> Self {
        #[cfg(any(target_arch = "aarch64", target_os = "android"))]
        { Self { mock: host_mock::MockHost::new() } }
        #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
        { Self { handle: unsafe { real_host::qrap_f1_create() } } }
    }

    pub fn init(&self, xclbin_path: &str) -> Result<(), i32> {
        #[cfg(any(target_arch = "aarch64", target_os = "android"))]
        { self.mock.init(xclbin_path) }
        #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
        {
            let c_path = std::ffi::CString::new(xclbin_path).unwrap();
            let rc = unsafe { real_host::qrap_f1_init(self.handle, c_path.as_ptr()) };
            if rc == 0 { Ok(()) } else { Err(rc) }
        }
    }

    pub fn fe_mul_batch(&self, a: &[Fe256], b: &[Fe256]) -> Vec<Fe256> {
        #[cfg(any(target_arch = "aarch64", target_os = "android"))]
        { self.mock.fe_mul_batch(a, b) }
        #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
        {
            let n = a.len() as u32;
            let mut c = vec![Fe256::default(); a.len()];
            unsafe {
                real_host::qrap_f1_fe_mul(self.handle, a.as_ptr(), b.as_ptr(), c.as_mut_ptr(), n);
            }
            c
        }
    }

    pub fn ntt(&self, data: &mut [Fe256], twiddles: &[Fe256], log_n: u32) {
        #[cfg(any(target_arch = "aarch64", target_os = "android"))]
        { self.mock.ntt(data, twiddles, log_n) }
        #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
        unsafe {
            real_host::qrap_f1_ntt(self.handle, data.as_mut_ptr(), twiddles.as_ptr(), log_n);
        }
    }
}

impl Drop for F1Accelerator {
    fn drop(&mut self) {
        #[cfg(all(not(target_arch = "aarch64"), not(target_os = "android")))]
        unsafe { real_host::qrap_f1_destroy(self.handle); }
    }
}

#[cfg(test)]
mod tests;
