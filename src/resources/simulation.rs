use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimSpeed {
    Paused,
    Normal,
    Fast,
    VeryFast,
    Ultra,
}

impl SimSpeed {
    pub fn multiplier(&self) -> f32 {
        match self {
            SimSpeed::Paused => 0.0,
            SimSpeed::Normal => 1.0,
            SimSpeed::Fast => 2.0,
            SimSpeed::VeryFast => 4.0,
            SimSpeed::Ultra => 8.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SimSpeed::Paused => "Paused",
            SimSpeed::Normal => "1x",
            SimSpeed::Fast => "2x",
            SimSpeed::VeryFast => "4x",
            SimSpeed::Ultra => "8x",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            SimSpeed::Paused => SimSpeed::Normal,
            SimSpeed::Normal => SimSpeed::Fast,
            SimSpeed::Fast => SimSpeed::VeryFast,
            SimSpeed::VeryFast => SimSpeed::Ultra,
            SimSpeed::Ultra => SimSpeed::Normal,
        }
    }
}

#[derive(Resource)]
pub struct SimClock {
    pub speed: SimSpeed,
    pub elapsed: f32,
    pub tick: u64,
}

impl Default for SimClock {
    fn default() -> Self {
        Self {
            speed: SimSpeed::Normal,
            elapsed: 0.0,
            tick: 0,
        }
    }
}

#[derive(Resource)]
pub struct SimConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub tile_size: f32,
    pub initial_ant_count: u32,
    pub ant_speed_worker: f32,
    pub ant_speed_soldier: f32,
    pub exploration_noise: f32,
    pub nest_position: bevy::math::Vec2,
}

impl Default for SimConfig {
    fn default() -> Self {
        let world_width = 2048.0;
        let world_height = 2048.0;
        Self {
            world_width,
            world_height,
            tile_size: 16.0,
            initial_ant_count: 50,
            ant_speed_worker: 80.0,
            ant_speed_soldier: 50.0,
            exploration_noise: 0.15,
            nest_position: bevy::math::Vec2::new(world_width * 0.25, world_height * 0.25),
        }
    }
}
