mod components;
mod plugins;
mod resources;
mod sim_core;
mod ui;

use bevy::prelude::*;
use bevy::window::WindowResolution;

use plugins::simulation::SimulationPlugin;
use plugins::terrain::TerrainPlugin;
use plugins::ldtk_maps::LdtkMapsPlugin;
use plugins::ai_log::AiLogPlugin;
use plugins::ant_ai::AntAiPlugin;
use plugins::camera::CameraPlugin;
use plugins::environment::EnvironmentPlugin;
use plugins::nest::NestPlugin;
use plugins::nest_ai::NestAiPlugin;
use plugins::nest_pheromone::NestPheromonePlugin;
use plugins::nest_navigation::NestNavigationPlugin;
use plugins::combat::CombatPlugin;
use plugins::egui_ui::EguiUiPlugin;
use plugins::pheromone::PheromonePlugin;
use plugins::ant_sprites::AntSpritePlugin;
use plugins::player::PlayerPlugin;
use plugins::queen_ai::QueenAiPlugin;
use plugins::spider_ai::SpiderAiPlugin;
use plugins::steering::SteeringPlugin;
use ui::hud::HudPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Colony — An Ant Colony Simulation".into(),
                resolution: WindowResolution::new(1280, 720),
                canvas: Some("#bevy".to_string()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
        .add_plugins((
            SimulationPlugin,
            TerrainPlugin,
            LdtkMapsPlugin,
            AntAiPlugin,
            CameraPlugin,
            EnvironmentPlugin,
            NestPlugin,
            NestAiPlugin,
            NestPheromonePlugin,
            NestNavigationPlugin,
            QueenAiPlugin,
        ))
        .add_plugins((
            PheromonePlugin,
            PlayerPlugin,
            SteeringPlugin,
            CombatPlugin,
            SpiderAiPlugin,
            AntSpritePlugin,
            EguiUiPlugin,
            HudPlugin,
            AiLogPlugin,
        ))
        .run();
}
