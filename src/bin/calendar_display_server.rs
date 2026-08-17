use epd_waveshare::epd7in5_v2::*;
use signal_hook::consts::signal::{SIGINT, SIGTERM, SIGUSR1};
use signal_hook::iterator::Signals;

use std::sync::{Arc, Mutex};

use calendar_display::data::{DisplayData, mk_run_args, run};

use std::io::{self, Write};

fn write_1bpp_bmp<W: Write>(
    writer: &mut W,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> io::Result<()> {
    let row_bytes = width.div_ceil(8);
    let padded_row_bytes = row_bytes.div_ceil(4) * 4;
    let image_size = padded_row_bytes * height;

    assert_eq!(
        pixels.len(),
        (row_bytes * height) as usize,
        "Wrong pixel buffer size"
    );

    let file_size = 14 + 40 + 8 + image_size;

    // BMP file header
    writer.write_all(b"BM")?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(&0u16.to_le_bytes())?;
    writer.write_all(&0u16.to_le_bytes())?;
    writer.write_all(&62u32.to_le_bytes())?;

    // BITMAPINFOHEADER
    writer.write_all(&40u32.to_le_bytes())?;
    writer.write_all(&(width as i32).to_le_bytes())?;
    writer.write_all(&(height as i32).to_le_bytes())?; // POSITIVE

    writer.write_all(&1u16.to_le_bytes())?; // planes
    writer.write_all(&1u16.to_le_bytes())?; // 1 bpp
    writer.write_all(&0u32.to_le_bytes())?; // BI_RGB
    writer.write_all(&image_size.to_le_bytes())?;
    writer.write_all(&0i32.to_le_bytes())?;
    writer.write_all(&0i32.to_le_bytes())?;
    writer.write_all(&2u32.to_le_bytes())?;
    writer.write_all(&2u32.to_le_bytes())?;

    // Palette: color 0 = black
    writer.write_all(&[0, 0, 0, 0])?;

    // Palette: color 1 = white
    writer.write_all(&[255, 255, 255, 0])?;

    // BMP stores rows bottom-to-top.
    let padding = vec![0u8; (padded_row_bytes - row_bytes) as usize];

    for y in (0..height as usize).rev() {
        let start = y * row_bytes as usize;
        let end = start + row_bytes as usize;

        writer.write_all(&pixels[start..end])?;
        writer.write_all(&padding)?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new([SIGUSR1, SIGINT, SIGTERM])?;

    let runargs = mk_run_args();
    let quit = Arc::new(Mutex::new(false));

    eprintln!("Running with config: {:?}", runargs);
    let display = Arc::new(Mutex::new(Display7in5::default()));

    eprintln!("Starting initial update");
    let state = Arc::new(Mutex::new(DisplayData::default()));

    {
        run(
            &mut display.lock().unwrap(),
            &runargs,
            &mut state.lock().unwrap(),
        );
    }

    eprintln!("Finished initial update");

    let listener = {
        let state = state.clone();
        let display = display.clone();
        let quit = quit.clone();
        eprintln!("Waiting for signals...");
        eprintln!("SIGUSR1 ? update display");
        eprintln!("SIGINT/SIGTERM ? exit");
        std::thread::spawn(move || {
            for signal in signals.forever() {
                match signal {
                    SIGUSR1 => {
                        eprintln!("SIGUSR1 received: running update");
                        run(
                            &mut display.lock().unwrap(),
                            &runargs,
                            &mut state.lock().unwrap(),
                        );
                    }
                    SIGINT | SIGTERM => {
                        eprintln!("Exit signal received");
                        *quit.lock().unwrap() = true;
                        break;
                    }
                    _ => {}
                }
            }
        })
    };

    let server = {
        let quit = quit.clone();
        let display = display.clone();

        std::thread::spawn(move || {
            let address =
                std::env::var("LISTEN_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = std::env::var("LISTEN_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080);
            let server =
                tiny_http::Server::http(format!("{}:{}", address, port)).unwrap_or_else(|e| {
                    eprintln!("Failed to start HTTP server on {}:{}: {}", address, port, e);
                    std::process::exit(1);
                });
            eprintln!("HTTP server listening on {}:{}", address, port);
            const SIZE: usize = (WIDTH * HEIGHT) as usize;
            let mut image_buffer = [0u8; SIZE];
            let mut writer = std::io::Cursor::new(vec![0u8; SIZE / 8 + 62]);
            while !*quit.lock().unwrap() {
                match server.recv_timeout(std::time::Duration::from_millis(1000)) {
                    Ok(Some(request)) => {
                        {
                            let display = display.lock().unwrap();
                            let buffer = &display.buffer();
                            for y in 0..HEIGHT as usize {
                                for x in 0..WIDTH as usize {
                                    let byte_idx = y * (WIDTH as usize / 8) + (x / 8);
                                    let bit = 7 - (x % 8);
                                    let pixel_value = (buffer[byte_idx] >> bit) & 1;
                                    image_buffer[y * WIDTH as usize + x] =
                                        if pixel_value == 1 { 255 } else { 0 };
                                }
                            }
                        }
                        {
                            writer.set_position(0);
                            let mut buf = [0u8; SIZE / 8];
                            for y in 0..HEIGHT as usize {
                                for x in 0..WIDTH as usize {
                                        let pixel_value = image_buffer[y * WIDTH as usize + x];
                                        if pixel_value > 0 {
                                            let byte_idx = y * (WIDTH as usize / 8) + (x / 8);
                                            let bit = 7 - (x % 8);
                                            buf[byte_idx] |= 1 << bit;
                                        }
                                    }
                            }
                            write_1bpp_bmp(&mut writer, &buf, WIDTH as u32, HEIGHT as u32)
                                .expect("Couldn't write BMP data");

                        }
                        let mut response =
                            tiny_http::Response::from_data(writer.get_ref().as_slice());
                        response.add_header(
                            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"image/bmp"[..])
                                .unwrap(),
                        );
                        if let Err(e) = request.respond(response) {
                            eprintln!("Error responding to request: {:?}", e);
                        }
                        writer.set_position(0);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Error receiving request: {:?}", e);
                    }
                }
            }
        })
    };

    match listener.join() {
        Ok(_) => eprintln!("Signal listener thread exited"),
        Err(e) => eprintln!("Signal listener thread panicked: {:?}", e),
    };
    match server.join() {
        Ok(_) => eprintln!("HTTP server thread exited"),
        Err(e) => eprintln!("HTTP server thread panicked: {:?}", e),
    };

    Ok(())
}
