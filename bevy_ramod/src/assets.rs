//! Bank asset loader — editor `bank.json` / `*.ramodbank.json`.

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use thiserror::Error;

pub type RamodBankHandle = Handle<RamodBankAsset>;

/// Loaded bank manifest (S5.8 editor Build / S8.3 loader).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct RamodBankAsset {
    pub name: String,
    pub project: Option<String>,
    pub events: Vec<BankEventAsset>,
    /// Directory containing bank.json (filled by load_bank_dir).
    pub base_dir: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankEventAsset {
    pub path: String,
    /// Logical asset name from the editor.
    #[serde(default)]
    pub asset: Option<String>,
    /// Relative path under the bank folder, e.g. `assets/footstep.wav`.
    #[serde(default)]
    pub asset_file: Option<String>,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub is_3d: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_min")]
    pub min_distance: f32,
    #[serde(default = "default_max")]
    pub max_distance: f32,
    #[serde(default)]
    pub parameters: Vec<(String, f32)>,
}

fn default_priority() -> i32 {
    128
}
fn default_min() -> f32 {
    1.0
}
fn default_max() -> f32 {
    50.0
}

#[derive(Deserialize)]
struct BankFile {
    name: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    events: Vec<BankEventAsset>,
}

#[derive(Default)]
pub struct RamodBankLoader;

#[derive(Debug, Error)]
pub enum BankLoaderError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl AssetLoader for RamodBankLoader {
    type Asset = RamodBankAsset;
    type Settings = ();
    type Error = BankLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let file: BankFile = serde_json::from_slice(&bytes)?;

        // Resolve base dir when the asset path is on disk.
        let base_dir = load_context
            .path()
            .parent()
            .map(|p| p.to_path_buf());

        Ok(RamodBankAsset {
            name: file.name,
            project: file.project,
            events: file.events,
            base_dir,
        })
    }

    fn extensions(&self) -> &[&'static str] {
        // Prefer dedicated extension; also accept plain bank.json via load_bank_dir.
        &["ramodbank.json"]
    }
}

impl RamodBankAsset {
    /// Parse bank.json from bytes (shared by loader + filesystem API).
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, BankLoaderError> {
        let file: BankFile = serde_json::from_slice(bytes)?;
        Ok(Self {
            name: file.name,
            project: file.project,
            events: file.events,
            base_dir: None,
        })
    }

    pub fn from_json_path(path: &std::path::Path) -> Result<Self, BankLoaderError> {
        let bytes = std::fs::read(path)?;
        let mut bank = Self::from_json_bytes(&bytes)?;
        bank.base_dir = path.parent().map(|p| p.to_path_buf());
        Ok(bank)
    }
}
