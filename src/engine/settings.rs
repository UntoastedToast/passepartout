//! User settings and their persistence.

use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::{Deserialize, Serialize};

use super::Result;
use super::presets::DEFAULT_PRESET;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Original,
    Jpeg,
    Png,
    Webp,
}

impl OutputFormat {
    pub const ALL: [OutputFormat; 4] = [Self::Original, Self::Jpeg, Self::Png, Self::Webp];

    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::Webp => "WebP (lossless)",
        }
    }

    /// File extension for this format; `None` keeps the source's extension.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Self::Original => None,
            Self::Jpeg => Some("jpg"),
            Self::Png => Some("png"),
            Self::Webp => Some("webp"),
        }
    }

    /// Only JPEG output has a quality setting.
    pub fn has_quality(self) -> bool {
        matches!(self, Self::Original | Self::Jpeg)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    /// `#rrggbb`
    pub background: String,
    pub output_format: OutputFormat,
    pub quality: u8,
    pub output_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            width: DEFAULT_PRESET.width,
            height: DEFAULT_PRESET.height,
            padding: 30,
            background: "#ffffff".into(),
            output_format: OutputFormat::default(),
            quality: 85,
            output_dir: home().join("Pictures").join("Passepartout"),
        }
    }
}

fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// `$XDG_CONFIG_HOME/passepartout/settings.json` (inside the sandbox when run as Flatpak).
pub fn config_path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"));
    base.join("passepartout").join("settings.json")
}

/// A missing or broken file yields the defaults; unknown fields are ignored.
pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

pub fn save(settings: &Settings, path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_profile_grid() {
        let s = Settings::default();
        assert_eq!((s.width, s.height, s.padding), (1080, 1440, 30));
    }

    #[test]
    fn roundtrip_and_fallbacks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("s.json");
        assert_eq!(load(&path), Settings::default());

        let custom = Settings { height: 1350, padding: 10, output_format: OutputFormat::Png, ..Settings::default() };
        save(&custom, &path).unwrap();
        assert_eq!(load(&path), custom);

        fs::write(&path, "{broken").unwrap();
        assert_eq!(load(&path), Settings::default());

        fs::write(&path, r#"{"width": 500, "unknown": 1}"#).unwrap();
        let partial = load(&path);
        assert_eq!((partial.width, partial.height), (500, 1440));
    }
}
