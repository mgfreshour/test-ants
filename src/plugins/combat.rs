use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{
    Ant, AntState, CarriedItem, ColonyMember, Follower, Health, Movement, PlayerControlled,
    PositionHistory, TrailSense,
};
use crate::components::map::MapId;
use crate::components::pheromone::PheromoneType;
use crate::resources::active_map::{MapRegistry, viewing_surface};
use crate::resources::pheromone::ColonyPheromones;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct CombatPlugin;

const RED_COLONY_ID: u32 = 1;
const COMBAT_RANGE: f32 = 15.0;
const ALARM_SENSE_RADIUS: i32 = 6;

#[derive(Component)]
pub struct Spider {
    pub hp: f32,
    pub attack_cooldown: f32,
}

#[derive(Component)]
pub struct Corpse {
    pub timer: f32,
}

#[derive(Component)]
pub struct EnemyColonyNest;

#[derive(Resource, Default)]
pub struct GameResult {
    pub decided: bool,
    pub player_won: bool,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {

        app.init_resource::<GameResult>()
            .add_systems(Startup, (spawn_red_colony, spawn_spider))
            .add_systems(
                Update,
                (
                    ant_combat_detection,
                    combat_resolution,
                    alarm_pheromone_deposit,
                    alarm_response_steering,
                    spider_ai,
                    death_system,
                    corpse_decay,
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

fn spawn_spider(mut commands: Commands, config: Res<SimConfig>) {
    let mut rng = rand::thread_rng();
    let cx = config.world_width / 2.0;
    let cy = config.world_height / 2.0;

    let x = cx + rng.gen_range(-300.0..300.0);
    let y = cy + rng.gen_range(-300.0..300.0);

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
        },
    ));
}

fn ant_combat_detection(
    clock: Res<SimClock>,
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

    for (entity, _, _, mut ant) in &mut query {
        if in_combat.contains(&entity) {
            ant.state = AntState::Defending;
        } else if ant.state == AntState::Defending {
            ant.state = AntState::Foraging;
        }
    }
}

fn combat_resolution(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(&Transform, &ColonyMember, &Ant, &mut Health)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    let combatants: Vec<(Vec2, u32)> = query
        .iter()
        .filter(|(_, _, ant, _)| ant.state == AntState::Defending || ant.state == AntState::Fighting)
        .map(|(t, c, _, _)| (t.translation.truncate(), c.colony_id))
        .collect();

    for (transform, colony, ant, mut health) in &mut query {
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
            health.current -= damage;
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
        (Without<PlayerControlled>, Without<Follower>, Without<CarriedItem>),
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

fn spider_ai(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut spider_query: Query<(&Transform, &mut Spider)>,
    mut ant_query: Query<(&Transform, &mut Health), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let spider_range = 50.0;

    for (spider_tf, mut spider) in &mut spider_query {
        spider.attack_cooldown = (spider.attack_cooldown - dt).max(0.0);
        if spider.attack_cooldown > 0.0 {
            continue;
        }

        let spider_pos = spider_tf.translation.truncate();

        for (ant_tf, mut health) in &mut ant_query {
            let dist = spider_pos.distance(ant_tf.translation.truncate());
            if dist < spider_range {
                health.current -= 8.0;
                spider.attack_cooldown = 0.5;
                break;
            }
        }
    }
}

fn player_attack(
    input: Res<ButtonInput<KeyCode>>,
    player_query: Query<(&Transform, &ColonyMember), With<PlayerControlled>>,
    mut enemy_query: Query<(&Transform, &ColonyMember, &mut Health), (With<Ant>, Without<PlayerControlled>)>,
    mut spider_query: Query<(&Transform, &mut Spider)>,
) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok((player_tf, player_colony)) = player_query.get_single() else {
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
            health.current -= damage;
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
) {
    for (entity, transform, health, ant) in &ant_query {
        if health.current <= 0.0 {
            let pos = transform.translation;
            let reason = if ant.hunger >= 1.0 { "starvation" } else { "combat" };
            info!("Ant died: reason={}, age={:.0}s, pos=({:.0},{:.0})", reason, ant.age, pos.x, pos.y);
            commands.entity(entity).despawn();
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.3, 0.2, 0.1, 0.6),
                    custom_size: Some(Vec2::splat(3.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 1.8),
                Corpse { timer: 5.0 },
            ));
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
