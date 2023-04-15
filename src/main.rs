extern crate chrono;
extern crate glium;
extern crate image;
extern crate lerp;

use chrono::Local;
use glium::glutin::event::{ElementState, Event, StartCause, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::Fullscreen;
use glium::uniforms::{self, Sampler};
use glium::{glutin, implement_vertex, uniform, Surface};
use lerp::Lerp;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::cell::RefCell;
use std::thread;
use std::time::{Duration, Instant};

struct SlimeMould {
    target_texture0: RefCell<glium::texture::Texture2d>,
    target_texture1: RefCell<glium::texture::Texture2d>,
    shader_pipeline: ShaderPipeline,
    preset: Preset,
    width: u32,
    height: u32,
}

#[derive(Lerp, PartialEq, Debug, Copy, Clone)]
struct Preset {
    // Initial config
    #[lerp(skip)]
    number_of_points: u32,
    #[lerp(f32)]
    starting_arrangement: StartingArrangement,
    average_starting_speed: f32,
    starting_speed_spread: f32,

    // Vertex Shader Uniforms
    speed_multiplier: f32,
    point_size: f32,
    random_steer_factor: f32,
    constant_steer_factor: f32,
    trail_strength: f32,
    search_radius: f32,
    #[lerp(f32)]
    wall_strategy: WallStrategy,
    #[lerp(f32)]
    color_strategy: ColorStrategy,

    // Fragment Shader Uniforms
    fade_speed: f32,
    blurring: f32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum StartingArrangement {
    Origin = 0,
    Random = 1,
    Ring = 2,
}

impl Lerp<f32> for StartingArrangement {
    fn lerp(self, other: Self, t: f32) -> Self {
        let a = self as u32 as f32;
        let b = other as u32 as f32;
        let result = a.lerp(b, t);
        match result.round() as u32 {
            0 => StartingArrangement::Origin,
            1 => StartingArrangement::Random,
            2 => StartingArrangement::Ring,
            _ => panic!("Invalid StartingArrangement"),
        }
    }
}

impl Distribution<StartingArrangement> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StartingArrangement {
        match rng.gen_range(0..=2) {
            0 => StartingArrangement::Origin,
            1 => StartingArrangement::Random,
            _ => StartingArrangement::Ring,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum WallStrategy {
    Wrap = 0,
    Bounce = 1,
    None = 2,
}

impl Lerp<f32> for WallStrategy {
    fn lerp(self, other: Self, t: f32) -> Self {
        let a = self as u32 as f32;
        let b = other as u32 as f32;
        let result = a.lerp(b, t);
        match result.round() as u32 {
            0 => WallStrategy::Wrap,
            1 => WallStrategy::Bounce,
            2 => WallStrategy::None,
            _ => panic!("Invalid WallStrategy"),
        }
    }
}

impl Distribution<WallStrategy> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> WallStrategy {
        match rng.gen_range(0..=1) {
            0 => WallStrategy::Wrap,
            _ => WallStrategy::Bounce,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum ColorStrategy {
    Direction = 0,
    Speed = 1,
    Position = 2,
    Grey = 3,
}

impl Lerp<f32> for ColorStrategy {
    fn lerp(self, other: Self, t: f32) -> Self {
        let a = self as u32 as f32;
        let b = other as u32 as f32;
        let result = a.lerp(b, t);
        match result.round() as u32 {
            0 => ColorStrategy::Direction,
            1 => ColorStrategy::Speed,
            2 => ColorStrategy::Position,
            3 => ColorStrategy::Grey,
            _ => panic!("Invalid WallStrategy"),
        }
    }
}

impl Distribution<ColorStrategy> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ColorStrategy {
        match rng.gen_range(0..=3) {
            0 => ColorStrategy::Direction,
            1 => ColorStrategy::Speed,
            2 => ColorStrategy::Position,
            _ => ColorStrategy::Grey,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum PresetName {
    GreenSlime,
    CollapsingBubble,
    SlimeRing,
    ShiftingWeb,
    Waves,
    Flower,
    ChristmasChaos,
    Explode,
    Tartan,
    Globe,
}

impl Distribution<PresetName> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PresetName {
        match rng.gen_range(0..=9) {
            0 => PresetName::GreenSlime,
            1 => PresetName::CollapsingBubble,
            2 => PresetName::SlimeRing,
            3 => PresetName::ShiftingWeb,
            4 => PresetName::Waves,
            5 => PresetName::Flower,
            6 => PresetName::ChristmasChaos,
            7 => PresetName::Explode,
            8 => PresetName::Tartan,
            _ => PresetName::Globe,
        }
    }
}

impl PresetName {
    fn from_u32(value: u32) -> PresetName {
        match value {
            1 => PresetName::GreenSlime,
            2 => PresetName::CollapsingBubble,
            3 => PresetName::SlimeRing,
            4 => PresetName::ShiftingWeb,
            5 => PresetName::Waves,
            6 => PresetName::Flower,
            7 => PresetName::ChristmasChaos,
            8 => PresetName::Explode,
            9 => PresetName::Tartan,
            _ => PresetName::Globe,
        }
    }
}

impl Preset {
    pub fn new(preset_name: PresetName) -> Preset {
        println!("Creating preset: {:?}", preset_name);
        match preset_name {
            PresetName::GreenSlime => Preset {
                number_of_points: u32::pow(2, 20),
                starting_arrangement: StartingArrangement::Origin,
                average_starting_speed: 0.0,
                starting_speed_spread: 0.3,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.1,
                constant_steer_factor: 0.1,
                trail_strength: 0.01,
                search_radius: 0.01,
                wall_strategy: WallStrategy::Bounce,
                color_strategy: ColorStrategy::Position,

                fade_speed: 0.01,
                blurring: 1.0,
            },
            PresetName::CollapsingBubble => Preset {
                number_of_points: u32::pow(2, 13),
                starting_arrangement: StartingArrangement::Ring,
                average_starting_speed: 0.5,
                starting_speed_spread: 0.1,

                speed_multiplier: 1.0,
                point_size: 1.5,
                random_steer_factor: 0.1,
                constant_steer_factor: 0.5,
                trail_strength: 0.2,
                search_radius: 0.1,
                wall_strategy: WallStrategy::Wrap,
                color_strategy: ColorStrategy::Direction,

                fade_speed: 0.005,
                blurring: 1.0,
            },
            PresetName::SlimeRing => Preset {
                number_of_points: u32::pow(2, 20),
                starting_arrangement: StartingArrangement::Ring,
                average_starting_speed: 0.1,
                starting_speed_spread: 0.1,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.1,
                constant_steer_factor: 0.4,
                trail_strength: 0.2,
                search_radius: 0.01,
                wall_strategy: WallStrategy::Wrap,
                color_strategy: ColorStrategy::Grey,

                fade_speed: 0.05,
                blurring: 1.0,
            },
            PresetName::ShiftingWeb => Preset {
                number_of_points: u32::pow(2, 18),
                starting_arrangement: StartingArrangement::Ring,
                average_starting_speed: 1.0,
                starting_speed_spread: 0.1,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.1,
                constant_steer_factor: 0.45,
                trail_strength: 0.2,
                search_radius: 0.05,
                wall_strategy: WallStrategy::Wrap,
                color_strategy: ColorStrategy::Position,

                fade_speed: 0.07,
                blurring: 1.0,
            },
            PresetName::Waves => Preset {
                number_of_points: u32::pow(2, 18),
                starting_arrangement: StartingArrangement::Origin,
                average_starting_speed: 1.0,
                starting_speed_spread: 0.0,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.04,
                constant_steer_factor: 0.07,
                trail_strength: 0.1,
                search_radius: 0.01,
                wall_strategy: WallStrategy::Bounce,
                color_strategy: ColorStrategy::Direction,

                fade_speed: 0.04,
                blurring: 1.0,
            },
            PresetName::Flower => Preset {
                number_of_points: u32::pow(2, 14),
                starting_arrangement: StartingArrangement::Origin,
                average_starting_speed: 0.0,
                starting_speed_spread: 0.8,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.02,
                constant_steer_factor: 0.04,
                trail_strength: 0.5,
                search_radius: 0.1,
                wall_strategy: WallStrategy::Bounce,
                color_strategy: ColorStrategy::Direction,

                fade_speed: 0.02,
                blurring: 1.0,
            },
            PresetName::ChristmasChaos => Preset {
                number_of_points: u32::pow(2, 12),
                starting_arrangement: StartingArrangement::Random,
                average_starting_speed: 0.9,
                starting_speed_spread: 0.0,

                speed_multiplier: 1.0,
                point_size: 3.0,
                random_steer_factor: 0.1,
                constant_steer_factor: 4.0,
                trail_strength: 0.2,
                search_radius: 0.1,
                wall_strategy: WallStrategy::Wrap,
                color_strategy: ColorStrategy::Direction,

                fade_speed: 0.02,
                blurring: 1.0,
            },
            PresetName::Explode => Preset {
                number_of_points: u32::pow(2, 18),
                starting_arrangement: StartingArrangement::Origin,
                average_starting_speed: 0.4,
                starting_speed_spread: 0.3,

                speed_multiplier: 1.0,
                point_size: 2.0,
                random_steer_factor: 0.05,
                constant_steer_factor: 0.1,
                trail_strength: 0.2,
                search_radius: 0.1,
                wall_strategy: WallStrategy::None,
                color_strategy: ColorStrategy::Grey,

                fade_speed: 0.0,
                blurring: 0.0,
            },
            PresetName::Tartan => Preset {
                number_of_points: u32::pow(2, 18),
                starting_arrangement: StartingArrangement::Origin,
                average_starting_speed: 0.8,
                starting_speed_spread: 0.1,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.05,
                constant_steer_factor: 0.01,
                trail_strength: 0.01,
                search_radius: 0.1,
                wall_strategy: WallStrategy::Wrap,
                color_strategy: ColorStrategy::Direction,

                fade_speed: 0.01,
                blurring: 1.0,
            },
            PresetName::Globe => Preset {
                number_of_points: u32::pow(2, 16),
                starting_arrangement: StartingArrangement::Ring,
                average_starting_speed: 0.0,
                starting_speed_spread: 0.3,

                speed_multiplier: 1.0,
                point_size: 1.0,
                random_steer_factor: 0.005,
                constant_steer_factor: 0.0,
                trail_strength: 0.2,
                search_radius: 0.01,
                wall_strategy: WallStrategy::Bounce,
                color_strategy: ColorStrategy::Grey,

                fade_speed: 0.005,
                blurring: 1.0,
            },
        }
    }
}

impl Distribution<Preset> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Preset {
        Preset {
            number_of_points: u32::pow(2, rng.gen_range(10..=20)),
            starting_arrangement: rng.gen(),
            average_starting_speed: rng.gen_range(0.0..=2.0),
            starting_speed_spread: rng.gen_range(0.0..=1.0),
            speed_multiplier: rng.gen_range(0.0..=2.0),
            point_size: rng.gen_range(0.0..=5.0),
            random_steer_factor: rng.gen_range(0.0..=0.1),
            constant_steer_factor: rng.gen_range(0.0..=5.0),
            trail_strength: rng.gen_range(0.0..=1.0),
            search_radius: rng.gen_range(0.0..=0.1),
            wall_strategy: rng.gen(),
            color_strategy: rng.gen(),
            fade_speed: rng.gen_range(0.0..=0.1),
            blurring: rng.gen_range(0.0..=1.0),
        }
    }
}

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

    let mut slime_mould = SlimeMould::new(&display, width, height, preset);

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
                                    Some(VirtualKeyCode::Back) => backspace_pressed = true,
                                    Some(VirtualKeyCode::C) => c_pressed = true,
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

#[derive(Copy, Clone)]
struct Vertex {
    a_vertex: [f32; 2],
}

#[derive(Copy, Clone)]
struct Position {
    a_position: [f32; 4],
}

impl Default for Position {
    fn default() -> Self {
        Self {
            a_position: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

implement_vertex!(Vertex, a_vertex);
implement_vertex!(Position, a_position);

struct ShaderPipeline {
    shader_1: glium::Program,
    shader_2: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    buffer_a: RefCell<glium::VertexBuffer<Position>>,
    buffer_b: RefCell<glium::VertexBuffer<Position>>,
    u_texture0: RefCell<glium::texture::Texture2d>,
    u_texture1: RefCell<glium::texture::Texture2d>,
}

impl ShaderPipeline {
    pub fn new(display: &glium::Display, width: u32, height: u32, preset: Preset) -> Self {
        // Shader 1
        let shader_1 = Self::get_shader_1(display);

        // Shader 2
        let shader_2 = Self::get_shader_2(display);

        // Textures
        let u_texture0 = glium::texture::Texture2d::empty_with_format(
            display,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            width,
            height,
        )
        .unwrap();

        let u_texture1 = glium::texture::Texture2d::empty_with_format(
            display,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            width,
            height,
        )
        .unwrap();

        let vertex_buffer = glium::VertexBuffer::new(
            display,
            &[
                Vertex {
                    a_vertex: [-1.0, -1.0],
                },
                Vertex {
                    a_vertex: [1.0, -1.0],
                },
                Vertex {
                    a_vertex: [1.0, 1.0],
                },
                Vertex {
                    a_vertex: [-1.0, 1.0],
                },
            ],
        )
        .unwrap();

        let (buffer_a, buffer_b) = Self::get_initial_locations(
            display,
            preset.number_of_points,
            preset.starting_arrangement,
            preset.average_starting_speed,
            preset.starting_speed_spread,
        );

        Self {
            buffer_a: RefCell::new(buffer_a),
            buffer_b: RefCell::new(buffer_b),
            vertex_buffer,
            shader_1,
            shader_2,
            u_texture0: RefCell::new(u_texture0),
            u_texture1: RefCell::new(u_texture1),
        }
    }

    fn clear_textures(&mut self, display: &glium::Display, width: u32, height: u32) {
        let u_texture0 = RefCell::new(
            glium::texture::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap(),
        );

        let u_texture1 = RefCell::new(
            glium::texture::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap(),
        );

        u_texture0.swap(&self.u_texture0);
        u_texture1.swap(&self.u_texture1);
    }

    pub fn clear(&mut self, display: &glium::Display, width: u32, height: u32) {
        self.clear_textures(display, width, height);
    }

    fn get_initial_locations(
        display: &glium::Display,
        n: u32,
        starting_arrangement: StartingArrangement,
        initial_speed: f32,
        speed_randomness: f32,
    ) -> (glium::VertexBuffer<Position>, glium::VertexBuffer<Position>) {
        let mut initial_locations = vec![Position::default(); n as usize];

        let pi_times_2_over_n = std::f32::consts::PI * 2.0 / n as f32;

        let mut rng = rand::thread_rng();
        for i in 0..n {
            let speed = (rng.gen_range(0.0..1.00) * 0.01 * speed_randomness + 0.01 * initial_speed)
                / 1000.0;
            initial_locations[i as usize] = Position {
                a_position: match starting_arrangement {
                    StartingArrangement::Random => [
                        rng.gen_range(-1.0..1.0), // x
                        rng.gen_range(-1.0..1.0), // y
                        speed,                    // speed
                        rng.gen_range(0.0..1.0),  // direction
                    ],
                    StartingArrangement::Ring => {
                        let a = i as f32 * pi_times_2_over_n; // angle

                        let d = 0.7; // distance from center
                        [
                            a.sin() * d,                                      // x
                            -a.cos() * d,                                     // y
                            speed,                                            // speed
                            1.0 + (a + std::f32::consts::FRAC_PI_2) / 1000.0, // direction
                        ]
                    }
                    StartingArrangement::Origin => {
                        let a = i as f32 * pi_times_2_over_n; // angle
                        [
                            0.0,
                            0.0,
                            speed,
                            1.0 + (a + std::f32::consts::FRAC_PI_2) / 1000.0,
                        ]
                    }
                },
            };
        }

        (
            glium::VertexBuffer::new(display, &initial_locations).unwrap(),
            glium::VertexBuffer::new(display, &initial_locations).unwrap(),
        )
    }

    fn get_shader_1(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 140
        precision highp float;

        attribute vec4 a_position; // The current position of the vertex
        
        uniform sampler2D u_texture1; // The previous frame's output from shader 2

        uniform float u_time;

        uniform float u_speed_multiplier;
        uniform uint u_wall_strategy;
        uniform uint u_color_strategy;
        uniform float u_random_steer_factor;
        uniform float u_constant_steer_factor;
        uniform float u_search_radius;
        uniform float u_trail_strength;
        uniform float u_vertex_radius;
        uniform float u_search_angle;

        // Passed to fragment shader
        varying vec4 v_color;

        float rand(vec2 co) {
            return fract(sin(dot(co.xy, vec2(12.9898,78.233))) * 43758.5453);
        }

        void main() {
            // Coord in texture space
            vec2 texcoord = vec2((a_position.x+1.0)/2.0, (a_position.y+1.0)/2.0);
            vec4 tex_val = texture2D(u_texture1, texcoord);

            // Get speed and direction
            float direction = (a_position.w-1.0)*1000.0; // Stored it in the w component
            float speed_var = (a_position.z)*1000.0; // Stored in the z component

            // Add some randomness to the direction before anything else
            direction += u_random_steer_factor*3.0*(rand(texcoord+tex_val.xy)-0.5);

            // Calculate current speed
            float speed = u_speed_multiplier * speed_var;

            // Read the underlying texture in three directions
            float sense_radius = u_search_radius;
            float sense_angle = u_search_angle;
            float sense_left = texture2D(
                u_texture1,
                vec2(
                    texcoord.x+cos(direction+sense_angle)*sense_radius,
                    texcoord.y+sin(direction+sense_angle)*sense_radius
                )
            ).b;
            float sense_right = texture2D(
                u_texture1,
                vec2(
                    texcoord.x+cos(direction-sense_angle)*sense_radius,
                    texcoord.y+sin(direction-sense_angle)*sense_radius
                )
            ).b;
            float sense_forward = texture2D(
                u_texture1,
                vec2(
                    texcoord.x+cos(direction)*sense_radius,
                    texcoord.y+sin(direction)*sense_radius
                )
            ).b;

            // Update direction based on sensed values
            float steer_amount = u_constant_steer_factor + u_random_steer_factor * rand(texcoord+tex_val.xy);

            // Straight ahead
            if (sense_forward > sense_left && sense_forward > sense_right) {
                direction += 0.0;
            } else if (sense_forward < sense_left && sense_forward < sense_right) { // random
                direction += u_random_steer_factor*(rand(texcoord+tex_val.xy)-0.5);
            } else if (sense_right > sense_left) {
                direction -= steer_amount; // Turn right
            } else if (sense_right < sense_left) {
                direction += steer_amount; // Turn left
            }

            // Start calculating our new position
            float y_new = a_position.y;
            float x_new = a_position.x;

            // Wall strategy
            switch (u_wall_strategy) {
                case 0u:
                    // reverse direction if hitting wall
                    if (y_new + speed*sin(direction) > 0.90) {
                        float d = atan(sin(direction), cos(direction));
                        direction -= 2.0*d;
                    }
                    if (y_new + speed*sin(direction) < -0.90) {
                        float d = atan(sin(direction), cos(direction));
                        direction -= 2.0*d;
                    }
                    if (x_new + speed*cos(direction) > 0.90) {
                        float d = atan(cos(direction), sin(direction));
                        direction += 2.0*d;
                    }
                    if (x_new + speed*cos(direction) < -0.90) {
                        float d = atan(cos(direction), sin(direction));
                        direction += 2.0*d;
                    }
                    break;
                case 1u:
                    // wrap around
                    if (y_new > 0.99) { y_new = -0.99; }
                    if (y_new < -0.99) { y_new = 0.99; }

                    if (x_new > 0.99) { x_new = -0.99; }
                    if (x_new < -0.99) { x_new = 0.99; }
                    break;
                case 2u:
                    // None
                    break;
            }

            // Update position based on direction
            y_new += speed*u_speed_multiplier*sin(direction);
            x_new += speed*u_speed_multiplier*cos(direction);

            // Set the color of this vert
            float r = 0.0;
            float g = 0.0;

            // Color strategy
            switch (u_color_strategy) {
                case 0u:
                    r = sin(direction);
                    g = cos(direction);
                    break;
                case 1u:
                    r = speed_var*50.0;
                    g = r;
                    break;
                case 2u:
                    r = abs(y_new)/2.0 + 0.5;
                    g = abs(x_new)/2.0 + 0.5;
                    break;
                case 3u:
                    r = u_trail_strength;
                    g = r;
                    break;
            }

            v_color = vec4(r, g, u_trail_strength, 1.0);

            // Send back the position and size
            gl_Position = vec4(x_new, y_new, speed_var/1000.0, 1.0+direction/1000.0);
            gl_PointSize = u_vertex_radius;
        }
    "#;

        let fragment_shader_src = r#"
        #version 140
        precision highp float;

        varying vec4 v_color;

        void main() {
            gl_FragColor = v_color;
        }
    "#;

        glium::Program::new(
            display,
            glium::program::ProgramCreationInput::SourceCode {
                vertex_shader: vertex_shader_src,
                fragment_shader: fragment_shader_src,
                geometry_shader: None,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                transform_feedback_varyings: Some((
                    vec!["gl_Position".to_string()],
                    glium::program::TransformFeedbackMode::Separate,
                )),
                outputs_srgb: false,
                uses_point_size: true,
            },
        )
        .unwrap()
    }

    fn get_shader_2(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 140
        attribute vec2 a_vertex;
        
        varying vec4 loc; // location in clip space
        uniform float u_time;

        void main(void) {
            gl_Position = vec4(a_vertex.x, a_vertex.y, 0.0, 1.0);
            loc = gl_Position; // pass to frag shader
        }
    "#;

        let fragment_shader_src = r#"
            #version 140
            precision highp float;
            uniform sampler2D u_texture0; // A texture input - the output of shader 1
            uniform sampler2D u_texture1; // A texture input - the previous frame's output from shader 2
            uniform float u_fade_speed; // TODO
            uniform float u_blur_fraction; // TODO

            uniform float u_time;

            varying vec4 loc; // from the vertex shader, used to compute texture locations

            // For blurring
            const float Directions = 8.0;
            const float Quality = 1.0; // 3 for snowflake
            const float Radius = 1.0/1200.0; // TODO pass in resolution
            float pixelCount = 1.0;

            void main() {

              // Convert the clip-space coordinates into texture space ones
              vec2 texcoord = vec2((loc.x+1.0)/2.0, (loc.y+1.0)/2.0); 
              
              // Gaussian Blur 
              vec4 blurred = texture2D(u_texture1, texcoord); // sample the previous frame    
              for( float d=0.0; d<6.3; d+=6.3/Directions){
                  for(float i=1.0/Quality; i<=1.0; i+=1.0/Quality){
                    blurred += texture2D(u_texture1, texcoord+vec2(cos(d),sin(d))*Radius*i); 		
                    pixelCount += 1.0;
                   }
              }
              blurred /= pixelCount;      
              
              vec4 shader1_out = texture2D(u_texture0, texcoord); // The output of shader 1
              vec4 prev_frame = texture2D(u_texture1, texcoord); // The output of shader 2 (previous frame)

              // Modify how much blurring by mixing the blurred version with the original
              blurred = prev_frame*(1.0-u_blur_fraction) + blurred*u_blur_fraction;
              
              // The output colour - adding the shader 1 output to the blurred version of the previous frame
              gl_FragColor = shader1_out + blurred*(1.0-u_fade_speed) - 0.0001;
            }
            "#;

        glium::Program::new(
            display,
            glium::program::ProgramCreationInput::SourceCode {
                vertex_shader: vertex_shader_src,
                fragment_shader: fragment_shader_src,
                geometry_shader: None,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                transform_feedback_varyings: None,
                outputs_srgb: false,
                uses_point_size: false,
            },
        )
        .unwrap()
    }

    pub fn reset_points(
        &mut self,
        display: &glium::Display,
        n: u32,
        starting_arrangement: StartingArrangement,
        initial_speed: f32,
        speed_randomness: f32,
    ) {
        let (buffer_a, buffer_b) = Self::get_initial_locations(
            display,
            n,
            starting_arrangement,
            initial_speed,
            speed_randomness,
        );

        let (buffer_a, buffer_b) = (RefCell::new(buffer_a), RefCell::new(buffer_b));

        self.buffer_a.swap(&buffer_a);
        self.buffer_b.swap(&buffer_b);
    }
}

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32, preset: Preset) -> Self {
        Self {
            target_texture0: RefCell::new(
                glium::texture::Texture2d::empty_with_format(
                    display,
                    glium::texture::UncompressedFloatFormat::F32F32F32F32,
                    glium::texture::MipmapsOption::NoMipmap,
                    width,
                    height,
                )
                .unwrap(),
            ),
            target_texture1: RefCell::new(
                glium::texture::Texture2d::empty_with_format(
                    display,
                    glium::texture::UncompressedFloatFormat::F32F32F32F32,
                    glium::texture::MipmapsOption::NoMipmap,
                    width,
                    height,
                )
                .unwrap(),
            ),
            shader_pipeline: ShaderPipeline::new(display, width, height, preset),
            preset,
            width,
            height,
        }
    }

    pub fn clear(&mut self, display: &glium::Display) {
        self.shader_pipeline.clear(display, self.width, self.height);
    }

    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = preset;
    }

    pub fn reset_points(&mut self, display: &glium::Display) {
        self.shader_pipeline.reset_points(
            display,
            self.preset.number_of_points,
            self.preset.starting_arrangement,
            self.preset.average_starting_speed,
            self.preset.starting_speed_spread,
        );
    }

    pub fn update(&mut self) {}

    pub fn draw(&self, frame: &mut impl glium::Surface, display: &glium::Display, u_time: f32) {
        {
            let target_texture = self.target_texture0.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
            self.draw_1(&mut framebuffer, display, u_time);
        }

        {
            // Swap target_texture with u_texture0
            std::mem::swap(
                &mut *self.target_texture0.borrow_mut(),
                &mut *self.shader_pipeline.u_texture0.borrow_mut(),
            );
        }

        self.shader_pipeline
            .buffer_a
            .swap(&self.shader_pipeline.buffer_b);

        frame.clear_color(0.0, 0.0, 0.0, 1.0);

        //self.draw_1(frame, display, u_time);

        {
            let target_texture = self.target_texture1.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
            self.draw_2(&mut framebuffer, display, u_time);
        }

        self.draw_2(frame, display, u_time);

        {
            // Swap target_texture with u_texture1
            std::mem::swap(
                &mut *self.target_texture1.borrow_mut(),
                &mut *self.shader_pipeline.u_texture1.borrow_mut(),
            );
        }
    }

    pub fn draw_1(&self, frame: &mut impl glium::Surface, display: &glium::Display, u_time: f32) {
        {
            let mut buffer_b = self.shader_pipeline.buffer_b.borrow_mut();
            let session = glium::vertex::TransformFeedbackSession::new(
                display,
                &self.shader_pipeline.shader_1,
                &mut buffer_b,
            )
            .unwrap();

            let draw_parameters = Self::get_draw_parameters_shader_1(&session);

            let u_texture1 = &*self.shader_pipeline.u_texture1.borrow();
            let uniforms = uniform! {
                u_texture1: Sampler::new(u_texture1).wrap_function(uniforms::SamplerWrapFunction::Repeat),
                u_speed_multiplier: self.preset.speed_multiplier,
                u_wall_strategy: self.preset.wall_strategy as u8,
                u_color_strategy: self.preset.color_strategy as u8,
                u_random_steer_factor: self.preset.random_steer_factor,
                u_constant_steer_factor: self.preset.constant_steer_factor,
                u_search_radius: self.preset.search_radius,
                u_trail_strength: self.preset.trail_strength,
                u_vertex_radius: self.preset.point_size,
                u_search_angle: 0.2f32,
                u_time: u_time,
            };

            // Draw shader_1 to the frame
            frame
                .draw(
                    &*self.shader_pipeline.buffer_a.borrow(),
                    glium::index::NoIndices(glium::index::PrimitiveType::Points),
                    &self.shader_pipeline.shader_1,
                    &uniforms,
                    &draw_parameters,
                )
                .unwrap();
        }
    }

    pub fn draw_2(&self, frame: &mut impl glium::Surface, _display: &glium::Display, u_time: f32) {
        let u_texture0 = &*self.shader_pipeline.u_texture0.borrow();
        let u_texture1 = &*self.shader_pipeline.u_texture1.borrow();
        let uniforms = uniform! {
            u_texture0: Sampler::new(u_texture0).wrap_function(uniforms::SamplerWrapFunction::Repeat),
            u_texture1: Sampler::new(u_texture1).wrap_function(uniforms::SamplerWrapFunction::Repeat),
            u_fade_speed: self.preset.fade_speed,
            u_blur_fraction: self.preset.blurring,
            u_time: u_time,
        };
        // Draw the results of shader_2 to the screen
        frame
            .draw(
                &self.shader_pipeline.vertex_buffer,
                glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan),
                &self.shader_pipeline.shader_2,
                &uniforms,
                &Self::get_draw_parameters_shader_2(),
            )
            .unwrap(); // */
    }

    fn get_draw_parameters_shader_2<'b>() -> glium::DrawParameters<'b> {
        glium::DrawParameters {
            ..Default::default()
        }
    }
    fn get_draw_parameters_shader_1<'b>(
        session: &'b glium::vertex::TransformFeedbackSession,
    ) -> glium::DrawParameters<'b> {
        glium::DrawParameters {
            transform_feedback: Some(session),
            ..Default::default()
        }
    }
}
