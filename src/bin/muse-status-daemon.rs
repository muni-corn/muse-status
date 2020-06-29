use muse_status::battery;
use muse_status::brightness;
use muse_status::daemon::Daemon;
use muse_status::date;
use muse_status::format::blocks::Block;
use muse_status::mpris;
use muse_status::network;
use muse_status::volume;
use muse_status::weather;
use std::env;

fn main() {
    // TODO parse from configuration file

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
    let mpris_block = mpris::MprisBlock::new();

    let volume_block = volume::VolumeBlock::new();
    let weather_block = weather::WeatherBlock::new();

    let primary_blocks: Vec<Box<dyn Block>> = vec![
        Box::new(date_block),
        Box::new(weather_block),
        Box::new(mpris_block),
    ];
    let secondary_blocks: Vec<Box<dyn Block>> = vec![
        Box::new(brightness_block),
        Box::new(volume_block),
        Box::new(network_block),
        Box::new(battery_block),
    ];
    let ternary_blocks: Vec<Box<dyn Block>> = Vec::new();

    let mut daemon = Daemon::new("localhost:1612");
    if let Err(e) = daemon.handle_flags(&env::args().skip(1).collect::<Vec<String>>()) {
        eprintln!("couldn't parse arguments: {}", e)
    }

    match daemon.start(primary_blocks, secondary_blocks, ternary_blocks) {
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
