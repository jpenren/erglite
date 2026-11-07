// SPDX-License-Identifier: MIT OR Apache-2.0
//! `erglite-cli`: simulate workouts, estimate routes and (with the `ble` feature) scan for devices.

use clap::{Parser, Subcommand};
use erglite_core::engine::Status;
use erglite_core::physics::estimate_route;
use erglite_core::profile::Profile;
use erglite_core::workout::{
    Control, End, GradeControl, Segment, SegmentNode, Workout, WorkoutKind,
};
use erglite_runtime::SessionRunner;
use erglite_transport::{SimConfig, SimulatedTrainer, TrainerBackend};
use std::path::Path;

#[derive(Parser)]
#[command(name = "erglite-cli", version, about = "ErgLite command line tools")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a workout (.json, .zwo or .gpx) against the simulated trainer in accelerated time.
    Simulate {
        /// Workout or route file.
        file: String,
        /// Rider FTP in watts.
        #[arg(long, default_value_t = 250)]
        ftp: u16,
        /// Power the simulated rider produces on routes.
        #[arg(long, default_value_t = 200.0)]
        rider_power: f64,
        /// Simplify GPX to one point every N meters (e.g., 500).
        #[arg(long)]
        simplify: Option<f64>,
    },
    /// Estimate the time of a route at a constant power.
    Estimate {
        /// Workout (.json) or GPX route.
        file: String,
        /// Power in watts.
        #[arg(long, default_value_t = 200.0)]
        power: f64,
        /// Simplify GPX to one point every N meters (e.g., 500).
        #[arg(long)]
        simplify: Option<f64>,
    },
    /// Scan for Bluetooth fitness devices (requires the `ble` feature).
    Scan {
        /// Scan time in seconds.
        #[arg(long, default_value_t = 8)]
        seconds: u64,
    },
}

fn load(path: &str, simplify: Option<f64>) -> Result<Workout, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "zwo" => erglite_formats::import_zwo(&text)
            .map(|r| r.workout)
            .map_err(|e| e.to_string()),
        "gpx" => {
            let route =
                erglite_formats::import_gpx_route(&text, 100.0, 9, simplify).map_err(|e| e.to_string())?;
            let length = route.total_distance_m();
            Ok(Workout {
                schema_version: 1,
                name: Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Route")
                    .to_string(),
                description: None,
                kind: WorkoutKind::Route,
                route: Some(route),
                segments: vec![SegmentNode::Single(Segment {
                    label: Some("Route".into()),
                    end: End::DistanceM(length),
                    control: Control::Grade(GradeControl::FromRoute),
                    cadence_rpm: None,
                    message: None,
                })],
            })
        }
        _ => serde_json::from_str(&text).map_err(|e| e.to_string()),
    }
}

fn mmss(s: f64) -> String {
    format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32)
}

async fn simulate(file: &str, ftp: u16, rider_power: f64, simplify: Option<f64>) -> Result<(), String> {
    let workout = load(file, simplify)?;
    let profile = Profile {
        ftp_w: ftp,
        ..Profile::default()
    };
    let sim = SimulatedTrainer::new(SimConfig {
        rider_power_w: rider_power,
        rider: profile.rider_params(),
        ..Default::default()
    });
    let mut runner =
        SessionRunner::new(sim, &workout, &profile, None, None).map_err(|e| e.to_string())?;
    runner
        .backend_mut()
        .connect()
        .await
        .map_err(|e| e.to_string())?;
    println!("{}  (FTP {ftp} W)", workout.name);
    runner.start().await;
    let mut ticks = 0u32;
    while runner.engine().status() != Status::Finished && ticks < 20_000 {
        let out = runner.tick(1000).await;
        if ticks % 300 == 0 {
            let t = out.telemetry.unwrap_or_default();
            println!(
                "  {:>6}  segment {:>2}/{:<2}  target {:>4}  power {:>4}  speed {:>5.1} km/h  grade {:>5.1} %  dist {:>6.2} km",
                mmss(out.snapshot.elapsed_s),
                out.snapshot.segment_index + 1,
                out.snapshot.segment_count,
                out.snapshot
                    .target_power_w
                    .map_or("-".into(), |w| w.to_string()),
                t.power_w.unwrap_or(0),
                t.speed_kmh.unwrap_or(0.0),
                out.snapshot.grade_pct.unwrap_or(0.0),
                out.snapshot.distance_m / 1000.0,
            );
        }
        ticks += 1;
    }
    let s = runner.finish(true).await.map_err(|e| e.to_string())?;
    println!(
        "\nDuration {}  avg {:.0} W  NP {}  TSS {}  distance {:.2} km  gain {:.0} m",
        mmss(s.duration_s),
        s.avg_power_w,
        s.normalized_power_w
            .map_or("-".into(), |v| format!("{v:.0} W")),
        s.tss.map_or("-".into(), |v| format!("{v:.0}")),
        s.distance_m / 1000.0,
        s.elevation_gain_m.max(0.0)
    );
    println!("Zone seconds: {:?}", s.zone_seconds);
    Ok(())
}

fn estimate(file: &str, power: f64, simplify: Option<f64>) -> Result<(), String> {
    let workout = load(file, simplify)?;
    let route = workout.route.ok_or("the file has no route")?;
    let e = estimate_route(&route, power, &Profile::default().rider_params());
    let (up, down) = route.grade_extremes();
    println!(
        "{:.1} km, +{:.0} m, max climb {:.1} %, max descent {:.1} %",
        route.total_distance_m() / 1000.0,
        route.elevation_gain_m(),
        up,
        down
    );
    println!(
        "At {power:.0} W: {} ({:.1} km/h average)",
        mmss(e.time_s),
        e.avg_speed_kmh
    );
    Ok(())
}

#[cfg(feature = "ble")]
async fn scan(seconds: u64) -> Result<(), String> {
    let found = erglite_transport::ble::scan(std::time::Duration::from_secs(seconds))
        .await
        .map_err(|e| e.to_string())?;
    for (d, _) in found {
        println!(
            "{:<10} {:<28} {:>5} dBm  {}",
            d.role,
            d.name,
            d.rssi.map_or("?".into(), |r| r.to_string()),
            d.id
        );
    }
    Ok(())
}

#[cfg(not(feature = "ble"))]
async fn scan(_seconds: u64) -> Result<(), String> {
    Err("built without Bluetooth support: rebuild with `--features ble`".into())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let result = match Cli::parse().command {
        Cmd::Simulate {
            file,
            ftp,
            rider_power,
            simplify,
        } => simulate(&file, ftp, rider_power, simplify).await,
        Cmd::Estimate { file, power, simplify } => estimate(&file, power, simplify),
        Cmd::Scan { seconds } => scan(seconds).await,
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
