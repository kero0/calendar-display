use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use fontdue::Font;
struct FontSpec<'a> {
    name: &'a str,
    size: f32,
    ranges: &'a [(u32, u32)],
    is_emoji: bool,
}
const FONT_SPECS: &[FontSpec] = &[
    FontSpec {
        name: "FONT_HEADER",
        size: 56.0,
        ranges: &[
            (0x0020, 0x007E), // Basic Latin
            (0x00A0, 0x00FF), // Latin-1 Supplement
            (0x0100, 0x017F), // Latin Extended-A
        ],
        is_emoji: false,
    },
    FontSpec {
        name: "FONT_BODY",
        size: 24.0,
        ranges: &[(0x0020, 0x007E), (0x00A0, 0x00FF), (0x0100, 0x017F)],
        is_emoji: false,
    },
    FontSpec {
        name: "FONT_LARGE",
        size: 64.0,
        ranges: &[(0x0020, 0x007E), (0x00A0, 0x00FF), (0x0100, 0x017F)],
        is_emoji: false,
    },
    FontSpec {
        name: "FONT_EMOJI",
        size: 56.0,
        ranges: &[
            (0x2600, 0x26FF),
            (0x2700, 0x27BF),
            (0x1F300, 0x1F5FF),
            (0x1F600, 0x1F64F),
            (0x1F680, 0x1F6FF),
        ],
        is_emoji: true,
    },
];

fn main() {
    let font = env::var("REGULAR_FONT_PATH").unwrap_or("assets/NotoSans-Regular.ttf".to_string());
    let emoji = env::var("EMOJI_FONT_PATH").unwrap_or("assets/NotoEmoji-Regular.ttf".to_string());
    println!("cargo:rerun-if-changed={}", font);
    println!("cargo:rerun-if-changed={}", emoji);

    let font_data = fs::read(font).expect("Failed to read regular font");
    let emoji_data = fs::read(emoji).expect("Failed to read emoji font");

    let font = Font::from_bytes(font_data, fontdue::FontSettings::default())
        .expect("Failed to parse font");
    let emoji = Font::from_bytes(emoji_data, fontdue::FontSettings::default())
        .expect("Failed to parse emoji font");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("fonts.rs");
    let mut file = File::create(&dest).unwrap();

    writeln!(
        file,
        r#"
// AUTO-GENERATED DO NOT EDIT

#[derive(Debug)]
pub struct Glyph {{
    pub width: u16,
    pub height: u16,
    pub x_advance: i16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub bitmap: &'static [u8],
}}

#[derive(Debug)]
pub struct Font {{
    pub pixel_size: u16,
    pub ascent: i16,
    pub descent: i16,
    pub glyphs: &'static [(char, Glyph)],
}}
"#
    )
    .unwrap();

    for &FontSpec {
        name,
        size,
        ranges,
        is_emoji,
    } in FONT_SPECS
    {
        let px = size;

        let mut glyphs = Vec::new();
        let mut ascent = 0i16;
        let mut descent = 0i16;

        for &(start, end) in ranges.iter() {
            for cp in start..=end {
                let ch = match char::from_u32(cp) {
                    Some(c) => c,
                    None => continue,
                };

                let (metrics, bitmap) = {
                    if is_emoji {
                        &emoji
                    } else {
                        &font
                    }
                }
                .rasterize(ch, px);

                if metrics.advance_width == 0.0 {
                    continue;
                }

                ascent = ascent.max(metrics.height as i16 + metrics.ymin as i16);
                descent = descent.min(metrics.ymin as i16);

                let x_offset = metrics.xmin as i16;
                let y_offset = metrics.ymin;

                let mut packed = Vec::<u8>::new();
                let mut current = 0u8;
                let mut bits = 0u8;

                for y in 0..metrics.height {
                    for x in 0..metrics.width {
                        let idx = y * metrics.width + x;
                        let on = bitmap[idx] > 128;

                        current <<= 1;
                        if on {
                            current |= 1;
                        }
                        bits += 1;

                        if bits == 8 {
                            packed.push(current);
                            current = 0;
                            bits = 0;
                        }
                    }
                }

                if bits != 0 {
                    current <<= 8 - bits;
                    packed.push(current);
                }

                let ch_expr = format!("'\\u{{{:x}}}'", ch as u32);

                glyphs.push(format!(
                    "    ({}, Glyph {{ width: {}, height: {}, x_advance: {}, x_offset: {}, y_offset: {}, bitmap: &{:?} }}),",
                    ch_expr,
                    metrics.width,
                    metrics.height,
                    metrics.advance_width as i16,
                    x_offset,
                    y_offset,
                    packed,
                ));
            }
        }

        writeln!(file, "\npub static {}: Font = Font {{", name).unwrap();
        writeln!(file, "    pixel_size: {},", px as u16).unwrap();
        writeln!(file, "    ascent: {},", ascent).unwrap();
        writeln!(file, "    descent: {},", descent).unwrap();
        writeln!(file, "    glyphs: &[").unwrap();

        for g in glyphs {
            writeln!(file, "{}", g).unwrap();
        }

        writeln!(file, "    ],").unwrap();
        writeln!(file, "}};").unwrap();
    }
}

