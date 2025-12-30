use chrono::{DateTime, Timelike, Utc};

#[derive(Debug)]
pub struct WeatherData {
    pub icon: &'static str,
    pub temperature: String,
    pub time: DateTime<Utc>,
}

#[derive(Debug)]
pub enum WeatherError {
    NoResults,
}
impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoResults => write!(f, "No results from api"),
        }
    }
}
impl std::error::Error for WeatherError {}

fn mkicon(icon_url: &str, is_daytime: bool) -> &'static str {
    match icon_url
        .split('/')
        .next_back()
        .unwrap()
        .split('?')
        .take(1)
        .collect::<Vec<_>>()[0]
        .split(',')
        .take(1)
        .collect::<Vec<_>>()[0]
    {
        // Clear / clouds
        "skc" if is_daytime => "\u{2600}",
        "skc" => "\u{1F319}",

        "few" if is_daytime => "\u{1F324}",
        "few" => "\u{1F325}",

        "sct" if is_daytime => "\u{26C5}",
        "sct" => "\u{1F325}",

        "bkn" if is_daytime => "\u{1F325}",
        "bkn" => "\u{2601}",

        "ovc" => "\u{2601}",

        // Wind
        "wind_skc" | "wind_few" | "wind_sct" | "wind_bkn" | "wind_ovc" => "\u{1F32C}",

        // Snow / ice
        "snow" => "\u{1F328}",
        "blizzard" => "\u{2744}",
        "fzra" => "\u{1F327}",
        "rain_fzra" | "snow_fzra" => "\u{1F327}",

        // Mixed precip
        "rain_snow" | "rain_sleet" | "snow_sleet" | "sleet" => "\u{1F326}",

        // Rain
        "rain_showers" | "rain" if is_daytime => "\u{1F327}",
        "rain_showers" | "rain" => "\u{1F327}",

        "rain_showers_hi" if is_daytime => "\u{1F326}",
        "rain_showers_hi" => "\u{1F326}",

        // Thunder
        "tsra" => "\u{26C8}",
        "tsra_sct" | "tsra_hi" => "\u{26A1}",

        // Severe
        "tornado" | "hurricane" | "tropical_storm" => "\u{1F32A}",

        // Atmosphere
        "dust" => "\u{1F4A8}",
        "smoke" => "\u{1F525}",
        "haze" => "\u{1F301}",
        "fog" => "\u{1F32B}",

        // Temperature
        "hot" => "\u{1F525}",
        "cold" => "\u{1F9CA}",

        _ => "",
    }
}

pub fn is_sun_up(lat_deg: f64, lon_deg: f64, now: DateTime<Utc>) -> bool {
    let lat = lat_deg.to_radians();

    // convert to Julian
    let unix_days = now.timestamp() as f64 / 86400.0;
    let jd = unix_days + 2440587.5;

    let n = jd - 2451545.0;

    // solar coords
    let mean_long = (280.460 + 0.9856474 * n).to_radians();
    let mean_anom = (357.528 + 0.9856003 * n).to_radians();

    let eclip_long =
        mean_long + (1.915 * mean_anom.sin() + 0.020 * (2.0 * mean_anom).sin()).to_radians();

    let obliq = (23.439 - 0.0000004 * n).to_radians();

    let decl = (obliq.sin() * eclip_long.sin()).asin();

    let y = (obliq / 2.0).tan().powi(2);

    let eq_time = 4.0
        * (y * (2.0 * mean_long).sin() - 2.0 * 0.0167 * mean_anom.sin()
            + 4.0 * 0.0167 * y * mean_anom.sin() * (2.0 * mean_long).cos()
            - 0.5 * y * y * (4.0 * mean_long).sin()
            - 1.25 * 0.0167 * 0.0167 * (2.0 * mean_anom).sin())
        .to_degrees();

    // solar time
    let time_utc_min = now.hour() as f64 * 60.0 + now.minute() as f64 + now.second() as f64 / 60.0;

    let true_solar_time = (time_utc_min + eq_time + 4.0 * lon_deg) % 1440.0;

    let hour_angle = if true_solar_time / 4.0 < 0.0 {
        true_solar_time / 4.0 + 180.0
    } else {
        true_solar_time / 4.0 - 180.0
    }
    .to_radians();

    // solar elevation
    let elevation = (lat.sin() * decl.sin() + lat.cos() * decl.cos() * hour_angle.cos())
        .asin()
        .to_degrees();

    elevation > -6.0
}

pub fn mkweather(lat: f64, lon: f64) -> Result<WeatherData, Box<dyn std::error::Error>> {
    let points_url = format!("https://api.weather.gov/points/{},{}", lat, lon);
    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let points_resp: serde_json::Value = client
        .execute(
            client
                .get(reqwest::Url::parse(&points_url).expect("ERROR: Bad url provided "))
                .build()
                .expect(""),
        )?
        .json()?;
    let forecast_url = points_resp["properties"]["forecast"]
        .as_str()
        .unwrap_or_else(|| {
            eprintln!("Bad forecast url returned {:?}", points_resp);
            ""
        });
    let forecast_resp: serde_json::Value = client
        .execute(client.get(reqwest::Url::parse(forecast_url)?).build()?)?
        .json()?;
    let periods = forecast_resp["properties"]["periods"]
        .as_array()
        .ok_or(WeatherError::NoResults)?;
    if periods.is_empty() {
        return Err(Box::new(WeatherError::NoResults));
    }
    let temp = periods[0]["temperature"].as_i64().expect("ERROR: ") as i32;
    let unit = periods[0]["temperatureUnit"].as_str().expect("ERROR: ");
    let icon = periods[0]["icon"].as_str().unwrap_or("");
    let is_daytime = is_sun_up(lat, lon, Utc::now());

    Ok(WeatherData {
        icon: mkicon(icon, is_daytime),
        temperature: format!("{}\u{B0}{}", temp, unit),
        time: Utc::now(),
    })
}
