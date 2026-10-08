//! Collect images and export them with the current settings.

use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use walkdir::WalkDir;

use super::Result;
use super::render::{fit_on_canvas, load_image};
use super::settings::Settings;

pub const IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "bmp", "gif", "tiff", "webp"];

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| extensions.contains(&e.to_lowercase().as_str()))
}

/// Expand files and folders (recursively) into a sorted, de-duplicated list of images.
pub fn collect_images(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .flat_map(|path| WalkDir::new(path).into_iter().flatten())
        .map(|entry| entry.into_path())
        .filter(|path| path.is_file() && has_extension(path, &IMAGE_EXTENSIONS))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn suffixed_path(source: &Path, settings: &Settings, suffix: &str) -> PathBuf {
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let extension = settings
        .output_format
        .extension()
        .map(String::from)
        .or_else(|| source.extension().map(|e| e.to_string_lossy().into_owned()))
        .unwrap_or_default();
    settings.output_dir.join(format!("{stem}_canvas{suffix}.{extension}"))
}

pub fn output_path(source: &Path, settings: &Settings) -> PathBuf {
    suffixed_path(source, settings, "")
}

pub fn export_image(source: &Path, target: &Path, settings: &Settings) -> Result<()> {
    let canvas = fit_on_canvas(&load_image(source)?, settings);
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    if has_extension(target, &["jpg", "jpeg"]) {
        let writer = BufWriter::new(File::create(target)?);
        canvas.write_with_encoder(JpegEncoder::new_with_quality(writer, settings.quality))?;
    } else {
        canvas.save(target)?;
    }
    Ok(())
}

/// Export all sources; names that would collide get a numeric suffix.
pub fn export(
    sources: &[PathBuf],
    settings: &Settings,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    let mut used = HashSet::new();
    for (i, source) in sources.iter().enumerate() {
        let mut target = output_path(source, settings);
        for n in 2.. {
            if used.insert(target.clone()) {
                break;
            }
            target = suffixed_path(source, settings, &format!("-{n}"));
        }
        export_image(source, &target, settings)?;
        written.push(target);
        on_progress(i + 1, sources.len());
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::settings::OutputFormat;
    use image::{Rgb, RgbImage};

    fn make_image(path: &Path) -> PathBuf {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbImage::from_pixel(400, 200, Rgb([255, 0, 0])).save(path).unwrap();
        path.to_path_buf()
    }

    #[test]
    fn collect_is_recursive_filtered_and_deduplicated() {
        let dir = tempfile::tempdir().unwrap();
        let a = make_image(&dir.path().join("a.png"));
        let b = make_image(&dir.path().join("sub").join("b.png"));
        fs::write(dir.path().join("notes.txt"), "x").unwrap();
        let expected = vec![a.clone(), b];
        assert_eq!(collect_images(&[dir.path().to_path_buf()]), expected);
        assert_eq!(collect_images(&[a, dir.path().to_path_buf()]), expected);
    }

    #[test]
    fn export_names_format_and_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let a = make_image(&dir.path().join("in/x/photo.png"));
        let b = make_image(&dir.path().join("in/y/photo.png"));
        let settings =
            Settings { output_dir: dir.path().join("out"), output_format: OutputFormat::Jpeg, ..Settings::default() };
        let mut progress = Vec::new();
        let written = export(&[a.clone(), b], &settings, |done, total| progress.push((done, total))).unwrap();
        let names: Vec<_> = written.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, ["photo_canvas.jpg", "photo_canvas-2.jpg"]);
        assert_eq!(progress, [(1, 2), (2, 2)]);
        assert_eq!(image::open(&written[0]).unwrap().to_rgb8().dimensions(), (1080, 1440));

        let original = Settings { output_dir: "o".into(), ..Settings::default() };
        assert_eq!(output_path(&a, &original).file_name().unwrap(), "photo_canvas.png");
    }

    #[test]
    fn every_supported_output_extension_can_be_written() {
        let dir = tempfile::tempdir().unwrap();
        let source = make_image(&dir.path().join("p.png"));
        for ext in IMAGE_EXTENSIONS {
            let target = dir.path().join(format!("out.{ext}"));
            export_image(&source, &target, &Settings::default()).unwrap_or_else(|e| panic!("{ext}: {e}"));
            assert!(target.exists());
        }
    }
}
