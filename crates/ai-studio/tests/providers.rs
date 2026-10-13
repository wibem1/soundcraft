use serde_json::{Value, json};
use soundcraft_ai_studio::{
    Bridge,
    assistant::{self, Action, Proposal, Provider, Settings, Turn, Usage},
    credentials::Secret,
    provider::{self, Transport},
};
use soundcraft_automation::Headless;
use soundcraft_engine::Engine;

fn turn() -> Vec<Turn> {
    vec![Turn { role: "user".into(), text: "Beschreibe das Projekt".into() }]
}
#[test]
fn each_provider_uses_its_actual_protocol_without_serializing_keys() {
    for provider in [Provider::OpenAI, Provider::Anthropic, Provider::Google] {
        let settings = Settings { provider, model: provider.default_model().into(), ..Default::default() };
        let body = provider::request(&settings, &turn(), &json!({"tracks":[]}), false).unwrap();
        assert!(!body.to_string().contains("api_key"));
        match provider {
            Provider::OpenAI => {
                assert_eq!(body["store"], false);
                assert!(body.get("input").is_some());
            }
            Provider::Anthropic => {
                assert!(body.get("system").is_some());
                assert!(body.get("max_tokens").is_some());
            }
            Provider::Google => {
                assert_eq!(body["contents"][0]["role"], "user");
                assert_eq!(body["generationConfig"]["responseMimeType"], "application/json");
            }
        }
    }
    assert!(provider::validate_model("gemini/x?key=secret").is_err());
}
#[test]
fn provider_usage_counts_cached_and_thinking_tokens_correctly() {
    let openai = json!({"status":"completed","output":[{"content":[{"type":"output_text","text":"hello"}]}],"usage":{"input_tokens":1000,"output_tokens":100,"input_tokens_details":{"cached_tokens":200},"output_tokens_details":{"reasoning_tokens":80}}});
    let (text, u, done) = provider::parse_response(Provider::OpenAI, &openai).unwrap();
    assert_eq!(text, "hello");
    assert!(done);
    assert_eq!(u.output, Some(100));
    assert_eq!(u.thoughts, 80);
    assert!((assistant::estimated_cost(Provider::OpenAI, "gpt-6.1-sol", &u).unwrap() - 0.00262).abs() < 1e-9);
    let claude = json!({"stop_reason":"end_turn","content":[{"type":"thinking","thinking":"private"},{"type":"text","text":"answer"}],"usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":1000}});
    let (text, u, _) = provider::parse_response(Provider::Anthropic, &claude).unwrap();
    assert_eq!(text, "answer");
    assert!((assistant::estimated_cost(Provider::Anthropic, "claude-sonnet-5-5", &u).unwrap() - 0.0008).abs() < 1e-9);
    let gemini = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"thought":true,"text":"private"},{"text":"answer"}]}}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":20,"thoughtsTokenCount":30}});
    let (text, u, _) = provider::parse_response(Provider::Google, &gemini).unwrap();
    assert_eq!(text, "answer");
    assert_eq!(u.output, Some(50));
    assert!(assistant::estimated_cost(Provider::Google, "unknown", &u).is_none());
    assert!(assistant::estimated_cost(Provider::OpenAI, "gpt-6.1-sol", &Usage::default()).is_none());
}
struct Fixture;
impl Transport for Fixture {
    fn post(&self, p: Provider, _: &str, key: &Secret, body: &Value) -> Result<Value, String> {
        assert_eq!(key.expose(), "sk-private-key");
        assert!(!body.to_string().contains("sk-private-key"));
        let text =
            json!({"reply":"Ich schlage eine neue Spur vor.","actions":[{"command":"track.new","params":{"name":"Keys","kind":"instrument"}}]})
                .to_string();
        Ok(match p {
            Provider::OpenAI => {
                json!({"model":"gpt-6.1-sol","status":"completed","output":[{"content":[{"type":"output_text","text":text}]}],"usage":{"input_tokens":10,"output_tokens":30}})
            }
            Provider::Anthropic => {
                json!({"model":"claude-sonnet-5-5","stop_reason":"end_turn","content":[{"type":"text","text":text}],"usage":{"input_tokens":10,"output_tokens":30}})
            }
            Provider::Google => {
                json!({"modelVersion":"gemini-3.8-flash","candidates":[{"finishReason":"STOP","content":{"parts":[{"text":text}]}}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":30}})
            }
        })
    }
}
#[test]
fn all_three_responses_can_propose_and_apply_a_real_verified_edit() {
    for p in [Provider::OpenAI, Provider::Anthropic, Provider::Google] {
        let settings = Settings { provider: p, model: p.default_model().into(), ..Default::default() };
        let answer = provider::send(&Fixture, &settings, &Secret::new("sk-private-key".into()).unwrap(), &turn(), &json!({}), true).unwrap();
        let mut bridge = Bridge::new(Headless::new(Engine::default()));
        assistant::apply(&mut bridge, &answer.proposal).unwrap();
        assert_eq!(bridge.inspect().unwrap()["tracks"][0]["name"], "Keys");
        assert!(!serde_json::to_string(&answer.receipt).unwrap().contains("sk-private-key"));
    }
}
#[test]
fn command_references_and_midi_postconditions_work_on_actual_engine() {
    let plan = Proposal {
        reply: "Notes".into(),
        actions: vec![
            Action { command: "track.new".into(), params: json!({"name":"Keys","kind":"instrument"}) },
            Action { command: "midi.new_clip".into(), params: json!({"track":"Keys","start":0,"length":{"seconds":4.0}}) },
            Action {
                command: "midi.note_add".into(),
                params: json!({"clip":"$result.1.clip","pitch":60,"start_ticks":0,"length_ticks":960,"velocity":80}),
            },
            Action { command: "mix.volume".into(), params: json!({"track":"Keys","db":-6.0}) },
        ],
    };
    let mut bridge = Bridge::new(Headless::new(Engine::default()));
    let result = assistant::apply(&mut bridge, &plan).unwrap();
    assert_eq!(result["applied"], 4);
    let clip = result["results"][1]["clip"].clone();
    let notes = bridge.execute("midi.notes", json!({"clip":clip}), soundcraft_ai_studio::Authority::ReadOnly).unwrap();
    assert_eq!(notes["notes"][0]["pitch"], 60);
    assert_eq!(bridge.inspect().unwrap()["tracks"][0]["volume_db"], -6.0);
}
#[test]
fn failed_edit_rolls_back_created_tracks() {
    let plan = Proposal {
        reply: "".into(),
        actions: vec![
            Action { command: "track.new".into(), params: json!({"name":"Temporary","kind":"audio"}) },
            Action { command: "midi.new_clip".into(), params: json!({"track":"Temporary"}) },
        ],
    };
    let mut bridge = Bridge::new(Headless::new(Engine::default()));
    assert!(assistant::apply(&mut bridge, &plan).is_err());
    assert_eq!(bridge.inspect().unwrap()["tracks"], json!([]));
}
#[test]
fn file_recording_and_out_of_range_actions_are_rejected() {
    for command in ["session.open", "session.save", "transport.record", "arbitrary"] {
        assert!(Proposal { reply: "".into(), actions: vec![Action { command: command.into(), params: json!({}) }] }.validate(true).is_err());
    }
    assert!(
        Proposal { reply: "".into(), actions: vec![Action { command: "mix.volume".into(), params: json!({"track":"Keys","db":200}) }] }
            .validate(true)
            .is_err()
    );
    assert!(Proposal::parse(&json!({"reply":"","actions":[{"command":"track.new","params":{"name":"X"}}]}).to_string(), false).is_err());
    assert!(assistant::resolve(&json!("$result.9.clip"), &[], 0).is_err());
}

#[test]
fn incomplete_or_invalid_proposals_preserve_usage_without_applying_edits() {
    struct Response(Value);
    impl Transport for Response {
        fn post(&self, _: Provider, _: &str, _: &Secret, _: &Value) -> Result<Value, String> {
            Ok(self.0.clone())
        }
    }
    let key = Secret::new("sk-private-key".into()).unwrap();
    for (status, command) in [("incomplete", "track.new"), ("completed", "session.save")] {
        let text = json!({"reply":"sk-private-key","actions":[{"command":command,"params":{"name":"Keys"}}]}).to_string();
        let response = Response(
            json!({"status":status,"output":[{"content":[{"type":"output_text","text":text}]}],"usage":{"input_tokens":10,"output_tokens":5}}),
        );
        let answer = provider::send(&response, &Settings::default(), &key, &turn(), &json!({}), true).unwrap();
        assert!(answer.proposal.actions.is_empty());
        assert_eq!(answer.receipt.usage.output, Some(5));
        assert!(answer.receipt.calculated_usd.is_some());
        assert!(!answer.proposal.reply.contains("sk-private-key"));
    }
}
#[test]
fn anthropic_hour_cache_writes_use_their_own_rate() {
    let usage = Usage { input: Some(100), output: Some(50), cache_write: 1000, cache_write_1h: 400, ..Default::default() };
    let cost = assistant::estimated_cost(Provider::Anthropic, "claude-sonnet-5-5", &usage).unwrap();
    assert!((cost - 0.0038).abs() < 1e-9);
}
