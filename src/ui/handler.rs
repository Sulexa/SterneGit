use std::rc::Rc;

use gpui_kit::{App, Window};

pub(crate) type Handler<T> = Rc<dyn Fn(&T, &mut Window, &mut App)>;

pub(crate) fn handler<T: 'static>(
    callback: impl Fn(&T, &mut Window, &mut App) + 'static,
) -> Handler<T> {
    Rc::new(callback)
}
