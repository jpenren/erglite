// SPDX-License-Identifier: MIT OR Apache-2.0
//! ErgLite Tauri shell: exposes the Rust engine to the UI through typed commands and events.
//!
//! The command and event names are the contract in `docs/specification.md` (section 13.4); the
//! TypeScript side lives in `app/ui/src/lib/backend/tauri.ts`.

use erglite_core::clock::{iso8601_utc, unix_now_s};
use erglite_core::engine::{Snapshot, Status};
use erglite_core::metrics::{SamplePoint, Summary};
use erglite_core::profile::Profile;
use erglite_core::workout::Workout;
use erglite_runtime::{RunnerEvent, SessionRunner, TickOutput};
use erglite_store::{SessionRow, Store};
use erglite_transport::{
    ConnectionState, DeviceInfo, SimConfig, SimulatedTrainer, TrainerBackend, TrainerCapabilities,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

const SIM_TRAINER_ID: &str = "sim-trainer";
const BUILTIN: [(&str, &str); 2] = [
    ("sweet-spot-3x8", include_str!("../../../fixtures/workouts/sweet-spot-3x8.json")),
    ("rolling-10km", include_str!("../../../fixtures/workouts/rolling-10km.json")),
];
const BUILTIN_ZWO: &str = include_str!("../../../fixtures/workouts/vo2max-4x4.zwo");

type Backend = Box<dyn TrainerBackend>;

struct Connected {
    backend: Backend,
}

enum Control {
    Pause,
    Resume,
    Skip,
    Intensity(f32),
    Difficulty(f32),
    Stop(oneshot::Sender<Result<Summary, String>>),
}

struct AppState {
    db_path: PathBuf,
    store: Mutex<Store>,
    trainer: tokio::sync::Mutex<Option<Connected>>,
    control: Mutex<Option<UnboundedSender<Control>>>,
    #[cfg(feature = "ble")]
    ble: Mutex<std::collections::HashMap<String, erglite_transport::ble::Peripheral>>,
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct WorkoutRecord {
    id: String,
    workout: Workout,
    source: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct StartOptions {
    difficulty: f32,
}

/// Payload of the `session:tick` event (mirrors `TickEvent` in `app/ui/src/lib/types.ts`).
#[derive(Serialize, Clone)]
struct TickDto {
    status: &'static str,
    segment_index: usize,
    segment_count: usize,
    elapsed_s: f64,
    distance_m: f64,
    route_pos_m: f64,
    segment_progress: f32,
    target_power_w: Option<u16>,
    grade_pct: Option<f32>,
    segment_remaining_s: Option<f64>,
    power_w: Option<u16>,
    cadence_rpm: Option<f32>,
    hr_bpm: Option<u8>,
    speed_kmh: Option<f32>,
    connection: &'static str,
    countdown_s: u32,
    notices: Vec<String>,
}

fn tick_dto(out: &TickOutput) -> TickDto {
    let s: &Snapshot = &out.snapshot;
    let t = out.telemetry.unwrap_or_default();
    TickDto {
        status: match s.status {
            Status::Idle => "idle",
            Status::Countdown => "countdown",
            Status::Running => "running",
            Status::Paused => "paused",
            Status::Finished => "finished",
        },
        segment_index: s.segment_index,
        segment_count: s.segment_count,
        elapsed_s: s.elapsed_s,
        distance_m: s.distance_m,
        route_pos_m: s.route_pos_m,
        segment_progress: s.segment_progress,
        target_power_w: s.target_power_w,
        grade_pct: s.grade_pct,
        segment_remaining_s: s.segment_remaining_s,
        power_w: t.power_w,
        cadence_rpm: t.cadence_rpm,
        hr_bpm: t.hr_bpm,
        speed_kmh: t.speed_kmh,
        connection: match out.connection {
            ConnectionState::Connected => "connected",
            ConnectionState::Connecting => "connecting",
            ConnectionState::Disconnected => "disconnected",
        },
        countdown_s: s.countdown_s,
        notices: out
            .events
            .iter()
            .map(|e| match e {
                RunnerEvent::Engine(ev) => format!("{ev:?}").to_lowercase(),
                RunnerEvent::ConnectionLost => "connection_lost".into(),
                RunnerEvent::ConnectionRestored => "connection_restored".into(),
                RunnerEvent::CommandFailed(m) => format!("command_failed: {m}"),
            })
            .collect(),
    }
}

fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock().map_err(|_| "internal lock poisoned".to_string())
}

#[tauri::command]
fn get_profile(state: State<'_, AppState>) -> Result<Profile, String> {
    lock(&state.store)?.profile().map_err(err)
}

#[tauri::command]
fn set_profile(state: State<'_, AppState>, profile: Profile) -> Result<(), String> {
    lock(&state.store)?.set_profile(&profile).map_err(err)
}

#[tauri::command]
fn list_workouts(state: State<'_, AppState>) -> Result<Vec<WorkoutRecord>, String> {
    let store = lock(&state.store)?;
    let mut out = Vec::new();
    for row in store.list_workouts().map_err(err)? {
        if let Some(workout) = store.workout(&row.id).map_err(err)? {
            out.push(WorkoutRecord {
                id: row.id,
                workout,
                source: row.source.unwrap_or_default(),
                updated_at: row.updated_at,
            });
        }
    }
    Ok(out)
}

#[tauri::command]
fn save_workout(state: State<'_, AppState>, id: Option<String>, workout: Workout, source: Option<String>) -> Result<String, String> {
    workout.validate().map_err(err)?;
    lock(&state.store)?
        .save_workout(id.as_deref(), &workout, source.as_deref().unwrap_or("user"))
        .map_err(err)
}

#[tauri::command]
fn delete_workout(state: State<'_, AppState>, id: String) -> Result<(), String> {
    lock(&state.store)?.delete_workout(&id).map_err(err)
}

#[tauri::command]
fn import_workout(state: State<'_, AppState>, file_name: String, text: String, simplify_every_m: Option<f64>) -> Result<WorkoutRecord, String> {
    let lower = file_name.to_lowercase();
    let (workout, source) = if lower.ends_with(".gpx") {
        let route = erglite_formats::import_gpx_route(&text, 100.0, 9, simplify_every_m).map_err(err)?;
        let name = file_name.trim_end_matches(".gpx").trim_end_matches(".GPX");
        (Workout::from_route(name, route), "gpx")
    } else {
        (erglite_formats::import_zwo(&text).map_err(err)?.workout, "zwo")
    };
    let store = lock(&state.store)?;
    let id = store.save_workout(None, &workout, source).map_err(err)?;
    Ok(WorkoutRecord { id, workout, source: source.into(), updated_at: iso8601_utc(unix_now_s()) })
}

#[tauri::command]
async fn scan_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    let mut out = vec![DeviceInfo { id: SIM_TRAINER_ID.into(), name: "Virtual Trainer".into(), role: "trainer".into(), rssi: None }];
    #[cfg(feature = "ble")]
    if let Ok(found) = erglite_transport::ble::scan(Duration::from_secs(6)).await {
        let mut map = lock(&state.ble)?;
        map.clear();
        for (info, peripheral) in found {
            map.insert(info.id.clone(), peripheral);
            out.push(info);
        }
    }
    #[cfg(not(feature = "ble"))]
    let _ = &state;
    Ok(out)
}

#[tauri::command]
async fn connect_trainer(state: State<'_, AppState>, id: String) -> Result<TrainerCapabilities, String> {
    let profile = lock(&state.store)?.profile().map_err(err)?;
    let mut backend: Backend = if id == SIM_TRAINER_ID {
        Box::new(SimulatedTrainer::new(SimConfig { rider: profile.rider_params(), rider_power_w: f64::from(profile.ftp_w) * 0.74, ..Default::default() }))
    } else {
        #[cfg(feature = "ble")]
        {
            let peripheral = lock(&state.ble)?.get(&id).cloned().ok_or("device not found, scan again")?;
            let sim = erglite_transport::ftms_simulation_defaults(profile.crr as f32, profile.rider_params().cw_kg_m() as f32);
            Box::new(erglite_transport::ble::BleFtmsBackend::new(peripheral, sim))
        }
        #[cfg(not(feature = "ble"))]
        return Err("this build has no Bluetooth support".into());
    };
    let caps = backend.connect().await.map_err(err)?;
    *state.trainer.lock().await = Some(Connected { backend });
    Ok(caps)
}

#[tauri::command]
async fn disconnect_trainer(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(mut c) = state.trainer.lock().await.take() {
        c.backend.disconnect().await.map_err(err)?;
    }
    Ok(())
}

async fn run_session(app: AppHandle, mut runner: SessionRunner<Backend>, mut rx: UnboundedReceiver<Control>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    runner.start().await;
    let mut finished = false;
    loop {
        tokio::select! {
            _ = interval.tick(), if !finished => {
                let out = runner.tick(1000).await;
                finished = out.snapshot.status == Status::Finished;
                let _ = app.emit("session:tick", tick_dto(&out));
            }
            cmd = rx.recv() => match cmd {
                Some(Control::Pause) => { runner.pause().await; }
                Some(Control::Resume) => { runner.resume().await; }
                Some(Control::Skip) => { runner.skip_segment().await; }
                Some(Control::Intensity(d)) => { runner.adjust_intensity(d).await; }
                Some(Control::Difficulty(k)) => runner.set_difficulty(k),
                Some(Control::Stop(reply)) => {
                    let result = runner.finish(true).await.map_err(err);
                    let _ = reply.send(result);
                    break;
                }
                None => break,
            },
        }
    }
    // Give the trainer back so the UI can keep using the connection.
    let state = app.state::<AppState>();
    let backend = runner.into_backend();
    *state.trainer.lock().await = Some(Connected { backend });
}

#[tauri::command]
async fn start_session(app: AppHandle, state: State<'_, AppState>, workout_id: Option<String>, workout: Workout, options: StartOptions) -> Result<String, String> {
    let profile = lock(&state.store)?.profile().map_err(err)?;
    let Connected { backend, .. } = state.trainer.lock().await.take().ok_or("connect a trainer first")?;
    let store = Store::open(&state.db_path).map_err(err)?;
    let mut runner = SessionRunner::new(backend, &workout, &profile, Some(store), workout_id.as_deref()).map_err(err)?;
    runner.set_difficulty(options.difficulty);
    let id = runner.session_id().unwrap_or_default().to_string();
    let (tx, rx) = unbounded_channel();
    *lock(&state.control)? = Some(tx);
    tauri::async_runtime::spawn(run_session(app, runner, rx));
    Ok(id)
}

fn send(state: &State<'_, AppState>, c: Control) -> Result<(), String> {
    lock(&state.control)?.as_ref().ok_or("no active session")?.send(c).map_err(err)
}

#[tauri::command]
fn pause_session(state: State<'_, AppState>) -> Result<(), String> { send(&state, Control::Pause) }
#[tauri::command]
fn resume_session(state: State<'_, AppState>) -> Result<(), String> { send(&state, Control::Resume) }
#[tauri::command]
fn skip_segment(state: State<'_, AppState>) -> Result<(), String> { send(&state, Control::Skip) }
#[tauri::command]
fn adjust_intensity(state: State<'_, AppState>, delta_pct: f32) -> Result<(), String> { send(&state, Control::Intensity(delta_pct)) }
#[tauri::command]
fn set_difficulty(state: State<'_, AppState>, scale: f32) -> Result<(), String> { send(&state, Control::Difficulty(scale)) }

#[tauri::command]
async fn stop_session(state: State<'_, AppState>) -> Result<Summary, String> {
    let (tx, rx) = oneshot::channel();
    send(&state, Control::Stop(tx))?;
    *lock(&state.control)? = None;
    rx.await.map_err(err)?
}

#[tauri::command]
fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionRow>, String> {
    lock(&state.store)?.list_sessions().map_err(err)
}

#[tauri::command]
fn session_samples(state: State<'_, AppState>, id: String) -> Result<Vec<SamplePoint>, String> {
    lock(&state.store)?.samples(&id).map_err(err)
}

#[tauri::command]
fn export_session(state: State<'_, AppState>, id: String, format: String) -> Result<String, String> {
    if format != "tcx" {
        return Err("only TCX export is available yet".into());
    }
    let store = lock(&state.store)?;
    let samples = store.samples(&id).map_err(err)?;
    let start = store.session_started_unix(&id).map_err(err)?;
    Ok(erglite_formats::export_tcx(start, &samples))
}

fn seed_library(store: &Store) {
    if store.list_workouts().map(|l| !l.is_empty()).unwrap_or(true) {
        return;
    }
    for (_, json) in BUILTIN {
        if let Ok(w) = serde_json::from_str::<Workout>(json) {
            let _ = store.save_workout(None, &w, "builtin");
        }
    }
    if let Ok(r) = erglite_formats::import_zwo(BUILTIN_ZWO) {
        let _ = store.save_workout(None, &r.workout, "builtin");
    }
}

/// Start the application.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = std::env::var_os("ERGLITE_DATA_DIR").map(PathBuf::from).unwrap_or(app.path().app_data_dir()?);
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("erglite.db");
            let store = Store::open(&db_path)?;
            seed_library(&store);
            app.manage(AppState {
                db_path,
                store: Mutex::new(store),
                trainer: tokio::sync::Mutex::new(None),
                control: Mutex::new(None),
                #[cfg(feature = "ble")]
                ble: Mutex::new(Default::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_profile, set_profile, list_workouts, save_workout, delete_workout, import_workout,
            scan_devices, connect_trainer, disconnect_trainer, start_session, pause_session,
            resume_session, skip_segment, adjust_intensity, set_difficulty, stop_session,
            list_sessions, session_samples, export_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ErgLite");
}
