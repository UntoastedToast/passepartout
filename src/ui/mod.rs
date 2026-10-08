//! GTK widgets only: no image logic lives here.

pub mod image_list;
pub mod preview;
pub mod settings_panel;
pub mod window;

use std::cell::RefCell;

/// A single "something happened" callback, set once after construction.
type Callback<T> = Box<dyn Fn(T)>;

pub struct Notifier<T = ()>(RefCell<Option<Callback<T>>>);

impl<T> Default for Notifier<T> {
    fn default() -> Self {
        Self(RefCell::new(None))
    }
}

impl<T> Notifier<T> {
    pub fn set(&self, callback: impl Fn(T) + 'static) {
        *self.0.borrow_mut() = Some(Box::new(callback));
    }

    pub fn emit(&self, value: T) {
        if let Some(callback) = &*self.0.borrow() {
            callback(value);
        }
    }
}
