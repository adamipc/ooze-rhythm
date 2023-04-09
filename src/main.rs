extern crate glium;
extern crate image;

mod teapot;

use glium::{glutin, implement_vertex, uniform, Surface};
use std::io::Cursor;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

struct Wall {
    positions: glium::VertexBuffer<Vertex>,
    indices: glium::index::NoIndices,
    diffuse_texture: glium::texture::SrgbTexture2d,
    normal_texture: glium::texture::Texture2d,
    shader_program: glium::Program,
}

struct Teapot {
    positions: glium::VertexBuffer<teapot::Vertex>,
    normals: glium::VertexBuffer<teapot::Normal>,
    indices: glium::IndexBuffer<u16>,
    shader_program: glium::Program,
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

    //let teapot = Teapot::new(&display);
    let wall = Wall::new(&display);

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
        //teapot.draw(&mut target);
        wall.draw(&mut target);
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

impl Wall {
    pub fn new(display: &glium::Display) -> Self {
        let shape = glium::vertex::VertexBuffer::new(
            display,
            &[
                Vertex {
                    position: [-1.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    tex_coords: [0.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    tex_coords: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    tex_coords: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, -1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    tex_coords: [1.0, 0.0],
                },
            ],
        )
        .unwrap();
        Self {
            positions: shape,
            indices: glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip),
            diffuse_texture: Self::load_texture(display),
            normal_texture: Self::load_normal(display),
            shader_program: Self::get_shader(display),
        }
    }
    fn get_shader(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 150

        in vec3 position;
        in vec3 normal;
        in vec2 tex_coords;

        out vec3 v_normal;
        out vec3 v_position;
        out vec2 v_tex_coords;

        uniform mat4 perspective;
        uniform mat4 view;
        uniform mat4 model;

        void main() {
            v_tex_coords = tex_coords;
            mat4 modelview = view * model;
            v_normal = transpose(inverse(mat3(modelview))) * normal;
            gl_Position = perspective * modelview * vec4(position, 1.0);
            v_position = gl_Position.xyz / gl_Position.w;
        }
    "#;

        let fragment_shader_src = r#"
        #version 140

        in vec3 v_normal;
        in vec3 v_position;
        in vec2 v_tex_coords;

        out vec4 color;

        uniform vec3 u_light;
        uniform sampler2D diffuse_texture;
        uniform sampler2D normal_texture;

        const vec3 specular_color = vec3(1.0, 1.0, 1.0);

        mat3 cotangent_frame(vec3 normal, vec3 pos, vec2 uv) {
            // get edge vectors of the pixel triangle
            vec3 dp1 = dFdx(pos);
            vec3 dp2 = dFdy(pos);
            vec2 duv1 = dFdx(uv);
            vec2 duv2 = dFdy(uv);

            // solve the linear system
            vec3 dp2perp = cross(dp2, normal);
            vec3 dp1perp = cross(normal, dp1);
            vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
            vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;

            // construct a scale-invariant frame 
            float invmax = inversesqrt(max(dot(T,T), dot(B,B)));
            return mat3(T * invmax, B * invmax, normal);
        }

        void main() {
            vec3 diffuse_color = texture(diffuse_texture, v_tex_coords).rgb;
            vec3 ambient_color = diffuse_color * 0.1;

            vec3 v_normal_unit = normalize(v_normal);
            vec3 normal_map = texture(normal_texture, v_tex_coords).rgb;
            mat3 tbn = cotangent_frame(v_normal_unit, -v_position, v_tex_coords);
            vec3 real_normal = normalize(tbn * -(normal_map * 2.0 - 1.0));

            float diffuse = max(dot(normalize(real_normal), normalize(u_light)), 0.0);

            vec3 camera_dir = normalize(-v_position);
            vec3 half_dir = normalize(normalize(u_light) + camera_dir);
            float specular = pow(max(dot(half_dir, real_normal), 0.0), 16.0);

            color = vec4(ambient_color + diffuse * diffuse_color + specular * specular_color, 1.0);
        }
    "#;

        glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap()
    }

    pub fn draw(&self, frame: &mut glium::Frame) {
        let params = Self::get_draw_parameters();
        let uniforms = self.get_uniforms(frame);

        // Draw the teacup
        frame
            .draw(
                &self.positions,
                &self.indices,
                &self.shader_program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
    fn get_draw_parameters<'a>() -> glium::DrawParameters<'a> {
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
    fn get_uniforms<'a>(&'a self, frame: &glium::Frame) -> impl 'a + glium::uniforms::Uniforms {
        uniform! {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32],
            ],
            u_light: [1.4, 0.4, -0.7f32],
            perspective: get_perspective_matrix(frame),
            // Virtual Camera
            view: get_view_matrix(&[0.5,  0.2, -3.0], &[-0.5, -0.2, 3.0], &[0.0, 1.0, 0.0]),
            diffuse_texture: &self.diffuse_texture,
            normal_texture: &self.normal_texture,

        }
    }

    fn load_normal<'a>(display: &glium::Display) -> glium::texture::Texture2d {
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

    fn load_texture<'a>(display: &glium::Display) -> glium::texture::SrgbTexture2d {
        let image = image::load(
            Cursor::new(&include_bytes!("..\\assets\\textures\\tuto-14-diffuse.jpg")),
            image::ImageFormat::Jpeg,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();

        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        glium::texture::SrgbTexture2d::new(display, image).unwrap()
    }
}
impl Teapot {
    pub fn new(display: &glium::Display) -> Self {
        Self {
            positions: glium::VertexBuffer::new(display, &teapot::VERTICES).unwrap(),
            normals: glium::VertexBuffer::new(display, &teapot::NORMALS).unwrap(),
            indices: glium::IndexBuffer::new(
                display,
                glium::index::PrimitiveType::TrianglesList,
                &teapot::INDICES,
            )
            .unwrap(),
            shader_program: Self::get_shader(display),
        }
    }
    fn get_shader(display: &glium::Display) -> glium::Program {
        let vertex_shader_src = r#"
        #version 150

        in vec3 position;
        in vec3 normal;

        out vec3 v_normal;
        out vec3 v_position;

        uniform mat4 perspective;
        uniform mat4 view;
        uniform mat4 model;

        void main() {
            mat4 modelview = view * model;
            v_normal = transpose(inverse(mat3(modelview))) * normal;
            gl_Position = perspective * modelview * vec4(position, 1.0);
            v_position = gl_Position.xyz / gl_Position.w;
        }
    "#;

        let fragment_shader_src = r#"
        #version 140

        in vec3 v_normal;
        in vec3 v_position;

        out vec4 color;

        uniform vec3 u_light;

        const vec3 ambient_color = vec3(0.2, 0.0, 0.0);
        const vec3 diffuse_color = vec3(0.6, 0.0, 0.0);
        const vec3 specular_color = vec3(1.0, 1.0, 1.0);

        void main() {
            float diffuse = max(dot(normalize(v_normal), normalize(u_light)), 0.0);

            vec3 camera_dir = normalize(-v_position);
            vec3 half_dir = normalize(normalize(u_light) + camera_dir);
            float specular = pow(max(dot(half_dir, normalize(v_normal)), 0.0), 16.0);

            color = vec4(ambient_color + diffuse * diffuse_color + specular * specular_color, 1.0);
        }
    "#;

        glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap()
    }

    pub fn draw(&self, frame: &mut glium::Frame) {
        let params = Self::get_draw_parameters();
        let uniforms = Self::get_uniforms(frame);

        // Draw the teacup
        frame
            .draw(
                (&self.positions, &self.normals),
                &self.indices,
                &self.shader_program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
    fn get_draw_parameters<'a>() -> glium::DrawParameters<'a> {
        glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            // disabled for teapot as it has holes in it
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        }
    }
    fn get_uniforms(frame: &glium::Frame) -> impl glium::uniforms::Uniforms {
        uniform! {
            model: [
                [0.01, 0.0, 0.0, 0.0],
                [0.0, 0.01, 0.0, 0.0],
                [0.0, 0.0, 0.01, 0.0],
                [0.0, 0.0, 2.0, 1.0f32],
            ],
            u_light: [1.4, 0.4, -0.7f32],
            perspective: get_perspective_matrix(frame),
            // Virtual Camera
            view: get_view_matrix(&[2.0, -1.0, 1.0], &[-2.0, 1.0, 1.0], &[0.0, 1.0, 0.0]),
        }
    }
}

fn get_draw_parameters<'a>() -> glium::DrawParameters<'a> {
    glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        // disabled for teapot as it has holes in it
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
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
