use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{
    Ant, AntState, CarriedItem, ColonyMember, DamageSource, Health, Movement, PlayerControlled,
    PositionHistory, TrailSense,
};
use crate::components::map::MapId;
use crate::components::pheromone::PheromoneType;
use crate::resources::active_map::{MapRegistry, viewing_surface};
use crate::resources::pheromone::ColonyPheromones;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::plugins::spider_ai::Spider;
use crate::sim_core::ant_logic;

pub struct CombatPlugin;

const RED_COLONY_ID: u32 = 1;
const COMBAT_RANGE: f32 = 15.0;
const ALARM_SENSE_RADIUS: i32 = 6;

/// Antlion pit trap — ants entering the pit slide to the center and take damage.
#[derive(Component)]
pub struct Antlion {
    pub hp: f32,
    pub pit_radius: f32,
    pub damage_per_sec: f32,
    pub pull_strength: f32,
}

/// Brief flash effect on an entity that was just hit in combat.
#[derive(Component)]
pub struct HitFlash {
    pub timer: f32,
    pub original_color: Color,
}

#[derive(Component)]
pub struct Corpse {
    pub timer: f32,
}

#[derive(Component)]
pub struct EnemyColonyNest;

/// Red colony AI strategy — tracks aggression ramp and raid timing.
#[derive(Resource)]
pub struct RedColonyStrategy {
    /// Current aggression level (0.0–1.0), computed from elapsed time.
    pub aggression: f32,
    /// Time since last raid attempt.
    pub raid_timer: f32,
    /// Base interval between raid attempts (seconds).
    pub base_raid_interval: f32,
    /// Duration over which aggression ramps from 0.1 to 0.9.
    pub ramp_duration: f32,
}

impl Default for RedColonyStrategy {
    fn default() -> Self {
        Self {
            aggression: 0.1,
            raid_timer: 0.0,
            base_raid_interval: 60.0, // attempt a raid every ~60s at low aggression
            ramp_duration: 300.0,     // reach max aggression after 5 minutes
        }
    }
}

#[derive(Resource, Default)]
pub struct GameResult {
    pub decided: bool,
    pub player_won: bool,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {

        app.init_resource::<GameResult>()
            .init_resource::<RedColonyStrategy>()
            .add_systems(Startup, (spawn_red_colony, spawn_antlion))
            .add_systems(
                Update,
                (
                    red_colony_strategy_update,
                    ant_combat_detection,
                    combat_resolution,
                    alarm_pheromone_deposit,
                    alarm_response_steering,
                    antlion_ai,
                    death_system,
                    corpse_decay,
                    hit_flash_decay,
                    victory_defeat_check,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                player_attack.run_if(viewing_surface),
            );
    }
}

fn spawn_red_colony(mut commands: Commands, config: Res<SimConfig>, registry: Res<MapRegistry>) {
    let mut rng = rand::thread_rng();

    let red_nest = Vec2::new(
        config.world_width - config.nest_position.x,
        config.world_height - config.nest_position.y,
    );

    let size = config.tile_size * 3.0;
    // Red colony nest entrance marker — lives on the surface map.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.5, 0.15, 0.1),
            custom_size: Some(Vec2::new(size, size)),
            ..default()
        },
        Transform::from_xyz(red_nest.x, red_nest.y, 1.0),
        EnemyColonyNest,
        MapId(registry.surface),
    ));

    for _ in 0..30 {
        let offset_x = rng.gen_range(-20.0..20.0);
        let offset_y = rng.gen_range(-20.0..20.0);

        commands.spawn((
            Sprite {
                color: Color::srgb(0.7, 0.15, 0.1),
                custom_size: Some(Vec2::splat(4.0)),
                ..default()
            },
            Transform::from_xyz(red_nest.x + offset_x, red_nest.y + offset_y, 2.0),
            Ant::new_worker(),
            Movement::with_random_direction(config.ant_speed_worker, &mut rng),
            Health::worker(),
            ColonyMember { colony_id: RED_COLONY_ID },
            PositionHistory::default(),
            TrailSense::default(),
            MapId(registry.surface),
        ));
    }
}

fn ant_combat_detection(
    clock: Res<SimClock>,
    grids: Option<Res<ColonyPheromones>>,
    mut query: Query<(Entity, &Transform, &ColonyMember, &mut Ant)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let snapshot: Vec<(Entity, Vec2, u32)> = query
        .iter()
        .map(|(e, t, c, _)| (e, t.translation.truncate(), c.colony_id))
        .collect();

    let mut in_combat: Vec<Entity> = Vec::new();

    for i in 0..snapshot.len() {
        for j in 0..snapshot.len() {
            if i == j || snapshot[i].2 == snapshot[j].2 {
                continue;
            }
            if snapshot[i].1.distance(snapshot[j].1) < COMBAT_RANGE {
                in_combat.push(snapshot[i].0);
                break;
            }
        }
    }

    for (entity, transform, colony, mut ant) in &mut query {
        if in_combat.contains(&entity) {
            ant.state = AntState::Defending;
        } else if ant.state == AntState::Defending {
            // Return to Attacking if AttackRecruit pheromone is present, else Foraging
            let mut attack_intensity = 0.0f32;
            if let Some(ref all_grids) = grids {
                if let Some(grid) = all_grids.get(colony.colony_id) {
                    let pos = transform.translation.truncate();
                    if let Some((gx, gy)) = grid.world_to_grid(pos) {
                        attack_intensity = grid.get(gx, gy, PheromoneType::AttackRecruit);
                    }
                }
            }
            ant.state = match ant_logic::post_combat_state(attack_intensity, 0.4) {
                "attacking" => AntState::Attacking,
                _ => AntState::Foraging,
            };
        }
    }
}

fn combat_resolution(
    mut commands: Commands,
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(Entity, &Transform, &ColonyMember, &Ant, &mut Health, &Sprite), Without<HitFlash>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    let combatants: Vec<(Vec2, u32)> = query
        .iter()
        .filter(|(_, _, _, ant, _, _)| ant.state == AntState::Defending || ant.state == AntState::Fighting)
        .map(|(_, t, c, _, _, _)| (t.translation.truncate(), c.colony_id))
        .collect();

    for (entity, transform, colony, ant, mut health, sprite) in &mut query {
        if ant.state != AntState::Defending && ant.state != AntState::Fighting {
            continue;
        }

        let pos = transform.translation.truncate();
        let nearby_enemies = combatants
            .iter()
            .filter(|(p, cid)| *cid != colony.colony_id && p.distance(pos) < COMBAT_RANGE * 2.0)
            .count();

        if nearby_enemies > 0 {
            let base_dps = 3.0 + rng.gen_range(-0.5..0.5);
            let damage = base_dps * nearby_enemies as f32 * dt;
            health.apply_damage(damage, DamageSource::EnemyAnt);

            // Apply hit flash effect.
            commands.entity(entity).insert(HitFlash {
                timer: 0.15,
                original_color: sprite.color,
            });
        }
    }
}

fn alarm_pheromone_deposit(
    clock: Res<SimClock>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    query: Query<(&Transform, &Ant, &ColonyMember)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Some(ref mut all_grids) = grids else { return };

    for (transform, ant, colony) in &query {
        if ant.state != AntState::Defending && ant.state != AntState::Fighting {
            continue;
        }
        let Some(grid) = all_grids.get_mut(colony.colony_id) else {
            continue;
        };
        let pos = transform.translation.truncate();
        if let Some((gx, gy)) = grid.world_to_grid(pos) {
            grid.deposit(gx, gy, PheromoneType::Alarm, 5.0, 200.0);
        }
    }
}

fn alarm_response_steering(
    clock: Res<SimClock>,
    grids: Option<Res<ColonyPheromones>>,
    mut query: Query<
        (&Transform, &ColonyMember, &mut Movement, &mut Ant, &mut TrailSense),
        (Without<PlayerControlled>, Without<CarriedItem>),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Some(ref all_grids) = grids else { return };

    for (transform, colony, mut movement, mut ant, mut sense) in &mut query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let Some(grid) = all_grids.get(colony.colony_id) else {
            continue;
        };

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        if let Some((gx, gy)) = grid.world_to_grid(pos) {
            if grid.get(gx, gy, PheromoneType::Alarm) >= 1.0 {
                let alarm_grad = grid.sense_gradient(gx, gy, PheromoneType::Alarm, fwd, ALARM_SENSE_RADIUS);
                if alarm_grad.length_squared() > 0.5 {
                    ant.state = AntState::Defending;
                    *sense = TrailSense::FollowingAlarm;
                    movement.direction = alarm_grad.normalize();
                }
            }
        }
    }
}

fn player_attack(
    mut events: MessageReader<crate::plugins::player::PlayerAction>,
    player_query: Query<(&Transform, &ColonyMember), With<PlayerControlled>>,
    mut enemy_query: Query<(&Transform, &ColonyMember, &mut Health), (With<Ant>, Without<PlayerControlled>)>,
    mut spider_query: Query<(&Transform, &mut Spider)>,
) {
    let triggered = events.read().any(|a| *a == crate::plugins::player::PlayerAction::Attack);
    if !triggered {
        return;
    }

    let Ok((player_tf, player_colony)) = player_query.single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let attack_range = 20.0;
    let damage = 5.0;

    let mut hit = false;
    for (tf, colony, mut health) in &mut enemy_query {
        if colony.colony_id == player_colony.colony_id {
            continue;
        }
        if player_pos.distance(tf.translation.truncate()) < attack_range {
            health.apply_damage(damage, DamageSource::Player);
            hit = true;
            break;
        }
    }

    if !hit {
        for (tf, mut spider) in &mut spider_query {
            if player_pos.distance(tf.translation.truncate()) < attack_range {
                spider.hp -= damage;
                break;
            }
        }
    }
}

fn death_system(
    mut commands: Commands,
    ant_query: Query<(Entity, &Transform, &Health, &Ant), With<Ant>>,
    spider_query: Query<(Entity, &Transform, &Spider)>,
    antlion_query: Query<(Entity, &Transform, &Antlion)>,
) {
    let mut rng = rand::thread_rng();

    for (entity, transform, health, ant) in &ant_query {
        if health.current <= 0.0 {
            let pos = transform.translation;
            let reason = health.last_damage_source.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into());
            info!("Ant died: reason={}, age={:.0}s, pos=({:.0},{:.0})", reason, ant.age, pos.x, pos.y);
            commands.entity(entity).despawn();
            // Corpse
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.3, 0.2, 0.1, 0.6),
                    custom_size: Some(Vec2::splat(3.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 1.8),
                Corpse { timer: 5.0 },
            ));
            // Death particle burst — 3 small particles that fly outward and fade.
            for _ in 0..3 {
                let angle = rng.gen::<f32>() * std::f32::consts::TAU;
                let offset = Vec2::new(angle.cos(), angle.sin()) * rng.gen_range(2.0..8.0);
                commands.spawn((
                    Sprite {
                        color: Color::srgba(0.5, 0.2, 0.1, 0.8),
                        custom_size: Some(Vec2::splat(1.5)),
                        ..default()
                    },
                    Transform::from_xyz(pos.x + offset.x, pos.y + offset.y, 2.0),
                    Corpse { timer: 0.5 },
                ));
            }
        }
    }

    for (entity, transform, spider) in &spider_query {
        if spider.hp <= 0.0 {
            let pos = transform.translation;
            commands.entity(entity).despawn();
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.3, 0.2, 0.1, 0.8),
                    custom_size: Some(Vec2::splat(10.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 1.8),
                Corpse { timer: 10.0 },
            ));
        }
    }

    for (entity, transform, antlion) in &antlion_query {
        if antlion.hp <= 0.0 {
            let pos = transform.translation;
            commands.entity(entity).despawn();
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.4, 0.35, 0.2, 0.7),
                    custom_size: Some(Vec2::splat(8.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 1.8),
                Corpse { timer: 8.0 },
            ));
        }
    }
}

fn corpse_decay(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<SimClock>,
    mut query: Query<(Entity, &mut Corpse, &mut Sprite)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (entity, mut corpse, mut sprite) in &mut query {
        corpse.timer -= dt;
        let alpha = (corpse.timer / 5.0).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(alpha * 0.6);
        if corpse.timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn victory_defeat_check(
    clock: Res<SimClock>,
    mut result: ResMut<GameResult>,
    query: Query<&ColonyMember, With<Ant>>,
) {
    if result.decided || clock.elapsed < 5.0 {
        return;
    }

    let mut player_count = 0u32;
    let mut enemy_count = 0u32;

    for member in &query {
        if member.colony_id == 0 {
            player_count += 1;
        } else {
            enemy_count += 1;
        }
    }

    if player_count == 0 && enemy_count > 0 {
        result.decided = true;
        result.player_won = false;
        info!("DEFEAT -- Your colony has been destroyed!");
    } else if enemy_count == 0 && player_count > 0 {
        result.decided = true;
        result.player_won = true;
        info!("VICTORY -- The enemy colony has been destroyed!");
    }
}

// ── Antlion ────────────────────────────────────────────────────────

fn spawn_antlion(mut commands: Commands, config: Res<SimConfig>, registry: Res<MapRegistry>) {
    let mut rng = rand::thread_rng();

    let player_nest = config.nest_position;
    let enemy_nest = Vec2::new(
        config.world_width - config.nest_position.x,
        config.world_height - config.nest_position.y,
    );
    let min_nest_dist = 150.0;

    // Pick a position that isn't too close to either nest.
    let (x, y) = loop {
        let x = rng.gen_range(200.0..config.world_width - 200.0);
        let y = rng.gen_range(200.0..config.world_height - 200.0);
        let pos = Vec2::new(x, y);
        if pos.distance(player_nest) > min_nest_dist && pos.distance(enemy_nest) > min_nest_dist {
            break (x, y);
        }
    };

    info!("Antlion spawned at ({:.0}, {:.0}), player_nest=({:.0},{:.0}), enemy_nest=({:.0},{:.0})",
        x, y, player_nest.x, player_nest.y, enemy_nest.x, enemy_nest.y);

    let map_id = MapId(registry.surface);

    // Pit visual — larger, slightly transparent circle.
    commands.spawn((
        Sprite {
            color: Color::srgba(0.6, 0.55, 0.4, 0.3),
            custom_size: Some(Vec2::splat(80.0)),
            ..default()
        },
        Transform::from_xyz(x, y, 0.8),
        crate::plugins::ant_sprites::AntlionPit,
        map_id,
    ));

    // Antlion entity at the center.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.5, 0.4, 0.2),
            custom_size: Some(Vec2::splat(8.0)),
            ..default()
        },
        Transform::from_xyz(x, y, 2.5),
        Antlion {
            hp: 40.0,
            pit_radius: 40.0,
            damage_per_sec: 4.0,
            pull_strength: 50.0,
        },
        map_id,
    ));
}

/// Antlion pit trap: ants within radius slide toward center and take damage.
fn antlion_ai(
    clock: Res<SimClock>,
    time: Res<Time>,
    antlion_query: Query<(&Transform, &Antlion)>,
    mut ant_query: Query<(&mut Transform, &mut Health), (With<Ant>, Without<Antlion>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (pit_tf, antlion) in &antlion_query {
        let pit_pos = pit_tf.translation.truncate();

        for (mut ant_tf, mut health) in &mut ant_query {
            let ant_pos = ant_tf.translation.truncate();
            let dist = pit_pos.distance(ant_pos);

            if dist < antlion.pit_radius && dist > 1.0 {
                // Pull toward center — stronger as ant gets closer.
                let pull_dir = (pit_pos - ant_pos).normalize();
                let pull_factor = 1.0 - (dist / antlion.pit_radius);
                let pull = pull_dir * antlion.pull_strength * pull_factor * dt;
                ant_tf.translation.x += pull.x;
                ant_tf.translation.y += pull.y;

                // Damage increases as ant nears center.
                if dist < antlion.pit_radius * 0.5 {
                    health.apply_damage(antlion.damage_per_sec * dt, DamageSource::Antlion);
                }
            }
        }
    }
}

// ── Red Colony Strategy ────────────────────────────────────────────

/// Updates red colony aggression over time and triggers raids.
fn red_colony_strategy_update(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut strategy: ResMut<RedColonyStrategy>,
    mut query: Query<(&Transform, &mut Movement, &ColonyMember, &mut Ant)>,
    config: Res<SimConfig>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    // Update aggression curve.
    strategy.aggression = ant_logic::red_aggression_curve(clock.elapsed, strategy.ramp_duration);
    strategy.raid_timer += dt;

    // Check if it's time for a raid.
    if ant_logic::should_raid(strategy.aggression, strategy.raid_timer, strategy.base_raid_interval) {
        strategy.raid_timer = 0.0;

        // Redirect a fraction of red foraging ants toward the player nest.
        let raid_fraction = strategy.aggression;
        let target = config.nest_position;
        let mut rng = rand::thread_rng();

        for (transform, mut movement, colony, mut ant) in &mut query {
            if colony.colony_id != RED_COLONY_ID {
                continue;
            }
            if ant.state != AntState::Foraging {
                continue;
            }
            if rng.gen::<f32>() > raid_fraction {
                continue;
            }
            // Redirect toward player nest.
            ant.state = AntState::Attacking;
            let ant_pos = transform.translation.truncate();
            let dir = (target - ant_pos).normalize_or_zero();
            if dir.length_squared() > 0.0 {
                movement.direction = dir;
            }
        }
    }
}

// ── Combat Visual Effects ──────────────────────────────────────────

/// Decays hit flash timers and restores original sprite color.
fn hit_flash_decay(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<SimClock>,
    mut query: Query<(Entity, &mut HitFlash, &mut Sprite)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (entity, mut flash, mut sprite) in &mut query {
        flash.timer -= dt;
        if flash.timer <= 0.0 {
            sprite.color = flash.original_color;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            // Lerp between white flash and original color.
            let t = flash.timer / 0.15; // 0.15s flash duration
            let white = Color::WHITE;
            let orig = flash.original_color;
            sprite.color = lerp_color(orig, white, t.clamp(0.0, 1.0));
        }
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}
