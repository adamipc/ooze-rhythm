use crate::midi;
use crate::preset::PresetName;
use glium::glutin::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use std::sync::mpsc::TryIter;

pub enum PresetSlot {
    Primary,
    Secondary,
    Beat,
}

pub enum InputEvent {
    ToggleFullscreen,
    ToggleAutomation,
    RandomizePreset(PresetSlot),
    LoadPreset(PresetSlot, PresetName),
    UpdateBlendValue(f32),
    UpdateBeatTransitionTime(f32),
    StopEventLoop,
    DumpState,
    ClearTextures,
    ResetPoints,
    TakeScreenshot,
}

pub fn input_callback(
    events: &Vec<Event<'_, ()>>,
    midi_events: TryIter<'_, midi::Mpd218Message>,
    primary_window_id: glium::glutin::window::WindowId,
) -> Vec<InputEvent> {
    let mut input_events = Vec::new();

    for event in events {
        if let Event::WindowEvent { event, window_id } = event {
            if *window_id == primary_window_id {
                match event {
                    WindowEvent::CloseRequested => input_events.push(InputEvent::StopEventLoop),
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let ElementState::Pressed = input.state {
                            match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => {
                                    input_events.push(InputEvent::StopEventLoop)
                                }
                                Some(VirtualKeyCode::Return) => {
                                    input_events.push(InputEvent::ToggleFullscreen)
                                }
                                Some(VirtualKeyCode::R) => input_events
                                    .push(InputEvent::RandomizePreset(PresetSlot::Primary)),
                                Some(VirtualKeyCode::P) => {
                                    input_events.push(InputEvent::ResetPoints)
                                }
                                Some(VirtualKeyCode::C) => {
                                    input_events.push(InputEvent::ClearTextures)
                                }
                                Some(VirtualKeyCode::S) => input_events.push(InputEvent::DumpState),
                                Some(VirtualKeyCode::A) => {
                                    input_events.push(InputEvent::ToggleAutomation)
                                }
                                Some(VirtualKeyCode::Back) => {
                                    input_events.push(InputEvent::TakeScreenshot)
                                }
                                _ => (),
                            }
                            // If we received a number
                            if input.scancode >= 2 && input.scancode <= 11 {
                                input_events.push(InputEvent::LoadPreset(
                                    PresetSlot::Primary,
                                    PresetName::from_u32((input.scancode - 1) % 10),
                                ));
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    // Midi receiver
    for m in midi_events {
        println!("{m:?}");
        match m {
            midi::Mpd218Message::PadPressed(pad, _velocity, _) => {
                if pad <= 9 {
                    input_events.push(InputEvent::LoadPreset(
                        PresetSlot::Primary,
                        PresetName::from_u32(pad as u32),
                    ));
                } else if (16..=25).contains(&pad) {
                    input_events.push(InputEvent::LoadPreset(
                        PresetSlot::Secondary,
                        PresetName::from_u32((pad - 16) as u32),
                    ));
                } else if (32..=41).contains(&pad) {
                    input_events.push(InputEvent::LoadPreset(
                        PresetSlot::Beat,
                        PresetName::from_u32((pad - 32) as u32),
                    ));
                } else {
                    match pad {
                        10 => input_events.push(InputEvent::ClearTextures),
                        11 => input_events.push(InputEvent::ResetPoints),
                        12 => input_events.push(InputEvent::RandomizePreset(PresetSlot::Primary)),
                        13 => input_events.push(InputEvent::RandomizePreset(PresetSlot::Secondary)),
                        14 => input_events.push(InputEvent::RandomizePreset(PresetSlot::Beat)),
                        15 => input_events.push(InputEvent::ToggleAutomation),
                        _ => (),
                    }
                }
            }
            midi::Mpd218Message::KnobChanged(knob, value, _) => {
                if knob == 0 {
                    input_events.push(InputEvent::UpdateBlendValue(value as f32 / 127.0));
                }
                if knob == 1 {
                    input_events.push(InputEvent::UpdateBeatTransitionTime(
                        value as f32 / 127.0 * 0.5,
                    ));
                }
            }
            _ => (),
        }
    }

    input_events
}
