# Nest AI, Pathfinding & Collision

## Design Document for Underground Nest Systems

---

## 1. Overview

This document covers three interconnected systems needed to make the nest a real, simulated space:

1. **Nest Ant AI** — Individual decision-making for ants performing nursing, digging, hauling, and other nest-interior tasks
2. **Pathfinding** — How ants navigate the tunnel/chamber topology underground (and eventually surface obstacles)
3. **Collision** — Preventing ants from overlapping, walking through walls, and piling up in narrow tunnels
4. **Nest Pheromones** — Chemical road-signs that mark chambers, guide navigation, and drive construction

The surface uses pheromone-gradient navigation by design — that's the core SimAnt mechanic. The nest uses a *different* kind of pheromone system: not trail-following gradients, but **chemical labels on surfaces** that identify chamber functions and guide ants to destinations. Real ants do exactly this (see Section 2). Pathfinding supplements the pheromone system for efficient tunnel routing, while the chemical labels drive the higher-level "where should I go?" decisions.

---

## 2. Biological Basis (Real Ant Behavior)

This section documents what real ants actually do inside their nests. The simulation should be grounded in these behaviors even where we simplify for gameplay.

### 2.1 Chemical Road-Signs Inside the Nest

Real ants **do use pheromones inside the nest**, but differently from surface trail pheromones.

A 2017 Nature Communications study (Heyman et al., "Ants regulate colony spatial organization using multiple chemical road-signs") demonstrated that *Lasius niger* ants deposit **multiple distinct chemical signatures on nest surfaces** that:

- Mark each chamber with a unique chemical profile identifying its function (queen chamber, brood area, food storage, entrance)
- Are functionally meaningful — nurses and foragers use them to navigate to specific destinations in the dark
- Enable different task groups to identify their specific nest destinations
- Stabilize colony spatial organization through chemically mediated local interactions

This is not the same as surface trail pheromone. Surface trails are temporary signals deposited by walking ants that evaporate over time. Nest chemical labels are **persistent surface coatings** maintained by regular ant traffic through each chamber. A chamber "smells like" its function because the ants using it constantly renew its chemical signature.

**Simulation implication:** The nest should have its own pheromone-like system — a per-cell chemical label that identifies chamber type. Ants navigating the nest follow these labels to find their destination. This replaces pure pathfinding for the "where am I going?" question while pathfinding handles the "which tunnel gets me there?" routing.

### 2.2 Queen Pheromone

The queen emits a well-established pheromone that:

- Conveys her fertility status and genotype
- Maintains reproductive dominance (suppresses worker egg-laying)
- Promotes colony integrity and cohesion
- Diffuses through the nest, strongest near the queen chamber

Workers are attracted to the queen pheromone. Its presence throughout the nest signals "queen is alive and well." Its absence triggers emergency queen-rearing behavior.

**Simulation implication:** Queen pheromone should diffuse from the queen's position outward through tunnels. It serves as a "queen health" signal and helps ants locate the queen chamber. Intensity drops with distance.

### 2.3 Brood Chemical Signatures

Whether true "brood pheromones" exist is debated in myrmecology, but brood clearly has chemical signatures:

- Cuticular hydrocarbons on brood surfaces differ between developmental stages (egg, larva, pupa) and between castes (worker vs. reproductive)
- Workers use these chemical signatures to discriminate, sort, and care for brood
- Brood is arranged in **concentric rings** by stage: eggs and micro-larvae at the center, progressively larger larvae outward, then pupae
- Space allocated to each brood type correlates with its metabolic needs
- Nurses translocate brood daily based on temperature preferences — cocoons and large larvae go to warmer spots during the day, cooler at night

**Simulation implication:** Brood items should emit a weak local signal indicating stage and whether they've been fed. Nurses sense these signals to find unfed larvae. Brood sorting (concentric arrangement) can emerge from nurses preferentially placing similar-stage brood near each other.

### 2.4 Nest Construction: Stigmergic Self-Organization

This is the most important biological finding for the digging system. **Ants do not follow a blueprint when digging.** Nest architecture emerges from simple local rules:

#### Building Pheromone
Ants deposit a **construction pheromone** onto excavated soil pellets and deposited building material (Khuong et al., PNAS 2016). This pheromone stimulates other ants to dig/build at the same location, creating a positive feedback loop:

```
Ant digs at spot X → deposits construction pheromone on soil
  → nearby ant detects pheromone → digs at or near spot X
    → more pheromone deposited → more ants recruited
      → tunnel/pillar forms at X
```

The pheromone's **decay rate** is the critical control parameter:
- **Dry conditions** → pheromone decays fast → fewer pillars → larger chambers
- **Humid conditions** → pheromone persists → more pillars → smaller chambers

This means the environment shapes nest architecture without any ant "deciding" chamber sizes.

#### Body-Size Template
Ants use their own body length as a physical template:
- When a pillar/wall reaches one body-length in height, workers switch from vertical to lateral building, forming a ceiling
- This produces chambers of consistent height proportional to ant body size

#### Self-Limiting Excavation
Digging is self-regulating through a collision-based feedback loop (multiple studies):

```
Small tunnel → high ant density at dig face → high collision rate → high digging activity
  ↓
Tunnel expands → ants spread out → fewer reach dig face → collision rate drops
  ↓
Digging slows → eventually near-stops despite no explicit "stop" signal
```

Key findings:
- Excavation follows a pattern: constant rate → rapid decay → slow decay (∝ t^(-1/2))
- Digging almost stops without any explicit negative feedback
- Total excavated volume scales proportionally with worker count
- Larger colonies produce more complex, branching networks
- Worker walking speed correlates with excavation rate

#### Soil Conditions
- Soil temperature and moisture **increase** digging efficiency (ants dig faster in warm, moist soil)
- Soil type affects rate but not architecture
- The CO₂ gradient hypothesis (dig toward CO₂-free air) was experimentally disproven — ants build the same architecture regardless of CO₂ gradients

#### Chamber Placement Rules (Emergent)
Vertical stratification emerges from temperature/humidity gradients:
- **Food storage**: near surface (quick forager access, cooler preservation)
- **Brood chambers**: moderate depth (stable warm temperature for development)
- **Queen chamber**: deep (protected, stable conditions)
- **Midden**: far from brood (disease prevention)

These aren't "rules" ants follow — they emerge from ants preferring certain conditions for certain tasks.

**Simulation implication:** Digging should be **primarily stigmergic**, not player-directed.

The player can *suggest* dig zones (designate areas), but autonomous digging should also occur:
1. Construction pheromone deposited at active dig faces attracts more diggers (positive feedback)
2. Excavation self-limits as tunnel space expands (collision-rate feedback)
3. Chamber size is influenced by an environmental "humidity" parameter controlling pheromone decay
4. Chamber function emerges from depth (food storage tends shallow, queen chamber tends deep)
5. Player designation overrides or supplements, not replaces, the stigmergic system

### 2.5 Summary: What This Means for the Simulation

| Biological Mechanism | Simulation System |
|---|---|
| Chemical road-signs on chamber surfaces | Nest pheromone grid: per-cell chamber-identity labels |
| Queen pheromone diffusing through nest | Queen pheromone layer: diffuses from queen position |
| Brood chemical signatures | Brood components emit local "need feeding" signals |
| Construction pheromone on soil | Dig pheromone layer: deposited at dig faces, attracts diggers |
| Body-size template for chamber height | Chamber height constant (already in grid cell size) |
| Self-limiting excavation via collision rate | Digging rate decreases as tunnel density increases nearby |
| Brood concentric sorting | Nurses place same-stage brood near each other |
| Temperature-driven brood translocation | Depth-based temperature gradient affects brood placement |

---

## 3. Nest Ant AI

### 3.1 Why Not Just Extend the Surface FSM?

The current surface FSM (`AntState` enum: Foraging, Returning, etc.) works because surface tasks are spatially simple: wander until you find food, then follow pheromones home. Nest tasks are different:

- **Nursing** requires locating unfed larvae (via brood signals), retrieving food from storage (via food-storage road-sign), delivering it, then finding the next larva
- **Digging** is stigmergic — ants are attracted to construction pheromone at dig faces, excavate, haul soil to the midden (via midden road-sign)
- **Hauling** involves moving items between specific chambers identified by their chemical labels
- **Queen attendance** means following the queen pheromone gradient to the queen chamber

These are multi-step task chains with dynamic prioritization. A flat FSM would need dozens of sub-states and brittle transition logic.

### 3.2 Recommended Approach: Utility AI + Nest Pheromones

**Utility AI** scores every candidate action numerically each tick and executes the highest-scoring one. **Nest pheromones** provide the sensory inputs that feed those scores. This mirrors real ant behavior: ants don't "decide" to nurse — they *respond* to chemical signals from unfed brood, and their response threshold varies with age and colony state.

Why utility AI fits:

- Ants fluidly switch between nursing, hauling, and other tasks based on chemical signals and colony needs
- No brittle state transitions — scoring naturally handles priority shifts
- Each consideration maps to a real sensory input (pheromone intensity, brood signal strength, proximity)
- Easy to tune: each consideration is an independent scoring curve
- Mirrors the biological "response threshold" model of task allocation

#### Scoring Model

Each nest ant evaluates candidate actions through weighted considerations. Many inputs come from the nest pheromone grid:

```
Action: FEED_LARVA
  × brood_need_signal(0.0–1.0)                  [Brood chamber pheromone intensity, Sigmoid]
  × proximity_to_brood_road_sign(0.0–1.0)       [Sensed from nest chamber labels]
  × food_available_in_storage(0.0–1.0)           [Step: 0 if empty, 1 if any]
  × ant_age_nursing_affinity(0.0–1.0)            [High for young ants]

Action: DIG_AT_FACE
  × construction_pheromone_intensity(0.0–1.0)    [Dig pheromone at nearby faces, Sigmoid]
  × dig_zone_designated(0.0–1.0)                 [Player boost, 0.3 base if stigmergic-only]
  × colony_needs_expansion(0.0–1.0)              [Based on population vs capacity]
  × ant_age_digging_affinity(0.0–1.0)            [High for mid-age ants]
  × collision_rate_at_face(1.0–0.0)              [Inversely proportional — self-limiting]

Action: HAUL_FOOD_TO_STORAGE
  × food_at_entrance(0.0–1.0)                    [Amount waiting at entrance]
  × food_storage_road_sign(0.0–1.0)              [Can the ant sense food-storage label?]
  × storage_not_full(0.0–1.0)                    [Capacity remaining]

Action: HAUL_WASTE_TO_MIDDEN
  × waste_in_chambers(0.0–1.0)                   [Cleanliness score]
  × midden_road_sign(0.0–1.0)                    [Can the ant sense midden label?]
  × proximity_to_waste(0.0–1.0)                  [Inverse distance]

Action: ATTEND_QUEEN
  × queen_pheromone_intensity(0.0–1.0)           [Gradient from queen position]
  × queen_hunger(0.0–1.0)                        [Queen needs feeding]
  × queen_unattended(0.0–1.0)                    [No other ant nearby]

Action: IDLE_IN_NEST
  × base_score: 0.1                              [Fallback, always available]
```

Scores are computed as the **product** of all considerations for an action. A zero in any consideration vetoes the action entirely. The highest-scoring action wins.

#### Integration with Existing FSM

The utility AI runs *only* for ants in the nest. Surface ants keep the existing FSM. The bridge:

```
Surface FSM                          Nest Utility AI
┌──────────┐                        ┌──────────────────┐
│ FORAGING │ ──(enters nest)──────► │ Evaluate actions  │
│ RETURNING│                        │ Score & pick      │
│ DEFENDING│ ◄──(exits to surface)──│ Execute best      │
└──────────┘                        └──────────────────┘
```

When a surface ant enters the nest entrance, it switches to the utility AI system. When the utility AI decides the ant should go forage (no nest tasks needed), the ant exits to the surface and resumes the FSM.

#### Bevy Implementation Strategy

Two viable approaches:

**Option A: Hand-rolled (recommended for this project)**
- A `NestAntBrain` component stores candidate action scores
- A `nest_utility_scoring` system evaluates all considerations each tick
- A `nest_action_execution` system runs the top-scored action
- Simple, no external dependency, full control over tuning

**Option B: `bevy_observed_utility` crate (v0.2.0, Bevy 0.15 compatible)**
- Provides scoring hierarchies, evaluator curves (Linear, Sigmoid, Exponential), and picking strategies out of the box
- ECS observer-based architecture, ergonomic API
- Good if we want production-grade utility AI with less boilerplate
- Adds a dependency; relatively new crate (~2K downloads)

**Recommendation:** Start with Option A for Sprint 4.5 to keep dependencies minimal and maintain full understanding of the system. Migrate to `bevy_observed_utility` later if the hand-rolled version becomes unwieldy.

### 3.3 Nest Ant Task Chains

Each action selected by the utility AI triggers a task chain — a sequence of sub-steps. Navigation within the nest uses a hybrid of **pheromone road-signs** (to identify the destination chamber) and **JPS pathfinding** (to route through tunnels efficiently).

```
FEED_LARVA task chain:
  1. Sense food-storage road-sign → identify destination
  2. Pathfind through tunnels to food storage chamber
  3. Pick up food unit
  4. Sense brood-chamber road-sign → identify destination
  5. Pathfind through tunnels to brood chamber
  6. Find nearest unfed larva (local brood signal)
  7. Deliver food to larva (larva.fed = true)
  8. Re-evaluate (loop)

DIG_AT_FACE task chain:
  1. Sense construction pheromone gradient → identify active dig face
     (or: follow player-designated dig zone marker)
  2. Pathfind to dig face
  3. Excavate soil cell (duration based on soil type + moisture)
  4. Deposit construction pheromone at excavated face (recruits more diggers)
  5. Cell becomes Tunnel
  6. Pick up soil particle
  7. Sense midden road-sign → pathfind to midden
  8. Drop soil
  9. Re-evaluate (loop — may return to same face if pheromone still strong)

HAUL_FOOD task chain:
  1. Sense food signal at entrance (food waiting to be stored)
  2. Pathfind to nest entrance
  3. Pick up food
  4. Sense food-storage road-sign → pathfind to storage chamber
  5. Drop food
  6. Re-evaluate (loop)

ATTEND_QUEEN task chain:
  1. Follow queen pheromone gradient (strongest near queen)
  2. Pathfind toward queen chamber
  3. Groom/feed queen (reduces queen hunger)
  4. Remain near queen until utility score shifts to another action
```

Task chains are tracked with a `NestTask` component:

```rust
#[derive(Component)]
enum NestTask {
    FeedLarva { step: FeedStep, target_larva: Option<Entity> },
    Dig { step: DigStep, target_cell: Option<(usize, usize)> },
    HaulFood { step: HaulStep },
    HaulWaste { step: HaulStep },
    AttendQueen { step: AttendStep },
    Idle,
}
```

### 3.4 Age-Based Affinity (Temporal Polyethism)

The utility AI naturally supports age-based task allocation through scoring curves. This mirrors real ant colonies where young ants stay deep in the nest and older ants work progressively closer to the surface:

| Age Range | Nursing Affinity | Digging Affinity | Hauling Affinity | Notes |
|---|---|---|---|---|
| 0–20% | 2.0× | 0.5× | 0.8× | Young ants nurse deep in nest |
| 20–40% | 0.8× | 1.5× | 1.2× | Mid-age ants dig at tunnel perimeter |
| 40–70% | 0.3× | 0.5× | 1.5× | Older ants haul between chambers, may exit to forage |
| 70%+ | 0.1× | 0.2× | 0.5× | Oldest ants primarily surface workers |

These multipliers are applied as additional considerations in the scoring, creating a natural age-based division of labor without hard-coded role assignments. Combined with pheromone inputs, this produces the real-world pattern where young ants cluster near the brood and queen while older ants work near the entrance.

---

## 4. Pathfinding

### 4.1 Hybrid Navigation: Pheromones Tell You Where, Pathfinding Gets You There

The nest uses a two-layer navigation system that mirrors how real ants work:

**Layer 1 — Chemical road-signs (WHERE to go):** Each chamber has a chemical identity label. Ants sense these labels to identify *which* chamber they need. A nurse senses the brood-chamber label and the food-storage label. The queen pheromone gradient points toward the queen. Construction pheromone marks active dig faces.

**Layer 2 — Tunnel routing (HOW to get there):** Once an ant knows its destination chamber, it needs to navigate through the tunnel network. In narrow, branching tunnels, pheromone gradients alone are unreliable — a T-junction where both branches eventually lead to the brood chamber would create equal gradient strength. JPS pathfinding efficiently routes through the tunnel graph.

This is biologically grounded: real ants use chemical labels on nest surfaces to identify destinations (Heyman et al. 2017), but they also learn spatial layout through experience and use path integration. Our JPS pathfinding is an abstraction of that spatial knowledge.

### 4.2 Algorithm Comparison

| Algorithm | Strengths | Weaknesses | Best For |
|---|---|---|---|
| **A*** | Simple, optimal paths | O(n) per agent per query, slow for many agents | Few agents, complex goals |
| **Jump Point Search (JPS)** | 10×+ faster than A* on uniform grids | Only works on uniform-cost grids | Grid-based nest, moderate agent count |
| **Hierarchical A* (HPA*)** | Fast for large maps, precomputed | Setup overhead, near-optimal (not optimal) | Large maps, many queries |
| **Flowfield** | One computation, all agents share it | Memory for grid, less useful with varied destinations | Many agents, same destination |

### 4.3 Recommended: JPS for Nest, Flowfield for Surface (Future)

**For the nest: Jump Point Search (JPS)**

The nest is a uniform-cost grid (all passable cells cost the same to traverse). JPS exploits grid symmetry to skip large regions, running 10×+ faster than A* on grids. The `grid_pathfinding` crate (v0.3.0) provides a Rust implementation with:

- JPS with improved pruning rules
- 4-neighbor and 8-neighbor support
- Pre-computed connected components (instantly rejects impossible paths)
- No Bevy dependency (pure algorithm crate, easy to integrate)

The nest grid is small (60×40 = 2,400 cells). Even unoptimized A* would be fast at this scale. JPS is chosen for headroom as the nest grows and for the efficiency pattern it establishes.

**Path caching:** Ants following the same task chain (e.g., all nurses going to brood chamber) will compute similar paths. Cache the last N paths keyed by (start_region, goal_region) and reuse when the tunnel structure hasn't changed.

**Path invalidation:** When a digger excavates a new cell, mark cached paths through that region as stale. The nest grid changes infrequently (only during active digging), so cache hit rates will be high.

**For the surface (future):** If surface obstacles are added (rocks, walls, buildings), a **flowfield** approach fits best — many ants going to the same food source or nest entrance can share a single flow computation. The `bevy_flowfield_tiles_plugin` (v0.14.0) or `bevy_pathfinding` (v0.1.0) crates are options. However, the surface pheromone system should remain the primary navigation for foraging; pathfinding would only supplement it for specific navigational tasks (e.g., returning to nest entrance through a maze).

### 4.4 Integration with Nest Grid

```
┌─────────────────────────────────────────────────────┐
│                  PATHFINDING FLOW                     │
│                                                       │
│  1. Utility AI picks action (e.g., FEED_LARVA)       │
│  2. Task chain needs to reach brood chamber           │
│  3. Query: pathfind(ant_grid_pos, brood_chamber_pos)  │
│                                                       │
│  4. Check path cache:                                 │
│     HIT  → return cached path                         │
│     MISS → run JPS on NestGrid                        │
│            (passable = Tunnel | Chamber)               │
│            cache result                                │
│                                                       │
│  5. Ant follows path: list of grid cells              │
│     Each tick: move toward next waypoint              │
│     When within threshold: advance to next cell       │
│                                                       │
│  6. Path complete → advance task chain step           │
└─────────────────────────────────────────────────────┘
```

#### Bevy Components

```rust
#[derive(Component)]
struct NestPath {
    waypoints: Vec<(usize, usize)>,
    current_index: usize,
}

#[derive(Resource)]
struct PathCache {
    paths: HashMap<(GridPos, GridPos), Vec<(usize, usize)>>,
    generation: u32,  // increments when nest grid changes
}
```

### 4.5 Crate Comparison

| Crate | Version | Bevy Compat | Approach | Notes |
|---|---|---|---|---|
| `grid_pathfinding` | 0.3.0 | N/A (pure algo) | JPS on grids | Best for nest. Lightweight, no Bevy dep. |
| `bevy_northstar` | 0.6.1 | Bevy 0.15 | HPA*, A*, Theta* | Heavy for our small grid. Good if nest gets huge. |
| `bevy_flowfield_tiles_plugin` | 0.14.0 | Bevy 0.14 | Flowfield sectors | Great for surface mobs. Version lag. |
| `bevy_pathfinding` | 0.1.0 | Bevy 0.15 | Flowfield + boids | Includes collision avoidance. Good for surface. |

**Recommendation:** Use `grid_pathfinding` for the nest (pure JPS, no dependency bloat). Evaluate `bevy_pathfinding` for surface pathfinding if/when surface obstacles are added.

---

## 5. Collision Detection & Avoidance

### 5.1 Two Layers of Collision

**Layer 1: Grid collision (wall avoidance)**
Ants cannot enter impassable cells (Soil, Rock, Clay). This is handled inherently by pathfinding — paths only traverse passable cells. For real-time movement between waypoints, clamp ant positions to passable cells and reject moves into walls.

**Layer 2: Agent-to-agent collision (ant avoidance)**
Multiple ants in the same narrow tunnel need to avoid stacking on top of each other. This is a *soft* collision — ants don't bounce off each other, they apply a gentle separation force.

### 5.2 Spatial Hash Grid (Already Exists)

The project already has a `SpatialGrid` resource (spatial hash map of entity positions). This is the foundation for both nest and surface collision queries.

For the nest, the spatial grid enables:
- **Neighbor queries** for separation steering (push apart ants within minimum distance)
- **Tunnel congestion detection** (too many ants in a narrow tunnel → some wait or reroute)
- **Task target queries** (find nearest unfed larva, nearest food unit)

### 5.3 Separation Steering

Each tick, for each nest ant:

```
separation_force = Vec2::ZERO
for each nearby_ant within SEPARATION_RADIUS:
    away = ant_pos - nearby_pos
    distance = away.length()
    if distance < MIN_SEPARATION and distance > 0.01:
        separation_force += away.normalize() * (1.0 - distance / MIN_SEPARATION)

ant_velocity += separation_force * SEPARATION_WEIGHT
```

Parameters:
- `SEPARATION_RADIUS`: 12.0 (how far to check for neighbors)
- `MIN_SEPARATION`: 6.0 (desired minimum distance between ants)
- `SEPARATION_WEIGHT`: 0.5 (how strongly ants push apart)

This uses the existing `SpatialGrid` for O(1) neighbor lookups.

### 5.4 Tunnel Traffic Management

Narrow tunnels (1 cell wide) create bottlenecks. Options:

**Option A: One-way flow (simple)**
Mark tunnels with a preferred flow direction based on dominant traffic. Ants moving against flow yield (pause briefly). Mimics real ant behavior where narrow trails develop directional conventions.

**Option B: Passing bays (realistic)**
Widen critical tunnel junctions to 2 cells. Ants meeting head-on in a 1-wide tunnel: one steps into the nearest bay, other passes, then both continue. Requires tracking tunnel width in the grid.

**Option C: Priority queuing (pragmatic)**
Ants carrying items have movement priority. Empty ants yield to laden ants. Simple priority check resolves most tunnel conflicts.

**Recommendation:** Start with Option C (priority queuing) as it's simplest to implement and biologically accurate — real ants carrying food have right-of-way in tunnels. Add Option B later for visual interest.

### 5.5 Surface Collision (Future)

The surface currently has no ant-to-ant collision. When adding it:

1. Reuse the existing `SpatialGrid`
2. Add separation steering (same as nest, different parameters for open space)
3. Consider `bevy_pathfinding`'s built-in boid-based collision avoidance if surface pathfinding is added

For physics-based collision (predators, environmental objects), **Avian 2D** (successor to bevy_xpbd) is the recommended crate:
- Active development, Bevy 0.15 compatible
- Modular — can use just collision detection without full physics simulation
- Supports collision events (`CollisionStart`/`CollisionEnd`)
- Spatial queries (raycasting, shape casting)

However, full physics is likely overkill for this project. The spatial hash + steering approach handles 90% of needs.

### 5.6 Performance Budget

| System | 100 nest ants | 1,000 surface ants | 10,000 total |
|---|---|---|---|
| Spatial grid rebuild | ~0.01ms | ~0.1ms | ~1ms |
| Separation steering | ~0.02ms | ~0.2ms | ~2ms |
| JPS pathfinding (per query) | ~0.01ms | N/A (surface uses pheromones) | — |
| Path following (per ant) | ~0.001ms | N/A | — |

Total nest AI + navigation budget: **< 2ms/frame** for 100 nest ants, well within the 16ms frame budget at 60fps.

---

## 6. Nest Pheromone System

The nest has its own pheromone grid, separate from the surface pheromone grid. It stores chemical information per cell and operates on different timescales.

### 6.1 Pheromone Layers

| Layer | Deposited By | Decay Rate | Purpose |
|---|---|---|---|
| **Chamber Identity** | Ants working in chamber (passive) | Very slow (stable label) | Marks chamber function: brood, food-storage, queen, midden, entrance |
| **Queen Signal** | Queen entity (continuous) | Medium (diffuses outward) | Gradient field: strongest at queen, weakens with distance through tunnels |
| **Construction** | Digger ants (at dig face) | Fast (minutes) | Attracts more diggers to active excavation sites, self-limiting |
| **Brood Need** | Unfed larvae (continuous) | Medium | "I'm hungry" signal, sensed by nurses |

### 6.2 Chamber Identity Labels

Each passable cell in the `NestGrid` carries a chemical label value per chamber type. The label is maintained by ant traffic — ants in a chamber continuously "refresh" its label through their presence. If a chamber is abandoned, its label slowly fades, allowing repurposing.

```rust
#[derive(Clone, Default)]
struct NestCellPheromones {
    chamber_labels: [f32; 5],  // [Brood, FoodStorage, Queen, Midden, Entrance]
    queen_signal: f32,
    construction: f32,
    brood_need: f32,
}
```

Ants sense labels in neighboring cells and can identify "I'm near the food storage area" or "the brood chamber is to my left." This informs the utility AI scoring and helps ants orient in the nest without explicit pathfinding for the initial "where am I?" question.

### 6.3 Construction Pheromone Dynamics

The construction pheromone drives stigmergic digging:

```
Each tick, for each nest cell:
  construction[x][y] *= (1.0 - CONSTRUCTION_DECAY_RATE)

When a digger excavates a cell:
  construction[adjacent_soil_cells] += CONSTRUCTION_DEPOSIT
  → nearby ants with DIG affinity sense this and score DIG_AT_FACE higher

Self-limiting feedback:
  effective_deposit = CONSTRUCTION_DEPOSIT * (1.0 / (1.0 + nearby_ant_count * 0.5))
  → more ants crowding a face = less pheromone per deposit = fewer recruits
```

The `CONSTRUCTION_DECAY_RATE` is a tunable parameter analogous to the biological humidity-dependent pheromone lifetime. Higher decay → ants spread out more → larger chambers. Lower decay → ants cluster → more pillars, smaller chambers.

### 6.4 Queen Signal Diffusion

Queen pheromone diffuses through passable cells only (not through soil walls), creating a gradient that follows tunnel connectivity:

```
Each tick:
  queen_signal at queen_position = QUEEN_SIGNAL_STRENGTH
  for each passable cell:
    queen_signal[x][y] *= (1.0 - QUEEN_SIGNAL_DECAY)
    diffuse to passable neighbors at QUEEN_DIFFUSE_RATE
```

This means the queen signal is strongest in the queen chamber, medium in adjacent tunnels, and weak in distant chambers. Ants following the queen pheromone gradient naturally route through tunnels toward the queen — a form of navigation that doesn't require explicit pathfinding.

### 6.5 Integration with Surface Pheromone System

The nest pheromone grid is a separate `Resource` from the surface `ColonyPheromones`. They don't interact directly, but share the nest entrance as a boundary:

- Surface HOME pheromone guides foragers to the nest entrance
- At the entrance, ants transition to the nest pheromone system
- Entrance road-sign is the strongest chamber label at the entrance cells
- Food deposited at the entrance generates a "food waiting" signal that haulers sense

---

## 7. Architecture Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                        NEST SYSTEMS                                  │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │ Nest Pheromone Grid                                      │       │
│  │  • Chamber identity labels (stable, per-cell)            │       │
│  │  • Queen signal (diffuses from queen through tunnels)    │       │
│  │  • Construction pheromone (fast-decaying, at dig faces)  │       │
│  │  • Brood need signal (emitted by unfed larvae)           │       │
│  └──────────────────────────┬───────────────────────────────┘       │
│                              │ sensory input                         │
│                              ▼                                       │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐           │
│  │ Utility AI   │───►│ Task Chain   │───►│ Pathfinding  │           │
│  │ (scoring     │    │ (steps)      │    │ (JPS routing │           │
│  │  via phero-  │    │              │    │  through     │           │
│  │  mone input) │    │              │    │  tunnels)    │           │
│  └──────────────┘    └──────────────┘    └──────┬───────┘           │
│                                                  │                   │
│                                          ┌───────▼───────┐           │
│                                          │ Path Following│           │
│                                          │ (waypoints)   │           │
│                                          └───────┬───────┘           │
│                                                  │                   │
│  ┌──────────────┐    ┌──────────────┐    ┌───────▼───────┐           │
│  │ Spatial Grid │───►│ Separation   │───►│ Movement      │           │
│  │ (queries)    │    │ Steering     │    │ (final pos)   │           │
│  └──────────────┘    └──────────────┘    └───────┬───────┘           │
│                                                  │                   │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │ Grid Collision (wall clamp, passability check)           │       │
│  └──────────────────────────────────────────────────────────┘       │
│                                                                      │
│  Resources: NestGrid, NestPheromoneGrid, SpatialGrid, ColonyFood   │
└─────────────────────────────────────────────────────────────────────┘
```

### System Execution Order (within Update)

```
 1. nest_pheromone_decay        — decay construction pheromone, diffuse queen signal
 2. nest_pheromone_emit         — queen emits signal, unfed brood emits need, diggers emit construction
 3. nest_chamber_label_refresh  — ants in chambers passively refresh identity labels
 4. nest_utility_scoring        — score all candidate actions (reads pheromone grid)
 5. nest_action_selection       — pick highest-scored action, set NestTask
 6. nest_task_advance           — advance task chain steps (request paths, etc.)
 7. nest_pathfind               — run JPS for ants needing new paths
 8. nest_path_following         — move ants along waypoints
 9. nest_separation_steering    — apply separation forces
10. nest_grid_collision         — clamp positions to passable cells
11. nest_task_completion        — detect completed sub-steps, advance chain
```

---

## 8. Crate Dependencies

### Required (add to Cargo.toml)

```toml
grid_pathfinding = "0.3"   # JPS pathfinding for nest grid
```

### Optional (evaluate later)

```toml
# If surface needs pathfinding around obstacles:
# bevy_pathfinding = "0.1"

# If full physics collision is needed for predators/hazards:
# avian2d = "0.2"

# If hand-rolled utility AI becomes too complex:
# bevy_observed_utility = "0.2"
```

---

## 9. Open Questions

1. ~~**Nest pheromones?**~~ **RESOLVED: Yes.** The nest has its own pheromone grid with four layers (chamber identity, queen signal, construction, brood need). See Section 6. This is biologically grounded in Heyman et al. 2017 and Khuong et al. 2016.

2. ~~**Digging AI autonomy?**~~ **RESOLVED: Primarily stigmergic, with player override.** Digging is driven by construction pheromone (positive feedback) and self-limits via crowding (negative feedback). Player designation boosts the score but doesn't replace autonomous excavation. See Section 2.4.

3. **Nest ant population:** How many ants should be simulated inside the nest at once? Currently 8 `NestWorker` entities are visual decoration. Real gameplay needs 20–50 active nest ants with full AI. Performance budget (Section 5.6) suggests 50 is comfortable.

4. **Cross-view continuity:** When switching between surface and underground views, should the same ant entities exist in both spaces? Or are surface ants and nest ants separate populations? The current implementation uses separate `NestWorker` entities that are purely visual. Recommendation: unified entities with an `Underground` marker component.

5. **Chamber label granularity:** Should chamber identity labels be binary (this cell IS brood chamber) or gradient (this cell is 80% brood, 20% tunnel)? Gradient labels would allow chambers to organically shift function over time but add complexity. Binary labels are simpler and sufficient for gameplay.

6. **Construction pheromone tuning as gameplay:** The humidity parameter controlling construction pheromone decay (and thus chamber size) could be an environmental factor that varies by patch in campaign mode — sandy patches produce different nest architecture than clay patches. This would make each colony's nest feel unique.

7. **Brood sorting depth:** Should we simulate concentric brood arrangement (eggs center, larvae outward, pupae outermost) or just cluster by stage? Full concentric sorting is visually impressive but adds nurse AI complexity. Cluster-by-stage is simpler and still reads well.

---

## 10. Biological References

| Source | Key Finding | Simulation Impact |
|---|---|---|
| Heyman et al. 2017, *Nature Communications* — "Ants regulate colony spatial organization using multiple chemical road-signs" | Ants deposit distinct chemical signatures on nest surfaces identifying chamber functions. Nurses and foragers use these to navigate in darkness. | Nest pheromone grid with chamber identity labels |
| Khuong et al. 2016, *PNAS* — "Stigmergic construction and topochemical information shape ant nest architecture" | Construction pheromone on building material recruits more diggers. Pheromone decay rate (climate-dependent) controls chamber size. Body-length template for chamber height. | Construction pheromone layer, stigmergic dig system, humidity parameter |
| Sendova-Franks & Franks 1999, *Behavioral Ecology & Sociobiology* — "Brood sorting by ants" | Brood arranged in concentric rings by stage. Space allocation correlates with metabolic needs. | Brood sorting behavior for nurses |
| Mailleux et al. 2024, *Insectes Sociaux* — "Ants deposit more pheromone close to food sources" | Ants deposit up to 22× more trail pheromone near food vs. near nest. Distant food sources get stronger pheromone. | Already implemented in surface foraging system |
| Bruce et al. 2018, *Insectes Sociaux* — "The digging dynamics of ant tunnels" | Excavation self-limits via collision-rate feedback. Digging rate ∝ worker walking speed. Smaller groups more sensitive to existing tunnel length. | Self-limiting dig feedback, collision-rate consideration in utility scoring |
| NSF 2023 — "Agitated ants: regulation of incipient nest excavation via collisional cues" | Ants estimate collision frequency; high collisions drive excavation, low collisions slow it. Multi-phase decay: constant → rapid → t^(-1/2). | Collision-rate input to dig utility scoring |
| Tschinkel 2013 — "Florida harvester ant nest architecture" | CO₂ gradient hypothesis disproven. Ants rebuild same architecture in new locations. Nest volume ∝ population. Vertical stratification of chambers. | Chamber depth rules, population-proportional nest size |
| Kipyatkov & Lopatina 2015 — "Brood translocation and temperature preference" | Nurses translocate brood daily based on temperature. Different stages have different temperature optima. | Depth-based temperature gradient affecting brood placement |
