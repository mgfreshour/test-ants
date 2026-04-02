
Let me re-read the key data model files to make sure I capture every detail for the LDtk project design.

Now let me also check the current nest grid defaults to get exact dimensions and the combat/environment plugin for surface-relevant spawn positions.





Here's a complete LDtk project setup plan tailored to your codebase.

---

# LDtk Project Setup Guide for Colony

## 1. Project Settings

| Setting | Value | Reason |
|---|---|---|
| **Grid size** | 16 px | Matches `SimConfig.tile_size` (16.0) and `NEST_CELL_SIZE` (16.0) |
| **Project layout** | `GridVania` | Levels placed on a grid — gives free neighbor detection for future campaign. Alternatively `Free` if you want arbitrary level placement |
| **External levels** | Off (internal) | Keep it simple; one `.ldtk` file. Turn on later if file gets large for campaign |
| **Image export** | Off | Not needed — `bevy_ecs_ldtk` loads the raw data |
| **Default pivot** | Center | Matches Bevy's sprite pivot |
| **Background color** | `#1a1a1a` | Dark, won't bleed through |

**File location:** `assets/maps/colony.ldtk`

---

## 2. Enums (Define First)

LDtk lets you define project-wide enums that can tag tiles and entity fields. Define these before creating layers.

### `CellType`
Maps 1:1 to your Rust `CellType` enum (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:6-13`).

| Value | Color swatch | Notes |
|---|---|---|
| `Soil` | `#735230` | Default diggable |
| `SoftSoil` | `#805C33` | Fast dig (1s) |
| `Clay` | `#8C6640` | Slow dig (6s) |
| `Rock` | `#666666` | Impassable |
| `Tunnel` | `#595959` | Passable corridor |
| `ChamberQueen` | `#401F2E` | Queen chamber |
| `ChamberBrood` | `#38261A` | Brood chamber |
| `ChamberFood` | `#332E14` | Food storage |
| `ChamberMidden` | `#2E291F` | Waste disposal |

> In LDtk, enums are flat (no nesting), so `Chamber(ChamberKind)` becomes 4 separate enum values. Your Rust loader maps `ChamberQueen` → `CellType::Chamber(ChamberKind::Queen)`, etc.

### `BiomeType`
Maps to `@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:4-9`.

| Value | Color swatch |
|---|---|
| `Grass` | `#3A7326` |
| `Dirt` | `#6B4423` |
| `Sand` | `#C2B280` |
| `Concrete` | `#999999` |

### `MapKind`
Used as a level-field enum (`@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:9-14`).

| Value | Notes |
|---|---|
| `Surface` | Overworld |
| `Nest` | Underground — paired with `colony_id` field |
| `SpecialZone` | Future use |

### `HazardKind` (optional, future)
| Value |
|---|
| `SpiderSpawn` |
| `AntlionSpawn` |
| `PesticideZone` |

---

## 3. Tilesets

You need **two** tilesets. Start with placeholder solid-color tiles matching your current [color()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:268:4-277:5) impls, upgrade to pixel art in Sprint L4.

### `terrain.png` — Surface tiles
- **Tile size:** 16×16
- **Layout:** 8 columns × 2 rows (16 tiles minimum)

| Index | Tile | Color (current code match) |
|---|---|---|
| 0 | Grass dark | `srgb(0.22, 0.45, 0.15)` |
| 1 | Grass light | `srgb(0.28, 0.52, 0.18)` |
| 2 | Dirt | `srgb(0.42, 0.27, 0.14)` |
| 3 | Sand | `srgb(0.76, 0.70, 0.50)` |
| 4 | Concrete | `srgb(0.60, 0.60, 0.60)` |
| 5 | Nest mound (outer) | `srgb(0.35, 0.25, 0.15)` |
| 6 | Nest hole (inner) | `srgb(0.08, 0.05, 0.02)` |
| 7 | Water/puddle | `srgb(0.25, 0.40, 0.65)` |
| 8–11 | Grass variants (for auto-rules) | slight hue shifts of 0/1 |
| 12–15 | Reserved for edge/transition tiles | |

### `nest.png` — Underground tiles
- **Tile size:** 16×16
- **Layout:** 8 columns × 2 rows

| Index | Tile | Color (matches [CellType::color()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:268:4-277:5)) |
|---|---|---|
| 0 | Soil | `srgb(0.45, 0.32, 0.18)` |
| 1 | SoftSoil | `srgb(0.50, 0.36, 0.20)` |
| 2 | Clay | `srgb(0.55, 0.40, 0.25)` |
| 3 | Rock | `srgb(0.40, 0.40, 0.40)` |
| 4 | Tunnel | `srgb(0.35, 0.35, 0.35)` |
| 5 | Chamber Queen | `srgb(0.25, 0.12, 0.18)` |
| 6 | Chamber Brood | `srgb(0.22, 0.15, 0.10)` |
| 7 | Chamber Food | `srgb(0.20, 0.18, 0.08)` |
| 8 | Chamber Midden | `srgb(0.18, 0.16, 0.12)` |
| 9–12 | Tunnel directional joins (future) | |
| 13–15 | Rock variants | |

> **Critical:** The tileset index order directly becomes the `TileTextureIndex` you'll set when ants excavate cells at runtime. Keep a constant mapping in Rust:
> ```rust
> fn cell_type_to_tile_index(cell: CellType) -> u32 {
>     match cell {
>         CellType::Soil => 0,
>         CellType::SoftSoil => 1,
>         // ...
>     }
> }
> ```

---

## 4. Levels

### Level: `Surface`
- **Size:** 128 × 128 tiles (= 2048 × 2048 px, matches [SimConfig](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/simulation.rs:61:0-70:1) world dimensions)
- **Level custom fields:**
  - `map_kind`: `MapKind::Surface`
  - `difficulty`: `Int` = 1
  - `biome`: `BiomeType::Grass`

**Layers** (ordered back-to-front in LDtk, which means listed top-to-bottom in the editor):

| # | Layer name | Type | Tileset | Purpose |
|---|---|---|---|---|
| 1 | `Entities` | Entity | — | FoodSource, PortalPoint, SpiderSpawn, AntlionSpawn, NestEntrance |
| 2 | `Biomes` | IntGrid | — | Biome regions (1=Grass, 2=Dirt, 3=Sand, 4=Concrete). No tileset needed — gameplay data only |
| 3 | [Terrain](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:11:0-13:1) | Tiles (or AutoLayer) | `terrain.png` | Visual tiles. Use Auto-Rules tied to `Biomes` layer: Grass biome → random grass dark/light |

> **Auto-Rule tip:** Create a rule group "Grass fill" on the [Terrain](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:11:0-13:1) layer that uses `Biomes` as its IntGrid source. Rule: "if Biomes cell = 1 → randomly pick tile 0 or 1." This recreates your current checkerboard pattern but with more natural randomness.

### Level: `PlayerNest`
- **Size:** 60 × 40 tiles (= 960 × 640 px, matches `NEST_WIDTH` × `NEST_HEIGHT`)
- **Level custom fields:**
  - `map_kind`: `MapKind::Nest`
  - `colony_id`: `Int` = 0
  - `nest_variant`: `String` = "default"

**Layers:**

| # | Layer name | Type | Tileset | Purpose |
|---|---|---|---|---|
| 1 | `Entities` | Entity | — | QueenSpawn, PortalPoint |
| 2 | `Cells` | IntGrid | `nest.png` | The actual cell data. IntGrid values map to `CellType` (see §5 below) |

Paint the `Cells` layer to match the current [NestGrid::default()](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:16:4-88:5) layout in `@/Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:16-89`:

```
Layout reference (col 30 = center):
Rows 0-1:  All SoftSoil (value 2)
Row 0-6, col 30: Tunnel (value 5) — entrance shaft
Rows 5-7, cols 27-30: ChamberFood (value 8) — food storage
Rows 8-11, cols 32-36: ChamberBrood (value 7) — brood chamber
Rows 5-8, cols 30-31: Tunnel (value 5) — connecting tunnel
Rows 15-17, cols 28-32: ChamberQueen (value 6) — queen chamber
Rows 9-15, col 30: Tunnel (value 5) — shaft to queen
Rows 20-22, cols 36-39: ChamberMidden (value 9) — midden
Rows 17-20, col 32: Tunnel (value 5) — tunnel to midden
Row 20, cols 33-35: Tunnel (value 5) — horizontal to midden
Bottom 2 rows: Rock (value 4)
Everything else: Soil (value 1)
```

### Level: `RedNest`
- **Size:** 60 × 40 tiles
- **Level custom fields:**
  - `map_kind`: `MapKind::Nest`
  - `colony_id`: `Int` = 1

Same layer structure as `PlayerNest`. Can be identical layout or a mirrored/different design for variety.

---

## 5. IntGrid Value Mapping

This is the **single most important mapping** — it bridges LDtk data → your Rust `CellType`.

### `Cells` layer (nest levels)

| IntGrid value | `CellType` | Tileset tile index | Color preview |
|---|---|---|---|
| **1** | `Soil` | 0 | Brown |
| **2** | `SoftSoil` | 1 | Light brown |
| **3** | `Clay` | 2 | Tan |
| **4** | `Rock` | 3 | Grey |
| **5** | `Tunnel` | 4 | Dark grey |
| **6** | `Chamber(Queen)` | 5 | Dark purple-brown |
| **7** | `Chamber(Brood)` | 6 | Warm brown |
| **8** | `Chamber(FoodStorage)` | 7 | Gold-brown |
| **9** | `Chamber(Midden)` | 8 | Grey-brown |

**In LDtk:** Set the IntGrid layer to use `nest.png` as its tileset, with each value assigned its matching tile. This gives you visual feedback while painting.

**In Rust:**
```rust
fn cell_type_from_int_grid(value: i32) -> CellType {
    match value {
        1 => CellType::Soil,
        2 => CellType::SoftSoil,
        3 => CellType::Clay,
        4 => CellType::Rock,
        5 => CellType::Tunnel,
        6 => CellType::Chamber(ChamberKind::Queen),
        7 => CellType::Chamber(ChamberKind::Brood),
        8 => CellType::Chamber(ChamberKind::FoodStorage),
        9 => CellType::Chamber(ChamberKind::Midden),
        _ => CellType::Soil, // fallback
    }
}
```

### `Biomes` layer (surface level)

| IntGrid value | `BiomeType` |
|---|---|
| **1** | `Grass` |
| **2** | `Dirt` |
| **3** | `Sand` |
| **4** | `Concrete` |

---

## 6. Entity Definitions

Define these in LDtk's **Entity Defs** panel. Each becomes a placeable object in Entity layers.

### [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1)
| Field | Type | Default | Notes |
|---|---|---|---|
| `amount` | Float | 15.0 | Maps to `FoodSource.remaining` and `FoodSource.max` |
| `max_amount` | Float | 15.0 | Separate max if you want partially-depleted food |
| `size` | Float | 12.0 | Sprite size |

- **Editor visual:** Small green circle, size scaled by `amount`
- **Placement:** Surface `Entities` layer only
- **Limit:** No per-level limit (place ~20 for default Quick Game)

### `PortalPoint`
| Field | Type | Default | Notes |
|---|---|---|---|
| `portal_id` | String | `""` | Matching ID pairs portals across levels (e.g., `"player_nest_entrance"`) |
| `colony_id` | Int | -1 | -1 = open to all, 0 = player, 1 = red |

- **Editor visual:** Blue diamond
- **Placement:** One per portal mouth per level. The Rust `wire_portals` system matches pairs by `portal_id`
- **Example:** Surface has `PortalPoint(portal_id="p0", colony_id=0)` at (512, 512). PlayerNest has `PortalPoint(portal_id="p0", colony_id=0)` at (480, 8). System pairs them.

### `QueenSpawn`
| Field | Type | Default | Notes |
|---|---|---|---|
| (none) | — | — | Position is all that matters |

- **Editor visual:** Gold crown icon
- **Placement:** One per nest level, in the queen chamber
- **Current position reference:** [nest_grid_to_world(30, 16)](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:29:0-36:1) = center of queen chamber

### `NestEntrance`
| Field | Type | Default | Notes |
|---|---|---|---|
| `colony_id` | Int | 0 | Which colony this entrance belongs to |

- **Editor visual:** Brown circle with down-arrow
- **Placement:** Surface level only. Replaces the mound/hole sprites from [setup_terrain](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/terrain.rs:30:0-84:1)
- **Current positions:** Player nest at `(512, 512)` (= `world_width * 0.25`), Red nest at `(1536, 1536)`

### `SpiderSpawn` (optional — for Sprint L4/campaign)
| Field | Type | Default | Notes |
|---|---|---|---|
| `hp` | Float | 50.0 | |
| `patrol_radius` | Float | 100.0 | |

### `AntlionSpawn` (optional — for Sprint L4/campaign)
| Field | Type | Default | Notes |
|---|---|---|---|
| `hp` | Float | 40.0 | |
| `pit_radius` | Float | 40.0 | |

---

## 7. Directory Structure

```
assets/
├── maps/
│   └── colony.ldtk          ← main project file
├── tilesets/
│   ├── terrain.png           ← surface tileset (16×16 tiles)
│   └── nest.png              ← underground tileset (16×16 tiles)
```

In `colony.ldtk`, tileset paths should be relative: `../../tilesets/terrain.png` (or whatever LDtk generates — it uses relative paths from the `.ldtk` file).

Add to `.gitignore`:
```
assets/maps/*.ldtk.bak
```

---

## 8. Step-by-Step Setup Procedure

1. **Install LDtk** — download from [ldtk.io](https://ldtk.io/) (free, v1.5+)
2. **Create project** — File → New, save as `assets/maps/colony.ldtk`, set grid to 16px, layout to `GridVania`
3. **Create tilesets** — Use any pixel editor (Aseprite, Piskel, even GIMP) to make `terrain.png` and `nest.png` as solid-color grids per §3. Import both into LDtk (Project Settings → Tilesets)
4. **Define enums** — Project Settings → Enums. Add `CellType`, `BiomeType`, `MapKind` per §2
5. **Define entities** — Project Settings → Entity Defs. Add [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1), `PortalPoint`, `QueenSpawn`, `NestEntrance` per §6
6. **Create Surface level** — Add level "Surface", 128×128 tiles. Add layers: [Terrain](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:11:0-13:1) (AutoLayer with `terrain.png`), `Biomes` (IntGrid), `Entities` (Entity)
7. **Paint surface** — Fill `Biomes` with Grass (1). Paint the [Terrain](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:11:0-13:1) auto-layer rules. Place ~20 [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) entities, 2 `NestEntrance`, 2 `PortalPoint` on surface
8. **Create PlayerNest level** — Add level "PlayerNest", 60×40 tiles. Add layers: `Cells` (IntGrid with `nest.png`), `Entities` (Entity)
9. **Paint nest** — Paint the `Cells` IntGrid following the layout reference in §4. Place `QueenSpawn` at grid (30, 16), `PortalPoint(portal_id="p0")` at grid (30, 0)
10. **Create RedNest level** — Duplicate PlayerNest, rename to "RedNest", change level field `colony_id` to 1. Adjust layout if desired. Place `PortalPoint(portal_id="p1")`
11. **Add level custom fields** — On each level, set `map_kind`, `colony_id`, etc. per §4
12. **Save and verify** — Open in a text editor to spot-check the JSON structure. LDtk files are human-readable JSON.

---

## 9. LDtk ↔ Rust Naming Convention

| LDtk identifier | Rust mapping | Where registered |
|---|---|---|
| Level field `map_kind` | `MapKind` component | `LdtkMapsPlugin` post-spawn system |
| Level field `colony_id` | `MapKind::Nest { colony_id }` | Same |
| IntGrid layer `Cells` | `NestGrid::from_ldtk()` | `register_ldtk_int_cell::<NestCellBundle>(1..=9)` |
| IntGrid layer `Biomes` | `Terrain { biome }` | `register_ldtk_int_cell::<BiomeBundle>(1..=4)` |
| Entity [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) | [FoodSource](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:16:0-19:1) component | `register_ldtk_entity::<FoodSourceBundle>("FoodSource")` |
| Entity `PortalPoint` | `PortalPointMarker` → wired to [MapPortal](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/map.rs:29:0-42:1) | `register_ldtk_entity::<PortalPointBundle>("PortalPoint")` |
| Entity `QueenSpawn` | `QueenSpawnMarker` → consumed by [spawn_queen](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:168:0-198:1) | `register_ldtk_entity::<QueenSpawnBundle>("QueenSpawn")` |

---

## 10. Key Design Decisions & Rationale

- **IntGrid for nest, not Tiles:** IntGrid carries semantic meaning (cell type). Your sim reads [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) for pathfinding/digging — you need the int values, not just visuals. Assign tiles to IntGrid values in LDtk so you get both.
- **Entities for portals, not IntGrid:** Portal positions are precise points, not grid-aligned cells. Entity layer gives sub-pixel positioning and custom fields.
- **Separate layers for Biomes vs. Terrain:** `Biomes` is gameplay data (affects evaporation, speed). [Terrain](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/terrain.rs:11:0-13:1) is visual tiles. Decoupling lets you change visuals without affecting gameplay.
- **`portal_id` string matching:** Portals need to pair across levels. A shared string ID is the simplest cross-level reference. Alternatives (level neighbor links, entity references) are more fragile in LDtk.
- **Keep [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) as runtime truth:** LDtk provides the initial layout. After loading, [NestGrid](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/resources/nest.rs:9:0-13:1) owns the mutable state. Visual tile updates go through `bevy_ecs_tilemap` tile mutation. LDtk data is never re-read after initial load (except hot-reload during dev).

If you want, switch to Code mode and I can create the placeholder tileset PNGs and the initial `colony.ldtk` file programmatically, or write the `LDTK_MAP_EDITING.md` doc to the `docs/` directory.