use crate::model::items::Item;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// On-disk format for `workspace.json`. Contains the request tree only.
/// Environment variables live in `env.json` so secrets don't end up committed.
/// Unknown fields in older workspace files (e.g. legacy `environments`) are
/// silently ignored by serde.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkspaceFile {
    pub items: Vec<Item>,
}

impl WorkspaceFile {
    fn empty() -> Self {
        Self { items: vec![] }
    }
}

#[derive(Debug)]
pub struct WorkspaceConfig {
    pub data: WorkspaceFile,
    pub path: PathBuf,
}

impl WorkspaceConfig {
    pub fn new_empty() -> Self {
        Self {
            data: WorkspaceFile::empty(),
            path: PathBuf::new(),
        }
    }

    pub fn create_from_file(path: &Path) -> std::io::Result<Self> {
        let data = if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_else(|_| WorkspaceFile::empty())
        } else {
            WorkspaceFile::empty()
        };

        let cfg = Self {
            data,
            path: path.to_path_buf(),
        };

        if !path.exists() {
            cfg.save()?;
        }

        Ok(cfg)
    }

    pub fn save_items_to_file(items: Vec<Item>, path: &Path) -> std::io::Result<()> {
        let cfg = Self {
            data: WorkspaceFile { items },
            path: path.to_path_buf(),
        };
        cfg.save()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(std::io::Error::other)?;
        fs::write(&self.path, json)
    }

    pub(crate) fn remove_from_items(
        items: &mut Vec<Item>,
        path: &[usize],
    ) -> Result<Item, &'static str> {
        if path.is_empty() {
            return Err("Path is empty");
        }
        if path.len() == 1 {
            if path[0] >= items.len() {
                return Err("Path out of bounds");
            }
            return Ok(items.remove(path[0]));
        }
        let first = path[0];
        if first >= items.len() {
            return Err("Path out of bounds");
        }
        match &mut items[first] {
            Item::Folder(folder) => Self::remove_from_items(&mut folder.items, &path[1..]),
            Item::Request(_) => Err("Path out of bounds"),
        }
    }
}
