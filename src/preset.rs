use lerp::Lerp;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
#[derive(Lerp, PartialEq, Debug, Copy, Clone)]
pub struct Preset {
    // Initial config
    #[lerp(skip)]
    pub number_of_points: u32,
    #[lerp(f32)]
    pub starting_arrangement: StartingArrangement,
    pub average_starting_speed: f32,
    pub starting_speed_spread: f32,

    // Vertex Shader Uniforms
    pub speed_multiplier: f32,
    pub point_size: f32,
    pub random_steer_factor: f32,
    pub constant_steer_factor: f32,
    pub trail_strength: f32,
    pub search_radius: f32,
    #[lerp(f32)]
    pub wall_strategy: WallStrategy,
    #[lerp(f32)]
    pub color_strategy: ColorStrategy,

    // Fragment Shader Uniforms
    pub fade_speed: f32,
    pub blurring: f32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum StartingArrangement {
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
pub enum WallStrategy {
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
pub enum ColorStrategy {
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
pub enum PresetName {
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
    pub fn from_u32(value: u32) -> PresetName {
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
