//! The one image operation: fit an image onto a padded canvas.

use std::path::Path;

use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage, Rgba, RgbaImage};

use super::Result;
use super::settings::Settings;

/// Decode an image and rotate it upright according to its EXIF orientation.
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let mut decoder = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Upright copy no larger than `max_side`, so previews stay fast.
pub fn load_preview_source(path: &Path, max_side: u32) -> Result<DynamicImage> {
    let img = load_image(path)?;
    if img.width().max(img.height()) <= max_side {
        return Ok(img);
    }
    Ok(img.thumbnail(max_side, max_side))
}

fn parse_color(hex: &str) -> [u8; 3] {
    let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0xff_ff_ff);
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}

/// Scale `img` to fit inside the padding and centre it on the canvas.
pub fn fit_on_canvas(img: &DynamicImage, settings: &Settings) -> RgbImage {
    let usable_w = settings.width.saturating_sub(2 * settings.padding).max(1);
    let usable_h = settings.height.saturating_sub(2 * settings.padding).max(1);
    let scale = (usable_w as f64 / img.width() as f64).min(usable_h as f64 / img.height() as f64);
    let width = ((img.width() as f64 * scale).round() as u32).max(1);
    let height = ((img.height() as f64 * scale).round() as u32).max(1);
    let scaled = imageops::resize(&img.to_rgba8(), width, height, FilterType::Lanczos3);

    let [r, g, b] = parse_color(&settings.background);
    let mut canvas = RgbaImage::from_pixel(settings.width, settings.height, Rgba([r, g, b, 255]));
    let x = (settings.width.saturating_sub(width) / 2) as i64;
    let y = (settings.height.saturating_sub(height) / 2) as i64;
    imageops::overlay(&mut canvas, &scaled, x, y);
    DynamicImage::ImageRgba8(canvas).to_rgb8()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    const RED: Rgb<u8> = Rgb([255, 0, 0]);
    const BLUE: Rgb<u8> = Rgb([0, 0, 255]);

    fn solid(width: u32, height: u32, color: Rgb<u8>) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, color))
    }

    #[test]
    fn canvas_has_target_size_and_centred_image() {
        let settings = Settings { background: "#0000ff".into(), ..Settings::default() };
        let out = fit_on_canvas(&solid(400, 200, RED), &settings);
        assert_eq!(out.dimensions(), (1080, 1440));
        assert_eq!(out.get_pixel(540, 720), &RED);
        assert_eq!(out.get_pixel(5, 5), &BLUE);
        assert_eq!(out.get_pixel(30, 720), &RED);
        assert_eq!(out.get_pixel(29, 720), &BLUE);
    }

    #[test]
    fn transparent_pixels_show_background() {
        let settings =
            Settings { width: 100, height: 100, padding: 0, background: "#00ff00".into(), ..Settings::default() };
        let clear = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 0])));
        assert_eq!(fit_on_canvas(&clear, &settings).get_pixel(50, 50), &Rgb([0, 255, 0]));
    }

    #[test]
    fn huge_padding_does_not_crash() {
        let settings = Settings { width: 100, height: 100, padding: 500, ..Settings::default() };
        assert_eq!(fit_on_canvas(&solid(10, 10, RED), &settings).dimensions(), (100, 100));
    }

    #[test]
    fn bad_color_falls_back_to_white() {
        assert_eq!(parse_color("nonsense"), [255, 255, 255]);
        assert_eq!(parse_color("#102030"), [0x10, 0x20, 0x30]);
    }
}
