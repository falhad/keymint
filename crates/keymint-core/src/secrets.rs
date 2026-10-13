use std::collections::HashMap;
use std::sync::Mutex;

use crate::config::ProviderKind;
use crate::error::Result;

/// Storage for API keys. The desktop shell implements this on top of the OS
/// keychain; keys never touch the config file or the frontend.
pub trait SecretStore: Send + Sync {
    fn get(&self, provider: ProviderKind) -> Result<Option<String>>;
    fn set(&self, provider: ProviderKind, key: &str) -> Result<()>;
    fn delete(&self, provider: ProviderKind) -> Result<()>;
}

/// In-memory store for tests.
#[derive(Default)]
pub struct MemorySecretStore(Mutex<HashMap<ProviderKind, String>>);

impl SecretStore for MemorySecretStore {
    fn get(&self, provider: ProviderKind) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap().get(&provider).cloned())
    }
    fn set(&self, provider: ProviderKind, key: &str) -> Result<()> {
        self.0.lock().unwrap().insert(provider, key.to_string());
        Ok(())
    }
    fn delete(&self, provider: ProviderKind) -> Result<()> {
        self.0.lock().unwrap().remove(&provider);
        Ok(())
    }
}
