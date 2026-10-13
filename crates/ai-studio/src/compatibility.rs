//! Compare stable command IDs and parameter documentation, never transient enabled flags.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const REQUIRED: &[&str] = &[
    "engine.commands",
    "midi.notes",
    "track.new",
    "midi.new_clip",
    "midi.note_add",
    "midi.note_edit",
    "mix.volume",
    "edit.undo",
    "session.save",
    "session.open",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub soundcraft_version: String,
    pub soundcraft_commit: String,
    pub commands: BTreeMap<String, String>,
}

impl Snapshot {
    pub fn from_commands(version: &str, commit: &str, value: &Value) -> Result<Self, String> {
        if version.trim().is_empty() || commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("SoundCraft version and full commit SHA required".into());
        }
        let list = value.as_array().ok_or("command discovery must return an array")?;
        if list.is_empty() || list.len() > 16384 {
            return Err("invalid command count".into());
        }
        let mut commands = BTreeMap::new();
        for item in list {
            let id = item.get("id").and_then(Value::as_str).ok_or("command id missing")?;
            let params = item.get("params").and_then(Value::as_str).ok_or("parameter documentation missing")?;
            if id.is_empty() || id.len() > 256 || params.len() > 16384 {
                return Err("invalid command descriptor".into());
            }
            if commands.insert(id.to_owned(), params.to_owned()).is_some() {
                return Err("duplicate command id".into());
            }
        }
        Ok(Self { soundcraft_version: version.into(), soundcraft_commit: commit.into(), commands })
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum Status {
    Compatible,
    Incompatible,
    NotFullyChecked,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub status: Status,
    pub missing_required: Vec<String>,
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub changed_parameters: Vec<String>,
    pub scope: &'static str,
}

/// Discovery alone can never mark a release compatible. Native behavioural tests remain required.
pub fn compare(reference: &Snapshot, candidate: &Snapshot) -> Report {
    let missing_required = REQUIRED.iter().filter(|id| !candidate.commands.contains_key(**id)).map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let removed = reference.commands.keys().filter(|id| !candidate.commands.contains_key(*id)).cloned().collect();
    let added = candidate.commands.keys().filter(|id| !reference.commands.contains_key(*id)).cloned().collect();
    let changed_parameters = reference
        .commands
        .iter()
        .filter(|(id, params)| candidate.commands.get(*id).is_some_and(|p| p != *params))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    Report {
        status: if missing_required.is_empty() && changed_parameters.is_empty() { Status::NotFullyChecked } else { Status::Incompatible },
        missing_required,
        removed,
        added,
        changed_parameters,
        scope: "Command discovery only; behaviour, MCP, desktop, macOS and Windows gates are not established by this comparison",
    }
}
