use muse_status::{
    battery, brightness, config, daemon::Daemon, date, format::blocks::Block, mpris, network,
    volume, weather,
};

fn main() {
    let mut env_args = std::env::args();
    let mut config_path = None;
    while let Some(arg) = env_args.next() {
        if arg == "--config" || arg == "-c" {
            config_path = Some(
                env_args
                    .next()
                    .unwrap_or_else(|| panic!("`{}` requires a value", arg)),
            );
        }
    }

    let config = if let Some(path) = config_path {
        config::Config::from_file(path).unwrap()
    } else {
        let path = config::default_config_path().unwrap();

        config::Config::from_file(path).unwrap()
    };

    let battery_block =
        battery::BatteryBlock::new(config.battery_config.clone());
    let brightness_block = brightness::BrightnessBlock::new(&config.brightness_id);
    let date_block = date::DateBlock::new();
    let network_block = match network::NetworkBlock::new(&config.network_interface_name) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("couldn't create network block: {}", e);
            return;
        }
    };
    let mpris_block = mpris::MprisBlock::new();
    let volume_block = volume::VolumeBlock::new();
    let weather_block =
        weather::WeatherBlock::new(config.weather_config.clone());

    let blocks: Vec<Box<dyn Block>> = vec![
        Box::new(date_block),
        Box::new(weather_block),
        Box::new(mpris_block),
        Box::new(brightness_block),
        Box::new(volume_block),
        Box::new(network_block),
        Box::new(battery_block),
    ];

    let daemon = Daemon::new(config);
    match daemon.start(blocks) {
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
