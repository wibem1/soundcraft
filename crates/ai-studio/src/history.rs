//! Local per-project conversations. Neither API secrets nor project snapshots are stored here.
use crate::assistant::{Receipt, Turn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub turns: Vec<Turn>,
    pub receipts: Vec<Receipt>,
}
fn filename(root: &Path, project: &str) -> PathBuf {
    let identity = if cfg!(target_os = "windows") { project.to_lowercase() } else { project.to_owned() };
    let digest = Sha256::digest(identity.as_bytes());
    root.join(format!("chat-{digest:x}.json"))
}
pub fn load(root: &Path, project: &str) -> Result<Conversation, String> {
    let path = filename(root, project);
    if !path.exists() {
        return Ok(Conversation::default());
    }
    if std::fs::metadata(&path).map_err(|_| "Dialogverlauf nicht lesbar")?.len() > 2 * 1024 * 1024 {
        return Err("Dialogverlauf ist zu groß".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Dialogverlauf nicht lesbar")?;
    let conversation: Conversation = serde_json::from_slice(&bytes).map_err(|_| "Dialogverlauf ist beschädigt")?;
    if conversation.turns.len() > 80 || conversation.receipts.len() > 500 {
        return Err("Dialogverlauf überschreitet das Limit".into());
    }
    Ok(conversation)
}
pub fn save(root: &Path, project: &str, conversation: &Conversation) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(conversation).map_err(|_| "Dialogverlauf konnte nicht erstellt werden")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Dialogverlauf ist zu groß".into());
    }
    std::fs::create_dir_all(root).map_err(|_| "Dialogordner konnte nicht erstellt werden")?;
    let path = filename(root, project);
    // Write a new file first; do not truncate an existing history on a failed write.
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|_| "Dialogverlauf konnte nicht gespeichert werden")?;
    if cfg!(target_os = "windows") && path.exists() {
        std::fs::remove_file(&path).map_err(|_| "Dialogverlauf konnte nicht ersetzt werden")?;
    }
    std::fs::rename(temporary, path).map_err(|_| "Dialogverlauf konnte nicht gespeichert werden".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_histories_roundtrip_and_remain_separate() {
        let root = std::env::temp_dir().join(format!("soundcraft-chat-history-{}", std::process::id()));
        let book = Conversation { turns: vec![Turn { role: "user".into(), text: "Meine Idee".into() }], receipts: Vec::new() };
        save(&root, "/private/first.soundcraft", &book).unwrap();
        assert_eq!(load(&root, "/private/first.soundcraft").unwrap().turns[0].text, "Meine Idee");
        assert!(load(&root, "/private/other.soundcraft").unwrap().turns.is_empty());
        assert!(!std::fs::read_to_string(filename(&root, "/private/first.soundcraft")).unwrap().contains("/private/first"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
