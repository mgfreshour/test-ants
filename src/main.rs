mod components;
mod plugins;
mod resources;
mod ui;

use bevy::prelude::*;

use plugins::simulation::SimulationPlugin;
use plugins::terrain::TerrainPlugin;
use plugins::ant_ai::AntAiPlugin;
use plugins::camera::CameraPlugin;
use ui::hud::HudPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Colony — An Ant Colony Simulation".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
        .add_plugins((
            SimulationPlugin,
            TerrainPlugin,
            AntAiPlugin,
            CameraPlugin,
            HudPlugin,
        ))
        .run();
}
