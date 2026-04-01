# Ant Behavior Patterns

## Biological Reference & Simulation Design

---

## 1. Stigmergy: Communication Without Direct Contact

Real ants coordinate through **stigmergy** — indirect communication by modifying the environment. The primary mechanism is chemical pheromone trails. No individual ant has a plan; colony-level intelligence emerges from thousands of ants each following local rules.

### Key Principle for Simulation

Each ant should only perceive its immediate neighborhood (8 surrounding cells). There is no global knowledge, no pathfinding, no central coordinator. All coordination emerges from pheromone gradients and simple state machines.

---

## 2. Pheromone Trail Mechanics

### 2.1 Two-Pheromone Foraging Model

Real ants use at least two pheromone types for foraging:

```
NEST ──────────────────────────── FOOD
  │                                  │
  │  ◄── HOME PHEROMONE ◄──         │
  │      (deposited by outbound     │
  │       ants leaving nest)         │
  │                                  │
  │     ── FOOD PHEROMONE ──►       │
  │      (deposited by returning    │
  │       ants carrying food)        │
  │                                  │
```

**Outbound ants** (leaving nest, searching for food):
- Deposit HOME pheromone as they walk
- Follow FOOD pheromone gradient uphill (toward food)
- If no FOOD pheromone detected, random walk with slight bias away from HOME pheromone

**Returning ants** (carrying food, heading home):
- Deposit FOOD pheromone as they walk
- Follow HOME pheromone gradient uphill (toward nest)
- FOOD pheromone intensity is proportional to food source quality

### 2.2 Positive Feedback Loop

```
  More ants        Stronger         More ants
  use trail  ───►  pheromone  ───►  attracted  ───┐
                   on trail         to trail       │
       ▲                                           │
       └───────────────────────────────────────────┘
```

This creates the characteristic "ant highway" effect where a few initial random discoveries consolidate into optimized routes.

### 2.3 Evaporation as Negative Feedback

Without evaporation, the first trail found would dominate forever, even if suboptimal. Evaporation provides negative feedback:

- Longer trails have weaker pheromone (more time to evaporate before reinforcement)
- Abandoned trails fade, freeing ants to discover new routes
- Shorter trails get reinforced more frequently and dominate

**Tuning Parameters:**

| Parameter | Low Value Effect | High Value Effect |
|---|---|---|
| Evaporation rate | Stable trails, slow adaptation | Volatile trails, fast adaptation |
| Diffusion rate | Sharp narrow trails | Broad fuzzy trails |
| Deposit amount | Weak signals, more exploration | Strong signals, fast convergence |
| Exploration noise | Ants stick to trails rigidly | Ants wander frequently |

### 2.4 Pheromone Grid Update Algorithm

Each simulation tick, for every cell (x, y) in the pheromone grid:

```
for each pheromone_type:
    # Evaporation
    grid[x][y] *= (1.0 - evaporation_rate)

    # Diffusion (spread to neighbors)
    total_neighbor = sum of grid[nx][ny] for all 8 neighbors
    average_neighbor = total_neighbor / 8.0
    grid[x][y] += diffusion_rate * (average_neighbor - grid[x][y])

    # Clamp
    grid[x][y] = clamp(grid[x][y], 0.0, max_intensity)
```

---

## 3. Individual Ant Behavior (Finite State Machine)

### 3.1 State Definitions

```
┌─────────────────────────────────────────────────────────────────────┐
│                       ANT STATE MACHINE                             │
│                                                                     │
│  ┌──────────┐                                                      │
│  │  IDLE    │  Ant is in nest, no task assigned                     │
│  │          │  → Check colony needs, transition to assigned role    │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ FORAGE   │  Ant is on surface, searching for food               │
│  │          │  → Follow food pheromone or random walk               │
│  │          │  → Deposit home pheromone                             │
│  │          │  → On finding food: pick up, transition to RETURN    │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ RETURN   │  Ant is carrying food, heading to nest               │
│  │          │  → Follow home pheromone gradient                     │
│  │          │  → Deposit food pheromone                             │
│  │          │  → On reaching nest: deposit food, transition IDLE   │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ NURSE    │  Ant is in nest, caring for brood                    │
│  │          │  → Move to brood chamber                              │
│  │          │  → Feed larvae from food storage                      │
│  │          │  → Groom eggs and pupae                               │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │  DIG     │  Ant is underground, excavating tunnels              │
│  │          │  → Move to designated dig zone                        │
│  │          │  → Remove soil cells, creating tunnel                 │
│  │          │  → Carry soil to surface (midden)                     │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ DEFEND   │  Ant is patrolling near nest entrance                │
│  │          │  → Wander near nest entrance                          │
│  │          │  → Attack any enemy ant in range                      │
│  │          │  → Emit alarm pheromone on enemy contact              │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ FIGHT    │  Ant is engaged in combat                            │
│  │          │  → Attack adjacent enemy each tick                    │
│  │          │  → Flee if HP < 20% and outnumbered                  │
│  │          │  → Victory: return to previous state                  │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ FLEE     │  Ant is retreating from danger                       │
│  │          │  → Move away from alarm pheromone source              │
│  │          │  → Head toward nest entrance                          │
│  │          │  → Transition to IDLE on reaching nest                │
│  └──────────┘                                                      │
│                                                                     │
│  ┌──────────┐                                                      │
│  │ FOLLOW   │  Ant is following the player-controlled ant          │
│  │          │  → Stay within 3 tiles of player ant                  │
│  │          │  → Attack what player attacks                         │
│  │          │  → Dismissed: return to IDLE                          │
│  └──────────┘                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 State Transition Rules

```
     hunger check           alarm pheromone
     (periodic)             detected
         │                      │
         ▼                      ▼
┌──────────────────────────────────────────────────────────┐
│                                                          │
│   IDLE ───────► FORAGE ──────► RETURN ──────► IDLE      │
│    │             │  ▲            │                        │
│    │             │  │            │                        │
│    │             │  └─ no food ──┘                        │
│    │             │   (timeout)                            │
│    │             │                                        │
│    │             ├──────────────► FIGHT ──────► (prev)    │
│    │             │  enemy contact                         │
│    │             │                                        │
│    ├───► NURSE   │                                        │
│    │  (assigned) │                                        │
│    │             │                                        │
│    ├───► DIG     │                                        │
│    │  (assigned) │                                        │
│    │             │                                        │
│    ├───► DEFEND  │                                        │
│    │  (assigned) │                                        │
│    │             │                                        │
│    └───► FOLLOW  │                                        │
│       (recruited)│                                        │
│                  │                                        │
│         ANY ─────┴──► FIGHT (alarm + defense role)       │
│         ANY ─────────► FLEE  (HP < 20%, outnumbered)     │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### 3.3 Role Assignment Algorithm

Each tick, the colony management system checks behavior sliders and assigns roles:

```
for each idle ant in colony:
    roll = random(0.0 .. 1.0)

    if roll < slider.forage:
        assign FORAGE
    elif roll < slider.forage + slider.nurse:
        assign NURSE
    elif roll < slider.forage + slider.nurse + slider.dig:
        assign DIG
    else:
        assign DEFEND
```

Ants already in a role do not get reassigned unless they return to IDLE. This provides stability while allowing gradual reallocation.

---

## 4. Temporal Polyethism (Age-Based Task Allocation)

Real ant colonies exhibit age-based division of labor. Young ants stay inside the nest; older ants venture outside. This can be simulated:

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│  Age 0-20%    │  NURSE (brood care, deep nest)      │
│               │  Stays underground                   │
│               │  Low aggression                      │
│               │                                      │
│  Age 20-40%   │  DIG (tunnel excavation)            │
│               │  Works at nest perimeter             │
│               │  Medium aggression                   │
│               │                                      │
│  Age 40-70%   │  FORAGE (food gathering)            │
│               │  Ventures to surface                 │
│               │  High aggression                     │
│               │                                      │
│  Age 70-90%   │  DEFEND (patrol, combat)            │
│               │  Guards nest entrance                │
│               │  Maximum aggression                  │
│               │                                      │
│  Age 90-100%  │  EXPENDABLE (risky tasks)           │
│               │  Sent on long-range scouting        │
│               │  First responders to threats         │
│               │                                      │
└──────────────────────────────────────────────────────┘
```

**Implementation:** Each ant has an `age` field (0.0 to 1.0, where 1.0 = death). The behavior slider weights are multiplied by age-based modifiers:

```rust
fn age_modifier(age: f32, role: Role) -> f32 {
    match role {
        Role::Nurse   => if age < 0.2 { 2.0 } else { 0.3 },
        Role::Dig     => if age < 0.4 { 1.5 } else { 0.5 },
        Role::Forage  => if age > 0.4 && age < 0.7 { 1.5 } else { 0.5 },
        Role::Defend  => if age > 0.7 { 2.0 } else { 0.3 },
    }
}
```

---

## 5. Foraging Behavior Detail

### 5.1 Foraging Decision Tree

```
                    ┌──────────────────┐
                    │ Ant is FORAGING   │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │ Sense food        │
              ┌─────┤ pheromone in     ├─────┐
              │     │ neighborhood?     │     │
             YES    └──────────────────┘    NO
              │                              │
              ▼                              ▼
    ┌──────────────────┐          ┌──────────────────┐
    │ Weight directions │          │ Random walk      │
    │ by pheromone      │          │ (biased away     │
    │ intensity^alpha   │          │  from home       │
    └────────┬─────────┘          │  pheromone)      │
             │                    └────────┬─────────┘
             ▼                             │
    ┌──────────────────┐                   │
    │ Add exploration   │◄──────────────────┘
    │ noise (random     │
    │ perturbation)     │
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │ Move one cell in  │
    │ selected direction│
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │ Deposit HOME      │
    │ pheromone at      │
    │ current position  │
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐       ┌──────────────────┐
    │ Food at current   │──YES─►│ Pick up food,    │
    │ position?         │       │ → RETURN state   │
    └────────┬─────────┘       └──────────────────┘
            NO
             │
             ▼
    ┌──────────────────┐       ┌──────────────────┐
    │ Forage timeout    │──YES─►│ Return to nest,  │
    │ exceeded?         │       │ → IDLE state     │
    └────────┬─────────┘       └──────────────────┘
            NO
             │
             └──── loop back to sense step
```

### 5.2 Return Behavior

```
                    ┌──────────────────┐
                    │ Ant is RETURNING  │
                    │ (carrying food)   │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │ Sense HOME        │
              ┌─────┤ pheromone in     ├─────┐
              │     │ neighborhood?     │     │
             YES    └──────────────────┘    NO
              │                              │
              ▼                              ▼
    ┌──────────────────┐          ┌──────────────────┐
    │ Move uphill on   │          │ Random walk      │
    │ HOME pheromone   │          │ (biased toward   │
    │ gradient         │          │  last known      │
    └────────┬─────────┘          │  nest direction) │
             │                    └────────┬─────────┘
             ▼                             │
    ┌──────────────────┐◄──────────────────┘
    │ Deposit FOOD      │
    │ pheromone at      │
    │ current position  │
    │ (intensity ∝      │
    │  food quality)    │
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐       ┌──────────────────┐
    │ At nest entrance? │──YES─►│ Enter nest,      │
    │                   │       │ deposit food,    │
    └────────┬─────────┘       │ → IDLE state     │
            NO                  └──────────────────┘
             │
             └──── loop
```

---

## 6. Combat Behavior Detail

### 6.1 Engagement Rules

```
┌─────────────────────────────────────────────────────┐
│                COMBAT ENGAGEMENT                     │
│                                                      │
│  Trigger Conditions:                                 │
│  1. Enemy ant enters adjacent cell                   │
│  2. Alarm pheromone detected (defenders respond)     │
│  3. Player initiates attack (Space key)              │
│                                                      │
│  Per-Tick Resolution:                                │
│                                                      │
│  damage = attacker.attack                            │
│         + random(-1, 1)                              │
│         + group_bonus                                │
│         + terrain_bonus                              │
│                                                      │
│  group_bonus = 0.1 * min(allies_adjacent, 5)        │
│              * attacker.attack                       │
│                                                      │
│  terrain_bonus:                                      │
│    Defending nest entrance: +50% damage              │
│    Uphill position: +10% damage                      │
│    In enemy territory: -10% damage                   │
│                                                      │
│  defender.hp -= damage                               │
│  if defender.hp <= 0: emit AntDiedEvent              │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### 6.2 Alarm Pheromone Cascade

When combat begins, the fighting ant emits alarm pheromone. This triggers a cascade:

```
  Combat starts at position (x, y)
              │
              ▼
  Emit alarm pheromone at (x, y)
              │
              ▼
  Alarm diffuses to neighboring cells
              │
              ▼
  Nearby DEFEND-role ants detect alarm
              │
              ▼
  Defenders switch to FIGHT state
  and move toward alarm source
              │
              ▼
  Arriving defenders emit MORE alarm
  (reinforcement cascade)
              │
              ▼
  Escalation continues until:
  - All enemies defeated
  - All defenders defeated
  - Alarm evaporates (timeout)
```

---

## 7. Nest Construction Behavior

### 7.1 Digging Algorithm

```
┌──────────────────────────────────────────────────┐
│              NEST DIGGING BEHAVIOR                │
│                                                   │
│  Ant in DIG state:                               │
│                                                   │
│  1. Check for designated dig zones               │
│     (player-marked or auto-generated)             │
│                                                   │
│  2. Move to nearest uncompleted dig zone          │
│                                                   │
│  3. At dig face:                                  │
│     - Check soil type                             │
│       Soft:  1 tick to excavate                   │
│       Clay:  3 ticks to excavate                  │
│       Rock:  impassable                           │
│                                                   │
│  4. Remove soil cell → becomes tunnel             │
│                                                   │
│  5. Pick up soil particle                         │
│                                                   │
│  6. Carry soil to midden (surface exit)           │
│                                                   │
│  7. Return to dig zone → repeat                   │
│                                                   │
│  Auto-expansion triggers:                         │
│  - Brood chamber >80% full → dig new chamber     │
│  - Food storage >80% full → dig new storage      │
│  - Population > capacity → expand tunnels         │
│                                                   │
└──────────────────────────────────────────────────┘
```

### 7.2 Chamber Types and Placement Rules

```
Surface
═══════════════════════════════════════
        │ entrance │
        └────┬─────┘
             │
    ┌────────┴────────┐
    │    Tunnel        │
    └────────┬────────┘
             │
   ┌─────────┴─────────┐
   │                    │
   ▼                    ▼
┌──────────┐      ┌──────────┐
│  Food    │      │  Brood   │
│  Storage │      │  Chamber │
│  (near   │      │  (warm,  │
│  surface)│      │  middle  │
└──────────┘      │  depth)  │
                  └──────────┘
                       │
              ┌────────┴────────┐
              │    Tunnel        │
              └────────┬────────┘
                       │
                       ▼
              ┌──────────────┐
              │    Queen     │
              │    Chamber   │
              │    (deepest, │
              │    protected)│
              └──────────────┘
                       │
              ┌────────┴────────┐
              │    Tunnel        │
              └────────┬────────┘
                       │
                       ▼
              ┌──────────────┐
              │    Midden    │
              │    (waste,   │
              │    far from  │
              │    brood)    │
              └──────────────┘

Placement Rules:
  - Queen chamber: deepest viable position
  - Brood chambers: middle depth, near queen
  - Food storage: near surface for quick forager access
  - Midden: far from brood (disease prevention)
  - Tunnels: minimum width 1 cell, branch factor ≤ 3
  - Structural: no chamber wider than 5 cells without support column
```

---

## 8. Colony-Level Emergent Behaviors

These behaviors are NOT explicitly programmed — they emerge from individual ant rules interacting:

### 8.1 Trail Optimization

```
Initial state:          After 100 ticks:        After 500 ticks:
(random exploration)    (trails forming)        (optimal path found)

N · · · · · · F         N · · · · · · F         N ─ ─ ─ ─ ─ ─ F
· * · · · · · ·         · ═ · · · · · ·         · · · · · · · ·
· · * · * · · ·         · · ═ · · · · ·         · · · · · · · ·
· · · * · · · ·         · · · ═ ═ · · ·         · · · · · · · ·
· · · · * · · ·         · · · · · ═ · ·         · · · · · · · ·
· · * · · * · ·         · · · · · · ═ ·         · · · · · · · ·

N = Nest    F = Food    * = wandering ant
═ = forming trail       ─ = established highway
```

### 8.2 Dynamic Reallocation

When a food source depletes, the trail evaporates naturally. Foragers finding no food at the trail's end begin random walking again, eventually discovering new sources and forming new trails.

### 8.3 Mass Recruitment

When a large food source is found (dead insect), the returning ant deposits extra-strong food pheromone. This recruits more ants faster, enabling the colony to harvest before competitors arrive.

### 8.4 Adaptive Defense

Alarm pheromone near the nest entrance causes more ants to transition to DEFEND. Once the threat passes, alarm evaporates and ants return to productive tasks. No explicit "threat level" variable needed.

---

## 9. Red Colony AI Strategy Layer

While individual red ants follow the same behavior rules as black ants, the red colony has a strategy layer that adjusts behavior sliders:

```
┌──────────────────────────────────────────────────────┐
│           RED COLONY STRATEGY AI                      │
│                                                       │
│  Evaluate every 30 seconds:                          │
│                                                       │
│  IF population < 50:                                  │
│     forage=0.6, nurse=0.3, dig=0.05, defend=0.05     │
│     (survival mode: grow the colony)                  │
│                                                       │
│  IF population > black_population * 1.5:             │
│     forage=0.3, nurse=0.1, dig=0.1, defend=0.5       │
│     (raid mode: attack while advantaged)              │
│                                                       │
│  IF queen.hp < queen.max_hp * 0.5:                   │
│     forage=0.1, nurse=0.1, dig=0.0, defend=0.8       │
│     (desperate defense: protect the queen)            │
│                                                       │
│  IF food_storage > 200:                              │
│     Increase soldier caste ratio to 40%              │
│     (militarize when resources allow)                 │
│                                                       │
│  IF territory_cells < black_territory * 0.5:         │
│     Send scout groups to claim new territory          │
│     (expansion when falling behind)                   │
│                                                       │
│  Raid Execution:                                      │
│     1. Accumulate 20+ soldiers near border            │
│     2. Lay trail pheromone toward black nest          │
│     3. All soldiers follow trail simultaneously       │
│     4. Target: kill workers, steal food               │
│                                                       │
└──────────────────────────────────────────────────────┘
```

---

## 10. Parameter Reference

### 10.1 Simulation Constants

| Parameter | Default | Range | Description |
|---|---|---|---|
| `PHEROMONE_EVAP_RATE` | 0.02 | 0.001-0.1 | Fraction decayed per tick |
| `PHEROMONE_DIFFUSE_RATE` | 0.005 | 0.001-0.05 | Fraction spread per tick |
| `PHEROMONE_DEPOSIT_AMOUNT` | 1.0 | 0.1-5.0 | Amount deposited per ant per tick |
| `PHEROMONE_MAX_INTENSITY` | 100.0 | 10-1000 | Ceiling clamp value |
| `EXPLORATION_NOISE` | 0.15 | 0.0-0.5 | Random direction perturbation |
| `ALPHA` (pheromone weight) | 2.0 | 1.0-5.0 | Exponent for pheromone influence |
| `BETA` (distance weight) | 1.0 | 0.5-3.0 | Exponent for distance influence |
| `ANT_SPEED_WORKER` | 5.0 | 1-10 | Tiles per second |
| `ANT_SPEED_SOLDIER` | 3.0 | 1-10 | Tiles per second |
| `ANT_LIFESPAN_WORKER` | 300.0 | 60-600 | Seconds |
| `ANT_LIFESPAN_QUEEN` | 1200.0 | 300-3600 | Seconds |
| `EGG_HATCH_TIME` | 30.0 | 10-60 | Seconds |
| `LARVA_GROW_TIME` | 45.0 | 15-90 | Seconds |
| `PUPA_DEVELOP_TIME` | 30.0 | 10-60 | Seconds |
| `QUEEN_EGG_RATE` | 0.1 | 0.01-1.0 | Eggs per second (if fed) |
| `HUNGER_RATE` | 0.01 | 0.001-0.05 | Hunger increase per tick |
| `RAIN_EVAP_MULTIPLIER` | 10.0 | 2-20 | Evaporation rate during rain |
| `FOOD_CRUMB_VALUE` | 5 | 1-20 | Food units per crumb |
| `FOOD_INSECT_VALUE` | 20 | 10-50 | Food units per dead insect |
| `FOOD_FRUIT_VALUE` | 50 | 20-100 | Food units per fruit source |

### 10.2 Recommended Starting Tuning

For a fun gameplay loop, start with these biases:

- High `EXPLORATION_NOISE` (0.2) — ants look lively and natural
- Moderate `EVAP_RATE` (0.02) — trails last ~50 seconds without reinforcement
- Low `DIFFUSE_RATE` (0.005) — trails stay visible and distinct
- High `ALPHA` (2.0) — strong pheromone trails are hard to ignore
- Short lifespans — keeps the colony dynamic, always producing new ants

Adjust based on playtesting. The sandbox mode should expose all parameters for live tuning.
