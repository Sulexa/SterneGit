use gpui_kit::{
    App, Context, DispatchPhase, HitboxBehavior, IntoElement, ScrollEvent, Styled as _, WeakEntity,
    Window, canvas,
};

pub(crate) fn ctrl_wheel_area<V: 'static>(
    view: WeakEntity<V>,
    take: fn(&mut V, &ScrollEvent, &mut Context<V>) -> bool,
) -> impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |_, hitbox, window, _| {
            window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                let taken = phase == DispatchPhase::Capture
                    && event.modifiers.secondary(cx)
                    && hitbox.should_handle_scroll(window)
                    && view.update(cx, |view, cx| take(view, event, cx)).unwrap_or(false);
                if taken {
                    cx.stop_propagation();
                }
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

pub(crate) fn ctrl_wheel_elsewhere(
    take: fn(&ScrollEvent, &mut Window, &mut App),
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                if phase == DispatchPhase::Capture && event.modifiers.secondary(cx) {
                    cx.stop_propagation();
                    take(event, window, cx);
                }
            });
        },
    )
    .absolute()
    .size_0()
}
