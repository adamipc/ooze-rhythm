use crate::preset::{Preset, PresetName};
use chrono::Local;
use glium::glutin::event::{ElementState, Event, StartCause, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::Fullscreen;
use glium::{glutin, Surface};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::{Duration, Instant};

pub mod midi;
pub mod preset;
pub mod screenshot;
pub mod shader_pipeline;
pub mod slime_mould;

pub mod beat {
    use beat_detector::{BeatInfo, StrategyKind};
    use cpal::traits::{DeviceTrait, HostTrait};
    use cpal::Device;
    use std::collections::BTreeMap;
    use std::io::stdin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub struct BeatDetector {}

    impl BeatDetector {
        pub fn new() -> Self {
            BeatDetector {}
        }

        pub fn start_listening(
            &self,
            mut callback: impl FnMut(BeatInfo) + Sync + Send + 'static,
        ) -> impl FnOnce() -> () + Send + 'static {
            let recording = Arc::new(AtomicBool::new(true));
            let recording_cpy = recording.clone();

            let exit_callback = move || {
                recording_cpy.store(false, Ordering::SeqCst);
            };

            println!("Supported hosts:\n {:?}", cpal::ALL_HOSTS);
            let available_hosts = cpal::available_hosts();
            println!("Available hosts:\n {:?}", available_hosts);

            let mut host = cpal::default_host();
            for host_id in available_hosts {
                println!("Host: {:?}", host_id.name());
                if host_id.name() == "ASIO" {
                    //println!("Using Asio host");
                    //host = cpal::host_from_id(host_id).unwrap();
                }
            }

            let mut devs = BTreeMap::new();
            for (i, dev) in host.input_devices().unwrap().enumerate() {
                devs.insert(dev.name().unwrap_or(format!("Unknown device #{}", i)), dev);
            }
            if devs.is_empty() {
                println!("No audio input devices found");
                return exit_callback;
            }

            let dev = if devs.len() > 1 {
                Self::select_input_device(devs)
            } else {
                devs.into_iter().next().unwrap().1
            };

            // Input configs
            if let Ok(conf) = dev.default_input_config() {
                println!("    Default input stream config:\n      {:?}", conf);
            }
            let input_configs = match dev.supported_input_configs() {
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

            let strategy = StrategyKind::Spectrum;

            let on_beat = move |info: BeatInfo| {
                callback(info);
            };
            let _ = beat_detector::record::start_listening(on_beat, Some(dev), strategy, recording)
                .unwrap();

            exit_callback
        }

        fn select_input_device(devs: BTreeMap<String, Device>) -> Device {
            println!("Available audio devices:");
            for (i, (name, _)) in devs.iter().enumerate() {
                println!(" [{}] {}", i, name);
            }

            println!("Select audio device: input device number and enter:");
            let mut input = String::new();
            while stdin().read_line(&mut input).unwrap() == 0 {}
            let input = input
                .trim()
                .parse::<u8>()
                .expect("Input must be a valid number!");

            devs.into_iter()
                .enumerate()
                .filter(|(i, _)| *i == input as usize)
                .map(|(_i, (_name, dev))| dev)
                .take(1)
                .next()
                .unwrap()
        }
    }
}

use yata::methods::EMA;
use yata::prelude::*;

fn main() {
    let midi_channel = midi::MidiChannel::new();

    let beat_detector = beat::BeatDetector::new();

    let mut ema = EMA::new(32, &500.0).unwrap();

    let (beat_sender, beat_receiver) = sync_channel(64);

    let mut last_beat = Instant::now();
    let _ = beat_detector.start_listening(move |info| {
        // beat detectors relative_ms is unreliable, since we
        // are reading live audio data just use the current time
        let current_beat = Instant::now();
        let time_since_last_beat = (current_beat - last_beat).as_millis() as f64;
        let ema_result = ema.next(&time_since_last_beat);

        last_beat = current_beat;
        //        println!("EMA: {ema_result} BPM: {}", 60_000.0 / ema_result);
        //        println!("Beat detected: {:?}", info,);
        beat_sender.send(ema_result).unwrap();
    });

    // 1. The **winit::EventsLoop** for handling events.
    let event_loop = glutin::event_loop::EventLoop::new();

    let monitor = event_loop.primary_monitor().unwrap();
    let monitor_size = monitor.size();

    let (width, height) = (monitor_size.width, monitor_size.height);

    // 2. Parameters for building the Window.
    let wb = glutin::window::WindowBuilder::new()
        .with_inner_size(glutin::dpi::LogicalSize::new(width as f32, height as f32))
        .with_title("Hello world!")
        .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(
            event_loop.primary_monitor(),
        )));

    // 3. Parameters for building the OpenGL context.
    let cb = glutin::ContextBuilder::new().with_depth_buffer(24);

    // 4. Build the Display with the given window and OpenGL context parameters
    //    and register the window with the event_loop.
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    // Start out with a randomized preset
    let preset = rand::random();

    // Create our slime mould simulation
    let mut slime_mould = slime_mould::SlimeMould::new(&display, width, height, preset);

    let mut fullscreen = false;

    let mut screenshot_taker = screenshot::AsyncScreenshotTaker::new(5);

    let mut beat_preset: preset::Preset = rand::random();
    let mut non_beat_preset = preset;

    let mut u_time: f32 = 0.0;
    let mut u_time_takeover = false;
    let mut beat_start_time = u_time;
    start_loop(event_loop, move |events| {
        screenshot_taker.next_frame();

        let mut got_beat = false;
        for _ in beat_receiver.try_iter() {
            got_beat = true;
        }

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        slime_mould.draw(&mut target, &display, u_time);
        target.finish().unwrap();

        if !u_time_takeover {
            u_time += 0.02;
        }
        slime_mould.update();

        let mut action = Action::Continue;

        let mut enter_pressed = false;
        let mut escape_pressed = false;
        let mut r_pressed = false;
        let mut p_pressed = false;
        let mut backspace_pressed = false;
        let mut c_pressed = false;
        let mut s_pressed = false;

        let mut number_pressed = -1;

        for event in events {
            if let Event::WindowEvent { event, window_id } = event {
                if *window_id == display.gl_window().window().id() {
                    match event {
                        WindowEvent::CloseRequested => action = Action::Stop,
                        WindowEvent::KeyboardInput { input, .. } => {
                            if let ElementState::Pressed = input.state {
                                match input.virtual_keycode {
                                    Some(VirtualKeyCode::Escape) => escape_pressed = true,
                                    Some(VirtualKeyCode::Return) => enter_pressed = true,
                                    Some(VirtualKeyCode::R) => r_pressed = true,
                                    Some(VirtualKeyCode::P) => p_pressed = true,
                                    Some(VirtualKeyCode::C) => c_pressed = true,
                                    Some(VirtualKeyCode::S) => s_pressed = true,
                                    Some(VirtualKeyCode::Back) => backspace_pressed = true,
                                    _ => (),
                                }
                                // If we received a number
                                if input.scancode >= 2 && input.scancode <= 11 {
                                    number_pressed = ((input.scancode - 1) % 10) as i32;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        // Midi receiver
        for m in midi_channel.try_iter() {
            println!("{m:?}");
            match m {
                midi::Mpd218Message::PadPressed(pad, _velocity, _) => {
                    if pad <= 9 {
                        number_pressed = pad as i32;
                    } else {
                        match pad {
                            10 => c_pressed = true,
                            11 => p_pressed = true,
                            12 => r_pressed = true,
                            13 => beat_preset = rand::random(),
                            _ => (),
                        }
                    }
                }
                midi::Mpd218Message::KnobChanged(knob, value, _) => {
                    if knob == 0 {
                        u_time = value as f32 / 127.0;
                        //println!("value: {value} u_time: {u_time}");
                        u_time_takeover = true;
                    }
                }
                _ => (),
            }
        }

        // Random preset
        if r_pressed {
            slime_mould.transition_preset(slime_mould.get_preset(), rand::random(), u_time, 1.0);
            u_time_takeover = false;
        }

        // Regenerate points
        if p_pressed {
            slime_mould.reset_points(&display);
        }

        if c_pressed {
            // Clear the textures and buffers
            slime_mould.clear(&display);
        }

        if number_pressed >= 0 {
            // Load presets
            slime_mould.transition_preset(
                slime_mould.get_preset(),
                Preset::new(PresetName::from_u32(number_pressed as u32)),
                u_time,
                1.0,
            );
            slime_mould.reset_points(&display);
            u_time_takeover = false;
        }

        if s_pressed {
            slime_mould.save_preset();
        }

        // /*
        if got_beat {
            beat_start_time = u_time;
            non_beat_preset = slime_mould.get_preset();
            slime_mould.transition_preset(non_beat_preset, beat_preset, u_time, 0.2);
        } else {
            if beat_start_time > 0.0 {
                if (u_time - beat_start_time) > 0.2 {
                    slime_mould.transition_preset(beat_preset, non_beat_preset, u_time, 0.1);
                    beat_start_time = -1.0;
                }
            }
        } // */
        if backspace_pressed {
            println!("Taking screenshot...");
            screenshot_taker.take_screenshot(&display);
        }

        for image_data in screenshot_taker.pickup_screenshots() {
            thread::spawn(move || {
                let pixels = {
                    let mut v = Vec::with_capacity(image_data.data.len() * 4);
                    for (a, b, c, d) in image_data.data {
                        v.push(a);
                        v.push(b);
                        v.push(c);
                        v.push(d);
                    }
                    v
                };

                let image_buffer =
                    image::ImageBuffer::from_raw(image_data.width, image_data.height, pixels)
                        .unwrap();

                let image = image::DynamicImage::ImageRgba8(image_buffer).flipv();
                let image_name = format!(
                    "slime_mould-{}.png",
                    Local::now().format("%Y-%m-%d_%H%M%S%.f")
                );
                image.save(image_name).unwrap();
            });
        }

        if enter_pressed {
            if fullscreen {
                display.gl_window().window().set_fullscreen(None);
                fullscreen = false;
            } else {
                let monitor = display
                    .gl_window()
                    .window()
                    .available_monitors()
                    .next()
                    .unwrap();
                let fs = Fullscreen::Borderless(Some(monitor));
                display.gl_window().window().set_fullscreen(Some(fs));

                fullscreen = true;
            }
        }

        if escape_pressed {
            action = Action::Stop;
        }

        action
    });
}

pub enum Action {
    Stop,
    Continue,
}

pub fn start_loop<F>(event_loop: EventLoop<()>, mut callback: F) -> !
where
    F: 'static + FnMut(&Vec<Event<'_, ()>>) -> Action,
{
    let mut events_buffer = Vec::new();
    let mut next_frame_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let run_callback = match event.to_static() {
            Some(Event::NewEvents(cause)) => matches!(
                cause,
                StartCause::ResumeTimeReached { .. } | StartCause::Init
            ),
            Some(event) => {
                events_buffer.push(event);
                false
            }
            None => {
                // Ignore this event.
                false
            }
        };

        let action = if run_callback {
            let action = callback(&events_buffer);
            next_frame_time = Instant::now() + Duration::from_nanos(16666667) / 2;
            // TODO: Add back the old accumulator loop in some way

            events_buffer.clear();
            action
        } else {
            Action::Continue
        };

        match action {
            Action::Continue => {
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            }
            Action::Stop => *control_flow = ControlFlow::Exit,
        }
    })
}
