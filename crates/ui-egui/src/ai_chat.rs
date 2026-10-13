//! Native conversation and explicit command proposals. Network work never runs in the audio/UI callback.
use crate::{SoundApp, ai_studio};
use serde_json::{Value, json};
use soundcraft_ai_studio::{
    Bridge,
    assistant::{self, Proposal, Provider, Receipt, Turn},
    credentials::{self, Secret},
    history::{self, Conversation},
    provider::{self, Answer, Https, Transport},
};
use soundcraft_automation::{Backend, Headless};
use soundcraft_engine::Engine;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

struct Pending {
    receiver: Receiver<Result<Answer, String>>,
    generation: u64,
    revision: u64,
    path: Option<String>,
}
pub struct EditPreview {
    pub plan: Proposal,
    pub generation: u64,
    pub revision: u64,
}
#[derive(Default)]
pub struct ChatState {
    pub input: String,
    pub api_input: String,
    secret: Option<Secret>,
    loaded_provider: Option<Provider>,
    pub conversation: Conversation,
    pub preview: Option<EditPreview>,
    pending: Option<Pending>,
    pub memory_root: Option<PathBuf>,
    path: Option<String>,
    generation: u64,
    pub status: String,
    pub edits_allowed: bool,
    pub include_dialog_diagnostics: bool,
    pub last_edit: Option<(u64, u64, usize)>,
    receipts: Vec<Receipt>,
}
impl Drop for ChatState {
    fn drop(&mut self) {
        credentials::clear_field(&mut self.api_input);
    }
}

struct Host<'a>(&'a mut SoundApp);
impl Backend for Host<'_> {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        match method {
            "session.inspect" | "engine.commands" => self.0.run(method, params),
            "engine.execute" => {
                let id = params.get("command").and_then(Value::as_str).ok_or("Befehl fehlt")?;
                self.0.run(id, params.get("params").cloned().unwrap_or(json!({})))
            }
            _ => Err("Schnittstelle nicht verfügbar".into()),
        }
    }
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "Native AI Bridge".into()
    }
}
fn remember(app: &mut SoundApp, diagnosis: Value) {
    if let Some(events) = diagnosis.get("events").and_then(Value::as_array) {
        app.ai_studio.events.extend(events.iter().cloned());
        let excess = app.ai_studio.events.len().saturating_sub(4096);
        app.ai_studio.events.drain(..excess);
    }
}
impl ChatState {
    fn persist(&mut self) {
        if let (Some(root), Some(path)) = (&self.memory_root, &self.path)
            && let Err(error) = history::save(root, path, &self.conversation)
        {
            self.status = error;
        }
    }
    pub fn project_replaced(&mut self) {
        self.persist();
        self.generation = self.generation.saturating_add(1);
        self.path = None;
        self.conversation = Conversation::default();
        self.preview = None;
        self.last_edit = None;
        self.input.clear();
    }
    fn sync_path(&mut self, path: Option<String>) {
        if self.path == path {
            return;
        }
        self.persist();
        // Save As retains the current conversation; a real project replacement resets it above.
        if self.conversation.turns.is_empty()
            && let (Some(root), Some(path)) = (&self.memory_root, &path)
        {
            match history::load(root, path) {
                Ok(book) => self.conversation = book,
                Err(error) => self.status = error,
            }
        }
        self.path = path;
        self.persist();
    }
    fn load_key(&mut self, provider: Provider) {
        if self.loaded_provider == Some(provider) {
            return;
        }
        self.secret = None;
        credentials::clear_field(&mut self.api_input);
        self.loaded_provider = Some(provider);
        match credentials::load(provider) {
            Ok(key) => self.secret = key,
            Err(error) => self.status = error,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn poll(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        let result = match pending.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(TryRecvError::Disconnected) => Some(Err("KI-Anfrage konnte nicht abgeschlossen werden".into())),
            Err(TryRecvError::Empty) => None,
        };
        let Some(result) = result else {
            return;
        };
        let Some(pending) = self.pending.take() else {
            return;
        };
        match result {
            Ok(answer) => {
                self.receipts.push(answer.receipt.clone());
                if self.receipts.len() > 500 {
                    self.receipts.remove(0);
                }
                if pending.generation != self.generation {
                    if let (Some(root), Some(path)) = (&self.memory_root, &pending.path)
                        && let Ok(mut old) = history::load(root, path)
                    {
                        old.turns.push(Turn { role: "assistant".into(), text: answer.proposal.reply });
                        old.receipts.push(answer.receipt);
                        let _ = history::save(root, path, &old);
                    }
                    self.status = "Antwort zum vorherigen Projekt gespeichert; keine Bearbeitung ausgeführt".into();
                    return;
                }
                self.conversation.turns.push(Turn { role: "assistant".into(), text: answer.proposal.reply.clone() });
                self.conversation.receipts.push(answer.receipt);
                self.preview = (!answer.proposal.actions.is_empty() && answer.complete).then_some(EditPreview {
                    plan: answer.proposal,
                    generation: self.generation,
                    revision: pending.revision,
                });
                self.status =
                    if self.preview.is_some() { "Bearbeitungsvorschlag bereit — noch nicht angewendet".into() } else { "Antwort erhalten".into() };
                self.persist();
            }
            Err(error) => {
                self.status = error;
                self.preview = None;
            }
        }
    }
    pub fn diagnostics(&self) -> Value {
        let mut result = json!({"requests":self.receipts,"request_in_progress":self.busy(),"dialog_included":self.include_dialog_diagnostics});
        if self.include_dialog_diagnostics {
            result["dialog"] = json!(self.conversation.turns);
        }
        result
    }
}

fn remove_paths(value: &mut Value) {
    match value {
        Value::Object(o) => {
            o.remove("path");
            for v in o.values_mut() {
                remove_paths(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                remove_paths(v);
            }
        }
        _ => {}
    }
}
pub fn context(app: &mut SoundApp) -> Result<Value, String> {
    ai_studio::refresh(app);
    let mut value = app.ai_studio.context.clone().ok_or("Projektkontext nicht verfügbar")?;
    remove_paths(&mut value);
    let selected = value.pointer("/selection/clips").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut clips = value
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|t| t.get("clips").and_then(Value::as_array).into_iter().flatten())
        .filter(|c| c.get("notes").is_some())
        .filter_map(|c| c.get("id").cloned())
        .collect::<Vec<_>>();
    clips.sort_by_key(|id| !selected.contains(id));
    let mut midi = Vec::new();
    let mut bridge = Bridge::new(Host(app));
    for id in clips.iter().take(8) {
        let mut notes = bridge.execute("midi.notes", json!({"clip":id}), soundcraft_ai_studio::Authority::ReadOnly)?;
        if let Some(a) = notes.get_mut("notes").and_then(Value::as_array_mut) {
            let total = a.len();
            a.truncate(256);
            notes["total_notes"] = json!(total);
            notes["truncated"] = json!(total > 256);
        }
        midi.push(json!({"clip":id,"data":notes}));
    }
    let diagnosis = bridge.diagnostics();
    remember(app, diagnosis);
    value["midi_content"] = json!(midi);
    value["midi_context_truncated"] = json!(clips.len() > 8);
    Ok(value)
}

pub fn send(app: &mut SoundApp, ctx: egui::Context) -> Result<(), String> {
    send_with(app, ctx, Https)
}
pub fn send_with<T: Transport + Send + 'static>(app: &mut SoundApp, ctx: egui::Context, transport: T) -> Result<(), String> {
    let chat = &mut app.ai_studio.chat;
    if chat.busy() {
        return Err("Eine KI-Anfrage läuft bereits".into());
    }
    chat.sync_path(app.engine.path.clone());
    let settings = app.ui.ai_settings.clone();
    chat.load_key(settings.provider);
    let key = chat.secret.as_ref().ok_or("Bitte einen API-Schlüssel eingeben oder laden")?.duplicate();
    let input = chat.input.trim().to_owned();
    if input.is_empty() || input.len() > 32768 {
        return Err("Bitte einen Auftrag mit höchstens 32768 Zeichen eingeben".into());
    }
    if chat.conversation.turns.len() >= 77 {
        return Err("Dialog ist lang; bitte einen neuen Dialog beginnen".into());
    }
    if input.contains(key.expose()) || settings.model.contains(key.expose()) {
        return Err("API-Schlüssel gehört ausschließlich in das Schlüsselfeld".into());
    }
    let edits = chat.edits_allowed;
    let mut turns = chat.conversation.turns.clone();
    turns.push(Turn { role: "user".into(), text: input.clone() });
    let generation = chat.generation;
    let revision = app.engine.revision;
    let path = app.engine.path.clone();
    let snapshot = context(app)?;
    if snapshot.to_string().contains(key.expose()) || turns.iter().any(|t| t.text.contains(key.expose())) {
        return Err("API-Schlüssel darf nicht im Dialog oder Projektkontext stehen".into());
    }
    provider::request(&settings, &turns, &snapshot, edits)?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("ai-studio-provider".into())
        .spawn(move || {
            let answer = provider::send(&transport, &settings, &key, &turns, &snapshot, edits);
            let _ = tx.send(answer);
            ctx.request_repaint();
        })
        .map_err(|_| "KI-Anfrage konnte nicht gestartet werden")?;
    let chat = &mut app.ai_studio.chat;
    chat.conversation.turns.push(Turn { role: "user".into(), text: input });
    chat.input.clear();
    chat.preview = None;
    chat.pending = Some(Pending { receiver: rx, generation, revision, path });
    chat.status = "KI arbeitet …".into();
    chat.persist();
    Ok(())
}

pub fn apply_preview(app: &mut SoundApp) -> Result<(), String> {
    let preview = app.ai_studio.chat.preview.as_ref().ok_or("Kein Bearbeitungsvorschlag vorhanden")?;
    if preview.generation != app.ai_studio.chat.generation || preview.revision != app.engine.revision {
        return Err("Projekt wurde inzwischen geändert. Bitte einen neuen Vorschlag anfordern".into());
    }
    let plan = preview.plan.clone();
    // Validate all commands against a separate clone via the official command API before touching the live document.
    let mut sandbox = Bridge::new(Headless::new(Engine::new(app.engine.session().clone())));
    assistant::apply(&mut sandbox, &plan)?;
    let (result, diagnosis) = {
        let mut bridge = Bridge::new(Host(app));
        let result = assistant::apply(&mut bridge, &plan);
        (result, bridge.diagnostics())
    };
    remember(app, diagnosis);
    result?;
    let steps = plan.actions.len();
    let chat = &mut app.ai_studio.chat;
    chat.preview = None;
    chat.last_edit = Some((chat.generation, app.engine.revision, steps));
    let text = format!("Bearbeitung ausgeführt und geprüft: {steps} Schritte. Rückgängig ist verfügbar.");
    chat.status = text.clone();
    chat.conversation.turns.push(Turn { role: "assistant".into(), text });
    chat.persist();
    ai_studio::refresh(app);
    Ok(())
}
pub fn undo_last(app: &mut SoundApp) -> Result<(), String> {
    let (generation, revision, steps) = app.ai_studio.chat.last_edit.ok_or("Keine KI-Bearbeitung zum Rückgängigmachen")?;
    if generation != app.ai_studio.chat.generation || revision != app.engine.revision {
        return Err("Projekt wurde danach geändert; bitte die normale Undo-Funktion verwenden".into());
    }
    let diagnosis = {
        let mut bridge = Bridge::new(Host(app));
        for _ in 0..steps {
            bridge.execute("edit.undo", json!({}), soundcraft_ai_studio::Authority::UserRequestedEdit)?;
        }
        bridge.diagnostics()
    };
    remember(app, diagnosis);
    app.ai_studio.chat.last_edit = None;
    app.ai_studio.chat.status = "KI-Bearbeitung rückgängig gemacht".into();
    ai_studio::refresh(app);
    Ok(())
}
fn action_label(action: &assistant::Action) -> String {
    match action.command.as_str() {
        "track.new" => "Neue Spur anlegen",
        "midi.new_clip" => "MIDI-Clip anlegen",
        "midi.note_add" => "MIDI-Note hinzufügen",
        "midi.note_edit" => "MIDI-Note bearbeiten",
        "mix.volume" => "Mixerpegel ändern",
        _ => "Bearbeitung",
    }
    .into()
}

pub fn show(app: &mut SoundApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    app.ai_studio.chat.sync_path(app.engine.path.clone());
    let before = app.ui.ai_settings.provider;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("ai_provider").selected_text(before.name()).show_ui(ui, |ui| {
            for p in [Provider::OpenAI, Provider::Anthropic, Provider::Google] {
                ui.selectable_value(&mut app.ui.ai_settings.provider, p, p.name());
            }
        });
        if before != app.ui.ai_settings.provider {
            app.ui.ai_settings.model = app.ui.ai_settings.provider.default_model().into();
        }
        egui::ComboBox::from_id_salt("ai_model").selected_text("Modell wählen").show_ui(ui, |ui| {
            for model in app.ui.ai_settings.provider.models() {
                ui.selectable_value(&mut app.ui.ai_settings.model, (*model).into(), *model);
            }
        });
    });
    ui.add(egui::TextEdit::singleline(&mut app.ui.ai_settings.model).hint_text("Modell-ID").desired_width(f32::INFINITY));
    let chat = &mut app.ai_studio.chat;
    chat.load_key(app.ui.ai_settings.provider);
    egui::CollapsingHeader::new("API-Schlüssel und Einstellungen").default_open(chat.secret.is_none()).show(ui, |ui| {
        ui.add(egui::TextEdit::singleline(&mut chat.api_input).password(true).hint_text("API-Schlüssel").desired_width(f32::INFINITY));
        ui.horizontal_wrapped(|ui| {
            if ui.button("Für diese Sitzung verwenden").clicked() {
                match Secret::new(std::mem::take(&mut chat.api_input)) {
                    Ok(key) => {
                        chat.secret = Some(key);
                        chat.status = "Schlüssel für diese Sitzung gesetzt".into();
                    }
                    Err(error) => chat.status = error,
                }
            }
            if credentials::available() && ui.button("Sicher speichern").clicked() {
                match Secret::new(std::mem::take(&mut chat.api_input)) {
                    Ok(key) => match credentials::save(app.ui.ai_settings.provider, &key) {
                        Ok(()) => {
                            chat.secret = Some(key);
                            chat.status = "Schlüssel sicher gespeichert".into();
                        }
                        Err(error) => {
                            chat.secret = Some(key);
                            chat.status = format!("{error}; Schlüssel gilt nur für diese Sitzung");
                        }
                    },
                    Err(error) => chat.status = error,
                }
            }
            if ui.button("Schlüssel entfernen").clicked() {
                match credentials::delete(app.ui.ai_settings.provider) {
                    Ok(()) => {
                        chat.secret = None;
                        credentials::clear_field(&mut chat.api_input);
                        chat.status = "Schlüssel entfernt".into();
                    }
                    Err(error) => chat.status = error,
                }
            }
        });
        ui.small(if chat.secret.is_some() { "API-Schlüssel verfügbar" } else { "Noch kein API-Schlüssel verfügbar" });
        ui.add(egui::Slider::new(&mut app.ui.ai_settings.max_output_tokens, 1024..=16384).text("Antwortlimit (Tokens)"));
        ui.small("Gesendete Nachrichten und der angezeigte Projektkontext gehen an den gewählten Anbieter.");
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt("ai_dialog").max_height(260.0).stick_to_bottom(true).show(ui, |ui| {
        if chat.conversation.turns.is_empty() {
            ui.label("Stelle eine Frage zum Projekt oder beschreibe eine gewünschte MIDI-Bearbeitung.");
        }
        for turn in &chat.conversation.turns {
            ui.group(|ui| {
                ui.strong(if turn.role == "user" { "Du" } else { "KI" });
                ui.label(&turn.text);
            });
        }
    });
    ui.add(egui::TextEdit::multiline(&mut chat.input).desired_rows(3).hint_text("Dein Auftrag …").desired_width(f32::INFINITY));
    ui.checkbox(&mut chat.edits_allowed, "Bearbeitungsvorschläge zulassen");
    let mut send_requested = false;
    let mut apply_requested = false;
    let mut undo_requested = false;
    ui.horizontal_wrapped(|ui| {
        send_requested =
            ui.add_enabled(!chat.busy() && chat.secret.is_some() && !chat.input.trim().is_empty(), egui::Button::new("Senden")).clicked();
        if ui.add_enabled(!chat.busy(), egui::Button::new("Neuer Dialog")).clicked() {
            chat.conversation = Conversation::default();
            chat.preview = None;
            chat.last_edit = None;
            chat.persist();
        }
        if chat.last_edit.is_some() {
            undo_requested = ui.button("KI-Bearbeitung rückgängig").clicked();
        }
        if chat.busy() {
            ui.spinner();
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    });
    let mut discard_requested = false;
    if let Some(preview) = &chat.preview {
        ui.group(|ui| {
            ui.strong(format!("Vorschlag: {} Schritte", preview.plan.actions.len()));
            for action in preview.plan.actions.iter().take(6) {
                ui.label(action_label(action));
            }
            if preview.plan.actions.len() > 6 {
                ui.small("Weitere Schritte sind im Vorschlag enthalten.");
            }
            apply_requested = ui.button("Vorschlag anwenden und prüfen").clicked();
            discard_requested = ui.button("Vorschlag verwerfen").clicked();
        });
    }
    if discard_requested {
        chat.preview = None;
    }
    let total = chat.conversation.receipts.iter().filter_map(|r| r.calculated_usd).sum::<f64>();
    if !chat.conversation.receipts.is_empty() {
        let unknown = chat.conversation.receipts.iter().filter(|r| r.calculated_usd.is_none()).count();
        ui.small(format!(
            "Dialogkosten: {total:.5} USD berechnet{}",
            if unknown > 0 { format!(" · {unknown} Anfrage(n) ohne hinterlegten Tarif") } else { String::new() }
        ));
        if let Some(receipt) = chat.conversation.receipts.last() {
            ui.small(format!(
                "{} · Eingabe {} / Ausgabe {} Tokens",
                receipt.model,
                receipt.usage.input.map(|v| v.to_string()).unwrap_or("unbekannt".into()),
                receipt.usage.output.map(|v| v.to_string()).unwrap_or("unbekannt".into())
            ));
        }
    }
    ui.label(&chat.status);
    if send_requested && let Err(error) = send(app, ctx.clone()) {
        app.ai_studio.chat.status = error;
    }
    if apply_requested && let Err(error) = apply_preview(app) {
        app.ai_studio.chat.status = error;
    }
    if undo_requested && let Err(error) = undo_last(app) {
        app.ai_studio.chat.status = error;
    }
}

#[cfg(test)]
pub fn set_test_key(state: &mut ChatState) {
    state.secret = Secret::new("test-key".into()).ok();
    state.loaded_provider = Some(Provider::OpenAI);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Services;
    use std::time::{Duration, Instant};
    struct Fixture;
    impl Transport for Fixture {
        fn post(&self, _: Provider, _: &str, key: &Secret, body: &Value) -> Result<Value, String> {
            assert_eq!(key.expose(), "test-key");
            assert!(!body.to_string().contains("test-key"));
            let text =
                json!({"reply":"Vorschlag für eine Spur", "actions":[{"command":"track.new","params":{"name":"AI Keys","kind":"instrument"}}]})
                    .to_string();
            Ok(
                json!({"status":"completed","output":[{"content":[{"type":"output_text","text":text}]}],"usage":{"input_tokens":100,"output_tokens":50}}),
            )
        }
    }
    fn app() -> SoundApp {
        let mut app = SoundApp::new(Engine::default(), None, Services::default());
        set_test_key(&mut app.ai_studio.chat);
        app.ai_studio.chat.input = "Neue Instrumentenspur".into();
        app.ai_studio.chat.edits_allowed = true;
        app
    }
    fn finish(app: &mut SoundApp) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.ai_studio.chat.busy() && Instant::now() < deadline {
            app.ai_studio.chat.poll();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(!app.ai_studio.chat.busy());
    }
    #[test]
    fn ai_chat_response_preview_apply_and_undo_use_live_engine() {
        let mut app = app();
        send_with(&mut app, egui::Context::default(), Fixture).unwrap();
        finish(&mut app);
        assert_eq!(app.engine.session().tracks.len(), 0);
        assert!(app.ai_studio.chat.preview.is_some());
        apply_preview(&mut app).unwrap();
        assert_eq!(app.engine.session().tracks.len(), 1);
        assert_eq!(app.run("session.inspect", json!({})).unwrap()["tracks"][0]["name"], "AI Keys");
        undo_last(&mut app).unwrap();
        assert_eq!(app.engine.session().tracks.len(), 0);
        assert!(!app.ai_studio.diagnostics().to_string().contains("test-key"));
        assert!(!serde_json::to_string(&app.ui).unwrap().contains("test-key"));
    }
    #[test]
    fn ai_chat_stale_revision_rejects_proposal_before_changes() {
        let mut app = app();
        send_with(&mut app, egui::Context::default(), Fixture).unwrap();
        finish(&mut app);
        app.run("track.new", json!({"name":"Manual","kind":"instrument"})).unwrap();
        assert!(apply_preview(&mut app).is_err());
        assert_eq!(app.engine.session().tracks.len(), 1);
    }
    #[test]
    fn ai_chat_project_replacement_discards_pending_edit() {
        let mut app = app();
        send_with(&mut app, egui::Context::default(), Fixture).unwrap();
        app.run("session.new", json!({"name":"Different"})).unwrap();
        finish(&mut app);
        assert!(app.ai_studio.chat.preview.is_none());
        assert!(app.ai_studio.chat.conversation.turns.is_empty());
        assert_eq!(app.engine.session().tracks.len(), 0);
    }
    #[test]
    fn ai_chat_context_strips_paths_and_rejects_secret_in_message() {
        let mut app = app();
        app.engine.path = Some("/private/song.soundcraft".into());
        assert!(!context(&mut app).unwrap().to_string().contains("/private"));
        app.ai_studio.chat.input = "test-key".into();
        assert!(send_with(&mut app, egui::Context::default(), Fixture).is_err());
        assert!(!app.ai_studio.chat.busy());
    }
}
