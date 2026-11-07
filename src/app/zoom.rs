use gpui_kit::{App, BorrowAppContext as _, Global, Pixels, Point, ScrollEvent, Window, px};
use serde_json::Value;

use crate::settings::{Fields, Folder, JsonFile};

const SETTINGS_FILE: &str = "settings.json";
const ZOOM_FIELD: &str = "zoom";
const LEVELS: [u16; 11] = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200];
const WHEEL_LINE: f32 = 20.;
const WHEEL_DISTANCE_PER_STEP: f32 = 50.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Zoom(u16);

impl Zoom {
    pub(crate) const ACTUAL_SIZE: Zoom = Zoom(100);

    fn from_percent(percent: u64) -> Option<Zoom> {
        let percent = u16::try_from(percent).ok()?;
        (LEVELS[0]..=LEVELS[LEVELS.len() - 1])
            .contains(&percent)
            .then_some(Zoom(percent))
    }

    fn read(fields: &Fields) -> Zoom {
        fields
            .get(ZOOM_FIELD)
            .and_then(Value::as_u64)
            .and_then(Zoom::from_percent)
            .unwrap_or(Zoom::ACTUAL_SIZE)
    }

    fn write(self, fields: &mut Fields) {
        fields.insert(ZOOM_FIELD.into(), self.0.into());
    }

    pub(crate) fn percent(self) -> u16 {
        self.0
    }

    pub(crate) fn factor(self) -> f32 {
        f32::from(self.0) / 100.
    }

    pub(crate) fn bigger(self) -> Option<Zoom> {
        LEVELS.into_iter().find(|&level| level > self.0).map(Zoom)
    }

    pub(crate) fn smaller(self) -> Option<Zoom> {
        LEVELS.into_iter().rev().find(|&level| level < self.0).map(Zoom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    Bigger,
    Smaller,
}

#[derive(Default)]
struct Wheel {
    gathered: f32,
}

impl Wheel {
    fn turn(&mut self, distance: f32) -> Option<Turn> {
        let same_way = self.gathered == 0. || self.gathered.signum() == distance.signum();
        let gathered = if same_way { self.gathered + distance } else { distance };
        if gathered.abs() < WHEEL_DISTANCE_PER_STEP {
            self.gathered = gathered;
            return None;
        }
        self.gathered = 0.;
        Some(if gathered > 0. { Turn::Bigger } else { Turn::Smaller })
    }
}

struct WindowZoom {
    zoom: Zoom,
    wheel: Wheel,
}

impl Global for WindowZoom {}

fn settings_file() -> Option<JsonFile> {
    JsonFile::in_folder(Folder::Config, SETTINGS_FILE)
}

pub(crate) fn install_zoom(cx: &mut App) {
    let zoom = settings_file().map_or(Zoom::ACTUAL_SIZE, |file| Zoom::read(&file.read()));
    cx.set_global(WindowZoom {
        zoom,
        wheel: Wheel::default(),
    });
}

pub(crate) fn current_zoom(cx: &App) -> Zoom {
    cx.try_global::<WindowZoom>()
        .map_or(Zoom::ACTUAL_SIZE, |state| state.zoom)
}

pub(crate) fn apply_zoom(window: &mut Window, cx: &mut App) {
    window.set_content_zoom(current_zoom(cx).factor(), cx);
}

pub(crate) fn zoom_to(zoom: Zoom, window: &mut Window, cx: &mut App) {
    if zoom == current_zoom(cx) {
        return;
    }
    cx.update_global::<WindowZoom, _>(|state, _| state.zoom = zoom);
    if let Some(file) = settings_file() {
        let _ = file.update(|fields| zoom.write(fields));
    }
    window.set_content_zoom(zoom.factor(), cx);
}

pub(crate) fn zoom_in(window: &mut Window, cx: &mut App) {
    if let Some(bigger) = current_zoom(cx).bigger() {
        zoom_to(bigger, window, cx);
    }
}

pub(crate) fn zoom_out(window: &mut Window, cx: &mut App) {
    if let Some(smaller) = current_zoom(cx).smaller() {
        zoom_to(smaller, window, cx);
    }
}

pub(crate) fn reset_zoom(window: &mut Window, cx: &mut App) {
    zoom_to(Zoom::ACTUAL_SIZE, window, cx);
}

fn wheel_distance(delta: Point<Pixels>) -> f32 {
    match f32::from(delta.y) {
        0. => f32::from(delta.x),
        vertical => vertical,
    }
}

pub(crate) fn zoom_by_wheel(event: &ScrollEvent, window: &mut Window, cx: &mut App) {
    let distance = wheel_distance(event.delta.pixel_delta(px(WHEEL_LINE)));
    let turn = cx.update_global::<WindowZoom, _>(|state, _| state.wheel.turn(distance));
    match turn {
        Some(Turn::Bigger) => zoom_in(window, cx),
        Some(Turn::Smaller) => zoom_out(window, cx),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{point, px};
    use serde_json::json;

    use super::{Fields, LEVELS, Turn, WHEEL_DISTANCE_PER_STEP, WHEEL_LINE, Wheel, Zoom, wheel_distance};

    fn fields(value: serde_json::Value) -> Fields {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn bigger_and_smaller_walk_the_levels_one_at_a_time() {
        let mut zoom = Zoom::ACTUAL_SIZE;
        let mut walked = vec![zoom.percent()];
        while let Some(bigger) = zoom.bigger() {
            zoom = bigger;
            walked.push(zoom.percent());
        }
        assert_eq!(walked, [100, 110, 125, 150, 175, 200]);
        assert_eq!(Zoom::ACTUAL_SIZE.smaller(), Some(Zoom(90)));
    }

    #[test]
    fn the_smallest_and_largest_levels_go_no_further() {
        assert_eq!(Zoom(LEVELS[0]).smaller(), None);
        assert_eq!(Zoom(LEVELS[LEVELS.len() - 1]).bigger(), None);
    }

    #[test]
    fn a_zoom_between_levels_steps_to_its_neighbours() {
        assert_eq!(Zoom(130).bigger(), Some(Zoom(150)));
        assert_eq!(Zoom(130).smaller(), Some(Zoom(125)));
    }

    #[test]
    fn the_zoom_reads_back_from_the_settings_file() {
        let mut saved = fields(json!({ "kept": true }));
        Zoom(125).write(&mut saved);
        assert_eq!(saved, fields(json!({ "kept": true, "zoom": 125 })));
        assert_eq!(Zoom::read(&saved), Zoom(125));
    }

    #[test]
    fn a_missing_or_impossible_zoom_reads_as_actual_size() {
        for saved in [json!({}), json!({ "zoom": 20 }), json!({ "zoom": 900 }), json!({ "zoom": "big" })] {
            assert_eq!(Zoom::read(&fields(saved)), Zoom::ACTUAL_SIZE);
        }
    }

    #[test]
    fn one_wheel_notch_turns_one_level() {
        let notch = 3. * WHEEL_LINE;
        let mut wheel = Wheel::default();
        assert_eq!(wheel.turn(notch), Some(Turn::Bigger));
        assert_eq!(wheel.turn(notch), Some(Turn::Bigger));
        assert_eq!(wheel.turn(-notch), Some(Turn::Smaller));
    }

    #[test]
    fn small_touchpad_moves_add_up_before_turning() {
        let mut wheel = Wheel::default();
        let step = WHEEL_DISTANCE_PER_STEP / 5.;
        let turns: Vec<_> = (0..10).map(|_| wheel.turn(step)).collect();
        assert_eq!(turns.iter().flatten().count(), 2);
        assert_eq!(turns[4], Some(Turn::Bigger));
    }

    #[test]
    fn a_wheel_turned_with_shift_held_zooms_too_though_windows_turns_it_sideways() {
        assert_eq!(wheel_distance(point(px(60.), px(0.))), 60.);
        assert_eq!(wheel_distance(point(px(5.), px(-60.))), -60.);
    }

    #[test]
    fn turning_back_starts_gathering_again() {
        let mut wheel = Wheel::default();
        assert_eq!(wheel.turn(WHEEL_DISTANCE_PER_STEP - 1.), None);
        assert_eq!(wheel.turn(-1.), None);
        assert_eq!(wheel.turn(-(WHEEL_DISTANCE_PER_STEP - 1.)), Some(Turn::Smaller));
    }
}
