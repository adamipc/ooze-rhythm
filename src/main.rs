///
/// Heavily inspired by (and code "borrowed" from): https://observablehq.com/@johnowhitaker/dotswarm-exploring-slime-mould-inspired-shaders
///
use crate::input::{InputEvent, PresetSlot};
use crate::preset::Preset;
use chrono::Local;
use glium::glutin::event::{Event, StartCause};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::Fullscreen;
use glium::{glutin, Surface};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::{Duration, Instant};

pub mod beat;
pub mod config;
pub mod input;
pub mod midi;
pub mod preset;
pub mod screenshot;
pub mod shader_pipeline;
pub mod slime_mould;

fn main() {
    let app_config = config::get_config();
    let midi_channel = midi::MidiChannel::new(app_config.midi_device_id);

    let mut beat_detector = beat::BeatDetector::new();

    let (beat_sender, beat_receiver) = sync_channel(64);

    if app_config.audio_host_name.is_some() && app_config.audio_device_id.is_some() {
        beat_detector.start_listening(
            app_config.audio_host_name.unwrap(),
            app_config.audio_device_id.unwrap(),
            move |(_, bpm)| {
                beat_sender.send(bpm).unwrap();
            },
        );
    };

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

    let mut fullscreen = false;
    let mut screenshot_taker = screenshot::AsyncScreenshotTaker::new(5);
    let primary_window_id = display.gl_window().window().id();

    // Create our slime mould simulation
    let mut slime_mould = slime_mould::SlimeMould::new(&display, width, height, rand::random());

    let mut beat_preset = rand::random();
    let mut non_beat_preset = slime_mould.get_preset();

    let mut u_time: f32 = 0.0;
    let mut beat_start_time = u_time;
    let mut blend_value = 0.0;
    let mut beat_transition_time = 0.2;
    let mut automate_presets = false;

    start_loop(event_loop, move |events| {
        screenshot_taker.next_frame();

        let mut got_beat = false;
        for _bpm in beat_receiver.try_iter() {
            got_beat = true;
            //println!("Got beat! BPM: {bpm:.2}");
        }

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        slime_mould.draw(&mut target, &display, u_time, blend_value);
        target.finish().unwrap();

        u_time += 0.02;

        let mut action = Action::Continue;

        for event in input::input_callback(events, midi_channel.try_iter(), primary_window_id) {
            match event {
                InputEvent::ToggleAutomation => automate_presets = !automate_presets,
                InputEvent::UpdateBlendValue(new_value) => blend_value = new_value,
                InputEvent::UpdateBeatTransitionTime(new_value) => beat_transition_time = new_value,
                InputEvent::RandomizePreset(slot) => {
                    let new_preset = rand::random();
                    match slot {
                        PresetSlot::Primary => {
                            slime_mould.transition_preset(new_preset, u_time, 1.0)
                        }
                        PresetSlot::Secondary => slime_mould.set_secondary_preset(new_preset),
                        PresetSlot::Beat => {
                            beat_preset = new_preset;
                        }
                    }
                }
                InputEvent::LoadPreset(slot, preset_name) => {
                    let new_preset = Preset::new(preset_name);
                    match slot {
                        PresetSlot::Primary => {
                            slime_mould.transition_preset(new_preset, u_time, 1.0);
                            slime_mould.reset_points();
                        }
                        PresetSlot::Secondary => slime_mould.set_secondary_preset(new_preset),
                        PresetSlot::Beat => {
                            beat_preset = new_preset;
                        }
                    }
                }
                InputEvent::ResetPoints => slime_mould.reset_points(),
                InputEvent::ClearTextures => slime_mould.clear(),
                // TODO: Make sure we dump all state that can effect the current visual
                // hopefully in such a way that it can easily be reloaded
                InputEvent::DumpState => slime_mould.save_preset(),
                InputEvent::TakeScreenshot => screenshot_taker.take_screenshot(&display),
                InputEvent::ToggleFullscreen => {
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
                InputEvent::StopEventLoop => action = Action::Stop,
            }
        }

        if automate_presets {
            beat_preset.update(u_time);
            slime_mould.update(u_time);
        }

        if got_beat {
            beat_start_time = u_time;
            non_beat_preset = slime_mould.get_preset();
            slime_mould.transition_preset(beat_preset, u_time, beat_transition_time);
        } else if beat_start_time > 0.0 && (u_time - beat_start_time) > beat_transition_time {
            slime_mould.transition_preset(non_beat_preset, u_time, beat_transition_time / 2.0);
            beat_start_time = -1.0;
        }

        for image_data in screenshot_taker.pickup_screenshots() {
            let image_name = format!(
                "slime_mould-{}.png",
                Local::now().format("%Y-%m-%d_%H%M%S%.f")
            );
            thread::spawn(move || {
                screenshot::save_screenshot(image_data, image_name);
            });
        }

        action
    });
}

pub enum Action {
    Stop,
    Continue,
}

pub fn start_loop<F>(event_loop: EventLoop<()>, mut callback: F)
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
            Action::Stop => {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
