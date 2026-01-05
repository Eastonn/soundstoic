use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub lock_enabled: bool,
    pub locked_uid: Option<String>,
    pub start_at_login: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            locked_uid: None,
            start_at_login: false,
        }
    }
}

pub struct ConfigStore {
    path: PathBuf,
    data: Mutex<Config>,
}

impl ConfigStore {
    pub fn load() -> Self {
        let path = config_path();
        let config = read_config(&path).unwrap_or_default();
        Self {
            path,
            data: Mutex::new(config),
        }
    }

    pub fn get(&self) -> Config {
        self.data.lock().expect("config lock").clone()
    }

    pub fn update<F>(&self, f: F) -> Config
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.data.lock().expect("config lock");
        f(&mut config);
        let _ = write_config(&self.path, &config);
        config.clone()
    }
}

fn config_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("soundstoic").join("config.json")
}

fn read_config(path: &Path) -> io::Result<Config> {
    let data = fs::read_to_string(path)?;
    let cfg = serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(cfg)
}

fn write_config(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, data)
}
