//! Live preview of the selected image on the canvas.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::thread;

use gtk::prelude::*;
use gtk::{gdk, glib};
use image::{DynamicImage, RgbImage};

use super::Notifier;
use crate::engine::render::{fit_on_canvas, load_preview_source};
use crate::engine::settings::Settings;

const PREVIEW_SOURCE_MAX_SIDE: u32 = 2000;
const PREVIEW_CANVAS_MAX_SIDE: u32 = 900;

pub struct Preview {
    pub widget: gtk::Picture,
    settings: Rc<RefCell<Settings>>,
    current: RefCell<Option<PathBuf>>,
    source: RefCell<Option<DynamicImage>>,
    /// Fires with a message when an image can't be read.
    pub failed: Notifier<String>,
}

impl Preview {
    pub fn new(settings: Rc<RefCell<Settings>>) -> Rc<Self> {
        let widget = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Contain)
            .can_shrink(true)
            .vexpand(true)
            .margin_top(24)
            .margin_bottom(12)
            .margin_start(24)
            .margin_end(24)
            .build();
        widget.add_css_class("card");
        Rc::new(Self {
            widget,
            settings,
            current: RefCell::default(),
            source: RefCell::default(),
            failed: Notifier::default(),
        })
    }

    /// Show another image (or nothing). The decoding happens off the UI thread.
    pub fn show(self: &Rc<Self>, path: Option<PathBuf>) {
        if *self.current.borrow() == path {
            return;
        }
        *self.source.borrow_mut() = None;
        *self.current.borrow_mut() = path.clone();
        let Some(path) = path else {
            self.widget.set_paintable(gdk::Paintable::NONE);
            return;
        };

        let (sender, receiver) = async_channel::bounded(1);
        let worker_path = path.clone();
        thread::spawn(move || {
            let loaded = load_preview_source(&worker_path, PREVIEW_SOURCE_MAX_SIDE).map_err(|e| e.to_string());
            let _ = sender.send_blocking(loaded);
        });
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(loaded) = receiver.recv().await else { return };
            if this.current.borrow().as_ref() != Some(&path) {
                return;
            }
            match loaded {
                Ok(image) => {
                    *this.source.borrow_mut() = Some(image);
                    this.refresh();
                }
                Err(message) => this.failed.emit(format!("Cannot read {}: {message}", path.display())),
            }
        });
    }

    /// Re-render the current image with the current settings.
    pub fn refresh(&self) {
        if let Some(source) = &*self.source.borrow() {
            let settings = preview_settings(&self.settings.borrow());
            self.widget.set_paintable(Some(&texture(&fit_on_canvas(source, &settings))));
        }
    }
}

fn preview_settings(settings: &Settings) -> Settings {
    let factor = (PREVIEW_CANVAS_MAX_SIDE as f64 / settings.width.max(settings.height) as f64).min(1.0);
    let scale = |value: u32| ((value as f64 * factor).round() as u32).max(1);
    Settings {
        width: scale(settings.width),
        height: scale(settings.height),
        padding: (settings.padding as f64 * factor).round() as u32,
        ..settings.clone()
    }
}

fn texture(canvas: &RgbImage) -> gdk::MemoryTexture {
    gdk::MemoryTexture::new(
        canvas.width() as i32,
        canvas.height() as i32,
        gdk::MemoryFormat::R8g8b8,
        &glib::Bytes::from(canvas.as_raw().as_slice()),
        canvas.width() as usize * 3,
    )
}
