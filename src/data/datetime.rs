use chrono::{Datelike, Timelike, Weekday};

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

pub fn mk_time_date() -> (String, String, u8, u8) {
    let now = chrono::Local::now();

    let date = match std::env::var("DASH_DATE_FORMAT") {
        Ok(format) => now.format(&format).to_string(),
        Err(_) => format!("{} {}", weekday_letter(now.weekday()), now.format("%b %-d")),
    };

    let time = now
        .format(&std::env::var("DASH_TIME_FORMAT").unwrap_or_else(|_| "%-I:%M %p".to_string()))
        .to_string();

    (shorten_meridiem_label(&time), date, now.hour() as u8, now.minute() as u8)
}
