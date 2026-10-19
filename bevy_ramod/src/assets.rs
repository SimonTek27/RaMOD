//! Bank asset loader (S8.3) — loads `*.ramodbank.json` manifests.

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use thiserror::Error;

/// Handle wrapper for ergonomics.
pub type RamodBankHandle = Handle<RamodBankAsset>;

/// Loaded bank manifest (from S5 export / editor Build).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct RamodBankAsset {
    pub name: String,
    pub events: Vec<BankEventAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankEventAsset {
    pub path: String,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub is_3d: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    128
}

#[derive(Deserialize)]
struct BankFile {
    name: String,
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
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let file: BankFile = serde_json::from_slice(&bytes)?;
        Ok(RamodBankAsset {
            name: file.name,
            events: file.events,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ramodbank.json", "json"]
    }
}
