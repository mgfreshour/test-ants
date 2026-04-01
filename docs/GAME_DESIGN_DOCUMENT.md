# Colony: An Ant Colony Simulation

## Game Design Document

---

## 1. Overview

**Title:** Colony
**Genre:** Real-time simulation / strategy
**Engine:** Bevy (Rust)
**Inspiration:** SimAnt (Maxis, 1991) by Will Wright & Justin McCormick
**Target Platform:** Desktop (Windows, macOS, Linux)

### 1.1 Elevator Pitch

Colony is a modern reimagining of SimAnt — the player guides a black ant colony from a single queen to total yard domination, competing against a rival red colony. Control individual ants, lay pheromone trails, manage caste ratios, expand underground nests, and face environmental hazards — all rendered in a stylized 2D view powered by Bevy's ECS architecture.

### 1.2 Design Pillars

1. **Emergent Complexity** — Simple per-ant rules produce sophisticated colony-level behavior through stigmergy and pheromone feedback loops.
2. **Dual-Scale Play** — Seamlessly switch between commanding a single ant on the ground and managing macro-level colony strategy (caste ratios, nest layout, expansion targets).
3. **Ecological Authenticity** — Grounded in real myrmecology: pheromone communication, age-based task allocation (temporal polyethism), brood care cycles, and nest architecture.
4. **Accessible Depth** — Easy to pick up (control one ant, find food), deep to master (optimize foraging efficiency, win territorial wars, colonize the house).

---

## 2. Game Modes

### 2.1 Quick Game

- Single yard patch, top-down surface view + side-view underground nest
- Black colony vs. AI-controlled red colony
- Victory: eliminate the red queen or drive all red ants from the patch
- Defeat: black queen dies with no replacement queens available

### 2.2 Campaign (Full Game)

- Overhead map divided into a grid of yard patches plus a house interior
- Player starts in one patch and must expand by producing winged queens that settle new patches
- Victory: colonize 70%+ of the house and eliminate the red colony entirely
- Progressive difficulty — later patches introduce more hazards (sprinklers, pets, pesticide)
- Between-patch strategy layer: choose which adjacent patch to colonize next

### 2.3 Sandbox (Experimental)

- Full environmental controls: spawn food, place walls/mazes, paint pheromones, adjust parameters
- Control both black and red colonies
- Adjustable simulation speed (pause, 1x, 2x, 4x, 8x)
- Data overlays: pheromone heat maps, population graphs, foraging efficiency metrics
- Useful for experimentation and understanding ant behavior systems

---

## 3. Core Mechanics

### 3.1 The Yellow Ant (Player Avatar)

The player directly controls one ant at a time, rendered distinctly (yellow highlight / glow). The controlled ant can:

| Action | Key/Input | Effect |
|---|---|---|
| Move | WASD / Arrow keys | Direct movement |
| Pick up item | E | Grab food, pebble, or larva |
| Drop item | Q | Release carried item |
| Attack | Space | Bite enemy ant, spider, etc. |
| Lay pheromone | Hold Shift + move | Deposit trail pheromone |
| Recruit | R | Nearby ants follow you |
| Dismiss | T | Release recruited ants |
| Exchange ant | Tab | Jump to a different ant |
| Regurgitate food | F | Share food with nearby nestmate |

### 3.2 Pheromone System

Pheromones are the primary communication mechanism. They are stored as floating-point intensity values on a 2D grid overlay.

**Pheromone Types:**

| Type | Color Overlay | Purpose |
|---|---|---|
| Home | Blue | Marks path back to nest entrance |
| Food | Green | Marks path to food source |
| Alarm | Red | Alerts nearby ants to danger / enemies |
| Trail | Yellow | General recruitment trail (player-laid) |
| Colony ID | (invisible) | Distinguishes friendly vs. enemy territory |

**Pheromone Dynamics:**
- **Deposition:** Ants deposit pheromone at their current cell each tick. Intensity is additive (more ants = stronger signal).
- **Evaporation:** Each tick, every cell's pheromone decays by a configurable factor (default 2% per second). This prevents stale trails from persisting indefinitely.
- **Diffusion:** Pheromone spreads to adjacent cells at a lower rate (default 0.5% per tick), creating gradient fields that ants can follow uphill.
- **Rain:** Rainfall events dramatically increase evaporation rate (10x), washing away trails.

### 3.3 Colony Management

The player accesses a Colony Panel (hotkey: C) with three control sliders:

**Behavior Allocation** (what workers do):
- Foraging (surface food gathering)
- Nursing (brood care in the nest)
- Excavation (tunnel digging)
- Defense (patrolling nest entrances and territory borders)

**Caste Ratios** (what eggs become):
- Workers (balanced stats, versatile)
- Soldiers (high HP/attack, slow, expensive to feed)
- Drones (males, needed for mating flights — only useful in campaign)

**Nest Management:**
- Designate dig zones underground
- Set chamber types: brood chamber, food storage, queen chamber, trash heap
- Relocate queen deeper when threats approach

### 3.4 Underground Nest (Side View)

The underground is rendered as a 2D cross-section with:

- **Tunnels:** Narrow passages connecting chambers, dug by excavator ants
- **Chambers:** Wider rooms with designated purposes
- **Soil types:** Soft soil (fast dig), clay (slow dig), rock (impassable)
- **Water table:** Digging too deep risks flooding
- **Structural integrity:** Unsupported wide chambers can collapse

Chamber types and their functions:

| Chamber | Function |
|---|---|
| Brood | Eggs, larvae, pupae develop here. Nurses required. |
| Food Storage | Collected food cached here. Workers retrieve as needed. |
| Queen Chamber | Queen resides and lays eggs. Must be protected. |
| Midden | Waste disposal. Placed far from brood to prevent disease. |
| Fungus Garden | (Late game) Cultivate fungus from leaf cuttings for food. |

### 3.5 Combat

Combat uses a simple stat-based system resolved per tick:

**Ant Stats:**
| Stat | Worker | Soldier | Description |
|---|---|---|---|
| HP | 10 | 25 | Health points |
| Attack | 2 | 6 | Damage per bite |
| Speed | 5 | 3 | Tiles per second |
| Food Cost | 1 | 3 | Food consumed per cycle |

**Combat Resolution:**
- Melee range (adjacent tile). Attacker deals damage = Attack ± random(0-1).
- Soldiers get a 50% damage bonus when defending near nest entrance.
- Group bonus: +10% attack per additional allied ant in the fight (up to +50%).
- Defeated ants drop carried items and become food (if enemy colony recovers them).

**Predators:**
| Predator | HP | Attack | Behavior |
|---|---|---|---|
| Spider | 80 | 15 | Ambush near web, attacks lone ants |
| Antlion | 60 | 20 | Pit trap, instant kill on contact |
| Human foot | — | 999 | Periodic random stomps, area damage |
| Lawnmower | — | 999 | Linear sweep across surface, total destruction in path |

### 3.6 Resource Economy

**Food Sources:**
- Crumbs (small, common) — 5 food units
- Dead insects (medium) — 20 food units, must be carried by 2+ ants
- Fruit/sugar (large) — 50 food units, depletes over time as ants harvest
- Fungus garden (renewable) — produces 2 food/cycle when maintained

**Food Flow:**
1. Forager finds food on surface
2. Forager picks up food, follows home pheromone back to nest
3. Forager deposits food in storage chamber
4. Nurses retrieve food to feed larvae and queen
5. Queen consumes food to produce eggs (1 food = 1 egg at base rate)

**Population Dynamics:**
- Egg → Larva: 30 seconds
- Larva → Pupa: 45 seconds
- Pupa → Adult: 30 seconds
- Adult lifespan: 5 minutes (workers), 3 minutes (soldiers), 20 minutes (queen)
- Colony collapse if queen dies and no new queens are in brood pipeline

---

## 4. AI Systems

### 4.1 Individual Ant AI (State Machine)

Each non-player ant runs a finite state machine:

```
         ┌──────────┐
    ┌───►│  IDLE     │◄──────────────┐
    │    └────┬─────┘               │
    │         │ hunger > threshold   │
    │         ▼                      │
    │    ┌──────────┐  found food   ┌┴─────────┐
    │    │ FORAGING  ├─────────────►│ RETURNING │
    │    └────┬─────┘              └───────────┘
    │         │ detect alarm                ▲
    │         ▼                             │
    │    ┌──────────┐  enemy fled     ┌─────┴─────┐
    │    │ FIGHTING  ├───────────────►│ RETURNING  │
    │    └──────────┘                └───────────┘
    │
    │    ┌──────────┐
    ├───►│ NURSING   │  (assigned)
    │    └──────────┘
    │    ┌──────────┐
    └───►│ DIGGING   │  (assigned)
         └──────────┘
```

**State Transitions:**
- IDLE → FORAGING: when colony food < threshold or behavior slider favors foraging
- FORAGING → RETURNING: when carrying food
- ANY → FIGHTING: when alarm pheromone detected and ant is assigned defense role
- FIGHTING → RETURNING: when no enemies nearby
- IDLE → NURSING: when behavior slider assigns nursing duty
- IDLE → DIGGING: when behavior slider assigns excavation duty

### 4.2 Ant Movement Algorithm

Ants navigate using a weighted random walk influenced by pheromones:

1. Sample pheromone intensity in 8 neighboring cells
2. Weight each direction by: `pheromone_intensity^alpha * (1/distance_to_target)^beta`
3. Add random noise factor (configurable `exploration_rate`)
4. Select direction probabilistically from weighted distribution
5. Move one cell in the selected direction

This produces realistic-looking meandering paths that converge on strong pheromone trails.

### 4.3 Red Colony AI (Opponent)

The AI-controlled red colony runs the same per-ant simulation but with a strategic layer:

- **Aggression Level:** Ramps up over time or in response to player territory expansion
- **Expansion Priority:** AI periodically sends scout groups to find new food and claim territory
- **Raid Behavior:** When red population exceeds black by 2x, AI launches coordinated raids
- **Queen Protection:** Red colony always maintains a guard contingent near its queen
- **Adaptive Difficulty:** In campaign mode, later patches have smarter/more aggressive red AI

### 4.4 Predator AI

- **Spider:** Spawns web at a random surface location. Stays near web. Attacks any ant entering a 3-tile radius. Relocates web after 2 minutes or if web destroyed.
- **Antlion:** Digs pit at sandy areas. Ants entering pit slide to center and take damage. Antlion emerges to finish trapped ants.
- **Human events:** Scripted hazards on timers — footsteps (random location, 3-tile radius), lawnmower (horizontal sweep), pesticide spray (lingering poison zone).

---

## 5. Visual Design

### 5.1 Art Style

- **Top-down surface:** Stylized pixel art at ~32x32 tile resolution. Grass, dirt, concrete, and indoor tile biomes.
- **Side-view underground:** Earth-tone cross-section. Soil layers visible. Chambers are rounded cutouts.
- **Ants:** Simple but expressive sprites (6-8 frames walk cycle). Color-coded by colony (black, red, yellow for player).
- **UI:** Clean, minimal HUD. Colony panel slides in from the side. Pheromone overlays toggled with hotkeys.

### 5.2 Camera

- Surface: Freely pannable top-down camera with zoom (scroll wheel)
- Underground: Horizontal scroll, vertical scroll, zoom
- Minimap in corner showing full patch with ant density overlay
- Smooth transition animation when switching surface ↔ underground

### 5.3 Visual Feedback

- Pheromone trails rendered as semi-transparent colored gradients on the tile grid
- Alarm pheromone pulses with a ripple effect
- Combat: small flash/shake on hit, ant fragments on death
- Food sources glow subtly to aid discovery
- Queen has a distinct crown-like visual marker

---

## 6. Audio Design

| Context | Sound |
|---|---|
| Ambient (surface) | Wind, birds, distant lawnmower |
| Ambient (underground) | Low rumble, subtle scratching |
| Ant movement | Faint chittering (grouped ants louder) |
| Combat | Sharp clicking, mandible snaps |
| Food found | Positive chime |
| Queen endangered | Urgent alarm tone |
| Colony milestone | Fanfare sting |
| Rain event | Rain patter, thunder |

Music: Ambient electronic/organic hybrid that shifts in intensity based on colony stress level (peaceful when thriving, tense during raids).

---

## 7. Campaign Progression

### 7.1 Map Structure

```
┌─────────────────────────────────────────┐
│           YARD (4x4 grid of patches)     │
│  ┌───┬───┬───┬───┐                      │
│  │ S │   │   │   │  S = Start patch      │
│  ├───┼───┼───┼───┤  R = Red colony HQ    │
│  │   │   │   │   │  H = House entrance   │
│  ├───┼───┼───┼───┤                      │
│  │   │   │ R │   │                      │
│  ├───┼───┼───┼───┤                      │
│  │   │   │   │ H │                      │
│  └───┴───┴───┴───┘                      │
│                                          │
│  ┌───────────────┐                      │
│  │   HOUSE        │                      │
│  │  (3x2 rooms)   │                      │
│  └───────────────┘                      │
└─────────────────────────────────────────┘
```

### 7.2 Expansion Mechanic

1. Colony reaches critical population in current patch
2. Player triggers mating flight (queen + drones produced)
3. Player chooses adjacent patch for new queen to land
4. New satellite colony established with small starter population
5. Player can switch active patch to manage any colony
6. Patches share no resources — each colony is self-sustaining

### 7.3 Difficulty Curve

| Phase | Patches | Challenges |
|---|---|---|
| Early | 1-3 | Basic foraging, simple red colony, spiders |
| Mid | 4-8 | Antlions, rain events, red raids, pesticide zones |
| Late | 9-12 | House interior (traps, poison bait), aggressive red AI |
| Endgame | House | Final red queen battle, exterminator events |

---

## 8. Technical Architecture

*See `ARCHITECTURE.md` for full Bevy ECS architecture and system diagrams.*

### 8.1 High-Level Module Map

```
colony/
├── src/
│   ├── main.rs                 # App entry, plugin registration
│   ├── plugins/
│   │   ├── simulation.rs       # Core sim tick, time management
│   │   ├── ant_ai.rs           # Ant state machines, behavior trees
│   │   ├── pheromone.rs        # Pheromone grid, diffusion, evaporation
│   │   ├── colony.rs           # Colony resources, caste management
│   │   ├── combat.rs           # Combat resolution systems
│   │   ├── nest.rs             # Underground nest, digging, chambers
│   │   ├── predators.rs        # Spider, antlion, human event AI
│   │   ├── terrain.rs          # Surface terrain, biomes, obstacles
│   │   ├── campaign.rs         # Map progression, patch management
│   │   └── environment.rs      # Weather, day/night, seasonal events
│   ├── components/
│   │   ├── ant.rs              # Ant, Caste, CarriedItem, AntState
│   │   ├── colony.rs           # Colony, FoodStorage, Population
│   │   ├── pheromone.rs        # PheromoneGrid, PheromoneType
│   │   ├── nest.rs             # Tunnel, Chamber, SoilType
│   │   ├── terrain.rs          # Tile, Biome, Obstacle
│   │   └── predator.rs         # Spider, Antlion, HumanEvent
│   ├── resources/
│   │   ├── simulation.rs       # SimConfig, SimClock
│   │   ├── colony_config.rs    # BehaviorSliders, CasteRatios
│   │   └── campaign_state.rs   # CampaignMap, PatchStatus
│   ├── events/
│   │   ├── combat.rs           # AttackEvent, DeathEvent
│   │   ├── colony.rs           # FoodDepositedEvent, EggLaidEvent
│   │   └── environment.rs      # RainEvent, PesticideEvent
│   └── ui/
│       ├── hud.rs              # Health, food, population display
│       ├── colony_panel.rs     # Behavior & caste sliders
│       ├── minimap.rs          # Patch overview
│       └── overlays.rs         # Pheromone visualization
├── assets/
│   ├── sprites/
│   ├── audio/
│   └── fonts/
└── Cargo.toml
```

### 8.2 Key ECS Design Decisions

- **Ants as Entities:** Each ant is an entity with `Transform`, `Ant`, `Caste`, `AntState`, `Health`, `Colony` components.
- **Pheromone Grid as Resource:** The pheromone field is a shared `Resource` (2D array), not per-entity — avoids millions of entities for grid cells.
- **Colony as Resource:** Colony-wide state (food stored, population counts, slider settings) stored as `Resource`, not component.
- **Systems organized by concern:** Each plugin registers its own systems into appropriate system sets (PreUpdate, Update, PostUpdate).
- **Events for cross-system communication:** Combat outcomes, food deposits, colony milestones communicated via Bevy Events.
- **Relationships (Bevy 0.16+):** Use ECS Relationships to link ants to their colony and brood to their chamber.

---

## 9. Scope and Milestones

### Milestone 1: Prototype (4 weeks)
- Single patch with surface + underground views
- Player-controlled yellow ant with movement and item pickup
- Basic ant AI (forage, return, idle)
- Pheromone grid with deposit/evaporate/diffuse
- Spawn food sources, basic collision

### Milestone 2: Colony Sim (4 weeks)
- Queen egg laying, brood development pipeline
- Caste system (workers, soldiers)
- Colony panel with behavior/caste sliders
- Underground nest digging and chamber designation
- Basic red colony opponent

### Milestone 3: Combat & Hazards (3 weeks)
- Combat system with stat resolution
- Spider and antlion predators
- Human hazard events (footsteps, lawnmower, rain)
- Alarm pheromone and defense behavior

### Milestone 4: Campaign (4 weeks)
- Multi-patch yard map
- Mating flight and satellite colony establishment
- House interior patches
- Campaign progression and victory conditions
- AI difficulty scaling

### Milestone 5: Polish (3 weeks)
- Art pass (sprites, animations, particle effects)
- Audio integration
- UI/UX polish, tutorials
- Balance tuning
- Performance optimization (target: 10,000 ants at 60fps)

---

## 10. References

- **SimAnt** (Maxis, 1991) — Primary inspiration for gameplay loop and mechanics
- **E.O. Wilson, "The Ants"** — Scientific reference for colony behavior and ecology
- **Ant Colony Optimization (Dorigo, 1992)** — Algorithmic basis for pheromone trail mechanics
- **Bevy Engine** (https://bevyengine.org) — ECS game engine in Rust
- **NetLogo Ant Foraging Model** — Reference implementation for pheromone diffusion/evaporation
