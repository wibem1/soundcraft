//! Deterministic workflow, always using a fresh engine and a fresh confined folder.
use crate::{Authority, Bridge};
use serde_json::{Value, json};
use soundcraft_automation::Headless;
use soundcraft_engine::{Engine, file_access::AutomationRoots};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, String> {
        let parent = std::env::temp_dir();
        for _ in 0..100 {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("soundcraft-ai-smoke-{}-{id}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("cannot allocate isolated test folder".into())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.into()) }
}
fn track(session: &Value) -> Result<&Value, String> {
    session
        .get("tracks")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().find(|t| t.get("name") == Some(&json!("AI Smoke"))))
        .ok_or_else(|| "synthetic track missing".into())
}

/// Returns metadata only even when a command fails. Does not connect to an existing desktop session.
pub fn run() -> Result<Value, String> {
    let scratch = Scratch::new()?;
    let roots = AutomationRoots::new(Some(&scratch.0), Some(&scratch.0))?;
    let mut bridge = Bridge::new(Headless::new(Engine::default()).with_automation_roots(roots));
    let mut stage = "discovery";
    let outcome = workflow(&mut bridge, &scratch, &mut stage);
    Ok(json!({
        "passed": outcome.is_ok(),
        "failed_check": if outcome.is_ok() { Value::Null } else { json!(stage) },
        "failure": if outcome.is_ok() { Value::Null } else { json!("workflow failed; inspect local test output") },
        "checks": ["discovery", "authorization", "track", "MIDI add/edit/undo", "mixer", "save/open"],
        "diagnostics": bridge.diagnostics(),
        "scope": "isolated headless workflow on this OS; does not prove native desktop or other platform compatibility"
    }))
}
fn workflow(bridge: &mut Bridge<Headless>, scratch: &Scratch, stage: &mut &str) -> Result<(), String> {
    let commands = bridge.commands()?;
    let list = commands.as_array().ok_or("invalid discovery")?;
    for required in crate::compatibility::REQUIRED {
        require(list.iter().any(|c| c.get("id").and_then(Value::as_str) == Some(required)), "required command missing")?;
    }
    *stage = "authorization";
    require(bridge.execute("track.new", json!({}), Authority::ReadOnly).is_err(), "read-only bridge allowed editing")?;
    let before = bridge.inspect()?;
    require(before.get("tracks").and_then(Value::as_array).is_some_and(Vec::is_empty), "isolation failed")?;
    let auth = Authority::UserRequestedEdit;
    *stage = "track";
    bridge.execute("track.new", json!({"name": "AI Smoke", "kind": "instrument", "format": "stereo"}), auth)?;
    let session = bridge.inspect()?;
    require(session.get("tracks").and_then(Value::as_array).map(Vec::len) == Some(1), "track count mismatch")?;
    *stage = "MIDI clip";
    let clip = bridge.execute("midi.new_clip", json!({"track": "AI Smoke", "start": 0, "length": {"seconds": 2.0}}), auth)?;
    let clip_id = clip.get("clip").and_then(Value::as_u64).ok_or("clip id missing")?;
    *stage = "MIDI add";
    bridge.execute("midi.note_add", json!({"clip": clip_id, "pitch": 60, "start_ticks": 0, "length_ticks": 480, "velocity": 80}), auth)?;
    let note_before = bridge.execute("midi.notes", json!({"clip": clip_id}), Authority::ReadOnly)?;
    require(note_before.pointer("/notes/0/pitch") == Some(&json!(60)), "note add failed")?;
    *stage = "MIDI edit";
    bridge.execute("midi.note_edit", json!({"clip": clip_id, "index": 0, "pitch": 67}), auth)?;
    let edited = bridge.execute("midi.notes", json!({"clip": clip_id}), Authority::ReadOnly)?;
    require(edited.pointer("/notes/0/pitch") == Some(&json!(67)), "note edit failed")?;
    *stage = "undo";
    bridge.execute("edit.undo", json!({}), auth)?;
    require(bridge.execute("midi.notes", json!({"clip": clip_id}), Authority::ReadOnly)? == note_before, "undo failed")?;
    *stage = "mixer";
    bridge.execute("mix.volume", json!({"track": "AI Smoke", "db": -9.0}), auth)?;
    require(track(&bridge.inspect()?)?.get("volume_db").and_then(Value::as_f64) == Some(-9.0), "mixer failed")?;
    *stage = "save";
    let path = scratch.0.join("smoke.scraft");
    bridge.execute("session.save", json!({"path": path.to_string_lossy()}), auth)?;
    bridge.execute("mix.volume", json!({"track": "AI Smoke", "db": -3.0}), auth)?;
    *stage = "reopen";
    bridge.execute("session.open", json!({"path": path.to_string_lossy()}), auth)?;
    let reopened = bridge.inspect()?;
    require(track(&reopened)?.get("volume_db").and_then(Value::as_f64) == Some(-9.0), "save/open mixer mismatch")?;
    require(bridge.execute("midi.notes", json!({"clip": clip_id}), Authority::ReadOnly)? == note_before, "save/open MIDI mismatch")?;
    Ok(())
}
