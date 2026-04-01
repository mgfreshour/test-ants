use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Caste {
    Worker,
    Soldier,
    Queen,
    Drone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AntState {
    Idle,
    Foraging,
    Returning,
    Nursing,
    Digging,
    Defending,
    Fighting,
    Fleeing,
    Following,
}

#[derive(Component)]
pub struct Ant {
    pub caste: Caste,
    pub state: AntState,
    pub age: f32,
    pub hunger: f32,
}

#[derive(Component)]
pub struct Movement {
    pub speed: f32,
    pub direction: Vec2,
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct ColonyMember {
    pub colony_id: u32,
}

#[derive(Component)]
pub struct CarriedItem {
    pub food_amount: f32,
}

#[derive(Component)]
pub struct PlayerControlled;

impl Ant {
    pub fn new_worker() -> Self {
        Self {
            caste: Caste::Worker,
            state: AntState::Foraging,
            age: 0.0,
            hunger: 0.0,
        }
    }

    pub fn new_soldier() -> Self {
        Self {
            caste: Caste::Soldier,
            state: AntState::Defending,
            age: 0.0,
            hunger: 0.0,
        }
    }
}

impl Movement {
    pub fn with_random_direction(speed: f32, rng: &mut impl rand::Rng) -> Self {
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        Self {
            speed,
            direction: Vec2::new(angle.cos(), angle.sin()),
        }
    }
}

impl Health {
    pub fn worker() -> Self {
        Self {
            current: 10.0,
            max: 10.0,
        }
    }

    pub fn soldier() -> Self {
        Self {
            current: 25.0,
            max: 25.0,
        }
    }
}
