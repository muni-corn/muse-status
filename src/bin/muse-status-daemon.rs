use muse_status::battery;
use muse_status::brightness;
use muse_status::daemon::Daemon;
use muse_status::date;
use muse_status::errors::*;
use muse_status::format;
use muse_status::format::blocks::Block;
use muse_status::network;
// use muse_status::mpris;
use muse_status::volume;
use muse_status::weather;
use std::env;

fn main() {
    // TODO parse from configuration file

    let formatter = match handle_args() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("couldn't parse arguments: {}", e);
            return;
        }
    };

    let daemon = Daemon::new("localhost:1612");

    let battery_block = battery::SmartBatteryBlock::new("BAT0", 30, 15);
    let brightness_block = brightness::BrightnessBlock::new("amdgpu_bl0");
    let date_block = date::DateBlock::new();
    let network_block = match network::NetworkBlock::new("wlo1") {
        Ok(n) => n,
        Err(e) => {
            eprintln!("couldn't create the network block: {}", e);
            return;
        }
    };

    // let mpris_block = mpris::MprisBlock::new();
    let volume_block = volume::VolumeBlock::new();
    let weather_block = weather::WeatherBlock::new();

    let primary_blocks: Vec<Box<dyn Block>> = vec![Box::new(date_block), Box::new(weather_block)];
    let secondary_blocks: Vec<Box<dyn Block>> = vec![
        Box::new(brightness_block),
        Box::new(volume_block),
        Box::new(network_block),
        Box::new(battery_block),
    ];
    let ternary_blocks: Vec<Box<dyn Block>> = Vec::new();

    match daemon.start(formatter, primary_blocks, secondary_blocks, ternary_blocks) {
        Ok(j) => {
            println!("the daemon is running");
            for handle in j {
                if let Err(e) = handle.join() {
                    eprintln!("couldn't join with thread: {:?}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("couldn't start daemon: {}", e);
        }
    }
}

/// Returns a Formatter by parsing arguments passed to the `muse-status-daemon` command, or an
/// Error if argument parsing failed.
fn handle_args() -> Result<format::Formatter, MuseStatusError> {
    let mut formatter: format::Formatter = Default::default();

    // must be a command if first (second, technically) argument doesn't start
    // with a dash. exit after command. otherwise, parse arguments
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(next) = args.next() {
            match arg.as_str() {
                "-p" | "--primary-color" => {
                    formatter.set_primary_color(&next)?;
                }
                "-s" | "--secondary-color" => {
                    formatter.set_secondary_color(&next)?;
                }
                "-f" | "--font" => {
                    formatter.set_text_font(&next);
                }
                "-i" | "--icon-font" => {
                    formatter.set_icon_font(&next);
                }
                "-m" | "--mode" => match next.as_str() {
                    "i3" => {
                        formatter.set_format_mode(format::Mode::JsonProtocol);
                    }
                    "lemon" => {
                        formatter.set_format_mode(format::Mode::Lemonbar);
                    }
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }
    }

    Ok(formatter)
}
