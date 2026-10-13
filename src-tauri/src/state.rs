use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use keymint_core::ai::AiService;
use keymint_core::commands::default_providers;
use keymint_core::config::Config;
use keymint_core::history::History;
use keymint_core::registry::{CommandProvider, Frecency, Registry};
use tauri::{AppHandle, Manager};

use crate::secrets::KeyringStore;

/// Ignore blur events this soon after showing: some window managers send a
/// spurious focus-out while the palette is being mapped.
const BLUR_GRACE: Duration = Duration::from_millis(350);

pub struct AppState {
    pub config: RwLock<Config>,
    pub registry: RwLock<Arc<Registry>>,
    pub providers: Vec<Box<dyn CommandProvider>>,
    pub ai: Arc<AiService>,
    pub usage: Mutex<Frecency>,
    pub history: Mutex<History>,
    pub config_path: PathBuf,
    pub usage_path: PathBuf,
    pub history_path: PathBuf,
    /// Problems worth showing in Settings (bad config file, shortcut conflicts).
    pub warnings: Mutex<Vec<String>>,
    pub shortcut_errors: Mutex<Vec<String>>,
    last_shown: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let dir = app.path().app_config_dir()?;
        let config_path = dir.join("config.json");
        let usage_path = dir.join("usage.json");
        let history_path = dir.join("history.json");

        let mut warnings = Vec::new();
        let config = match Config::load(&config_path) {
            Ok(c) => c,
            Err(e) => {
                // Keep the broken file so nothing the user wrote is lost.
                let backup = config_path.with_extension("json.bak");
                let moved = std::fs::rename(&config_path, &backup).is_ok();
                warnings.push(format!(
                    "{e}. Using defaults{}",
                    if moved { format!("; the old file was saved as {}", backup.display()) } else { String::new() }
                ));
                Config::default()
            }
        };

        let providers = default_providers();
        let registry = Arc::new(Registry::build(&providers, &config));
        let ai = Arc::new(AiService::new(Arc::new(KeyringStore))?);

        Ok(Self {
            usage: Mutex::new(Frecency::load(&usage_path)),
            history: Mutex::new(History::load(&history_path)),
            config: RwLock::new(config),
            registry: RwLock::new(registry),
            providers,
            ai,
            config_path,
            usage_path,
            history_path,
            warnings: Mutex::new(warnings),
            shortcut_errors: Mutex::new(Vec::new()),
            last_shown: Mutex::new(None),
        })
    }

    pub fn config(&self) -> Config {
        self.config.read().expect("config lock").clone()
    }

    pub fn registry(&self) -> Arc<Registry> {
        self.registry.read().expect("registry lock").clone()
    }

    /// Apply a new config: swap it in and rebuild the registry from all providers.
    pub fn set_config(&self, cfg: Config) {
        let registry = Arc::new(Registry::build(&self.providers, &cfg));
        *self.config.write().expect("config lock") = cfg;
        *self.registry.write().expect("registry lock") = registry;
    }

    pub fn mark_shown(&self) {
        *self.last_shown.lock().expect("lock") = Some(Instant::now());
    }

    pub fn recently_shown(&self) -> bool {
        self.last_shown.lock().expect("lock").is_some_and(|t| t.elapsed() < BLUR_GRACE)
    }
}
