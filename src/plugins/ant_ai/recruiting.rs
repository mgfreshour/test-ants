use bevy::prelude::*;
use rand::Rng;
use crate::components::ant::{Ant, AntState, ColonyMember, Movement, PlayerControlled, SteeringTarget, SteeringWeights, TrailSense};
use crate::components::map::{MapId, MapMarker};
use crate::components::nest::NestTask;
use crate::resources::active_map::MapRegistry;
use crate::resources::nest::NestGrid;
use crate::resources::nest_pheromone::NestPheromoneGrid;
use crate::resources::pheromone::ColonyPheromones;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::components::pheromone::PheromoneType;
use crate::sim_core::ant_logic;

const FORWARD_WEIGHT: f32 = 0.6;
const PHERO_SENSE_RADIUS: i32 = 4;
const RECRUIT_SENSE_THRESHOLD: f32 = 0.8;
const RECRUIT_GRADIENT_WEIGHT: f32 = 2.0;
const ATTACK_ENEMY_DETECT_RANGE: f32 = 80.0;

/// Following ants: steer along the Recruit pheromone gradient toward the player.
/// When the pheromone fades below threshold, revert to Foraging.
pub fn ant_follow_recruit_steering(
    clock: Res<SimClock>,
    grids: Option<Res<ColonyPheromones>>,
    mut query: Query<(&Transform, &mut Movement, &mut Ant, &ColonyMember, &mut TrailSense, &mut SteeringTarget, &mut SteeringWeights), Without<PlayerControlled>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();

    for (transform, mut movement, mut ant, colony, mut sense, mut steering_target, mut steering_weights) in &mut query {
        if ant.state != AntState::Following {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        let mut has_signal = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::Recruit);
                    if local >= RECRUIT_SENSE_THRESHOLD * 0.5 {
                        has_signal = true;
                        let grad = grid.sense_gradient(
                            gx, gy, PheromoneType::Recruit, fwd, PHERO_SENSE_RADIUS + 2,
                        );
                        if grad.length_squared() > 0.001 {
                            let jitter = Vec2::new(
                                rng.gen_range(-0.1..0.1),
                                rng.gen_range(-0.1..0.1),
                            );
                            let new_dir = (grad.normalize() * RECRUIT_GRADIENT_WEIGHT
                                + fwd * 0.3
                                + jitter)
                                .normalize_or_zero();
                            if new_dir != Vec2::ZERO {
                                *steering_target = SteeringTarget::Direction { target: new_dir };
                                *steering_weights = SteeringWeights {
                                    seek_weight: 1.0,
                                    separation_weight: 0.5,
                                    forward_weight: FORWARD_WEIGHT,
                                };
                            }
                        }
                    }
                }
            }
        }

        if has_signal {
            *sense = TrailSense::FollowingTrail;
        } else {
            ant.state = AntState::Foraging;
            *sense = TrailSense::Searching;
        }
    }
}

/// Attack-recruit followers: steer along the AttackRecruit pheromone gradient like
/// regular followers, but also aggressively target nearby enemies.
pub fn ant_attack_recruit_steering(
    clock: Res<SimClock>,
    grids: Option<Res<ColonyPheromones>>,
    mut query: Query<(&Transform, &mut Movement, &mut Ant, &ColonyMember, &mut TrailSense, &mut SteeringTarget, &mut SteeringWeights), Without<PlayerControlled>>,
    enemy_query: Query<(&Transform, &ColonyMember), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();

    for (transform, mut movement, mut ant, colony, mut sense, mut steering_target, mut steering_weights) in &mut query {
        if ant.state != AntState::Attacking {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        // Check for nearby enemies first — attack takes priority over gradient following
        let mut nearest_enemy: Option<(f32, Vec2)> = None;
        for (enemy_tf, enemy_col) in &enemy_query {
            if enemy_col.colony_id == colony.colony_id {
                continue;
            }
            let enemy_pos = enemy_tf.translation.truncate();
            let dist = pos.distance(enemy_pos);
            if dist < ATTACK_ENEMY_DETECT_RANGE {
                if nearest_enemy.is_none() || dist < nearest_enemy.unwrap().0 {
                    nearest_enemy = Some((dist, enemy_pos));
                }
            }
        }

        if let Some((dist, enemy_pos)) = nearest_enemy {
            let to_enemy = (enemy_pos - pos).normalize_or_zero();
            *steering_target = SteeringTarget::Direction { target: to_enemy };
            *steering_weights = SteeringWeights {
                seek_weight: 1.0,
                separation_weight: 0.5,
                forward_weight: FORWARD_WEIGHT,
            };
            // Within combat range, combat_detection system handles state transition
            if dist < 15.0 {
                ant.state = AntState::Defending;
                *sense = TrailSense::FollowingAlarm;
            } else {
                *sense = TrailSense::FollowingAttack;
            }
            continue;
        }

        // No enemies nearby — follow the AttackRecruit gradient
        let mut has_signal = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::AttackRecruit);
                    if local >= RECRUIT_SENSE_THRESHOLD * 0.5 {
                        has_signal = true;
                        let grad = grid.sense_gradient(
                            gx, gy, PheromoneType::AttackRecruit, fwd, PHERO_SENSE_RADIUS + 2,
                        );
                        if grad.length_squared() > 0.001 {
                            let jitter = Vec2::new(
                                rng.gen_range(-0.1..0.1),
                                rng.gen_range(-0.1..0.1),
                            );
                            let new_dir = (grad.normalize() * RECRUIT_GRADIENT_WEIGHT
                                + fwd * 0.3
                                + jitter)
                                .normalize_or_zero();
                            if new_dir != Vec2::ZERO {
                                *steering_target = SteeringTarget::Direction { target: new_dir };
                                *steering_weights = SteeringWeights {
                                    seek_weight: 1.0,
                                    separation_weight: 0.5,
                                    forward_weight: FORWARD_WEIGHT,
                                };
                            }
                        }
                    }
                }
            }
        }

        if has_signal {
            *sense = TrailSense::FollowingAttack;
        } else {
            ant.state = AntState::Foraging;
            *sense = TrailSense::Searching;
        }
    }
}

/// Underground followers: steer along the Recruit pheromone gradient in the nest.
pub fn nest_recruit_following(
    clock: Res<SimClock>,
    registry: Res<MapRegistry>,
    nest_phero_query: Query<(&NestPheromoneGrid, &NestGrid), With<MapMarker>>,
    mut query: Query<
        (&Transform, &mut Movement, &mut Ant, &MapId, &mut TrailSense),
        (With<NestTask>, Without<PlayerControlled>),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();

    for (transform, mut movement, mut ant, map_id, mut sense) in &mut query {
        if ant.state != AntState::Following {
            continue;
        }
        // Only for underground ants.
        if map_id.0 == registry.surface {
            continue;
        }

        let Ok((phero_grid, nest_grid)) = nest_phero_query.get(map_id.0) else {
            continue;
        };

        let pos = transform.translation.truncate();
        let Some((gx, gy)) = crate::plugins::nest_pheromone::world_to_nest_grid(pos) else {
            continue;
        };

        let local = phero_grid.get(gx, gy).recruit;
        if local < 0.5 {
            // No recruit signal — revert to idle-like foraging.
            ant.state = AntState::Foraging;
            *sense = TrailSense::Searching;
            continue;
        }

        let grad = phero_grid.sense_trail_recruit_gradient(nest_grid, gx, gy, true, 4);
        if grad.length_squared() > 0.001 {
            let jitter = Vec2::new(
                rng.gen_range(-0.1..0.1),
                rng.gen_range(-0.1..0.1),
            );
            let new_dir = (grad.normalize() * 0.7 + movement.direction * 0.3 + jitter)
                .normalize_or_zero();
            if new_dir != Vec2::ZERO {
                movement.direction = new_dir;
            }
        }

        *sense = TrailSense::FollowingTrail;
    }
}
