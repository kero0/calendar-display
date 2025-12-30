use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};
use ical::IcalParser;
use std::error::Error;
use std::{fs::read_to_string, io::BufReader};

#[derive(Debug, Default)]
pub struct Calendar {
    pub events: Vec<CalendarEvent>,
    pub time: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct CalendarEvent {
    pub title: String,
    pub start: DateTime<Local>,
    pub end: Option<DateTime<Local>>,
    pub allday: bool,
}

fn load_ics(source: &str) -> Result<String, Box<dyn Error>> {
    let source = source.replace("webcal://", "https://");
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::blocking::get(source)?;
        Ok(response.text()?)
    } else {
        Ok(read_to_string(source)?)
    }
}

fn parse_ics_datetime(value: &str, allday: &mut bool) -> Option<DateTime<Local>> {
    // all day event
    if value.len() == 8 {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        *allday = true;
        return date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single();
    }

    // date + time event
    let fmt = match value.len() {
        13 => "%Y%m%dT%H%M",
        15 => "%Y%m%dT%H%M%S",
        16 => "%Y%m%dT%H%M%SZ",
        _ => {
            eprintln!("Don't know how to parse date time string `{}`", value);
            return None;
        }
    };

    let dt = NaiveDateTime::parse_from_str(value, fmt).ok()?;
    dt.and_local_timezone(Local).single()
}

pub fn mkcalendar(path: &str, max_events: usize) -> Result<Calendar, Box<dyn Error>> {
    let text = load_ics(path)?;
    let parser = IcalParser::new(BufReader::new(text.as_bytes()));

    let today = Local::now().date_naive();

    let mut events = parser
        .filter_map(|x| x.ok())
        .flat_map(|x| x.events)
        .filter_map(|event| {
            let mut title: Option<String> = None;
            let mut start: Option<DateTime<Local>> = None;
            let mut end: Option<DateTime<Local>> = None;
            let mut allday = false;

            for prop in event.properties {
                match prop.name.as_str() {
                    "SUMMARY" => title = prop.value,
                    "DTSTART" => {
                        if let Some(v) = prop.value {
                            if let Some(dt) = parse_ics_datetime(&v, &mut allday) {
                                start = Some(dt);
                            }
                        }
                    }

                    "DTEND" => {
                        if let Some(v) = prop.value {
                            if let Some(dt) = parse_ics_datetime(&v, &mut allday) {
                                end = Some(dt);
                            }
                        }
                    }
                    _ => {}
                }
            }

            let (summary, start) = match (title, start) {
                (Some(summary), Some(start)) => (summary, start),
                _ => {
                    return None;
                }
            };

            if start.date_naive() < today {
                return None;
            }

            Some(CalendarEvent {
                title: summary,
                start,
                end,
                allday,
            })
        })
        .collect::<Vec<CalendarEvent>>();

    events.dedup();

    Ok(Calendar {
        time: Utc::now(),
        events: if max_events < events.len() {
            events[0..max_events].to_vec()
        } else {
            events
        },
    })
}
