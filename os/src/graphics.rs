pub mod text;

use core::ptr;

use alloc::{boxed::Box, vec};
use bootloader_api::info::{self, PixelFormat};
use kernel::graphics::{FrameBufferRenderer, RENDERER};
use spin::MutexGuard;

use crate::App;

#[derive()]
pub struct WindowSettings {
    pub window_size_pixels: Vec2,
    pub window_offset_pixels: Vec2,
}
impl WindowSettings {
    pub fn new(window_size_pixels: Vec2, window_offset_pixels: Vec2) -> WindowSettings {
        WindowSettings {
            window_offset_pixels,
            window_size_pixels,
        }
    }
    pub fn blank() -> WindowSettings {
        WindowSettings {
            window_size_pixels: Vec2 { x: 0, y: 0 },
            window_offset_pixels: Vec2 { x: 0, y: 0 },
        }
    }
}
impl<'a> RendererAbstraction<'a> {
    pub fn draw_pixel(&mut self, pixel: Pixel) {
        let x = pixel.pos.x as usize + self.window_settings.window_offset_pixels.x as usize;
        let y = (self.window_settings.window_size_pixels.y
            - 1
            - pixel.pos.y
            - self.window_settings.window_offset_pixels.y) as usize;

        let pixel_offset = y * self.frame_buffer_renderer.info.stride + x;
        let color = [pixel.color.b, pixel.color.g, pixel.color.r, 255];
        let bytes_per_pixel = self.frame_buffer_renderer.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.frame_buffer_renderer.buffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        //?
        // let _ = unsafe { ptr::read_volatile(&self.frame_buffer_renderer.buffer[byte_offset]) };
    }

    pub fn fill_window_with_color(&mut self, color: Color) {
        for x in 0..self.window_settings.window_size_pixels.x {
            for y in 0..self.window_settings.window_size_pixels.y {
                self.draw_pixel(Pixel {
                    pos: Vec2 { x, y },
                    color,
                });
            }
        }
    }
}
pub fn request_renderer<'a>(window_settings: &'a WindowSettings) -> RendererAbstraction<'a> {
    let renderer = kernel::graphics::RENDERER.get().unwrap().lock();

    RendererAbstraction {
        window_settings,
        frame_buffer_renderer: renderer,
    }
}
pub struct RendererAbstraction<'a> {
    window_settings: &'a WindowSettings,
    frame_buffer_renderer: MutexGuard<'a, FrameBufferRenderer>,
}

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: u16,
    pub y: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct Pixel {
    pos: Vec2,
    color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color {
    /// x  0-1
    pub fn lerp(self, rhs: Color, x: f32) -> Color {
        Color {
            r: (self.r as f32 * x + rhs.r as f32 * (1.0 - x)) as u8,
            g: (self.g as f32 * x + rhs.g as f32 * (1.0 - x)) as u8,
            b: (self.b as f32 * x + rhs.b as f32 * (1.0 - x)) as u8,
        }
    }
}

impl core::ops::Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Color {
            r: (self.r as f32 * rhs) as u8,
            g: (self.g as f32 * rhs) as u8,
            b: (self.b as f32 * rhs) as u8,
        }
    }
}
impl core::ops::Add<Color> for Color {
    type Output = Self;

    fn add(self, rhs: Color) -> Self::Output {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}
