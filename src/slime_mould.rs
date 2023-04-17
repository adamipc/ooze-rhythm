use crate::preset::Preset;
use crate::shader_pipeline::ShaderPipeline;

pub struct SlimeMould {
    shader_pipeline: ShaderPipeline,
    old_preset: Preset,
    preset: Preset,
    lerp_time: f32,
    lerp_length: f32,
    width: u32,
    height: u32,
}

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32, preset: Preset) -> Self {
        Self {
            shader_pipeline: ShaderPipeline::new(display, width, height, preset),
            old_preset: preset,
            preset,
            lerp_time: 0.0f32,
            lerp_length: 0.0f32,
            width,
            height,
        }
    }

    pub fn save_preset(&self) {
        println!("{:?}", self.preset);
    }

    pub fn draw(&self, frame: &mut impl glium::Surface, display: &glium::Display, u_time: f32) {
        self.shader_pipeline.draw(
            frame,
            display,
            self.preset,
            self.old_preset,
            self.lerp_time,
            self.lerp_length,
            u_time,
        );
    }

    pub fn clear(&mut self, display: &glium::Display) {
        self.shader_pipeline.clear(display, self.width, self.height);
    }
    pub fn transition_preset(
        &mut self,
        preset_from: Preset,
        preset_to: Preset,
        u_time: f32,
        transition_length: f32,
    ) {
        self.old_preset = preset_from;
        self.preset = preset_to;
        self.lerp_time = u_time;
        self.lerp_length = transition_length;
    }

    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = preset;
        self.lerp_length = 0.0f32;
    }

    pub fn get_preset(&self) -> Preset {
        self.preset
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
}
