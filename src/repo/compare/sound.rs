use std::sync::Arc;

use gpui_kit::{
    AnyElement, Bounds, Context, Corners, FocusHandle, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, PaintQuad, ParentElement as _, Pixels,
    Render, ScrollEvent, SharedString, Styled as _, Window,
    assets::IconName,
    canvas,
    component::{ActiveTheme as _, h_flex, v_flex},
    div, fill, point,
    prelude::FluentBuilder as _,
    px, size,
};

use super::{
    Direction, Position,
    details::{DetailsRow, MediaDetails, Version, VersionTag},
    picture::{file_size, signed_size},
    transport::{Transport, clock},
};
use crate::{
    actions::{LoopChange, MEDIA_CONTEXT, PlayPause, SwitchVersion},
    media::{Player, Region, Sound, audible_changes},
    ui::{Look as _, ctrl_wheel_area, handler},
};

const RULER: f32 = 28.;
const LANE_TOP: f32 = 40.;
const LANE_BOTTOM: f32 = 12.;
const BAR_STEP: f32 = 2.;
const BAR_WIDTH: f32 = 1.4;
const QUIET_LANE: f32 = 0.4;
const STRIPE_STEP: f32 = 6.;
const BADGE: f32 = 18.;
const SHORTEST_VIEW: f64 = 0.5;
const ZOOM_STEP: f64 = 1.25;
const WHEEL_LINE: f32 = 20.;
const TICK_GAP: f64 = 70.;
const TICK_STEPS: [f64; 10] = [0.5, 1., 2., 5., 10., 15., 30., 60., 120., 300.];
const DETAILS: [&str; 8] = [
    "Duration",
    "Sample rate",
    "Channels",
    "Codec",
    "Bit rate",
    "Loudness",
    "Peak",
    "File size",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundMode {
    Split,
    Difference,
}

impl SoundMode {
    pub(crate) const ALL: [SoundMode; 2] = [SoundMode::Split, SoundMode::Difference];

    pub(crate) fn id(self) -> &'static str {
        match self {
            SoundMode::Split => "sound-split",
            SoundMode::Difference => "sound-difference",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            SoundMode::Split => "Split",
            SoundMode::Difference => "Difference",
        }
    }

    pub(crate) fn icon(self) -> IconName {
        match self {
            SoundMode::Split => IconName::AudioLines,
            SoundMode::Difference => IconName::Diff,
        }
    }

    pub(crate) fn tooltip(self) -> &'static str {
        match self {
            SoundMode::Split => "Show both waveforms on one time scale",
            SoundMode::Difference => "Show only what sounds different",
        }
    }
}

pub(crate) struct Sounds {
    before: Option<Arc<Sound>>,
    after: Option<Arc<Sound>>,
    regions: Vec<Region>,
}

impl Sounds {
    pub(crate) fn between(before: Option<Sound>, after: Option<Sound>) -> Self {
        let regions = match (&before, &after) {
            (Some(before), Some(after)) => audible_changes(before, after),
            _ => Vec::new(),
        };
        Self {
            before: before.map(Arc::new),
            after: after.map(Arc::new),
            regions,
        }
    }

    fn longest(&self) -> f64 {
        [&self.before, &self.after]
            .into_iter()
            .flatten()
            .map(|sound| sound.duration())
            .fold(0., f64::max)
    }

    fn get(&self, version: Version) -> Option<&Arc<Sound>> {
        match version {
            Version::Before => self.before.as_ref(),
            Version::After => self.after.as_ref(),
        }
    }

    pub(crate) fn facts(&self) -> SharedString {
        let length = |sound: &Arc<Sound>| clock(sound.duration(), 1);
        match (&self.before, &self.after) {
            (Some(before), Some(after)) => format!("{} → {}", length(before), length(after)).into(),
            (Some(only), None) | (None, Some(only)) => length(only).into(),
            (None, None) => SharedString::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Lane {
    bounds: Bounds<Pixels>,
    version: Option<Version>,
}

pub(crate) struct SoundCompare {
    sounds: Sounds,
    imports: Option<crate::unity::ImportChanges>,
    sources: [SharedString; 2],
    mode: SoundMode,
    player: Player,
    view: (f64, f64),
    current: Option<usize>,
    repeating: bool,
    scrubbing: bool,
    area: Option<Bounds<Pixels>>,
    trouble: Option<SharedString>,
    focus: FocusHandle,
}

impl SoundCompare {
    pub(crate) fn set_imports(&mut self, imports: Option<crate::unity::ImportChanges>) {
        self.imports = imports;
    }

    pub(crate) fn new(sounds: Sounds, sources: [SharedString; 2], cx: &mut Context<Self>) -> Self {
        let player = Player::new(sounds.before.clone(), sounds.after.clone());
        let longest = sounds.longest();
        Self {
            sounds,
            imports: None,
            sources,
            mode: SoundMode::Split,
            player,
            view: (0., longest.max(SHORTEST_VIEW)),
            current: None,
            repeating: false,
            scrubbing: false,
            area: None,
            trouble: None,
            focus: cx.focus_handle(),
        }
    }

    pub(crate) fn facts(&self) -> SharedString {
        self.sounds.facts()
    }

    pub(crate) fn mode(&self) -> SoundMode {
        self.mode
    }

    pub(crate) fn compares_both(&self) -> bool {
        self.sounds.before.is_some() && self.sounds.after.is_some()
    }

    pub(crate) fn set_mode(&mut self, mode: SoundMode, cx: &mut Context<Self>) {
        if self.compares_both() {
            self.mode = mode;
            cx.notify();
        }
    }

    pub(crate) fn fits(&self) -> bool {
        self.view.0 <= 0. && self.view.1 >= self.sounds.longest()
    }

    pub(crate) fn fit(&mut self, cx: &mut Context<Self>) {
        self.view = (0., self.sounds.longest().max(SHORTEST_VIEW));
        cx.notify();
    }

    pub(crate) fn zoom_in(&mut self, cx: &mut Context<Self>) {
        let middle = (self.view.0 + self.view.1) / 2.;
        self.zoom_around(middle, 1. / ZOOM_STEP);
        cx.notify();
    }

    pub(crate) fn zoom_out(&mut self, cx: &mut Context<Self>) {
        let middle = (self.view.0 + self.view.1) / 2.;
        self.zoom_around(middle, ZOOM_STEP);
        cx.notify();
    }

    pub(crate) fn change_position(&self) -> Option<Position> {
        let count = self.sounds.regions.len();
        (count > 0).then(|| Position {
            index: self.current.unwrap_or(0).min(count - 1),
            count,
        })
    }

    pub(crate) fn step_change(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let count = self.sounds.regions.len();
        if count == 0 {
            return;
        }
        let next = match (self.current, direction) {
            (None, Direction::Forward) => 0,
            (None, Direction::Back) => count - 1,
            (Some(at), Direction::Forward) => (at + 1) % count,
            (Some(at), Direction::Back) => (at + count - 1) % count,
        };
        self.current = Some(next);
        let region = self.sounds.regions[next];
        self.player.seek(region.from);
        self.show(region.from, region.to);
        self.sync_repeat();
        cx.notify();
    }

    fn sync_repeat(&self) {
        let range = self
            .current
            .filter(|_| self.repeating)
            .and_then(|at| self.sounds.regions.get(at))
            .map(|region| (region.from, region.to));
        self.player.repeat(range);
    }

    fn show(&mut self, from: f64, to: f64) {
        let span = self.view.1 - self.view.0;
        if from >= self.view.0 && to <= self.view.1 {
            return;
        }
        let start = (from - span * 0.1).max(0.);
        self.view = (start, start + span);
    }

    fn zoom_around(&mut self, at: f64, factor: f64) {
        let longest = self.sounds.longest().max(SHORTEST_VIEW);
        let span = ((self.view.1 - self.view.0) * factor).clamp(SHORTEST_VIEW, longest);
        let share = (at - self.view.0) / (self.view.1 - self.view.0).max(f64::EPSILON);
        let start = (at - span * share).clamp(0., (longest - span).max(0.));
        self.view = (start, start + span);
    }

    fn toggle_play(&mut self, cx: &mut Context<Self>) {
        self.trouble = self.player.toggle().err().map(|error| format!("{error:#}").into());
        cx.notify();
    }

    fn switch_version(&mut self, cx: &mut Context<Self>) {
        if self.compares_both() {
            self.player.hear(1 - self.player.heard());
            cx.notify();
        }
    }

    fn hear(&mut self, version: Version, cx: &mut Context<Self>) {
        self.player.hear(match version {
            Version::Before => 0,
            Version::After => 1,
        });
        cx.notify();
    }

    fn toggle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeating = !self.repeating;
        if self.repeating && self.current.is_none() {
            let position = self.player.position();
            self.current = self
                .sounds
                .regions
                .iter()
                .position(|region| region.to > position)
                .or((!self.sounds.regions.is_empty()).then_some(0));
        }
        self.sync_repeat();
        cx.notify();
    }

    fn heard(&self) -> Option<Version> {
        let heard = if self.player.heard() == 0 { Version::Before } else { Version::After };
        self.sounds.get(heard).map(|_| heard)
    }

    fn time_at(&self, x: Pixels) -> Option<f64> {
        let area = self.area?;
        let share = f64::from((x - area.origin.x) / area.size.width).clamp(0., 1.);
        Some(self.view.0 + share * (self.view.1 - self.view.0))
    }

    fn x_of(&self, area: Bounds<Pixels>, seconds: f64) -> Pixels {
        let share = (seconds - self.view.0) / (self.view.1 - self.view.0).max(f64::EPSILON);
        area.origin.x + area.size.width * share as f32
    }

    fn lanes(&self, area: Bounds<Pixels>) -> Vec<Lane> {
        let top = area.origin.y + px(RULER);
        let height = area.size.height - px(RULER);
        let versions: Vec<Option<Version>> = match self.mode {
            SoundMode::Difference if self.compares_both() => vec![None],
            _ => [Version::Before, Version::After]
                .into_iter()
                .filter(|version| self.sounds.get(*version).is_some())
                .map(Some)
                .collect(),
        };
        let count = versions.len().max(1) as f32;
        versions
            .into_iter()
            .enumerate()
            .map(|(ix, version)| Lane {
                bounds: Bounds {
                    origin: point(area.origin.x, top + height / count * ix as f32),
                    size: size(area.size.width, height / count),
                },
                version,
            })
            .collect()
    }

    fn on_press(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        if let Some(at) = self.time_at(event.position.x) {
            self.player.seek(at);
            self.current = self
                .sounds
                .regions
                .iter()
                .position(|region| (region.from..region.to).contains(&at))
                .or(self.current);
            self.sync_repeat();
            self.scrubbing = true;
            cx.notify();
        }
    }

    fn on_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.scrubbing && event.pressed_button == Some(MouseButton::Left) {
            if let Some(at) = self.time_at(event.position.x) {
                self.player.seek(at);
                cx.notify();
            }
        } else {
            self.scrubbing = false;
        }
    }

    fn zoom_by_wheel(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) -> bool {
        let delta = event.delta.pixel_delta(px(WHEEL_LINE));
        let at = self.time_at(event.position.x).unwrap_or((self.view.0 + self.view.1) / 2.);
        let factor = if f32::from(delta.y) > 0. { 1. / ZOOM_STEP } else { ZOOM_STEP };
        self.zoom_around(at, factor);
        cx.notify();
        true
    }

    fn on_wheel(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(WHEEL_LINE));
        let span = self.view.1 - self.view.0;
        if let Some(area) = self.area {
            let moved = if f32::from(delta.x).abs() > f32::from(delta.y).abs() { delta.x } else { delta.y };
            let shift = -f64::from(moved / area.size.width) * span;
            let longest = self.sounds.longest().max(SHORTEST_VIEW);
            let start = (self.view.0 + shift).clamp(0., (longest - span).max(0.));
            self.view = (start, start + span);
        }
        cx.notify();
    }

    fn follow_playhead(&mut self) {
        let position = self.player.position();
        let span = self.view.1 - self.view.0;
        if position > self.view.1 || position < self.view.0 {
            let longest = self.sounds.longest().max(SHORTEST_VIEW);
            let start = (position - span * 0.1).clamp(0., (longest - span).max(0.));
            self.view = (start, start + span);
        }
    }

    fn paint_list(&mut self, area: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Vec<PaintQuad> {
        if self.area != Some(area) {
            self.area = Some(area);
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        let theme = cx.theme();
        let heard = self.heard();
        let mut quads = Vec::new();
        quads.push(fill(
            Bounds {
                origin: point(area.origin.x, area.origin.y + px(RULER - 1.)),
                size: size(area.size.width, px(1.)),
            },
            theme.border,
        ));
        for tick in ticks(self.view, f64::from(f32::from(area.size.width))) {
            let x = self.x_of(area, tick);
            quads.push(fill(
                Bounds {
                    origin: point(x, area.origin.y + px(RULER - 9.)),
                    size: size(px(1.), px(8.)),
                },
                theme.muted_foreground.opacity(0.6),
            ));
        }
        for (ix, region) in self.sounds.regions.iter().enumerate() {
            let left = self.x_of(area, region.from).max(area.origin.x);
            let right = self.x_of(area, region.to).min(area.origin.x + area.size.width);
            if right <= left {
                continue;
            }
            let current = self.current == Some(ix);
            let band = Bounds {
                origin: point(left, area.origin.y),
                size: size(right - left, area.size.height),
            };
            quads.push(fill(band, theme.yellow.opacity(if current { 0.12 } else { 0.06 })));
            let edge = theme.yellow.opacity(if current { 0.75 } else { 0.35 });
            for x in [left, right - px(1.)] {
                quads.push(fill(
                    Bounds {
                        origin: point(x, area.origin.y),
                        size: size(px(1.), area.size.height),
                    },
                    edge,
                ));
            }
        }
        let columns = (f32::from(area.size.width) / BAR_STEP).max(1.) as usize;
        for (ix, lane) in self.lanes(area).iter().enumerate() {
            if ix > 0 {
                quads.push(fill(
                    Bounds {
                        origin: lane.bounds.origin,
                        size: size(lane.bounds.size.width, px(1.)),
                    },
                    theme.border,
                ));
            }
            let levels = match lane.version {
                Some(version) => self
                    .sounds
                    .get(version)
                    .map(|sound| sound.levels(self.view.0, self.view.1, columns))
                    .unwrap_or_default(),
                None => self.difference_levels(columns),
            };
            let quiet = lane.version.is_some() && lane.version != heard;
            let opacity = if quiet { QUIET_LANE } else { 1. };
            let height = (lane.bounds.size.height - px(LANE_TOP + LANE_BOTTOM)).max(px(2.));
            let middle = lane.bounds.origin.y + px(LANE_TOP) + height / 2.;
            for (column, level) in levels.iter().enumerate() {
                let Some((low, high)) = *level else {
                    continue;
                };
                let seconds = self.view.0 + (self.view.1 - self.view.0) * (column as f64 + 0.5) / columns as f64;
                let changed = self
                    .sounds
                    .regions
                    .iter()
                    .any(|region| (region.from..region.to).contains(&seconds));
                let color: Hsla = if changed || lane.version.is_none() { theme.yellow } else { theme.blue };
                let top = middle - height / 2. * high.clamp(-1., 1.);
                let bottom = middle - height / 2. * low.clamp(-1., 1.);
                quads.push(fill(
                    Bounds {
                        origin: point(area.origin.x + px(column as f32 * BAR_STEP), top),
                        size: size(px(BAR_WIDTH), (bottom - top).max(px(1.))),
                    },
                    color.opacity(opacity),
                ));
            }
            if let Some(sound) = lane.version.and_then(|version| self.sounds.get(version)) {
                let ended = self.x_of(area, sound.duration());
                let right = area.origin.x + area.size.width;
                let mut x = ended.max(area.origin.x);
                while x < right {
                    quads.push(fill(
                        Bounds {
                            origin: point(x, lane.bounds.origin.y + px(1.)),
                            size: size(px(2.), lane.bounds.size.height - px(1.)),
                        },
                        theme.foreground.opacity(0.06),
                    ));
                    x += px(STRIPE_STEP);
                }
            }
        }
        let playhead = self.x_of(area, self.player.position());
        if playhead >= area.origin.x && playhead <= area.origin.x + area.size.width {
            quads.push(fill(
                Bounds {
                    origin: point(playhead - px(1.), area.origin.y),
                    size: size(px(2.), area.size.height),
                },
                theme.foreground,
            ));
            quads.push(
                fill(
                    Bounds {
                        origin: point(playhead - px(5.), area.origin.y),
                        size: size(px(10.), px(10.)),
                    },
                    theme.foreground,
                )
                .corner_radii(Corners::all(px(5.))),
            );
        }
        quads
    }

    fn difference_levels(&self, columns: usize) -> Vec<Option<(f32, f32)>> {
        let (Some(before), Some(after)) = (&self.sounds.before, &self.sounds.after) else {
            return Vec::new();
        };
        let span = (self.view.1 - self.view.0).max(f64::EPSILON);
        let longest = self.sounds.longest();
        (0..columns)
            .map(|column| {
                let start = self.view.0 + span * column as f64 / columns as f64;
                if start >= longest {
                    return None;
                }
                let end = self.view.0 + span * (column + 1) as f64 / columns as f64;
                let samples = 64;
                let biggest = (0..samples)
                    .map(|ix| {
                        let at = start + (end - start) * ix as f64 / samples as f64;
                        (after.at(at, 0) - before.at(at, 0)).abs()
                    })
                    .fold(0f32, f32::max);
                Some((-biggest, biggest))
            })
            .collect()
    }

    fn overlays(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(area) = self.area else {
            return Vec::new();
        };
        let theme = cx.theme();
        let relative = |x: Pixels| x - area.origin.x;
        let mut overlays: Vec<AnyElement> = ticks(self.view, f64::from(f32::from(area.size.width)))
            .into_iter()
            .map(|tick| {
                div()
                    .absolute()
                    .left(relative(self.x_of(area, tick)) + px(4.))
                    .top(px(4.))
                    .text_size(px(11.))
                    .font_family(theme.mono_font_family.clone())
                    .text_color(theme.muted_foreground)
                    .child(clock(tick, 0))
                    .into_any_element()
            })
            .collect();
        let heard = self.heard();
        for lane in self.lanes(area) {
            let top = lane.bounds.origin.y - area.origin.y + px(10.);
            let tag = match lane.version {
                Some(version) => VersionTag::new(version, self.sources[usize::from(version == Version::After)].clone())
                    .heard(heard == Some(version))
                    .into_any_element(),
                None => h_flex()
                    .h(px(22.))
                    .px_2()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover.opacity(0.88))
                    .xs_text()
                    .child("What sounds different")
                    .into_any_element(),
            };
            overlays.push(div().absolute().left_0().top(top).child(tag).into_any_element());
        }
        for (ix, region) in self.sounds.regions.iter().enumerate() {
            let middle = self.x_of(area, (region.from + region.to) / 2.);
            if middle < area.origin.x || middle > area.origin.x + area.size.width {
                continue;
            }
            let current = self.current == Some(ix);
            overlays.push(
                div()
                    .absolute()
                    .left(relative(middle) - px(BADGE / 2.))
                    .top(px(RULER + 6.))
                    .size(px(BADGE))
                    .rounded_full()
                    .border_1()
                    .border_color(if current { theme.foreground } else { theme.border })
                    .bg(if current { theme.foreground } else { theme.secondary })
                    .text_color(if current { theme.background } else { theme.foreground })
                    .text_size(px(11.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(format!("{}", ix + 1))
                    .into_any_element(),
            );
        }
        overlays
    }

    fn details(&self) -> MediaDetails {
        let values = |sound: &Sound| -> Vec<SharedString> {
            vec![
                clock(sound.duration(), 1).into(),
                format!("{} kHz", f64::from(sound.rate) / 1000.).into(),
                sound.channel_name().into(),
                sound.codec_name().into(),
                sound
                    .bit_rate()
                    .map_or_else(|| "—".to_owned(), |rate| format!("{} kb/s", rate / 1000))
                    .into(),
                sound
                    .loudness
                    .map_or_else(|| "Silent".to_owned(), |lufs| format!("{} LUFS", signed(lufs, 1)))
                    .into(),
                if sound.peak.is_finite() {
                    format!("{} dB", signed(sound.peak, 1)).into()
                } else {
                    "Silent".into()
                },
                file_size(sound.bytes).into(),
            ]
        };
        let mut details = MediaDetails::new(DETAILS.to_vec());
        for version in [Version::Before, Version::After] {
            if let Some(sound) = self.sounds.get(version) {
                details = details.row(DetailsRow {
                    version,
                    source: self.sources[usize::from(version == Version::After)].clone(),
                    values: values(sound),
                });
            }
        }
        let deltas = match (&self.sounds.before, &self.sounds.after) {
            (Some(before), Some(after)) => {
                let longer = after.duration() - before.duration();
                let louder = after.loudness.zip(before.loudness).map(|(after, before)| after - before);
                vec![
                    Some(format!("{} s", signed(longer, 1)).into()),
                    None,
                    None,
                    None,
                    None,
                    louder.map(|louder| signed(louder, 1).into()),
                    None,
                    Some(signed_size(after.bytes as i64 - before.bytes as i64).into()),
                ]
            }
            _ => Vec::new(),
        };
        details.deltas(deltas)
    }
}

impl Render for SoundCompare {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playing = self.player.is_playing();
        if playing {
            self.follow_playhead();
            window.request_animation_frame();
        }
        let entity = cx.entity();
        let waves = canvas(
            move |bounds, window, cx| entity.update(cx, |this, cx| this.paint_list(bounds, window, cx)),
            |_, quads, window, _| {
                for quad in quads {
                    window.paint_quad(quad);
                }
            },
        )
        .absolute()
        .size_full();
        let overlays = self.overlays(cx);
        let length = self.sounds.longest();
        let transport = Transport::new(
            self.player.position(),
            length,
            playing,
            self.repeating,
            self.heard(),
            handler(cx.listener(|this, _: &(), _, cx| this.toggle_play(cx))),
            handler(cx.listener(|this, _: &(), _, cx| this.toggle_repeat(cx))),
            handler(cx.listener(|this, version: &Version, _, cx| this.hear(*version, cx))),
        )
        .trouble(self.trouble.clone());
        v_flex()
            .size_full()
            .key_context(MEDIA_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &PlayPause, _, cx| this.toggle_play(cx)))
            .on_action(cx.listener(|this, _: &SwitchVersion, _, cx| this.switch_version(cx)))
            .on_action(cx.listener(|this, _: &LoopChange, _, cx| this.toggle_repeat(cx)))
            .child(
                div().flex_1().min_h_0().px_3().pb_3().child(
                    div()
                        .id("sound-lanes")
                        .relative()
                        .size_full()
                        .overflow_hidden()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| this.on_press(event, window, cx)))
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| this.on_move(event, cx)))
                        .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.scrubbing = false))
                        .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| this.on_wheel(event, cx)))
                        .child(waves)
                        .children(overlays)
                        .child(ctrl_wheel_area(cx.entity().downgrade(), Self::zoom_by_wheel)),
                ),
            )
            .child(transport)
            .child(self.details())
            .when_some(self.imports.as_ref(), |this, imports| this.child(super::details::import_table(imports, &self.sources)))
    }
}

fn ticks(view: (f64, f64), width: f64) -> Vec<f64> {
    let span = (view.1 - view.0).max(f64::EPSILON);
    let step = TICK_STEPS
        .into_iter()
        .find(|step| step / span * width >= TICK_GAP)
        .unwrap_or(600.);
    let first = (view.0 / step).ceil() * step;
    (0..)
        .map(|ix| first + step * f64::from(ix))
        .take_while(|tick| *tick <= view.1)
        .collect()
}

fn signed(value: f64, digits: usize) -> String {
    let sign = if value < 0. { "−" } else if value > 0. { "+" } else { "" };
    format!("{sign}{:.digits$}", value.abs())
}

#[cfg(test)]
mod tests {
    use super::{signed, ticks};

    #[test]
    fn ruler_ticks_stay_readable_at_any_zoom() {
        assert_eq!(ticks((0., 160.), 700.), [0., 30., 60., 90., 120., 150.]);
        assert_eq!(ticks((10.2, 13.), 700.), [10.5, 11., 11.5, 12., 12.5, 13.]);
    }

    #[test]
    fn signed_numbers_use_a_true_minus() {
        assert_eq!(signed(-14.21, 1), "−14.2");
        assert_eq!(signed(1.44, 1), "+1.4");
        assert_eq!(signed(0., 1), "0.0");
    }
}
