use beat_detector::{BeatInfo, StrategyKind};
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use yata::methods::EMA;
use yata::prelude::*;

pub fn list_audio_devices() {
    println!("Supported hosts:\n {:?}", cpal::ALL_HOSTS);
    let available_hosts = cpal::available_hosts();
    println!("Available hosts:\n {:?}", available_hosts);

    for host_id in available_hosts {
        println!("Host: {:?}", host_id.name());
        let host = cpal::host_from_id(host_id).unwrap();

        println!("Available audio devices:");
        for (i, device) in host.input_devices().unwrap().enumerate() {
            let name = device.name().unwrap_or(format!("Unknown device #{}", i));
            println!(" [{}] {}", i, name);

            // Input configs
            if let Ok(conf) = device.default_input_config() {
                println!("    Default input stream config:\n      {:?}", conf);
            }
            let input_configs = match device.supported_input_configs() {
                Ok(f) => f.collect(),
                Err(e) => {
                    println!("    Error getting supported input configs: {:?}", e);
                    Vec::new()
                }
            };
            if !input_configs.is_empty() {
                println!("    All supported input stream configs:");
                for (config_index, config) in input_configs.into_iter().enumerate() {
                    println!("      {}. {:?}", config_index, config);
                }
            }
        }
    }
}

pub struct BeatDetector {
    // This should be called on drop since we can't
    // call it from the event_loop
    // TODO: https://docs.rs/drop-move/latest/drop_move/
    exit_callback: Box<dyn FnOnce() + 'static>,
}

impl Default for BeatDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl BeatDetector {
    pub fn new() -> Self {
        BeatDetector {
            exit_callback: Box::new(|| ()),
        }
    }

    pub fn start_listening(
        &mut self,
        host_name: String,
        device_id: usize,
        sensitivity: u32,
        mut callback: impl FnMut((BeatInfo, f64)) + Sync + Send + 'static,
    ) {
        let recording = Arc::new(AtomicBool::new(true));
        let recording_cpy = recording.clone();

        let exit_callback = move || {
            recording_cpy.store(false, Ordering::SeqCst);
        };
        self.exit_callback = Box::new(exit_callback);

        let mut audio_device = None;
        for host_id in cpal::available_hosts() {
            if host_id.name() == host_name {
                let host = cpal::host_from_id(host_id).unwrap();
                for (i, device) in host.input_devices().unwrap().enumerate() {
                    if i == device_id {
                        audio_device = Some(device);
                    }
                }
            }
        }

        if let Some(device) = audio_device {
            let strategy = StrategyKind::Spectrum;

            let mut ema = EMA::new(32, &500.0).unwrap();

            let mut last_beat = Instant::now();
            let on_beat = move |info: BeatInfo| {
                // beat detectors relative_ms is unreliable, since we
                // are reading live audio data just use the current time
                let current_beat = Instant::now();
                let time_since_last_beat = (current_beat - last_beat).as_millis() as f64;
                let ema_result = ema.next(&time_since_last_beat);

                last_beat = current_beat;
                //        println!("EMA: {ema_result} BPM: {}", 60_000.0 / ema_result);
                //        println!("Beat detected: {:?}", info,);
                callback((info, 60_000.0 / ema_result));
            };
            let _ = beat_detector::record::start_listening(
                on_beat,
                Some(device),
                strategy,
                sensitivity as f32,
                recording,
            )
            .unwrap();
        }
    }
}
