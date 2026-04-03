use bevy::prelude::*;

use crate::components::ant::{Ant, CarriedItem, Caste, AntState, Movement};
use crate::components::terrain::FoodSource;
use crate::plugins::combat::{Antlion, EnemyColonyNest};
use crate::plugins::spider_ai::{Spider, SpiderState};

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

// ── Nest spritesheet ──────────────────────────────────────────────

const NEST_FRAME_SIZE: u32 = 64;
const NEST_COLS: u32 = 8;
const NEST_ROWS: u32 = 4;

const NEST_ROW_PLAYER_IDLE: usize = 0;
const NEST_ROW_PLAYER_ACTIVE: usize = 1;
const NEST_ROW_ENEMY_IDLE: usize = 2;
const NEST_ROW_ENEMY_ACTIVE: usize = 3;

#[derive(Resource)]
pub struct NestSpritesheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

/// Marker component for player colony nest mound sprites on the surface.
#[derive(Component)]
pub struct NestMound;

#[derive(Component)]
pub struct NestAnimation {
    pub timer: f32,
    pub frame: usize,
    pub fps: f32,
    pub row: usize,
}

impl NestAnimation {
    pub fn player() -> Self {
        Self { timer: 0.0, frame: 0, fps: 3.0, row: NEST_ROW_PLAYER_IDLE }
    }
    pub fn enemy() -> Self {
        Self { timer: 0.0, frame: 0, fps: 3.0, row: NEST_ROW_ENEMY_IDLE }
    }
}

// ── Antlion spritesheet ───────────────────────────────────────────

const ANTLION_FRAME_SIZE: u32 = 96;
const ANTLION_COLS: u32 = 8;
const ANTLION_ROWS: u32 = 4;

const ANTLION_ROW_IDLE: usize = 0;
const ANTLION_ROW_ATTACK: usize = 1;
const ANTLION_ROW_DEATH: usize = 2;
const ANTLION_ROW_PIT: usize = 3;

#[derive(Resource)]
pub struct AntlionSpritesheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

/// Marker for the antlion sand pit visual entity.
#[derive(Component)]
pub struct AntlionPit;

#[derive(Component)]
pub struct AntlionAnimation {
    pub timer: f32,
    pub frame: usize,
    pub fps: f32,
    pub row: usize,
}

impl AntlionAnimation {
    pub fn creature() -> Self {
        Self { timer: 0.0, frame: 0, fps: 4.0, row: ANTLION_ROW_IDLE }
    }
    pub fn pit() -> Self {
        Self { timer: 0.0, frame: 0, fps: 2.0, row: ANTLION_ROW_PIT }
    }
}

// ── Spider spritesheet ─────────────────────────────────────────────

const SPIDER_FRAME_SIZE: u32 = 48;
const SPIDER_COLS: u32 = 8;
const SPIDER_ROWS: u32 = 3;

const SPIDER_ROW_WALK: usize = 0;
const SPIDER_ROW_ATTACK: usize = 1;
const SPIDER_ROW_DEATH: usize = 2;

#[derive(Resource)]
pub struct SpiderSpritesheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
pub struct SpiderAnimation {
    pub timer: f32,
    pub frame: usize,
    pub fps: f32,
    pub row: usize,
}

impl Default for SpiderAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            frame: 0,
            fps: 6.0,
            row: SPIDER_ROW_WALK,
        }
    }
}

// ── Food spritesheet ──────────────────────────────────────────────

const FOOD_FRAME_SIZE: u32 = 32;
const FOOD_COLS: u32 = 8;
const FOOD_ROWS: u32 = 1;

/// Food variant indices in the spritesheet.
/// 0=turkey leg, 1=ham, 2=whole roast, 3=steak, 4=drumstick, 5=apple, 6=cheese, 7=bread
const FOOD_LARGE: [usize; 4] = [0, 1, 2, 3]; // big food (amount > 50)
const FOOD_SMALL: [usize; 4] = [4, 5, 6, 7]; // small food (amount <= 50)

#[derive(Resource)]
pub struct FoodSpritesheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

/// Marks a food entity that has been given a sprite variant.
#[derive(Component)]
pub struct FoodVariant(pub usize);

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
        app.add_systems(Startup, (
                load_spritesheet, load_spider_spritesheet,
                load_nest_spritesheet, load_antlion_spritesheet,
                load_food_spritesheet,
            ))
            .add_systems(Update, (
                retrofit_ant_sprites, animate_sprites, select_sprite_row, orient_sprites,
                retrofit_spider_sprites, animate_spider_sprites, select_spider_row,
                retrofit_nest_sprites, animate_nest_sprites, select_nest_row,
                retrofit_antlion_sprites, animate_antlion_sprites, select_antlion_row,
                retrofit_food_sprites, scale_food_sprites,
            ).chain());
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

// ── Spider systems ─────────────────────────────────────────────────

fn load_spider_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("spider_spritesheet.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(SPIDER_FRAME_SIZE), SPIDER_COLS, SPIDER_ROWS, None, None,
    );
    let layout_handle = layouts.add(layout);
    commands.insert_resource(SpiderSpritesheet { image, layout: layout_handle });
}

fn retrofit_spider_sprites(
    mut commands: Commands,
    sheet: Option<Res<SpiderSpritesheet>>,
    mut query: Query<(Entity, &mut Sprite), (With<Spider>, Without<SpiderAnimation>)>,
) {
    let Some(sheet) = sheet else { return };
    for (entity, mut sprite) in &mut query {
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(18.0));
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: 0,
        });
        commands.entity(entity).insert(SpiderAnimation::default());
    }
}

fn animate_spider_sprites(
    time: Res<Time>,
    clock: Res<crate::resources::simulation::SimClock>,
    mut query: Query<&mut SpiderAnimation>,
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
            anim.frame = (anim.frame + 1) % SPIDER_COLS as usize;
        }
    }
}

fn select_spider_row(
    mut query: Query<(&Spider, &mut SpiderAnimation, &mut Sprite)>,
) {
    for (spider, mut anim, mut sprite) in &mut query {
        let row = if spider.hp <= 0.0 {
            SPIDER_ROW_DEATH
        } else if spider.state == SpiderState::Chasing && spider.attack_cooldown > 0.0 {
            SPIDER_ROW_ATTACK
        } else {
            SPIDER_ROW_WALK
        };

        if anim.row != row {
            anim.row = row;
            anim.frame = 0;
            anim.timer = 0.0;
        }

        let index = anim.row * SPIDER_COLS as usize + anim.frame;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }
}

// ── Nest systems ──────────────────────────────────────────────────

fn load_nest_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("nest_spritesheet.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(NEST_FRAME_SIZE), NEST_COLS, NEST_ROWS, None, None,
    );
    let layout_handle = layouts.add(layout);
    commands.insert_resource(NestSpritesheet { image, layout: layout_handle });
}

/// Apply the nest spritesheet to player nest mounds (NestMound marker).
fn retrofit_nest_sprites(
    mut commands: Commands,
    sheet: Option<Res<NestSpritesheet>>,
    mut player_query: Query<(Entity, &mut Sprite), (With<NestMound>, Without<NestAnimation>)>,
    mut enemy_query: Query<(Entity, &mut Sprite), (With<EnemyColonyNest>, Without<NestAnimation>, Without<NestMound>)>,
) {
    let Some(sheet) = sheet else { return };

    for (entity, mut sprite) in &mut player_query {
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(36.0));
        sprite.color = Color::WHITE;
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: 0,
        });
        commands.entity(entity).insert(NestAnimation::player());
    }

    for (entity, mut sprite) in &mut enemy_query {
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(36.0));
        sprite.color = Color::WHITE;
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: NEST_ROW_ENEMY_IDLE * NEST_COLS as usize,
        });
        commands.entity(entity).insert(NestAnimation::enemy());
    }
}

fn animate_nest_sprites(
    time: Res<Time>,
    clock: Res<crate::resources::simulation::SimClock>,
    mut query: Query<&mut NestAnimation>,
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
            anim.frame = (anim.frame + 1) % NEST_COLS as usize;
        }
    }
}

fn select_nest_row(
    mut query: Query<(&NestAnimation, &mut Sprite)>,
) {
    for (anim, mut sprite) in &mut query {
        let index = anim.row * NEST_COLS as usize + anim.frame;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }
}

// ── Antlion systems ──────────────────────────────────────────────

fn load_antlion_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("antlion_spritesheet.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(ANTLION_FRAME_SIZE), ANTLION_COLS, ANTLION_ROWS, None, None,
    );
    let layout_handle = layouts.add(layout);
    commands.insert_resource(AntlionSpritesheet { image, layout: layout_handle });
}

fn retrofit_antlion_sprites(
    mut commands: Commands,
    sheet: Option<Res<AntlionSpritesheet>>,
    mut creature_query: Query<(Entity, &mut Sprite), (With<Antlion>, Without<AntlionAnimation>)>,
    mut pit_query: Query<(Entity, &mut Sprite), (With<AntlionPit>, Without<AntlionAnimation>, Without<Antlion>)>,
) {
    let Some(sheet) = sheet else { return };

    for (entity, mut sprite) in &mut creature_query {
        info!("Retrofit antlion creature sprite: {:?}", entity);
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(24.0));
        sprite.color = Color::WHITE;
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: 0,
        });
        commands.entity(entity).insert(AntlionAnimation::creature());
    }

    for (entity, mut sprite) in &mut pit_query {
        info!("Retrofit antlion pit sprite: {:?}", entity);
        sprite.image = sheet.image.clone();
        sprite.custom_size = Some(Vec2::splat(80.0));
        sprite.color = Color::WHITE;
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: ANTLION_ROW_PIT * ANTLION_COLS as usize,
        });
        commands.entity(entity).insert(AntlionAnimation::pit());
    }
}

fn animate_antlion_sprites(
    time: Res<Time>,
    clock: Res<crate::resources::simulation::SimClock>,
    mut query: Query<&mut AntlionAnimation>,
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
            anim.frame = (anim.frame + 1) % ANTLION_COLS as usize;
        }
    }
}

fn select_antlion_row(
    mut creature_query: Query<(&Antlion, &mut AntlionAnimation, &mut Sprite)>,
    mut pit_query: Query<(&AntlionAnimation, &mut Sprite), (With<AntlionPit>, Without<Antlion>)>,
) {
    for (antlion, mut anim, mut sprite) in &mut creature_query {
        let row = if antlion.hp <= 0.0 {
            ANTLION_ROW_DEATH
        } else {
            ANTLION_ROW_IDLE
        };

        if anim.row != row {
            anim.row = row;
            anim.frame = 0;
            anim.timer = 0.0;
        }

        let index = anim.row * ANTLION_COLS as usize + anim.frame;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }

    for (anim, mut sprite) in &mut pit_query {
        let index = anim.row * ANTLION_COLS as usize + anim.frame;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }
}

// ── Food systems ──────────────────────────────────────────────────

fn load_food_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("food_spritesheet.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(FOOD_FRAME_SIZE), FOOD_COLS, FOOD_ROWS, None, None,
    );
    let layout_handle = layouts.add(layout);
    commands.insert_resource(FoodSpritesheet { image, layout: layout_handle });
}

/// Apply the food spritesheet to any FoodSource entity that doesn't have a FoodVariant yet.
fn retrofit_food_sprites(
    mut commands: Commands,
    sheet: Option<Res<FoodSpritesheet>>,
    mut query: Query<(Entity, &FoodSource, &mut Sprite), Without<FoodVariant>>,
) {
    let Some(sheet) = sheet else { return };
    let mut rng = rand::thread_rng();
    use rand::Rng;

    for (entity, food, mut sprite) in &mut query {
        // Pick a variant based on food amount — large food gets meat, small gets snacks
        let variant = if food.max > 50.0 {
            FOOD_LARGE[rng.gen_range(0..FOOD_LARGE.len())]
        } else {
            FOOD_SMALL[rng.gen_range(0..FOOD_SMALL.len())]
        };

        sprite.image = sheet.image.clone();
        sprite.color = Color::WHITE;
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: variant,
        });

        // Scale display size based on food amount
        let size = if food.max > 50.0 { 14.0 } else { 10.0 };
        sprite.custom_size = Some(Vec2::splat(size));

        commands.entity(entity).insert(FoodVariant(variant));
    }
}

/// Shrink food sprite as it gets consumed.
fn scale_food_sprites(
    mut query: Query<(&FoodSource, &mut Sprite), With<FoodVariant>>,
) {
    for (food, mut sprite) in &mut query {
        let ratio = (food.remaining / food.max).clamp(0.1, 1.0);
        let base = if food.max > 50.0 { 14.0 } else { 10.0 };
        let size = base * (0.5 + 0.5 * ratio); // shrinks to 50% at minimum
        sprite.custom_size = Some(Vec2::splat(size));
    }
}

// ── Ant retrofit ───────────────────────────────────────────────────

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
