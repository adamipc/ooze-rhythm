use crate::beat;
use crate::midi;
use std::env;

use config::Config;

#[derive(Debug, Default, serde_derive::Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub audio_host_name: Option<String>,
    pub audio_device_id: Option<usize>,
    pub midi_device_id: Option<usize>,
    pub beat_sensitivity: Option<u32>,
}

pub fn get_config() -> AppConfig {
    let config = Config::builder()
        .add_source(config::File::with_name("Config"))
        .add_source(config::Environment::with_prefix("MOLD"))
        .build()
        .unwrap_or(Config::default());

    let mut app_config: AppConfig = config.try_deserialize().unwrap();

    let mut args = env::args();
    let _ = args.next();
    let mut arg = args.next();
    while let Some(argument) = arg {
        match &argument[..] {
            "--set-beat-sensitivity" => {
                let sensitivity = args.next().unwrap();
                app_config.beat_sensitivity = Some(sensitivity.parse().unwrap());
            }
            "--set-audio-device" => {
                let device_identifier = args.next().unwrap();
                // We allow passing on command line like "ASIO:3"
                let (host, device) = device_identifier.split_once(':').unwrap();
                app_config.audio_host_name = Some(host.to_string());
                app_config.audio_device_id = Some(device.parse().unwrap());
            }
            "--list-midi-devices" => {
                midi::list_midi_devices();
            }
            "--set-midi-device" => {
                let device_identifier = args.next().unwrap();
                app_config.midi_device_id = Some(device_identifier.parse().unwrap());
            }
            "--list-audio-devices" => {
                beat::list_audio_devices();
            }
            &_ => todo!("Unknown argument: {}", argument),
        }
        arg = args.next();
    }

    app_config
}
