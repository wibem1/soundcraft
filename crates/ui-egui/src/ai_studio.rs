//! Native assistant panel. Reads use the documented command surface; tests run in a fresh engine.
use crate::SoundApp;
use serde_json::{Value, json};
use soundcraft_ai_studio::Bridge;
use soundcraft_automation::Backend;
use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Default)]
pub struct PanelState {
    pub context: Option<Value>,
    revision: Option<u64>,
    metadata: Value,
    #[cfg(not(target_arch = "wasm32"))]
    pub chat: crate::ai_chat::ChatState,
    pub(crate) events: Vec<Value>,
    pub self_test: Option<Value>,
    worker: Option<Receiver<Result<Value, String>>>,
    pub status: String,
}

/// Borrow the live host only for read methods. No parallel access to the app or audio callback.
struct ReadBackend<'a>(&'a mut SoundApp);
impl Backend for ReadBackend<'_> {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        match method {
            "session.inspect" | "engine.commands" => self.0.run(method, params),
            _ => Err("native panel adapter is read-only".into()),
        }
    }
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "SoundCraft native panel".into()
    }
}

pub fn refresh(app: &mut SoundApp) {
    let revision = app.engine.revision;
    let (result, diagnosis) = {
        let mut bridge = Bridge::new(ReadBackend(app));
        let result = bridge.inspect();
        (result, bridge.diagnostics())
    };
    let state = &mut app.ai_studio;
    state.metadata = diagnosis;
    if let Some(events) = state.metadata.get("events").and_then(Value::as_array) {
        state.events.extend(events.iter().cloned());
        let excess = state.events.len().saturating_sub(4096);
        state.events.drain(..excess);
    }
    match result {
        Ok(value) => {
            state.context = Some(value);
            state.revision = Some(revision);
            state.status = "Projektübersicht aktuell".into();
        }
        Err(_) => {
            state.context = None;
            state.revision = Some(revision);
            state.status = "Projekt konnte nicht gelesen werden".into();
        }
    }
}

impl PanelState {
    pub fn diagnostics(&self) -> Value {
        let mut result = self.metadata.clone();
        if !result.is_object() {
            result = json!({"format":"SoundCraft-AI-Studio-Diagnostics","schema_version":1});
        }
        result["events"] = json!(self.events);
        result["self_test"] = self.self_test.clone().unwrap_or(Value::Null);
        result["musical_data_included"] = json!(false);
        #[cfg(not(target_arch = "wasm32"))]
        {
            result["ai"] = self.chat.diagnostics();
            result["musical_data_included"] = json!(self.chat.include_dialog_diagnostics);
        }
        result
    }
    pub fn running(&self) -> bool {
        self.worker.is_some()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_self_test(&mut self, ctx: egui::Context) -> Result<(), String> {
        if self.running() {
            return Err("system check already running".into());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("ai-studio-self-test".into())
            .spawn(move || {
                let _ = tx.send(soundcraft_ai_studio::smoke::run());
                ctx.request_repaint();
            })
            .map_err(|_| "system check could not be started".to_owned())?;
        self.worker = Some(rx);
        self.self_test = None;
        self.status = "Systemprüfung läuft in einem separaten Testprojekt …".into();
        Ok(())
    }

    pub fn poll(&mut self) {
        let Some(worker) = &self.worker else { return };
        match worker.try_recv() {
            Ok(Ok(report)) => {
                self.status = if report.get("passed").and_then(Value::as_bool) == Some(true) {
                    "Systemprüfung bestanden".into()
                } else {
                    "Systemprüfung fehlgeschlagen — siehe Diagnose".into()
                };
                self.self_test = Some(report);
                self.worker = None;
            }
            Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
                self.status = "Systemprüfung konnte nicht abgeschlossen werden".into();
                self.self_test = Some(json!({"passed":false,"failure":"worker_error"}));
                self.worker = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

pub fn show(app: &mut SoundApp, ctx: &egui::Context) {
    app.ai_studio.poll();
    #[cfg(not(target_arch = "wasm32"))]
    app.ai_studio.chat.poll();
    if !app.ui.show_ai_studio {
        app.ai_studio.context = None;
        app.ai_studio.revision = None;
        return;
    }
    if app.ai_studio.revision != Some(app.engine.revision) {
        refresh(app);
    }
    let mut open = app.ui.show_ai_studio;
    let mut refresh_requested = false;
    #[cfg(not(target_arch = "wasm32"))]
    let mut test_requested = false;
    let mut export_requested = false;
    egui::Window::new("AI Studio")
        .id(egui::Id::new("ai_studio_panel"))
        .open(&mut open)
        .default_size(egui::vec2(530.0, 740.0))
        .default_pos(egui::pos2(760.0, 140.0))
        .min_size(egui::vec2(310.0, 260.0))
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("ai_studio_scroll").show(ui, |ui| {
                ui.heading("Dein musikalischer Assistent");
                #[cfg(not(target_arch = "wasm32"))]
                crate::ai_chat::show(app, ui, ctx);
                #[cfg(target_arch = "wasm32")]
                ui.label("KI-Dialog ist in der Desktop-Version verfügbar.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.strong("Aktuelles Projekt");
                    refresh_requested = ui.button("Aktualisieren").clicked();
                });
                if let Some(session) = &app.ai_studio.context {
                    ui.label(egui::RichText::new(session.get("name").and_then(Value::as_str).unwrap_or("Unbenannt")).strong());
                    let tracks = session.get("tracks").and_then(Value::as_array);
                    ui.label(format!("{} Spuren", tracks.map(Vec::len).unwrap_or(0)));
                    if let Some(tracks) = tracks {
                        egui::ScrollArea::vertical().id_salt("ai_studio_tracks").max_height(120.0).show(ui, |ui| {
                            for track in tracks.iter().take(128) {
                                let name = track.get("name").and_then(Value::as_str).unwrap_or("Spur");
                                let kind = track.get("kind").and_then(Value::as_str).unwrap_or("");
                                let clips = track.get("clips").and_then(Value::as_array).map(Vec::len).unwrap_or(0);
                                ui.group(|ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.strong(name);
                                    ui.small(format!("{kind} · {clips} Clips"));
                                });
                            }
                            if tracks.len() > 128 {
                                ui.label("Weitere Spuren sind im Projekt vorhanden.");
                            }
                        });
                    }
                } else {
                    ui.label("Keine Projektübersicht verfügbar.");
                }
                ui.add_space(12.0);
                ui.separator();
                ui.strong("Diagnose und Systemprüfung");
                ui.label("Die Prüfung verwendet ein eigenes Testprojekt. Dein Musikprojekt bleibt unverändert.");
                ui.horizontal_wrapped(|ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        test_requested = ui.add_enabled(!app.ai_studio.running(), egui::Button::new("System prüfen")).clicked();
                    }
                    if ui.button("Diagnose kopieren").clicked() {
                        match serde_json::to_string_pretty(&app.ai_studio.diagnostics()) {
                            Ok(text) => {
                                ctx.copy_text(text);
                                app.ai_studio.status = "Diagnose kopiert".into();
                            }
                            Err(_) => app.ai_studio.status = "Diagnose konnte nicht erstellt werden".into(),
                        }
                    }
                    if app.services.pick_save.is_some() {
                        export_requested = ui.button("Diagnose speichern").clicked();
                    }
                });
                if app.ai_studio.running() {
                    ui.spinner();
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                ui.label(&app.ai_studio.status);
                #[cfg(not(target_arch = "wasm32"))]
                ui.checkbox(&mut app.ai_studio.chat.include_dialog_diagnostics, "Dialog in die Diagnose aufnehmen");
                ui.small("Zugangsdaten werden niemals in die Diagnose aufgenommen.");
                ui.add_space(12.0);
                ui.small(format!("SoundCraft AI Studio {} · 13.10.2026 · Entwicklungsversion", soundcraft_ai_studio::VERSION));
            });
        });
    app.ui.show_ai_studio = open;
    if refresh_requested {
        refresh(app);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if test_requested && app.ai_studio.start_self_test(ctx.clone()).is_err() {
        app.ai_studio.status = "Systemprüfung konnte nicht gestartet werden".into();
    }
    if export_requested {
        let path = app.services.pick_save.as_ref().and_then(|pick| pick("AI Studio Diagnose", "SoundCraft-AI-Studio-Diagnose.json"));
        if let Some(path) = path {
            let result =
                serde_json::to_vec_pretty(&app.ai_studio.diagnostics()).map_err(|_| ()).and_then(|bytes| std::fs::write(path, bytes).map_err(|_| ()));
            app.ai_studio.status = if result.is_ok() { "Diagnose gespeichert".into() } else { "Diagnose konnte nicht gespeichert werden".into() };
        }
    }
}
