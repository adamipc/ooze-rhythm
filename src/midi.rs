use midir::{Ignore, MidiInput};
use std::sync::mpsc::sync_channel;

pub struct MidiChannel<T> {
    receiver: std::sync::mpsc::Receiver<T>,
}

impl<T> MidiChannel<T>
where
    T: std::convert::From<(u64, [u8; 3])> + std::marker::Send + 'static,
{
    pub fn new() -> Self {
        let receiver = Self::setup_midi_input();
        Self { receiver }
    }

    pub fn try_iter(&self) -> std::sync::mpsc::TryIter<T> {
        self.receiver.try_iter()
    }

    fn setup_midi_input() -> std::sync::mpsc::Receiver<T> {
        let (sender, receiver) = sync_channel(64);
        let mut midi_in = MidiInput::new("midir reading input").unwrap();
        midi_in.ignore(Ignore::None);

        let in_ports = midi_in.ports();
        let in_port = match in_ports.len() {
            0 => {
                println!("no midi input port found");
                return receiver;
            }
            1 => {
                println!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                &in_ports[0]
            }
            _ => &in_ports[in_ports.len() - 1],
        };

        println!("\nOpening connection");
        let in_port_name = midi_in.port_name(in_port).unwrap();

        let _conn_in = midi_in.connect(
            in_port,
            "midir-read-input",
            move |time, message, _| {
                let len = std::cmp::min(message.len(), MAX_MIDI);
                let mut data = [0; MAX_MIDI];
                data[..len].copy_from_slice(&message[..len]);
                sender.send((time, data).into()).unwrap();
            },
            (),
        );

        println!("Connection open, reading input from '{}'.", in_port_name);
        receiver
    }
}
const MAX_MIDI: usize = 3;

#[derive(Copy, Clone)]
struct MidiCopy {
    len: usize,
    data: [u8; MAX_MIDI],
    time: u64,
}

impl std::fmt::Debug for MidiCopy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Midi {{ time: {}, len: {}, data: {:?} }}",
            self.time,
            self.len,
            &self.data[..self.len]
        )
    }
}

// Pad 0-47, Knob 0-5
// Velocity 0-127
// Knob value 0-127
// Time u64
#[derive(Debug, Copy, Clone)]
pub enum Mpd218Message {
    PadPressed(u8, u8, u64),
    PadHeld(u8, u64),
    PadReleased(u8, u8, u64),
    KnobChanged(u8, u8, u64),
    Unknown([u8; MAX_MIDI], u64),
}

impl From<(u64, [u8; 3])> for Mpd218Message {
    fn from(tuple: (u64, [u8; 3])) -> Self {
        let (time, data) = tuple;
        // data[0] == 153 // pad pressed
        // data[0] == 217 // pad held
        // data[0] == 137 // pad released
        // data[0] == 176 // knob turned
        // data[1] for pads is 36-84
        // data[1] for knobs is 0-127
        // pad number is not passed when held so velocity is in data[1]
        // held data is only supplied for first pad held
        // and pad number is in last_pad
        // Knobs are 3,9, 12-27
        //
        match data[0] {
            153 => {
                let pad = data[1] - 36;
                let velocity = data[2];
                //println!("Pad {} pressed with velocity {}", pad, velocity);
                Mpd218Message::PadPressed(pad, velocity, time)
            }
            217 => {
                let velocity = data[1];
                //println!("Pad {} held with velocity {}", pad, velocity);
                Mpd218Message::PadHeld(velocity, time)
            }
            137 => {
                let pad = data[1] - 36;
                let velocity = data[2];
                //println!("Pad {} released with velocity {}", pad, velocity);
                Mpd218Message::PadReleased(pad, velocity, time)
            }
            176 => {
                let mut knob = data[1] - 3;
                if knob > 0 {
                    knob -= 5;
                }
                if knob > 1 {
                    knob -= 2;
                }
                let value = data[2];
                //println!("Knob {} value {}", knob, value);
                Mpd218Message::KnobChanged(knob, value, time)
            }
            _ => {
                println!("Unknown message: {:?}", data);
                Mpd218Message::Unknown(data, time)
            }
        }
    }
}
