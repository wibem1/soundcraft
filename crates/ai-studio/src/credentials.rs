//! Provider secrets are never serialized. Persistent entries use native OS stores only.
use crate::assistant::Provider;
use zeroize::Zeroizing;

pub struct Secret(Zeroizing<String>);
impl Secret {
    pub fn new(value: String) -> Result<Self, String> {
        let value = Zeroizing::new(value);
        if value.trim().is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
            return Err("API-Schlüssel ist leer oder ungültig".into());
        }
        Ok(Self(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn duplicate(&self) -> Self {
        Self(Zeroizing::new(self.0.to_string()))
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn entry(provider: Provider) -> Result<keyring::Entry, String> {
    keyring::Entry::new("SoundCraft-AI-Studio", provider.name()).map_err(|_| "Sichere Schlüsselablage nicht verfügbar".into())
}
pub fn available() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}
pub fn load(provider: Provider) -> Result<Option<Secret>, String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        match entry(provider)?.get_password() {
            Ok(value) => Secret::new(value).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("API-Schlüssel konnte nicht sicher geladen werden".into()),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = provider;
        Ok(None)
    }
}
pub fn save(provider: Provider, secret: &Secret) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        entry(provider)?.set_password(secret.expose()).map_err(|_| "API-Schlüssel konnte nicht sicher gespeichert werden".into())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (provider, secret);
        Err("Auf diesem System ist nur die Verwendung für diese Sitzung verfügbar".into())
    }
}
pub fn delete(provider: Provider) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        match entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("Gespeicherter Schlüssel konnte nicht gelöscht werden".into()),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = provider;
        Ok(())
    }
}

pub fn clear_field(value: &mut String) {
    use zeroize::Zeroize;
    value.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_field_is_cleared_and_invalid_headers_are_rejected() {
        let mut field = "private-api-key".to_owned();
        clear_field(&mut field);
        assert!(field.is_empty());
        assert!(Secret::new("bad\nkey".into()).is_err());
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn native_os_store_can_roundtrip_and_delete_isolated_test_entry() {
        let user = format!("test-{}", std::process::id());
        let entry = keyring::Entry::new("SoundCraft-AI-Studio-CI-Test", &user).unwrap();
        entry.set_password("fixture-not-a-real-api-key").unwrap();
        let loaded = entry.get_password();
        let deleted = entry.delete_credential();
        assert_eq!(loaded.unwrap(), "fixture-not-a-real-api-key");
        deleted.unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }
}
