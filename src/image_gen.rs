use crate::data::calendar::CalendarEvent;
use crate::{data::DisplayData, fonts::*};
use embedded_graphics::{prelude::*, primitives::*};
use epd_waveshare::color::Color;

pub type Disp = epd_waveshare::epd7in5_v2::Display7in5;

pub const WIDTH: i32 = 800;
pub const HEIGHT: i32 = 480;
const TOP: i32 = 50;
const LEFT_COL_X: i32 = 15;
const LEFT_COL_W: i32 = 350;
const RIGHT_COL_X: i32 = LEFT_COL_X + LEFT_COL_W;
const LINE_GAP: i32 = 4;
const TITLE_LINES_MAX: i32 = 2;

const LINE_HEIGHT: i32 = (FONT_BODY.ascent - FONT_BODY.descent) as i32;
const DETAIL_BLOCK_HEIGHT: i32 = LINE_HEIGHT + LINE_GAP;
const BOTTOM_LIMIT: i32 = HEIGHT - (LINE_GAP * 2);

pub fn create_image(
    display: &mut Disp,
    data: &DisplayData,
) -> Result<(), Box<dyn std::error::Error>> {
    Rectangle::new(Point::new(0, 0), Size::new(WIDTH as u32, HEIGHT as u32))
        .into_styled(PrimitiveStyle::with_fill(Color::Black))
        .draw(display)?;

    // Left column
    let mut y = (TOP as f32 * 1.5) as i32;

    // Date + Time
    draw_text(display, &FONT_HEADER, data.date.as_str(), Point::new(LEFT_COL_X, y))?;
    y += 2 * (FONT_HEADER.ascent - FONT_HEADER.descent) as i32;
    draw_text(display, &FONT_LARGE, data.time.as_str(), Point::new(LEFT_COL_X, y))?;
    y += 2 * (FONT_LARGE.ascent - FONT_LARGE.descent) as i32;

    // Weather
    let icon_glyph = FONT_EMOJI
        .glyphs
        .iter()
        .find(|(c, _)| data.weather.icon.starts_with(*c));
    if let Some((_, glyph)) = icon_glyph {
        draw_text(
            display,
            &FONT_EMOJI,
            data.weather.icon,
            Point::new(LEFT_COL_X, y),
        )?;
        draw_text(
            display,
            &FONT_HEADER,
            data.weather.temperature.as_str(),
            Point::new(LEFT_COL_X + glyph.width as i32 * 2, y),
        )?;
    } else {
        draw_text(
            display,
            &FONT_HEADER,
            data.weather.temperature.as_str(),
            Point::new(LEFT_COL_X, y),
        )?;
    }

    // Right Column
    let mut y = TOP;

    // Calendar
    for event in &data.calendar.events {
        let title_lines = wrap_text(
            &FONT_BODY,
            event.title.as_str(),
            (WIDTH - LEFT_COL_W) as i16,
            TITLE_LINES_MAX as usize,
        );

        if y + title_lines.len() as i32 * (FONT_BODY.ascent - FONT_BODY.descent) as i32
            + DETAIL_BLOCK_HEIGHT
            > BOTTOM_LIMIT
        {
            break;
        }

        for line in title_lines {
            draw_text(display, &FONT_BODY, line, Point::new(RIGHT_COL_X, y))?;
            y += FONT_BODY.pixel_size as i32 + LINE_GAP;
        }

        draw_text(
            display,
            &FONT_BODY,
            match &event {
                CalendarEvent {
                    start,
                    end: None,
                    allday: true,
                    title: _,
                } => start.format("%a %b %d").to_string(),
                CalendarEvent {
                    start,
                    end: Some(end),
                    allday: true,
                    title: _,
                } => format!("{} - {}", start.format("%a %b %d"), end.format("%a %b %d")),
                CalendarEvent {
                    start,
                    end: None,
                    allday: false,
                    title: _,
                } => start.format("%a %b %d %-I %p").to_string(),
                CalendarEvent {
                    start,
                    end: Some(end),
                    allday: false,
                    title: _,
                } => format!(
                    "{} - {}",
                    start.format("%a %b %d %-I %p"),
                    end.format("%a %b %d %-I %p")
                ),
            }
            .as_str(),
            Point::new(RIGHT_COL_X, y),
        )?;
        y += (FONT_BODY.ascent - FONT_BODY.descent) as i32 + LINE_GAP;
    }

    Ok(())
}

fn wrap_text<'a>(font: &Font, text: &'a str, max_width: i16, max_lines: usize) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_space = None;
    let mut width = 0;

    lines.reserve_exact(max_lines);
    let chars: Vec<(usize, char)> = text.char_indices().collect();

    for (i, (_, ch)) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            last_space = Some(i);
        }

        if let Some((_, g)) = font.glyphs.iter().find(|(c, _)| *c == *ch) {
            width += g.x_advance;
        }

        if width > max_width {
            let break_i = last_space.unwrap_or(i);
            let end = chars[break_i].0;
            lines.push(&text[start..end]);

            if lines.len() >= max_lines {
                return lines;
            }

            start = chars[break_i + 1].0;
            width = 0;
            last_space = None;
        }
    }

    if start < text.len() && lines.len() < max_lines {
        lines.push(&text[start..]);
    }

    lines
}
