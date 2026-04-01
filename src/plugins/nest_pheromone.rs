use bevy::prelude::*;

use crate::components::nest::{Brood, BroodStage, CellType, Queen};
use crate::plugins::nest::{GameView, NestViewEntity};
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pheromone::{
    chamber_kind_to_label, NestPheromoneConfig, NestPheromoneGrid,
    LABEL_ENTRANCE,
};
use crate::resources::simulation::{SimClock, SimSpeed};

pub struct NestPheromonePlugin;

impl Plugin for NestPheromonePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NestPheromoneGrid>()
            .init_resource::<NestPheromoneConfig>()
            .init_resource::<NestPheromoneOverlayState>()
            .add_systems(Startup, seed_pheromones)
            .add_systems(
                Update,
                (
                    nest_pheromone_decay,
                    queen_signal_emission,
                    nest_queen_signal_diffusion,
                    brood_need_emission,
                    chamber_label_refresh,
                )
                    .chain(),
            )
            .add_systems(Startup, spawn_overlay_tiles)
            .add_systems(Update, toggle_nest_pheromone_overlay)
            .add_systems(
                Update,
                update_overlay_visuals.run_if(in_state(GameView::Underground)),
            );
    }
}

/// Seed initial chamber labels from the nest layout.
fn seed_pheromones(mut phero_grid: ResMut<NestPheromoneGrid>, nest_grid: Res<NestGrid>) {
    phero_grid.seed_from_grid(&nest_grid);
}

/// Decay all pheromone layers each tick.
fn nest_pheromone_decay(
    clock: Res<SimClock>,
    config: Res<NestPheromoneConfig>,
    mut phero_grid: ResMut<NestPheromoneGrid>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    phero_grid.decay(&config);
}

/// Queen emits signal at her position.
fn queen_signal_emission(
    clock: Res<SimClock>,
    config: Res<NestPheromoneConfig>,
    mut phero_grid: ResMut<NestPheromoneGrid>,
    queen_query: Query<&Transform, With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for transform in &queen_query {
        if let Some((gx, gy)) = world_to_nest_grid(transform.translation.truncate()) {
            if let Some(cell) = phero_grid.get_mut(gx, gy) {
                cell.queen_signal = config.queen_signal_strength;
            }
        }
    }
}

/// Diffuse queen signal through passable cells.
fn nest_queen_signal_diffusion(
    clock: Res<SimClock>,
    config: Res<NestPheromoneConfig>,
    nest_grid: Res<NestGrid>,
    mut phero_grid: ResMut<NestPheromoneGrid>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    phero_grid.diffuse_queen_signal(&nest_grid, config.queen_signal_diffuse);
}

/// Unfed larvae emit brood need signal.
fn brood_need_emission(
    clock: Res<SimClock>,
    config: Res<NestPheromoneConfig>,
    mut phero_grid: ResMut<NestPheromoneGrid>,
    brood_query: Query<(&Transform, &Brood)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (transform, brood) in &brood_query {
        if brood.stage != BroodStage::Larva || brood.fed {
            continue;
        }
        if let Some((gx, gy)) = world_to_nest_grid(transform.translation.truncate()) {
            if let Some(cell) = phero_grid.get_mut(gx, gy) {
                cell.brood_need = (cell.brood_need + config.brood_need_emission).min(1.0);
            }
        }
    }
}

/// Ants present in a chamber passively refresh its identity label.
/// For now, this just reinforces labels based on the grid layout since
/// we don't have Underground marker ants yet (Sprint 6).
fn chamber_label_refresh(
    clock: Res<SimClock>,
    config: Res<NestPheromoneConfig>,
    nest_grid: Res<NestGrid>,
    mut phero_grid: ResMut<NestPheromoneGrid>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Refresh labels based on grid structure (ants will do this in Sprint 6)
    for y in 0..nest_grid.height {
        for x in 0..nest_grid.width {
            let cell = nest_grid.get(x, y);
            if let CellType::Chamber(kind) = cell {
                let label_idx = chamber_kind_to_label(kind);
                if let Some(phero) = phero_grid.get_mut(x, y) {
                    phero.chamber_labels[label_idx] =
                        (phero.chamber_labels[label_idx] + config.label_refresh_amount).min(1.0);
                }
            }
            // Keep entrance labels alive
            if cell == CellType::Tunnel && y <= 1 {
                if let Some(phero) = phero_grid.get_mut(x, y) {
                    phero.chamber_labels[LABEL_ENTRANCE] =
                        (phero.chamber_labels[LABEL_ENTRANCE] + config.label_refresh_amount)
                            .min(1.0);
                }
            }
        }
    }
}

// ── Overlay ──────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct NestPheromoneOverlayState {
    visible: bool,
}

#[derive(Component)]
struct NestPheromoneOverlayTile {
    grid_x: usize,
    grid_y: usize,
}

fn nest_grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;
    Vec2::new(
        offset_x + gx as f32 * NEST_CELL_SIZE + NEST_CELL_SIZE / 2.0,
        offset_y - gy as f32 * NEST_CELL_SIZE - NEST_CELL_SIZE / 2.0,
    )
}

/// Convert world position to nest grid coordinates.
pub fn world_to_nest_grid(pos: Vec2) -> Option<(usize, usize)> {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;

    let gx = ((pos.x - offset_x) / NEST_CELL_SIZE).floor() as i32;
    let gy = ((offset_y - pos.y) / NEST_CELL_SIZE).floor() as i32;

    if gx >= 0 && gy >= 0 && (gx as usize) < NEST_WIDTH && (gy as usize) < NEST_HEIGHT {
        Some((gx as usize, gy as usize))
    } else {
        None
    }
}

fn spawn_overlay_tiles(mut commands: Commands) {
    for y in 0..NEST_HEIGHT {
        for x in 0..NEST_WIDTH {
            let w = nest_grid_to_world(x, y);
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                    custom_size: Some(Vec2::splat(NEST_CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, 5.0), // above nest tiles
                Visibility::Hidden,
                NestViewEntity,
                NestPheromoneOverlayTile { grid_x: x, grid_y: y },
            ));
        }
    }
}

/// Toggle overlay with N key when in underground view.
fn toggle_nest_pheromone_overlay(
    input: Res<ButtonInput<KeyCode>>,
    view: Res<State<GameView>>,
    mut state: ResMut<NestPheromoneOverlayState>,
) {
    if *view.get() == GameView::Underground && input.just_pressed(KeyCode::KeyN) {
        state.visible = !state.visible;
    }
}

/// Update overlay tile colors based on pheromone data.
fn update_overlay_visuals(
    state: Res<NestPheromoneOverlayState>,
    phero_grid: Res<NestPheromoneGrid>,
    nest_grid: Res<NestGrid>,
    mut query: Query<(&NestPheromoneOverlayTile, &mut Sprite)>,
) {
    for (tile, mut sprite) in &mut query {
        if !state.visible {
            sprite.color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        }

        let cell = phero_grid.get(tile.grid_x, tile.grid_y);
        let nest_cell = nest_grid.get(tile.grid_x, tile.grid_y);

        if !nest_cell.is_passable() {
            sprite.color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        }

        // Blend multiple signals into a single color:
        // Blue = queen signal, Pink = brood need, Orange = construction
        // Chamber labels shown as subtle background tint
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;
        let mut a = 0.0f32;

        // Queen signal: blue
        let qs = cell.queen_signal;
        if qs > 0.01 {
            b += qs * 0.8;
            a += qs * 0.4;
        }

        // Brood need: pink (r + slight b)
        let bn = cell.brood_need;
        if bn > 0.01 {
            r += bn * 0.9;
            b += bn * 0.3;
            a += bn * 0.5;
        }

        // Construction: orange (r + g)
        let cp = cell.construction;
        if cp > 0.01 {
            r += cp * 0.9;
            g += cp * 0.5;
            a += cp * 0.5;
        }

        // Chamber labels as subtle tint
        let max_label = cell
            .chamber_labels
            .iter()
            .cloned()
            .fold(0.0f32, f32::max);
        if max_label > 0.1 {
            // Find dominant label for tint color
            let dominant = cell
                .chamber_labels
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);

            let tint = 0.15 * max_label;
            match dominant {
                0 => { r += tint; g += tint * 0.5; } // Brood: warm
                1 => { g += tint; }                    // FoodStorage: green
                2 => { r += tint * 0.7; g += tint * 0.5; b += tint * 0.1; } // Queen: gold
                3 => { r += tint * 0.4; g += tint * 0.3; b += tint * 0.2; } // Midden: gray-brown
                4 => { r += tint * 0.3; g += tint * 0.3; b += tint * 0.8; } // Entrance: light blue
                _ => {}
            }
            a += tint * 0.3;
        }

        sprite.color = Color::srgba(r.min(1.0), g.min(1.0), b.min(1.0), a.min(0.6));
    }
}
