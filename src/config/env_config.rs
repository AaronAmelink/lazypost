use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persistent store for environment variables. Lives in its own file
/// (default `env.json`) so it can be gitignored — values frequently contain
/// secrets like API tokens that should never end up in a shared workspace
/// file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EnvFile {
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub data: EnvFile,
    pub path: PathBuf,
}

impl EnvConfig {
    pub fn load(path: &Path) -> Self {
        let data = if path.exists() {
            File::open(path)
                .ok()
                .map(BufReader::new)
                .and_then(|r| serde_json::from_reader(r).ok())
                .unwrap_or_default()
        } else {
            EnvFile::default()
        };
        Self {
            data,
            path: path.to_path_buf(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(std::io::Error::other)?;
        fs::write(&self.path, json)
    }
}
