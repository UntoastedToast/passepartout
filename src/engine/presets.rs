//! Canvas size presets for current Instagram formats.

pub struct Preset {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Preset {
    pub fn label(&self) -> String {
        format!("{} ({} × {})", self.name, self.width, self.height)
    }
}

pub const PRESETS: [Preset; 5] = [
    Preset { name: "Profile grid 3:4", width: 1080, height: 1440 },
    Preset { name: "Feed portrait 4:5", width: 1080, height: 1350 },
    Preset { name: "Feed square 1:1", width: 1080, height: 1080 },
    Preset { name: "Feed landscape 1.91:1", width: 1080, height: 566 },
    Preset { name: "Story / Reel 9:16", width: 1080, height: 1920 },
];

pub const DEFAULT_PRESET: &Preset = &PRESETS[0];

/// Index of the preset matching the size, or `None` for a custom size.
pub fn find_preset(width: u32, height: u32) -> Option<usize> {
    PRESETS.iter().position(|p| p.width == width && p.height == height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_found_by_its_size() {
        for (i, p) in PRESETS.iter().enumerate() {
            assert_eq!(find_preset(p.width, p.height), Some(i));
        }
        assert_eq!(find_preset(1000, 1000), None);
    }
}
