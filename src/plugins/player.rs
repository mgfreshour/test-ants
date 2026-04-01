use bevy::prelude::*;

use crate::components::ant::{
    Ant, AntState, CarriedItem, ColonyMember, Movement, PlayerControlled,
};
use crate::components::map::{MapId, MapMarker};
use crate::components::pheromone::PheromoneType;
use crate::components::terrain::FoodSource;
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::camera::MainCamera;
use crate::resources::active_map::{MapRegistry, viewing_surface};
use crate::resources::pheromone::{ColonyPheromones, PheromoneConfig};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

/// Radius (in grid cells) around the player where Recruit pheromone is deposited
const RECRUIT_DEPOSIT_RADIUS: i32 = 3;

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
            .add_systems(
                Update,
                (
                    designate_player_ant,
                    update_follower_count,
                ),
            )
            .add_systems(
                Update,
                (
                    toggle_player_mode,
                    toggle_recruit_mode,
                    player_movement,
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
                    .chain()
                    .run_if(viewing_surface),
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

fn player_movement(
    clock: Res<SimClock>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut query: Query<(&mut Transform, &Movement, &mut Ant), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let Ok((mut transform, movement, mut ant)) = query.get_single_mut() else {
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

    if dir != Vec2::ZERO {
        let dir = dir.normalize();
        let speed = movement.speed * clock.speed.multiplier() * time.delta_secs();
        transform.translation.x += dir.x * speed;
        transform.translation.y += dir.y * speed;

        if ant.state != AntState::Returning {
            ant.state = AntState::Foraging;
        }
    }
}

fn player_pickup(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &Ant), (With<PlayerControlled>, Without<CarriedItem>)>,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Ok((player_entity, player_tf, _ant)) = player_query.get_single() else {
        return;
    };

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
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    config: Res<SimConfig>,
    registry: Res<MapRegistry>,
    mut food_query: Query<&mut ColonyFood, With<MapMarker>>,
    query: Query<(Entity, &Transform, &CarriedItem), With<PlayerControlled>>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyQ) {
        return;
    }

    let Ok((entity, transform, carried)) = query.get_single() else {
        return;
    };

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
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &CarriedItem), With<PlayerControlled>>,
    mut ant_query: Query<(&Transform, &mut Ant), Without<PlayerControlled>>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Ok((player_entity, player_tf, carried)) = player_query.get_single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();

    // Find nearest friendly ant within range
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

fn player_pheromone(
    clock: Res<SimClock>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    pconfig: Res<PheromoneConfig>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    query: Query<(&Transform, &ColonyMember), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let shift = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);
    if !shift {
        return;
    }

    let Ok((transform, colony)) = query.get_single() else {
        return;
    };

    let Some(ref mut all_grids) = grids else { return };
    let Some(grid) = all_grids.get_mut(colony.colony_id) else {
        return;
    };
    let pos = transform.translation.truncate();
    if let Some((gx, gy)) = grid.world_to_grid(pos) {
        let amt = pconfig.deposit_amount(PheromoneType::Trail) * 3.0;
        grid.deposit(gx, gy, PheromoneType::Trail, amt, pconfig.max_intensity);
    }
}

/// Hold R to deposit recruit pheromone (follow or attack, based on current mode)
/// in a wide area around the player. Nearby ants will sense the gradient and
/// follow it toward the player.
fn player_recruit_pheromone(
    clock: Res<SimClock>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    recruit_mode: Res<RecruitMode>,
    pconfig: Res<PheromoneConfig>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    query: Query<(&Transform, &ColonyMember), With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    if !input.pressed(KeyCode::KeyR) {
        return;
    }

    let Ok((transform, colony)) = query.get_single() else {
        return;
    };

    let Some(ref mut all_grids) = grids else { return };
    let Some(grid) = all_grids.get_mut(colony.colony_id) else {
        return;
    };
    let pos = transform.translation.truncate();
    let Some((cx, cy)) = grid.world_to_grid(pos) else {
        return;
    };

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
}

/// Press T to clear both Recruit and AttackRecruit pheromone for this colony,
/// dismissing all followers regardless of mode.
fn player_dismiss_pheromone(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    query: Query<&ColonyMember, With<PlayerControlled>>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyT) {
        return;
    }

    let Ok(colony) = query.get_single() else {
        return;
    };

    let Some(ref mut all_grids) = grids else { return };
    let Some(grid) = all_grids.get_mut(colony.colony_id) else {
        return;
    };

    for y in 0..grid.height {
        for x in 0..grid.width {
            grid.clear_type(x, y, PheromoneType::Recruit);
            grid.clear_type(x, y, PheromoneType::AttackRecruit);
        }
    }
}

fn exchange_ant(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<PlayerControlled>>,
    candidate_query: Query<(Entity, &Transform, &MapId), (With<Ant>, Without<PlayerControlled>)>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyX) {
        return;
    }

    let Ok((player_entity, player_tf)) = player_query.get_single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let mut nearest: Option<(Entity, f32)> = None;

    for (entity, tf, map_id) in &candidate_query {
        // Only swap to surface ants.
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

fn update_follower_count(
    mut count: ResMut<FollowerCount>,
    query: Query<&Ant>,
) {
    count.0 = query.iter().filter(|a| a.state == AntState::Following).count();
}

fn camera_follow_player(
    mode: Res<PlayerMode>,
    player_query: Query<&Transform, (With<PlayerControlled>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    if !mode.follow_camera {
        return;
    }

    let Ok(player_tf) = player_query.get_single() else {
        return;
    };

    let Ok(mut cam_tf) = camera_query.get_single_mut() else {
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
    let Ok((mut sprite, carried)) = query.get_single_mut() else {
        return;
    };

    sprite.color = if carried.is_some() {
        PLAYER_CARRY_COLOR
    } else {
        PLAYER_COLOR
    };
    sprite.custom_size = Some(Vec2::splat(6.0));
}
