//! Developer tools. Never connects to a user's session for smoke tests.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
use soundcraft_ai_studio::compatibility::{Snapshot, Status, compare};
use std::process::ExitCode;

fn run() -> Result<bool, String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("self-test") if args.len() == 1 => {
            let report = soundcraft_ai_studio::smoke::run()?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
            Ok(report.get("passed").and_then(serde_json::Value::as_bool) == Some(true))
        }
        Some("headless-snapshot") if args.len() == 1 => {
            let mut bridge = soundcraft_ai_studio::Bridge::new(soundcraft_automation::Headless::new(soundcraft_engine::Engine::default()));
            let snapshot = Snapshot::from_commands("0.6.1", "c0cb99340891d306e216582db465f3594b89dc1d", &bridge.commands()?)?;
            println!("{}", serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?);
            Ok(true)
        }
        Some("snapshot") if args.len() == 4 => {
            let port = args.get(1).ok_or("port missing")?.parse::<u16>().map_err(|e| e.to_string())?;
            let version = args.get(2).ok_or("version missing")?;
            let commit = args.get(3).ok_or("commit missing")?;
            let mut bridge = soundcraft_ai_studio::Bridge::new(soundcraft_ai_studio::transport::LocalControl::new(port)?);
            let snapshot = Snapshot::from_commands(version, commit, &bridge.commands()?)?;
            println!("{}", serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?);
            Ok(true)
        }
        Some("compare") if args.len() == 3 => {
            let mut snapshots = Vec::new();
            for path in args.iter().skip(1) {
                let content = std::fs::read(path).map_err(|e| e.to_string())?;
                let snapshot: Snapshot = serde_json::from_slice(&content).map_err(|e| e.to_string())?;
                snapshots.push(snapshot);
            }
            let reference = snapshots.first().ok_or("reference missing")?;
            let candidate = snapshots.get(1).ok_or("candidate missing")?;
            let report = compare(reference, candidate);
            println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
            Ok(report.status == Status::Compatible)
        }
        _ => {
            Err("usage: soundcraft-ai-studio self-test | headless-snapshot | snapshot PORT VERSION COMMIT | compare REFERENCE.json CANDIDATE.json"
                .into())
        }
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
