pub mod data;
pub mod fonts;
pub mod image_gen;

#[cfg(test)]
mod tests {
    use epd_waveshare::epd7in5_v2::Display7in5;
    use image::{ImageBuffer, Luma};

    use crate::data::{self, DisplayData};
    use crate::image_gen::{HEIGHT, WIDTH};

    #[test]
    fn render_to_png() -> Result<(), Box<dyn std::error::Error>> {
        let runargs = data::RunArgs {
            lat: 42.3297,
            lon: -83.0425,
            ics: vec!["./test/test.ics".to_string()],
            max_events: 10,
            weather_ttl: 0,
            calendar_ttl: 0,
        };
        eprintln!("Test render with config: {:?}", runargs);

        let mut display = Display7in5::default();

        let mut state = DisplayData::default();
        data::run(&mut display, &runargs, &mut state);

        eprintln!("Data: {:?}", &state);

        let mut img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(WIDTH as u32, HEIGHT as u32);

        let buffer = display.buffer();

        let bytes_per_row = (WIDTH as usize + 7) / 8;

        for y in 0..HEIGHT as usize {
            for x in 0..WIDTH as usize {
                let byte_idx = y * bytes_per_row + (x / 8);
                let bit = 7 - (x % 8);

                let value = if (buffer[byte_idx] >> bit) & 1 == 0 {
                    255
                } else {
                    0
                };

                img.put_pixel(x as u32, y as u32, Luma([value]));
            }
        }

        img.save("./test/test_output.png")?;

        Ok(())
    }
}
