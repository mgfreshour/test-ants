use bevy::prelude::*;

use crate::components::map::{MapId, MapKind, MapMarker};
use crate::components::nest::{Brood, CarriedBy, CellType, DigStep, FoodEntity, NestTask};
use crate::plugins::nest_navigation::world_to_nest_grid;
use crate::resources::active_map::ActiveMap;
use crate::resources::nest::{NestGrid, PlayerDigZones};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::{NestPheromoneConfig, NestPheromoneGrid};
use crate::resources::simulation::{SimClock, SimSpeed};

use super::{ExcavatedCell, ExpandZone, NestTaskLabel};

/// Update carried item positions to follow the ant carrying them.
pub(super) fn update_carried_item_positions(
    ant_query: Query<(Entity, &Transform), With<NestTask>>,
    mut item_query: Query<(&mut Transform, &CarriedBy), (Or<(With<Brood>, With<FoodEntity>)>, Without<NestTask>)>,
) {
    for (mut item_tf, carried_by) in &mut item_query {
        if let Ok((_, ant_tf)) = ant_query.get(carried_by.0) {
            // Position item at ant's location with slight offset.
            item_tf.translation.x = ant_tf.translation.x;
            item_tf.translation.y = ant_tf.translation.y + 3.0;
        }
    }
}

pub(super) fn cleanup_orphaned_carried_items(
    mut commands: Commands,
    item_query: Query<(Entity, &CarriedBy), Or<(With<FoodEntity>, With<Brood>)>>,
    ant_query: Query<(), With<NestTask>>,
) {
    for (item_entity, carried_by) in &item_query {
        if ant_query.get(carried_by.0).is_err() {
            commands.entity(item_entity).remove::<CarriedBy>();
        }
    }
}

/// Process ExpandZone markers: convert tunnel cells to chambers, update sprites and pheromones.
pub(super) fn apply_zone_expansions(
    mut commands: Commands,
    mut map_query: Query<(&mut NestGrid, &mut NestPathCache, &mut NestPheromoneGrid), With<MapMarker>>,
    mut query: Query<(Entity, &ExpandZone, &MapId)>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId), Without<ExpandZone>>,
) {
    use crate::resources::nest_pheromone::chamber_kind_to_label;

    for (entity, expand, map_id) in &mut query {
        let Ok((mut grid, mut path_cache, mut phero_grid)) = map_query.get_mut(map_id.0) else {
            commands.entity(entity).remove::<ExpandZone>();
            continue;
        };

        let (x, y) = (expand.x, expand.y);
        let chamber = expand.chamber;

        if grid.get(x, y) == CellType::Tunnel {
            grid.set(x, y, CellType::Chamber(chamber));
            path_cache.invalidate();

            if let Some(phero) = phero_grid.get_mut(x, y) {
                let label_idx = chamber_kind_to_label(chamber);
                phero.chamber_labels[label_idx] = 1.0;
            }

            for (tile, mut sprite, tile_map_id) in &mut tile_query {
                if tile_map_id.0 == map_id.0 && tile.grid_x == x && tile.grid_y == y {
                    sprite.color = CellType::Chamber(chamber).color();
                    break;
                }
            }
        }
        commands.entity(entity).remove::<ExpandZone>();
    }
}

/// Process ExcavatedCell markers: mutate the NestGrid, invalidate path cache,
/// and update the tile sprite so the player sees the newly dug tunnel.
pub(super) fn apply_excavated_cells(
    mut commands: Commands,
    mut map_query: Query<(&mut NestGrid, &mut NestPathCache), With<MapMarker>>,
    mut query: Query<(Entity, &ExcavatedCell, &MapId)>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId), Without<ExcavatedCell>>,
) {
    for (entity, excavated, map_id) in &mut query {
        let Ok((mut grid, mut path_cache)) = map_query.get_mut(map_id.0) else {
            commands.entity(entity).remove::<ExcavatedCell>();
            continue;
        };

        let (x, y) = (excavated.x, excavated.y);
        if grid.get(x, y).is_diggable() {
            grid.set(x, y, CellType::Tunnel);
            path_cache.invalidate();

            // Update the tile sprite color to match the new cell type.
            for (tile, mut sprite, tile_map_id) in &mut tile_query {
                if tile_map_id.0 == map_id.0 && tile.grid_x == x && tile.grid_y == y {
                    sprite.color = CellType::Tunnel.color();
                    break;
                }
            }
        }
        commands.entity(entity).remove::<ExcavatedCell>();
    }
}

/// Diggers deposit construction pheromone at their target dig face.
/// Self-limiting: pheromone concentration caps and nearby crowding dampens deposit.
pub(super) fn construction_pheromone_deposit(
    clock: Res<SimClock>,
    time: Res<Time>,
    phero_config: Res<NestPheromoneConfig>,
    mut map_query: Query<&mut NestPheromoneGrid, With<MapMarker>>,
    query: Query<(&Transform, &NestTask, &MapId)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    // Humidity boosts deposit: humid → stronger deposits → tighter clusters.
    let deposit_rate = crate::sim_core::nest_transitions::humidity_scaled_deposit(0.15, phero_config.humidity);
    let max_construction = 1.0;

    for (transform, task, map_id) in &query {
        let Ok(mut phero_grid) = map_query.get_mut(map_id.0) else { continue };

        if let NestTask::Dig { step, target_cell, .. } = task {
            // Only deposit while actively excavating or approaching dig face.
            if *step != DigStep::Excavate && *step != DigStep::GoToFace {
                continue;
            }
            if let Some((tx, ty)) = target_cell {
                if let Some(cell) = phero_grid.get_mut(*tx, *ty) {
                    // Self-limiting: deposit less when concentration is already high.
                    let headroom = (max_construction - cell.construction).max(0.0);
                    cell.construction += deposit_rate * dt * headroom;
                    cell.construction = cell.construction.min(max_construction);
                }

                // Also deposit lightly on the ant's current position.
                let pos = transform.translation.truncate();
                if let Some((gx, gy)) = world_to_nest_grid(pos) {
                    if let Some(cell) = phero_grid.get_mut(gx, gy) {
                        let headroom = (max_construction - cell.construction).max(0.0);
                        cell.construction += deposit_rate * 0.3 * dt * headroom;
                        cell.construction = cell.construction.min(max_construction);
                    }
                }
            }
        }
    }
}

/// Gentle push-apart force for nest ants to prevent clumping in tunnels.
/// Laden ants (carrying food/soil/brood) have movement priority — empty ants
/// yield by receiving stronger push forces while laden ants resist being pushed.
pub(super) fn nest_separation_steering(
    clock: Res<SimClock>,
    time: Res<Time>,
    map_query: Query<&NestGrid, With<MapMarker>>,
    mut query: Query<(Entity, &mut Transform, &MapId, &NestTask)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let separation_radius = 8.0f32;
    let separation_strength = 30.0f32;

    // Collect positions and carrying state to avoid borrow conflicts.
    let positions: Vec<(Entity, Vec2, Entity, bool)> = query
        .iter()
        .map(|(e, t, m, task)| (e, t.translation.truncate(), m.0, task.is_carrying()))
        .collect();

    for (entity, mut transform, map_id, task) in &mut query {
        let Ok(grid) = map_query.get(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let is_laden = task.is_carrying();
        let mut push = Vec2::ZERO;

        for &(other_entity, other_pos, other_map, other_laden) in &positions {
            // Only push against ants on the same map.
            if other_entity == entity || other_map != map_id.0 {
                continue;
            }
            let diff = pos - other_pos;
            let dist = diff.length();
            if dist > 0.1 && dist < separation_radius {
                let base_force = diff.normalize() * (1.0 - dist / separation_radius);

                // Tunnel traffic priority: empty ants yield to laden ants.
                let weight = if !is_laden && other_laden {
                    // Empty ant near a laden ant — yield strongly.
                    2.0
                } else if is_laden && !other_laden {
                    // Laden ant near an empty ant — resist being pushed.
                    0.3
                } else {
                    1.0
                };

                push += base_force * weight;
            }
        }

        if push.length() > 0.01 {
            let displacement = push.normalize() * separation_strength * dt;
            let new_pos = pos + displacement;

            // Only apply if new position is still in a passable cell.
            if let Some((gx, gy)) = world_to_nest_grid(new_pos) {
                if grid.get(gx, gy).is_passable() {
                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                }
            }
        }
    }
}

/// In underground view, left-click to designate dig zones, right-click to clear.
pub(super) fn player_dig_zone_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::plugins::camera::MainCamera>>,
    active: Res<ActiveMap>,
    mut map_query: Query<(&NestGrid, &mut PlayerDigZones), With<MapMarker>>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId)>,
) {
    // Only process when viewing a nest.
    if !matches!(active.kind, MapKind::Nest { .. }) {
        return;
    }

    let left = mouse.just_pressed(MouseButton::Left);
    let right = mouse.just_pressed(MouseButton::Right);
    if !left && !right {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, cam_transform)) = camera_query.single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };

    let Some((gx, gy)) = world_to_nest_grid(world_pos) else { return };

    let Ok((grid, mut dig_zones)) = map_query.get_mut(active.entity) else { return };

    if left {
        // Only allow marking diggable cells.
        if grid.get(gx, gy).is_diggable() {
            dig_zones.cells.insert((gx, gy));
            // Tint the tile to show it's designated.
            for (tile, mut sprite, tile_map) in &mut tile_query {
                if tile_map.0 == active.entity && tile.grid_x == gx && tile.grid_y == gy {
                    sprite.color = Color::srgb(0.6, 0.45, 0.2);
                    break;
                }
            }
        }
    } else if right {
        if dig_zones.cells.remove(&(gx, gy)) {
            // Restore original color.
            let cell = grid.get(gx, gy);
            for (tile, mut sprite, tile_map) in &mut tile_query {
                if tile_map.0 == active.entity && tile.grid_x == gx && tile.grid_y == gy {
                    sprite.color = cell.color();
                    break;
                }
            }
        }
    }
}

/// Show task letter above each nest ant when viewing any nest.
pub(super) fn nest_task_labels(
    active: Res<ActiveMap>,
    mut commands: Commands,
    ant_query: Query<(Entity, &NestTask, &MapId, Option<&Children>)>,
    existing_labels: Query<Entity, With<NestTaskLabel>>,
) {
    // Clean up old labels.
    for entity in &existing_labels {
        commands.entity(entity).despawn();
    }

    if !matches!(active.kind, MapKind::Nest { .. }) {
        return;
    }

    for (entity, task, map_id, _children) in &ant_query {
        // Only label ants on the currently viewed nest.
        if map_id.0 != active.entity {
            continue;
        }

        let label = task.label();
        let color = task.color();

        let label_entity = commands
            .spawn((
                Text2d::new(label),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(color),
                Transform::from_xyz(0.0, 6.0, 0.1),
                NestTaskLabel,
                MapId(map_id.0),
            ))
            .id();

        commands.entity(entity).add_child(label_entity);
    }
}
