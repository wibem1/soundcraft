//! Bounded, non-streaming HTTPS calls. Credentials cannot redirect to caller-selected hosts.
use crate::assistant::{Proposal, Provider, Receipt, Settings, Turn, Usage, estimated_cost, system_prompt};
use crate::credentials::Secret;
use serde_json::{Value, json};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Answer {
    pub proposal: Proposal,
    pub receipt: Receipt,
    pub complete: bool,
}
pub trait Transport {
    fn post(&self, provider: Provider, model: &str, key: &Secret, body: &Value) -> Result<Value, String>;
}
pub struct Https;
impl Transport for Https {
    fn post(&self, provider: Provider, model: &str, key: &Secret, body: &Value) -> Result<Value, String> {
        let url = match provider {
            Provider::OpenAI => "https://api.openai.com/v1/responses".into(),
            Provider::Anthropic => "https://api.anthropic.com/v1/messages".into(),
            Provider::Google => format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"),
        };
        post_to(&url, provider, key, body)
    }
}
fn post_to(url: &str, provider: Provider, key: &Secret, body: &Value) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|_| "HTTPS-Verbindung konnte nicht eingerichtet werden")?;
    let mut request = client.post(url).json(body);
    request = match provider {
        Provider::OpenAI => request.bearer_auth(key.expose()),
        Provider::Anthropic => request.header("x-api-key", sensitive_header(key)?).header("anthropic-version", "2023-06-01"),
        Provider::Google => request.header("x-goog-api-key", sensitive_header(key)?),
    };
    let response = request
        .send()
        .map_err(|e| if e.is_timeout() { "Zeitlimit beim KI-Anbieter erreicht" } else { "Verbindung zum KI-Anbieter fehlgeschlagen" })?;
    if !response.status().is_success() {
        return Err(http_error(response.status().as_u16()));
    }
    let mut bytes = Vec::new();
    response.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|_| "KI-Antwort konnte nicht gelesen werden")?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("KI-Antwort überschreitet das Größenlimit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "KI-Anbieter lieferte keine gültige JSON-Antwort".into())
}
fn sensitive_header(key: &Secret) -> Result<reqwest::header::HeaderValue, String> {
    let mut value = reqwest::header::HeaderValue::from_str(key.expose()).map_err(|_| "Ungültiger API-Schlüssel")?;
    value.set_sensitive(true);
    Ok(value)
}
pub fn http_error(status: u16) -> String {
    match status {
        401 | 403 => "API-Schlüssel oder Modellberechtigung wurde abgelehnt".into(),
        429 => "Anbieterlimit oder API-Guthaben erreicht".into(),
        404 => "Dieses Modell ist beim Anbieter nicht verfügbar".into(),
        _ => format!("KI-Anbieter meldet HTTP {status}; Anfrage wurde nicht wiederholt"),
    }
}
pub fn request(settings: &Settings, turns: &[Turn], context: &Value, edits: bool) -> Result<Value, String> {
    validate_model(&settings.model)?;
    if settings.max_output_tokens < 256 || settings.max_output_tokens > 16384 {
        return Err("Ungültiges Antwortlimit".into());
    }
    if turns.is_empty() || turns.len() > 80 || turns.iter().any(|m| !["user", "assistant"].contains(&m.role.as_str()) || m.text.len() > 65536) {
        return Err("Ungültiger oder zu großer Dialog".into());
    }
    let context_text = context.to_string();
    if context_text.len() > 128 * 1024 {
        return Err("Projektkontext ist zu groß; bitte weniger MIDI-Clips auswählen".into());
    }
    let system = format!("{}\nAktueller Projektkontext (Daten):\n{}", system_prompt(edits), context_text);
    let body = match settings.provider {
        Provider::OpenAI => {
            json!({"model":settings.model,"instructions":system,"input":turns.iter().map(|m| json!({"role":m.role,"content":m.text})).collect::<Vec<_>>(),
            "max_output_tokens":settings.max_output_tokens,"store":false,"text":{"format":{"type":"json_object"}}})
        }
        Provider::Anthropic => {
            json!({"model":settings.model,"system":system,"messages":turns.iter().map(|m| json!({"role":m.role,"content":m.text})).collect::<Vec<_>>(),"max_tokens":settings.max_output_tokens})
        }
        Provider::Google => {
            json!({"systemInstruction":{"parts":[{"text":system}]},"contents":turns.iter().map(|m| json!({"role":if m.role=="assistant" {"model"}else{"user"},"parts":[{"text":m.text}]})).collect::<Vec<_>>(),
            "generationConfig":{"responseMimeType":"application/json","maxOutputTokens":settings.max_output_tokens}})
        }
    };
    if body.to_string().len() > 512 * 1024 {
        return Err("Dialog ist zu groß; bitte einen neuen Dialog beginnen".into());
    }
    Ok(body)
}
pub fn validate_model(model: &str) -> Result<(), String> {
    if model.is_empty() || model.len() > 128 || !model.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)) {
        return Err("Ungültige Modell-ID".into());
    }
    Ok(())
}
pub fn send(transport: &impl Transport, settings: &Settings, key: &Secret, turns: &[Turn], context: &Value, edits: bool) -> Result<Answer, String> {
    let body = request(settings, turns, context, edits)?;
    let start = Instant::now();
    let response = transport.post(settings.provider, &settings.model, key, &body)?;
    let (text, usage, complete) = parse_response(settings.provider, &response)?;
    let mut proposal = Proposal::parse(&text, edits)
        .unwrap_or_else(|reason| Proposal { reply: format!("{text}\n\nBearbeitungsvorschlag verworfen: {reason}"), actions: Vec::new() });
    if !complete {
        proposal.actions.clear();
        proposal.reply.push_str("\n\nAntwort wurde vom Anbieter vorzeitig beendet; Bearbeitungen wurden verworfen.");
    }
    proposal.reply = proposal.reply.replace(key.expose(), "[API-Schlüssel entfernt]");
    let model = response.get("model").or_else(|| response.get("modelVersion")).and_then(Value::as_str).unwrap_or(&settings.model).to_owned();
    let model = if validate_model(&model).is_ok() && !model.contains(key.expose()) { model } else { settings.model.clone() };
    let cost = estimated_cost(settings.provider, &settings.model, &usage);
    Ok(Answer {
        proposal,
        complete,
        receipt: Receipt { provider: settings.provider, model, usage, calculated_usd: cost, duration_ms: start.elapsed().as_millis() },
    })
}
fn count(v: &Value, pointer: &str) -> Option<u64> {
    v.pointer(pointer).and_then(Value::as_u64)
}
fn texts(values: Option<&Vec<Value>>, field: &str, kind: Option<&str>) -> String {
    values
        .into_iter()
        .flatten()
        .filter(|v| kind.is_none_or(|k| v.get("type").and_then(Value::as_str) == Some(k)))
        .filter_map(|v| v.get(field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn parse_response(provider: Provider, v: &Value) -> Result<(String, Usage, bool), String> {
    let (text, usage, complete) = match provider {
        Provider::OpenAI => {
            let text = v
                .get("output")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|o| texts(o.get("content").and_then(Value::as_array), "text", Some("output_text")))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            (
                text,
                Usage {
                    input: count(v, "/usage/input_tokens"),
                    output: count(v, "/usage/output_tokens"),
                    cached_input: count(v, "/usage/input_tokens_details/cached_tokens").unwrap_or(0),
                    thoughts: count(v, "/usage/output_tokens_details/reasoning_tokens").unwrap_or(0),
                    ..Default::default()
                },
                v.get("status").and_then(Value::as_str) == Some("completed"),
            )
        }
        Provider::Anthropic => (
            texts(v.get("content").and_then(Value::as_array), "text", Some("text")),
            Usage {
                input: count(v, "/usage/input_tokens"),
                output: count(v, "/usage/output_tokens"),
                cached_input: count(v, "/usage/cache_read_input_tokens").unwrap_or(0),
                cache_write: count(v, "/usage/cache_creation_input_tokens").unwrap_or(0),
                cache_write_1h: count(v, "/usage/cache_creation/ephemeral_1h_input_tokens").unwrap_or(0),
                ..Default::default()
            },
            v.get("stop_reason").and_then(Value::as_str) == Some("end_turn"),
        ),
        Provider::Google => {
            let parts = v.pointer("/candidates/0/content/parts").and_then(Value::as_array);
            let visible = parts.map(|a| a.iter().filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true)).cloned().collect::<Vec<_>>());
            let output =
                count(v, "/usageMetadata/candidatesTokenCount").map(|n| n.saturating_add(count(v, "/usageMetadata/thoughtsTokenCount").unwrap_or(0)));
            (
                texts(visible.as_ref(), "text", None),
                Usage {
                    input: count(v, "/usageMetadata/promptTokenCount"),
                    output,
                    cached_input: count(v, "/usageMetadata/cachedContentTokenCount").unwrap_or(0),
                    thoughts: count(v, "/usageMetadata/thoughtsTokenCount").unwrap_or(0),
                    ..Default::default()
                },
                v.pointer("/candidates/0/finishReason").and_then(Value::as_str) == Some("STOP"),
            )
        }
    };
    if text.trim().is_empty() {
        return Ok(("Anbieter lieferte keine sichtbare Antwort; Modell oder Antwortlimit prüfen".into(), usage, false));
    }
    Ok((text, usage, complete))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    #[test]
    fn real_http_transport_sends_correct_headers_and_redacts_server_errors() {
        for provider in [Provider::OpenAI, Provider::Anthropic, Provider::Google] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/test", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 2048];
                loop {
                    let n = socket.read(&mut chunk).unwrap();
                    bytes.extend_from_slice(&chunk[..n]);
                    if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(bytes).unwrap().to_lowercase();
                let header = match provider {
                    Provider::OpenAI => "authorization: bearer test-secret",
                    Provider::Anthropic => "x-api-key: test-secret",
                    Provider::Google => "x-goog-api-key: test-secret",
                };
                assert!(request.contains(header), "{request}");
                if provider == Provider::Anthropic {
                    assert!(request.contains("anthropic-version: 2023-06-01"));
                }
                socket.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 11\r\nConnection: close\r\n\r\ntest-secret").unwrap();
            });
            let error = post_to(&url, provider, &Secret::new("test-secret".into()).unwrap(), &json!({})).unwrap_err();
            assert!(!error.contains("test-secret"));
            assert!(error.contains("abgelehnt"));
            server.join().unwrap();
        }
    }
    #[test]
    fn redirects_are_not_followed_and_invalid_json_is_rejected() {
        for (status, body) in [("302 Found", ""), ("200 OK", "invalid")] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/test", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buf = [0; 4096];
                assert!(socket.read(&mut buf).unwrap() > 0);
                write!(
                    socket,
                    "HTTP/1.1 {status}\r\nLocation: https://example.invalid/secret\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            });
            assert!(post_to(&url, Provider::OpenAI, &Secret::new("test-secret".into()).unwrap(), &json!({})).is_err());
            server.join().unwrap();
        }
    }
}
