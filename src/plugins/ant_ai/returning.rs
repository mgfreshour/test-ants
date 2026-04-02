use bevy::prelude::*;
use rand::Rng;
use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, Movement, PlayerControlled, PositionHistory, SteeringTarget, SteeringWeights, TrailSense};
use crate::components::map::MapPortal;
use crate::resources::active_map::MapRegistry;
use crate::resources::pheromone::ColonyPheromones;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::components::pheromone::PheromoneType;
use crate::sim_core::ant_logic;

const ANTI_BACKTRACK_WEIGHT: f32 = 0.35;
const FORWARD_WEIGHT: f32 = 0.6;
const SENSE_RANGE: f32 = 60.0;
const PHERO_SENSE_RADIUS: i32 = 4;
const PHERO_TRAIL_WEIGHT: f32 = 1.5;
const MIN_SENSE_INTENSITY: f32 = 1.5;

/// Returning ants: follow HOME pheromone gradient back to nest.
/// Within SENSE_RANGE of the nearest matching portal, head straight for it.
pub fn ant_return_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    grids: Option<Res<ColonyPheromones>>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut query: Query<(&Transform, &mut Movement, &Ant, &ColonyMember, &crate::components::map::MapId, &PositionHistory, &mut TrailSense, &mut SteeringTarget, &mut SteeringWeights), (With<CarriedItem>, Without<PlayerControlled>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise * 0.5;

    for (transform, mut movement, ant, colony, map_id, history, mut sense, mut steering_target, mut steering_weights) in &mut query {
        if ant.state != AntState::Returning {
            continue;
        }

        let pos = transform.translation.truncate();

        // Find the nearest portal on this ant's map leading toward a nest.
        let nest_pos = portal_query
            .iter()
            .filter(|p| {
                p.map == map_id.0
                    && p.target_map != registry.surface
                    && p.colony_id.map_or(true, |id| id == colony.colony_id)
            })
            .min_by(|a, b| {
                pos.distance(a.position)
                    .partial_cmp(&pos.distance(b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.position)
            .unwrap_or(config.nest_position);
        let to_nest = nest_pos - pos;
        let dist_to_nest = to_nest.length();

        if dist_to_nest < SENSE_RANGE {
            let target_dir = to_nest.normalize_or_zero();
            *steering_target = SteeringTarget::Direction { target: target_dir };
            *steering_weights = SteeringWeights {
                seek_weight: 1.0,
                separation_weight: 0.5,
                forward_weight: FORWARD_WEIGHT,
            };
            *sense = TrailSense::BeelineNest;
            continue;
        }

        let fwd = movement.direction;
        let mut bias = Vec2::ZERO;

        let mut on_trail = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::Home);
                    if local >= MIN_SENSE_INTENSITY {
                        let home_grad =
                            grid.sense_gradient(gx, gy, PheromoneType::Home, fwd, PHERO_SENSE_RADIUS);
                        if home_grad.length_squared() > 0.01 {
                            bias += home_grad.normalize() * PHERO_TRAIL_WEIGHT;
                            on_trail = true;
                        }
                    }
                }
            }
        }

        *sense = if on_trail { TrailSense::FollowingHome } else { TrailSense::Searching };

        if dist_to_nest > 1.0 {
            bias += to_nest.normalize() * 0.03;
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut target_dir = (fwd * FORWARD_WEIGHT + perturbed_fwd * noise_scale + bias + momentum)
            .normalize_or_zero();
        if target_dir == Vec2::ZERO {
            target_dir = perturbed_fwd;
        }

        *steering_target = SteeringTarget::Direction { target: target_dir };
        *steering_weights = SteeringWeights {
            seek_weight: 1.0,
            separation_weight: 0.5,
            forward_weight: FORWARD_WEIGHT,
        };
    }
}
