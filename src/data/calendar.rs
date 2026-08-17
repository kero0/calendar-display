use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
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

fn parse_naive_datetime(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M"))
        .ok()
}

fn tzid_from_params(params: &Option<Vec<(String, Vec<String>)>>) -> Option<&str> {
    let (.., values) = params
        .as_ref()?
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("TZID"))?;
    values.first().map(String::as_str)
}

fn parse_ics_datetime(value: &str, tzid: Option<&str>, allday: &mut bool) -> Option<DateTime<Local>> {
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

    // Explicit UTC timestamps (ending in Z) must be interpreted as UTC first.
    if let Some(value_no_z) = value.strip_suffix('Z') {
        let dt_utc = parse_naive_datetime(value_no_z)?.and_utc();
        return Some(dt_utc.with_timezone(&Local));
    }

    let dt = parse_naive_datetime(value)?;

    if let Some(tzid) = tzid {
        match tzid.parse::<Tz>() {
            Ok(tz) => {
                let dt_tz = tz
                    .from_local_datetime(&dt)
                    .single()
                    .or_else(|| tz.from_local_datetime(&dt).earliest())
                    .or_else(|| tz.from_local_datetime(&dt).latest())?;
                return Some(dt_tz.with_timezone(&Local));
            }
            Err(_) => eprintln!("Unknown TZID `{}`; falling back to local timezone", tzid),
        }
    }

    Local
        .from_local_datetime(&dt)
        .single()
        .or_else(|| Local.from_local_datetime(&dt).earliest())
        .or_else(|| Local.from_local_datetime(&dt).latest())
}

fn parse_ics_file(path: &str, today: NaiveDate) -> Result<Vec<CalendarEvent>, Box<dyn Error>> {
    let text = load_ics(path)?;
    let parser = IcalParser::new(BufReader::new(text.as_bytes()));

    let events = parser
        .flat_map(|x| x.ok())
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
                        let tzid = tzid_from_params(&prop.params);
                        if let Some(v) = prop.value
                            && let Some(dt) = parse_ics_datetime(&v, tzid, &mut allday)
                        {
                            start = Some(dt);
                        }
                    }

                    "DTEND" => {
                        let tzid = tzid_from_params(&prop.params);
                        if let Some(v) = prop.value
                            && let Some(dt) = parse_ics_datetime(&v, tzid, &mut allday)
                        {
                            end = Some(dt);
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

    Ok(events)
}

pub fn mkcalendar(paths: &[String], max_events: usize) -> Result<Calendar, Box<dyn Error>> {
    let today = Local::now().date_naive();
    let mut events = paths
        .iter()
        .flat_map(|path| parse_ics_file(path, today).unwrap_or_default())
        .collect::<Vec<CalendarEvent>>();

    events.dedup();
    events.sort_by_key(|a| a.start);
    events.truncate(max_events);

    Ok(Calendar {
        time: Utc::now(),
        events,
    })
}
