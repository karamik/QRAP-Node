//! AWS F1 Xilinx VU9P — XRT Runtime FFI Layer
//!
//! Requires:
//! - Xilinx XRT installed (xrt.h, libxrt_core.so)
//! - AWS FPGA Developer Kit
//! - AFI (Amazon FPGA Image) loaded
//!
//! Build: `cargo build -p qrap-fpga --features aws-f1`

pub mod prover;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};

// XRT Device handle opaque type
#[repr(C)]
pub struct XrtDevice {
    _private: [u8; 0],
}

#[repr(C)]
pub struct XrtBuffer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct XrtKernel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct XrtRun {
    _private: [u8; 0],
}

#[link(name = "xrt_core")]
#[link(name = "xrt_coreutil")]
extern "C" {
    pub fn xrtDeviceOpen(index: c_uint) -> *mut XrtDevice;
    pub fn xrtDeviceClose(dev: *mut XrtDevice) -> c_int;
    pub fn xrtDeviceLoadXclbin(dev: *mut XrtDevice, filename: *const c_char) -> c_int;
    pub fn xrtPLKernelOpen(
        dev: *mut XrtDevice,
        uuid: *const c_void,
        name: *const c_char,
    ) -> *mut XrtKernel;
    pub fn xrtKernelClose(krnl: *mut XrtKernel) -> c_int;
    pub fn xrtBOAlloc(
        dev: *mut XrtDevice,
        size: usize,
        flags: c_uint,
        grp: c_uint,
    ) -> *mut XrtBuffer;
    pub fn xrtBOFree(buf: *mut XrtBuffer) -> c_int;
    pub fn xrtBOSync(buf: *mut XrtBuffer, dir: c_int, size: usize, offset: usize) -> c_int;
    pub fn xrtBOMap(buf: *mut XrtBuffer) -> *mut c_void;
    pub fn xrtBOSize(buf: *mut XrtBuffer) -> usize;
    pub fn xrtRunOpen(krnl: *mut XrtKernel) -> *mut XrtRun;
    pub fn xrtRunClose(run: *mut XrtRun) -> c_int;
    pub fn xrtRunSetArg(run: *mut XrtRun, index: c_int, arg: *const c_void, bytes: usize) -> c_int;
    pub fn xrtRunStart(run: *mut XrtRun) -> c_int;
    pub fn xrtRunWait(run: *mut XrtRun, timeout_ms: c_int) -> c_int;
    pub fn xrtErrorCodeGetString(err: c_ulong, buf: *mut c_char, len: usize) -> c_int;
}

pub const XCL_BO_SYNC_BO_TO_DEVICE: c_int = 1;
pub const XCL_BO_SYNC_BO_FROM_DEVICE: c_int = 2;
pub const XRT_BO_FLAGS_NONE: c_uint = 0;
pub const XRT_BO_FLAGS_CACHEABLE: c_uint = 1 << 0;

pub struct XrtDeviceHandle {
    dev: *mut XrtDevice,
}

impl XrtDeviceHandle {
    pub fn open(index: u32) -> Option<Self> {
        let dev = unsafe { xrtDeviceOpen(index) };
        if dev.is_null() {
            None
        } else {
            Some(Self { dev })
        }
    }
    pub fn load_xclbin(&self, path: &str) -> Result<(), i32> {
        let c_path = CString::new(path).map_err(|_| -1)?;
        let rc = unsafe { xrtDeviceLoadXclbin(self.dev, c_path.as_ptr()) };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
    pub fn as_ptr(&self) -> *mut XrtDevice {
        self.dev
    }
}

impl Drop for XrtDeviceHandle {
    fn drop(&mut self) {
        unsafe {
            xrtDeviceClose(self.dev);
        }
    }
}

pub struct XrtKernelHandle {
    krnl: *mut XrtKernel,
}

impl XrtKernelHandle {
    pub fn open(dev: &XrtDeviceHandle, name: &str) -> Option<Self> {
        let c_name = CString::new(name).ok()?;
        let krnl = unsafe { xrtPLKernelOpen(dev.as_ptr(), std::ptr::null(), c_name.as_ptr()) };
        if krnl.is_null() {
            None
        } else {
            Some(Self { krnl })
        }
    }
    pub fn as_ptr(&self) -> *mut XrtKernel {
        self.krnl
    }
}

impl Drop for XrtKernelHandle {
    fn drop(&mut self) {
        unsafe {
            xrtKernelClose(self.krnl);
        }
    }
}

pub struct XrtBufferHandle {
    buf: *mut XrtBuffer,
    mapped: *mut u8,
    size: usize,
}

impl XrtBufferHandle {
    pub fn alloc(dev: &XrtDeviceHandle, size: usize) -> Option<Self> {
        let buf = unsafe { xrtBOAlloc(dev.as_ptr(), size, XRT_BO_FLAGS_NONE, 0) };
        if buf.is_null() {
            return None;
        }
        let mapped = unsafe { xrtBOMap(buf) as *mut u8 };
        if mapped.is_null() {
            unsafe {
                xrtBOFree(buf);
            }
            return None;
        }
        Some(Self { buf, mapped, size })
    }
    pub fn sync_to_device(&self) -> Result<(), i32> {
        let rc = unsafe { xrtBOSync(self.buf, XCL_BO_SYNC_BO_TO_DEVICE, self.size, 0) };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
    pub fn sync_from_device(&self) -> Result<(), i32> {
        let rc = unsafe { xrtBOSync(self.buf, XCL_BO_SYNC_BO_FROM_DEVICE, self.size, 0) };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.mapped, self.size) }
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mapped, self.size) }
    }
}

impl Drop for XrtBufferHandle {
    fn drop(&mut self) {
        unsafe {
            xrtBOFree(self.buf);
        }
    }
}

pub struct XrtRunHandle {
    run: *mut XrtRun,
}

impl XrtRunHandle {
    pub fn open(krnl: &XrtKernelHandle) -> Option<Self> {
        let run = unsafe { xrtRunOpen(krnl.as_ptr()) };
        if run.is_null() {
            None
        } else {
            Some(Self { run })
        }
    }
    pub fn set_arg<T: Sized>(&self, index: i32, arg: &T) -> Result<(), i32> {
        let rc = unsafe {
            xrtRunSetArg(
                self.run,
                index,
                arg as *const T as *const c_void,
                std::mem::size_of::<T>(),
            )
        };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
    pub fn start(&self) -> Result<(), i32> {
        let rc = unsafe { xrtRunStart(self.run) };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
    pub fn wait(&self, timeout_ms: i32) -> Result<(), i32> {
        let rc = unsafe { xrtRunWait(self.run, timeout_ms) };
        if rc != 0 {
            Err(rc)
        } else {
            Ok(())
        }
    }
}

impl Drop for XrtRunHandle {
    fn drop(&mut self) {
        unsafe {
            xrtRunClose(self.run);
        }
    }
}

// XRT handles are thread-safe (XRT runtime manages synchronization)
unsafe impl Send for XrtDeviceHandle {}
unsafe impl Send for XrtKernelHandle {}
unsafe impl Send for XrtBufferHandle {}
unsafe impl Send for XrtRunHandle {}

unsafe impl Sync for XrtDeviceHandle {}
unsafe impl Sync for XrtKernelHandle {}
unsafe impl Sync for XrtBufferHandle {}
unsafe impl Sync for XrtRunHandle {}
