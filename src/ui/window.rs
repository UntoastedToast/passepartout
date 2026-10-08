//! Main window: wires the settings panel, image list, preview and export together.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use super::image_list::ImageList;
use super::preview::Preview;
use super::settings_panel::SettingsPanel;
use crate::engine::batch::{IMAGE_EXTENSIONS, collect_images, export};
use crate::engine::settings::{self, Settings};

const REFRESH_DELAY: Duration = Duration::from_millis(150);

pub struct Window {
    pub window: adw::ApplicationWindow,
    settings: Rc<RefCell<Settings>>,
    images: Rc<ImageList>,
    preview: Rc<Preview>,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    progress: gtk::ProgressBar,
    export_action: gio::SimpleAction,
    exporting: Cell<bool>,
    refresh_timer: RefCell<Option<glib::SourceId>>,
}

enum ExportMessage {
    Progress(usize, usize),
    Done(Result<usize, String>),
}

impl Window {
    pub fn new(app: &adw::Application, settings: Settings) -> Rc<Self> {
        let settings = Rc::new(RefCell::new(settings));
        let panel = SettingsPanel::new(settings.clone());
        let split = adw::OverlaySplitView::builder().min_sidebar_width(320.0).max_sidebar_width(380.0).build();

        let this = Rc::new(Self {
            window: adw::ApplicationWindow::builder()
                .application(app)
                .title("Passepartout")
                .default_width(1100)
                .default_height(760)
                .build(),
            settings: settings.clone(),
            images: ImageList::new(),
            preview: Preview::new(settings),
            toasts: adw::ToastOverlay::new(),
            stack: gtk::Stack::new(),
            progress: gtk::ProgressBar::builder().visible(false).build(),
            export_action: gio::SimpleAction::new("export", None),
            exporting: Cell::new(false),
            refresh_timer: RefCell::default(),
        });

        this.build_layout(&split, &panel);
        this.add_actions(app);
        this.add_breakpoint(&split);
        this.connect_signals(&panel);
        this.update_state();
        this
    }

    pub fn add_paths(&self, paths: &[PathBuf]) {
        let images = collect_images(paths);
        if images.is_empty() {
            self.toast("No supported images found");
        }
        self.images.add_paths(images);
    }

    fn build_layout(&self, split: &adw::OverlaySplitView, panel: &SettingsPanel) {
        let sidebar = adw::ToolbarView::new();
        sidebar
            .add_top_bar(&adw::HeaderBar::builder().title_widget(&adw::WindowTitle::new("Passepartout", "")).build());
        sidebar.set_content(Some(&panel.page));
        split.set_sidebar(Some(&sidebar));

        self.stack.add_named(&self.empty_page(), Some("empty"));
        self.stack.add_named(&self.images_page(), Some("images"));

        let content = adw::ToolbarView::new();
        content.add_top_bar(&self.content_header(split));
        content.set_content(Some(&self.stack));
        content.add_bottom_bar(&self.progress);
        split.set_content(Some(&content));

        self.toasts.set_child(Some(split));
        self.window.set_content(Some(&self.toasts));
    }

    fn content_header(&self, split: &adw::OverlaySplitView) -> adw::HeaderBar {
        let header = adw::HeaderBar::new();
        let toggle =
            gtk::ToggleButton::builder().icon_name("sidebar-show-symbolic").tooltip_text("Show settings").build();
        split.bind_property("show-sidebar", &toggle, "active").bidirectional().sync_create().build();
        header.pack_start(&toggle);

        let more = gio::Menu::new();
        more.append(Some("Add Folder…"), Some("win.add-folder"));
        let add =
            adw::SplitButton::builder().label("Add Images").menu_model(&more).action_name("win.add-images").build();
        header.pack_start(&add);

        let export = gtk::Button::builder().label("Export").action_name("win.export").build();
        export.add_css_class("suggested-action");
        header.pack_end(&export);
        header
    }

    fn empty_page(&self) -> adw::StatusPage {
        let page = adw::StatusPage::builder()
            .icon_name("image-x-generic-symbolic")
            .title("Drop Images Here")
            .description("Add images or whole folders to place them on a canvas.")
            .build();
        let button =
            gtk::Button::builder().label("Add Images").action_name("win.add-images").halign(gtk::Align::Center).build();
        button.add_css_class("suggested-action");
        button.add_css_class("pill");
        page.set_child(Some(&button));
        page
    }

    fn images_page(&self) -> gtk::Box {
        let scroller = gtk::ScrolledWindow::builder()
            .child(&self.images.widget)
            .propagate_natural_height(true)
            .max_content_height(220)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .margin_start(24)
            .margin_end(24)
            .margin_bottom(24)
            .build();
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        page.append(&self.preview.widget);
        page.append(&scroller);
        page
    }

    fn add_actions(self: &Rc<Self>, app: &adw::Application) {
        let add_images = gio::SimpleAction::new("add-images", None);
        add_images.connect_activate(glib::clone!(
            #[strong(rename_to = w)]
            self,
            move |_, _| w.choose_images()
        ));
        let add_folder = gio::SimpleAction::new("add-folder", None);
        add_folder.connect_activate(glib::clone!(
            #[strong(rename_to = w)]
            self,
            move |_, _| w.choose_folder()
        ));
        self.export_action.connect_activate(glib::clone!(
            #[strong(rename_to = w)]
            self,
            move |_, _| w.start_export()
        ));

        for (action, accel) in
            [(&add_images, "<Control>o"), (&add_folder, "<Control><Shift>o"), (&self.export_action, "<Control>e")]
        {
            self.window.add_action(action);
            app.set_accels_for_action(&format!("win.{}", action.name()), &[accel]);
        }
    }

    fn add_breakpoint(&self, split: &adw::OverlaySplitView) {
        let breakpoint = adw::Breakpoint::new(
            adw::BreakpointCondition::parse("max-width: 760sp").expect("valid breakpoint condition"),
        );
        breakpoint.add_setter(split, "collapsed", Some(&true.to_value()));
        self.window.add_breakpoint(breakpoint);
    }

    fn connect_signals(self: &Rc<Self>, panel: &SettingsPanel) {
        panel.changed.set(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.schedule_refresh()
        ));
        self.images.changed.set(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.update_state()
        ));
        self.images.widget.connect_row_selected(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, _| {
                w.preview.show(w.images.selected());
            }
        ));
        self.preview.failed.set(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |message: String| w.toast(&message)
        ));
        self.window.connect_close_request(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                w.save_settings();
                glib::Propagation::Proceed
            }
        ));

        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        drop.connect_drop(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            false,
            move |_, value, _, _| {
                let Ok(list) = value.get::<gdk::FileList>() else { return false };
                w.add_files(list.files());
                true
            }
        ));
        self.stack.add_controller(drop);
    }

    fn update_state(&self) {
        let has_images = !self.images.paths().is_empty();
        self.stack.set_visible_child_name(if has_images { "images" } else { "empty" });
        self.export_action.set_enabled(has_images && !self.exporting.get());
    }

    fn schedule_refresh(self: &Rc<Self>) {
        if let Some(timer) = self.refresh_timer.borrow_mut().take() {
            timer.remove();
        }
        let this = self.clone();
        let timer = glib::timeout_add_local_once(REFRESH_DELAY, move || {
            this.refresh_timer.borrow_mut().take();
            this.save_settings();
            this.preview.refresh();
        });
        *self.refresh_timer.borrow_mut() = Some(timer);
    }

    fn save_settings(&self) {
        if let Err(error) = settings::save(&self.settings.borrow(), &settings::config_path()) {
            self.toast(&format!("Could not save settings: {error}"));
        }
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    fn add_files(&self, files: Vec<gio::File>) {
        let paths: Vec<PathBuf> = files.iter().filter_map(|f| f.path()).collect();
        self.add_paths(&paths);
    }

    fn choose_images(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Images"));
        for extension in IMAGE_EXTENSIONS {
            filter.add_suffix(extension);
        }
        let dialog = gtk::FileDialog::builder().title("Add images").default_filter(&filter).build();
        let this = self.clone();
        dialog.open_multiple(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if let Ok(list) = result {
                this.add_files(list.iter::<gio::File>().flatten().collect());
            }
        });
    }

    fn choose_folder(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Add folder").build();
        let this = self.clone();
        dialog.select_folder(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if let Ok(folder) = result {
                this.add_files(vec![folder]);
            }
        });
    }

    fn start_export(self: &Rc<Self>) {
        self.exporting.set(true);
        self.update_state();
        self.progress.set_fraction(0.0);
        self.progress.set_visible(true);

        let settings = self.settings.borrow().clone();
        let sources = self.images.paths();
        let (sender, receiver) = async_channel::unbounded();
        let worker_settings = settings.clone();
        thread::spawn(move || {
            let progress = sender.clone();
            let result = export(&sources, &worker_settings, |done, total| {
                let _ = progress.send_blocking(ExportMessage::Progress(done, total));
            });
            let _ =
                sender.send_blocking(ExportMessage::Done(result.map(|files| files.len()).map_err(|e| e.to_string())));
        });

        let this = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(message) = receiver.recv().await {
                match message {
                    ExportMessage::Progress(done, total) => this.progress.set_fraction(done as f64 / total as f64),
                    ExportMessage::Done(result) => this.finish_export(result, &settings),
                }
            }
        });
    }

    fn finish_export(self: &Rc<Self>, result: Result<usize, String>, settings: &Settings) {
        self.exporting.set(false);
        self.progress.set_visible(false);
        self.update_state();
        match result {
            Ok(count) => {
                let toast = adw::Toast::builder()
                    .title(format!("Exported {count} image(s)"))
                    .button_label("Open Folder")
                    .build();
                let folder = gio::File::for_path(&settings.output_dir);
                let window = self.window.clone();
                toast.connect_button_clicked(move |_| {
                    gtk::FileLauncher::new(Some(&folder)).launch(Some(&window), gio::Cancellable::NONE, |_| {});
                });
                self.toasts.add_toast(toast);
            }
            Err(error) => self.toast(&format!("Export failed: {error}")),
        }
    }
}
