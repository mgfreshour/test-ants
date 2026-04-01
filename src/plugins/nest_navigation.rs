use bevy::prelude::*;

use crate::components::map::{MapId, MapKind, MapMarker};
use crate::components::nest::NestPath;
use crate::resources::active_map::viewing_nest;
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::{GridPos, NestPathCache};
use crate::resources::simulation::{SimClock, SimSpeed};

pub struct NestNavigationPlugin;

/// Movement speed for nest ants (pixels per second).
const NEST_ANT_SPEED: f32 = 40.0;

/// Distance threshold to consider a waypoint reached.
const WAYPOINT_THRESHOLD: f32 = 3.0;

impl Plugin for NestNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NestDebugPaths>()
            .add_systems(
                Update,
                (
                    nest_path_following,
                    nest_grid_collision,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                update_debug_path_lines.run_if(viewing_nest),
            );
    }
}

/// Convert nest grid coordinates to world position.
pub fn nest_grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;
    Vec2::new(
        offset_x + gx as f32 * NEST_CELL_SIZE + NEST_CELL_SIZE / 2.0,
        offset_y - gy as f32 * NEST_CELL_SIZE - NEST_CELL_SIZE / 2.0,
    )
}

/// Convert world position to nest grid coordinates.
pub fn world_to_nest_grid(pos: Vec2) -> Option<GridPos> {
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

/// Move ants along their computed path waypoints.
fn nest_path_following(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut NestPath)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut transform, mut path) in &mut query {
        if path.current_index >= path.waypoints.len() {
            continue;
        }

        let target_grid = path.waypoints[path.current_index];
        let target_world = nest_grid_to_world(target_grid.0, target_grid.1);
        let pos = transform.translation.truncate();
        let to_target = target_world - pos;
        let dist = to_target.length();

        if dist < WAYPOINT_THRESHOLD {
            path.current_index += 1;
            transform.translation.x = target_world.x;
            transform.translation.y = target_world.y;
        } else {
            let dir = to_target.normalize();
            let step = dir * NEST_ANT_SPEED * dt;
            if step.length() > dist {
                transform.translation.x = target_world.x;
                transform.translation.y = target_world.y;
            } else {
                transform.translation.x += step.x;
                transform.translation.y += step.y;
            }
        }
    }
}

/// Clamp ant positions to passable cells.
/// Any ant on a nest map that ends up in a wall is relocated to the nearest
/// passable cell. Ants outside the grid entirely are sent to the entrance.
fn nest_grid_collision(
    map_query: Query<&NestGrid, With<MapMarker>>,
    mut ant_query: Query<(Entity, &mut Transform, &MapId)>,
) {
    for (entity, mut transform, map_id) in &mut ant_query {
        let Ok(grid) = map_query.get(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        match world_to_nest_grid(pos) {
            Some((gx, gy)) if !grid.get(gx, gy).is_passable() => {
                if let Some((nx, ny)) = find_nearest_passable(grid, gx, gy) {
                    debug!(
                        "Ant {:?} in wall at grid ({}, {}), world ({:.1}, {:.1}) — relocating to ({}, {})",
                        entity, gx, gy, pos.x, pos.y, nx, ny
                    );
                    let safe = nest_grid_to_world(nx, ny);
                    transform.translation.x = safe.x;
                    transform.translation.y = safe.y;
                }
            }
            None => {
                debug!(
                    "Ant {:?} outside nest grid at world ({:.1}, {:.1}) — teleporting to entrance",
                    entity, pos.x, pos.y
                );
                if let Some((ex, ey)) = find_entrance(grid) {
                    let safe = nest_grid_to_world(ex, ey);
                    transform.translation.x = safe.x;
                    transform.translation.y = safe.y;
                }
            }
            _ => {}
        }
    }
}

fn find_entrance(grid: &NestGrid) -> Option<GridPos> {
    let cx = grid.width / 2;
    for y in 0..grid.height {
        if grid.get(cx, y).is_passable() {
            return Some((cx, y));
        }
    }
    None
}

fn find_nearest_passable(grid: &NestGrid, x: usize, y: usize) -> Option<GridPos> {
    for radius in 1i32..10 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < grid.width
                    && (ny as usize) < grid.height
                    && grid.get(nx as usize, ny as usize).is_passable()
                {
                    return Some((nx as usize, ny as usize));
                }
            }
        }
    }
    None
}

// ── Debug path visualization ──────────────────────────────────────────

#[derive(Resource, Default)]
pub struct NestDebugPaths {
    pub enabled: bool,
}

#[derive(Component)]
pub struct DebugPathLine;

fn update_debug_path_lines(
    input: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<NestDebugPaths>,
    mut commands: Commands,
    path_query: Query<(&Transform, &NestPath, &MapId)>,
    existing_lines: Query<Entity, With<DebugPathLine>>,
) {
    if input.just_pressed(KeyCode::KeyP) {
        debug.enabled = !debug.enabled;
    }

    for entity in &existing_lines {
        commands.entity(entity).despawn();
    }

    if !debug.enabled {
        return;
    }

    for (_transform, path, map_id) in &path_query {
        if path.current_index >= path.waypoints.len() {
            continue;
        }

        for i in path.current_index..path.waypoints.len() {
            let wp = path.waypoints[i];
            let wp_world = nest_grid_to_world(wp.0, wp.1);

            let color = if i == path.waypoints.len() - 1 {
                Color::srgba(0.0, 1.0, 0.0, 0.6)
            } else {
                Color::srgba(1.0, 1.0, 0.0, 0.3)
            };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(3.0)),
                    ..default()
                },
                Transform::from_xyz(wp_world.x, wp_world.y, 4.0),
                MapId(map_id.0),
                DebugPathLine,
            ));
        }
    }
}
