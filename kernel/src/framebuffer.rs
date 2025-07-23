// use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
// use core::{fmt, ptr};
// use font_constants::BACKUP_CHAR;
// use noto_sans_mono_bitmap::{
//     FontWeight, RasterHeight, RasterizedChar, get_raster, get_raster_width,
// };
//
// use crate::logger;
//
// pub fn init_frame_buffer(framebuffer: &'static mut FrameBuffer) {
//     let framebuffer_info: FrameBufferInfo = framebuffer.info();
//     let buffer = framebuffer.buffer_mut();
//
//
//     log::debug!("OK:Frame buffer init");
// }
//
//     fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
//         let pixel_offset = y * self.info.stride + x;
//         let color = match self.info.pixel_format {
//             PixelFormat::Rgb => [intensity, intensity, intensity, 0],
//             PixelFormat::Bgr => [intensity, intensity, intensity, 0],
//             PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
//             other => {
//                 // set a supported (but invalid) pixel format before panicking to avoid a double
//                 // panic; it might not be readable though
//                 self.info.pixel_format = PixelFormat::Rgb;
//                 panic!("pixel format {:?} not supported in logger", other)
//             }
//         };
//         let bytes_per_pixel = self.info.bytes_per_pixel;
//         let byte_offset = pixel_offset * bytes_per_pixel;
//         self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
//             .copy_from_slice(&color[..bytes_per_pixel]);
//         let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
//     }
//
// }
//
// unsafe impl Send for FrameBufferWriter {}
// unsafe impl Sync for FrameBufferWriter {}
//
// impl fmt::Write for FrameBufferWriter {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         for c in s.chars() {
//             self.write_char(c);
//         }
//         Ok(())
//     }
// }
