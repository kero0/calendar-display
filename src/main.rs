use std::{cell::RefCell, rc::Rc};

use epd_waveshare::epd7in5_v2::*;
use epd_waveshare::prelude::WaveshareDisplay;
use linux_embedded_hal::{
    spidev::{self, SpidevOptions},
    Delay, SPIError, SpidevDevice,
};
use rppal::gpio::{Gpio, OutputPin};
use signal_hook::consts::signal::{SIGINT, SIGTERM, SIGUSR1};
use signal_hook::iterator::Signals;

use crate::{
    data::{mk_run_args, run, DisplayData, RunArgs},
    image_gen::Disp,
};
mod data;
mod fonts;
mod image_gen;

pub const EPD_RST_PIN: u8 = 17;
pub const EPD_DC_PIN: u8 = 25;
pub const EPD_CS_PIN: u8 = 8;
pub const EPD_PWR_PIN: u8 = 18;
pub const EPD_BUSY_PIN: u8 = 24;
pub const EPD_MOSI_PIN: u8 = 10;
pub const EPD_SCLK_PIN: u8 = 11;

type Device<'a> =
    Epd7in5<SpidevDevice, rppal::gpio::InputPin, &'a mut OutputPin, &'a mut OutputPin, Delay>;

fn run_and_update(
    cs: &mut OutputPin,
    pwr: &mut OutputPin,
    display: &mut Disp,
    device: &mut Device,
    delay: &mut Delay,
    spi: &mut SpidevDevice,
    runargs: &RunArgs,
    state: Rc<RefCell<DisplayData>>,
) {
    run(display, &runargs, state);
    eprintln!("Updating display");
    cs.set_high();
    pwr.set_high();
    if let Err(e) = device.wake_up(spi, delay) {
        eprintln!("Couldn't wake up display: {e}");
        return;
    };
    if let Err(e) = device.update_and_display_frame(spi, display.buffer(), delay) {
        eprintln!("Couldn't update display: {e}");
    }
    if let Err(e) = device.sleep(spi, delay) {
        eprintln!("Couldn't put display to sleep: {e}");
    }
    cs.set_low();
    pwr.set_low();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runargs = mk_run_args();

    eprintln!("Running with config: {:?}", runargs);

    let mut spi = SpidevDevice::open("/dev/spidev0.0").expect("spidev directory");
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(4_000_000)
        .mode(spidev::SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).expect("spi configuration");

    eprintln!("setting up gpio");
    let gpio = Gpio::new()?;

    let busy = gpio.get(EPD_BUSY_PIN)?.into_input();
    let mut rst = gpio.get(EPD_RST_PIN)?.into_output();
    let mut dc = gpio.get(EPD_DC_PIN)?.into_output();
    let mut cs = gpio.get(EPD_CS_PIN)?.into_output();
    let mut pwr = gpio.get(EPD_PWR_PIN)?.into_output();

    cs.set_high();
    pwr.set_high();

    let mut delay = Delay {};
    let mut device = Device::new(&mut spi, busy, &mut dc, &mut rst, &mut delay, None)
        .expect("Failed to create Epd7in5");
    eprintln!("Created display");
    let mut display = Display7in5::default();

    cs.set_low();
    pwr.set_low();

    eprintln!("Device successfully initialized!");

    eprintln!("Starting initial update");
    let  state = Rc::new(RefCell::new(DisplayData::default()));
    run_and_update(
        &mut cs,
        &mut pwr,
        &mut display,
        &mut device,
        &mut delay,
        &mut spi,
        &runargs,
        state.clone(),
    );
    eprintln!("Finished initial update");

    let mut signals = Signals::new([SIGUSR1, SIGINT, SIGTERM])?;
    eprintln!("Waiting for signals...");
    eprintln!("SIGUSR1 ? update display");
    eprintln!("SIGINT/SIGTERM ? exit");

    for signal in signals.forever() {
        match signal {
            SIGUSR1 => {
                println!("SIGUSR1 received: running update");
                run_and_update(
                    &mut cs,
                    &mut pwr,
                    &mut display,
                    &mut device,
                    &mut delay,
                    &mut spi,
                    &runargs,
                    state.clone(),
                );
            }
            SIGINT | SIGTERM => {
                println!("Exit signal received");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Luma};

    use image_gen::{HEIGHT, WIDTH};

    #[test]
    fn render_to_png() -> Result<(), Box<dyn std::error::Error>> {
        let runargs = data::RunArgs {
            lat: 42.3297,
            lon: -83.0425,
            ics: "./test/test.ics".to_string(),
            max_events: 10,
            weather_ttl: 0,
            calendar_ttl: 0,
        };
        eprintln!("Test render with config: {:?}", runargs);

        let mut display = Display7in5::default();

        let data = data::run(&mut display, &runargs, None);

        println!("Data: {:?}", data);

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
