use crate::preset::{InitialParameters, Preset, StartingArrangement};
use glium::uniforms::{self, Sampler};
use glium::{implement_vertex, uniform, Surface};
use lerp::Lerp;
use rand::Rng;
use std::cell::RefCell;

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

pub struct ShaderPipeline {
    reset_points_before_draw: bool,
    clear_textures_before_draw: bool,
    initial_parameters: InitialParameters,
    shader_1: glium::Program,
    shader_2: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    buffer_a: RefCell<glium::VertexBuffer<Position>>,
    buffer_b: RefCell<glium::VertexBuffer<Position>>,
    u_texture0: RefCell<glium::texture::Texture2d>,
    u_texture1: RefCell<glium::texture::Texture2d>,
    target_texture0: RefCell<glium::texture::Texture2d>,
    target_texture1: RefCell<glium::texture::Texture2d>,
    width: u32,
    height: u32,
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

        let (buffer_a, buffer_b) = Self::get_initial_locations(display, preset.initial_parameters);

        Self {
            width,
            height,
            reset_points_before_draw: false,
            clear_textures_before_draw: false,
            initial_parameters: preset.initial_parameters,
            buffer_a: RefCell::new(buffer_a),
            buffer_b: RefCell::new(buffer_b),
            vertex_buffer,
            shader_1,
            shader_2,
            u_texture0: RefCell::new(u_texture0),
            u_texture1: RefCell::new(u_texture1),
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
        }
    }
    pub fn draw(
        &mut self,
        frame: &mut impl glium::Surface,
        display: &glium::Display,
        preset: Preset,
        old_preset: Preset,
        lerp_start: f32,
        lerp_length: f32,
        u_time: f32,
    ) {
        if self.clear_textures_before_draw {
            self.clear_textures(display, self.width, self.height);
            self.clear_textures_before_draw = false;
        }

        if self.reset_points_before_draw {
            let (buffer_a, buffer_b) =
                Self::get_initial_locations(display, self.initial_parameters);

            let (buffer_a, buffer_b) = (RefCell::new(buffer_a), RefCell::new(buffer_b));

            self.buffer_a.swap(&buffer_a);
            self.buffer_b.swap(&buffer_b);

            self.reset_points_before_draw = false;
        }

        let lerp_now = (u_time - lerp_start).abs();
        //println!("u_time: {u_time} lerp_start: {lerp_start} lerp_now: {lerp_now}");
        let lerp_preset = lerp_now < lerp_length;
        let preset = if lerp_preset {
            old_preset.lerp(preset, lerp_now / lerp_length)
        } else {
            preset
        };

        {
            let target_texture = self.target_texture0.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
            self.draw_1(&mut framebuffer, display, preset, u_time);
        }

        {
            // Swap target_texture with u_texture0
            std::mem::swap(
                &mut *self.target_texture0.borrow_mut(),
                &mut *self.u_texture0.borrow_mut(),
            );
        }

        self.buffer_a.swap(&self.buffer_b);

        frame.clear_color(0.0, 0.0, 0.0, 1.0);

        //self.draw_1(frame, display, u_time);

        {
            let target_texture = self.target_texture1.borrow();
            let mut framebuffer =
                glium::framebuffer::SimpleFrameBuffer::new(display, &*target_texture).unwrap();
            framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
            self.draw_2(&mut framebuffer, display, preset, u_time);
        }

        self.draw_2(frame, display, preset, u_time);

        {
            // Swap target_texture with u_texture1
            std::mem::swap(
                &mut *self.target_texture1.borrow_mut(),
                &mut *self.u_texture1.borrow_mut(),
            );
        }
    }

    fn draw_1(
        &self,
        frame: &mut impl glium::Surface,
        display: &glium::Display,
        preset: Preset,
        u_time: f32,
    ) {
        {
            let mut buffer_b = self.buffer_b.borrow_mut();
            let session = glium::vertex::TransformFeedbackSession::new(
                display,
                &self.shader_1,
                &mut buffer_b,
            )
            .unwrap();

            let draw_parameters = Self::get_draw_parameters_shader_1(&session);

            let u_texture1 = &*self.u_texture1.borrow();
            let uniforms = uniform! {
                u_texture1: Sampler::new(u_texture1).wrap_function(uniforms::SamplerWrapFunction::Repeat),
                u_speed_multiplier: preset.speed_multiplier,
                u_wall_strategy: preset.wall_strategy as u8,
                u_color_strategy: preset.color_strategy as u8,
                u_random_steer_factor: preset.random_steer_factor,
                u_constant_steer_factor: preset.constant_steer_factor,
                u_search_radius: preset.search_radius,
                u_trail_strength: preset.trail_strength,
                u_vertex_radius: preset.point_size,
                u_search_angle: 0.2f32,
                u_time: u_time,
            };

            // Draw shader_1 to the frame
            frame
                .draw(
                    &*self.buffer_a.borrow(),
                    glium::index::NoIndices(glium::index::PrimitiveType::Points),
                    &self.shader_1,
                    &uniforms,
                    &draw_parameters,
                )
                .unwrap();
        }
    }

    fn draw_2(
        &self,
        frame: &mut impl glium::Surface,
        _display: &glium::Display,
        preset: Preset,
        u_time: f32,
    ) {
        let u_texture0 = &*self.u_texture0.borrow();
        let u_texture1 = &*self.u_texture1.borrow();
        let uniforms = uniform! {
            u_texture0: Sampler::new(u_texture0).wrap_function(uniforms::SamplerWrapFunction::Repeat),
            u_texture1: Sampler::new(u_texture1).wrap_function(uniforms::SamplerWrapFunction::Repeat),
            u_fade_speed: preset.fade_speed,
            u_blur_fraction: preset.blurring,
            u_time: u_time,
            u_max_distance: 1.0f32,
        };
        // Draw the results of shader_2 to the screen
        frame
            .draw(
                &self.vertex_buffer,
                glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan),
                &self.shader_2,
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

    pub fn clear(&mut self) {
        self.clear_textures_before_draw = true;
    }

    fn get_initial_locations(
        display: &glium::Display,
        initial_parameters: InitialParameters,
    ) -> (glium::VertexBuffer<Position>, glium::VertexBuffer<Position>) {
        let mut initial_locations =
            vec![Position::default(); initial_parameters.number_of_points as usize];

        let pi_times_2_over_n =
            std::f32::consts::PI * 2.0 / initial_parameters.number_of_points as f32;

        let mut rng = rand::thread_rng();
        for i in 0..initial_parameters.number_of_points {
            let speed =
                (rng.gen_range(0.0..1.00) * 0.01 * initial_parameters.starting_speed_spread
                    + 0.01 * initial_parameters.average_starting_speed)
                    / 1000.0;
            initial_locations[i as usize] = Position {
                a_position: match initial_parameters.starting_arrangement {
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
        uniform float u_max_distance;

        // Passed to fragment shader
        varying vec4 v_color;

        float rand(vec2 co) {
            return fract(sin(dot(co.xy, vec2(12.9898,78.233))) * 43758.5453);
        }

        vec3 hsv2rgb(vec3 c) {
            vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
            vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
            return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
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

            float randomAngle = 0.0;
            // Wall strategy
            switch (u_wall_strategy) {
                case 0u:
                    // None
                    break;
                case 1u:
                    // wrap around
                    if (y_new > 0.99) { y_new = -0.99; }
                    if (y_new < -0.99) { y_new = 0.99; }

                    if (x_new > 0.99) { x_new = -0.99; }
                    if (x_new < -0.99) { x_new = 0.99; }
                    break;
                // BounceRandom
                case 3u:
                    randomAngle = rand(texcoord+tex_val.xy)*u_random_steer_factor;
                case 2u:
                    // reverse direction if hitting wall
                    if (y_new + speed*sin(direction) > 0.90) {
                        float d = atan(sin(direction), cos(direction));
                        direction -= 2.0*d + randomAngle;
                    }
                    if (y_new + speed*sin(direction) < -0.90) {
                        float d = atan(sin(direction), cos(direction));
                        direction -= 2.0*d + randomAngle;
                    }
                    if (x_new + speed*cos(direction) > 0.90) {
                        float d = atan(cos(direction), sin(direction));
                        direction += 2.0*d + randomAngle;
                    }
                    if (x_new + speed*cos(direction) < -0.90) {
                        float d = atan(cos(direction), sin(direction));
                        direction += 2.0*d + randomAngle;
                    }
                    break;
                // Slow and reverse
                case 4u:
                    float boundary = 0.75;
                    float slowdownFactor = 0.75;

                    if (y_new + speed * sin(direction) > boundary || y_new + speed * sin(direction) < -boundary) {
                        speed *= slowdownFactor;
                        direction = 3.14159 - direction;
                    }
                    if (x_new + speed * cos(direction) > boundary || x_new + speed * cos(direction) < -boundary) {
                        speed *= slowdownFactor;
                        direction = -direction;
                    }
                    break;
            }

            // Update position based on direction
            y_new += speed*u_speed_multiplier*sin(direction);
            x_new += speed*u_speed_multiplier*cos(direction);

            // Set the color of this vert
            float r = 0.0;
            float g = 0.0;
            float b = 0.0;

            // Color strategy
            switch (u_color_strategy) {
                case 0u:
                    r = sin(direction);
                    g = cos(direction);
                    b = u_trail_strength;
                    break;
                case 1u:
                    r = speed_var*50.0;
                    g = r;
                    b = u_trail_strength;
                    break;
                case 2u:
                    r = abs(y_new)/2.0 + 0.5;
                    g = abs(x_new)/2.0 + 0.5;
                    b = u_trail_strength;
                    break;
                case 3u:
                    r = u_trail_strength;
                    g = r;
                    b = r;
                    break;
                // Color strategy 4: Hue shifting based on position
                case 4u:
                    float distanceFromCenter = sqrt(x_new * x_new + y_new * y_new);
                    float normalizedDistance = distanceFromCenter / 1.3;
                    float hue = atan(y_new, x_new) / (2.0 * 3.14159) + 0.5;
                    vec3 hsv = vec3(hue, 1.0-normalizedDistance, 1.0);
                    vec3 rgb = hsv2rgb(hsv); 
                    r = rgb.r;
                    g = rgb.g;
                    b = u_trail_strength;
                    break;
                // Color strategy 5: Gradient based on distance from center
                case 5u:
                    distanceFromCenter = sqrt(x_new * x_new + y_new * y_new);
                    normalizedDistance = distanceFromCenter / 1.3;
                    r = mix(0.2, 1.0, normalizedDistance);
                    g = mix(0.5, 1.0, normalizedDistance);
                    b = u_trail_strength;
                    break;

                // Color strategy 6: Color oscillation based on time
                case 6u:
                    float timeFactor = sin(u_time * 0.5);
                    r = 0.5 + 0.5 * sin(2.0 * 3.14159 * (x_new + y_new) + timeFactor);
                    g = 0.5 + 0.5 * sin(2.0 * 3.14159 * (x_new - y_new) + timeFactor);
                    b = u_trail_strength;
                    break;
            }

            v_color = vec4(r, g, b, 1.0);

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

    pub fn reset_points(&mut self, initial_parameters: InitialParameters) {
        self.reset_points_before_draw = true;
        self.initial_parameters = initial_parameters;
    }
}
