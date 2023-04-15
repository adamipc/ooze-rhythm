use crate::preset::Preset;
use crate::shader_pipeline::ShaderPipeline;

pub struct SlimeMould {
    shader_pipeline: ShaderPipeline,
    preset: Preset,
    width: u32,
    height: u32,
}

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32, preset: Preset) -> Self {
        Self {
            shader_pipeline: ShaderPipeline::new(display, width, height, preset),
            preset,
            width,
            height,
        }
    }

    pub fn save_preset(&self) {
        println!("{:?}", self.preset);
    }

    pub fn draw(&self, frame: &mut impl glium::Surface, display: &glium::Display, u_time: f32) {
        self.shader_pipeline
            .draw(frame, display, self.preset, u_time);
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
}
