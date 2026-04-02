use bevy::prelude::*;
use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, TrailSense, PlayerControlled};
use crate::components::nest::NestTask;

#[derive(Component)]
pub struct StateLabel;

pub fn spawn_state_labels(
    mut commands: Commands,
    query: Query<Entity, (With<Ant>, Without<Children>, Without<NestTask>)>,
) {
    for entity in &query {
        let child = commands.spawn((
            Text2d::new("F"),
            TextFont {
                font_size: 8.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            Transform::from_xyz(0.0, 5.0, 0.1),
            StateLabel,
        )).id();
        commands.entity(entity).add_child(child);
    }
}

pub fn update_state_labels(
    ant_query: Query<(&Ant, Option<&TrailSense>, &Children), Without<NestTask>>,
    mut label_query: Query<(&mut Text2d, &mut TextColor), With<StateLabel>>,
) {
    for (ant, sense, children) in &ant_query {
        let sense = sense.copied().unwrap_or_default();

        let (letter, color) = match ant.state {
            AntState::Defending | AntState::Fighting => ("!", Color::srgb(1.0, 0.3, 0.3)),
            AntState::Following => (">", Color::srgb(0.5, 0.8, 1.0)),
            AntState::Attacking => ("!", Color::srgb(1.0, 0.35, 0.2)),
            AntState::Idle => ("I", Color::srgba(1.0, 1.0, 1.0, 0.5)),
            AntState::Nursing => ("N", Color::srgb(0.8, 0.6, 1.0)),
            AntState::Digging => ("G", Color::srgb(0.7, 0.5, 0.3)),
            AntState::Fleeing => ("X", Color::srgb(1.0, 1.0, 0.2)),
            AntState::Foraging => match sense {
                TrailSense::FollowingFood => ("f", Color::srgb(1.0, 0.6, 0.1)),
                TrailSense::FollowingAlarm => ("a", Color::srgb(1.0, 0.2, 0.2)),
                TrailSense::FollowingTrail => ("t", Color::srgb(0.8, 0.8, 0.2)),
                TrailSense::BeelineFood => ("*", Color::srgb(0.2, 1.0, 0.2)),
                _ => ("?", Color::srgba(1.0, 1.0, 1.0, 0.6)),
            },
            AntState::Returning => match sense {
                TrailSense::FollowingHome => ("h", Color::srgb(0.4, 0.6, 1.0)),
                TrailSense::BeelineNest => ("^", Color::srgb(0.3, 1.0, 1.0)),
                _ => ("r", Color::srgba(1.0, 1.0, 1.0, 0.6)),
            },
        };

        for child in children.iter() {
            if let Ok((mut text, mut text_color)) = label_query.get_mut(child) {
                **text = letter.to_string();
                *text_color = TextColor(color);
            }
        }
    }
}

/// Tint ants based on state: dark = foraging, green-tinted = carrying food
pub fn update_ant_visuals(
    mut query: Query<(&Ant, &ColonyMember, &mut Sprite, Option<&CarriedItem>), Without<PlayerControlled>>,
) {
    for (ant, colony, mut sprite, carried) in &mut query {
        let is_red = colony.colony_id != 0;
        let fighting = ant.state == AntState::Defending || ant.state == AntState::Fighting;

        sprite.color = match (is_red, carried.is_some(), fighting) {
            (_, _, true) => Color::srgb(1.0, 0.2, 0.2),
            (true, true, _) => Color::srgb(0.9, 0.3, 0.1),
            (true, false, _) => Color::srgb(0.7, 0.15, 0.1),
            (false, true, _) => Color::srgb(0.9, 0.4, 0.1),
            (false, false, _) => Color::srgb(0.1, 0.1, 0.1),
        };
    }
}
