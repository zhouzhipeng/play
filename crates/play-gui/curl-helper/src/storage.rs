use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::executor::ExecutionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
}

impl Group {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurlItem {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub results: Vec<ExecutionResult>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CurlItem {
    pub fn new(name: String, command: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            command,
            selected: false,
            results: Vec::new(),
            group_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            self.name.clone()
        } else {
            let cmd = self.command.replace('\n', " ").replace("\\", "");
            if cmd.len() > 60 {
                format!("{}...", &cmd[..60])
            } else {
                cmd
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppData {
    pub curls: Vec<CurlItem>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            curls: Vec::new(),
            groups: Vec::new(),
        }
    }
}

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("curl-helper");

        Self { path: dir.join("data.json") }
    }

    pub fn load(&self) -> AppData {
        if !self.path.exists() {
            return AppData::default();
        }

        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppData::default(),
        }
    }

    pub fn save(&self, data: &AppData) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(data) {
            let _ = fs::write(&self.path, json);
        }
    }
}
