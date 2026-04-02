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
    Attacking,
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

/// Ring buffer of recent positions so ants avoid backtracking.
/// Stores the last N positions and exposes an anti-backtrack direction.
const HISTORY_LEN: usize = 16;

#[derive(Component)]
pub struct PositionHistory {
    buf: [Vec2; HISTORY_LEN],
    idx: usize,
    count: usize,
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self {
            buf: [Vec2::ZERO; HISTORY_LEN],
            idx: 0,
            count: 0,
        }
    }
}

impl PositionHistory {
    pub fn record(&mut self, pos: Vec2) {
        self.buf[self.idx] = pos;
        self.idx = (self.idx + 1) % HISTORY_LEN;
        if self.count < HISTORY_LEN {
            self.count += 1;
        }
    }

    /// Returns a unit vector pointing AWAY from the centroid of recent positions,
    /// giving the ant forward momentum and preventing U-turns.
    pub fn anti_backtrack(&self, current_pos: Vec2) -> Vec2 {
        if self.count < 2 {
            return Vec2::ZERO;
        }
        let mut centroid = Vec2::ZERO;
        for i in 0..self.count {
            centroid += self.buf[i];
        }
        centroid /= self.count as f32;

        let away = current_pos - centroid;
        if away.length_squared() > 0.1 {
            away.normalize()
        } else {
            Vec2::ZERO
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.idx = 0;
    }
}

#[derive(Component)]
pub struct CarriedItem {
    pub food_amount: f32,
}

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct Follower;

/// Marker component for ants currently in the nest (underground).
#[derive(Component)]
pub struct Underground;

/// What trail, if any, this ant is currently following.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrailSense {
    #[default]
    Searching,
    FollowingFood,
    FollowingHome,
    FollowingAlarm,
    FollowingTrail,
    FollowingAttack,
    BeelineFood,
    BeelineNest,
}

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
