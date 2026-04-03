# Colony: Implementation Plan

## Sprint-Based Roadmap — Each Sprint Ends with a Demo

---

## Overview

22 two-week sprints. Each sprint produces a runnable build with visible new functionality. Sprints 1–11 built the core simulation (surface + underground). Sprints 12–18 unify the split ant system into a single job-based architecture. Sprints 19–22 add gameplay modes and polish.

```
Sprint  1   █░░░░░░░░░░░░░░░░░░░░░  Wandering Ants ✓
Sprint  2   ██░░░░░░░░░░░░░░░░░░░░  Pheromone Trails ✓
Sprint  3   ███░░░░░░░░░░░░░░░░░░░  Foraging Loop ✓
Sprint  4   ████░░░░░░░░░░░░░░░░░░  Colony & Nest ✓
Sprint  5   █████░░░░░░░░░░░░░░░░░  Nest Pheromones & Pathfinding ✓
Sprint  6   ██████░░░░░░░░░░░░░░░░  Nest Ant AI ✓
Sprint  7   ███████░░░░░░░░░░░░░░░  Stigmergic Digging & Collision ✓
Sprint  8   ████████░░░░░░░░░░░░░░  Player Control ✓
Sprint  9   █████████░░░░░░░░░░░░░  Combat & Enemies ✓
Sprint 10   ██████████░░░░░░░░░░░░  Colony Management UI ✓
Sprint 11   ███████████░░░░░░░░░░░  Player HUD & Action Bar ✓
Sprint 12   ████████████░░░░░░░░░░  Spawn at Egg Location ✓
Sprint 13   ██████████████░░░░░░░░░  AntJob Component ✓
Sprint 14   ██████████████░░░░░░░░  Job-Driven Transitions ✓
Sprint 15   ███████████████░░░░░░░  Unified Steering ✓
Sprint 16   ████████████████░░░░░░  Split AI Files ✓
Sprint 17   █████████████████░░░░░  Unified AI Dispatch ✓
Sprint 18   ██████████████████░░░░  Cleanup Legacy Paths (partial)
Sprint 19   ███████████████████░░░  Environment & Hazards ✓
Sprint 20   ████████████████████░░  Quick Game Complete
Sprint 21   █████████████████████░  Campaign Mode
Sprint 22   ██████████████████████  Polish & Sandbox
```

---

## Completed Sprints (1–19) — Summary

| # | Sprint | Focus | Status |
|---|---|---|---|
| 1-5 | Foundation | Wandering, pheromones, foraging, nest setup, pathfinding | ✓ |
| 6-7 | Nest AI & Digging | Task system, transitions, excavation, collision | ✓ |
| 8-9 | Player & Combat | WASD control, followers, enemies, death/victory | ✓ |
| 10-11 | UI & HUD | egui panels, colony stats, action bar, minimap | ✓ |
| 12 | Spawn at Egg Loc | Brood hatches at pupa position, unified ant pool bootstrap | ✓ |
| 13 | AntJob Component | Job tagging, assignment system, age-based affinity | ✓ |
| 14 | Job-Driven Transitions | Portal entry/exit keyed to AntJob, ping-pong cooldown | ✓ |
| 15 | Unified Steering | Single steering system for both maps, obstacle hook ready | ✓ |
| 16 | Split AI Files | ant_ai/ & nest_ai/ modularized into focused domains | ✓ |
| 17 | Unified AI Dispatch | Systems read AntJob+MapId, NestTask becomes sub-task | ✓ |
| 18 | Cleanup Legacy | Removed AntState::Nursing/Digging, cleaned ColonyStats | ✓ |
| 19 | Environment & Hazards | Rain pheromone decay, flooding, footsteps, lawnmower, day/night | ✓ |

**Key Artifacts:**
- Ant lifecycle: spawn (brood) → hatch → job assign → transition (surface↔nest) → forage/nest-work → feed → age → die
- Job system: AntJob enum (Forager/Nurse/Digger/Defender/Unassigned) assigned dynamically by BehaviorSliders ratios
- Steering: unified `SteeringTarget` + `apply_steering` system for both maps; pure math in `sim_core/steering.rs`
- Nest AI: job-based task eligibility; tasks are sub-state within AntJob

---

---

## Sprint 20: Quick Game Complete (Weeks 39-40)

> *Formerly Sprint 13.*

### Goal
A fully playable Quick Game mode from main menu to victory/defeat screen. Balanced, fun, 10-15 minute play sessions. This is the first "real game" milestone.

### Tasks

| # | Task | Est |
|---|---|---|
| 20.1 | Main menu — title screen with "Quick Game", "Campaign" (grayed out), "Sandbox" (grayed out) buttons | 4h |
| 20.2 | Game state machine — Loading → MainMenu → InGame → Paused → GameOver. Bevy `States` with enter/exit systems | 4h |
| 20.3 | Pause menu — ESC pauses sim, shows Resume / Restart / Quit to Menu | 3h |
| 20.4 | Game over screen — Victory: "Colony Dominant!" with stats. Defeat: "Colony Lost" with cause | 3h |
| 20.5 | Quick game setup — balanced starting conditions. Black and red colony start equidistant | 3h |
| 20.6 | Balance tuning — playtest and adjust: ant stats, food spawn rates, red AI aggression curve, predator HP, event frequency. Target: competitive game winnable in 10-15 min | 8h |
| 20.7 | Red AI improvements — smarter raid timing, retreat behavior, expansion priority | 6h |
| 20.8 | Tutorial hints — first-time popups: "Press WASD to move", "Hold Shift to lay trail", "Press R to recruit", etc | 4h |
| 20.9 | End-to-end QA — test full game loop 10+ times. Fix crashes, softlocks, degenerate strategies | 6h |

### Demo
> Launch game. Title screen appears. Click "Quick Game". Player spawns as yellow ant. Tutorial hints guide first actions. Forage food, grow colony, clash with red ants, survive hazards. Red colony sends raids. Counter-attack with recruited soldiers. Kill the red queen — victory screen shows stats. Play again or quit to menu.

### Acceptance Criteria
- [ ] Full game loop: menu → play → win/lose → menu
- [x] Game is winnable and losable (victory/defeat detection exists)
- [ ] 10-15 minute play session feels complete
- [ ] No crashes or softlocks in 10 consecutive playthroughs
- [ ] Tutorial teaches core controls
- [ ] Red AI provides meaningful challenge

---

## Sprint 21: Campaign Mode (Weeks 41-42)

> *Formerly Sprint 14.*

### Goal
Multi-patch campaign where the player colonizes a 4x4 yard grid and a house. Mating flights establish satellite colonies. Difficulty escalates. The game has long-term progression and replayability.

### Tasks

| # | Task | Est |
|---|---|---|
| 21.1 | Campaign map screen — 4x4 grid of yard patches + house | 5h |
| 21.2 | Patch generation — procedurally varied terrain, food sources, hazards based on difficulty tier | 6h |
| 21.3 | Mating flight mechanic — trigger flight, choose target patch, establish new colony | 5h |
| 21.4 | Patch switching — switch active patch from campaign map, maintain simulation state per patch | 6h |
| 21.5 | Difficulty scaling — later patches have more red ants, more predators, more hazards | 4h |
| 21.6 | Red colony campaign AI — red colony also expands to adjacent patches | 5h |
| 21.7 | House interior gameplay — indoor hazards (poison bait, traps, exterminator), indoor food sources | 3h |
| 21.8 | Campaign victory/defeat — win at 70% house + red eliminated. Lose if all queens dead | 3h |
| 21.9 | Campaign save/load — serialize campaign state to disk, resume from main menu | 5h |

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

## Sprint 22: Visual Polish, Audio & Sandbox (Weeks 43-44)

> *Formerly Sprint 15. Now also absorbs all visual/audio polish deferred from earlier sprints.*

### Goal
All visual and audio polish consolidated here. Sandbox mode with full environmental controls. Art/audio pass across the entire game. Performance optimization. This is the release-candidate sprint.

### Tasks

| # | Task | Est |
|---|---|---|
| 22.1 | Sandbox mode — spawn from menu. Full controls: spawn food, place walls, paint pheromones, spawn/kill ants, control either colony | 6h |
| 22.2 | Sandbox parameter panel — expose all simulation constants with live-edit egui sliders | 4h |
| 22.3 | Data overlays for sandbox — pheromone heat maps, ant density map, population over time graph, foraging efficiency metric | 5h |
| 22.4 | **Art polish** — replace placeholder sprites with proper pixel art. Ant walk animations (6-8 frames). Food sprites. Terrain tiles. UI skinning | 8h |
| 22.5 | **Audio system** — integrate `bevy_audio`. Ambient outdoor soundscape, underground ambience. Combat clicks, food pickup chime, alarm tone. At least 8 sound effects | 6h |
| 22.6 | **Weather visual effects** *(deferred from Sprint 19)* — rain particles, darkened sky, puddle sprites, footstep shadow, mower sprite animation | 5h |
| 22.7 | **Combat visual effects** *(deferred from Sprint 9)* — hit flash, damage numbers, death particle burst | 4h |
| 22.8 | **Player visual feedback** *(deferred from Sprint 8)* — ant highlight glow, pheromone deposit particles, recruit radius indicator | 4h |
| 22.9 | **Day/night visual cycle** *(deferred from Sprint 19)* — subtle lighting shift, darkened sky at night, dawn/dusk gradients | 3h |
| 22.10 | **Nest excavation feedback** *(deferred from Sprint 7)* — excavated cells flash, soil particles on hauling ants | 2h |
| 22.11 | **House interior tileset** *(deferred from Sprint 21)* — indoor tiles (kitchen tile, carpet, wood), indoor food sources | 5h |
| 22.12 | Performance optimization — profile with 10K ants. Optimize pheromone grids (SIMD or compute shader). LOD for distant ants | 8h |
| 22.13 | Settings screen — resolution, fullscreen, volume sliders, control rebinding | 4h |
| 22.14 | Accessibility — colorblind mode for pheromone overlays, adjustable UI scale, key rebinding | 3h |
| 22.15 | Bug fix buffer — address bugs found in Sprint 20-21 playtesting | 6h |
| 22.16 | Final QA — full pass on all three modes (Quick, Campaign, Sandbox). Performance benchmarks on min-spec hardware | 5h |

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
| Pheromone grid performance bottleneck at scale | Medium | High | Profile early. Fallback: reduce grid resolution, GPU compute shader for diffusion. |
| Bevy breaking changes between versions | Medium | Medium | Pin Bevy version in `Cargo.toml`. Only upgrade between sprints. |
| Job assignment oscillation | Medium | Medium | Hysteresis band in pure logic; extensive unit tests (Sprint 13). |
| Ants stuck ping-ponging at portals | Medium | Medium | Cooldown timer per ant (Sprint 14); test with high speed multiplier. |
| Movement regression in unified steering | Medium | High | A/B comparison at each step; keep old systems until verified (Sprint 15). |
| Scope creep | High | Medium | Each sprint has a locked scope. Nice-to-haves deferred to post-release. |
| Art/audio takes longer than estimated | Medium | Low | Placeholder art is fine for all sprints until Sprint 22. Game is fully playable without polish. |

---

## Dependency Chain

```
Sprints 1-11 (COMPLETE) ──► Sprint 12 ──► Sprint 13 ──► Sprint 14 ◄──┐
         (surface +        (spawn at     (AntJob       (job-driven     │
          underground       egg loc)      component)    transitions) ──┤
          foundation)                                                   │
                                                                        │
                                         Sprint 15 ◄────────────────────┘
                                      (unified steering)
                                             │
                                             ▼
                                      Sprint 16 ──► Sprint 17 ──► Sprint 18
                                    (split AI      (unified AI    (cleanup)
                                     files)         dispatch)
                                                        │
                  ┌─────────────────────────────────────┘
                  │
                  ▼
          Sprint 19 ──► Sprint 20 ──► Sprint 21 ──► Sprint 22
        (environment)  (quick game)  (campaign)    (polish +
                                                     sandbox)

Sprints 12-14 can overlap/parallelize (12→13 sequential, 14 can start after 13, 15 independent).
Sprint 15 (steering) best done after 14 so path conversions are clean.
Sprint 16 (file split) best done after 15 to reflect final architecture.
Sprints 17-18 must be sequential.
Sprints 19-22 are gameplay/content, independent of refactor (19 already done).
```

Each sprint produces a demoable build because they stack vertically — Sprint N always works as a superset of Sprint N-1, never as an isolated branch that needs integration later.
