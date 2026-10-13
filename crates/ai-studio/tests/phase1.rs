use serde_json::{Value, json};
use soundcraft_ai_studio::{
    Authority, Bridge,
    compatibility::{REQUIRED, Snapshot, Status, compare},
    smoke,
};
use soundcraft_automation::{Backend, Headless};
use soundcraft_engine::Engine;

fn snapshot() -> Snapshot {
    let mut bridge = Bridge::new(Headless::new(Engine::default()));
    Snapshot::from_commands("0.6.1", &"a".repeat(40), &bridge.commands().unwrap()).unwrap()
}

#[test]
fn real_engine_workflow() {
    let result = smoke::run().unwrap();
    assert_eq!(result["passed"], true, "{result}");
}

#[test]
fn discovery_is_never_a_native_compatibility_claim() {
    let reference = snapshot();
    assert!(REQUIRED.iter().all(|id| reference.commands.contains_key(*id)));
    assert_eq!(compare(&reference, &reference).status, Status::NotFullyChecked);
    let mut candidate = reference.clone();
    candidate.commands.remove("midi.note_edit");
    let report = compare(&reference, &candidate);
    assert_eq!(report.status, Status::Incompatible);
    assert_eq!(report.missing_required, ["midi.note_edit"]);
}

#[test]
fn contract_changes_are_visible_and_state_is_ignored() {
    let reference = snapshot();
    let mut candidate = reference.clone();
    candidate.commands.insert("mix.volume".into(), "{changed}".into());
    candidate.commands.insert("future.command".into(), "{}".into());
    let report = compare(&reference, &candidate);
    assert_eq!(report.changed_parameters, ["mix.volume"]);
    assert_eq!(report.added, ["future.command"]);
    assert_eq!(report.status, Status::Incompatible);
    let a = json!([{"id":"test", "params":"{}", "enabled":false}]);
    let b = json!([{"id":"test", "params":"{}", "enabled":true}]);
    assert_eq!(Snapshot::from_commands("v", &"a".repeat(40), &a).unwrap(), Snapshot::from_commands("v", &"a".repeat(40), &b).unwrap());
}

#[test]
fn malformed_discovery_is_rejected() {
    for data in [json!({}), json!([]), json!([{"id":"x"}]), json!([{"id":"x","params":"{}"},{"id":"x","params":"{}"}])] {
        assert!(Snapshot::from_commands("v", &"a".repeat(40), &data).is_err());
    }
    assert!(Snapshot::from_commands("v", "unknown", &json!([])).is_err());
}

#[test]
fn denied_actions_do_not_change_real_engine() {
    let mut bridge = Bridge::new(Headless::new(Engine::default()));
    let before = bridge.inspect().unwrap();
    assert!(bridge.execute("track.new", json!({"name":"Blocked"}), Authority::ReadOnly).is_err());
    assert!(bridge.execute("transport.record", json!({}), Authority::UserRequestedEdit).is_err());
    assert!(bridge.execute("track.new", json!([]), Authority::UserRequestedEdit).is_err());
    assert_eq!(before, bridge.inspect().unwrap());
}

struct Failing;
impl Backend for Failing {
    fn call(&mut self, _: &str, _: Value) -> Result<Value, String> {
        Err("Authorization: Bearer sk-secret; private song and /private/path".into())
    }
    fn has_ui(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        "sk-secret".into()
    }
}

#[test]
fn default_diagnostics_never_copy_secrets_content_or_backend_errors() {
    let mut bridge = Bridge::new(Failing);
    assert!(bridge.execute("track.new", json!({"name":"private song", "api_key":"sk-secret"}), Authority::UserRequestedEdit).is_err());
    assert!(bridge.execute("sk-secret", json!({}), Authority::UserRequestedEdit).is_err());
    let text = bridge.diagnostics().to_string();
    for secret in ["sk-secret", "private song", "/private/path", "Bearer", "api_key"] {
        assert!(!text.contains(secret), "leaked {secret}");
    }
    assert!(text.contains("backend_error"));
}
