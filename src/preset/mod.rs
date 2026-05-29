//! Preset discovery and loading.
//!
//! A preset is a small TOML manifest in `assets/presets/` pointing at a WGSL
//! shader (relative to the assets directory). This indirection is deliberate:
//! it is the format a future `.milk` → WGSL converter will emit into, so the
//! runtime never needs to know whether a preset was hand-written or generated.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// The on-disk manifest (`*.toml`).
#[derive(Clone, Debug, Deserialize)]
pub struct PresetSpec {
    /// Human-readable name, used for selection and on-screen logging.
    pub name: String,
    /// Path to the WGSL body, relative to the assets directory.
    pub shader: String,
    /// Optional description.
    #[serde(default)]
    pub description: String,
}

/// A loaded preset: its manifest plus the resolved absolute shader path.
#[derive(Clone, Debug)]
pub struct Preset {
    pub spec: PresetSpec,
    pub shader_path: PathBuf,
}

impl Preset {
    /// Read the preset's WGSL body from disk.
    pub fn load_body(&self) -> Result<String> {
        std::fs::read_to_string(&self.shader_path)
            .with_context(|| format!("reading shader {}", self.shader_path.display()))
    }
}

/// Discover every `*.toml` preset under `<assets_dir>/presets`, sorted by name
/// for a stable cycling order.
pub fn load_all(assets_dir: &Path) -> Result<Vec<Preset>> {
    let dir = assets_dir.join("presets");
    let mut presets = Vec::new();

    let entries = std::fs::read_dir(&dir)
        .with_context(|| format!("reading preset directory {}", dir.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading preset {}", path.display()))?;
        let spec: PresetSpec =
            toml::from_str(&text).with_context(|| format!("parsing preset {}", path.display()))?;
        let shader_path = assets_dir.join(&spec.shader);
        presets.push(Preset { spec, shader_path });
    }

    if presets.is_empty() {
        anyhow::bail!("no presets found in {}", dir.display());
    }
    presets.sort_by(|a, b| a.spec.name.cmp(&b.spec.name));
    Ok(presets)
}
