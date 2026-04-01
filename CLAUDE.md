# Claude Code Context

## Feature Index

### App Entry and Wiring
- `src/main.rs`: App bootstrap, plugin registration order, window setup.
- `src/plugins/mod.rs`: Plugin module list.
- `src/components/mod.rs`: Component module exports.
- `src/resources/mod.rs`: Resource module exports.
- `src/ui/mod.rs`: UI module exports.

### Simulation and World
- `src/plugins/simulation.rs`: Sim clock, pause/speed controls.
- `src/plugins/terrain.rs`: Surface tile rendering, initial food spawn, periodic food drops.
- `src/resources/simulation.rs`: Core simulation configuration and clock state.
- `src/resources/active_map.rs`: Active map state and map view helpers.

### Surface Ant AI and Colony Loop
- `src/plugins/ant_ai.rs`: Surface ant spawning, foraging/returning behavior, hunger/starvation, pickup/dropoff flow, ant visuals.
- `src/components/ant.rs`: Ant state, movement, caste/follower/player-control related components.
- `src/resources/spatial_grid.rs`: Spatial indexing used by ant systems.
- `src/components/terrain.rs`: Food source data components.

### Nest, Maps, and Underground Systems
- `src/plugins/nest.rs`: Surface/nest map setup, portals, queen spawn, brood lifecycle, colony stats updates, map visibility.
- `src/plugins/nest_ai.rs`: Underground worker utility AI (nursing/digging/hauling/queen care), task execution, excavation, player dig designations.
- `src/plugins/nest_navigation.rs`: Nest path following, grid/world conversion, collision correction, debug path overlay.
- `src/resources/nest.rs`: Nest grid model, constants, dig/stack support resources.
- `src/resources/nest_pathfinding.rs`: Path cache and pathfinding helpers.
- `src/components/nest.rs`: Queen/brood/task/path/stacked-item related components.

### Pheromone Systems
- `src/plugins/pheromone.rs`: Surface pheromone simulation and overlay (`H` toggle).
- `src/plugins/nest_pheromone.rs`: Nest pheromone simulation (queen/brood/construction/chamber labels) and overlay (`N` toggle).
- `src/resources/pheromone.rs`: Surface pheromone grid/config structures.
- `src/resources/nest_pheromone.rs`: Nest pheromone grid/config/labels.
- `src/components/pheromone.rs`: Pheromone tile/type components.

### Player Controls, Camera, and Combat
- `src/plugins/player.rs`: Player ant control, follower recruitment/dismissal, manual pheromone trail, ant swapping, camera follow.
- `src/plugins/camera.rs`: Free camera pan/zoom/clamp behavior on surface.
- `src/plugins/combat.rs`: Enemy colony + spider, combat state transitions, damage/death, victory/defeat checks.
- `src/resources/colony.rs`: Colony-level stats/sliders/caste-ratio resources used by UI and nest logic.

### UI and UX
- `src/ui/hud.rs`: Main HUD text (mode, stats, controls, overlay, FPS).
- `src/ui/colony_panel.rs`: Colony behavior slider display and keyboard controls.
- `src/components/map.rs`: Map identity, map kinds, portal definitions.

### Project Notes
- `docs/IMPLEMENTATION_PLAN.md`: Feature implementation roadmap notes.
- `docs/NEST_AI_AND_NAVIGATION.md`: Deep-dive notes for nest AI/navigation.
- `docs/SIMULATION_TESTING_WORK_PLAN.md`: Simulation testing work plan.

## Testing Workflow

Before marking tasks as complete or declaring code changes finished:

1. **Build check**: Run `cargo build 2>&1` to verify the code compiles without errors
2. **Runtime validation**: Run the application with timeout to verify it starts without panics:
   ```bash
   timeout 3s cargo run 2>&1
   ```
   - Exit code 124 indicates successful timeout (app ran without panicking)
   - Any other exit code or panic output indicates a runtime error that must be fixed

This ensures both compile-time and runtime correctness before considering work complete.
