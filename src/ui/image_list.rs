//! The list of images queued for export.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use super::Notifier;

pub struct ImageList {
    pub widget: gtk::ListBox,
    rows: RefCell<Vec<(adw::ActionRow, PathBuf)>>,
    /// Fires when images are added or removed.
    pub changed: Notifier,
}

impl ImageList {
    pub fn new() -> Rc<Self> {
        let widget = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Single).build();
        widget.add_css_class("boxed-list");
        Rc::new(Self { widget, rows: RefCell::default(), changed: Notifier::default() })
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.rows.borrow().iter().map(|(_, path)| path.clone()).collect()
    }

    pub fn selected(&self) -> Option<PathBuf> {
        let selected = self.widget.selected_row()?;
        let rows = self.rows.borrow();
        rows.iter().find(|(row, _)| row.upcast_ref::<gtk::ListBoxRow>() == &selected).map(|(_, path)| path.clone())
    }

    pub fn add_paths(self: &Rc<Self>, paths: Vec<PathBuf>) {
        for path in paths {
            if !self.paths().contains(&path) {
                self.append(path);
            }
        }
        if self.widget.selected_row().is_none() {
            self.widget.select_row(self.widget.row_at_index(0).as_ref());
        }
        self.changed.emit(());
    }

    fn append(self: &Rc<Self>, path: PathBuf) {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&path.file_name().unwrap_or_default().to_string_lossy()))
            .subtitle(glib::markup_escape_text(&path.parent().unwrap_or(&path).to_string_lossy()))
            .title_lines(1)
            .subtitle_lines(1)
            .build();
        let remove = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Remove")
            .build();
        remove.add_css_class("flat");
        row.add_suffix(&remove);
        remove.connect_clicked(glib::clone!(
            #[strong(rename_to = list)]
            self,
            #[weak]
            row,
            move |_| list.remove(&row)
        ));
        self.widget.append(&row);
        self.rows.borrow_mut().push((row, path));
    }

    fn remove(&self, row: &adw::ActionRow) {
        let was_selected = self.widget.selected_row().is_some_and(|r| &r == row.upcast_ref::<gtk::ListBoxRow>());
        let index = row.index();
        self.widget.remove(row);
        self.rows.borrow_mut().retain(|(r, _)| r != row);
        if was_selected {
            let last = self.rows.borrow().len() as i32 - 1;
            self.widget.select_row(self.widget.row_at_index(index.min(last)).as_ref());
        }
        self.changed.emit(());
    }
}
