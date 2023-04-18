use crate::preset::Preset;
use crate::shader_pipeline::ShaderPipeline;
use lerp::Lerp;

pub struct SlimeMould {
    shader_pipeline: ShaderPipeline,
    old_preset: Preset,
    preset: Preset,
    secondary_preset: Preset,
    lerp_time: f32,
    lerp_length: f32,
}

impl SlimeMould {
    pub fn new(display: &glium::Display, width: u32, height: u32, preset: Preset) -> Self {
        Self {
            shader_pipeline: ShaderPipeline::new(display, width, height, preset),
            old_preset: preset,
            secondary_preset: preset,
            preset,
            lerp_time: 0.0f32,
            lerp_length: 0.0f32,
        }
    }

    pub fn save_preset(&self) {
        println!(
            "preset: {:?}\nsecondary_preset: {:?}",
            self.preset, self.secondary_preset
        );
    }

    pub fn draw(
        &mut self,
        frame: &mut impl glium::Surface,
        display: &glium::Display,
        u_time: f32,
        blend: f32,
    ) {
        let lerp_now = (u_time - self.lerp_time).abs();
        //println!("u_time: {u_time} lerp_start: {lerp_start} lerp_now: {lerp_now}");
        let lerp_preset = lerp_now < self.lerp_length;
        let preset = if lerp_preset {
            self.old_preset
                .lerp(self.preset, lerp_now / self.lerp_length)
        } else {
            self.preset.lerp(self.secondary_preset, blend)
        };

        self.shader_pipeline.draw(frame, display, preset, u_time);
    }

    pub fn clear(&mut self) {
        self.shader_pipeline.clear();
    }
    pub fn transition_preset(&mut self, preset_to: Preset, u_time: f32, transition_length: f32) {
        self.old_preset = self.preset;
        self.preset = preset_to;
        self.lerp_time = u_time;
        self.lerp_length = transition_length;
    }

    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = preset;
        self.lerp_length = 0.0f32;
    }

    pub fn set_secondary_preset(&mut self, preset: Preset) {
        self.secondary_preset = preset;
    }

    pub fn get_preset(&self) -> Preset {
        self.preset
    }

    pub fn reset_points(&mut self) {
        self.shader_pipeline
            .reset_points(self.preset.initial_parameters);
    }

    pub fn update(&mut self) {}
}
