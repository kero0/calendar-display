use crate::image_gen::create_image;
use crate::{data::calendar::Calendar, image_gen::Disp};
use calendar::mkcalendar;
use chrono::Utc;
use datetime::mk_time_date;
use std::env::{self, VarError};
use weather::{WeatherData, mkweather};

pub mod calendar;
pub mod datetime;
pub mod weather;

#[derive(Debug, Default)]
pub struct DisplayData {
    pub weather: WeatherData,
    pub calendar: Calendar,
    pub date: String,
    pub time: String,
    pub hour: u8,
    pub minute: u8,
}
#[derive(Debug)]
pub struct RunArgs {
    pub lat: f64,
    pub lon: f64,
    pub ics: Vec<String>,
    pub max_events: usize,
    pub weather_ttl: i64,
    pub calendar_ttl: i64,
}

pub fn run(display: &mut Disp, args: &RunArgs, data: &mut DisplayData) {
    let (time, date, hour, minute) = mk_time_date();

    let now = Utc::now();
    data.date = date;
    data.time = time;
    data.hour = hour;
    data.minute = minute;

    if (now - data.weather.time).num_seconds() > args.weather_ttl {
        match mkweather(args.lat, args.lon) {
            Ok(weather) => data.weather = weather,
            Err(e) => eprintln!("Failed to fetch calendar: {}", e),
        };
    }
    if (now - data.calendar.time).num_seconds() > args.calendar_ttl {
        match mkcalendar(&args.ics, args.max_events) {
            Ok(calendar) => {
                data.calendar = calendar;
            }
            Err(e) => eprintln!("Failed to fetch calendar: {}", e),
        }
    }

    eprintln!("{:?}", data);

    match create_image(display, data) {
        Ok(()) => eprintln!("Successfully updated display"),
        Err(err) => eprintln!("Failed to update display: {:?}", err),
    }
}

pub fn mk_run_args() -> RunArgs {
    let lat = env::var("LAT")
        .expect("LAT env var not set")
        .parse::<f64>()
        .expect("LAT must be a float");

    let lon = env::var("LON")
        .expect("LON env var not set")
        .parse::<f64>()
        .expect("LON must be a float");

    let ics = env::var("ICS")
        .expect("ICS env var not set")
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let max_events = match env::var("MAX_EVENTS") {
        Ok(s) => s.parse().expect("MAX_EVENTS must be an integer"),
        Err(VarError::NotPresent) => 10,
        Err(VarError::NotUnicode(_)) => panic!("MAX_EVENTS must be unicode"),
    };

    let weather_ttl = match env::var("WEATHER_TTL") {
        Ok(s) => s.parse().expect("WEATHER_TTL must be an integer (seconds)"),
        Err(VarError::NotPresent) => 1800,
        Err(VarError::NotUnicode(_)) => panic!("WEATHER_TTL must be unicode"),
    };

    let calendar_ttl = match env::var("CALENDAR_TTL") {
        Ok(s) => s
            .parse()
            .expect("CALENDAR_TTL must be an integer (seconds)"),
        Err(VarError::NotPresent) => 600,
        Err(VarError::NotUnicode(_)) => panic!("CALENDAR_TTL must be unicode"),
    };

    RunArgs {
        lat,
        lon,
        ics,
        max_events,
        weather_ttl,
        calendar_ttl,
    }
}
