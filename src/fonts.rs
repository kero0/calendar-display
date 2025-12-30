use embedded_graphics::prelude::*;
use epd_waveshare::color::Color;

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

use crate::image_gen::Disp;

pub fn draw_text(
    display: &mut Disp,
    font: &Font,
    text: &str,
    origin: Point, // baseline origin
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cursor_x = origin.x;
    let baseline_y = origin.y;

    for ch in text.chars() {
        let glyph = match font.glyphs.iter().find(|(c, _)| *c == ch) {
            Some((_, g)) => g,
            None => continue, // skip missing glyphs silently
        };

        let glyph_x = cursor_x + glyph.x_offset as i32;
        let glyph_y = baseline_y - glyph.y_offset as i32 - glyph.height as i32;

        let mut bit_index = 0;

        for y in 0..glyph.height {
            for x in 0..glyph.width {
                let byte = glyph.bitmap[bit_index >> 3];
                let bit = 7 - (bit_index & 7);

                if (byte >> bit) & 1 != 0 {
                    Pixel(
                        Point::new(glyph_x + x as i32, glyph_y + y as i32),
                        Color::White,
                    )
                    .draw(display)?;
                }

                bit_index += 1;
            }
        }

        cursor_x += glyph.x_advance as i32;
    }

    Ok(())
}
