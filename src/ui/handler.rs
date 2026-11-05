use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};

const DOUBLE_CLICK: usize = 2;

pub(crate) type Handler<T> = Rc<dyn Fn(&T, &mut Window, &mut App)>;

pub(crate) fn handler<T: 'static>(
    callback: impl Fn(&T, &mut Window, &mut App) + 'static,
) -> Handler<T> {
    Rc::new(callback)
}

pub(crate) fn is_double_click(event: &ClickEvent) -> bool {
    event.click_count() == DOUBLE_CLICK
}
