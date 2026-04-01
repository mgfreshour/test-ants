use bevy::prelude::*;

use crate::components::pheromone::{PheromoneOverlayTile, PheromoneType};
use crate::resources::pheromone::{ColonyPheromones, PheromoneConfig};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct PheromonePlugin;

#[derive(Resource)]
pub struct OverlayState {
    pub visible: bool,
    pub display_type: OverlayDisplay,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayDisplay {
    All,
    Home,
    Food,
    Alarm,
    Trail,
    Recruit,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            visible: false,
            display_type: OverlayDisplay::All,
        }
    }
}

impl Plugin for PheromonePlugin {
    fn build(&self, app: &mut App) {
        use crate::resources::active_map::viewing_surface;

        app.init_resource::<PheromoneConfig>()
            .init_resource::<OverlayState>()
            .add_systems(Startup, init_pheromone_grid)
            .add_systems(Update, pheromone_evaporate_diffuse)
            .add_systems(
                Update,
                (toggle_overlay, update_overlay_visuals)
                    .chain()
                    .run_if(viewing_surface),
            );
    }
}

fn init_pheromone_grid(mut commands: Commands, config: Res<SimConfig>) {
    let cell_size = config.tile_size;
    let grids = ColonyPheromones::new(
        config.world_width, config.world_height, cell_size, &[0, 1],
    );

    let w = grids.width();
    let h = grids.height();

    for y in 0..h {
        for x in 0..w {
            let wx = x as f32 * cell_size + cell_size / 2.0;
            let wy = y as f32 * cell_size + cell_size / 2.0;

            commands.spawn((
                Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                    custom_size: Some(Vec2::splat(cell_size)),
                    ..default()
                },
                Transform::from_xyz(wx, wy, 5.0),
                Visibility::Hidden,
                PheromoneOverlayTile {
                    grid_x: x,
                    grid_y: y,
                },
            ));
        }
    }

    commands.insert_resource(grids);
}

fn pheromone_evaporate_diffuse(
    clock: Res<SimClock>,
    pconfig: Res<PheromoneConfig>,
    mut grids: ResMut<ColonyPheromones>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    grids.evaporate_all(&pconfig.evaporation_rates);
    grids.diffuse_all(&pconfig.diffusion_rates, pconfig.max_intensity);
}

fn toggle_overlay(input: Res<ButtonInput<KeyCode>>, mut state: ResMut<OverlayState>) {
    if input.just_pressed(KeyCode::KeyH) {
        if !state.visible {
            state.visible = true;
            state.display_type = OverlayDisplay::All;
        } else {
            state.display_type = match state.display_type {
                OverlayDisplay::All => OverlayDisplay::Home,
                OverlayDisplay::Home => OverlayDisplay::Food,
                OverlayDisplay::Food => OverlayDisplay::Alarm,
                OverlayDisplay::Alarm => OverlayDisplay::Trail,
                OverlayDisplay::Trail => OverlayDisplay::Recruit,
                OverlayDisplay::Recruit => {
                    state.visible = false;
                    OverlayDisplay::All
                }
            };
        }
    }
}

fn update_overlay_visuals(
    grids: Res<ColonyPheromones>,
    state: Res<OverlayState>,
    pconfig: Res<PheromoneConfig>,
    mut query: Query<(
        &PheromoneOverlayTile,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    for (tile, mut sprite, mut visibility) in &mut query {
        if !state.visible {
            *visibility = Visibility::Hidden;
            continue;
        }

        let x = tile.grid_x;
        let y = tile.grid_y;
        let values = grids.combined_get_all(x, y);
        let max = pconfig.max_intensity;

        let (r, g, b, total) = match state.display_type {
            OverlayDisplay::All => {
                let home = values[PheromoneType::Home.index()] / max;
                let food = values[PheromoneType::Food.index()] / max;
                let alarm = values[PheromoneType::Alarm.index()] / max;
                let trail = values[PheromoneType::Trail.index()] / max;
                let recruit = values[PheromoneType::Recruit.index()] / max;
                let r = alarm + trail * 0.8;
                let g = food + trail * 0.7 + recruit * 0.9;
                let b = home + recruit;
                let total = home + food + alarm + trail + recruit;
                (r, g, b, total)
            }
            OverlayDisplay::Home => {
                let v = values[PheromoneType::Home.index()] / max;
                (0.2 * v, 0.4 * v, v, v)
            }
            OverlayDisplay::Food => {
                let v = values[PheromoneType::Food.index()] / max;
                (0.2 * v, v, 0.2 * v, v)
            }
            OverlayDisplay::Alarm => {
                let v = values[PheromoneType::Alarm.index()] / max;
                (v, 0.1 * v, 0.1 * v, v)
            }
            OverlayDisplay::Trail => {
                let v = values[PheromoneType::Trail.index()] / max;
                (v, 0.9 * v, 0.1 * v, v)
            }
            OverlayDisplay::Recruit => {
                let v = values[PheromoneType::Recruit.index()] / max;
                (0.3 * v, 0.9 * v, v, v)
            }
        };

        if total < 0.001 {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Visible;
            let alpha = total.clamp(0.0, 1.0) * 0.6;
            sprite.color = Color::srgba(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), alpha);
        }
    }
}
