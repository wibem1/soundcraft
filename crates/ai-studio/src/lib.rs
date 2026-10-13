//! Phase 1 uses only the documented control protocol; no audio or UI integration.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod compatibility;
pub mod smoke;
pub mod transport;

use serde::Serialize;
use serde_json::{Value, json};
use soundcraft_automation::Backend;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const READ_COMMANDS: &[&str] = &["midi.notes", "engine.history"];
pub const WRITE_COMMANDS: &[&str] =
    &["track.new", "midi.new_clip", "midi.note_add", "midi.note_edit", "mix.volume", "edit.undo", "session.save", "session.open"];

/// Supplied by the host after the user authorizes this action. Never inferred from model output.
#[derive(Clone, Copy)]
pub enum Authority {
    ReadOnly,
    UserRequestedEdit,
}

/// Intentionally excludes parameters, replies, arbitrary error text, paths and musical content.
#[derive(Debug, Serialize)]
pub struct Event {
    pub timestamp_ms: u128,
    pub elapsed_ms: u128,
    pub operation: String,
    pub outcome: &'static str,
}

pub struct Bridge<B> {
    backend: B,
    events: Vec<Event>,
}

impl<B: Backend> Bridge<B> {
    pub fn new(backend: B) -> Self {
        Self { backend, events: Vec::new() }
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    fn record(&mut self, operation: &str, start: Instant, outcome: &'static str) {
        // Bound retained metadata. No unrestricted values from callers reach this log.
        if self.events.len() >= 4096 {
            self.events.remove(0);
        }
        self.events.push(Event {
            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
            elapsed_ms: start.elapsed().as_millis(),
            operation: operation.to_owned(),
            outcome,
        });
    }

    fn call(&mut self, method: &str, params: Value, operation: &str) -> Result<Value, String> {
        let start = Instant::now();
        let result = self.backend.call(method, params);
        self.record(operation, start, if result.is_ok() { "ok" } else { "backend_error" });
        result
    }

    pub fn inspect(&mut self) -> Result<Value, String> {
        self.call("session.inspect", json!({"detail": "full"}), "session.inspect")
    }

    pub fn commands(&mut self) -> Result<Value, String> {
        self.call("engine.commands", json!({}), "engine.commands")
    }

    pub fn execute(&mut self, command: &str, params: Value, authority: Authority) -> Result<Value, String> {
        let start = Instant::now();
        let read = READ_COMMANDS.contains(&command);
        let write = WRITE_COMMANDS.contains(&command);
        if !read && !write {
            self.record("rejected_command", start, "not_allowlisted");
            return Err("command is outside the Phase 1 allowlist".into());
        }
        if write && matches!(authority, Authority::ReadOnly) {
            self.record(command, start, "authorization_required");
            return Err("explicit user authorization required for editing".into());
        }
        if !params.is_object() {
            self.record(command, start, "invalid_parameters");
            return Err("command parameters must be an object".into());
        }
        self.call("engine.execute", json!({"command": command, "params": params}), command)
    }

    pub fn diagnostics(&self) -> Value {
        json!({
            "format": "SoundCraft-AI-Studio-Diagnostics", "schema_version": 1,
            "ai_studio_version": VERSION, "build_commit": env!("AI_STUDIO_BUILD_COMMIT"),
            "source_date": "2026-10-13", "soundcraft_base_version": "0.6.1",
            "soundcraft_base_commit": "c0cb99340891d306e216582db465f3594b89dc1d", "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
            "events": self.events, "musical_data_included": false,
            "provider": null, "model": null, "tokens": 0, "api_cost": 0,
            "scope": "Phase 1: no external AI calls"
        })
    }
}
