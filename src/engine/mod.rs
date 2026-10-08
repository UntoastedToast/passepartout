//! Image logic without any GTK: presets, settings, rendering and batch export.

pub mod batch;
pub mod presets;
pub mod render;
pub mod settings;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
