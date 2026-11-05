use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentLibrary {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub default_library_directory: String,
    #[serde(default)]
    pub recent_libraries: Vec<RecentLibrary>,
    #[serde(default)]
    pub last_opened_path: Option<String>,
}

const MAX_RECENT: usize = 10;

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "fuzz")
}

pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    config_dir().join("app_config.json")
}

/// Where the optional ONNX embedding model lives.
pub fn model_dir() -> PathBuf {
    config_dir().join("models").join("embeddinggemma")
}

fn default_library_dir() -> PathBuf {
    UserDirs::new()
        .and_then(|u| u.document_dir().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("FuzzLibraries")
}

impl AppConfig {
    pub fn load() -> Self {
        let mut cfg: AppConfig = std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default();
        if cfg.default_library_directory.is_empty() {
            cfg.default_library_directory = default_library_dir().to_string_lossy().into_owned();
        }
        // Drop recents that no longer exist on disk.
        cfg.recent_libraries
            .retain(|r| std::path::Path::new(&r.path).exists());
        cfg
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config_dir());
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), s);
        }
    }

    /// Move (or insert) a library to the front of the LRU list.
    pub fn touch_recent(&mut self, name: &str, path: &str) {
        self.recent_libraries.retain(|r| r.path != path);
        self.recent_libraries.insert(
            0,
            RecentLibrary {
                name: name.to_string(),
                path: path.to_string(),
            },
        );
        self.recent_libraries.truncate(MAX_RECENT);
        self.last_opened_path = Some(path.to_string());
    }

    pub fn forget(&mut self, path: &str) {
        self.recent_libraries.retain(|r| r.path != path);
        if self.last_opened_path.as_deref() == Some(path) {
            self.last_opened_path = None;
        }
    }
}
