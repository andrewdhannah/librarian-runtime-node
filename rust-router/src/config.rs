//! Config loader for `model-profiles.json`.
//!
//! Reads and validates the model profile configuration file.
//! The router reads this at startup only — changes require a restart.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// A single model profile from the configuration file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Profile {
    pub alias: String,
    pub model_file: String,
    pub model_path: String,
    #[serde(default)]
    pub gguf_size_gb: f64,
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_ngl")]
    pub ngl: u32,
    #[serde(default = "default_context")]
    pub context: u32,
    pub port: u16,
    pub launch_command: Option<String>,
    #[serde(default)]
    pub task_classes: Vec<String>,
    #[serde(default = "default_verified_status")]
    pub verified_status: String,
    pub evidence_path: Option<String>,
    #[serde(default = "default_authority_status")]
    pub authority_status: String,
    #[serde(default)]
    pub limitations: String,
    #[serde(default)]
    pub known_behavior: String,
    #[serde(default)]
    pub test_cells: Vec<String>,
}

fn default_backend() -> String { "vulkan".to_string() }
fn default_ngl() -> u32 { 99 }
fn default_context() -> u32 { 1024 }
fn default_verified_status() -> String { "unverified".to_string() }
fn default_authority_status() -> String { "advisory_only".to_string() }

/// Top-level config file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub _meta: serde_json::Value,
    #[serde(default)]
    pub defaults: serde_json::Value,
    pub profiles: Vec<Profile>,
}

/// In-memory profile store, indexed by alias.
/// The router creates this once at startup.
#[derive(Debug, Clone)]
pub struct ProfileManager {
    profiles: HashMap<String, Profile>,
    profile_list: Vec<Profile>,
}

impl ProfileManager {
    /// Load profiles from `model-profiles.json` at the given path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config '{}': {}", path.display(), e))?;
        let config: RuntimeConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config '{}': {}", path.display(), e))?;

        let mut profiles = HashMap::new();
        for p in &config.profiles {
            profiles.insert(p.alias.clone(), p.clone());
        }

        Ok(ProfileManager {
            profiles,
            profile_list: config.profiles,
        })
    }

    /// Try profile sources in order. First match wins.
    pub fn load_from_sources(sources: &[&Path]) -> Result<Self, String> {
        let mut last_err = String::new();
        for source in sources {
            if source.exists() {
                match Self::load(source) {
                    Ok(pm) => return Ok(pm),
                    Err(e) => last_err = format!("{}: {}", source.display(), e),
                }
            }
        }
        Err(format!(
            "No profile source found. Tried {} sources. Last error: {}",
            sources.len(),
            last_err
        ))
    }

    pub fn get(&self, alias: &str) -> Option<&Profile> {
        self.profiles.get(alias)
    }

    pub fn list_all(&self) -> Vec<serde_json::Value> {
        self.profile_list
            .iter()
            .map(|p| {
                serde_json::json!({
                    "alias": p.alias,
                    "task_classes": p.task_classes,
                    "verified": p.verified_status == "verified",
                    "port": p.port,
                    "model_file": p.model_file,
                })
            })
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.profile_list.iter()
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    pub fn aliases(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
}
