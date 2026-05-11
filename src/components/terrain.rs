use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BiomeType {
    Grass,
    Dirt,
    Sand,
    Concrete,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Terrain {
    pub biome: BiomeType,
}

#[derive(Component)]
pub struct FoodSource {
    pub remaining: f32,
    pub max: f32,
}
