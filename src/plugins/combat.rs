use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{
    Ant, AntState, CarriedItem, CombatTarget, ColonyMember, DamageSource, Health, Movement,
    PlayerControlled, PositionHistory, TargetKind, TrailSense,
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
                    counter_attack_on_damage,
                    combat_target_selection,
                    fighting_steering,
                    fighting_apply_damage,
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

/// Assign a `CombatTarget` to any ant whose nearest hostile (ant or spider)
/// is within engagement range, and transition it to `AntState::Fighting`.
///
/// Engagement range is base `COMBAT_RANGE` for the player colony and
/// `engagement_range(COMBAT_RANGE, red_strategy.aggression)` for the red
/// colony, so late-game red raids are more proactive about committing.
///
/// Ants that were `Defending` but no longer have any enemy in their sensing
/// range demote back to `Foraging` via the existing hysteresis helpers; the
/// `Attacking` demote branch is intentionally suppressed here (recruit-based
/// `Attacking` still enters via `foraging.rs`).
fn combat_target_selection(
    mut commands: Commands,
    clock: Res<SimClock>,
    grids: Option<Res<ColonyPheromones>>,
    red_strategy: Res<RedColonyStrategy>,
    spider_query: Query<(Entity, &Transform, &MapId), With<Spider>>,
    mut ant_query: Query<
        (Entity, &Transform, &ColonyMember, &MapId, &mut Ant, Option<&CombatTarget>),
        With<Health>,
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Build a flat list of potential hostile contacts. Ants carry colony
    // identity; spiders are treated as "colony = SPIDER_COLONY" so the pure
    // helper rejects same-colony entries consistently.
    //
    // We also filter by MapId so ants in the blue nest can't engage ants in
    // the red nest (their world coords overlap but they're on separate maps).
    const SPIDER_COLONY: u32 = u32::MAX;
    let ant_snapshot: Vec<(u32, Vec2, u32, Entity)> = ant_query
        .iter()
        .map(|(e, t, c, m, _, _)| (e.index_u32(), t.translation.truncate(), c.colony_id, m.0))
        .collect();
    let spider_snapshot: Vec<(u32, Vec2, u32, Entity)> = spider_query
        .iter()
        .map(|(e, t, m)| (e.index_u32(), t.translation.truncate(), SPIDER_COLONY, m.0))
        .collect();

    // Index -> Entity lookup for the result of the pure helper.
    let mut entity_by_index: std::collections::HashMap<u32, (Entity, TargetKind)> =
        std::collections::HashMap::new();
    for (e, _, _, _, _, _) in ant_query.iter() {
        entity_by_index.insert(e.index_u32(), (e, TargetKind::Ant));
    }
    for (e, _, _) in spider_query.iter() {
        entity_by_index.insert(e.index_u32(), (e, TargetKind::Spider));
    }

    let red_engage = ant_logic::engagement_range(COMBAT_RANGE, red_strategy.aggression);

    for (entity, transform, colony, self_map, mut ant, current_target) in &mut ant_query {
        let pos = transform.translation.truncate();
        let engage = if colony.colony_id == RED_COLONY_ID {
            red_engage
        } else {
            COMBAT_RANGE
        };

        // Merge ant + spider snapshots, excluding self and entries on a
        // different map.
        let mut candidates: Vec<(u32, Vec2, u32)> =
            Vec::with_capacity(ant_snapshot.len() + spider_snapshot.len());
        for &(id, p, cid, map) in &ant_snapshot {
            if id != entity.index_u32() && map == self_map.0 {
                candidates.push((id, p, cid));
            }
        }
        for &(id, p, cid, map) in &spider_snapshot {
            if map == self_map.0 {
                candidates.push((id, p, cid));
            }
        }

        let picked = ant_logic::select_combat_target(pos, colony.colony_id, &candidates, engage);

        match (picked, current_target.is_some()) {
            (Some(target_idx), false) => {
                // Newly engaging: commit to the target and enter Fighting.
                if let Some(&(target_entity, kind)) = entity_by_index.get(&target_idx) {
                    commands.entity(entity).insert(CombatTarget { entity: target_entity, kind });
                    ant.set_state(AntState::Fighting, clock.elapsed);
                }
            }
            (Some(_), true) => {
                // Already engaged — do not retarget mid-fight (commitment
                // rule 2b). fighting_apply_damage drops the target when it
                // dies.
            }
            (None, true) => {
                // No enemy in range, but we're still committed. Leave
                // CombatTarget in place; fighting_apply_damage will verify
                // the target still exists next frame. This path matters when
                // the target is temporarily outside engage range — the
                // steering system will close the gap.
            }
            (None, false) => {
                // No target, no commitment. Handle stale Defending demote so
                // ants that sensed alarm but never found a target can return
                // to Foraging.
                if ant.state == AntState::Defending {
                    let (local_alarm, attack_intensity) = if let Some(ref all_grids) = grids {
                        if let Some(grid) = all_grids.get(colony.colony_id) {
                            if let Some((gx, gy)) = grid.world_to_grid(pos) {
                                (
                                    grid.get(gx, gy, PheromoneType::Alarm),
                                    grid.get(gx, gy, PheromoneType::AttackRecruit),
                                )
                            } else {
                                (0.0, 0.0)
                            }
                        } else {
                            (0.0, 0.0)
                        }
                    } else {
                        (0.0, 0.0)
                    };

                    let time_in_state = clock.elapsed - ant.state_entered_at;
                    match ant_logic::should_demote_from_defending(
                        true,
                        false,
                        local_alarm,
                        attack_intensity,
                        // Effectively-unreachable recruit threshold: combat
                        // no longer drives the Attacking branch (recruit
                        // pheromone in foraging.rs still does).
                        f32::INFINITY,
                        time_in_state,
                    ) {
                        ant_logic::DefendingExit::Stay => {}
                        ant_logic::DefendingExit::Foraging => {
                            ant.set_state(AntState::Foraging, clock.elapsed);
                        }
                        ant_logic::DefendingExit::Attacking => {
                            // Unreachable given the infinite threshold above;
                            // if it ever fires, prefer Foraging over the
                            // legacy raid branch.
                            ant.set_state(AntState::Foraging, clock.elapsed);
                        }
                    }
                }
            }
        }
    }
}

/// If an ant was just damaged by an identifiable attacker, commit to fighting
/// that attacker. Environmental damage (starvation, flood, lawnmower, player)
/// leaves `last_attacker = None` and is ignored here.
fn counter_attack_on_damage(
    mut commands: Commands,
    clock: Res<SimClock>,
    mut query: Query<
        (Entity, &mut Health, &mut Ant, Option<&CombatTarget>),
        With<ColonyMember>,
    >,
    ant_targets: Query<(), (With<Ant>, With<Health>)>,
    spider_targets: Query<(), With<Spider>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (entity, mut health, mut ant, current_target) in &mut query {
        let Some(attacker) = health.last_attacker else {
            continue;
        };

        // Consume the latch so we don't re-trigger every frame.
        health.last_attacker = None;

        if current_target.is_some() {
            // Already committed elsewhere — don't retarget.
            continue;
        }

        let kind = if ant_targets.get(attacker).is_ok() {
            TargetKind::Ant
        } else if spider_targets.get(attacker).is_ok() {
            TargetKind::Spider
        } else {
            // Attacker was despawned in the same frame, or not a valid
            // targetable kind. Skip.
            continue;
        };

        commands.entity(entity).insert(CombatTarget { entity: attacker, kind });
        ant.set_state(AntState::Fighting, clock.elapsed);
    }
}

/// Steer Fighting ants toward their committed target every frame. Overrides
/// whatever forager/returner steering produced earlier this tick.
fn fighting_steering(
    clock: Res<SimClock>,
    target_positions: Query<&Transform, Without<Ant>>,
    ant_target_positions: Query<&Transform, With<Ant>>,
    mut query: Query<(&Transform, &CombatTarget, &mut Movement, &mut TrailSense), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (ant_tf, target, mut movement, mut sense) in &mut query {
        // Target is either an Ant or Spider. We need a Transform for either.
        let target_tf = match target.kind {
            TargetKind::Ant => ant_target_positions.get(target.entity).ok(),
            TargetKind::Spider => target_positions.get(target.entity).ok(),
        };
        let Some(tt) = target_tf else {
            continue; // fighting_apply_damage will handle cleanup this tick
        };
        let to_target = tt.translation.truncate() - ant_tf.translation.truncate();
        let dist = to_target.length();

        // Once within combat range, plant and fight — zero velocity while
        // keeping the facing direction pointed at the target so sprite/overlay
        // hints still read correctly. Outside of range, chase the target.
        if dist < COMBAT_RANGE {
            movement.direction = Vec2::ZERO;
        } else {
            let dir = to_target.normalize_or_zero();
            if dir.length_squared() > 0.0 {
                movement.direction = dir;
            }
        }
        *sense = TrailSense::FollowingAlarm;
        let _ = clock; // silence unused if we later skip on pause
    }
}

/// Apply damage from Fighting ants to their committed target when in range,
/// and drop the commitment when the target is gone or dead.
fn fighting_apply_damage(
    mut commands: Commands,
    clock: Res<SimClock>,
    time: Res<Time>,
    mut ant_healths: Query<(Entity, &Transform, &mut Health), With<Ant>>,
    mut spider_healths: Query<(Entity, &Transform, &mut Spider)>,
    attackers: Query<
        (Entity, &Transform, &CombatTarget, &Sprite, Has<HitFlash>),
        With<Ant>,
    >,
    mut ants_to_reset: Query<&mut Ant>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    // First pass: figure out who is in range of their target and how much
    // damage to deal. We collect to a local Vec so we can then mutate the
    // target's Health/Spider safely (attacker and target queries disjoint by
    // entity, but borrow-checker-wise easier this way).
    struct PendingHit {
        attacker: Entity,
        target: Entity,
        kind: TargetKind,
        damage: f32,
        flash_color: Color,
    }

    let mut drops: Vec<Entity> = Vec::new();
    let mut hits: Vec<PendingHit> = Vec::new();

    for (attacker_entity, att_tf, target, sprite, has_flash) in &attackers {
        let att_pos = att_tf.translation.truncate();

        let target_alive_and_pos = match target.kind {
            TargetKind::Ant => ant_healths
                .get(target.entity)
                .ok()
                .map(|(_, t, h)| (t.translation.truncate(), h.current > 0.0)),
            TargetKind::Spider => spider_healths
                .get(target.entity)
                .ok()
                .map(|(_, t, s)| (t.translation.truncate(), s.hp > 0.0)),
        };

        let Some((target_pos, alive)) = target_alive_and_pos else {
            drops.push(attacker_entity);
            continue;
        };
        if ant_logic::should_drop_target(alive) {
            drops.push(attacker_entity);
            continue;
        }

        // Skip damage/flash refresh while the flash is still decaying — that
        // keeps the old DPS cadence (~1 hit per 0.15 s) while still letting
        // drop-detection run every frame.
        if has_flash {
            continue;
        }

        if att_pos.distance(target_pos) < COMBAT_RANGE {
            let base_dps = 3.0 + rng.gen_range(-0.5..0.5);
            let damage = base_dps * dt;
            hits.push(PendingHit {
                attacker: attacker_entity,
                target: target.entity,
                kind: target.kind,
                damage,
                flash_color: sprite.color,
            });
        }
    }

    // Apply damage.
    for hit in hits {
        match hit.kind {
            TargetKind::Ant => {
                if let Ok((_, _, mut health)) = ant_healths.get_mut(hit.target) {
                    health.apply_damage_from(hit.damage, DamageSource::EnemyAnt, hit.attacker);
                }
            }
            TargetKind::Spider => {
                if let Ok((_, _, mut spider)) = spider_healths.get_mut(hit.target) {
                    spider.hp -= hit.damage;
                }
            }
        }
        commands.entity(hit.attacker).insert(HitFlash {
            timer: 0.15,
            original_color: hit.flash_color,
        });
    }

    // Drop stale targets and return to Foraging.
    for e in drops {
        commands.entity(e).remove::<CombatTarget>();
        if let Ok(mut ant) = ants_to_reset.get_mut(e) {
            ant.set_state(AntState::Foraging, clock.elapsed);
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
    aggression: Res<crate::resources::colony::AggressionSettings>,
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

        // Player colony (0) listens to the UI slider; other colonies keep the
        // static default so AI-driven colonies stay balanced.
        let promote_threshold = if colony.colony_id == 0 {
            aggression.alarm_threshold
        } else {
            ant_logic::ALARM_PROMOTE_THRESHOLD
        };

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        if let Some((gx, gy)) = grid.world_to_grid(pos) {
            let local_alarm = grid.get(gx, gy, PheromoneType::Alarm);
            if local_alarm >= promote_threshold {
                let alarm_grad = grid.sense_gradient(gx, gy, PheromoneType::Alarm, fwd, ALARM_SENSE_RADIUS);
                let time_in_state = clock.elapsed - ant.state_entered_at;
                if ant_logic::should_promote_to_defending(
                    true,
                    local_alarm,
                    alarm_grad.length_squared(),
                    time_in_state,
                    promote_threshold,
                ) {
                    ant.set_state(AntState::Defending, clock.elapsed);
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

    let map_id = MapId(registry.surface);

    // Pit visual — larger, slightly transparent circle.
    commands.spawn((
        Sprite {
            color: Color::srgba(0.6, 0.55, 0.4, 0.3),
            custom_size: Some(Vec2::splat(80.0)),
            ..default()
        },
        Transform::from_xyz(x, y, 1.2),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ant::{Caste, Health};
    use crate::resources::colony::AggressionSettings;
    use crate::resources::simulation::{SimClock, SimSpeed};

    /// Build a minimal Bevy App with just the systems needed to exercise
    /// combat target selection, steering, and damage application. No
    /// rendering, no pheromones, no spiders spawned — callers spawn the
    /// entities they want.
    fn make_app() -> App {
        let mut app = App::new();
        // No MinimalPlugins — we don't want TimePlugin resetting Time every
        // frame. We manage Time manually in `tick()`.
        app.init_resource::<Time>();
        app.insert_resource(SimClock { tick: 0, elapsed: 0.0, speed: SimSpeed::Normal });
        app.insert_resource(RedColonyStrategy::default());
        app.insert_resource(AggressionSettings::default());
        app.add_systems(
            Update,
            (
                counter_attack_on_damage,
                combat_target_selection,
                fighting_steering,
                fighting_apply_damage,
                hit_flash_decay,
            )
                .chain(),
        );
        app
    }

    fn spawn_test_ant(
        app: &mut App,
        colony_id: u32,
        pos: Vec2,
        hp: f32,
    ) -> Entity {
        // Shared dummy map so combat_target_selection's map filter allows
        // both spawn_test_ant entities to see each other. Stored as a
        // resource so multiple spawns reuse the same id.
        let map_entity = if let Some(m) = app.world().get_resource::<TestMap>() {
            m.0
        } else {
            let e = app.world_mut().spawn_empty().id();
            app.world_mut().insert_resource(TestMap(e));
            e
        };
        app.world_mut()
            .spawn((
                Ant {
                    caste: Caste::Worker,
                    state: AntState::Foraging,
                    age: 0.0,
                    hunger: 0.0,
                    state_entered_at: 0.0,
                },
                Movement { speed: 20.0, direction: Vec2::X },
                Health { current: hp, max: hp, last_damage_source: None, last_attacker: None },
                ColonyMember { colony_id },
                MapId(map_entity),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                Sprite::default(),
                TrailSense::default(),
            ))
            .id()
    }

    #[derive(Resource)]
    struct TestMap(Entity);

    fn tick(app: &mut App, dt_secs: f32) {
        {
            let mut clock = app.world_mut().resource_mut::<SimClock>();
            clock.tick += 1;
            clock.elapsed += dt_secs;
        }
        {
            // MinimalPlugins advances Time using real wall-clock deltas,
            // which are unreliable in tests. Stamp a known delta so
            // fighting_apply_damage deals predictable DPS per tick.
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(dt_secs));
        }
        app.update();
    }

    #[test]
    fn sim_plugin_combat_commits_target_until_dead() {
        let mut app = make_app();

        // Two opposing ants just inside engage range.
        let attacker = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);
        let victim = spawn_test_ant(&mut app, 1, Vec2::new(COMBAT_RANGE - 1.0, 0.0), 10.0);

        // First frame: combat_target_selection should latch both ants onto
        // each other and flip them to Fighting.
        tick(&mut app, 0.016);

        let attacker_target = app
            .world()
            .entity(attacker)
            .get::<CombatTarget>()
            .copied()
            .expect("attacker should have a CombatTarget after one frame");
        assert_eq!(attacker_target.entity, victim);
        assert_eq!(
            app.world().entity(attacker).get::<Ant>().unwrap().state,
            AntState::Fighting
        );

        // Run enough ticks to kill the victim at ~3 DPS on 10 HP.
        for _ in 0..500 {
            let victim_hp = app.world().entity(victim).get::<Health>().unwrap().current;
            if victim_hp <= 0.0 {
                break;
            }
            tick(&mut app, 0.05);
        }

        let victim_hp = app.world().entity(victim).get::<Health>().unwrap().current;
        assert_eq!(victim_hp, 0.0, "victim should be at zero HP");

        // Attacker kept its target throughout the fight.
        assert!(
            app.world().entity(attacker).get::<CombatTarget>().is_some(),
            "attacker should still hold CombatTarget until should_drop_target fires"
        );
    }

    #[test]
    fn sim_plugin_combat_drops_target_when_target_despawned() {
        let mut app = make_app();
        let attacker = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);
        let victim = spawn_test_ant(&mut app, 1, Vec2::new(COMBAT_RANGE - 1.0, 0.0), 10.0);

        // Frame 1: latch.
        tick(&mut app, 0.016);
        assert!(app.world().entity(attacker).get::<CombatTarget>().is_some());

        // Simulate death: despawn the victim directly.
        app.world_mut().entity_mut(victim).despawn();

        // Frame 2: fighting_apply_damage should see the target is gone and
        // drop the CombatTarget + return attacker to Foraging.
        tick(&mut app, 0.016);

        assert!(
            app.world().entity(attacker).get::<CombatTarget>().is_none(),
            "CombatTarget should be removed once target despawns"
        );
        assert_eq!(
            app.world().entity(attacker).get::<Ant>().unwrap().state,
            AntState::Foraging,
            "attacker should return to Foraging after dropping target"
        );
    }

    #[test]
    fn sim_plugin_counter_attack_latches_onto_attacker() {
        let mut app = make_app();
        // Keep them OUT of engage range so combat_target_selection won't
        // auto-commit — we want to exercise the counter-attack path only.
        let victim = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);
        let attacker = spawn_test_ant(&mut app, 1, Vec2::new(100.0, 0.0), 10.0);

        // Pretend the attacker already hit the victim from range.
        {
            let mut victim_mut = app.world_mut().entity_mut(victim);
            let mut h = victim_mut.get_mut::<Health>().unwrap();
            h.last_attacker = Some(attacker);
            h.last_damage_source = Some(DamageSource::EnemyAnt);
        }

        tick(&mut app, 0.016);

        let target = app.world().entity(victim).get::<CombatTarget>().copied();
        assert_eq!(
            target.map(|t| t.entity),
            Some(attacker),
            "victim should lock onto its attacker via counter-attack"
        );
        assert_eq!(
            app.world().entity(victim).get::<Ant>().unwrap().state,
            AntState::Fighting
        );
    }

    #[test]
    fn sim_plugin_fighting_ant_plants_when_target_is_in_range() {
        let mut app = make_app();
        let attacker = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);
        let _victim = spawn_test_ant(&mut app, 1, Vec2::new(COMBAT_RANGE - 2.0, 0.0), 100.0);

        // One tick to latch + steer.
        tick(&mut app, 0.016);

        let movement = app.world().entity(attacker).get::<Movement>().unwrap();
        assert_eq!(
            movement.direction,
            Vec2::ZERO,
            "attacker should zero its movement direction when within COMBAT_RANGE"
        );
    }

    #[test]
    fn sim_plugin_fighting_ant_chases_when_target_out_of_range() {
        let mut app = make_app();
        let attacker = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);
        // Place victim within engagement range for initial target selection,
        // then move them just past COMBAT_RANGE so the steer-toward path runs.
        let victim = spawn_test_ant(&mut app, 1, Vec2::new(COMBAT_RANGE - 1.0, 0.0), 100.0);

        tick(&mut app, 0.016);
        assert!(app.world().entity(attacker).get::<CombatTarget>().is_some());

        // Slide the victim further away to force chase.
        {
            let mut victim_mut = app.world_mut().entity_mut(victim);
            let mut t = victim_mut.get_mut::<Transform>().unwrap();
            t.translation.x = COMBAT_RANGE + 5.0;
        }
        tick(&mut app, 0.016);

        let movement = app.world().entity(attacker).get::<Movement>().unwrap();
        assert!(
            movement.direction.x > 0.9,
            "attacker should steer toward the target when out of range, got {:?}",
            movement.direction
        );
    }

    #[test]
    fn sim_plugin_starvation_damage_does_not_trigger_counter_attack() {
        let mut app = make_app();
        let ant = spawn_test_ant(&mut app, 0, Vec2::ZERO, 10.0);

        // Apply environmental damage — no attacker entity.
        {
            let mut ant_mut = app.world_mut().entity_mut(ant);
            let mut h = ant_mut.get_mut::<Health>().unwrap();
            h.apply_damage(3.0, DamageSource::Starvation);
        }

        tick(&mut app, 0.016);

        assert!(
            app.world().entity(ant).get::<CombatTarget>().is_none(),
            "Starvation must not produce a CombatTarget"
        );
    }
}
