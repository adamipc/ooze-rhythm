extern crate glium;
extern crate image;

use glium::{glutin, implement_vertex, uniform, Surface, Vertex};
use std::io::Cursor;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

struct SlimeMould {
    width: u32,
    height: u32,
    target_texture: glium::texture::Texture2d,
    u_texture0: glium::texture::Texture2d,
    u_texture1: glium::texture::Texture2d,
    shader_1: glium::Program,
    shader_2: glium::Program,
    display_texture: glium::texture::Texture2d,
    shader_pipeline: ShaderPipeline,
}

implement_vertex!(Vertex, position, normal, tex_coords);

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

    let slime_mould = SlimeMould::new(&display, 1024, 768);

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
        target.clear_color_and_depth((0.1, 0.7, 0.5, 1.0), 1.0);
        slime_mould.draw(&mut target);
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

struct ShaderPipeline {
    shader_1: glium::Program,
    shader_2: glium::Program,
    u_texture0: glium::texture::Texture2d,
    u_texture1: glium::texture::Texture2d,
}

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            target_texture: glium::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap(),
            u_texture0: glium::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap(),
            u_texture1: glium::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap(),
            display_texture: Self::load_texture(display),
            shader_pipeline: Self::create_shader_pipeline(display),
        }
    }

    fn create_shader_pipeline(display: &glium::Display) -> ShaderPipeline {
        // Shader 1
        let shader_1 = Self::get_shader_1(display);

        // Attributes + Uniforms
        let a_position_1 = shader_1.get_attrib("a_position").unwrap();
        let u_time_1 = shader_1.get_uniform("u_time").unwrap();
        let u_speed_multiplier_1 = shader_1.get_uniform("u_speed_multiplier").unwrap();
        let u_texture0_1 = shader_1.get_uniform("u_texture0").unwrap();
        let u_texture1_1 = shader_1.get_uniform("u_texture1").unwrap();

        // Shader 2
        let shader_2 = Self::get_shader_2(display);

        // Attributes + Uniforms
        let a_vertex_2 = shader_2.get_attrib("a_vertex").unwrap();
        let u_time_2 = shader_2.get_uniform("u_time").unwrap();
        let u_texture0_2 = shader_2.get_uniform("u_texture0").unwrap();
        let u_texture1_2 = shader_2.get_uniform("u_texture1").unwrap();
        let u_fade_speed = shader_2.get_uniform("u_fade_speed").unwrap();
        let u_blur_fraction = shader_2.get_uniform("u_blur_fraction").unwrap();

        // Change texture units of shader 2 to use same as shader 1
        u_texture0_2 = 0;
        u_texture1_2 = 1;

        ShaderPipeline {
            shader_1,
            shader_2,
            u_texture0_1: u_texture0_1,
            u_texture1_1,
        }
    }

    fn get_shader_2(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        attribute vec2 a_vertex;
        
        varying vec4 loc; // location in clip space

        void main(void) {
            gl_Position = vec4(a_vertex, 0.0, 1.0);
            loc = gl_position; // pass to frag shader
        }
    "#;

        let fragment_shader_src = r#"
        precision highp float;

        uniform sampler2D u_texture0; // Output of shader 1
        uniform sampler2D u_texture1; // Previous frame's output from shader 2

        uniform float u_time;
        uniform float u_fade_speed;
        uniform float u_blur_fraction;

        varying vec4 loc; // from the vertex shader, used to compute texture locations

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
        precision highp float;

        attribute vec4 a_position; // The current position of the vertex
        
        uniform sampler2D u_texture0; // The previous frame's output from shader 1
        uniform sampler2D u_texture1; // The previous frame's output from shader 2

        uniform float speed_multipler;

        // Passed to fragment shader
        varying vec4 v_color;

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
            float speed_var = (a.position.z)*1000.0; // Stored in the z component

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
            } else if (sense_forward < sense_left && sense_foward < sense_right) { // random
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
            y_new += speed*speed_multipler*sin(direction);
            x_new += speed*speed_multipler*cos(direction);

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
        precision highp float;

        varying vec4 v_color;

        void main() {
            gl_FragColor = v_color;
        }
    "#;

        glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap()
    }

    pub fn draw(&self, frame: &mut glium::Frame) {
        let params = Self::get_draw_parameters();
        let uniforms = self.get_uniforms(frame);

        self.display_texture
            .as_surface()
            .fill(frame, glium::uniforms::MagnifySamplerFilter::Linear);
    }
    fn get_draw_parameters<'b>() -> glium::DrawParameters<'b> {
        glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        }
    }
    fn get_uniforms<'b>(&'b self, frame: &glium::Frame) -> impl 'b + glium::uniforms::Uniforms {
        uniform! {}
    }

    fn load_normal<'b>(display: &glium::Display) -> glium::texture::Texture2d {
        let image = image::load(
            Cursor::new(&include_bytes!("..\\assets\\textures\\tuto-14-normal.png")),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();

        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        glium::texture::Texture2d::new(display, image).unwrap()
    }

    fn load_texture<'b>(display: &glium::Display) -> glium::texture::Texture2d {
        let image = image::load(
            Cursor::new(&include_bytes!("..\\assets\\textures\\tuto-14-diffuse.jpg")),
            image::ImageFormat::Jpeg,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();

        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        glium::texture::Texture2d::new(display, image).unwrap()
    }
}

fn get_view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [
        up[1] * f[2] - up[2] * f[1],
        up[2] * f[0] - up[0] * f[2],
        up[0] * f[1] - up[1] * f[0],
    ];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [
        f[1] * s_norm[2] - f[2] * s_norm[1],
        f[2] * s_norm[0] - f[0] * s_norm[2],
        f[0] * s_norm[1] - f[1] * s_norm[0],
    ];

    let p = [
        -position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
        -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
        -position[0] * f[0] - position[1] * f[1] - position[2] * f[2],
    ];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0f32],
    ]
}

type PerspectiveMatrix = [[f32; 4]; 4];
fn get_perspective_matrix(frame: &glium::Frame) -> PerspectiveMatrix {
    let (width, height) = frame.get_dimensions();
    let aspect_ratio = height as f32 / width as f32;

    let fov: f32 = std::f32::consts::PI / 3.0;
    let zfar = 1024.0;
    let znear = 0.1;

    let f = 1.0 / (fov / 2.0).tan();

    [
        [f * aspect_ratio, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
        [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
    ]
}
