use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, DamageSource, Health};
use crate::components::map::MapId;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::plugins::camera::MainCamera;
use crate::plugins::player::ToastQueue;

/// Environment hazards and dynamic events: rain, flooding, footsteps, lawnmower, pesticide, day/night.
pub struct EnvironmentPlugin;

/// Message to trigger a hazard event manually.
#[derive(Message)]
pub enum HazardEvent {
    TriggerRain,
    TriggerFootstep,
    TriggerLawnmower,
    TriggerPesticide,
}

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnvironmentState::default())
            .add_message::<HazardEvent>()
            .add_systems(Update, (
                handle_manual_hazards,
                update_animated_hazards,
                update_day_night_cycle,
                update_rain_state,
                update_hazard_events,
            ).chain())
            .add_systems(Startup, spawn_night_overlay)
            .add_systems(Update, (
                sync_hazard_visuals,
                update_rain_visuals,
                update_night_overlay,
            ));
    }
}

/// Global environment state tracking weather, time, and active hazards.
#[derive(Resource, Clone, Debug)]
pub struct EnvironmentState {
    /// 0.0 = midnight, 0.5 = noon, cycles every 3 minutes (180 seconds)
    pub time_of_day: f32,
    /// true if currently raining
    pub is_raining: bool,
    /// time until next rain event (seconds)
    pub rain_timer: f32,
    /// base evaporation multiplier (1.0 normal, 10.0 during rain)
    pub evaporation_multiplier: f32,
    /// water level in nest (0.0 = dry, 1.0 = flooded)
    pub flood_level: f32,
    /// active hazard zones (id, zone)
    pub active_hazards: Vec<(u64, HazardZone)>,
    /// Active lawnmowers sweeping across the yard.
    pub active_mowers: Vec<ActiveMower>,
    /// Active footstep paths (human walking across the yard).
    pub active_footstep_paths: Vec<ActiveFootstepPath>,
    /// Fractional raindrop accumulator (carries sub-frame remainders).
    pub rain_spawn_accum: f32,
}

/// A lawnmower that sweeps horizontally across the yard.
#[derive(Clone, Debug)]
pub struct ActiveMower {
    /// Fixed Y coordinate of the mow line.
    pub y: f32,
    /// Current X position.
    pub current_x: f32,
    /// X position where the mower finishes.
    pub end_x: f32,
    /// Pixels per sim-second.
    pub speed: f32,
    /// Direction sign: 1.0 (left→right) or -1.0 (right→left).
    pub direction: f32,
    /// Damage radius.
    pub radius: f32,
    /// Hazard id for the moving zone (so visuals track it).
    pub hazard_id: u64,
}

/// A human walking across the yard, producing sequential footsteps.
#[derive(Clone, Debug)]
pub struct ActiveFootstepPath {
    /// Normalised direction of travel.
    pub direction: Vec2,
    /// Perpendicular vector (for left/right offset).
    pub perpendicular: Vec2,
    /// Position of the *next* footstep.
    pub next_pos: Vec2,
    /// Distance between successive steps along the direction.
    pub stride_length: f32,
    /// Lateral offset magnitude (alternates ±).
    pub lateral_offset: f32,
    /// true = next step is left foot.
    pub is_left: bool,
    /// Time until the next footstep lands.
    pub step_timer: f32,
    /// Interval between footsteps.
    pub step_interval: f32,
    /// Number of steps remaining.
    pub steps_remaining: u32,
    /// Hazard id of the currently visible footstep (if any).
    pub current_hazard_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HazardKind {
    Footstep,
    Lawnmower,
    Pesticide,
}

impl From<HazardKind> for DamageSource {
    fn from(kind: HazardKind) -> Self {
        match kind {
            HazardKind::Footstep => DamageSource::Footstep,
            HazardKind::Lawnmower => DamageSource::Lawnmower,
            HazardKind::Pesticide => DamageSource::Pesticide,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HazardZone {
    pub kind: HazardKind,
    pub position: Vec2,
    pub radius: f32,
    pub damage_per_tick: f32,
    pub remaining_time: f32,
    pub max_time: f32,
    /// If set, damage uses a rectangular AABB (half-width, half-height) instead of radius.
    pub half_size: Option<Vec2>,
}

/// Marker for a sprite entity that visualises a HazardZone on the main map.
#[derive(Component)]
pub struct HazardVisual {
    /// Index into `EnvironmentState::active_hazards` at spawn time.
    /// Re-synced each frame by matching position.
    pub hazard_id: u64,
}

/// Unique counter so each hazard zone gets a stable visual id.
static NEXT_HAZARD_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Monotonic id attached to each HazardZone for visual pairing.
impl HazardZone {
    fn next_id() -> u64 {
        NEXT_HAZARD_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// Marker for raindrop sprite entities.
#[derive(Component)]
pub struct RainDrop {
    pub velocity: f32,
}

/// Marker for the full-screen night-time darkness overlay.
#[derive(Component)]
pub struct NightOverlay;

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            time_of_day: 0.5, // Start at noon
            is_raining: false,
            rain_timer: 120.0, // First rain in 2 minutes
            evaporation_multiplier: 1.0,
            flood_level: 0.0,
            active_hazards: Vec::new(),
            active_mowers: Vec::new(),
            active_footstep_paths: Vec::new(),
            rain_spawn_accum: 0.0,
        }
    }
}

/// Update day/night cycle (3-minute cycle).
fn update_day_night_cycle(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let cycle_duration = 180.0; // 3 minutes
    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    env.time_of_day = (env.time_of_day + delta / cycle_duration) % 1.0;
}

/// Rain event management and pheromone impact.
fn update_rain_state(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
    mut toasts: ResMut<ToastQueue>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    if !env.is_raining {
        env.rain_timer -= delta;
        if env.rain_timer <= 0.0 {
            // Start rain
            env.is_raining = true;
            env.evaporation_multiplier = 10.0;
            env.flood_level = 0.0;
            toasts.push("Rain starting!".to_string());
        }
    } else {
        // Rain active for 30-60 seconds
        env.flood_level += delta * 0.5; // Gradually fill
        if env.flood_level > 1.0 {
            env.flood_level = 1.0;
        }

        if env.rain_timer > 0.0 {
            env.rain_timer -= delta;
        } else {
            // Stop rain after 60 seconds
            env.is_raining = false;
            env.evaporation_multiplier = 1.0;
            env.rain_timer = rand::thread_rng().gen_range(120.0..300.0); // 2-5 min until next rain
        }
    }

    // Drain flood level gradually
    if !env.is_raining && env.flood_level > 0.0 {
        env.flood_level = (env.flood_level - delta * 0.1).max(0.0);
    }
}

/// Handle manually triggered hazard events from UI.
fn handle_manual_hazards(
    mut env: ResMut<EnvironmentState>,
    mut events: MessageReader<HazardEvent>,
    mut toasts: ResMut<ToastQueue>,
) {
    for event in events.read() {
        match event {
            HazardEvent::TriggerRain => {
                env.is_raining = true;
                env.evaporation_multiplier = 10.0;
                env.flood_level = 0.0;
                env.rain_timer = 60.0;
                toasts.push("Rain triggered!".to_string());
            }
            HazardEvent::TriggerFootstep => {
                spawn_footstep_path(&mut env, &mut rand::thread_rng());
                toasts.push("Human spotted!".to_string());
            }
            HazardEvent::TriggerLawnmower => {
                spawn_mower(&mut env, &mut rand::thread_rng());
                toasts.push("Lawnmower!".to_string());
            }
            HazardEvent::TriggerPesticide => {
                let pos = Vec2::new(
                    rand::thread_rng().gen_range(100.0..1180.0),
                    rand::thread_rng().gen_range(100.0..620.0),
                );
                let id = HazardZone::next_id();
                env.active_hazards.push((id, HazardZone {
                    kind: HazardKind::Pesticide,
                    position: pos,
                    radius: 50.0,
                    damage_per_tick: 5.0,
                    remaining_time: 30.0,
                    max_time: 30.0,
                    half_size: None,
                }));
                toasts.push("Pesticide spray!".to_string());
            }
        }
    }
}

/// Manage hazard events: footsteps, lawnmower, pesticide.
fn update_hazard_events(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
    mut toasts: ResMut<ToastQueue>,
    mut query: Query<(&Transform, &mut Health), With<Ant>>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    // Update existing hazard zones
    env.active_hazards.retain_mut(|(_id, hazard)| {
        hazard.remaining_time -= delta;
        hazard.remaining_time > 0.0
    });

    // Apply damage from active hazards
    for (transform, mut health) in query.iter_mut() {
        let pos = transform.translation.truncate();
        for (_id, hazard) in &env.active_hazards {
            let hit = if let Some(hs) = hazard.half_size {
                // Rectangular AABB check
                let d = (pos - hazard.position).abs();
                d.x < hs.x && d.y < hs.y
            } else {
                pos.distance(hazard.position) < hazard.radius
            };
            if hit {
                health.apply_damage(hazard.damage_per_tick * delta, DamageSource::from(hazard.kind));
            }
        }
    }

    // Random hazard events (roughly every 30-60 seconds)
    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() < 0.001 {
        let event_type = rng.gen_range(0..3);
        match event_type {
            0 => {
                spawn_footstep_path(&mut env, &mut rng);
                toasts.push("Watch out — human!".to_string());
            }
            1 => {
                spawn_mower(&mut env, &mut rng);
                toasts.push("Lawnmower approaching!".to_string());
            }
            2 => {
                // Pesticide zone
                let pos = Vec2::new(
                    rng.gen_range(100.0..1180.0),
                    rng.gen_range(100.0..620.0),
                );
                let id = HazardZone::next_id();
                env.active_hazards.push((id, HazardZone {
                    kind: HazardKind::Pesticide,
                    position: pos,
                    radius: 50.0,
                    damage_per_tick: 5.0, // Damage over time
                    remaining_time: 30.0,
                    max_time: 30.0,
                    half_size: None,
                }));
                toasts.push("Pesticide spray detected!".to_string());
            }
            _ => {}
        }
    }
}

// ── Animated Hazard Helpers ────────────────────────────────────────

/// Spawn a lawnmower that sweeps horizontally across the yard.
fn spawn_mower(env: &mut EnvironmentState, rng: &mut impl Rng) {
    let y = rng.gen_range(200.0..1800.0);
    let left_to_right = rng.gen_bool(0.5);
    let (start_x, end_x, dir) = if left_to_right {
        (0.0_f32, 2048.0_f32, 1.0_f32)
    } else {
        (2048.0_f32, 0.0_f32, -1.0_f32)
    };

    let id = HazardZone::next_id();
    let radius = 60.0;
    let speed = rng.gen_range(200.0..350.0);

    // Seed the initial hazard zone at the start position
    env.active_hazards.push((id, HazardZone {
        kind: HazardKind::Lawnmower,
        position: Vec2::new(start_x, y),
        radius,
        damage_per_tick: 999.0,
        remaining_time: 999.0, // managed by ActiveMower, not the timer
        max_time: 999.0,
        half_size: None,
    }));

    env.active_mowers.push(ActiveMower {
        y,
        current_x: start_x,
        end_x,
        speed,
        direction: dir,
        radius,
        hazard_id: id,
    });
}

/// Spawn a human walking across the yard (a sequence of footsteps).
fn spawn_footstep_path(env: &mut EnvironmentState, rng: &mut impl Rng) {
    // Pick a random start along one edge and a direction roughly across the yard
    let angle = rng.gen_range(-0.4..0.4); // slight variation from straight across
    let start_edge = rng.gen_range(0..4); // 0=left, 1=right, 2=bottom, 3=top

    let (start, base_angle) = match start_edge {
        0 => (Vec2::new(0.0, rng.gen_range(200.0..1800.0)), 0.0_f32),           // left → right
        1 => (Vec2::new(2048.0, rng.gen_range(200.0..1800.0)), std::f32::consts::PI), // right → left
        2 => (Vec2::new(rng.gen_range(200.0..1800.0), 0.0), std::f32::consts::FRAC_PI_2),   // bottom → top
        _ => (Vec2::new(rng.gen_range(200.0..1800.0), 2048.0), -std::f32::consts::FRAC_PI_2), // top → bottom
    };

    let final_angle = base_angle + angle;
    let direction = Vec2::new(final_angle.cos(), final_angle.sin());
    let perpendicular = Vec2::new(-direction.y, direction.x);

    env.active_footstep_paths.push(ActiveFootstepPath {
        direction,
        perpendicular,
        next_pos: start,
        stride_length: rng.gen_range(80.0..120.0),
        lateral_offset: rng.gen_range(15.0..22.0),
        is_left: true,
        step_timer: 0.0, // first step immediately
        step_interval: rng.gen_range(0.6..0.9),
        steps_remaining: rng.gen_range(35..50),
        current_hazard_id: None,
    });
}

/// Advance animated hazards (mowers and footstep paths) each frame.
fn update_animated_hazards(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    // ── Lawnmowers ──────────────────────────────────────────────
    let mut finished_mower_ids: Vec<u64> = Vec::new();
    let mut mower_updates: Vec<(u64, Vec2)> = Vec::new(); // (hazard_id, new_position)

    for mower in &mut env.active_mowers {
        mower.current_x += mower.direction * mower.speed * delta;

        // Check if mower has crossed its end point
        let done = if mower.direction > 0.0 {
            mower.current_x >= mower.end_x
        } else {
            mower.current_x <= mower.end_x
        };

        if done {
            finished_mower_ids.push(mower.hazard_id);
        } else {
            mower_updates.push((mower.hazard_id, Vec2::new(mower.current_x, mower.y)));
        }
    }

    // Apply mower position updates to their hazard zones
    for (hid, new_pos) in &mower_updates {
        if let Some((_id, zone)) = env.active_hazards.iter_mut().find(|(id, _)| id == hid) {
            zone.position = *new_pos;
        }
    }

    // Remove finished mowers and their hazard zones
    for id in &finished_mower_ids {
        env.active_mowers.retain(|m| m.hazard_id != *id);
        env.active_hazards.retain(|(hid, _)| hid != id);
    }

    // ── Footstep Paths ──────────────────────────────────────────
    let mut new_hazards: Vec<(u64, HazardZone)> = Vec::new();
    let mut remove_hazard_ids: Vec<u64> = Vec::new();
    let mut finished_paths: Vec<usize> = Vec::new();

    for (i, path) in env.active_footstep_paths.iter_mut().enumerate() {
        path.step_timer -= delta;
        if path.step_timer > 0.0 {
            continue;
        }

        if path.steps_remaining == 0 {
            // Remove the last visible footstep and mark path as done
            if let Some(old_id) = path.current_hazard_id.take() {
                remove_hazard_ids.push(old_id);
            }
            finished_paths.push(i);
            continue;
        }

        // Remove previous footstep zone
        if let Some(old_id) = path.current_hazard_id.take() {
            remove_hazard_ids.push(old_id);
        }

        // Place new footstep
        let lateral = if path.is_left { -path.lateral_offset } else { path.lateral_offset };
        let foot_pos = path.next_pos + path.perpendicular * lateral;

        let new_id = HazardZone::next_id();
        new_hazards.push((new_id, HazardZone {
            kind: HazardKind::Footstep,
            position: foot_pos,
            radius: 60.0,
            damage_per_tick: 999.0,
            remaining_time: path.step_interval + 0.1, // slightly longer than interval so it overlaps
            max_time: path.step_interval + 0.1,
            half_size: Some(Vec2::new(25.0, 55.0)), // foot-shaped rectangle (~50 wide x 110 tall)
        }));

        path.current_hazard_id = Some(new_id);
        path.next_pos += path.direction * path.stride_length;
        path.is_left = !path.is_left;
        path.steps_remaining -= 1;
        path.step_timer = path.step_interval;
    }

    // Apply deferred changes
    for id in &remove_hazard_ids {
        env.active_hazards.retain(|(hid, _)| hid != id);
    }
    env.active_hazards.extend(new_hazards);

    // Remove finished paths (iterate in reverse to keep indices valid)
    finished_paths.sort_unstable();
    for i in finished_paths.into_iter().rev() {
        env.active_footstep_paths.remove(i);
    }
}

// ── Visual Systems ─────────────────────────────────────────────────

/// Spawn a single full-screen overlay sprite used for night-time darkening.
fn spawn_night_overlay(mut commands: Commands, config: Res<SimConfig>) {
    let size = Vec2::new(config.world_width * 3.0, config.world_height * 3.0);
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.05, 0.0),
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(config.world_width / 2.0, config.world_height / 2.0, 6.0),
        NightOverlay,
    ));
}

/// Update the night overlay alpha based on time_of_day.
fn update_night_overlay(
    env: Res<EnvironmentState>,
    mut query: Query<&mut Sprite, With<NightOverlay>>,
) {
    // 0.5 = noon (bright), 0.0/1.0 = midnight (dark)
    // Map distance from noon to darkness factor
    let dist_from_noon = (env.time_of_day - 0.5).abs() * 2.0; // 0..1
    let darkness = (dist_from_noon * 0.65).clamp(0.0, 0.65);

    for mut sprite in &mut query {
        sprite.color = Color::srgba(0.0, 0.0, 0.05, darkness);
    }
}

/// Sync hazard zone visual sprite entities with the active_hazards list.
/// Spawns new sprites for new hazards, updates existing ones, despawns stale ones.
fn sync_hazard_visuals(
    mut commands: Commands,
    env: Res<EnvironmentState>,
    registry: Res<MapRegistry>,
    time: Res<Time>,
    mut visuals: Query<(Entity, &HazardVisual, &mut Sprite, &mut Transform)>,
) {
    // Collect active hazard ids
    let active_ids: Vec<u64> = env.active_hazards.iter().map(|(id, _)| *id).collect();

    // Despawn visuals whose hazard no longer exists
    for (entity, hv, _, _) in visuals.iter() {
        if !active_ids.contains(&hv.hazard_id) {
            commands.entity(entity).despawn();
        }
    }

    // Collect existing visual ids
    let existing_ids: Vec<u64> = visuals.iter().map(|(_, hv, _, _)| hv.hazard_id).collect();

    // Spawn visuals for new hazards
    for (id, hazard) in &env.active_hazards {
        if existing_ids.contains(id) {
            continue;
        }

        let (color, size) = match hazard.kind {
            HazardKind::Footstep => {
                let hs = hazard.half_size.unwrap_or(Vec2::new(25.0, 55.0));
                (
                    Color::srgba(0.2, 0.15, 0.1, 0.55),
                    hs * 2.0, // full width x full height
                )
            }
            HazardKind::Lawnmower => (
                Color::srgba(0.9, 0.2, 0.1, 0.35),
                Vec2::new(hazard.radius * 4.0, hazard.radius * 2.0),
            ),
            HazardKind::Pesticide => (
                Color::srgba(0.5, 0.8, 0.1, 0.25),
                Vec2::splat(hazard.radius * 2.0),
            ),
        };

        commands.spawn((
            Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_xyz(hazard.position.x, hazard.position.y, 7.0),
            HazardVisual { hazard_id: *id },
            MapId(registry.surface),
        ));
    }

    // Update existing visuals (fade alpha as remaining_time decreases, pulse pesticide)
    let t = time.elapsed_secs();
    for (_entity, hv, mut sprite, mut transform) in visuals.iter_mut() {
        if let Some((_id, hazard)) = env.active_hazards.iter().find(|(id, _)| *id == hv.hazard_id)
        {
            // Update position (in case we ever animate lawnmower sweep)
            transform.translation.x = hazard.position.x;
            transform.translation.y = hazard.position.y;

            let life_frac = (hazard.remaining_time / hazard.max_time).clamp(0.0, 1.0);

            match hazard.kind {
                HazardKind::Footstep => {
                    // Fade out as it expires
                    let base_alpha = 0.55 * life_frac;
                    sprite.color = Color::srgba(0.2, 0.15, 0.1, base_alpha);
                }
                HazardKind::Lawnmower => {
                    // Slight pulsing danger zone
                    let pulse = 0.3 + 0.1 * (t * 8.0).sin();
                    sprite.color = Color::srgba(0.9, 0.2, 0.1, pulse);
                }
                HazardKind::Pesticide => {
                    // Slow sinusoidal alpha pulse
                    let pulse = 0.2 + 0.12 * (t * 2.0).sin();
                    sprite.color = Color::srgba(0.5, 0.8, 0.1, pulse);
                }
            }
        }
    }
}

const MAX_RAINDROPS: usize = 100;
const RAIN_SPAWN_RATE: f32 = 60.0; // drops per second

/// Spawn, animate, and despawn raindrop sprite entities when raining.
fn update_rain_visuals(
    mut commands: Commands,
    mut env: ResMut<EnvironmentState>,
    time: Res<Time>,
    sim_clock: Res<SimClock>,
    registry: Res<MapRegistry>,
    camera_query: Query<(&Transform, &Projection), With<MainCamera>>,
    mut drops: Query<(Entity, &mut RainDrop, &mut Transform, &mut Sprite), Without<MainCamera>>,
) {
    let delta = time.delta_secs();

    // If not raining, despawn all existing drops and reset accumulator
    if !env.is_raining {
        for (entity, _, _, _) in drops.iter() {
            commands.entity(entity).despawn();
        }
        env.rain_spawn_accum = 0.0;
        return;
    }

    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();

    // Get camera viewport bounds for spawning rain in visible area
    let (cam_pos, cam_proj) = match camera_query.single() {
        Ok(v) => v,
        Err(_) => return,
    };
    let scale = match cam_proj {
        Projection::Orthographic(ref ortho) => ortho.scale,
        _ => 1.0,
    };
    let half_w = 640.0 * scale;
    let half_h = 360.0 * scale;
    let cam_x = cam_pos.translation.x;
    let cam_y = cam_pos.translation.y;

    // Accumulate fractional drops across frames
    env.rain_spawn_accum += RAIN_SPAWN_RATE * delta;
    let current_count = drops.iter().count();
    let whole_drops = env.rain_spawn_accum as usize;
    let to_spawn = whole_drops.min(MAX_RAINDROPS.saturating_sub(current_count));
    env.rain_spawn_accum -= to_spawn as f32;
    // Clamp accumulator to avoid runaway after long pauses/unfocus
    env.rain_spawn_accum = env.rain_spawn_accum.min(RAIN_SPAWN_RATE);

    for _ in 0..to_spawn {
        let x = rng.gen_range((cam_x - half_w)..(cam_x + half_w));
        let y = cam_y + half_h + rng.gen_range(0.0..50.0);
        let velocity = rng.gen_range(300.0..500.0);

        // Small blue diamond-ish raindrop
        commands.spawn((
            Sprite {
                color: Color::srgba(0.4, 0.6, 1.0, 0.35),
                custom_size: Some(Vec2::new(2.0, 6.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 8.0),
            RainDrop { velocity },
            MapId(registry.surface),
        ));
    }

    // Animate existing drops downward, despawn if below viewport
    let bottom = cam_y - half_h - 20.0;
    for (entity, drop, mut transform, _sprite) in drops.iter_mut() {
        transform.translation.y -= drop.velocity * delta * sim_clock.speed.multiplier();
        if transform.translation.y < bottom {
            commands.entity(entity).despawn();
        }
    }
}
