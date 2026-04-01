use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{
    Ant, AntState, CarriedItem, Caste, ColonyMember, Movement, PlayerControlled, PositionHistory,
};
use crate::components::pheromone::PheromoneType;
use crate::components::terrain::FoodSource;
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::camera::MainCamera;
use crate::resources::pheromone::{PheromoneConfig, PheromoneGrid};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

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
const RECRUIT_RADIUS: f32 = 100.0;
const PICKUP_RANGE: f32 = 25.0;
const FOLLOW_DISTANCE: f32 = 30.0;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMode>()
            .init_resource::<FollowerCount>()
            .add_systems(
                Update,
                (
                    designate_player_ant,
                    toggle_player_mode,
                    player_movement,
                    player_pickup,
                    player_drop,
                    player_pheromone,
                    player_recruit,
                    player_dismiss,
                    exchange_ant,
                    follower_steering,
                    update_follower_count,
                    camera_follow_player,
                    update_player_visual,
                )
                    .chain(),
            );
    }
}

fn designate_player_ant(
    mut commands: Commands,
    query: Query<Entity, With<Ant>>,
    existing: Query<Entity, With<PlayerControlled>>,
) {
    if !existing.is_empty() {
        return;
    }
    if let Some(entity) = query.iter().next() {
        commands.entity(entity).insert(PlayerControlled);
    }
}

fn toggle_player_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlayerMode>,
) {
    if input.just_pressed(KeyCode::KeyF) {
        if mode.controlling {
            mode.controlling = false;
            mode.follow_camera = false;
        } else {
            mode.controlling = true;
            mode.follow_camera = true;
        }
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
    mut colony_food: ResMut<ColonyFood>,
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
        colony_food.stored += carried.food_amount;
    }
    commands.entity(entity).remove::<CarriedItem>();
}

fn player_pheromone(
    clock: Res<SimClock>,
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    pconfig: Res<PheromoneConfig>,
    mut grid: Option<ResMut<PheromoneGrid>>,
    query: Query<&Transform, With<PlayerControlled>>,
) {
    if !mode.controlling || clock.speed == SimSpeed::Paused {
        return;
    }

    let shift = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);
    if !shift {
        return;
    }

    let Ok(transform) = query.get_single() else {
        return;
    };

    let Some(ref mut grid) = grid else { return };
    let pos = transform.translation.truncate();
    if let Some((gx, gy)) = grid.world_to_grid(pos) {
        let amt = pconfig.deposit_amount(PheromoneType::Trail) * 3.0;
        grid.deposit(gx, gy, PheromoneType::Trail, amt, pconfig.max_intensity);
    }
}

fn player_recruit(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    player_query: Query<&Transform, With<PlayerControlled>>,
    ant_query: Query<
        (Entity, &Transform, &Ant),
        (Without<PlayerControlled>, Without<CarriedItem>),
    >,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyR) {
        return;
    }

    let Ok(player_tf) = player_query.get_single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let mut recruited = 0;

    for (entity, tf, ant) in &ant_query {
        if ant.state == AntState::Following {
            continue;
        }
        let dist = player_pos.distance(tf.translation.truncate());
        if dist < RECRUIT_RADIUS && recruited < 8 {
            commands.entity(entity).insert(crate::components::ant::Follower);
            recruited += 1;
        }
    }
}

fn player_dismiss(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Ant), With<crate::components::ant::Follower>>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyT) {
        return;
    }

    for (entity, mut ant) in &mut query {
        ant.state = AntState::Foraging;
        commands.entity(entity).remove::<crate::components::ant::Follower>();
    }
}

fn exchange_ant(
    input: Res<ButtonInput<KeyCode>>,
    mode: Res<PlayerMode>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<PlayerControlled>>,
    candidate_query: Query<(Entity, &Transform), (With<Ant>, Without<PlayerControlled>)>,
) {
    if !mode.controlling || !input.just_pressed(KeyCode::KeyX) {
        return;
    }

    let Ok((player_entity, player_tf)) = player_query.get_single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let mut nearest: Option<(Entity, f32)> = None;

    for (entity, tf) in &candidate_query {
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

fn follower_steering(
    clock: Res<SimClock>,
    time: Res<Time>,
    player_query: Query<&Transform, With<PlayerControlled>>,
    mut follower_query: Query<
        (&mut Transform, &Movement, &mut Ant),
        (With<crate::components::ant::Follower>, Without<PlayerControlled>),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Ok(player_tf) = player_query.get_single() else {
        return;
    };

    let target = player_tf.translation.truncate();
    let mut rng = rand::thread_rng();

    for (mut tf, movement, mut ant) in &mut follower_query {
        ant.state = AntState::Following;
        let pos = tf.translation.truncate();
        let to_player = target - pos;
        let dist = to_player.length();

        if dist > FOLLOW_DISTANCE {
            let dir = to_player.normalize();
            let jitter = Vec2::new(
                rng.gen_range(-0.15..0.15),
                rng.gen_range(-0.15..0.15),
            );
            let move_dir = (dir + jitter).normalize();
            let speed = movement.speed * clock.speed.multiplier() * time.delta_secs();
            tf.translation.x += move_dir.x * speed;
            tf.translation.y += move_dir.y * speed;
        }
    }
}

fn update_follower_count(
    mut count: ResMut<FollowerCount>,
    query: Query<&crate::components::ant::Follower>,
) {
    count.0 = query.iter().count();
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
