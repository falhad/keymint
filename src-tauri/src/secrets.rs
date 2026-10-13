use keymint_core::config::ProviderKind;
use keymint_core::secrets::SecretStore;
use keymint_core::{Error, Result};

const SERVICE: &str = "dev.keymint.app";

/// API keys live in the OS keychain (Keychain, Credential Manager, Secret Service).
pub struct KeyringStore;

fn entry(kind: ProviderKind) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("api-key-{}", kind.as_str())).map_err(|e| Error::Secret(e.to_string()))
}

impl SecretStore for KeyringStore {
    fn get(&self, provider: ProviderKind) -> Result<Option<String>> {
        match entry(provider)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }

    fn set(&self, provider: ProviderKind, key: &str) -> Result<()> {
        entry(provider)?.set_password(key).map_err(|e| Error::Secret(e.to_string()))
    }

    fn delete(&self, provider: ProviderKind) -> Result<()> {
        match entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }
}
