use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, DamageSource, Health};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct SpiderAiPlugin;

// ── Constants ──────────────────────────────────────────────────────

const SPIDER_ATTACK_RANGE: f32 = 20.0;
const SPIDER_ATTACK_DAMAGE: f32 = 10.0;
const SPIDER_ATTACK_COOLDOWN: f32 = 1.0;
const SPIDER_VISION_RANGE: f32 = 80.0;
/// Half-angle of the vision cone in radians (π/4 → 90° total cone).
const SPIDER_VISION_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_4;
const SPIDER_CHASE_SPEED: f32 = 60.0;
const SPIDER_PATROL_SPEED: f32 = 15.0;
/// Min/max seconds between patrol movements while idle.
const PATROL_INTERVAL_MIN: f32 = 15.0;
const PATROL_INTERVAL_MAX: f32 = 45.0;
/// How long a single patrol movement lasts.
const PATROL_DURATION_MIN: f32 = 1.0;
const PATROL_DURATION_MAX: f32 = 2.0;
/// How far the spider will lose interest in a chase target beyond vision range.
const CHASE_LEASH: f32 = SPIDER_VISION_RANGE * 1.5;

// ── Components ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiderState {
    Idle,
    Patrolling,
    Chasing,
}

#[derive(Component)]
pub struct Spider {
    pub hp: f32,
    pub attack_cooldown: f32,
    pub state: SpiderState,
    /// Current facing direction (unit vector).
    pub facing: Vec2,
    pub speed: f32,
    pub chase_target: Option<Entity>,
    /// Countdown until next patrol begins.
    pub patrol_timer: f32,
    /// Remaining duration of current patrol movement.
    pub patrol_remaining: f32,
    /// Direction for the current patrol movement.
    pub patrol_direction: Vec2,
}

// ── Plugin ─────────────────────────────────────────────────────────

impl Plugin for SpiderAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_spider)
            .add_systems(
                Update,
                (
                    spider_patrol,
                    spider_vision,
                    spider_chase,
                    spider_attack,
                )
                    .chain(),
            );
    }
}

// ── Spawn ──────────────────────────────────────────────────────────

fn spawn_spider(mut commands: Commands, config: Res<SimConfig>) {
    let mut rng = rand::thread_rng();
    let cx = config.world_width / 2.0;
    let cy = config.world_height / 2.0;

    let x = cx + rng.gen_range(-300.0..300.0);
    let y = cy + rng.gen_range(-300.0..300.0);

    let angle = rng.gen::<f32>() * std::f32::consts::TAU;
    let facing = Vec2::new(angle.cos(), angle.sin());

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.2, 0.15),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_xyz(x, y, 2.5),
        Spider {
            hp: 50.0,
            attack_cooldown: 0.0,
            state: SpiderState::Idle,
            facing,
            speed: 0.0,
            chase_target: None,
            patrol_timer: rng.gen_range(PATROL_INTERVAL_MIN..PATROL_INTERVAL_MAX),
            patrol_remaining: 0.0,
            patrol_direction: Vec2::ZERO,
        },
    ));
}

// ── Patrol ─────────────────────────────────────────────────────────

/// Rare, short repositioning — spiders are ambush predators.
fn spider_patrol(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<SimConfig>,
    mut query: Query<(&mut Transform, &mut Spider)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (mut transform, mut spider) in &mut query {
        // Only patrol when idle or already patrolling.
        if spider.state == SpiderState::Chasing {
            continue;
        }

        if spider.state == SpiderState::Patrolling {
            // Continue current patrol movement.
            spider.patrol_remaining -= dt;
            if spider.patrol_remaining <= 0.0 {
                spider.state = SpiderState::Idle;
                spider.speed = 0.0;
                spider.patrol_timer = rng.gen_range(PATROL_INTERVAL_MIN..PATROL_INTERVAL_MAX);
            } else {
                // Move in patrol direction.
                let delta = spider.patrol_direction * SPIDER_PATROL_SPEED * dt;
                transform.translation.x += delta.x;
                transform.translation.y += delta.y;

                // Clamp within world bounds.
                transform.translation.x = transform.translation.x.clamp(0.0, config.world_width);
                transform.translation.y = transform.translation.y.clamp(0.0, config.world_height);

                // Update facing to patrol direction.
                spider.facing = spider.patrol_direction;
            }
            continue;
        }

        // Idle — count down to next patrol.
        spider.patrol_timer -= dt;
        if spider.patrol_timer <= 0.0 {
            let angle = rng.gen::<f32>() * std::f32::consts::TAU;
            spider.patrol_direction = Vec2::new(angle.cos(), angle.sin());
            spider.patrol_remaining = rng.gen_range(PATROL_DURATION_MIN..PATROL_DURATION_MAX);
            spider.state = SpiderState::Patrolling;
            spider.speed = SPIDER_PATROL_SPEED;
            spider.facing = spider.patrol_direction;
        }
    }
}

// ── Vision Cone ────────────────────────────────────────────────────

/// Scan for ants within the vision cone. If one is spotted, begin chasing.
fn spider_vision(
    clock: Res<SimClock>,
    mut spider_query: Query<(Entity, &Transform, &mut Spider)>,
    ant_query: Query<(Entity, &Transform), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (_spider_entity, spider_tf, mut spider) in &mut spider_query {
        // Only scan when idle or patrolling.
        if spider.state == SpiderState::Chasing {
            continue;
        }

        let spider_pos = spider_tf.translation.truncate();
        let facing = spider.facing;

        let mut closest: Option<(Entity, f32)> = None;

        for (ant_entity, ant_tf) in &ant_query {
            let ant_pos = ant_tf.translation.truncate();
            let to_ant = ant_pos - spider_pos;
            let dist = to_ant.length();

            if dist > SPIDER_VISION_RANGE || dist < 0.1 {
                continue;
            }

            // Check vision cone: angle between facing and direction to ant.
            let to_ant_norm = to_ant / dist;
            let dot = facing.dot(to_ant_norm).clamp(-1.0, 1.0);
            let angle = dot.acos();

            if angle <= SPIDER_VISION_HALF_ANGLE {
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((ant_entity, dist));
                }
            }
        }

        if let Some((target, _)) = closest {
            spider.state = SpiderState::Chasing;
            spider.chase_target = Some(target);
            spider.speed = SPIDER_CHASE_SPEED;
        }
    }
}

// ── Chase ──────────────────────────────────────────────────────────

/// Move toward the chase target. Give up if the target is despawned or too far.
fn spider_chase(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<SimConfig>,
    mut spider_query: Query<(&mut Transform, &mut Spider)>,
    ant_query: Query<&Transform, (With<Ant>, Without<Spider>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut spider_tf, mut spider) in &mut spider_query {
        if spider.state != SpiderState::Chasing {
            continue;
        }

        let target_entity = match spider.chase_target {
            Some(e) => e,
            None => {
                spider.state = SpiderState::Idle;
                spider.speed = 0.0;
                continue;
            }
        };

        let target_tf = match ant_query.get(target_entity) {
            Ok(tf) => tf,
            Err(_) => {
                // Target despawned — return to idle.
                spider.state = SpiderState::Idle;
                spider.chase_target = None;
                spider.speed = 0.0;
                continue;
            }
        };

        let spider_pos = spider_tf.translation.truncate();
        let target_pos = target_tf.translation.truncate();
        let to_target = target_pos - spider_pos;
        let dist = to_target.length();

        // Lost the target — too far away.
        if dist > CHASE_LEASH {
            spider.state = SpiderState::Idle;
            spider.chase_target = None;
            spider.speed = 0.0;
            continue;
        }

        // Move toward target.
        if dist > 1.0 {
            let dir = to_target / dist;
            spider.facing = dir;
            let move_dist = (SPIDER_CHASE_SPEED * dt).min(dist);
            spider_tf.translation.x += dir.x * move_dist;
            spider_tf.translation.y += dir.y * move_dist;

            // Clamp within world bounds.
            spider_tf.translation.x = spider_tf.translation.x.clamp(0.0, config.world_width);
            spider_tf.translation.y = spider_tf.translation.y.clamp(0.0, config.world_height);
        }
    }
}

// ── Attack ─────────────────────────────────────────────────────────

/// Deal damage to ants within attack range while chasing.
fn spider_attack(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut spider_query: Query<(Entity, &Transform, &mut Spider)>,
    mut ant_query: Query<(Entity, &Transform, &mut Health), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (spider_entity, spider_tf, mut spider) in &mut spider_query {
        spider.attack_cooldown = (spider.attack_cooldown - dt).max(0.0);
        if spider.attack_cooldown > 0.0 {
            continue;
        }

        if spider.state != SpiderState::Chasing {
            continue;
        }

        let spider_pos = spider_tf.translation.truncate();

        // Prefer the chase target, but hit any ant in range.
        let mut hit = false;
        for (entity, ant_tf, mut health) in &mut ant_query {
            let dist = spider_pos.distance(ant_tf.translation.truncate());
            if dist < SPIDER_ATTACK_RANGE {
                health.apply_damage_from(SPIDER_ATTACK_DAMAGE, DamageSource::Spider, spider_entity);
                spider.attack_cooldown = SPIDER_ATTACK_COOLDOWN;
                hit = true;

                // If we hit our chase target, keep chasing it.
                // If we hit something else, switch target to it.
                spider.chase_target = Some(entity);
                break;
            }
        }

        // If chasing but nothing in attack range, keep chasing (handled by spider_chase).
        let _ = hit;
    }
}
