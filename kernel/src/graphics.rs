use bootloader_api::info::{FrameBuffer, FrameBufferInfo};
use conquer_once::spin::OnceCell;
use lazy_static::lazy_static;
use spin::Mutex;
use spinning_top::{Spinlock, lock_api::RawMutex};

pub static RENDERER: OnceCell<spin::Mutex<FrameBufferRenderer>> = OnceCell::uninit();

pub struct FrameBufferRenderer {
    pub buffer: &'static mut [u8],
    pub info: FrameBufferInfo,
}

impl FrameBufferRenderer {
    pub fn new(framebuffer: &'static mut FrameBuffer) -> FrameBufferRenderer {
        let info: FrameBufferInfo = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        FrameBufferRenderer { buffer, info }
    }
}
