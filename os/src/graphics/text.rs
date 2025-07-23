use log::debug;
use noto_sans_mono_bitmap::{RasterizedChar, get_raster};

use crate::graphics::{Color, Pixel, Vec2, text::font_constants::BACKUP_CHAR};

/// Additional vertical space between lines
pub const LINE_SPACING: usize = 2;
/// Additional horizontal space between characters.
pub const LETTER_SPACING: usize = 0;

/// Padding from the border. Prevent that font is too close to border.
const BORDER_PADDING: usize = 1;

/// Constants for the usage of the [`noto_sans_mono_bitmap`] crate.
pub mod font_constants {
    use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster_width};

    use super::*;

    /// Height of each char raster. The font size is ~0.84% of this. Thus, this is the line height that
    /// enables multiple characters to be side-by-side and appear optically in one line in a natural way.
    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

    /// The width of each single symbol of the mono space font.
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

    /// Backup character if a desired symbol is not available by the font.
    /// The '�' character requires the feature "unicode-specials".
    pub const BACKUP_CHAR: char = '?';

    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

/// Returns the raster of the given char or the raster of [`font_constants::BACKUP_CHAR`].
fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(
            c,
            font_constants::FONT_WEIGHT,
            font_constants::CHAR_RASTER_HEIGHT,
        )
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Should get raster of backup char."))
}

impl<'a> super::RendererAbstraction<'a> {
    pub fn draw_char_no_anti_aliasing(
        &mut self,
        bottom_left_corner: Vec2,
        char: char,
        text_color: Color,
    ) {
        let rendered_char = get_char_raster(char);
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                if *byte == 0 {
                    continue;
                }
                let color = text_color;
                let pos = Vec2 {
                    x: bottom_left_corner.x + x as u16,
                    y: bottom_left_corner.y + font_constants::CHAR_RASTER_HEIGHT.val() as u16
                        - y as u16,
                };

                self.draw_pixel(Pixel { pos, color });
            }
        }
    }

    pub fn draw_char(
        &mut self,
        bottom_left_corner: Vec2,
        char: char,
        text_color: Color,
        background_color: Color,
    ) {
        let rendered_char = get_char_raster(char);
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                if *byte == 0 {
                    continue;
                }
                let color = text_color.lerp(background_color, *byte as f32 / 255.0);
                let pos = Vec2 {
                    x: bottom_left_corner.x + x as u16,
                    y: bottom_left_corner.y + font_constants::CHAR_RASTER_HEIGHT.val() as u16
                        - y as u16,
                };

                self.draw_pixel(Pixel { pos, color });
            }
        }
    }

    /// writes all pixels in size of typical char to specified color  
    pub fn draw_block_char(&mut self, bottom_left_corner: Vec2, color: Color) {
        let rendered_char = get_char_raster('a');
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, _) in row.iter().enumerate() {
                let pos = Vec2 {
                    x: bottom_left_corner.x + x as u16,
                    y: bottom_left_corner.y + y as u16,
                };

                self.draw_pixel(Pixel { pos, color });
            }
        }
    }
}
