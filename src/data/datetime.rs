pub fn mk_time_date() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format(&std::env::var("DASH_TIME_FORMAT").unwrap_or_else(|_| "%-I:%M %p".to_string()))
            .to_string(),
        now.format(&std::env::var("DASH_DATE_FORMAT").unwrap_or_else(|_| "%a %b %-d".to_string()))
            .to_string(),
    )
}
