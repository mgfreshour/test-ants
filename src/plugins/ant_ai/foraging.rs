use bevy::prelude::*;
use rand::Rng;
use crate::components::ant::{Ant, AntJob, AntState, CarriedItem, ColonyMember, Movement, PlayerControlled, PositionHistory, SteeringTarget, SteeringWeights, TrailSense};
use crate::components::terrain::FoodSource;
use crate::resources::pheromone::ColonyPheromones;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::components::pheromone::PheromoneType;
use crate::sim_core::ant_logic;

const ANTI_BACKTRACK_WEIGHT: f32 = 0.35;
const FORWARD_WEIGHT: f32 = 0.6;
const SENSE_RANGE: f32 = 60.0;
const PHERO_SENSE_RADIUS: i32 = 4;
const PHERO_TRAIL_WEIGHT: f32 = 1.5;
const TRAIL_FOLLOW_CHANCE: u32 = 60;
const TRAIL_EPOCH_RATE: f32 = 0.33;
const MIN_SENSE_INTENSITY: f32 = 1.5;
const RECRUIT_SENSE_THRESHOLD: f32 = 0.8;
const RECRUIT_GRADIENT_WEIGHT: f32 = 2.0;

fn should_follow_trail(entity: Entity, elapsed: f32) -> bool {
    ant_logic::should_follow_trail(entity.index_u32(), elapsed, TRAIL_EPOCH_RATE, TRAIL_FOLLOW_CHANCE)
}

/// Foraging ants: follow FOOD pheromone gradient or random walk, biased away from HOME.
/// Within SENSE_RANGE of a food source, head straight for it.
/// If Recruit pheromone is detected, switch to Following state.
pub fn ant_forage_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    grids: Option<Res<ColonyPheromones>>,
    food_query: Query<&Transform, With<FoodSource>>,
    mut query: Query<(Entity, &Transform, &mut Movement, &mut Ant, &AntJob, &ColonyMember, &PositionHistory, &mut TrailSense, &mut SteeringTarget, &mut SteeringWeights), (Without<CarriedItem>, Without<PlayerControlled>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise;

    for (entity, transform, mut movement, mut ant, job, colony, history, mut sense, mut steering_target, mut steering_weights) in &mut query {
        // Only Forager and Unassigned ants use foraging AI
        if !matches!(job, AntJob::Forager | AntJob::Unassigned) {
            continue;
        }
        if ant.state != AntState::Foraging {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        // Check for Recruit / AttackRecruit pheromone — if strong enough, switch state
        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let attack_local = grid.get(gx, gy, PheromoneType::AttackRecruit);
                    let recruit_local = grid.get(gx, gy, PheromoneType::Recruit);
                    match ant_logic::recruit_entry_decision(attack_local, recruit_local, RECRUIT_SENSE_THRESHOLD) {
                        Some("attack") => { ant.state = AntState::Attacking; continue; }
                        Some("follow") => { ant.state = AntState::Following; continue; }
                        _ => {}
                    }
                }
            }
        }

        let mut nearest_food: Option<(f32, Vec2)> = None;
        for food_tf in &food_query {
            let food_pos = food_tf.translation.truncate();
            let dist = pos.distance(food_pos);
            if dist < SENSE_RANGE {
                if nearest_food.is_none() || dist < nearest_food.unwrap().0 {
                    nearest_food = Some((dist, food_pos));
                }
            }
        }

        if let Some((_, food_pos)) = nearest_food {
            let to_food = (food_pos - pos).normalize_or_zero();
            *steering_target = SteeringTarget::Direction { target: to_food };
            *steering_weights = SteeringWeights {
                seek_weight: 1.0,
                separation_weight: 0.5,
                forward_weight: FORWARD_WEIGHT,
            };
            *sense = TrailSense::BeelineFood;
            continue;
        }

        let mut pheromone_bias = Vec2::ZERO;
        let mut on_trail = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::Food);
                    if local >= MIN_SENSE_INTENSITY
                        && should_follow_trail(entity, clock.elapsed)
                    {
                        let food_grad =
                            grid.sense_gradient(gx, gy, PheromoneType::Food, fwd, PHERO_SENSE_RADIUS);
                        if food_grad.length_squared() > 0.01 {
                            let fg = food_grad.normalize();
                            let along = fwd.dot(fg) * fwd;
                            let lateral = fg - along;
                            if lateral.length_squared() > 0.001 {
                                pheromone_bias += lateral.normalize() * PHERO_TRAIL_WEIGHT;
                            }
                            on_trail = true;
                        }
                    }
                }
            }
        }

        *sense = if on_trail { TrailSense::FollowingFood } else { TrailSense::Searching };

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut target_dir = (fwd * FORWARD_WEIGHT
            + perturbed_fwd * noise_scale
            + pheromone_bias
            + momentum)
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
