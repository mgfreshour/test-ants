//! AI decision/state logging.
//!
//! Writes NDJSON to `logs/ai-<unix_ts>.ndjson`. Three event kinds:
//! - `transition`: emitted when an ant's tracked snapshot changes.
//! - `aggregate`: once per sim-second colony-wide state/job histogram.
//! - `thrash_dump`: per-ant ring buffer, emitted only when an ant
//!   changes state more than once per second for two consecutive seconds.
//!
//! Query recipes:
//! - `jq 'select(.event=="transition" and .ant==42)' logs/ai-*.ndjson`
//! - `jq 'select(.event=="thrash_dump")' logs/ai-*.ndjson`
//! - `duckdb -c "SELECT event, COUNT(*) FROM read_json_auto('logs/*.ndjson') GROUP BY event"`
//!
//! Schema invariants:
//! - `nest_task` is non-null only when `on_surface == false`. Surface ants
//!   never carry a meaningful nest task, so surface snapshots always report
//!   `nest_task: null` even if a stale `NestTask` component is still attached.
//!
//! Disable without removing code: set [`AI_LOG_ENABLED`] to `false`.

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use serde_json::{json, Value};

use crate::components::ant::{Ant, AntJob, AntState, CarriedItem};
use crate::components::map::MapId;
use crate::components::nest::NestTask;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::ai_log::{diff, AntSnapshot, ThrashTracker};

/// Master switch for AI decision logging.
///
/// Set to `false` to disable all file I/O, per-ant tracking, and
/// ring-buffer sampling without removing any code. The `AiLogPlugin`
/// short-circuits in its builder and each system guards itself, so
/// nothing runs and no file is created.
pub const AI_LOG_ENABLED: bool = true;

/// How often the ring buffer samples a frame.
const TRACE_SAMPLE_EVERY_N_FRAMES: u8 = 2;

/// Ring buffer capacity (frames).
const TRACE_CAPACITY: usize = 64;

pub struct AiLogPlugin;

impl Plugin for AiLogPlugin {
    fn build(&self, app: &mut App) {
        if !AI_LOG_ENABLED {
            return;
        }

        let writer = match AiLogWriter::new() {
            Ok(w) => w,
            Err(e) => {
                warn!("AiLogPlugin disabled: failed to open log file: {e}");
                return;
            }
        };

        app.insert_resource(writer).add_systems(
            Update,
            (
                auto_insert_ai_log_components,
                sample_ai_trace,
                detect_ai_transitions,
                detect_thrash,
                aggregate_ai_stats,
            )
                .chain(),
        );
    }
}

#[derive(Component)]
pub struct AiLogState {
    pub prev: Option<AntSnapshot>,
    pub tracker: ThrashTracker,
    pub frames_since_sample: u8,
}

impl Default for AiLogState {
    fn default() -> Self {
        Self {
            prev: None,
            tracker: ThrashTracker::new(),
            frames_since_sample: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TraceFrame {
    pub tick: u64,
    pub elapsed: f32,
    pub snapshot: AntSnapshot,
    pub pos: Vec2,
    pub hunger: f32,
}

#[derive(Component)]
pub struct AiTrace {
    buf: [Option<TraceFrame>; TRACE_CAPACITY],
    head: usize,
    count: usize,
}

impl Default for AiTrace {
    fn default() -> Self {
        Self {
            buf: [None; TRACE_CAPACITY],
            head: 0,
            count: 0,
        }
    }
}

impl AiTrace {
    pub fn push(&mut self, frame: TraceFrame) {
        self.buf[self.head] = Some(frame);
        self.head = (self.head + 1) % TRACE_CAPACITY;
        if self.count < TRACE_CAPACITY {
            self.count += 1;
        }
    }

    /// Returns frames in chronological (oldest -> newest) order.
    pub fn frames(&self) -> Vec<TraceFrame> {
        let mut out = Vec::with_capacity(self.count);
        if self.count < TRACE_CAPACITY {
            for i in 0..self.count {
                if let Some(f) = self.buf[i] {
                    out.push(f);
                }
            }
        } else {
            for i in 0..TRACE_CAPACITY {
                let idx = (self.head + i) % TRACE_CAPACITY;
                if let Some(f) = self.buf[idx] {
                    out.push(f);
                }
            }
        }
        out
    }
}

#[derive(Resource)]
pub struct AiLogWriter {
    file: BufWriter<File>,
    transitions_in_second: u32,
    last_aggregate_elapsed: f32,
}

impl AiLogWriter {
    fn new() -> std::io::Result<Self> {
        create_dir_all("logs")?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = format!("logs/ai-{ts}.ndjson");
        let file = File::create(&path)?;
        info!("AI log writing to {path}");
        Ok(Self {
            file: BufWriter::new(file),
            transitions_in_second: 0,
            last_aggregate_elapsed: 0.0,
        })
    }

    fn write_line(&mut self, value: &Value) {
        if let Err(e) = writeln!(self.file, "{value}") {
            warn!("ai_log write failed: {e}");
        }
    }

    fn flush(&mut self) {
        let _ = self.file.flush();
    }
}

fn snapshot_for(
    state: AntState,
    job: AntJob,
    carrying: bool,
    on_surface: bool,
    nest_task: Option<&NestTask>,
) -> AntSnapshot {
    // Surface ants never execute a nest task; suppress any leaked NestTask
    // component from the snapshot so transition diffs and thrash detection
    // aren't polluted by portal-transition command-flush ordering.
    let nest_task_label = if on_surface {
        None
    } else {
        nest_task.map(|t| t.label())
    };
    AntSnapshot {
        state,
        job,
        carrying,
        on_surface,
        nest_task: nest_task_label,
    }
}

fn snapshot_to_json(s: &AntSnapshot) -> Value {
    json!({
        "state": format!("{:?}", s.state),
        "job": format!("{:?}", s.job),
        "carrying": s.carrying,
        "on_surface": s.on_surface,
        "nest_task": s.nest_task,
    })
}

/// Auto-attach `AiLogState` and `AiTrace` to any ant missing them.
/// Ants are spawned from many plugins; this keeps the wiring in one place.
fn auto_insert_ai_log_components(
    mut commands: Commands,
    query: Query<Entity, (With<Ant>, Without<AiLogState>)>,
) {
    if !AI_LOG_ENABLED {
        return;
    }
    for entity in &query {
        commands
            .entity(entity)
            .insert((AiLogState::default(), AiTrace::default()));
    }
}

fn sample_ai_trace(
    clock: Res<SimClock>,
    registry: Option<Res<MapRegistry>>,
    mut query: Query<(
        &Transform,
        &Ant,
        Option<&AntJob>,
        Option<&CarriedItem>,
        Option<&NestTask>,
        &MapId,
        &mut AiLogState,
        &mut AiTrace,
    )>,
) {
    if !AI_LOG_ENABLED || clock.speed == SimSpeed::Paused {
        return;
    }
    let surface_entity = registry.as_ref().map(|r| r.surface);

    for (tf, ant, job_opt, carried, nest_task, map_id, mut log_state, mut trace) in &mut query {
        log_state.frames_since_sample = log_state.frames_since_sample.saturating_add(1);
        if log_state.frames_since_sample < TRACE_SAMPLE_EVERY_N_FRAMES {
            continue;
        }
        log_state.frames_since_sample = 0;

        let on_surface = surface_entity.map_or(true, |s| map_id.0 == s);
        let snap = snapshot_for(
            ant.state,
            job_opt.copied().unwrap_or(AntJob::Unassigned),
            carried.is_some(),
            on_surface,
            nest_task,
        );

        trace.push(TraceFrame {
            tick: clock.tick,
            elapsed: clock.elapsed,
            snapshot: snap,
            pos: tf.translation.truncate(),
            hunger: ant.hunger,
        });
    }
}

fn detect_ai_transitions(
    clock: Res<SimClock>,
    registry: Option<Res<MapRegistry>>,
    mut writer: ResMut<AiLogWriter>,
    mut query: Query<(
        Entity,
        &Transform,
        &Ant,
        Option<&AntJob>,
        Option<&CarriedItem>,
        Option<&NestTask>,
        &MapId,
        &mut AiLogState,
    )>,
) {
    if !AI_LOG_ENABLED || clock.speed == SimSpeed::Paused {
        return;
    }
    let surface_entity = registry.as_ref().map(|r| r.surface);

    for (entity, tf, ant, job_opt, carried, nest_task, map_id, mut log_state) in &mut query {
        let on_surface = surface_entity.map_or(true, |s| map_id.0 == s);
        let curr = snapshot_for(
            ant.state,
            job_opt.copied().unwrap_or(AntJob::Unassigned),
            carried.is_some(),
            on_surface,
            nest_task,
        );

        match log_state.prev {
            None => {
                log_state.prev = Some(curr);
            }
            Some(prev) => {
                if let Some(record) = diff(prev, curr) {
                    log_state.tracker.record_change();
                    writer.transitions_in_second += 1;
                    let line = json!({
                        "ts": clock.elapsed,
                        "tick": clock.tick,
                        "ant": entity.index_u32(),
                        "event": "transition",
                        "from": snapshot_to_json(&record.from),
                        "to": snapshot_to_json(&record.to),
                        "pos": [tf.translation.x, tf.translation.y],
                        "hunger": ant.hunger,
                    });
                    writer.write_line(&line);
                    log_state.prev = Some(curr);
                }
            }
        }
    }
}

fn detect_thrash(
    clock: Res<SimClock>,
    mut writer: ResMut<AiLogWriter>,
    mut query: Query<(Entity, &mut AiLogState, &AiTrace)>,
) {
    if !AI_LOG_ENABLED || clock.speed == SimSpeed::Paused {
        return;
    }
    for (entity, mut log_state, trace) in &mut query {
        let result = log_state.tracker.tick(clock.elapsed);
        if !result.should_dump {
            continue;
        }

        let frames: Vec<Value> = trace
            .frames()
            .into_iter()
            .map(|f| {
                json!({
                    "tick": f.tick,
                    "ts": f.elapsed,
                    "snapshot": snapshot_to_json(&f.snapshot),
                    "pos": [f.pos.x, f.pos.y],
                    "hunger": f.hunger,
                })
            })
            .collect();

        let line = json!({
            "ts": clock.elapsed,
            "tick": clock.tick,
            "ant": entity.index_u32(),
            "event": "thrash_dump",
            "window_sec": crate::sim_core::ai_log::THRASH_BUCKET_SECS
                * crate::sim_core::ai_log::THRASH_TRIGGER_WINDOWS as f32,
            "frames": frames,
        });
        writer.write_line(&line);
    }
}

fn aggregate_ai_stats(
    clock: Res<SimClock>,
    mut writer: ResMut<AiLogWriter>,
    query: Query<(&Ant, Option<&AntJob>)>,
) {
    if !AI_LOG_ENABLED || clock.speed == SimSpeed::Paused {
        return;
    }
    if clock.elapsed - writer.last_aggregate_elapsed < 1.0 {
        return;
    }

    let mut by_state: HashMap<&'static str, u32> = HashMap::new();
    let mut by_job: HashMap<&'static str, u32> = HashMap::new();
    let mut total: u32 = 0;

    for (ant, job_opt) in &query {
        total += 1;
        *by_state.entry(state_label(ant.state)).or_insert(0) += 1;
        let job = job_opt.copied().unwrap_or(AntJob::Unassigned);
        *by_job.entry(job_label(job)).or_insert(0) += 1;
    }

    let line = json!({
        "ts": clock.elapsed,
        "tick": clock.tick,
        "event": "aggregate",
        "total": total,
        "by_state": by_state,
        "by_job": by_job,
        "transitions_last_sec": writer.transitions_in_second,
    });
    writer.write_line(&line);
    writer.transitions_in_second = 0;
    writer.last_aggregate_elapsed = clock.elapsed;
    writer.flush();
}

fn state_label(s: AntState) -> &'static str {
    match s {
        AntState::Idle => "Idle",
        AntState::Foraging => "Foraging",
        AntState::Returning => "Returning",
        AntState::Defending => "Defending",
        AntState::Fighting => "Fighting",
        AntState::Fleeing => "Fleeing",
        AntState::Following => "Following",
        AntState::Attacking => "Attacking",
    }
}

fn job_label(j: AntJob) -> &'static str {
    match j {
        AntJob::Forager => "Forager",
        AntJob::Nurse => "Nurse",
        AntJob::Digger => "Digger",
        AntJob::Defender => "Defender",
        AntJob::Unassigned => "Unassigned",
    }
}
