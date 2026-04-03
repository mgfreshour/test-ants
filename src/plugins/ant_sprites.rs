use bevy::prelude::*;

use crate::components::ant::{Ant, CarriedItem, Caste, AntState, Movement};

/// Spritesheet layout constants.
const FRAME_SIZE: u32 = 32;
const COLS: u32 = 8;
const ROWS: u32 = 5;

/// Row indices in the spritesheet.
const ROW_WORKER_WALK: usize = 0;
const ROW_WORKER_CARRY: usize = 1;
const ROW_SOLDIER_WALK: usize = 2;
const ROW_SOLDIER_FIGHT: usize = 3;
const ROW_QUEEN: usize = 4;

/// Shared resource holding the loaded spritesheet handles.
#[derive(Resource)]
pub struct AntSpritesheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

/// Per-entity animation state.
#[derive(Component)]
pub struct AntAnimation {
    pub timer: f32,
    pub frame: usize,
    /// Frames per second for the walk cycle.
    pub fps: f32,
}

impl Default for AntAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            frame: 0,
            fps: 8.0,
        }
    }
}

pub struct AntSpritePlugin;

impl Plugin for AntSpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_spritesheet)
            .add_systems(Update, (retrofit_ant_sprites, animate_sprites, select_sprite_row, orient_sprites).chain());
    }
}

fn load_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("ant_spritesheet.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(FRAME_SIZE), COLS, ROWS, None, None);
    let layout_handle = layouts.add(layout);

    commands.insert_resource(AntSpritesheet {
        image,
        layout: layout_handle,
    });
}

/// Advance the walk-cycle frame timer.
fn animate_sprites(
    time: Res<Time>,
    clock: Res<crate::resources::simulation::SimClock>,
    mut query: Query<&mut AntAnimation>,
) {
    if clock.speed == crate::resources::simulation::SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for mut anim in &mut query {
        anim.timer += dt;
        let frame_dur = 1.0 / anim.fps;
        if anim.timer >= frame_dur {
            anim.timer -= frame_dur;
            anim.frame = (anim.frame + 1) % COLS as usize;
        }
    }
}

/// Pick the correct atlas row based on caste/state and set the atlas index.
fn select_sprite_row(
    mut query: Query<(&Ant, &AntAnimation, &mut Sprite, Option<&CarriedItem>)>,
) {
    for (ant, anim, mut sprite, carried) in &mut query {
        let row = match ant.caste {
            Caste::Queen => ROW_QUEEN,
            Caste::Soldier => {
                if ant.state == AntState::Defending || ant.state == AntState::Fighting {
                    ROW_SOLDIER_FIGHT
                } else {
                    ROW_SOLDIER_WALK
                }
            }
            _ => {
                if carried.is_some() {
                    ROW_WORKER_CARRY
                } else {
                    ROW_WORKER_WALK
                }
            }
        };

        let index = row * COLS as usize + anim.frame;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }
}

/// Rotate the sprite to face the movement direction.
/// The spritesheet frames face RIGHT (+X), so angle = atan2(dir.y, dir.x).
fn orient_sprites(
    mut query: Query<(&Movement, &mut Transform), With<Ant>>,
) {
    for (movement, mut transform) in &mut query {
        let dir = movement.direction;
        if dir.length_squared() > 0.001 {
            let angle = dir.y.atan2(dir.x);
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

/// Automatically apply the spritesheet to any ant that was spawned without it.
/// This runs every frame but only affects newly-spawned ants (those missing AntAnimation).
fn retrofit_ant_sprites(
    mut commands: Commands,
    sheet: Option<Res<AntSpritesheet>>,
    mut query: Query<(Entity, &Ant, &mut Sprite), Without<AntAnimation>>,
) {
    let Some(sheet) = sheet else { return };

    for (entity, ant, mut sprite) in &mut query {
        let size = match ant.caste {
            Caste::Queen => 14.0,
            Caste::Soldier => 10.0,
            _ => 8.0,
        };
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: 0,
        });

        commands.entity(entity).insert(AntAnimation::default());
    }
}
