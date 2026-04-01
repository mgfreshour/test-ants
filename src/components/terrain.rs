use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeType {
    Grass,
    Dirt,
    Sand,
    Concrete,
}

#[derive(Component)]
pub struct Terrain {
    pub biome: BiomeType,
}

#[derive(Component)]
pub struct FoodSource {
    pub remaining: f32,
    pub max: f32,
}
