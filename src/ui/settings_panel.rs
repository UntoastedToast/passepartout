//! Sidebar with every customization, bound to a shared `Settings`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use super::Notifier;
use crate::engine::presets::{PRESETS, find_preset};
use crate::engine::settings::{OutputFormat, Settings};

const MAX_SIZE: f64 = 8000.0;

pub struct SettingsPanel {
    pub page: adw::PreferencesPage,
    /// Fires after any edit made through the UI.
    pub changed: Notifier,
    settings: Rc<RefCell<Settings>>,
    syncing: Cell<bool>,
    preset: adw::ComboRow,
    width: adw::SpinRow,
    height: adw::SpinRow,
    padding: adw::SpinRow,
    background: gtk::ColorDialogButton,
    format: adw::ComboRow,
    quality: adw::SpinRow,
    folder: adw::ActionRow,
}

fn spin(title: &str, min: f64, max: f64) -> adw::SpinRow {
    let row = adw::SpinRow::with_range(min, max, 1.0);
    row.set_title(title);
    row
}

fn combo(title: &str, labels: &[String], use_subtitle: bool) -> adw::ComboRow {
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    // `use_subtitle` must come before the model, or the first selection never shows.
    adw::ComboRow::builder().title(title).use_subtitle(use_subtitle).model(&gtk::StringList::new(&labels)).build()
}

fn add_group(page: &adw::PreferencesPage, title: &str, rows: &[&gtk::Widget]) {
    let group = adw::PreferencesGroup::builder().title(title).build();
    for row in rows {
        group.add(*row);
    }
    page.add(&group);
}

fn to_hex(color: &gdk::RGBA) -> String {
    let channel = |c: f32| (c * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", channel(color.red()), channel(color.green()), channel(color.blue()))
}

impl SettingsPanel {
    pub fn new(settings: Rc<RefCell<Settings>>) -> Rc<Self> {
        let page = adw::PreferencesPage::new();

        let mut preset_labels: Vec<String> = PRESETS.iter().map(|p| p.label()).collect();
        preset_labels.push("Custom".into());
        let preset = combo("Preset", &preset_labels, true);
        let width = spin("Width", 16.0, MAX_SIZE);
        let height = spin("Height", 16.0, MAX_SIZE);
        add_group(&page, "Canvas", &[preset.upcast_ref(), width.upcast_ref(), height.upcast_ref()]);

        let padding = spin("Padding", 0.0, MAX_SIZE / 2.0);
        let background = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::builder().with_alpha(false).build()));
        background.set_valign(gtk::Align::Center);
        let background_row = adw::ActionRow::builder().title("Background").activatable_widget(&background).build();
        background_row.add_suffix(&background);
        add_group(&page, "Frame", &[padding.upcast_ref(), background_row.upcast_ref()]);

        let format_labels: Vec<String> = OutputFormat::ALL.iter().map(|f| f.label().into()).collect();
        let format = combo("Format", &format_labels, false);
        let quality = spin("Quality", 1.0, 100.0);
        let folder = adw::ActionRow::builder().title("Folder").use_markup(false).build();
        let choose = gtk::Button::builder()
            .icon_name("folder-open-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Choose folder")
            .build();
        choose.add_css_class("flat");
        folder.add_suffix(&choose);
        add_group(&page, "Output", &[format.upcast_ref(), quality.upcast_ref(), folder.upcast_ref()]);

        let panel = Rc::new(Self {
            page,
            changed: Notifier::default(),
            settings,
            syncing: Cell::new(false),
            preset,
            width,
            height,
            padding,
            background,
            format,
            quality,
            folder,
        });
        panel.load();
        panel.connect_signals(&choose);
        panel
    }

    fn connect_signals(self: &Rc<Self>, choose: &gtk::Button) {
        self.preset.connect_selected_notify(glib::clone!(
            #[strong(rename_to = p)]
            self,
            move |_| p.on_preset()
        ));
        for row in [&self.width, &self.height] {
            row.connect_value_notify(glib::clone!(
                #[strong(rename_to = p)]
                self,
                move |_| p.on_size()
            ));
        }
        for row in [&self.padding, &self.quality] {
            row.connect_value_notify(glib::clone!(
                #[strong(rename_to = p)]
                self,
                move |_| p.on_values()
            ));
        }
        self.format.connect_selected_notify(glib::clone!(
            #[strong(rename_to = p)]
            self,
            move |_| p.on_values()
        ));
        self.background.connect_rgba_notify(glib::clone!(
            #[strong(rename_to = p)]
            self,
            move |_| p.on_values()
        ));
        choose.connect_clicked(glib::clone!(
            #[strong(rename_to = p)]
            self,
            move |_| p.choose_folder()
        ));
    }

    fn load(&self) {
        self.syncing.set(true);
        {
            let s = self.settings.borrow();
            self.width.set_value(s.width.into());
            self.height.set_value(s.height.into());
            self.padding.set_value(s.padding.into());
            self.quality.set_value(s.quality.into());
            self.format.set_selected(OutputFormat::ALL.iter().position(|f| *f == s.output_format).unwrap_or(0) as u32);
            self.folder.set_subtitle(&s.output_dir.to_string_lossy());
            self.quality.set_visible(s.output_format.has_quality());
            if let Ok(color) = gdk::RGBA::parse(s.background.as_str()) {
                self.background.set_rgba(&color);
            }
            let preset = find_preset(s.width, s.height).unwrap_or(PRESETS.len());
            self.preset.set_selected(preset as u32);
        }
        self.syncing.set(false);
    }

    fn on_preset(&self) {
        let index = self.preset.selected() as usize;
        if self.syncing.get() || index >= PRESETS.len() {
            return;
        }
        {
            let mut s = self.settings.borrow_mut();
            (s.width, s.height) = (PRESETS[index].width, PRESETS[index].height);
        }
        self.load();
        self.changed.emit(());
    }

    fn on_size(&self) {
        if self.syncing.get() {
            return;
        }
        {
            let mut s = self.settings.borrow_mut();
            (s.width, s.height) = (self.width.value() as u32, self.height.value() as u32);
        }
        self.load();
        self.changed.emit(());
    }

    fn on_values(&self) {
        if self.syncing.get() {
            return;
        }
        {
            let mut s = self.settings.borrow_mut();
            s.padding = self.padding.value() as u32;
            s.quality = self.quality.value() as u8;
            s.output_format = OutputFormat::ALL[self.format.selected() as usize];
            s.background = to_hex(&self.background.rgba());
        }
        self.load();
        self.changed.emit(());
    }

    fn choose_folder(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Output folder").build();
        dialog.set_initial_folder(Some(&gio::File::for_path(&self.settings.borrow().output_dir)));
        let window = self.page.root().and_downcast::<gtk::Window>();
        let panel = self.clone();
        dialog.select_folder(window.as_ref(), gio::Cancellable::NONE, move |result| {
            if let Some(path) = result.ok().and_then(|folder| folder.path()) {
                panel.settings.borrow_mut().output_dir = path;
                panel.load();
                panel.changed.emit(());
            }
        });
    }
}
