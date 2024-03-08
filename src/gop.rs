use core::ffi::c_void;

#[repr(C)]
pub enum FfiPixelFormat {
    Rgb,
    Bgr
}

#[repr(C)]
pub struct GopInfo {
    pub frame_buffer: *const c_void,
    pub holizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub stride: usize,
    pub pixel_format: FfiPixelFormat,
}

pub fn clear_screen(gop_info: &GopInfo) {
        for h in 0..gop_info.vertical_resolution {
                for w in 0..gop_info.holizontal_resolution {
                        let mut ptr = gop_info.frame_buffer as *mut u32;
                        ptr = unsafe { ptr.add(h * gop_info.stride + w) };
                        unsafe {
                                *ptr = 0xffffffff;
                        }
                }
        }
}

