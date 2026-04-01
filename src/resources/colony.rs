use bevy::prelude::*;

use crate::components::ant::Caste;

#[derive(Resource)]
pub struct BehaviorSliders {
    pub forage: f32,
    pub nurse: f32,
    pub dig: f32,
    pub defend: f32,
}

impl Default for BehaviorSliders {
    fn default() -> Self {
        Self {
            forage: 0.6,
            nurse: 0.2,
            dig: 0.1,
            defend: 0.1,
        }
    }
}

#[derive(Resource)]
pub struct CasteRatios {
    pub worker: f32,
    pub soldier: f32,
    pub drone: f32,
}

impl Default for CasteRatios {
    fn default() -> Self {
        Self {
            worker: 0.70,
            soldier: 0.20,
            drone: 0.10,
        }
    }
}

impl CasteRatios {
    pub fn pick_caste(&self, roll: f32) -> Caste {
        if roll < self.worker {
            Caste::Worker
        } else if roll < self.worker + self.soldier {
            Caste::Soldier
        } else {
            Caste::Drone
        }
    }
}

#[derive(Resource, Default)]
pub struct ColonyStats {
    pub workers: u32,
    pub soldiers: u32,
    pub drones: u32,
    pub eggs: u32,
    pub larvae: u32,
    pub pupae: u32,
}
