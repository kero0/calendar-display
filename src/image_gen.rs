use crate::data::calendar::CalendarEvent;
use crate::{data::DisplayData, fonts::*};
use chrono::{Datelike, Weekday};
use embedded_graphics::{prelude::*, primitives::*};
use epd_waveshare::color::Color;
use std::f32::consts::PI;

pub type Disp = epd_waveshare::epd7in5_v2::Display7in5;

pub const WIDTH: i32 = 800;
pub const HEIGHT: i32 = 480;
const TOP: i32 = 50;
const LEFT_COL_X: i32 = 10;
const LEFT_COL_W: i32 = 300;
const RIGHT_COL_X: i32 = LEFT_COL_X + LEFT_COL_W;
const LINE_GAP: i32 = 4;
const TITLE_LINES_MAX: i32 = 2;
const CLOCK_DIAMETER: i32 = LEFT_COL_W * 13 / 20;
const CLOCK_RADIUS: i32 = CLOCK_DIAMETER / 2;

const LINE_HEIGHT: i32 = (FONT_BODY.ascent - FONT_BODY.descent) as i32;
const DETAIL_BLOCK_HEIGHT: i32 = LINE_HEIGHT + LINE_GAP;
const BOTTOM_LIMIT: i32 = HEIGHT - (LINE_GAP * 2);

fn weekday_letter(weekday: Weekday) -> char {
    match weekday {
        Weekday::Mon => 'M',
        Weekday::Tue => 'T',
        Weekday::Wed => 'W',
        Weekday::Thu => 'R',
        Weekday::Fri => 'F',
        Weekday::Sat => 'S',
        Weekday::Sun => 'U',
    }
}

fn shorten_meridiem_label(text: &str) -> String {
    text.replace(" AM", " A")
        .replace(" PM", " P")
        .replace(" am", " a")
        .replace(" pm", " p")
}

pub fn create_image(
    display: &mut Disp,
    data: &DisplayData,
) -> Result<(), Box<dyn std::error::Error>> {
    Rectangle::new(Point::new(0, 0), Size::new(WIDTH as u32, HEIGHT as u32))
        .into_styled(PrimitiveStyle::with_fill(Color::Black))
        .draw(display)?;

    // Left column
    let mut y = (TOP as f32 * 1.5) as i32;

    // Date + Clock
    draw_text(display, &FONT_HEADER, data.date.as_str(), Point::new(LEFT_COL_X, y))?;
    y += (1.6 * ((FONT_HEADER.ascent - FONT_HEADER.descent) as f32)) as i32;

    let clock_center = Point::new(LEFT_COL_X + (LEFT_COL_W * 4 / 5) / 2, y + CLOCK_RADIUS / 2);
    draw_analog_clock(display, clock_center, CLOCK_RADIUS, data.hour, data.minute)?;
    y += CLOCK_DIAMETER + 36;

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
                } => format!("{} {}", weekday_letter(start.weekday()), start.format("%b %d")),
                CalendarEvent {
                    start,
                    end: Some(end),
                    allday: true,
                    title: _,
                } => format!(
                    "{} {} - {} {}",
                    weekday_letter(start.weekday()),
                    start.format("%b %d"),
                    weekday_letter(end.weekday()),
                    end.format("%b %d")
                ),
                CalendarEvent {
                    start,
                    end: None,
                    allday: false,
                    title: _,
                } => format!(
                    "{} {}",
                    weekday_letter(start.weekday()),
                    shorten_meridiem_label(&start.format("%b %d %-I:%M %p").to_string())
                ),
                CalendarEvent {
                    start,
                    end: Some(end),
                    allday: false,
                    title: _,
                } => format!(
                    "{} {} - {} {}",
                    weekday_letter(start.weekday()),
                    shorten_meridiem_label(&start.format("%b %d %-I:%M %p").to_string()),
                    weekday_letter(end.weekday()),
                    shorten_meridiem_label(&end.format("%b %d %-I:%M %p").to_string())
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

fn draw_analog_clock(
    display: &mut Disp,
    center: Point,
    radius: i32,
    hour: u8,
    minute: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let rounded_minute = ((minute as i32 + 7) / 15 * 15).rem_euclid(60);
    let rounded_hour = (hour as i32 + if rounded_minute == 0 && minute >= 53 { 1 } else { 0 }) % 12;

    let face = Circle::new(
        Point::new(center.x - radius, center.y - radius),
        (radius * 2) as u32,
    )
    .into_styled(PrimitiveStyle::with_stroke(Color::White, 1));
    face.draw(display)?;

    for tick in 0..12 {
        let angle = tick as f32 * (PI / 6.0);
        let is_quarter = tick % 3 == 0;
        let outer = clock_point(center, radius - 2, angle);
        let inner = clock_point(center, radius - if is_quarter { 12 } else { 8 }, angle);
        Line::new(inner, outer)
            .into_styled(PrimitiveStyle::with_stroke(Color::White, 1))
            .draw(display)?;
    }

    let minute_angle = rounded_minute as f32 * (PI / 30.0);
    let hour_angle = (rounded_hour as f32 + rounded_minute as f32 / 60.0) * (PI / 6.0);

    Line::new(center, clock_point(center, radius - 15, minute_angle))
        .into_styled(PrimitiveStyle::with_stroke(Color::White, 2))
        .draw(display)?;
    Line::new(center, clock_point(center, radius - 35, hour_angle))
        .into_styled(PrimitiveStyle::with_stroke(Color::White, 2))
        .draw(display)?;

    Circle::new(Point::new(center.x - 2, center.y - 2), 4)
        .into_styled(PrimitiveStyle::with_fill(Color::White))
        .draw(display)?;

    Ok(())
}

fn clock_point(center: Point, length: i32, angle: f32) -> Point {
    let x = center.x as f32 + length as f32 * angle.sin();
    let y = center.y as f32 - length as f32 * angle.cos();
    Point::new(x.round() as i32, y.round() as i32)
}
