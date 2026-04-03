


Here are the detailed sprint plans for LDtk integration, formatted to match your existing [IMPLEMENTATION_PLAN.md](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/docs/IMPLEMENTATION_PLAN.md:0:0-0:0) conventions. These slot in as a parallel track — they can run alongside or after Sprints 12–18 (ant unification) and should ideally complete before Sprint 21 (Campaign Mode), since campaign benefits from designer-editable multi-patch maps.

---

# LDtk Map Integration — Sprint Plans

## Progress

| Sprint | Status | Notes |
|--------|--------|-------|
| **L1** | **Done** | Surface loads from LDtk, tilemap rendering, MapId tagging |
| **L2** | **Done** | Nest IntGrid pipeline, NestGrid::from_intgrid, TileColor mutation on dig |
| **L3** | **Done** | Entity integration (food/portal/queen/entrance from LDtk), portal wiring by portal_id, dig task bug fix |
| **L4** | Not started | Tilesets, multiple layouts, campaign prep |

**Open items from L2/L3:**
- Nest dimensions still use NEST_WIDTH/NEST_HEIGHT constants (not dynamic from LDtk metadata)
- Hot-reload not implemented
- MapRegistry still built statically in PreStartup; dynamic level discovery deferred to L4

## Overview

4 sprints (L1–L4). Each produces a runnable build. L1–L2 are sequential. L3 depends on L2. L4 is polish/content and can overlap with Sprint 21 (Campaign).

```
Sprint L1  ████████████████  LDtk Foundation & Surface Map         ✅ DONE
Sprint L2  ████████████████  Nest IntGrid Pipeline                 ✅ DONE
Sprint L3  ████████████████  Entity Integration & Portals          ✅ DONE
Sprint L4  ░░░░░░░░░░░░░░░░  Polish, Tilesets & Campaign Prep
```

**Dependency chain:**
```
Sprint L1 ──► Sprint L2 ──► Sprint L3 ──► Sprint L4
(foundation)  (nest grids)  (entities &    (tilesets &
                             portals)       campaign maps)
                                │
                                ▼
                          Sprint 21 (Campaign Mode)
```

**Prerequisites:** None — these sprints are independent of the ant unification track (Sprints 12–18). However, L3's portal wiring benefits from Sprint 14 (job-driven transitions) being complete.

---

## Sprint L1: LDtk Foundation & Surface Map

### Goal
Add `bevy_ecs_ldtk` + `bevy_ecs_tilemap` dependencies, create the LDtk project file with a Surface level, and replace the procedural grass-tile spawning with an LDtk-loaded tilemap. Existing gameplay unchanged — this is a rendering-source swap for the surface only.

### Tasks

| # | Task | Est |
|---|---|---|
| L1.1 | Add `bevy_ecs_ldtk = "0.14"` to [Cargo.toml](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/Cargo.toml:0:0-0:0), verify it compiles with Bevy 0.18 | 1h |
| L1.2 | Register `LdtkPlugin` in `main.rs`, configure `LdtkSettings` (set `LevelSpawnBehavior::UseWorldTranslation`, disable `SetClearColor`) | 2h |
| L1.3 | Create minimal tileset PNG: 16×16 tiles for grass_dark, grass_light, dirt, sand, concrete, nest_mound, nest_hole (placeholder colors matching current `Sprite` colors) | 3h |
| L1.4 | Install LDtk editor, create `assets/maps/colony.ldtk` project. Configure 16px grid, import tileset | 2h |
| L1.5 | Design Surface level in LDtk: Tile layer for grass checkerboard using Auto-Rules (alternating dark/light), nest mound at `config.nest_position` equivalent | 4h |
| L1.6 | Create `src/plugins/ldtk_maps.rs` — new `LdtkMapsPlugin`: loads `.ldtk` asset, spawns `LdtkWorldBundle`, tags the spawned level entity with [MapMarker](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:18:0-18:21) + `MapKind::Surface` | 4h |
| L1.7 | Refactor [setup_terrain](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:30:0-84:1): remove the grass tile loop (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:63-84`), keep nest entrance marker sprites (or move them to LDtk entities in L3) | 3h |
| L1.8 | Ensure [MapId(registry.surface)](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:5:0-5:29) is applied to LDtk-spawned tile entities so [sync_map_visibility](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:263:0-280:1) still works. Use `bevy_ecs_ldtk`'s `LevelEvent::Spawned` to tag tiles post-spawn | 3h |
| L1.9 | Verify `SimConfig.world_width/world_height` matches LDtk level pixel dimensions. Extract from `LdtkProject` asset at startup or hardcode to match | 2h |
| L1.10 | Update `assets/` directory structure: add `.gitignore` entry for LDtk backups (`*.ldtk.bak`), ensure `.ldtk` + tileset PNG are tracked | 1h |
| L1.11 | Build check + runtime validation — surface renders from LDtk, ants forage normally, pheromones work | 2h |

### Demo
> Launch game. Surface map now loads from the LDtk file instead of spawning individual grass sprites. Tilemap renders via `bevy_ecs_tilemap` chunked rendering — visually identical checkerboard pattern but more GPU-efficient. All existing gameplay (foraging, pheromones, combat, hazards) works unchanged. Tab to nest view — still uses old procedural rendering (Sprint L2).

### Acceptance Criteria
- [x] `bevy_ecs_ldtk` compiles and runs with Bevy 0.18
- [x] Surface level loads from `.ldtk` file
- [x] Grass tilemap renders correctly (matches old visual)
- [x] MapId tagging works — visibility toggle via Tab still functions
- [x] Ants forage/return/fight normally on the LDtk surface
- [x] Pheromone overlay still renders on top of tilemap
- [x] No regression in existing tests

**Files touched:**
- [Cargo.toml](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/Cargo.toml:0:0-0:0) — add `bevy_ecs_ldtk`
- `src/main.rs` — register `LdtkMapsPlugin`
- `src/plugins/ldtk_maps.rs` (new) — LDtk loading, level tagging
- [src/plugins/terrain.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:0:0-0:0) — remove grass tile loop, keep food spawning
- `src/plugins/mod.rs` — export new module
- `assets/maps/colony.ldtk` (new) — LDtk project
- `assets/tilesets/terrain.png` (new) — tileset sprite sheet

**Risks:**
- `bevy_ecs_tilemap` rendering may conflict with existing `Sprite`-based overlays (pheromones, hazards). Mitigation: pheromones render at z=5+, tiles at z=0 — layering should work, but verify.
- LDtk asset loading is async — systems that assume surface exists at `Startup` need to wait for `LevelEvent::Spawned`. May need to move some logic to `PostStartup` or use a run condition.

---

## Sprint L2: Nest IntGrid Pipeline

### Goal
Create nest levels in LDtk using IntGrid layers. Build a sync pipeline that populates [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) from LDtk IntGrid data at level load. Support runtime mutation (digging) by keeping [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) as the mutable truth and syncing visual tile changes back to the tilemap.

### Tasks

| # | Task | Est |
|---|---|---|
| L2.1 | Define IntGrid value mapping in LDtk project: `1`=Soil, `2`=SoftSoil, `3`=Clay, `4`=Rock, `5`=Tunnel, `6`=Chamber(Queen), `7`=Chamber(Brood), `8`=Chamber(FoodStorage), `9`=Chamber(Midden) | 1h |
| L2.2 | Create tileset for nest cells: 16×16 tiles matching current [CellType::color()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:42:4-56:5) values (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:43-57`). One tile per `CellType` variant | 2h |
| L2.3 | Design "PlayerNest" level in LDtk: 60×40 IntGrid layer. Recreate the default layout from [NestGrid::default()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/simulation.rs:51:4-57:5) (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:16-89`) — entrance tunnel, food storage, brood chamber, queen chamber, midden, connecting tunnels | 3h |
| L2.4 | Design "RedNest" level in LDtk: identical layout (or a distinct mirror/variation for visual interest) | 2h |
| L2.5 | Register IntGrid cell bundles: `app.register_ldtk_int_cell::<NestCellBundle>(1)` through `(9)`. Bundle carries `NestTile { grid_x, grid_y }` derived from `GridCoords` | 4h |
| L2.6 | Create `NestGrid::from_ldtk()` constructor — system that runs on `LevelEvent::Spawned` for nest levels. Queries all `IntGridCell` + `GridCoords` entities in the level, builds the `cells: Vec<Vec<CellType>>` array. Replaces [NestGrid::default()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/simulation.rs:51:4-57:5) | 5h |
| L2.7 | Attach [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) (+ `NestPheromoneGrid`, `NestPathCache`, etc.) to the nest map entity after LDtk level spawn, replacing the current [setup_maps](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:65:0-138:1) bundle insertion (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:74-88`) | 4h |
| L2.8 | Runtime tile mutation system: when [NestGrid::set()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:100:4-108:5) is called (excavation), find the corresponding `bevy_ecs_tilemap` tile entity via `GridCoords` and update its `TileTextureIndex` to the new `CellType`'s tileset index. Replaces current [NestTile](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:60:0-63:1) sprite color updates in `nest_ai.rs` excavation systems | 6h |
| L2.9 | Make `NEST_WIDTH` / `NEST_HEIGHT` dynamic — read from LDtk level dimensions instead of constants. Update `NestPheromoneGrid` and `NestPathCache` to use dynamic sizing | 4h |
| L2.10 | Remove [render_nest](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:142:0-166:1) system (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:143-167`) — LDtk + `bevy_ecs_tilemap` handles initial tile rendering | 2h |
| L2.11 | Verify [nest_grid_to_world](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:29:0-36:1) / world-to-grid conversions align with LDtk coordinate system. LDtk uses top-left origin; Bevy uses center-origin. Adjust offsets if needed | 3h |
| L2.12 | Unit tests: `NestGrid::from_ldtk()` with mock `IntGridCell` data produces correct grid | 3h |
| L2.13 | Build check + runtime validation — nest renders from LDtk, digging works, pathfinding works, brood lifecycle works | 3h |

### Demo
> Tab to nest view. The underground map now loads from the LDtk file — same layout as before but rendered via `bevy_ecs_tilemap`. Ants navigate tunnels, nurse brood, and dig new passages. When a digger excavates soil, the tilemap tile visually updates to tunnel coloring. Open the LDtk editor, move the queen chamber deeper, save — hot-reload updates the nest layout in-game.

### Acceptance Criteria
- [x] PlayerNest and RedNest levels load from LDtk IntGrid
- [x] NestGrid is populated from LDtk data (not hardcoded default())
- [ ] Nest dimensions are read from LDtk level metadata
- [x] Excavation visually updates tilemap tiles in real-time
- [x] Pathfinding and pheromone grids work with LDtk-loaded dimensions
- [x] nest_grid_to_world coordinate conversion matches LDtk layout
- [x] Existing nest AI (nursing, hauling, queen care, digging) all function correctly
- [x] Unit tests for `from_intgrid` pass
- [ ] Hot-reload of `.ldtk` file updates nest layout

**Files touched:**
- `src/plugins/ldtk_maps.rs` — nest level spawn handling, `NestGrid::from_ldtk()` system
- [src/resources/nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:0:0-0:0) — `from_ldtk()` constructor, dynamic width/height, remove `Default` impl (or keep for tests)
- [src/plugins/nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0) — remove [render_nest](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:142:0-166:1), refactor [setup_maps](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:65:0-138:1) to defer nest component insertion until LDtk spawn
- `src/plugins/nest_ai.rs` — update excavation visual sync to use `TileTextureIndex` instead of [Sprite.color](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:42:4-56:5)
- `src/resources/nest_pheromone.rs` — dynamic grid sizing
- `src/resources/nest_pathfinding.rs` — dynamic grid sizing
- `assets/maps/colony.ldtk` — add PlayerNest + RedNest levels
- `assets/tilesets/nest.png` (new) — nest tileset

**Risks:**
- **Coordinate system mismatch**: LDtk IntGrid Y=0 is top, Bevy world Y+ is up. [nest_grid_to_world](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:29:0-36:1) already inverts Y (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:30-37`), but LDtk's `GridCoords` may use a different convention than your `(grid_x, grid_y)`. Must verify with a test level.
- **Hot-reload + runtime state**: If the LDtk file is hot-reloaded mid-game, the [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) must be rebuilt but in-progress dig operations and ant positions will reference stale grid data. Mitigation: only allow hot-reload when paused, or gate it behind a dev flag.
- **Tilemap tile mutation API**: `bevy_ecs_tilemap` tile updates require setting `TileTextureIndex` and marking the chunk dirty. This is different from just changing [Sprite.color](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:42:4-56:5). Need to understand the `bevy_ecs_tilemap` chunk invalidation API.

---

## Sprint L3: Entity Integration & Portal Wiring

### Goal
Move food sources, portal positions, queen spawn points, and nest entrance markers into LDtk Entity layers. Replace hardcoded positions with designer-placed entities. Wire portal pairs across levels using a shared `portal_id` field.

### Tasks

| # | Task | Est |
|---|---|---|
| L3.1 | Define LDtk entity definitions in the project: [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) (fields: `amount: Float`, `max: Float`, `size: Float`), `PortalPoint` (fields: `portal_id: String`, `colony_id: Int`, `is_entrance: Bool`), `QueenSpawn`, `NestEntrance` | 3h |
| L3.2 | Place entities in Surface level: ~20 [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) entities at varied positions, 2 `NestEntrance` markers (player + red), 2 `PortalPoint` entities at nest entrances | 2h |
| L3.3 | Place entities in nest levels: `QueenSpawn` in queen chamber, `PortalPoint` at entrance tunnel top cell. One per nest level | 2h |
| L3.4 | Implement `#[derive(LdtkEntity)]` for `FoodSourceBundle`: auto-inserts [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) component from LDtk field values, `Sprite` with size/color derived from amount, [MapId](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:5:0-5:29) from parent level | 4h |
| L3.5 | Implement `#[derive(LdtkEntity)]` for `PortalPointMarker`: captures `portal_id`, `colony_id`, position. Does NOT auto-create [MapPortal](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:29:0-42:1) — that happens in the wiring system | 3h |
| L3.6 | Create `wire_portals` system: runs after all levels are spawned. Queries all `PortalPointMarker` entities, groups by `portal_id`, creates [MapPortal](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:29:0-42:1) pairs via [spawn_portal_pair](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:47:0-79:1). This replaces the hardcoded portal creation in [setup_maps](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:65:0-138:1) (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:91-129`) | 5h |
| L3.7 | Implement `#[derive(LdtkEntity)]` for `QueenSpawnMarker`: stores position. [spawn_queen](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:168:0-198:1) reads these markers instead of hardcoding [nest_grid_to_world(cx, 16)](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:29:0-36:1) (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:169-199`) | 3h |
| L3.8 | Refactor [setup_maps](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:65:0-138:1) to be event-driven: spawn map entities with [MapMarker](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:18:0-18:21) + `MapKind` when `LevelEvent::Spawned` fires, then [MapRegistry](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/active_map.rs:28:0-37:1) is built after all levels report spawned. Use a state machine or run condition | 5h |
| L3.9 | Remove [spawn_food_sources](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:86:0-141:1) from [terrain.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:0:0-0:0) (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:87-142`) — initial food now comes from LDtk entities. Keep [random_food_drops](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:143:0-195:1) for runtime spawning | 2h |
| L3.10 | Remove nest entrance marker sprites from [setup_terrain](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:30:0-84:1) (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:39-61`) — now an LDtk entity | 1h |
| L3.11 | Handle [MapId](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:5:0-5:29) assignment for LDtk-spawned entities: when a level spawns, tag all child entities with [MapId(level_map_entity)](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:5:0-5:29) so visibility toggling works | 3h |
| L3.12 | Test: portal wiring connects surface ↔ player nest and surface ↔ red nest correctly | 2h |
| L3.13 | Test: food sources from LDtk have correct [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) component values | 2h |
| L3.14 | Build check + full runtime validation — portals work, food pickups work, queen spawns correctly | 2h |

### Demo
> All map content now comes from the LDtk project file. Open LDtk editor — drag food sources to new positions, save, hot-reload. Food appears in new locations. Move the queen spawn marker deeper in the nest — queen now starts there. Add a third nest level ("SpecialZone") with its own portal — it appears in the Tab cycle. All portal transitions, food foraging, nest AI, and combat work as before.

### Acceptance Criteria
- [x] Food sources load from LDtk entity fields (amount, size)
- [x] Portal pairs auto-wire across levels by `portal_id`
- [x] Queen spawn position comes from LDtk entity
- [x] Nest entrance marker comes from LDtk entity
- [ ] MapRegistry built dynamically from spawned levels (still built in PreStartup; LDtk entities populate it)
- [x] MapId correctly assigned to all LDtk-spawned entities
- [ ] Adding a new level in LDtk automatically appears in the map cycle
- [x] No hardcoded positions remain in Rust code for map content
- [x] random_food_drops still works for dynamic runtime food
- [x] All portal transitions function correctly
- [x] Tests pass

**Files touched:**
- `src/plugins/ldtk_maps.rs` — entity bundles (`LdtkEntity` derives), `wire_portals`, [MapRegistry](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/active_map.rs:28:0-37:1) builder, [MapId](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:5:0-5:29) tagger
- [src/plugins/nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0) — refactor [setup_maps](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:65:0-138:1) to event-driven, refactor [spawn_queen](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:168:0-198:1) to use `QueenSpawnMarker`
- [src/plugins/terrain.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:0:0-0:0) — remove [spawn_food_sources](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:86:0-141:1), remove nest entrance sprites
- [src/components/map.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:0:0-0:0) — possibly add `PortalPointMarker` component
- `assets/maps/colony.ldtk` — add entities to all levels

---

## Sprint L4: Tilesets, Multiple Layouts & Campaign Prep

### Goal
Replace placeholder solid-color tilesets with proper pixel art. Create multiple nest layout variants for campaign use. Add LDtk Auto-Rules for terrain variation. Establish the asset pipeline for Sprint 21 (Campaign Mode) where each yard patch is an LDtk level.

### Tasks

| # | Task | Est |
|---|---|---|
| L4.1 | Create proper surface tileset: grass (3 variants), dirt, sand, concrete, water puddle, nest mound, stones/pebbles. 16×16 pixel art. At least 16 tiles | 6h |
| L4.2 | Create proper nest tileset: soil (light/dark), soft soil, clay, rock (3 variants), tunnel (4 directional joins), chamber floors (queen=purple tint, brood=warm, food=gold, midden=grey). At least 20 tiles | 6h |
| L4.3 | Set up LDtk Auto-Rules for surface: grass variation (random 3-tile grass), dirt paths, concrete edges, biome transitions | 4h |
| L4.4 | Set up LDtk Auto-Rules for nest: tunnel connections (auto-pick correct tunnel join tile based on neighbors), chamber borders, rock clusters | 4h |
| L4.5 | Create 3 nest layout variants: "Shallow" (wide, near surface), "Deep" (vertical, deep queen chamber), "Complex" (branching tunnels, multiple exits). Each as a separate LDtk level | 5h |
| L4.6 | Create 3 surface terrain variants for campaign patches: "Garden" (lots of food, few hazards), "Concrete" (sparse food, fast ants), "Wooded" (dense obstacles, predator spawns) | 5h |
| L4.7 | Add LDtk Enum definitions mirroring `BiomeType` (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:4-9`). Tag surface tiles with biome enum for gameplay effects | 3h |
| L4.8 | Implement `LdtkIntCell` for `BiomeTile` bundle: surface IntGrid layer with biome values, used by environment systems for terrain-dependent behavior (e.g., sand = faster evaporation) | 3h |
| L4.9 | Add level metadata custom fields in LDtk: `difficulty: Int`, `biome: Enum`, `nest_variant: String`. Read these in `LdtkMapsPlugin` to configure per-level gameplay parameters | 3h |
| L4.10 | Create `assets/maps/campaign.ldtk` — separate LDtk project for campaign with 16+ patch levels. Or extend `colony.ldtk` with a world layout using LDtk's multi-world feature | 4h |
| L4.11 | Document LDtk editing workflow: how to add a new level, required layers, entity placement rules, tileset conventions. Add to `docs/LDTK_MAP_EDITING.md` | 3h |
| L4.12 | Performance benchmark: surface with full tileset vs. old sprite approach. Measure FPS with 5K ants + pheromone overlay | 2h |
| L4.13 | Build check + runtime validation with new tilesets | 2h |

### Demo
> Surface now has proper pixel-art grass with natural variation — LDtk Auto-Rules create organic-looking terrain. Underground nests have directional tunnel tiles and tinted chamber floors. Open LDtk editor to show 3 nest variants: Shallow (quick to traverse), Deep (defensible), Complex (maze-like). Switch to campaign project — 16 yard patches visible in LDtk's world view, each with different biomes and food layouts. All ready for Sprint 21 campaign integration.

### Acceptance Criteria
- [ ] Surface renders with varied pixel-art tileset (not solid colors)
- [ ] Nest renders with directional tunnel joins and tinted chambers
- [ ] LDtk Auto-Rules produce natural-looking terrain variation
- [ ] 3 nest layout variants exist and are loadable
- [ ] 3 surface terrain variants exist for campaign use
- [ ] Biome enum tags affect gameplay (e.g., terrain speed modifiers)
- [ ] Level metadata fields readable from Rust
- [ ] Campaign LDtk project with 16+ levels exists
- [ ] `LDTK_MAP_EDITING.md` documents the workflow
- [ ] Performance is equal or better than old sprite approach

**Files touched:**
- `assets/tilesets/terrain.png` — proper pixel art
- `assets/tilesets/nest.png` — proper pixel art
- `assets/maps/colony.ldtk` — auto-rules, nest variants
- `assets/maps/campaign.ldtk` (new) — campaign patch levels
- `src/plugins/ldtk_maps.rs` — biome tile bundle, level metadata reading
- [src/components/terrain.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:0:0-0:0) — `BiomeTile` bundle
- `docs/LDTK_MAP_EDITING.md` (new)

---

## Effort Summary

| Sprint | Focus | Est Total |
|---|---|---|
| **L1** | Foundation + Surface | ~27h |
| **L2** | Nest IntGrid Pipeline | ~42h |
| **L3** | Entities + Portals | ~37h |
| **L4** | Tilesets + Campaign Prep | ~50h |
| **Total** | | **~156h** |

## Integration with Existing Roadmap

```
Sprints 12-18 (ant unification) ──────────────────────────────────┐
                                                                   │
Sprint L1 ──► Sprint L2 ──► Sprint L3 ──► Sprint L4 ─────────────┤
(surface)     (nest grids)   (entities)    (tilesets +             │
                                            campaign maps)         │
                                                 │                 │
Sprint 19 ✓ (environment) ──────────────────────┤                 │
                                                 ▼                 │
                                          Sprint 20 ◄─────────────┘
                                        (quick game)
                                              │
                                              ▼
                                        Sprint 21 (campaign) ◄── L4 campaign maps
                                              │
                                              ▼
                                        Sprint 22 (polish)
```

**Key notes:**
- **L1–L2 can run in parallel** with Sprints 12–18 (they touch different files)
- **L3 benefits from Sprint 14** being done (portal wiring is cleaner with job-driven transitions)
- **L4 directly feeds Sprint 21** (campaign levels are the maps campaign mode loads)
- **Sprint 20 (Quick Game)** should use LDtk maps if L1–L3 are complete by then
- [NestGrid::default()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/simulation.rs:51:4-57:5) should be **kept for unit tests** even after LDtk loading replaces it at runtime

If you'd like, switch to Code mode and I can write this to `docs/LDTK_INTEGRATION_PLAN.md`.