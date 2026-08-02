//! Secure storage of secrets using the OS credential store.
//!
//! API keys must never be persisted in the plain SQLite settings table. This
//! module routes secret values through the macOS Keychain (via `keyring`),
//! keeping the public command interface unchanged so the frontend does not
//! need to know where the value is stored.
//!
//! ## Security considerations
//!
//! - The `api_key` setting is stored in the Keychain, never in SQLite.
//! - `list_settings_command` masks secret values so they are not leaked to
//!   the UI or any future API consumer.
//! - The [`SecretStore`] trait keeps command handlers decoupled from the OS
//!   credential store so they can be exercised with an in-memory double in
//!   integration tests (see the command test harness).

use anyhow::{Context, Result};

/// Service name used for all EchoNote Keychain entries.
pub const KEYCHAIN_SERVICE: &str = "com.huynhdung.echo-note";

/// Account name used for the OpenAI-compatible API key.
pub const API_KEY_ACCOUNT: &str = "api_key";

/// Settings keys whose values are secrets and must never live in SQLite.
pub const SECRET_SETTING_KEYS: &[&str] = &[API_KEY_ACCOUNT];

/// Abstraction over the OS credential store so command handlers can be
/// tested with an in-memory double instead of touching the real Keychain.
pub trait SecretStore: Send + Sync {
    /// Read a secret, returning `None` when no entry exists.
    fn get(&self, account: &str) -> Result<Option<String>>;

    /// Create or update a secret.
    fn set(&self, account: &str, value: &str) -> Result<()>;

    /// Remove a secret. Returns `Ok(false)` when the entry does not exist.
    fn delete(&self, account: &str) -> Result<bool>;
}

/// Production store backed by the macOS Keychain.
#[derive(Debug, Default)]
pub struct KeychainStore;

impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, account)
            .context("Failed to create Keychain entry")?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to read Keychain entry: {e}")),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, account)
            .context("Failed to create Keychain entry")?;
        entry
            .set_password(value)
            .context("Failed to write Keychain entry")?;
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<bool> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, account)
            .context("Failed to create Keychain entry")?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to delete Keychain entry: {e}")),
        }
    }
}

/// In-memory store used as a test double. Kept in production code so the
/// command integration test harness can construct [`crate::AppStateExt`]
/// without touching the real Keychain. Referenced by unit and integration
/// tests, hence the explicit allow for the non-test build.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    entries: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemorySecretStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for InMemorySecretStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock in-memory store: {e}"))?;
        Ok(entries.get(account).cloned())
    }

    fn set(&self, account: &str, value: &str) -> Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock in-memory store: {e}"))?;
        entries.insert(account.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<bool> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock in-memory store: {e}"))?;
        Ok(entries.remove(account).is_some())
    }
}

/// Returns true when a settings key holds a secret that must be routed to
/// the credential store instead of SQLite.
pub fn is_secret_setting(key: &str) -> bool {
    SECRET_SETTING_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_round_trips_secrets() {
        let store = InMemorySecretStore::new();

        assert!(store.get("api_key").unwrap().is_none());

        store.set("api_key", "sk-test-123").unwrap();
        assert_eq!(store.get("api_key").unwrap().as_deref(), Some("sk-test-123"));

        assert!(store.delete("api_key").unwrap());
        assert!(store.get("api_key").unwrap().is_none());
        assert!(!store.delete("api_key").unwrap());
    }

    #[test]
    fn in_memory_store_keeps_accounts_independent() {
        let store = InMemorySecretStore::new();
        store.set("api_key", "one").unwrap();
        store.set("other", "two").unwrap();

        assert_eq!(store.get("api_key").unwrap().as_deref(), Some("one"));
        assert_eq!(store.get("other").unwrap().as_deref(), Some("two"));
        assert!(store.get("missing").unwrap().is_none());
    }

    #[test]
    fn api_key_is_recognised_as_secret_setting() {
        assert!(is_secret_setting("api_key"));
        assert!(!is_secret_setting("whisper_model_size"));
        assert!(!is_secret_setting("audio_device"));
    }
}
