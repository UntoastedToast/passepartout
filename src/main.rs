mod engine;
mod ui;

use std::cell::OnceCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio};

use engine::settings;
use ui::window::Window;

const APP_ID: &str = "io.github.untoastedtoast.Passepartout";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_OPEN).build();
    let window: Rc<OnceCell<Rc<Window>>> = Rc::default();

    app.connect_startup(|_| {
        if cfg!(debug_assertions)
            && let Some(display) = gdk::Display::default()
        {
            gtk::IconTheme::for_display(&display).add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons"));
        }
        gtk::Window::set_default_icon_name(APP_ID);
    });

    let present = {
        let window = window.clone();
        move |app: &adw::Application| {
            let window = window.get_or_init(|| Window::new(app, settings::load(&settings::config_path()))).clone();
            window.window.present();
            window
        }
    };
    app.connect_activate({
        let present = present.clone();
        move |app| {
            present(app);
        }
    });
    app.connect_open(move |app, files, _hint| {
        let paths: Vec<PathBuf> = files.iter().filter_map(|f| f.path()).collect();
        present(app).add_paths(&paths);
    });

    app.run()
}
