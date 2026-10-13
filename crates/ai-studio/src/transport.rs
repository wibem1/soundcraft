//! Loopback-only, bounded JSON-lines client. A failed edit is never automatically replayed.
use serde_json::{Value, json};
use soundcraft_automation::Backend;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

const MAX_BYTES: u64 = 4 * 1024 * 1024;
pub struct LocalControl {
    port: u16,
    connection: Option<BufReader<TcpStream>>,
    next_id: u64,
}
impl LocalControl {
    pub fn new(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("a listening control port is required".into());
        }
        Ok(Self { port, connection: None, next_id: 1 })
    }
    fn roundtrip(&mut self, method: &str, params: Value) -> Result<Value, String> {
        if self.connection.is_none() {
            let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
            let stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|e| e.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
            stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
            self.connection = Some(BufReader::new(stream));
        }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or("request id exhausted")?;
        let request = json!({"id": id, "method": method, "params": params}).to_string();
        if request.len() as u64 >= MAX_BYTES {
            return Err("control request exceeds limit".into());
        }
        let connection = self.connection.as_mut().ok_or("not connected")?;
        writeln!(connection.get_mut(), "{request}").map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        connection.take(MAX_BYTES).read_until(b'\n', &mut bytes).map_err(|e| e.to_string())?;
        if bytes.last() != Some(&b'\n') {
            return Err("control reply closed, timed out or exceeds limit; action outcome may be unknown".into());
        }
        let reply: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if reply.get("id").and_then(Value::as_u64) != Some(id) {
            return Err("control reply id mismatch; action outcome may be unknown".into());
        }
        match reply.get("ok").and_then(Value::as_bool) {
            Some(true) => reply.get("result").cloned().ok_or_else(|| "control result missing".into()),
            Some(false) => Err(reply.get("error").and_then(Value::as_str).unwrap_or("control command failed").to_owned()),
            None => Err("invalid control reply".into()),
        }
    }
}
impl Backend for LocalControl {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let result = self.roundtrip(method, params);
        if result.is_err() {
            self.connection = None;
        }
        result
    }
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        format!("SoundCraft loopback control port {}", self.port)
    }
}
