# Aseprite Sprite Loading Plan

## Library: `bevy_aseprite_ultra`

- **Repo**: https://github.com/Lommix/bevy_aseprite_ultra
- **Cargo**: `bevy_aseprite_ultra = "0.8"` (Bevy 0.18 — v0.8.1)
- **Input**: `.aseprite` binary files dropped directly into `assets/`
- **Key features**: hot reloading, tag-based animation control, animation chaining, one-shot events, sprite slices with pivots, blend modes, layer visibility, asset processor for release builds
- **API**: component-driven — `AseAnimation` for animations, `AseSlice` for static sprites

---

## Aseprite File Design

### File: `assets/ants/worker.aseprite`
Tags (animations):
| Tag | Frames | Description |
|-----|--------|-------------|
| `walk` | 8 | Default walk cycle |
| `carry` | 8 | Walking while carrying food |
| `idle` | 4 | Standing still |
| `fight` | 4 | Combat stance / attack |

### File: `assets/ants/soldier.aseprite`
Tags:
| Tag | Frames | Description |
|-----|--------|-------------|
| `walk` | 8 | Patrol / move cycle |
| `fight` | 6 | Mandible attack animation |
| `idle` | 4 | Guard stance |

### File: `assets/ants/queen.aseprite`
Tags:
| Tag | Frames | Description |
|-----|--------|-------------|
| `idle` | 4 | Resting / egg-laying |
| `walk` | 4 | Slow movement |

### File: `assets/ants/drone.aseprite` (optional)
Tags:
| Tag | Frames | Description |
|-----|--------|-------------|
| `walk` | 4 | Basic movement |

### Art Guidelines
- **Frame size**: 32×32 pixels (matches current spritesheet)
- **Orientation**: Facing RIGHT (+X); game rotates sprites via `Transform`
- **Palette**: Earth tones matching current pixel art
- **Layers**: Body, Legs, Carried-item (toggle layer visibility per tag)

---

## Integration Plan

### Phase 1: Add Dependency and Plugin
1. Add `bevy_aseprite_ultra = "0.8"` to `Cargo.toml`
2. Register `AsepriteUltraPlugin` in `main.rs`
3. Place `.aseprite` files in `assets/ants/`

### Phase 2: Update `ant_sprites.rs`
Replace the current `AntSpritesheet` / `AntAnimation` / `retrofit_ant_sprites` system with Aseprite-driven components.

**New resource:**
```rust
#[derive(Resource)]
pub struct AntAseprites {
    pub worker: Handle<Aseprite>,
    pub soldier: Handle<Aseprite>,
    pub queen: Handle<Aseprite>,
}
```

**Startup system:**
```rust
fn load_ant_aseprites(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(AntAseprites {
        worker: server.load("ants/worker.aseprite"),
        soldier: server.load("ants/soldier.aseprite"),
        queen: server.load("ants/queen.aseprite"),
    });
}
```

**Retrofit system** (replaces current `retrofit_ant_sprites`):
```rust
fn retrofit_ant_sprites(
    mut commands: Commands,
    ase: Option<Res<AntAseprites>>,
    query: Query<(Entity, &Ant), Without<AseAnimation>>,
) {
    let Some(ase) = ase else { return };
    for (entity, ant) in &query {
        let (handle, tag) = match ant.caste {
            Caste::Queen   => (ase.queen.clone(),   "idle"),
            Caste::Soldier => (ase.soldier.clone(),  "walk"),
            _              => (ase.worker.clone(),   "walk"),
        };
        commands.entity(entity).insert((
            AseAnimation {
                aseprite: handle,
                animation: Animation::tag(tag).with_repeat(AnimationRepeat::Loop),
            },
            Sprite {
                custom_size: Some(Vec2::splat(match ant.caste {
                    Caste::Queen => 14.0,
                    Caste::Soldier => 10.0,
                    _ => 8.0,
                })),
                ..default()
            },
        ));
    }
}
```

### Phase 3: State-Driven Animation Switching
Replace `select_sprite_row` with a system that switches Aseprite tags based on ant state:

```rust
fn update_ant_animation_tag(
    mut query: Query<(&Ant, &mut AseAnimation, Option<&CarriedItem>), Changed<Ant>>,
) {
    for (ant, mut ase_anim, carried) in &mut query {
        let tag = match ant.caste {
            Caste::Soldier => match ant.state {
                AntState::Defending | AntState::Fighting => "fight",
                _ => "walk",
            },
            Caste::Queen => match ant.state {
                AntState::Idle => "idle",
                _ => "walk",
            },
            _ => {
                if carried.is_some() { "carry" } else { "walk" }
            }
        };
        // Only update if tag actually changed to avoid restarting animation
        if ase_anim.animation.tag_name() != Some(tag) {
            ase_anim.animation = Animation::tag(tag)
                .with_repeat(AnimationRepeat::Loop);
        }
    }
}
```

### Phase 4: Orientation (keep existing)
The `orient_sprites` system already rotates `Transform` to face movement direction — this stays unchanged.

### Phase 5: Cleanup
- Remove `tools/generate_spritesheet.py` (or keep for reference)
- Remove `assets/ant_spritesheet.png`
- Remove `AntSpritesheet`, `AntAnimation`, old `animate_sprites`, `select_sprite_row` from `ant_sprites.rs`
- Update `update_ant_visuals` in `ant_ai.rs`: tinting via `Sprite.color` still works on top of Aseprite sprites for colony coloring

### Phase 6: Artist Workflow
- Artist edits `.aseprite` files in Aseprite
- With `bevy/file_watcher` feature, changes hot-reload in-game instantly
- No export step, no JSON, no re-running Python scripts

---

## Migration Checklist
- [ ] Add `bevy_aseprite_ultra = "0.8"` to Cargo.toml
- [ ] Create `.aseprite` files for worker, soldier, queen
- [ ] Register `AsepriteUltraPlugin` in main.rs
- [ ] Rewrite `ant_sprites.rs` to use Aseprite components
- [ ] Verify `update_ant_visuals` color tinting still works with Aseprite sprites
- [ ] Verify orient_sprites rotation still works
- [ ] Build check + runtime validation
- [ ] Remove old spritesheet assets and generator script
