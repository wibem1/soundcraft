use egui::{Pos2, RawInput, Rect, vec2};
use serde_json::{Value, json};
use soundcraft_ui_egui::{Services, SoundApp, UiState, ai_studio, menus};
use std::time::{Duration, Instant};

fn app() -> SoundApp {
    let mut engine = soundcraft_engine::Engine::default();
    engine.execute("track.new", &json!({"name":"Private melody", "kind":"instrument"})).unwrap();
    SoundApp::new(engine, None, Services::default())
}
fn frame(app: &mut SoundApp, ctx: &egui::Context, width: f32) {
    ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 800.0))), ..Default::default() }, |ui| {
        app.logic(ui.ctx());
        app.ui(ui);
    })
    .textures_delta
    .clear();
}
#[test]
fn menu_toggle_and_saved_visibility_follow_existing_ui_contract() {
    let mut app = app();
    assert!(!app.ui.show_ai_studio);
    assert_eq!(app.run("window.ai_studio", json!({"value":true})).unwrap(), json!({"value":true}));
    let saved = serde_json::to_value(&app.ui).unwrap();
    let restored: UiState = serde_json::from_value(saved).unwrap();
    assert!(restored.show_ai_studio);
    let legacy: UiState = serde_json::from_value(json!({})).unwrap();
    assert!(!legacy.show_ai_studio);
    assert!(menus::UI_COMMANDS.iter().any(|(id, _, path, _)| *id == "window.ai_studio" && *path == "Window > AI Studio"));
    app.run("window.hide_floating", json!({})).unwrap();
    assert!(!app.ui.show_ai_studio);
}
#[test]
fn real_panel_reads_without_editing_and_diagnostics_omit_project_content() {
    let mut app = app();
    let before = app.run("session.inspect", json!({"detail":"full"})).unwrap();
    let revision = app.engine.revision;
    app.run("window.ai_studio", json!({"value":true})).unwrap();
    let ctx = egui::Context::default();
    for _ in 0..8 {
        frame(&mut app, &ctx, 900.0);
    }
    assert_eq!(app.ai_studio.context.as_ref().unwrap()["tracks"][0]["name"], "Private melody");
    assert_eq!(app.engine.revision, revision);
    assert_eq!(app.run("session.inspect", json!({"detail":"full"})).unwrap(), before);
    let diagnosis = app.ai_studio.diagnostics().to_string();
    assert!(!diagnosis.contains("Private melody"));
    assert!(diagnosis.contains("session.inspect"));
    app.run("session.new", json!({"name":"Replacement"})).unwrap();
    frame(&mut app, &ctx, 480.0);
    assert_eq!(app.ai_studio.context.as_ref().unwrap()["name"], "Replacement");
    app.run("window.ai_studio", json!({"value":false})).unwrap();
    frame(&mut app, &ctx, 480.0);
    assert!(app.ai_studio.context.is_none());
}
#[test]
fn asynchronous_self_test_preserves_the_live_project() {
    let mut app = app();
    let before = app.run("session.inspect", json!({"detail":"full"})).unwrap();
    ai_studio::refresh(&mut app);
    app.ai_studio.start_self_test(egui::Context::default()).unwrap();
    assert!(app.ai_studio.start_self_test(egui::Context::default()).is_err());
    let deadline = Instant::now() + Duration::from_secs(15);
    while app.ai_studio.running() && Instant::now() < deadline {
        app.ai_studio.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.ai_studio.self_test.as_ref().unwrap()["passed"], true);
    assert_eq!(app.run("session.inspect", json!({"detail":"full"})).unwrap(), before);
    let exported: Value = app.ai_studio.diagnostics();
    assert_eq!(exported["musical_data_included"], false);
    assert!(!exported.to_string().contains("Private melody"));
}
