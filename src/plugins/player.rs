use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{
    Ant, AntState, CarriedItem, ColonyMember, Movement, PlayerControlled, PortalCooldown,
};
use crate::components::map::{MapId, MapKind, MapMarker, MapPortal, PORTAL_RANGE};
use crate::components::nest::{NestPath, NestTask};
use crate::components::pheromone::PheromoneType;
use crate::components::terrain::FoodSource;
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::camera::MainCamera;
use crate::plugins::nest_navigation::world_to_nest_grid;
use crate::resources::active_map::{ActiveMap, MapRegistry, SavedCamera, SavedCameraStates};
use crate::resources::nest::NestGrid;
use crate::resources::nest_pheromone::{NestPheromoneConfig, NestPheromoneGrid};
use crate::resources::pheromone::{ColonyPheromones, PheromoneConfig};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

/// Radius (in grid cells) around the player where Recruit pheromone is deposited
const RECRUIT_DEPOSIT_RADIUS: i32 = 3;

/// Nest ant movement speed (matches nest_navigation::NEST_ANT_SPEED).
const NEST_PLAYER_SPEED: f32 = 60.0;

/// Default nest-view camera scale (matches nest.rs).
const NEST_CAMERA_SCALE: f32 = 0.7;

// ── Player action events ───────────────────────────────────────────

/// Discrete player actions emitted by keyboard or UI buttons.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    Pickup,
    Drop,
    Recruit,
    Dismiss,
    Swap,
    Attack,
    Feed,
}

/// Per-frame snapshot of which player actions are currently available.
/// Computed by `update_action_context` and read by the egui action bar.
#[derive(Resource, Default)]
pub struct ActionContext {
    pub can_pickup: bool,
    pub can_drop: bool,
    pub can_recruit: bool,
    pub can_dismiss: bool,
    pub can_swap: bool,
    pub can_attack: bool,
    pub can_feed: bool,
    pub trail_active: bool,
}

/// Queue of notification toasts to display in the UI.
#[derive(Resource, Default)]
pub struct ToastQueue {
    pub toasts: Vec<Toast>,
}

pub struct Toast {
    pub message: String,
    pub timer: f32,
}

impl ToastQueue {
    pub fn push(&mut self, message: impl Into<String>) {
        self.toasts.push(Toast {
            message: message.into(),
            timer: 3.0,
        });
        // Keep at most 5 queued.
        while self.toasts.len() > 5 {
            self.toasts.remove(0);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for toast in &mut self.toasts {
            toast.timer -= dt;
        }
        self.toasts.retain(|t| t.timer > 0.0);
    }
}

/// Which pheromone the R key deposits: follow (Recruit) or attack (AttackRecruit).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecruitMode {
    Follow,
    Attack,
}

impl Default for RecruitMode {
    fn default() -> Self {
        Self::Follow
    }
}

impl RecruitMode {
    pub fn pheromone_type(self) -> PheromoneType {
        match self {
            RecruitMode::Follow => PheromoneType::Recruit,
            RecruitMode::Attack => PheromoneType::AttackRecruit,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RecruitMode::Follow => "Follow",
            RecruitMode::Attack => "Attack",
        }
    }
}

pub struct PlayerPlugin;

#[derive(Resource)]
pub struct PlayerMode {
    pub controlling: bool,
    pub follow_camera: bool,
}

impl Default for PlayerMode {
    fn default() -> Self {
        Self {
            controlling: true,
            follow_camera: true,
        }
    }
}

#[derive(Resource, Default)]
pub struct FollowerCount(pub usize);

const PLAYER_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const PLAYER_CARRY_COLOR: Color = Color::srgb(1.0, 0.6, 0.0);
const PICKUP_RANGE: f32 = 25.0;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {

        app.init_resource::<PlayerMode>()
            .init_resource::<FollowerCount>()
            .init_resource::<RecruitMode>()
            .init_resource::<ActionContext>()
            .init_resource::<ToastQueue>()
            .add_message::<PlayerAction>()
            .add_systems(
                Update,
                (
                    designate_player_ant,
                    update_follower_count,
                    update_action_context,
                    tick_toasts,
                    keyboard_to_player_actions,
                ),
            )
            // Systems that work on any map (no viewing_surface gate).
            .add_systems(
                Update,
                (
                    toggle_player_mode,
                    toggle_recruit_mode,
                    player_portal_transition,
                    player_movement,
                    player_nest_collision,
                    player_pickup,
                    player_drop,
                    player_regurgitate,
                    player_pheromone,
                    player_recruit_pheromone,
                    player_dismiss_pheromone,
                    exchange_ant,
                    camera_follow_player,
                    update_player_visual,
                )
                    .chain(),
            );
    }
}

fn designate_player_ant(
    mut commands: Commands,
    registry: Res<MapRegistry>,
    query: Query<(Entity, &ColonyMember, &MapId), With<Ant>>,
    existing: Query<Entity, With<PlayerControlled>>,
) {
    if !existing.is_empty() {
        return;
    }
    for (entity, colony, map_id) in &query {
        if colony.colony_id == 0 && map_id.0 == registry.surface {
            commands.entity(entity).insert(PlayerControlled);
            break;
        }
    }
}

fn toggle_player_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlayerMode>,
) {
    if input.just_pressed(KeyCode::KeyG) {
        if mode.controlling {
            mode.controlling = false;
            mode.follow_camera = false;
        } else {
            mode.controlling = true;
            mode.follow_camera = true;
        }
    }
}

fn toggle_recruit_mode(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut recruit_mode: ResMut<RecruitMode>,
) {
    if !mode.controlling {
        return;
    }
    if input.just_pressed(KeyCode::KeyV) {
        *recruit_mode = match *recruit_mode {
            RecruitMode::Follow => RecruitMode::Attack,
            RecruitMode::Attack => RecruitMode::Follow,
        };
    }
}

// ── Portal transition ───────────────────────────────────────────────

/// Press Enter near a portal to transition the player (and followers) between maps.
fn player_portal_transition(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    registry: Res<MapRegistry>,
    config: Res<SimConfig>,
    mut active: ResMut<ActiveMap>,
    mut saved: ResMut<SavedCameraStates>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    portal_query: Query<&MapPortal>,
    map_kind_query: Query<&MapKind, With<MapMarker>>,
    mut player_query: Query<
        (Entity, &mut Transform, &mut MapId, &mut Visibility, &ColonyMember),
        (With<PlayerControlled>, Without<MainCamera>),
    >,
    mut follower_query: Query<
        (Entity, &mut Transform, &mut Ant, &mut MapId, &mut Visibility),
        (Without<PlayerControlled>, Without<MainCamera>),
    >,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::Enter) {
        return;
    }

    let Ok((player_entity, mut player_tf, mut player_map, mut player_vis, colony)) =
        player_query.single_mut()
    else {
        return;
    };

    let player_pos = player_tf.translation.truncate();

    // Find a portal on the player's current map within range.
    let mut found_portal: Option<&MapPortal> = None;
    for portal in &portal_query {
        if portal.map != player_map.0 {
            continue;
        }
        // Player can enter any portal regardless of colony restriction.
        if player_pos.distance(portal.position) <= PORTAL_RANGE {
            found_portal = Some(portal);
            break;
        }
    }

    let Some(portal) = found_portal else { return };

    let target_map = portal.target_map;
    let target_pos = portal.target_position;
    let portal_pos = portal.position;
    let entering_nest = target_map != registry.surface;

    // Transition the player.
    player_map.0 = target_map;
    player_tf.translation.x = target_pos.x;
    player_tf.translation.y = target_pos.y;
    *player_vis = Visibility::Hidden; // sync_map_visibility will correct

    // If entering nest, remove any NestTask (player is player-controlled, not AI).
    // If leaving nest, also clean up.
    commands.entity(player_entity).remove::<NestTask>();
    commands.entity(player_entity).remove::<NestPath>();

    // Transition followers near the portal.
    let follower_range = PORTAL_RANGE * 3.0;
    let mut rng = rand::thread_rng();
    for (entity, mut tf, mut ant, mut map_id, mut vis) in &mut follower_query {
        if ant.state != AntState::Following || map_id.0 != active.entity {
            continue;
        }
        let dist = tf.translation.truncate().distance(portal_pos);
        if dist > follower_range {
            continue;
        }

        map_id.0 = target_map;
        let jitter_x = rng.gen_range(-12.0..12.0f32);
        let jitter_y = rng.gen_range(-12.0..12.0f32);
        tf.translation.x = target_pos.x + jitter_x;
        tf.translation.y = target_pos.y + jitter_y;
        *vis = Visibility::Hidden;

        if entering_nest {
            commands.entity(entity).insert((
                NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 },
                PortalCooldown::new(),
            ));
            ant.state = AntState::Following;
        } else {
            commands.entity(entity).remove::<NestTask>();
            commands.entity(entity).remove::<NestPath>();
            commands.entity(entity).insert(PortalCooldown::new());
            ant.state = AntState::Following;
        }
    }

    // Switch the active map view (mirrors cycle_map_view logic).
    let Ok((mut cam_tf, mut proj)) = camera_query.single_mut() else { return };

    // Save camera state for the map we're leaving.
    let current_scale = match *proj {
        Projection::Orthographic(ref ortho) => ortho.scale,
        _ => 1.0,
    };
    saved.0.insert(active.entity, SavedCamera {
        position: cam_tf.translation.truncate(),
        scale: current_scale,
    });

    let target_kind = map_kind_query.get(target_map).copied().unwrap_or(MapKind::Surface);

    // Restore or set default camera for the target map.
    if let Some(cam) = saved.0.get(&target_map) {
        cam_tf.translation.x = cam.position.x;
        cam_tf.translation.y = cam.position.y;
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = cam.scale;
        }
    } else if entering_nest {
        cam_tf.translation.x = target_pos.x;
        cam_tf.translation.y = target_pos.y;
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = NEST_CAMERA_SCALE;
        }
    } else {
        cam_tf.translation.x = config.world_width / 2.0;
        cam_tf.translation.y = config.world_height / 2.0;
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = 1.0;
        }
    }

    active.entity = target_map;
    active.kind = target_kind;
}

// ── Movement ────────────────────────────────────────────────────────

fn player_movement(
    clock: Res<SimClock>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    nest_query: Query<&NestGrid, With<MapMarker>>,
    mut query: Query<(&mut Transform, &Movement, &mut Ant, &MapId), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let Ok((mut transform, movement, mut ant, map_id)) = query.single_mut() else {
        return;
    };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }

    if dir == Vec2::ZERO {
        return;
    }

    let dir = dir.normalize();
    let is_underground = map_id.0 != registry.surface;

    if is_underground {
        // Underground: use nest speed, check passability before moving.
        let speed = NEST_PLAYER_SPEED * clock.speed.multiplier() * time.delta_secs();
        let new_x = transform.translation.x + dir.x * speed;
        let new_y = transform.translation.y + dir.y * speed;
        let new_pos = Vec2::new(new_x, new_y);

        if let Ok(grid) = nest_query.get(map_id.0) {
            if let Some((gx, gy)) = world_to_nest_grid(new_pos) {
                if grid.get(gx, gy).is_passable() {
                    transform.translation.x = new_x;
                    transform.translation.y = new_y;
                }
            }
            // If out of bounds or not passable, don't move.
        }
    } else {
        // Surface: existing logic.
        let speed = movement.speed * clock.speed.multiplier() * time.delta_secs();
        transform.translation.x += dir.x * speed;
        transform.translation.y += dir.y * speed;
    }

    if ant.state != AntState::Returning {
        ant.state = AntState::Foraging;
    }
}

/// Clamp player position to passable cells when underground.
fn player_nest_collision(
    registry: Res<MapRegistry>,
    nest_query: Query<&NestGrid, With<MapMarker>>,
    mut query: Query<(&mut Transform, &MapId), With<PlayerControlled>>,
) {
    let Ok((mut transform, map_id)) = query.single_mut() else { return };
    if map_id.0 == registry.surface {
        return;
    }

    let Ok(grid) = nest_query.get(map_id.0) else { return };
    let pos = transform.translation.truncate();

    match world_to_nest_grid(pos) {
        Some((gx, gy)) if !grid.get(gx, gy).is_passable() => {
            // Find nearest passable cell.
            for radius in 1i32..10 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx.abs() != radius && dy.abs() != radius {
                            continue;
                        }
                        let nx = gx as i32 + dx;
                        let ny = gy as i32 + dy;
                        if nx >= 0
                            && ny >= 0
                            && (nx as usize) < grid.width
                            && (ny as usize) < grid.height
                            && grid.get(nx as usize, ny as usize).is_passable()
                        {
                            let safe = crate::plugins::nest_navigation::nest_grid_to_world(
                                nx as usize, ny as usize,
                            );
                            transform.translation.x = safe.x;
                            transform.translation.y = safe.y;
                            return;
                        }
                    }
                }
            }
        }
        None => {
            // Outside grid entirely — teleport to entrance.
            let cx = grid.width / 2;
            for y in 0..grid.height {
                if grid.get(cx, y).is_passable() {
                    let safe = crate::plugins::nest_navigation::nest_grid_to_world(cx, y);
                    transform.translation.x = safe.x;
                    transform.translation.y = safe.y;
                    return;
                }
            }
        }
        _ => {}
    }
}

// ── Surface-only interactions ───────────────────────────────────────

fn player_pickup(
    mut events: MessageReader<PlayerAction>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &MapId), (With<PlayerControlled>, Without<CarriedItem>)>,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    let triggered = events.read().any(|a| *a == PlayerAction::Pickup);
    if !mode.controlling || !triggered {
        return;
    }

    let Ok((player_entity, player_tf, map_id)) = player_query.single() else {
        return;
    };
    if map_id.0 != registry.surface {
        return;
    }

    let player_pos = player_tf.translation.truncate();

    for (food_tf, mut food) in &mut food_query {
        if food.remaining <= 0.0 {
            continue;
        }
        let dist = player_pos.distance(food_tf.translation.truncate());
        if dist < PICKUP_RANGE {
            let amount = food.remaining.min(5.0);
            food.remaining -= amount;
            commands
                .entity(player_entity)
                .insert(CarriedItem { food_amount: amount });
            break;
        }
    }
}

fn player_drop(
    mut events: MessageReader<PlayerAction>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    config: Res<SimConfig>,
    mut food_query: Query<&mut ColonyFood, With<MapMarker>>,
    query: Query<(Entity, &Transform, &CarriedItem, &MapId), With<PlayerControlled>>,
) {
    let triggered = events.read().any(|a| *a == PlayerAction::Drop);
    if !mode.controlling || !triggered {
        return;
    }

    let Ok((entity, transform, carried, map_id)) = query.single() else {
        return;
    };
    if map_id.0 != registry.surface {
        return;
    }

    let pos = transform.translation.truncate();
    let dist_to_nest = pos.distance(config.nest_position);

    if dist_to_nest < 40.0 {
        if let Ok(mut food) = food_query.get_mut(registry.player_nest) {
            food.stored += carried.food_amount;
        }
    }
    commands.entity(entity).remove::<CarriedItem>();
}

fn player_regurgitate(
    mut events: MessageReader<PlayerAction>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &CarriedItem, &MapId), With<PlayerControlled>>,
    mut ant_query: Query<(&Transform, &mut Ant), Without<PlayerControlled>>,
) {
    let triggered = events.read().any(|a| *a == PlayerAction::Feed);
    if !mode.controlling || !triggered {
        return;
    }

    let Ok((player_entity, player_tf, carried, map_id)) = player_query.single() else {
        return;
    };
    if map_id.0 != registry.surface {
        return;
    }

    let player_pos = player_tf.translation.truncate();

    let mut nearest: Option<(f32, Mut<Ant>)> = None;
    for (tf, ant) in &mut ant_query {
        let dist = player_pos.distance(tf.translation.truncate());
        if dist < PICKUP_RANGE {
            if nearest.is_none() || dist < nearest.as_ref().unwrap().0 {
                nearest = Some((dist, ant));
            }
        }
    }

    if let Some((_dist, mut target_ant)) = nearest {
        let relief = (carried.food_amount * 0.1).min(target_ant.hunger);
        target_ant.hunger = (target_ant.hunger - relief).max(0.0);
        commands.entity(player_entity).remove::<CarriedItem>();
    }
}

// ── Pheromone systems (work on both maps) ───────────────────────────

fn player_pheromone(
    clock: Res<SimClock>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    pconfig: Res<PheromoneConfig>,
    nest_pconfig: Res<NestPheromoneConfig>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    mut nest_phero_query: Query<&mut NestPheromoneGrid, With<MapMarker>>,
    query: Query<(&Transform, &ColonyMember, &MapId), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let shift = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);
    if !shift {
        return;
    }

    let Ok((transform, colony, map_id)) = query.single() else {
        return;
    };

    let pos = transform.translation.truncate();

    if map_id.0 == registry.surface {
        // Surface: deposit Trail into ColonyPheromones.
        let Some(ref mut all_grids) = grids else { return };
        let Some(grid) = all_grids.get_mut(colony.colony_id) else { return };
        if let Some((gx, gy)) = grid.world_to_grid(pos) {
            let amt = pconfig.deposit_amount(PheromoneType::Trail) * 3.0;
            grid.deposit(gx, gy, PheromoneType::Trail, amt, pconfig.max_intensity);
        }
    } else {
        // Underground: deposit trail into NestPheromoneGrid.
        let Ok(mut phero_grid) = nest_phero_query.get_mut(map_id.0) else { return };
        if let Some((gx, gy)) = world_to_nest_grid(pos) {
            if let Some(cell) = phero_grid.get_mut(gx, gy) {
                let amt = pconfig.deposit_amount(PheromoneType::Trail) * 3.0;
                cell.trail = (cell.trail + amt).min(nest_pconfig.trail_recruit_max);
            }
        }
    }
}

/// Hold R to deposit recruit pheromone (follow or attack, based on current mode)
/// in a wide area around the player. Works on both surface and underground.
fn player_recruit_pheromone(
    mut events: MessageReader<PlayerAction>,
    clock: Res<SimClock>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    recruit_mode: Res<RecruitMode>,
    pconfig: Res<PheromoneConfig>,
    nest_pconfig: Res<NestPheromoneConfig>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    mut nest_phero_query: Query<&mut NestPheromoneGrid, With<MapMarker>>,
    query: Query<(&Transform, &ColonyMember, &MapId), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let from_event = events.read().any(|a| *a == PlayerAction::Recruit);
    let from_key = input.pressed(KeyCode::KeyR);
    if !from_event && !from_key {
        return;
    }

    let Ok((transform, colony, map_id)) = query.single() else {
        return;
    };

    let pos = transform.translation.truncate();

    if map_id.0 == registry.surface {
        // Surface deposit.
        let Some(ref mut all_grids) = grids else { return };
        let Some(grid) = all_grids.get_mut(colony.colony_id) else { return };
        let Some((cx, cy)) = grid.world_to_grid(pos) else { return };

        let ptype = recruit_mode.pheromone_type();
        let amt = pconfig.deposit_amount(ptype);
        for dy in -RECRUIT_DEPOSIT_RADIUS..=RECRUIT_DEPOSIT_RADIUS {
            for dx in -RECRUIT_DEPOSIT_RADIUS..=RECRUIT_DEPOSIT_RADIUS {
                let gx = cx as i32 + dx;
                let gy = cy as i32 + dy;
                if gx >= 0 && gy >= 0 && (gx as usize) < grid.width && (gy as usize) < grid.height {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let falloff = 1.0 / (1.0 + dist);
                    grid.deposit(
                        gx as usize,
                        gy as usize,
                        ptype,
                        amt * falloff,
                        pconfig.max_intensity,
                    );
                }
            }
        }
    } else {
        // Underground deposit — always use Recruit channel in nest grid.
        let Ok(mut phero_grid) = nest_phero_query.get_mut(map_id.0) else { return };
        let Some((cx, cy)) = world_to_nest_grid(pos) else { return };

        let amt = pconfig.deposit_amount(PheromoneType::Recruit);
        let max = nest_pconfig.trail_recruit_max;
        for dy in -RECRUIT_DEPOSIT_RADIUS..=RECRUIT_DEPOSIT_RADIUS {
            for dx in -RECRUIT_DEPOSIT_RADIUS..=RECRUIT_DEPOSIT_RADIUS {
                let gx = cx as i32 + dx;
                let gy = cy as i32 + dy;
                if gx >= 0
                    && gy >= 0
                    && (gx as usize) < phero_grid.width
                    && (gy as usize) < phero_grid.height
                {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let falloff = 1.0 / (1.0 + dist);
                    if let Some(cell) = phero_grid.get_mut(gx as usize, gy as usize) {
                        cell.recruit = (cell.recruit + amt * falloff).min(max);
                    }
                }
            }
        }
    }
}

/// Press T to clear recruit pheromone, dismissing followers. Works on both maps.
fn player_dismiss_pheromone(
    mut events: MessageReader<PlayerAction>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    mut nest_phero_query: Query<&mut NestPheromoneGrid, With<MapMarker>>,
    query: Query<(&ColonyMember, &MapId), With<PlayerControlled>>,
) {
    let triggered = events.read().any(|a| *a == PlayerAction::Dismiss);
    if !mode.controlling || !triggered {
        return;
    }

    let Ok((colony, map_id)) = query.single() else {
        return;
    };

    if map_id.0 == registry.surface {
        let Some(ref mut all_grids) = grids else { return };
        let Some(grid) = all_grids.get_mut(colony.colony_id) else { return };
        for y in 0..grid.height {
            for x in 0..grid.width {
                grid.clear_type(x, y, PheromoneType::Recruit);
                grid.clear_type(x, y, PheromoneType::AttackRecruit);
            }
        }
    } else {
        let Ok(mut phero_grid) = nest_phero_query.get_mut(map_id.0) else { return };
        for cell in &mut phero_grid.cells {
            cell.recruit = 0.0;
        }
    }
}

// ── Exchange / swap ant ─────────────────────────────────────────────

fn exchange_ant(
    mut events: MessageReader<PlayerAction>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &MapId), With<PlayerControlled>>,
    candidate_query: Query<(Entity, &Transform, &MapId), (With<Ant>, Without<PlayerControlled>)>,
) {
    let triggered = events.read().any(|a| *a == PlayerAction::Swap);
    if !mode.controlling || !triggered {
        return;
    }

    let Ok((player_entity, player_tf, player_map)) = player_query.single() else {
        return;
    };
    // Only swap to surface ants when on surface.
    if player_map.0 != registry.surface {
        return;
    }

    let player_pos = player_tf.translation.truncate();
    let mut nearest: Option<(Entity, f32)> = None;

    for (entity, tf, map_id) in &candidate_query {
        if map_id.0 != registry.surface {
            continue;
        }
        let dist = player_pos.distance(tf.translation.truncate());
        if nearest.is_none() || dist < nearest.unwrap().1 {
            nearest = Some((entity, dist));
        }
    }

    if let Some((new_entity, _)) = nearest {
        commands.entity(player_entity).remove::<PlayerControlled>();
        commands.entity(new_entity).insert(PlayerControlled);
    }
}

// ── Common systems ──────────────────────────────────────────────────

fn update_follower_count(
    mut count: ResMut<FollowerCount>,
    query: Query<&Ant>,
) {
    count.0 = query.iter().filter(|a| a.state == AntState::Following).count();
}

fn camera_follow_player(
    mode: Res<PlayerMode>,
    active: Res<crate::resources::active_map::ActiveMap>,
    player_query: Query<(&Transform, &crate::components::map::MapId), (With<PlayerControlled>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    if !mode.follow_camera {
        return;
    }

    let Ok((player_tf, player_map)) = player_query.single() else {
        return;
    };

    // Only follow the player when viewing the map they're on.
    if player_map.0 != active.entity {
        return;
    }

    let Ok(mut cam_tf) = camera_query.single_mut() else {
        return;
    };

    let target = player_tf.translation.truncate();
    let current = cam_tf.translation.truncate();
    let smoothed = current.lerp(target, 0.08);
    cam_tf.translation.x = smoothed.x;
    cam_tf.translation.y = smoothed.y;
}

fn update_player_visual(
    mut query: Query<(&mut Sprite, Option<&CarriedItem>), With<PlayerControlled>>,
) {
    let Ok((mut sprite, carried)) = query.single_mut() else {
        return;
    };

    sprite.color = if carried.is_some() {
        PLAYER_CARRY_COLOR
    } else {
        PLAYER_COLOR
    };
    sprite.custom_size = Some(Vec2::splat(6.0));
}

// ── Keyboard→event bridge ──────────────────────────────────────────

fn keyboard_to_player_actions(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut writer: MessageWriter<PlayerAction>,
) {
    if !mode.controlling {
        return;
    }
    if input.just_pressed(KeyCode::KeyE) {
        writer.write(PlayerAction::Pickup);
    }
    if input.just_pressed(KeyCode::KeyQ) {
        writer.write(PlayerAction::Drop);
    }
    if input.just_pressed(KeyCode::KeyF) {
        writer.write(PlayerAction::Feed);
    }
    if input.just_pressed(KeyCode::KeyT) {
        writer.write(PlayerAction::Dismiss);
    }
    if input.just_pressed(KeyCode::KeyX) {
        writer.write(PlayerAction::Swap);
    }
    if input.just_pressed(KeyCode::Space) {
        writer.write(PlayerAction::Attack);
    }
}

// ── Action context (enables/disables UI buttons) ───────────────────

fn update_action_context(
    mut ctx: ResMut<ActionContext>,
    mode: Res<PlayerMode>,
    input: Res<ButtonInput<KeyCode>>,
    registry: Res<MapRegistry>,
    followers: Res<FollowerCount>,
    player_query: Query<(&Transform, &MapId, Option<&CarriedItem>), With<PlayerControlled>>,
    food_query: Query<(&Transform, &FoodSource)>,
    ant_query: Query<(&Transform, &ColonyMember, &MapId), (With<Ant>, Without<PlayerControlled>)>,
) {
    *ctx = ActionContext::default();
    if !mode.controlling {
        return;
    }

    let Ok((player_tf, map_id, carried)) = player_query.single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let on_surface = map_id.0 == registry.surface;

    // Pickup: food nearby and not carrying
    if on_surface && carried.is_none() {
        ctx.can_pickup = food_query.iter().any(|(tf, food)| {
            food.remaining > 0.0 && player_pos.distance(tf.translation.truncate()) < PICKUP_RANGE
        });
    }

    // Drop: carrying something
    ctx.can_drop = carried.is_some();

    // Feed: carrying and ant nearby
    if carried.is_some() {
        ctx.can_feed = ant_query.iter().any(|(tf, _, mid)| {
            mid.0 == map_id.0 && player_pos.distance(tf.translation.truncate()) < PICKUP_RANGE
        });
    }

    // Recruit: always available when controlling
    ctx.can_recruit = true;

    // Dismiss: has followers
    ctx.can_dismiss = followers.0 > 0;

    // Swap: other ants on same map
    ctx.can_swap = on_surface && ant_query.iter().any(|(_, _, mid)| mid.0 == registry.surface);

    // Attack: enemies nearby
    if on_surface {
        ctx.can_attack = ant_query.iter().any(|(tf, col, mid)| {
            mid.0 == registry.surface
                && col.colony_id != 0
                && player_pos.distance(tf.translation.truncate()) < 20.0
        });
    }

    // Trail active
    ctx.trail_active = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);
}

fn tick_toasts(
    time: Res<Time>,
    mut toasts: ResMut<ToastQueue>,
) {
    toasts.tick(time.delta_secs());
}
