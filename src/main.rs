extern crate chrono;
extern crate glium;
extern crate image;
extern crate lerp;

use crate::preset::{Preset, PresetName};
use chrono::Local;
use glium::glutin::event::{ElementState, Event, StartCause, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::Fullscreen;
use glium::{glutin, Surface};
use std::thread;
use std::time::{Duration, Instant};

pub mod preset;
pub mod shader_pipeline;
pub mod slime_mould;

fn main() {
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

    let preset = rand::random();

    let mut slime_mould = slime_mould::SlimeMould::new(&display, width, height, preset);

    let mut fullscreen = false;

    let mut screenshot_taker = screenshot::AsyncScreenshotTaker::new(5);

    let mut u_time: f32 = 0.0;
    start_loop(event_loop, move |events| {
        screenshot_taker.next_frame();

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        slime_mould.draw(&mut target, &display, u_time);
        target.finish().unwrap();

        u_time += 0.0001;
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

        if c_pressed {
            // Clear the textures and buffers
            slime_mould.clear(&display);
        }

        if number_pressed >= 0 {
            // Load presets
            slime_mould.set_preset(Preset::new(PresetName::from_u32(number_pressed as u32)));
            slime_mould.reset_points(&display);
        }

        if s_pressed {
            slime_mould.save_preset();
        }

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

        // Random preset
        if r_pressed {
            slime_mould.set_preset(rand::random());
        }

        // Regenerate points
        if p_pressed {
            slime_mould.reset_points(&display);
        }

        action
    });
}

mod screenshot {
    use glium::Surface;
    use std::borrow::Cow;
    use std::collections::VecDeque;
    use std::vec::Vec;

    // Container that holds image data as vector of (u8, u8, u8, u8).
    // This is used to take data from PixelBuffer and move it to another thread
    // with minimum conversions done on main thread.
    pub struct RGBAImageData {
        pub data: Vec<(u8, u8, u8, u8)>,
        pub width: u32,
        pub height: u32,
    }

    impl glium::texture::Texture2dDataSink<(u8, u8, u8, u8)> for RGBAImageData {
        fn from_raw(data: Cow<'_, [(u8, u8, u8, u8)]>, width: u32, height: u32) -> Self {
            RGBAImageData {
                data: data.into_owned(),
                width,
                height,
            }
        }
    }

    struct AsyncScreenshotTask {
        pub target_frame: u64,
        pub pixel_buffer: glium::texture::pixel_buffer::PixelBuffer<(u8, u8, u8, u8)>,
    }

    impl AsyncScreenshotTask {
        fn new(facade: &dyn glium::backend::Facade, target_frame: u64) -> Self {
            // Get information about current framebuffer
            let dimensions = facade.get_context().get_framebuffer_dimensions();
            let rect = glium::Rect {
                left: 0,
                bottom: 0,
                width: dimensions.0,
                height: dimensions.1,
            };
            let blit_target = glium::BlitTarget {
                left: 0,
                bottom: 0,
                width: dimensions.0 as i32,
                height: dimensions.1 as i32,
            };

            // Create temporary texture and blit the front buffer to it
            let texture =
                glium::texture::Texture2d::empty(facade, dimensions.0, dimensions.1).unwrap();
            let framebuffer = glium::framebuffer::SimpleFrameBuffer::new(facade, &texture).unwrap();
            framebuffer.blit_from_frame(
                &rect,
                &blit_target,
                glium::uniforms::MagnifySamplerFilter::Nearest,
            );

            // Read the texture into new pixel buffer
            let pixel_buffer = texture.read_to_pixel_buffer();

            AsyncScreenshotTask {
                target_frame,
                pixel_buffer,
            }
        }

        fn read_image_data(self) -> RGBAImageData {
            self.pixel_buffer.read_as_texture_2d().unwrap()
        }
    }

    pub struct ScreenshotIterator<'a>(&'a mut AsyncScreenshotTaker);

    impl<'a> Iterator for ScreenshotIterator<'a> {
        type Item = RGBAImageData;

        fn next(&mut self) -> Option<RGBAImageData> {
            if self
                .0
                .screenshot_tasks
                .front()
                .map(|task| task.target_frame)
                == Some(self.0.frame)
            {
                let task = self.0.screenshot_tasks.pop_front().unwrap();
                Some(task.read_image_data())
            } else {
                None
            }
        }
    }

    pub struct AsyncScreenshotTaker {
        screenshot_delay: u64,
        frame: u64,
        screenshot_tasks: VecDeque<AsyncScreenshotTask>,
    }

    impl AsyncScreenshotTaker {
        pub fn new(screenshot_delay: u64) -> Self {
            AsyncScreenshotTaker {
                screenshot_delay,
                frame: 0,
                screenshot_tasks: VecDeque::new(),
            }
        }

        pub fn next_frame(&mut self) {
            self.frame += 1;
        }

        pub fn pickup_screenshots(&mut self) -> ScreenshotIterator<'_> {
            ScreenshotIterator(self)
        }

        pub fn take_screenshot(&mut self, facade: &dyn glium::backend::Facade) {
            self.screenshot_tasks.push_back(AsyncScreenshotTask::new(
                facade,
                self.frame + self.screenshot_delay,
            ));
        }
    }
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
