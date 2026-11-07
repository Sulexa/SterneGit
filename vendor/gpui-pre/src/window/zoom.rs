use crate::{
    BackGestureEvent, Bounds, Corners, Edges, FileDropEvent, LongPressEvent, MouseDownEvent,
    MouseExitEvent, MouseMoveEvent, MousePressureEvent, MouseUpEvent, PinchEvent, PlatformInput,
    Pixels, Point, ScrollDelta, ScrollEvent, Size, TouchDragEvent, TouchEvent, WindowInsets,
};

/// A length the window's content zoom scales: in window units on one side,
/// in platform units on the other.
pub(crate) trait Zoomed: Sized {
    /// Converts window units to platform units.
    fn zoomed(self, zoom: f32) -> Self;

    /// Converts platform units to window units.
    fn unzoomed(self, zoom: f32) -> Self {
        self.zoomed(1. / zoom)
    }
}

impl Zoomed for Pixels {
    fn zoomed(self, zoom: f32) -> Self {
        self * zoom
    }
}

impl Zoomed for Point<Pixels> {
    fn zoomed(self, zoom: f32) -> Self {
        self.map(|value| value * zoom)
    }
}

impl Zoomed for Size<Pixels> {
    fn zoomed(self, zoom: f32) -> Self {
        self.map(|value| value * zoom)
    }
}

impl Zoomed for Bounds<Pixels> {
    fn zoomed(self, zoom: f32) -> Self {
        self.map(|value| value * zoom)
    }
}

impl Zoomed for Edges<Pixels> {
    fn zoomed(self, zoom: f32) -> Self {
        self.map(|value| *value * zoom)
    }
}

impl Zoomed for Corners<Pixels> {
    fn zoomed(self, zoom: f32) -> Self {
        self.map(|value| *value * zoom)
    }
}

impl Zoomed for WindowInsets {
    fn zoomed(self, zoom: f32) -> Self {
        WindowInsets {
            safe_area: self.safe_area.zoomed(zoom),
            ime: self.ime.zoomed(zoom),
            corner_radii: self.corner_radii.zoomed(zoom),
        }
    }
}

impl Zoomed for ScrollDelta {
    fn zoomed(self, zoom: f32) -> Self {
        match self {
            ScrollDelta::Pixels(delta) => ScrollDelta::Pixels(delta.zoomed(zoom)),
            ScrollDelta::Lines(lines) => ScrollDelta::Lines(lines),
        }
    }
}

/// Moves the positions of an event from the platform into the window's
/// zoomed units. Events the window makes itself are already in them.
pub(crate) fn unzoomed_input(input: PlatformInput, zoom: f32) -> PlatformInput {
    if zoom == 1. {
        return input;
    }
    let at = |position: Point<Pixels>| position.unzoomed(zoom);
    match input {
        PlatformInput::MouseDown(event) => PlatformInput::MouseDown(MouseDownEvent {
            position: at(event.position),
            ..event
        }),
        PlatformInput::MouseUp(event) => PlatformInput::MouseUp(MouseUpEvent {
            position: at(event.position),
            ..event
        }),
        PlatformInput::MousePressure(event) => {
            PlatformInput::MousePressure(MousePressureEvent {
                position: at(event.position),
                ..event
            })
        }
        PlatformInput::MouseMove(event) => PlatformInput::MouseMove(MouseMoveEvent {
            position: at(event.position),
            ..event
        }),
        PlatformInput::MouseExited(event) => PlatformInput::MouseExited(MouseExitEvent {
            position: at(event.position),
            ..event
        }),
        PlatformInput::Scroll(event) => PlatformInput::Scroll(ScrollEvent {
            position: at(event.position),
            delta: event.delta.unzoomed(zoom),
            ..event
        }),
        PlatformInput::Pinch(event) => PlatformInput::Pinch(PinchEvent {
            position: at(event.position),
            ..event
        }),
        PlatformInput::LongPress(event) => PlatformInput::LongPress(LongPressEvent {
            start_position: at(event.start_position),
            position: at(event.position),
            ..event
        }),
        PlatformInput::TouchDrag(event) => PlatformInput::TouchDrag(TouchDragEvent {
            start_position: at(event.start_position),
            position: at(event.position),
            ..event
        }),
        PlatformInput::BackGesture(event) => PlatformInput::BackGesture(BackGestureEvent {
            start_position: at(event.start_position),
            ..event
        }),
        PlatformInput::Touch(event) => PlatformInput::Touch(TouchEvent {
            position: at(event.position),
            predicted_position: event.predicted_position.map(at),
            ..event
        }),
        PlatformInput::FileDrop(FileDropEvent::Entered { position, paths }) => {
            PlatformInput::FileDrop(FileDropEvent::Entered {
                position: at(position),
                paths,
            })
        }
        PlatformInput::FileDrop(FileDropEvent::Pending { position }) => {
            PlatformInput::FileDrop(FileDropEvent::Pending {
                position: at(position),
            })
        }
        PlatformInput::FileDrop(FileDropEvent::Submit { position }) => {
            PlatformInput::FileDrop(FileDropEvent::Submit {
                position: at(position),
            })
        }
        input @ (PlatformInput::KeyDown(_)
        | PlatformInput::KeyUp(_)
        | PlatformInput::ModifiersChanged(_)
        | PlatformInput::FileDrop(FileDropEvent::Exited | FileDropEvent::Ended)) => input,
    }
}
