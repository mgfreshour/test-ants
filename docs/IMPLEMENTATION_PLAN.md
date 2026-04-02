# Colony: Implementation Plan

## Sprint-Based Roadmap — Each Sprint Ends with a Demo

---

## Overview

15 two-week sprints. Each sprint produces a runnable build with visible new functionality. The plan front-loads core simulation systems (pheromones, ant AI, rendering) so that every subsequent sprint layers on top of a working, watchable ant colony. Sprints 5–7 build the underground nest into a real simulated space across three incremental layers. Sprints 10–11 add a proper `bevy_egui` UI layer so players can manage their colony without memorizing keyboard shortcuts.

```
Sprint  1   ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Wandering Ants
Sprint  2   ████░░░░░░░░░░░░░░░░░░░░░░░░░░  Pheromone Trails
Sprint  3   ██████░░░░░░░░░░░░░░░░░░░░░░░░  Foraging Loop
Sprint  4   ████████░░░░░░░░░░░░░░░░░░░░░░  Colony & Nest
Sprint  5   ██████████░░░░░░░░░░░░░░░░░░░░  Nest Pheromones & Pathfinding
Sprint  6   ████████████░░░░░░░░░░░░░░░░░░  Nest Ant AI
Sprint  7   ██████████████░░░░░░░░░░░░░░░░  Stigmergic Digging & Collision
Sprint  8   ████████████████░░░░░░░░░░░░░░  Player Control
Sprint  9   ██████████████████░░░░░░░░░░░░  Combat & Enemies
Sprint 10   ████████████████████░░░░░░░░░░  Colony Management UI (bevy_egui)
Sprint 11   ██████████████████████░░░░░░░░  Player HUD & Action Bar
Sprint 12   ████████████████████████░░░░░░  Environment & Hazards
Sprint 13   ██████████████████████████░░░░  Quick Game Complete
Sprint 14   ████████████████████████████░░  Campaign Mode
Sprint 15   ██████████████████████████████  Polish & Sandbox
```

---

## Sprint 1: Wandering Ants (Weeks 1-2)

### Goal
A window with a green surface, a nest entrance, and 50 ants wandering around randomly. Camera pans and zooms. This proves the Bevy project scaffolding, sprite rendering, ECS entity spawning, and basic movement all work.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 1.1 | Project setup — `cargo init`, add Bevy dependency, basic `main.rs` with `DefaultPlugins` | — | 2h |
| 1.2 | Define core components: `Ant`, `Caste`, `AntState`, `Movement`, `Health`, `ColonyMember` | `components/ant.rs` | 3h |
| 1.3 | Create `SimulationPlugin` — fixed timestep, `SimClock` resource, sim speed control (pause/play) | `plugins/simulation.rs` | 3h |
| 1.4 | Create `TerrainPlugin` — generate a flat grass tilemap (128x128), render with Bevy `Sprite` or tilemap crate | `plugins/terrain.rs`, `components/terrain.rs` | 6h |
| 1.5 | Create `AntSpawnSystem` — spawn 50 worker ant entities at nest entrance with random headings | `plugins/ant_ai.rs` | 3h |
| 1.6 | Create `AntMovementSystem` — each tick, ant moves in current direction with random perturbation (no pheromones yet), bounce off world edges | `plugins/ant_ai.rs` | 4h |
| 1.7 | Sprite rendering — placeholder colored circles (black = worker, larger = soldier). Batch rendering. | `assets/sprites/` | 3h |
| 1.8 | Camera plugin — pan (WASD/drag), zoom (scroll wheel), clamp to world bounds | `plugins/camera.rs` | 4h |
| 1.9 | Basic HUD — ant count, sim speed, FPS counter | `ui/hud.rs` | 3h |
| 1.10 | Spatial hash grid resource — build + rebuild each frame for neighbor queries | `resources/spatial_grid.rs` | 4h |

### Demo
> Open the app. 50 black dots wander around a green field. Pan and zoom the camera. Press Space to pause/unpause. FPS and ant count shown in corner.

### Acceptance Criteria
- [x] `cargo run` opens a window with rendered terrain
- [x] 50 ant entities move each frame with bounded random walk
- [x] Camera pans and zooms smoothly
- [x] Sim pauses and resumes
- [x] Spatial hash grid populated (verified via debug log)

---

## Sprint 2: Pheromone Trails (Weeks 3-4)

### Goal
Ants leave visible pheromone trails as they move. Pheromones evaporate and diffuse over time. Toggle an overlay to see the pheromone heat map. This is the single most important system in the game — get it right early.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 2.1 | Define `PheromoneGrid` resource — 2D array of `[f32; 4]` (home, food, alarm, trail) per cell, matching terrain grid resolution | `components/pheromone.rs`, `resources/pheromone.rs` | 4h |
| 2.2 | `PheromoneDepositSystem` — ants deposit HOME pheromone at current cell each tick | `plugins/pheromone.rs` | 3h |
| 2.3 | `PheromoneEvaporationSystem` — each cell decays by `EVAP_RATE` per tick | `plugins/pheromone.rs` | 2h |
| 2.4 | `PheromoneDiffusionSystem` — each cell spreads to 8 neighbors at `DIFFUSE_RATE` | `plugins/pheromone.rs` | 3h |
| 2.5 | `PheromoneSenseSystem` — each ant samples 8 neighbor cells, stores as perception data on component | `plugins/pheromone.rs` | 4h |
| 2.6 | Update `AntMovementSystem` — movement now weighted by pheromone gradient (still random, but biased toward pheromone) | `plugins/ant_ai.rs` | 4h |
| 2.7 | Pheromone overlay renderer — semi-transparent color layer on terrain. Blue=home, green=food, red=alarm. Toggle with `H` key. | `ui/overlays.rs` | 6h |
| 2.8 | Expose tuning params — `EVAP_RATE`, `DIFFUSE_RATE`, `DEPOSIT_AMOUNT` as `Resource` so they can be tweaked at runtime via debug UI | `resources/simulation.rs` | 2h |
| 2.9 | Debug panel — small egui window showing pheromone values at cursor position, editable parameters | `ui/debug.rs` | 4h |

### Demo
> Ants wander from the nest entrance. Toggle the pheromone overlay with H — blue trails radiate outward from the nest. Trails fade over time. Adjust evaporation rate in the debug panel and watch trails shrink faster or persist longer.

### Acceptance Criteria
- [x] Pheromone overlay visually shows trails matching ant paths
- [x] Trails fade over ~50 seconds with default evaporation
- [x] Diffusion creates smooth gradients (no sharp cell boundaries)
- [x] Debug panel reads/writes pheromone parameters live
- [x] Performance: 200 ants + pheromone grid at 60fps

---

## Sprint 3: Foraging Loop (Weeks 5-6)

### Goal
Place food sources on the surface. Ants discover food, pick it up, follow home pheromone back to the nest, and deposit it. The full forage-return loop with dual pheromone trails is working. Watching 100 ants form an efficient trail to a food pile is the first "wow" moment.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 3.1 | Define `FoodSource` component — position, remaining units, type (crumb/insect/fruit) | `components/terrain.rs` | 2h |
| 3.2 | `FoodSpawnSystem` — scatter 5-10 food sources randomly on surface at game start | `plugins/terrain.rs` | 2h |
| 3.3 | `FoodDetectionSystem` — foraging ants check current + adjacent cells for food | `plugins/ant_ai.rs` | 3h |
| 3.4 | Define `CarriedItem` component — optional, attached to ant when carrying food | `components/ant.rs` | 1h |
| 3.5 | `ItemPickupSystem` — ant at food source picks up one unit, food source decrements | `plugins/ant_ai.rs` | 3h |
| 3.6 | Implement FORAGE state — ant leaves nest, deposits HOME pheromone, follows FOOD pheromone gradient or random walks, picks up food on contact | `plugins/ant_ai.rs` | 6h |
| 3.7 | Implement RETURN state — ant carrying food deposits FOOD pheromone, follows HOME pheromone gradient back to nest | `plugins/ant_ai.rs` | 5h |
| 3.8 | `NestEntranceSystem` — define nest entrance tile(s), detect ant arriving with food, remove `CarriedItem`, add to colony food counter | `plugins/colony.rs` | 3h |
| 3.9 | Colony `FoodStorage` resource — tracks total stored food, displayed in HUD | `resources/colony_config.rs`, `ui/hud.rs` | 2h |
| 3.10 | Food source rendering — sprites for crumbs (small dot), insects (larger), fruit (colored circle). Shrink as depleted. | `assets/sprites/` | 3h |
| 3.11 | Visual indicator on ants carrying food — small green dot above ant sprite | rendering | 2h |
| 3.12 | Forage timeout — if ant doesn't find food in N seconds, return to nest idle | `plugins/ant_ai.rs` | 1h |

### Demo
> 100 ants pour out of the nest. They scatter randomly at first. One finds a food pile — it picks up food and returns, leaving a green (food) pheromone trail. Other ants detect the trail and follow it. Within a minute, a clear ant highway forms between nest and food. Toggle overlay to see the dual blue/green trail. Food counter in HUD climbs. When the pile depletes, the trail fades and ants scatter to find new sources.

### Acceptance Criteria
- [x] Ants transition IDLE → FORAGE → RETURN → IDLE correctly
- [x] Dual pheromone trails (home + food) form visible highways
- [x] Food counter increments when ants return with food
- [x] Depleted food sources disappear
- [x] Ants carrying food are visually distinct
- [x] Trail naturally optimizes to shortest path over time

---

## Sprint 4: Colony & Nest (Weeks 7-8)

### Goal
Underground side-view nest with a queen laying eggs, brood development pipeline, and digging ants expanding tunnels. Switch between surface and underground views. The colony now feels alive and self-sustaining.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 4.1 | Define `NestGrid` resource — 2D array of `CellType` (Soil, SoftSoil, Clay, Rock, Tunnel, Chamber) for underground cross-section | `components/nest.rs` | 4h |
| 4.2 | Nest renderer — side-view rendering of the underground grid. Brown soil, dark tunnels, colored chambers. | `plugins/nest.rs`, rendering | 6h |
| 4.3 | View switching — Tab key toggles between surface (top-down) and underground (side-view). Smooth transition. Camera resets per view. | `plugins/camera.rs` | 4h |
| 4.4 | Queen entity — spawned in queen chamber, `Queen` marker component, `EggLaySystem` produces eggs at rate proportional to food | `components/ant.rs`, `plugins/colony.rs` | 4h |
| 4.5 | Brood entities — Egg, Larva, Pupa components with timer. `BroodDevelopmentSystem` advances lifecycle (egg→larva→pupa→adult). | `plugins/colony.rs` | 5h |
| 4.6 | `NursingSystem` — nurse-state ants in brood chamber feed larvae (consumes food storage), unfed larvae die | `plugins/colony.rs` | 4h |
| 4.7 | `AntSpawnFromBroodSystem` — when pupa matures, spawn new ant entity with caste determined by `CasteRatios` resource | `plugins/colony.rs` | 3h |
| 4.8 | `DiggingSystem` — dig-state ants move to dig zones, remove soil cells over time, expand tunnels | `plugins/nest.rs` | 5h |
| 4.9 | Colony management panel (basic) — sliders for behavior allocation (forage/nurse/dig/defend %) and caste ratios (worker/soldier/drone %) | `ui/colony_panel.rs` | 5h |
| 4.10 | Population tracking — `Population` resource updated each frame (workers, soldiers, drones, brood count). Shown in HUD. | `resources/colony_config.rs`, `ui/hud.rs` | 2h |
| 4.11 | Ant aging — each ant has `age` field, dies at `LIFESPAN`. Natural death creates replacement demand. | `plugins/ant_ai.rs` | 2h |
| 4.12 | Initial nest layout — pre-dig a starting nest with queen chamber, one brood chamber, one food storage, connecting tunnels | `plugins/nest.rs` | 2h |

### Demo
> Start game. Surface shows ants foraging. Press Tab — camera transitions underground to a cross-section view showing the queen in her chamber, brood in the nursery, nurses feeding larvae. Eggs hatch into larvae, larvae pupate, pupae emerge as new worker ants that climb to the surface and start foraging. Open colony panel — adjust caste slider to produce more soldiers. Adjust behavior slider to send more ants digging. Watch tunnels extend in real time. Population counter climbs.

### Acceptance Criteria
- [x] Surface ↔ underground view toggle works smoothly
- [x] Queen lays eggs that progress through full lifecycle
- [x] New ants spawn from mature pupae and join the workforce
- [x] Nurses feed larvae, unfed larvae die
- [x] Digging ants extend tunnels visibly in the nest view
- [x] Colony panel sliders affect ant role assignment and caste ratios
- [x] Colony is self-sustaining: food in → eggs → ants → more food

---

## Sprint 5: Nest Pheromones & Pathfinding (Weeks 9-10)

### Goal
The nest gets its own pheromone system and pathfinding infrastructure. Ants can navigate tunnels efficiently via JPS, and the nest has chemical "road-signs" marking chamber functions — just like real ants use (Heyman et al. 2017). Queen pheromone diffuses through tunnels. This sprint builds the sensing and movement foundation that nest AI (Sprint 6) and digging (Sprint 7) depend on.

*See `NEST_AI_AND_NAVIGATION.md` for the full design rationale, biological basis, and algorithm choices.*

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 5.1 | `NestPheromoneGrid` resource — per-cell struct with four layers: chamber identity labels `[f32; 5]`, queen signal `f32`, construction pheromone `f32`, brood need `f32`. Sized to match `NestGrid`. | `resources/nest_pheromone.rs` | 4h |
| 5.2 | Chamber identity label system — ants present in a chamber passively refresh its identity label each tick. Labels decay slowly when chambers are unoccupied, allowing repurposing. Initial labels seeded from `NestGrid` chamber types at startup. | `plugins/nest_pheromone.rs` | 4h |
| 5.3 | Queen signal diffusion — queen entity emits signal at her position each tick. Signal diffuses through passable cells only (not through soil walls), creating a gradient that follows tunnel connectivity. Configurable decay and diffuse rates. | `plugins/nest_pheromone.rs` | 4h |
| 5.4 | Brood need signal — unfed `Brood` entities in `Larva` stage emit a local "hungry" signal on the nest pheromone grid. Signal intensity proportional to time since last fed. Decays on feeding. | `plugins/nest_pheromone.rs` | 3h |
| 5.5 | Nest pheromone decay/diffusion tick — each frame: decay construction pheromone at `CONSTRUCTION_DECAY_RATE`, decay queen signal, diffuse queen signal to passable neighbors. Analogous to surface pheromone evaporate/diffuse systems. | `plugins/nest_pheromone.rs` | 3h |
| 5.6 | Nest pheromone overlay — toggle with a key when in underground view. Color-coded: blue = queen signal gradient, pink = brood need, orange = construction, chamber labels shown as subtle tinted backgrounds per cell. | `ui/nest_overlays.rs` | 5h |
| 5.7 | Add `grid_pathfinding` dependency to `Cargo.toml` | `Cargo.toml` | 1h |
| 5.8 | `NestPathfinding` resource — wraps `grid_pathfinding` JPS, converts `NestGrid` passability into pathfinding grid. Rebuilt when nest structure changes (new tunnel dug). | `resources/nest_pathfinding.rs` | 4h |
| 5.9 | `NestPath` component — stores waypoint list and current index. `nest_path_following` system moves ants along waypoints toward next grid cell. | `components/nest.rs`, `plugins/nest_navigation.rs` | 4h |
| 5.10 | Path cache resource — `HashMap<(GridPos, GridPos), Vec<GridPos>>` with generation counter. Invalidated when `NestGrid` changes. | `resources/nest_pathfinding.rs` | 3h |
| 5.11 | Grid collision clamping — `nest_grid_collision` system rejects ant movement into impassable cells (`Soil`, `Rock`, `Clay`). Clamps position to nearest passable cell. | `plugins/nest_navigation.rs` | 2h |
| 5.12 | Test harness — spawn 8 nest ants with hardcoded destinations (e.g., "go to brood chamber", "go to queen chamber"). Verify they pathfind correctly through tunnels and stop at destination. Visual path lines drawn for debugging. | `plugins/nest_navigation.rs` | 3h |

### Demo
> Switch to underground view. 8 nest ants navigate purposefully through tunnels — no random wandering. Each has a hardcoded destination and follows JPS-computed paths through the correct tunnel branches. Toggle the pheromone overlay: blue gradient radiates outward from the queen chamber through the tunnel network. Chamber cells are subtly tinted by their identity labels. Pink spots appear near unfed larvae. Path debug lines show each ant's computed route. Ants cannot walk through walls.

### Acceptance Criteria
- [x] Nest pheromone grid stores and updates four layers per cell
- [x] Queen signal diffuses through tunnels (visible on overlay)
- [x] Brood need signal appears near unfed larvae
- [x] Chamber identity labels are seeded from grid and refreshed by ant presence
- [x] A* pathfinding computes valid tunnel routes between any two passable cells
- [x] Ants follow computed paths smoothly (no teleporting, no wall clipping)
- [x] Path cache avoids redundant pathfinding queries
- [x] Pheromone overlay renders all layers with distinct colors
- [x] Performance: pheromone grid update + 8 pathfinding queries at 60fps

---

## Sprint 6: Nest Ant AI (Weeks 11-12)

### Goal
Nest ants get their own utility-based AI that reads pheromone inputs and dynamically assigns tasks: nursing, hauling food, attending the queen. Decorative `NestWorker` entities are replaced with real ant entities running the utility system. Ants enter and exit the nest, transitioning between the surface FSM and the nest utility AI. The nest feels like a functioning workplace.

*See `NEST_AI_AND_NAVIGATION.md` Sections 3 and 6 for scoring model and biological basis.*

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 6.1 | `Underground` marker component — tracks which ants are currently in the nest. Added when ant enters nest entrance, removed when ant exits to surface. | `components/ant.rs` | 2h |
| 6.2 | Replace decorative `NestWorker` entities with real `Ant` entities spawned underground at startup. Same components as surface ants (`Ant`, `Movement`, `Health`, `ColonyMember`) plus `Underground` marker. | `plugins/nest.rs` | 4h |
| 6.3 | Cross-view ant continuity — surface ants entering the nest entrance gain `Underground` marker and switch to nest AI. Nest ants with no tasks lose `Underground` and resume surface FSM. Track position in both coordinate systems. | `plugins/nest_ai.rs` | 5h |
| 6.4 | Define `NestTask` enum component — `FeedLarva`, `HaulFood`, `HaulWaste`, `AttendQueen`, `Idle`. Each variant tracks sub-step and target entity/cell. (Digging deferred to Sprint 7.) | `components/nest.rs` | 3h |
| 6.5 | Utility scoring system — `nest_utility_scoring` evaluates candidate actions for each `Underground` ant. Reads nest pheromone grid for inputs: brood need signal → FEED_LARVA score, queen signal → ATTEND_QUEEN score, food at entrance → HAUL_FOOD score. Product-of-considerations selection. | `plugins/nest_ai.rs` | 8h |
| 6.6 | Age-based affinity multipliers — young ants score FEED_LARVA higher, mid-age score hauling higher, oldest ants score low on all nest tasks (prefer to exit and forage). Modulates utility scoring. | `plugins/nest_ai.rs` | 3h |
| 6.7 | Task chain execution framework — `nest_task_advance` system drives sub-steps generically: request pathfind → follow path → perform action → advance step or re-evaluate. Shared by all task chains. | `plugins/nest_ai.rs` | 6h |
| 6.8 | Feed larva task chain — nurse senses brood-need signal, pathfinds to food storage (via chamber label), picks up food, pathfinds to brood chamber, finds nearest unfed larva, delivers food, `larva.fed = true`. | `plugins/nest_ai.rs` | 5h |
| 6.9 | Haul food task chain — hauler senses food at entrance, pathfinds to entrance, picks up food, senses food-storage label, pathfinds to storage chamber, drops food. Bridges surface foraging with underground economy. | `plugins/nest_ai.rs` | 4h |
| 6.10 | Attend queen task chain — ant follows queen pheromone gradient, pathfinds toward queen chamber, grooms/feeds queen (reduces queen hunger), remains until utility score shifts. | `plugins/nest_ai.rs` | 3h |
| 6.11 | Nest ant task labels — when in underground view, show letter above each ant: N=nursing, H=hauling, Q=queen-attending, I=idle. Reuses the `StateLabel` pattern from surface. | `ui/nest_debug.rs` | 3h |
| 6.12 | Nest ant population scaling — as colony grows, more ants are assigned underground via behavior sliders. Nurse/dig/defend slider percentages now also govern how many surface ants transition to nest duty. | `plugins/nest_ai.rs` | 3h |

### Demo
> Switch to underground view. 15–20 ants are working purposefully. A nurse senses a hungry larva (pink pheromone glow), walks to food storage, picks up food, navigates to the brood chamber, and feeds the larva — the pink signal fades. A hauler picks up food deposited at the entrance by a surface forager and carries it to storage. An ant near the queen grooms her. Task labels float above each ant. Adjust the colony panel nursing slider up — more ants transition underground and start nursing. Set it low — ants exit the nest and resume foraging on the surface. The colony panel now visibly controls the surface/nest workforce split.

### Acceptance Criteria
- [x] Utility AI dynamically assigns tasks based on pheromone inputs and colony needs
- [x] Nurses complete full feed cycle: storage → brood chamber → feed larva
- [x] Haulers move food from nest entrance to storage chamber
- [x] Queen attendants follow queen pheromone gradient to queen chamber
- [x] Age-based affinity creates visible generational task division
- [x] Surface ants entering nest switch to utility AI; nest ants exiting resume FSM
- [x] Task labels visible in underground view
- [x] Colony panel sliders affect nest workforce size
- [x] Performance: 20 nest ants with full utility AI + pathfinding at 60fps

---

## Sprint 7: Stigmergic Digging & Collision (Weeks 13-14)

### Goal
Ants autonomously dig and expand the nest using stigmergic construction pheromone — just like real ants (Khuong et al. 2016). Digging self-limits via crowding feedback. Collision detection prevents ants from stacking in narrow tunnels. Player can designate dig zones to guide expansion, but autonomous digging also occurs. The nest grows organically.

*See `NEST_AI_AND_NAVIGATION.md` Sections 2.4 and 6.3 for biological basis and pheromone dynamics.*

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 7.1 | Construction pheromone dynamics — digger ants deposit construction pheromone at active dig faces. Decays at configurable `CONSTRUCTION_DECAY_RATE`. Nearby ants with dig affinity sense this and score DIG_AT_FACE higher in utility AI. | `plugins/nest_pheromone.rs`, `plugins/nest_ai.rs` | 4h |
| 7.2 | Self-limiting feedback — effective construction pheromone deposit scales inversely with nearby ant density: `deposit *= 1.0 / (1.0 + nearby_count * 0.5)`. More ants crowding a face → less pheromone → fewer recruits. Mirrors real ant collision-rate feedback. | `plugins/nest_pheromone.rs` | 3h |
| 7.3 | Dig task chain — digger senses construction pheromone gradient (or player-designated zone), pathfinds to dig face, excavates soil cell (duration varies: SoftSoil 1s, Soil 3s, Clay 6s, Rock impassable), cell becomes `Tunnel`, deposits construction pheromone on adjacent soil. | `plugins/nest_ai.rs` | 6h |
| 7.4 | Soil hauling sub-chain — after excavating, digger picks up soil particle, pathfinds to midden (via midden road-sign), drops soil. Then re-evaluates: may return to same face if construction pheromone still strong. | `plugins/nest_ai.rs` | 4h |
| 7.5 | Nest grid mutation — when a cell is excavated, update `NestGrid`, re-render affected tile sprite, invalidate path cache generation counter, rebuild JPS pathfinding grid. | `plugins/nest.rs`, `resources/nest_pathfinding.rs` | 4h |
| 7.6 | Player dig zone designation — click cells in underground view to mark as dig targets. Marked cells get a utility scoring boost (0.3 base stigmergic → 1.0 with player designation). Visual indicator on marked cells. | `plugins/nest.rs`, `ui/` | 4h |
| 7.7 | Auto-expansion triggers — when brood chamber is >80% occupied, colony needs boost dig utility near the brood area. When food storage is full, boost dig near storage area. Creates organic expansion without explicit commands. | `plugins/nest_ai.rs` | 4h |
| 7.8 | Separation steering — `nest_separation_steering` system applies gentle push-apart force using `SpatialGrid` neighbor queries. Prevents ant stacking in tunnels and chambers. | `plugins/nest_navigation.rs` | 3h |
| 7.9 | Tunnel traffic priority — ants carrying items (food or soil) have movement priority in 1-wide tunnels. Empty ants yield (brief pause) to laden ants approaching head-on. | `plugins/nest_navigation.rs` | 3h |
| 7.10 | Construction pheromone humidity parameter — expose `CONSTRUCTION_DECAY_RATE` as a per-colony tunable. Higher decay (dry conditions) → ants spread out → larger chambers. Lower decay (humid) → ants cluster → smaller chambers, more pillars. | `resources/nest_pheromone.rs` | 2h |
| 7.11 | Nest expansion visual feedback — newly excavated cells flash briefly. Construction pheromone visible on overlay as orange hotspots at active dig faces. Soil particles visible on hauling ants. | rendering, `ui/nest_overlays.rs` | 3h |
| 7.12 | Stress test — spawn 50 nest ants with mixed tasks (nursing, hauling, digging). Verify no deadlocks in tunnels, no wall clipping, digging self-limits as space expands, nest grows organically over 5 minutes. | QA | 4h |

### Demo
> Switch to underground view. Several ants cluster at the edge of a tunnel, digging. Orange construction pheromone glows at the dig face on the overlay. As one ant excavates a soil cell, it flashes and becomes dark tunnel. The digger picks up soil debris, carries it through tunnels to the midden, drops it, then returns — drawn back by the construction pheromone. As more ants join the dig site, the pheromone deposit per ant decreases and recruitment slows — the excavation self-regulates. Click a soil cell to designate it as a dig target — ants redirect toward the player's chosen expansion. Meanwhile, ants carrying food yield right-of-way in a narrow tunnel to an empty ant. When the brood chamber fills up, ants spontaneously begin digging an adjacent expansion without player input. Over 5 minutes, the nest visibly grows from its starting layout into a larger network.

### Acceptance Criteria
- [x] Construction pheromone deposited at dig faces, visible on overlay
- [x] Digging self-limits as tunnel space expands (fewer ants reach face → less pheromone)
- [x] Diggers complete full cycle: excavate → haul soil to midden → return
- [x] Excavated cells update nest grid, re-render, and invalidate path cache
- [x] Player dig zone designation boosts utility scoring for targeted cells
- [x] Auto-expansion triggers when chambers are at capacity
- [x] Ants avoid stacking via separation steering
- [x] Laden ants have tunnel priority over empty ants
- [x] Humidity parameter visibly affects dig spread and chamber size
- [x] No deadlocks or wall-clipping with 50 concurrent nest ants
- [x] Performance: 50 nest ants + digging + collision + pheromones at 60fps

---

## Sprint 8: Player Control (Weeks 15-16)

### Goal
The player can directly control one ant (yellow highlight), pick up food, lay pheromone trails, recruit followers, and exchange to different ants. This is the core "SimAnt feel" — you are one ant in the colony.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 8.1 | `PlayerControlled` marker component — exactly one ant has this at any time. Yellow sprite tint. Camera follows this ant optionally (toggle with `F` key). | `components/ant.rs` | 3h |
| 8.2 | Player movement — WASD overrides AI movement for the controlled ant. Collision with terrain obstacles. | `plugins/player.rs` | 4h |
| 8.3 | Player item interaction — E to pick up food/pebble, Q to drop. Carry indicator visible. | `plugins/player.rs` | 3h |
| 8.4 | Player pheromone laying — hold Shift while moving to deposit trail pheromone (yellow). Other ants follow this trail. | `plugins/player.rs` | 3h |
| 8.5 | Player attack — Space key attacks adjacent enemy. Attack animation/flash. | `plugins/player.rs` | 3h |
| 8.6 | Recruit system — R key: nearby friendly ants within 5-tile radius enter FOLLOW state, trail behind player. T key: dismiss all followers back to IDLE. | `plugins/player.rs`, `plugins/ant_ai.rs` | 5h |
| 8.7 | Exchange ant — Tab now opens ant-type selector (worker/soldier). Nearest ant of selected type becomes the new player ant. Previous ant resumes AI. | `plugins/player.rs` | 4h |
| 8.8 | Regurgitate food — F key: share food with adjacent nestmate (transfers carried food to another ant's hunger). | `plugins/player.rs` | 2h |
| 8.9 | Player HUD additions — controlled ant's HP bar, hunger bar, carried item indicator, follower count. | `ui/hud.rs` | 3h |
| 8.10 | Camera follow mode — when enabled, camera smoothly tracks player ant. Player can still nudge pan. Disable to free-cam. | `plugins/camera.rs` | 3h |
| 8.11 | Visual feedback — ant highlight glow, pheromone deposit particles, recruit radius indicator on R press. | rendering | 4h |

### Demo
> Spawn in as the yellow ant at the nest entrance. Walk around with WASD. Find a food pile — press E to pick up food. Hold Shift and walk back to the nest, laying a yellow trail. AI ants start following your trail. Press R near a group — 5 ants start following you. Lead them to a new food source. Press Tab to jump into a soldier ant. Walk to the edge of the map. Press T to dismiss followers.

### Acceptance Criteria
- [x] Player ant is visually distinct (yellow) and controllable with WASD
- [x] Picking up and dropping items works
- [x] Shift+move deposits visible trail pheromone that AI ants follow
- [x] R recruits nearby ants, T dismisses them
- [x] X exchanges to a different ant
- [x] Camera follow mode tracks player ant smoothly
- [ ] Player HUD shows HP, hunger, carried item, follower count
- [x] Player attack (Space key) — combat.rs:298
- [x] Regurgitate food to adjacent nestmate (F key) + hunger system

---

## Sprint 9: Combat & Enemies (Weeks 17-18)

### Goal
A rival red colony exists on the map. Red and black ants fight on contact. Spiders and antlions are predators. The alarm pheromone cascade rallies defenders. This sprint introduces conflict and stakes.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 9.1 | Red colony — spawn a second colony entity with its own nest entrance, queen, starting ants (30). Red-tinted sprites. | `plugins/colony.rs` | 4h |
| 9.2 | Colony membership — `ColonyMember { colony_id }` component on every ant. Ants distinguish friend from foe by colony ID. | `components/ant.rs` | 2h |
| 9.3 | `CombatDetectionSystem` — when ants from different colonies occupy adjacent cells, trigger combat state | `plugins/combat.rs` | 4h |
| 9.4 | `CombatResolutionSystem` — per-tick damage calculation: base attack ± random ± group bonus ± terrain bonus. Apply damage to `Health`. | `plugins/combat.rs` | 5h |
| 9.5 | `DeathSystem` — ant at 0 HP despawns, drops carried item, emits `AntDiedEvent`. Optional: corpse sprite lingers briefly. | `plugins/combat.rs` | 3h |
| 9.6 | Alarm pheromone — fighting ants emit alarm pheromone. `AlarmResponseSystem` causes nearby DEFEND-role ants to switch to FIGHT and move toward source. | `plugins/pheromone.rs`, `plugins/ant_ai.rs` | 5h |
| 9.7 | Spider predator — entity with web zone. `SpiderAISystem`: wait at web, attack ants entering 3-tile radius, relocate after timeout. High HP, lethal to lone ants. | `plugins/predators.rs` | 5h |
| 9.8 | Antlion predator — pit trap entity at sandy areas. Ants entering pit slide to center, take damage. Antlion emerges to finish. | `plugins/predators.rs` | 4h |
| 9.9 | Red colony basic AI — same ant FSM but with a strategy layer that adjusts behavior sliders over time (start defensive, ramp to aggressive). | `plugins/ant_ai.rs` | 5h |
| 9.10 | Combat visual effects — hit flash, small damage numbers or shake, death particle burst. | rendering | 3h |
| 9.11 | Victory/defeat detection — `VictoryCheckSystem` monitors queen HP for both colonies. Queen death → colony collapse event. Game over screen. | `plugins/simulation.rs`, `ui/` | 3h |

### Demo
> Two colonies on opposite sides of the map. Black ants forage and expand. Red ants do the same. Eventually scouts meet in the middle — a fight breaks out. Alarm pheromone pulses red on the overlay. Defender ants rush to the fight. A small battle plays out with hit flashes and death particles. Meanwhile, a spider lurks near a food source — lone foragers get killed, but a recruited group of 6 can overwhelm it. Kill the red queen → victory screen. Lose yours → defeat.

### Acceptance Criteria
- [x] Red colony operates autonomously with its own nest, queen, ants
- [x] Ants from opposing colonies fight on contact
- [x] Damage, HP, death all work correctly
- [x] Alarm pheromone recruits defenders to combat zones
- [x] Spider kills lone ants, can be killed by groups
- [ ] Antlion traps work
- [x] Killing enemy queen triggers victory; losing yours triggers defeat
- [ ] Red colony AI has strategy layer (aggression curve, raid timing)
- [ ] Combat visual effects (hit flash, damage numbers)

---

## Sprint 10: Colony Management UI (Weeks 19-20)

### Goal
Replace the text-based colony panel and keyboard-only controls with a proper `bevy_egui` UI. Players get a collapsible colony management panel with real sliders for job distribution, caste birthrates, and aggression. The panel adapts to the current view (surface vs. underground) and exposes all colony tuning without memorizing key bindings. Keyboard shortcuts still work as hotkeys for power users.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 10.1 | Add `bevy_egui` dependency to `Cargo.toml`. Create `UiPlugin` that initializes egui context and consumes input when UI is focused (prevents WASD moving the ant while typing in a slider). | `Cargo.toml`, `plugins/egui_ui.rs` | 3h |
| 10.2 | Colony Management Panel — collapsible left-side panel with sections: **Job Distribution** (forage/nurse/dig/defend sliders, constrained to sum to 1.0), **Caste Birthrates** (worker/soldier/drone sliders, constrained to sum to 1.0), **Aggression** (patrol radius, defender response threshold). Reads/writes `BehaviorSliders`, `CasteRatios`, new `AggressionSettings` resource. | `plugins/egui_ui.rs`, `resources/colony.rs` | 8h |
| 10.3 | Colony Stats section in the panel — population breakdown (workers/soldiers/drones), brood counts (eggs/larvae/pupae), food stored, ants underground vs. surface. Read-only display from `ColonyStats`, `ColonyFood`, underground ant count. | `plugins/egui_ui.rs` | 3h |
| 10.4 | Sim Controls toolbar — top bar with play/pause button, speed selector (1x/2x/4x/8x), elapsed time display. Replaces Space/Period keyboard cycling (hotkeys still work). | `plugins/egui_ui.rs` | 3h |
| 10.5 | View Toggle — surface/underground button in toolbar with current view indicator. Replaces Tab-only switching (Tab still works as hotkey). | `plugins/egui_ui.rs` | 2h |
| 10.6 | Overlay Controls — checkboxes/dropdown for pheromone overlay mode (Off/All/Home/Food/Alarm/Trail on surface; Off/Queen/Brood/Construction on underground). Replaces H/N key cycling. | `plugins/egui_ui.rs` | 3h |
| 10.7 | Nest View Controls — when underground, show dig zone toggle (click-to-designate mode), pheromone overlay type selector, path debug toggle. Replaces N/P keys. | `plugins/egui_ui.rs` | 3h |
| 10.8 | `AggressionSettings` resource — patrol radius, alarm response threshold, defender ratio within defend-allocated ants. Wired into `ant_ai.rs` alarm response and `combat.rs` detection range. | `resources/colony.rs`, `plugins/ant_ai.rs`, `plugins/combat.rs` | 4h |
| 10.9 | Slider constraint system — job distribution sliders auto-normalize to sum to 1.0 (adjusting others proportionally when one moves). Same for caste birthrate sliders. | `plugins/egui_ui.rs` | 3h |
| 10.10 | Remove old text-based colony panel (`colony_panel.rs`) and keyboard-only slider controls (1/2/3/4 keys). Migrate all functionality to egui. Keep keyboard shortcuts as hotkeys wired through egui. | `ui/colony_panel.rs` (remove), `plugins/egui_ui.rs` | 2h |
| 10.11 | Tooltip system — hover any slider or stat for a brief explanation (e.g., "Forage: % of surface ants dedicated to finding food"). | `plugins/egui_ui.rs` | 2h |

### Demo
> Open the game. A collapsible panel on the left shows colony stats at a glance: 47 workers, 8 soldiers, 12 brood, 35 food stored. Below that, four job distribution sliders — drag "Nurse" up and "Forage" auto-adjusts down. A "Caste Birthrates" section lets you boost soldier production to 40%. An "Aggression" section widens the patrol radius. Top toolbar has play/pause, speed selector showing "2x", and a "Surface/Underground" toggle button. Switch to underground view — the panel updates to show nest-specific stats and a "Dig Zones" toggle. Hover over any slider for a tooltip explaining what it does. Press Tab — view switches (hotkey still works). The panel is unobtrusive and can be collapsed to just an icon.

### Acceptance Criteria
- [ ] `bevy_egui` integrated and rendering panels
- [ ] Job distribution sliders read/write `BehaviorSliders`, auto-normalize to 1.0
- [ ] Caste birthrate sliders read/write `CasteRatios`, auto-normalize to 1.0
- [ ] Aggression settings control patrol/alarm behavior
- [ ] Colony stats display matches actual simulation state
- [ ] Sim speed and pause controllable via UI buttons
- [ ] View toggle works from UI (Tab hotkey preserved)
- [ ] Overlay controls replace keyboard cycling
- [ ] Panel adapts content to current view (surface vs. underground)
- [ ] Tooltips on all interactive elements
- [ ] Old text-based colony panel removed
- [ ] UI does not consume game input when not focused (WASD still moves ant)

---

## Sprint 11: Player HUD & Action Bar (Weeks 21-22)

### Goal
The player-controlled ant gets a proper HUD with health bar, carried item display, and clickable action buttons. Modern players can play entirely with mouse + minimal keys. The action bar provides all player interactions (pick up, drop, recruit, dismiss, swap, lay trail) as buttons with hotkey labels. Context-sensitive: buttons gray out when unavailable. This replaces the bottom text line of keyboard hints.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 11.1 | Player Info HUD — bottom-center bar showing: ant sprite/caste icon, HP bar (green→red), hunger bar, carried item icon with amount, follower count badge. Updates live from `PlayerControlled` entity queries. | `plugins/egui_ui.rs` | 5h |
| 11.2 | Action Bar — row of icon buttons: Pick Up (E), Drop (Q), Lay Trail (Shift), Recruit (R), Dismiss (T), Swap Ant (X), Attack (Space). Each shows hotkey label. Clicking triggers the same action as the hotkey. | `plugins/egui_ui.rs`, `plugins/player.rs` | 6h |
| 11.3 | Context-sensitive button states — Pick Up grayed when no food nearby or already carrying. Drop grayed when not carrying. Recruit grayed when no ants nearby. Attack grayed when no enemies adjacent. Trail button toggles (highlighted when active). | `plugins/egui_ui.rs` | 4h |
| 11.4 | Action event system — UI buttons emit `PlayerAction` events that the player plugin consumes, unifying keyboard and mouse input paths. Refactor `player.rs` to read events instead of raw `ButtonInput<KeyCode>` for actions. | `plugins/player.rs`, `plugins/egui_ui.rs` | 5h |
| 11.5 | Minimap widget — small corner overlay (egui `Window`) showing world extent, ant density as colored dots (black=friendly, red=enemy), food sources as green dots, nest entrance marker. Click to pan camera. | `plugins/egui_ui.rs` | 5h |
| 11.6 | Notification toast system — brief egui toasts for game events: "Rain starting!", "Enemy spotted!", "Queen laid an egg", "Larva hatched". Auto-dismiss after 3 seconds. Queue up to 3 visible. | `plugins/egui_ui.rs` | 3h |
| 11.7 | Remove old text-based HUD (`ui/hud.rs` text blob). Replace with the egui-based HUD components. Keep FPS counter as a small egui label in the corner. | `ui/hud.rs` (simplify), `plugins/egui_ui.rs` | 3h |
| 11.8 | Keyboard shortcut reference — `?` key or button opens a small overlay listing all hotkeys grouped by category (movement, actions, camera, colony). Dismissable. | `plugins/egui_ui.rs` | 2h |
| 11.9 | Panel toggle hotkey — `` ` `` (backtick) toggles the colony management panel visibility. `Escape` closes any open panel/overlay. | `plugins/egui_ui.rs` | 1h |

### Demo
> Player ant has a sleek bottom bar: green HP bar, "Carrying: 5.0 food" with icon, "3 followers" badge. Below it, action buttons: [Pick Up (E)] is grayed (already carrying), [Drop (Q)] is lit, [Trail (Shift)] glows when held, [Recruit (R)] shows a radius preview on hover, [Swap (X)] [Attack (Space)]. Click "Drop" near the nest — food deposited, button grays out, "Pick Up" lights up. Corner minimap shows ant clusters and a red colony dot across the map. A toast slides in: "Enemy ants spotted near food source!" then fades. Press `?` — hotkey reference card appears. Press backtick — colony panel slides shut for an uncluttered view.

### Acceptance Criteria
- [ ] Player HP bar, hunger, carried item, follower count displayed visually
- [ ] All player actions available as clickable buttons with hotkey labels
- [ ] Buttons context-sensitive (grayed when unavailable)
- [ ] Mouse clicks on action buttons trigger same behavior as hotkeys
- [ ] Minimap shows world overview with clickable camera panning
- [ ] Notification toasts appear for key game events
- [ ] Old text HUD replaced with egui components
- [ ] `?` opens hotkey reference overlay
- [ ] Panel toggle with backtick, Escape closes overlays
- [ ] UI does not interfere with game input when panels not focused

---

## Sprint 12: Environment & Hazards (Weeks 23-24)

> *Formerly Sprint 10.*

### Goal
Rain washes away pheromone trails. Human footsteps crush ants. Lawnmowers sweep across the surface. Day/night cycle affects ant behavior. The world feels dynamic and dangerous.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 12.1 | Rain event system — periodic rain (random interval 2-5 min). During rain: pheromone evaporation rate 10x. Minimal visual indicator (tinted screen overlay). | `plugins/environment.rs` | 4h |
| 12.2 | Nest flooding — heavy rain causes water level to rise in lowest nest tunnels. Ants in flooded areas take damage. Player incentive to not dig too deep. | `plugins/environment.rs`, `plugins/nest.rs` | 4h |
| 12.3 | Human footstep event — random position, 3-tile radius area damage (999). Brief shadow warning before impact. | `plugins/environment.rs` | 3h |
| 12.4 | Lawnmower event — horizontal sweep from left edge to right at a random y-position. Kills everything in path. Brief warning indicator. | `plugins/environment.rs` | 3h |
| 12.5 | Pesticide spray — lingering poison zone (10x10 area). Ants entering take damage over time. Persists for 30 seconds. Tinted ground marker. | `plugins/environment.rs` | 3h |
| 12.6 | Day/night cycle — 3-minute cycle. Ants forage more during day, return to nest at night. Behavioral changes only (visual polish deferred to Sprint 15). | `plugins/environment.rs` | 3h |
| 12.7 | Event notification system — egui toasts for hazard events: "Rain starting!", "Watch out — footstep!", "Lawnmower approaching!" (reuses Sprint 11 toast system). | `plugins/egui_ui.rs` | 2h |

### Demo
> Colony is foraging happily. Rain starts — pheromone overlay fades rapidly, ants lose their trails and scatter. Rain stops, ants rebuild trails. A shadow appears — STOMP. 8 ants crushed. A lawnmower warning toast appears — it sweeps across, devastating a foraging line. Tinted ground marks a pesticide zone, ants walking through take damage. Night falls, ants return to nest. Day breaks, they emerge again.

### Acceptance Criteria
- [ ] Rain accelerates pheromone decay, ants lose trails
- [ ] Flooding damages ants in deep tunnels
- [ ] Footstep kills ants in area with warning
- [ ] Lawnmower sweeps and kills in path
- [ ] Pesticide creates lingering damage zone
- [ ] Day/night cycle affects ant behavior
- [ ] Hazard event notifications via egui toasts

---

## Sprint 13: Quick Game Complete (Weeks 25-26)

> *Formerly Sprint 11.*

### Goal
A fully playable Quick Game mode from main menu to victory/defeat screen. Balanced, fun, 10-15 minute play sessions. This is the first "real game" milestone.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 11.1 | Main menu — title screen with "Quick Game", "Campaign" (grayed out), "Sandbox" (grayed out) buttons. | `ui/main_menu.rs` | 4h |
| 11.2 | Game state machine — Loading → MainMenu → InGame → Paused → GameOver. Bevy `States` with enter/exit systems. | `plugins/simulation.rs` | 4h |
| 11.3 | Pause menu — ESC pauses sim, shows Resume / Restart / Quit to Menu. | `ui/pause_menu.rs` | 3h |
| 11.4 | Game over screen — Victory: "Colony Dominant!" with stats (ants produced, food gathered, time). Defeat: "Colony Lost" with cause. | `ui/gameover.rs` | 3h |
| 11.5 | Quick game setup — balanced starting conditions. Black and red colony start equidistant. 5 food sources. 2 spiders. 1 antlion. | `plugins/simulation.rs` | 3h |
| 11.6 | Balance tuning — playtest and adjust: ant stats, food spawn rates, red AI aggression curve, predator HP, event frequency. Target: competitive game winnable in 10-15 min. | all systems | 8h |
| 11.7 | Red AI improvements — smarter raid timing, retreat behavior, expansion priority. Should feel like a competent opponent. | `plugins/ant_ai.rs` | 6h |
| 11.8 | Minimap — corner overlay showing full patch, ant density (black/red dots), food sources, predator locations. | `ui/minimap.rs` | 5h |
| 11.9 | Tutorial hints — first-time popups: "Press WASD to move", "Hold Shift to lay trail", "Press R to recruit", etc. Dismissable, non-intrusive. | `ui/tutorial.rs` | 4h |
| 11.10 | End-to-end QA — test full game loop 10+ times. Fix crashes, softlocks, degenerate strategies. | QA | 6h |

### Demo
> Launch game. Title screen appears. Click "Quick Game". Player spawns as yellow ant. Tutorial hints guide first actions. Forage food, grow colony, clash with red ants, survive hazards. Red colony sends raids. Counter-attack with recruited soldiers. Kill the red queen — victory screen shows stats. Play again or quit to menu.

### Acceptance Criteria
- [ ] Full game loop: menu → play → win/lose → menu
- [x] Game is winnable and losable (victory/defeat detection exists)
- [ ] 10-15 minute play session feels complete
- [ ] No crashes or softlocks in 10 consecutive playthroughs
- [ ] Tutorial teaches core controls
- [ ] Minimap provides strategic awareness
- [ ] Red AI provides meaningful challenge

---

## Sprint 14: Campaign Mode (Weeks 27-28)

> *Formerly Sprint 12.*

### Goal
Multi-patch campaign where the player colonizes a 4x4 yard grid and a house. Mating flights establish satellite colonies. Difficulty escalates. The game has long-term progression and replayability.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 12.1 | Campaign map screen — 4x4 grid of yard patches + house. Player selects which patch to enter. Colonized patches shown in black, red in red, neutral in gray. | `ui/campaign_map.rs` | 5h |
| 12.2 | Patch generation — each patch has procedurally varied terrain, food sources, hazards based on difficulty tier. | `plugins/campaign.rs`, `plugins/terrain.rs` | 6h |
| 12.3 | Mating flight mechanic — when colony hits critical population, player can trigger mating flight. Choose target adjacent patch. New colony established with starter queen + 10 workers. | `plugins/campaign.rs` | 5h |
| 12.4 | Patch switching — player can switch active patch from campaign map. Each patch maintains its own simulation state (ants, pheromones, nest). | `plugins/campaign.rs` | 6h |
| 12.5 | Difficulty scaling — later patches have more red ants, more predators, more hazards. House patches have poison bait, traps, exterminator events. | `plugins/campaign.rs` | 4h |
| 12.6 | Red colony campaign AI — red colony also expands to adjacent patches. Race for territory. Red AI more aggressive in later patches. | `plugins/ant_ai.rs` | 5h |
| 12.7 | House interior gameplay — distinct indoor hazards (poison bait, traps, exterminator events), indoor food source types (sugar, crumbs, pet food). Visual tileset deferred to Sprint 15. | `plugins/campaign.rs` | 3h |
| 12.8 | Campaign victory/defeat — win at 70% house + red eliminated. Lose if all queens dead. Stats screen with campaign summary. | `plugins/campaign.rs`, `ui/gameover.rs` | 3h |
| 12.9 | Campaign save/load — serialize campaign state (patch statuses, colony states) to disk. Resume from main menu. | `plugins/campaign.rs` | 5h |

### Demo
> Start campaign. Map shows 16 yard patches. Enter the starting patch — play a mini Quick Game. Colony grows. Trigger mating flight — choose adjacent patch. Switch to new patch — a fledgling colony with 10 ants. Build it up. Meanwhile, red colony has colonized two patches on their side. Race to claim the house patches. Enter the house — indoor tileset, different food sources, new hazards. Win by dominating the house.

### Acceptance Criteria
- [ ] Campaign map screen works with 4x4 + house layout
- [ ] Mating flight establishes functional satellite colonies
- [ ] Switching patches preserves simulation state
- [ ] Difficulty clearly escalates across patches
- [ ] House interior plays differently from yard
- [ ] Campaign save/load round-trips correctly
- [ ] Campaign is winnable in ~60-90 minutes

---

## Sprint 15: Visual Polish, Audio & Sandbox (Weeks 29-30)

> *Formerly Sprint 13. Now also absorbs all visual/audio polish deferred from earlier sprints.*

### Goal
All visual and audio polish consolidated here. Sandbox mode with full environmental controls. Art/audio pass across the entire game. Performance optimization. This is the release-candidate sprint. The game should be fully playable and balanced before this sprint — this sprint makes it *look and sound* good.

### Tasks

| # | Task | Components / Systems | Est |
|---|---|---|---|
| 15.1 | Sandbox mode — spawn from menu. Full controls: spawn food, place walls, paint pheromones, spawn/kill ants, control either colony. | `plugins/sandbox.rs` | 6h |
| 15.2 | Sandbox parameter panel — expose all simulation constants (evap rate, diffuse rate, ant speed, lifespans, nest humidity, etc.) with live-edit egui sliders. | `plugins/egui_ui.rs` | 4h |
| 15.3 | Data overlays for sandbox — pheromone heat maps (surface + nest), ant density map, population over time graph, foraging efficiency metric. | `plugins/egui_ui.rs` | 5h |
| 15.4 | **Art polish** — replace placeholder sprites with proper pixel art. Ant walk animations (6-8 frames). Food sprites. Terrain tiles. UI skinning. | art, `assets/` | 8h |
| 15.5 | **Audio system** — integrate `bevy_audio`. Ambient outdoor soundscape, underground ambience. Combat clicks, food pickup chime, alarm tone. At least 8 sound effects. | audio, `assets/audio/` | 6h |
| 15.6 | **Weather visual effects** *(deferred from Sprint 12)* — rain particles, darkened sky, puddle sprites, footstep shadow, mower sprite animation. Layer on top of existing hazard mechanics. | rendering | 5h |
| 15.7 | **Combat visual effects** *(deferred from Sprint 9)* — hit flash, small damage numbers or shake, death particle burst. | rendering | 4h |
| 15.8 | **Player visual feedback** *(deferred from Sprint 8)* — ant highlight glow, pheromone deposit particles, recruit radius indicator on R press. | rendering | 4h |
| 15.9 | **Day/night visual cycle** *(deferred from Sprint 12)* — subtle lighting shift on terrain, darkened sky at night, dawn/dusk color gradients. | rendering | 3h |
| 15.10 | **Nest excavation feedback** *(deferred from Sprint 7)* — newly excavated cells flash briefly, soil particles visible on hauling ants. | rendering | 2h |
| 15.11 | **House interior tileset** *(deferred from Sprint 14)* — indoor tiles (kitchen tile, carpet, wood), indoor food sources (sugar bowl, crumbs, pet food). | `assets/`, `plugins/terrain.rs` | 5h |
| 15.12 | Performance optimization — profile with 10K ants. Optimize pheromone grids (SIMD or compute shader). Reduce AI update frequency. LOD for distant ants. Sprite batching audit. | all systems | 8h |
| 15.13 | Settings screen — resolution, fullscreen, volume sliders, control rebinding. | `plugins/egui_ui.rs` | 4h |
| 15.14 | Accessibility — colorblind mode for pheromone overlays, adjustable UI scale, key rebinding. | `plugins/egui_ui.rs` | 3h |
| 15.15 | Bug fix buffer — address bugs found in Sprint 13-14 playtesting. | all | 6h |
| 15.16 | Final QA — full pass on all three modes (Quick, Campaign, Sandbox). Performance benchmarks on min-spec hardware. | QA | 5h |

### Demo
> Full game showcase. Title screen with three playable modes. Quick Game is polished and balanced. Campaign takes the player from a single patch to house domination. Sandbox lets you create a 10,000-ant mega-colony and watch emergent behavior with live-tunable parameters and data overlays. Art is pixel-perfect, audio is immersive, and it runs at 60fps. Rain has particle effects and darkened sky. Combat has hit flashes and death particles. The player ant glows and shows a recruit radius.

### Acceptance Criteria
- [ ] Sandbox mode fully functional with all controls
- [ ] All three game modes accessible from main menu
- [ ] Proper pixel art sprites with animations
- [ ] Ambient + contextual audio throughout (8+ sound effects)
- [ ] Weather, combat, player, and nest visual effects polished
- [ ] House interior tileset in campaign
- [ ] 10,000 ants at 60fps on target hardware
- [ ] No known crash bugs
- [ ] Settings screen with volume, resolution, colorblind mode

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Pheromone grid performance bottleneck at scale | Medium | High | Profile early (Sprint 2). Fallback: reduce grid resolution, GPU compute shader for diffusion. |
| Bevy breaking changes between versions | Medium | Medium | Pin Bevy version in `Cargo.toml`. Only upgrade between sprints. |
| Ant AI feels too random / not intelligent | Medium | High | Tune pheromone parameters extensively in Sprint 3. Sandbox mode (Sprint 10) helps. |
| Red AI too easy or too hard | High | Medium | Dedicate balance time in Sprint 8. Expose difficulty slider. |
| Campaign state management complexity | Medium | Medium | Keep patches isolated (no cross-patch simulation). Serialize minimal state. |
| Scope creep | High | Medium | Each sprint has a locked scope. Nice-to-haves deferred to post-release. |
| Art/audio takes longer than estimated | Medium | Low | Placeholder art is fine for all sprints until Sprint 15. Game is fully playable without polish. |

---

## Dependency Chain

```
Sprint 1 ──► Sprint 2 ──► Sprint 3 ──► Sprint 4 ──┐
  (ants)     (pheromone)   (foraging)   (colony)    │
                                                     │
  ┌──────────────────────────────────────────────────┘
  │
  ├──► Sprint 5 ──► Sprint 6 ──► Sprint 7 ──┐
  │    (nest         (nest ant    (stigmergic │
  │     pheromones    AI)          digging &  │
  │     & pathfind)                collision) │
  │                                           │
  ├───────────────────────────────────────────┘
  │
  ├──► Sprint 8 ──► Sprint 9 ──► Sprint 10 ──► Sprint 11
  │    (player)     (combat)     (colony UI     (player HUD
  │                               bevy_egui)     & action bar)
  │                                                │
  │                  Sprint 12 ◄───────────────────┘
  │                  (environ.)
  │                      │
  │                      ▼
  │               Sprint 13 ──► Sprint 14 ──► Sprint 15
  │               (quick game)  (campaign)    (polish)
  │
  └─ Sprints 1-4 build the surface simulation engine.
     Sprints 5-7 build the nest into a real simulation.
     Sprints 8-9 add player control and combat.
     Sprints 10-11 add a proper bevy_egui UI layer.
     Sprints 12-14 add gameplay modes and content (mechanics only).
     Sprint 15 consolidates ALL visual/audio polish + sandbox.
```

Each sprint produces a demoable build because they stack vertically — Sprint N always works as a superset of Sprint N-1, never as an isolated branch that needs integration later.
