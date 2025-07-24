#![no_std]
#![no_main]

mod commands;

extern crate alloc;

use core::{default, str::FromStr, u16};

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use log::warn;
use os::graphics::{text::font_constants::CHAR_RASTER_HEIGHT, *};
use pc_keyboard::DecodedKey;
use spin::Mutex;

pub struct Terminal {
    window_settings: WindowSettings,
    current_str_input: String,
    current_char_x: u16,
    logs: bool,
    commands: BTreeMap<String, commands::OnCommandFunction>,
}

impl Terminal {
    pub fn new() -> Terminal {
        Terminal {
            window_settings: WindowSettings::blank(),
            current_str_input: String::new(),
            current_char_x: 0,
            logs: true,
            commands: commands::init_commands(),
        }
    }

    fn new_char_pos(&self) -> Vec2 {
        Vec2 {
            x: self.current_char_x * CHAR_WIDTH,
            y: 0,
        }
    }
    fn on_log(&mut self, log_data: &[u8; 80]) {}
}
// converts rows/columns to pixels
fn char_pos(x: u16, y: u16) -> Vec2 {
    Vec2 {
        x: x * CHAR_WIDTH,
        y: y * CHAR_HEIGHT,
    }
}
pub const CURSOR: Color = Color {
    r: 252,
    g: 230,
    b: 169,
};
pub const TEXT: Color = Color {
    r: 169,
    g: 234,
    b: 252,
};
const BACKGROUND_HISTORY: Color = Color {
    r: 20,
    g: 20,
    b: 20,
};
const BACKGROUND_COMMAND: Color = Color {
    r: 40,
    g: 40,
    b: 40,
};
const CHAR_WIDTH: u16 = os::graphics::text::font_constants::CHAR_RASTER_WIDTH as u16;
const CHAR_HEIGHT: u16 = os::graphics::text::font_constants::CHAR_RASTER_HEIGHT.val() as u16;

impl os::App for Terminal {
    fn on_key_pressed(&mut self, key: &DecodedKey) {
        match key {
            DecodedKey::RawKey(key_code) => match key_code {
                default => warn!("key is not supported: {default:?}"),
            },
            DecodedKey::Unicode(char) => match char {
                '\t' => {}
                '\r' => {}
                '\n' => {
                    // enter
                    self.parse_and_run_current_command();

                    let mut renderer = request_renderer(&self.window_settings);

                    for x in 0..self.current_char_x + 1 {
                        renderer.draw_block_char(char_pos(x, 0), BACKGROUND_COMMAND);
                    }

                    self.current_str_input.clear();
                    self.current_char_x = 0;

                    renderer.draw_char(
                        char_pos(self.current_char_x, 0),
                        '|',
                        CURSOR,
                        BACKGROUND_COMMAND,
                    );
                }
                '\u{1b}' => {
                    // esc
                }
                '\u{8}' => {
                    // backspace

                    let mut renderer = request_renderer(&self.window_settings);
                    if self.current_char_x == 0 {
                        return;
                    }

                    // clear last cursor
                    renderer.draw_char_no_anti_aliasing(
                        char_pos(self.current_char_x, 0),
                        '|',
                        BACKGROUND_COMMAND,
                    );

                    self.current_char_x -= 1;
                    let pos = self.new_char_pos();
                    self.current_str_input
                        .remove(self.current_str_input.len() - 1);
                    renderer.draw_block_char(pos, BACKGROUND_COMMAND);

                    // draw current cursor
                    renderer.draw_char(pos, '|', CURSOR, BACKGROUND_COMMAND);
                }
                _ => {
                    // clear last cursor

                    let mut renderer = request_renderer(&self.window_settings);

                    renderer.draw_char_no_anti_aliasing(
                        char_pos(self.current_char_x, 0),
                        '|',
                        BACKGROUND_COMMAND,
                    );

                    let pos = self.new_char_pos();
                    self.current_str_input += char.encode_utf8(&mut [0; 1]);
                    renderer.draw_char(pos, *char, TEXT, BACKGROUND_COMMAND);
                    self.current_char_x += 1;

                    // draw current cursor
                    renderer.draw_char(
                        char_pos(self.current_char_x, 0),
                        '|',
                        CURSOR,
                        BACKGROUND_COMMAND,
                    );
                }
            },
        }
    }

    fn on_time(&mut self) {
        todo!()
    }

    fn init(&mut self, graphics_data: WindowSettings) {
        // let app = Box::new(self) as os::AppType;
        // os::ON_LOG_LISTENERS.lock().push(Mutex::new();
        self.window_settings = graphics_data;

        let mut renderer = request_renderer(&self.window_settings);

        renderer.fill_window_with_color(BACKGROUND_HISTORY);

        // draw command bar
        for x in 0..self.window_settings.window_size_pixels.x / CHAR_WIDTH {
            renderer.draw_block_char(
                Vec2 {
                    x: x * CHAR_WIDTH,
                    y: 0,
                },
                BACKGROUND_COMMAND,
            );
        }
        renderer.draw_char(
            char_pos(self.current_char_x, 0),
            '|',
            CURSOR,
            BACKGROUND_COMMAND,
        );
    }

    fn on_log(&mut self, log: &[u8; os::MAX_LOG_SIZE]) {
        todo!()
    }
}
