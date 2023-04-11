extern crate glium;
extern crate image;

use glium::{glutin, implement_vertex, uniform, Surface};
use rand::Rng;
use std::cell::RefCell;

struct SlimeMould {
    width: u32,
    height: u32,
    target_texture: RefCell<glium::texture::Texture2d>,
    shader_pipeline: ShaderPipeline,
}

fn main() {
    // 1. The **winit::EventsLoop** for handling events.
    let event_loop = glutin::event_loop::EventLoop::new();

    // 2. Parameters for building the Window.
    let wb = glutin::window::WindowBuilder::new()
        .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0))
        .with_title("Hello world!");

    // 3. Parameters for building the OpenGL context.
    let cb = glutin::ContextBuilder::new().with_depth_buffer(24);

    // 4. Build the Display with the given window and OpenGL context parameters
    //    and register the window with the event_loop.
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let mut slime_mould = SlimeMould::new(&display, 1024, 768);

    // Loop forever until we receive `CloseRequested` event.
    event_loop.run(move |ev, _, control_flow| {
        let frame_start_time = std::time::Instant::now();

        // Check to see if we should exit before doing work
        match ev {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    // Request to exit
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                _ => return,
            },
            // Wait until we are ready to draw again
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => (),
        }

        // Draw stuff!
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        slime_mould.draw(&mut target, &display);
        target.finish().unwrap();
        // End draw stuff...

        // Attempt to display 60 frames per second.
        let next_frame_time = frame_start_time + std::time::Duration::from_nanos(16_666_667);

        // Request to wait until the frame time is up
        // QUESTION: what if this time has already passed? I assume this is a noop?
        if next_frame_time > std::time::Instant::now() {
            *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);
        }
    });
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

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            target_texture: RefCell::new(
                glium::texture::Texture2d::empty_with_format(
                    display,
                    glium::texture::UncompressedFloatFormat::F32F32F32F32,
                    glium::texture::MipmapsOption::NoMipmap,
                    width,
                    height,
                )
                .unwrap(),
            ),
            shader_pipeline: Self::create_shader_pipeline(display, width, height),
        }
    }

    fn create_shader_pipeline(display: &glium::Display, width: u32, height: u32) -> ShaderPipeline {
        // Shader 1
        let shader_1 = Self::get_shader_1(display);

        for uniform in shader_1.uniforms() {
            println!("{:?}", uniform);
        }
        for attribute in shader_1.attributes() {
            println!("{:?}", attribute);
        }

        // Shader 2
        let shader_2 = Self::get_shader_2(display);

        for uniform in shader_2.uniforms() {
            println!("{:?}", uniform);
        }
        for attribute in shader_2.attributes() {
            println!("{:?}", attribute);
        }

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

        let (buffer_a, buffer_b) = Self::get_initial_locations(display, u32::pow(2, 16));

        ShaderPipeline {
            buffer_a: RefCell::new(buffer_a),
            buffer_b: RefCell::new(buffer_b),
            vertex_buffer,
            shader_1,
            shader_2,
            u_texture0: RefCell::new(u_texture0),
            u_texture1: RefCell::new(u_texture1),
        }
    }

    fn get_shader_2(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 150
        attribute vec2 a_vertex;
        
        out vec4 loc; // location in clip space

        void main(void) {
            gl_Position = vec4(a_vertex, 0.0, 1.0);
            loc = gl_Position; // pass to frag shader
        }
    "#;

        let fragment_shader_src = r#"
        #version 150
        precision highp float;

        uniform sampler2D u_texture0; // Output of shader 1
        uniform sampler2D u_texture1; // Previous frame's output from shader 2

        uniform float u_time;
        uniform float u_fade_speed;
        uniform float u_blur_fraction;

        in vec4 loc; // from the vertex shader, used to compute texture locations

        // for blurring
        const float Directions = 8.0;
        const float Quality = 1.0; // 3 for snowflake
        const float Radius = 1.0/12000.0; // TODO pass in resolution
        float pixel_count = 1.0;

        void main() {
            // Convert the clip-space coordinates into texture space ones
            vec2 texcoord = vec2((loc.x+1.0)/2.0, (loc.y+1.0)/2.0);

            // Gaussian blur
            vec4 blurred = texture2D(u_texture1, texcoord); // previous frame sample
            for (float d = 0.0; d < 6.3; d += 6.3 / Directions) {
                for (float i = 1.0/Quality; i <= 1.0; i += 1.0/Quality) {
                    blurred += texture2D(u_texture1, texcoord+vec2(cos(d),sin(d))*Radius*i);
                    pixel_count += 1.0;
                }
            }
            blurred /= pixel_count;

            vec4 shader1_out = texture2D(u_texture0, texcoord); // Output of shader 1
            vec4 prev_frame = texture2D(u_texture1, texcoord); // Previous frame of shader 2

            // Modify how much blurring by mixing the blurred version with the original
            blurred = prev_frame*(1.0-u_blur_fraction) + blurred*u_blur_fraction;

            // The output color - adding shader 1 output to the blurred version of previous frame
            gl_FragColor = shader1_out + blurred*(1.0-u_fade_speed) - 0.0001;
        }
    "#;

        glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap()
    }

    fn get_shader_1(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 150
        precision highp float;

        in vec4 a_position; // The current position of the vertex
        
        uniform sampler2D u_texture1; // The previous frame's output from shader 2

        uniform float speed_multiplier;

        // Passed to fragment shader
        out vec4 v_color;

        // TODO: make these uniform inputs?
        const float random_steer_factor = 0.1;
        const float constant_steer_factor = 0.5;
        const float search_radius = 0.1;
        const float search_angle = 0.2;
        const float trail_strength = 0.2;
        const float vertex_radius = 1.0;

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
            direction += random_steer_factor*3.0*(rand(texcoord+tex_val.xy)-0.5);

            // Calculate current speed
            float speed = speed_multiplier * speed_var;

            // Read the underlying texture in three directions
            float sense_radius = search_radius;
            float sense_angle = search_angle;
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
            float steer_amount = constant_steer_factor + random_steer_factor * rand(texcoord+tex_val.xy);

            // Straight ahead
            if (sense_forward > sense_left && sense_forward > sense_right) {
                direction += 0.0;
            } else if (sense_forward < sense_left && sense_forward < sense_right) { // random
                direction += random_steer_factor*(rand(texcoord+tex_val.xy)-0.5);
            } else if (sense_right > sense_left) {
                direction -= steer_amount; // Turn right
            } else if (sense_right < sense_left) {
                direction += steer_amount; // Turn left
            }

            // Start calculating our new position
            float y_new = a_position.x;
            float x_new = a_position.x;

            // Hard coded bounce handling at edges
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

            // Update position based on direction
            y_new += speed*speed_multiplier*sin(direction);
            x_new += speed*speed_multiplier*cos(direction);

            // Set the color of this vert
            float r = 0.0;
            float g = 0.0;

            // hard coded color strategy for direction
            r = sin(direction);
            g = cos(direction);

            v_color = vec4(r, g, trail_strength, 1.0);

            // Send back the position and size
            gl_Position = vec4(x_new, y_new, speed_var/1000.0, 1.0+direction/1000.0);
            gl_PointSize = vertex_radius;
        }
    "#;

        let fragment_shader_src = r#"
        #version 150
        precision highp float;

        out vec4 v_color;

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

    fn get_initial_locations(
        display: &glium::Display,
        n: u32,
    ) -> (glium::VertexBuffer<Position>, glium::VertexBuffer<Position>) {
        let mut initial_locations = vec![Position::default(); n as usize];
        let initial_speed = 1.0;
        let speed_randomness = 0.1;

        let mut rng = rand::thread_rng();
        for i in 0..n {
            let x: f32 = rng.gen_range(-1.0..1.0);
            let y: f32 = rng.gen_range(-1.0..1.0);
            let speed: f32 = (rng.gen_range(0.0..1.00) * 0.01 * speed_randomness
                + 0.01 * initial_speed)
                / 1000.0;
            let direction: f32 = rng.gen_range(0.0..1.0);
            initial_locations[i as usize] = Position {
                a_position: [x, y, speed, direction],
            };
        }

        (
            glium::VertexBuffer::new(display, &initial_locations).unwrap(),
            glium::VertexBuffer::new(display, &initial_locations).unwrap(),
        )
    }

    pub fn draw(&self, frame: &mut impl glium::Surface, display: &glium::Display) {
        {
            let target_texture = self.target_texture.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            self.draw_1(&mut framebuffer, display);
        }

        {
            // Read pixels from target_texture to u_texture0
            let mut u_texture0 = self.shader_pipeline.u_texture0.borrow_mut();
            *u_texture0 = glium::texture::Texture2d::new(
                display,
                self.target_texture
                    .borrow()
                    .read_to_pixel_buffer()
                    .read_as_texture_2d::<glium::texture::RawImage2d<u8>>()
                    .unwrap(),
            )
            .unwrap();
        }

        self.draw_1(frame, display);

        {
            let target_texture = self.target_texture.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            self.draw_2(&mut framebuffer, display);
        }

        self.draw_2(frame, display);

        // Read pixels from target_texture to u_texture1
        let mut u_texture1 = self.shader_pipeline.u_texture1.borrow_mut();
        *u_texture1 = glium::texture::Texture2d::new(
            display,
            self.target_texture
                .borrow()
                .read_to_pixel_buffer()
                .read_as_texture_2d::<glium::texture::RawImage2d<u8>>()
                .unwrap(),
        )
        .unwrap();
    }

    pub fn draw_1(&self, frame: &mut impl glium::Surface, display: &glium::Display) {
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
            let uniforms = uniform! { u_texture1: u_texture1, speed_multiplier: 1.0f32 };

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
        // Swap buffers
        std::mem::swap(
            &mut self.shader_pipeline.buffer_a.borrow_mut(),
            &mut self.shader_pipeline.buffer_b.borrow_mut(),
        );
    }

    pub fn draw_2(&self, frame: &mut impl glium::Surface, display: &glium::Display) {
        let u_texture0 = &*self.shader_pipeline.u_texture0.borrow();
        let u_texture1 = &*self.shader_pipeline.u_texture1.borrow();
        let uniforms = uniform! {
            u_texture0: u_texture0,
            u_texture1: u_texture1,
            u_fade_speed: 0.07f32,
            u_blur_fraction: 1.0f32,
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
