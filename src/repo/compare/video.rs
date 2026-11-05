use std::{path::PathBuf, sync::Arc};

use gpui_kit::{
    AnyElement, AppContext as _, Bounds, Context, Corners, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, PaintQuad, ParentElement as _, Pixels,
    Render, SharedString, Styled as _, Window, canvas,
    component::{ActiveTheme as _, h_flex, v_flex},
    div, fill, point, px, size,
};

use super::{
    Direction, PictureCompare, Pictures, Position,
    details::{DetailsRow, MediaDetails, Version, version_dot},
    picture::{file_size, signed_size},
    transport::{Transport, clock},
};
use crate::{
    actions::{LoopChange, MEDIA_CONTEXT, PlayPause, SwitchVersion},
    media::{FrameStream, Picture, Player, Region, Sound, VideoFacts, frame_at},
    ui::{Job, Look as _, handler, run_in_background},
};

const WIDEST_FRAME: u32 = 960;
const MOST_FRAMES_PER_SECOND: f64 = 24.;
const LABELS: f32 = 132.;
const NUMBERS: f32 = 22.;
const LANE: f32 = 12.;
const LANE_GAP: f32 = 8.;
const TICKS: f32 = 16.;
const BADGE: f32 = 18.;
const TICK_GAP: f64 = 70.;
const TICK_STEPS: [f64; 9] = [1., 2., 5., 10., 15., 30., 60., 120., 300.];
const DETAILS: [&str; 7] = ["Resolution", "Frame rate", "Duration", "Video", "Audio", "Bit rate", "File size"];

pub(crate) struct TemporaryCopy(pub(crate) PathBuf);

impl Drop for TemporaryCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub(crate) struct VideoSide {
    pub(crate) file: PathBuf,
    pub(crate) facts: VideoFacts,
    pub(crate) sound: Option<Arc<Sound>>,
    pub(crate) _copy: Option<TemporaryCopy>,
}

pub(crate) struct Videos {
    pub(crate) before: Option<VideoSide>,
    pub(crate) after: Option<VideoSide>,
    pub(crate) changes: Vec<Region>,
}

impl Videos {
    fn side(&self, version: Version) -> Option<&VideoSide> {
        match version {
            Version::Before => self.before.as_ref(),
            Version::After => self.after.as_ref(),
        }
    }

    fn length(&self) -> f64 {
        [&self.before, &self.after]
            .into_iter()
            .flatten()
            .map(|side| side.facts.duration)
            .fold(0., f64::max)
    }

    fn frame_step(&self) -> f64 {
        let rate = self
            .after
            .as_ref()
            .or(self.before.as_ref())
            .and_then(|side| side.facts.frame_rate)
            .unwrap_or(30.);
        1. / rate
    }

    pub(crate) fn facts(&self) -> SharedString {
        let length = |side: &VideoSide| clock(side.facts.duration, 2);
        match (&self.before, &self.after) {
            (Some(before), Some(after)) => format!("{} → {}", length(before), length(after)).into(),
            (Some(only), None) | (None, Some(only)) => length(only).into(),
            (None, None) => SharedString::default(),
        }
    }
}

pub(crate) struct VideoCompare {
    videos: Videos,
    sources: [SharedString; 2],
    frames: Entity<PictureCompare>,
    player: Player,
    current: Option<usize>,
    repeating: bool,
    shown: [Option<Arc<Picture>>; 2],
    fetching: Job,
    wanted: Option<f64>,
    streams: Vec<(usize, FrameStream)>,
    trouble: Option<SharedString>,
    area: Option<Bounds<Pixels>>,
    scrubbing: bool,
    focus: FocusHandle,
}

impl VideoCompare {
    pub(crate) fn new(videos: Videos, sources: [SharedString; 2], cx: &mut Context<Self>) -> Self {
        let frames = cx.new(|_| PictureCompare::new(Pictures::between(None, None), sources.clone()).without_details());
        let player = Player::lasting(
            videos.before.as_ref().and_then(|side| side.sound.clone()),
            videos.after.as_ref().and_then(|side| side.sound.clone()),
            videos.length(),
        );
        let mut this = Self {
            videos,
            sources,
            frames,
            player,
            current: None,
            repeating: false,
            shown: [None, None],
            fetching: Job::default(),
            wanted: None,
            streams: Vec::new(),
            trouble: None,
            area: None,
            scrubbing: false,
            focus: cx.focus_handle(),
        };
        this.fetch_frames(0., cx);
        this
    }

    pub(crate) fn frames(&self) -> &Entity<PictureCompare> {
        &self.frames
    }

    pub(crate) fn facts(&self) -> SharedString {
        self.videos.facts()
    }

    pub(crate) fn change_position(&self) -> Option<Position> {
        let count = self.videos.changes.len();
        (count > 0).then(|| Position {
            index: self.current.unwrap_or(0).min(count - 1),
            count,
        })
    }

    pub(crate) fn step_change(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let count = self.videos.changes.len();
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
        self.sync_repeat();
        let from = self.videos.changes[next].from;
        self.seek(from, cx);
    }

    fn sync_repeat(&self) {
        let range = self
            .current
            .filter(|_| self.repeating)
            .and_then(|at| self.videos.changes.get(at))
            .map(|region| (region.from, region.to));
        self.player.repeat(range);
    }

    fn seek(&mut self, seconds: f64, cx: &mut Context<Self>) {
        self.player.seek(seconds);
        if self.player.is_playing() {
            self.start_streams(cx);
        } else {
            self.fetch_frames(seconds, cx);
        }
        cx.notify();
    }

    fn fetch_frames(&mut self, seconds: f64, cx: &mut Context<Self>) {
        if self.fetching.is_running() {
            self.wanted = Some(seconds);
            return;
        }
        let wanted: Vec<(usize, PathBuf, (u32, u32))> = [&self.videos.before, &self.videos.after]
            .into_iter()
            .enumerate()
            .filter_map(|(ix, side)| side.as_ref().map(|side| (ix, side.file.clone(), side.facts.frame_size(WIDEST_FRAME))))
            .collect();
        let task = run_in_background(
            cx,
            move || {
                wanted
                    .into_iter()
                    .map(|(ix, file, size)| {
                        let shown = (seconds.max(0.) - 0.001).max(0.);
                        (ix, frame_at(&file, shown, size).ok())
                    })
                    .collect::<Vec<_>>()
            },
            move |this, frames, cx| {
                this.fetching.finish();
                for (ix, frame) in frames {
                    this.shown[ix] = frame.map(Arc::new);
                }
                this.show_frames(cx);
                if let Some(next) = this.wanted.take().filter(|next| (next - seconds).abs() > f64::EPSILON) {
                    this.fetch_frames(next, cx);
                }
            },
        );
        self.fetching.run(task);
    }

    fn show_frames(&mut self, cx: &mut Context<Self>) {
        let pictures = Pictures::shared(self.shown[0].clone(), self.shown[1].clone());
        let sources = self.sources.clone();
        self.frames.update(cx, |frames, cx| frames.replace(pictures, sources, cx));
        cx.notify();
    }

    fn start_streams(&mut self, cx: &mut Context<Self>) {
        self.streams.clear();
        let from = self.player.position();
        for (ix, side) in [&self.videos.before, &self.videos.after].into_iter().enumerate() {
            let Some(side) = side else {
                continue;
            };
            let rate = side.facts.frame_rate.unwrap_or(MOST_FRAMES_PER_SECOND).min(MOST_FRAMES_PER_SECOND);
            match FrameStream::start(&side.file, from, side.facts.frame_size(WIDEST_FRAME), rate) {
                Ok(stream) => self.streams.push((ix, stream)),
                Err(error) => self.trouble = Some(format!("{error:#}").into()),
            }
        }
        cx.notify();
    }

    fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.player.is_playing() {
            self.player.pause();
            self.streams.clear();
            self.fetch_frames(self.player.position(), cx);
        } else {
            match self.player.toggle() {
                Ok(()) => self.start_streams(cx),
                Err(error) => self.trouble = Some(format!("{error:#}").into()),
            }
        }
        cx.notify();
    }

    fn step_frame(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if self.player.is_playing() {
            self.toggle_play(cx);
        }
        let step = self.videos.frame_step();
        let at = match direction {
            Direction::Back => self.player.position() - step,
            Direction::Forward => self.player.position() + step,
        };
        self.seek(at.clamp(0., self.videos.length()), cx);
    }

    fn switch_version(&mut self, cx: &mut Context<Self>) {
        self.player.hear(1 - self.player.heard());
        cx.notify();
    }

    fn hear(&mut self, version: Version, cx: &mut Context<Self>) {
        self.player.hear(usize::from(version == Version::After));
        cx.notify();
    }

    fn toggle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeating = !self.repeating;
        if self.repeating && self.current.is_none() && !self.videos.changes.is_empty() {
            self.current = Some(0);
        }
        self.sync_repeat();
        cx.notify();
    }

    fn heard(&self) -> Option<Version> {
        let heard = if self.player.heard() == 0 { Version::Before } else { Version::After };
        self.videos.side(heard).and_then(|side| side.sound.as_ref()).map(|_| heard)
    }

    fn pull_stream_frames(&mut self, cx: &mut Context<Self>) {
        let clock = self.player.position();
        let mut changed = false;
        for (ix, stream) in &mut self.streams {
            if let Some(frame) = stream.latest_until(clock) {
                self.shown[*ix] = Some(Arc::new(frame.picture));
                changed = true;
            }
        }
        if changed {
            self.show_frames(cx);
        }
    }

    fn tracks(&self, area: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds {
            origin: point(area.origin.x + px(LABELS), area.origin.y),
            size: size(area.size.width - px(LABELS), area.size.height),
        }
    }

    fn x_of(&self, tracks: Bounds<Pixels>, seconds: f64) -> Pixels {
        let share = seconds / self.videos.length().max(f64::EPSILON);
        tracks.origin.x + tracks.size.width * share as f32
    }

    fn lane_top(&self, area: Bounds<Pixels>, version: Version) -> Pixels {
        let index = usize::from(version == Version::After) as f32;
        area.origin.y + px(NUMBERS) + px(LANE + LANE_GAP) * index
    }

    fn paint_list(&mut self, area: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Vec<PaintQuad> {
        if self.area != Some(area) {
            self.area = Some(area);
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        let theme = cx.theme();
        let tracks = self.tracks(area);
        let mut quads = Vec::new();
        for version in [Version::Before, Version::After] {
            let Some(side) = self.videos.side(version) else {
                continue;
            };
            let top = self.lane_top(area, version);
            let end = self.x_of(tracks, side.facts.duration);
            quads.push(
                fill(
                    Bounds {
                        origin: point(tracks.origin.x, top),
                        size: size(end - tracks.origin.x, px(LANE)),
                    },
                    theme.secondary,
                )
                .corner_radii(Corners::all(px(3.))),
            );
            let mut x = end + px(2.);
            while x < tracks.origin.x + tracks.size.width {
                quads.push(fill(
                    Bounds {
                        origin: point(x, top),
                        size: size(px(2.), px(LANE)),
                    },
                    theme.foreground.opacity(0.08),
                ));
                x += px(6.);
            }
            for (ix, change) in self.videos.changes.iter().enumerate() {
                if change.from >= side.facts.duration {
                    continue;
                }
                let left = self.x_of(tracks, change.from);
                let right = self.x_of(tracks, change.to.min(side.facts.duration));
                quads.push(
                    fill(
                        Bounds {
                            origin: point(left, top),
                            size: size((right - left).max(px(2.)), px(LANE)),
                        },
                        theme.yellow.opacity(if self.current == Some(ix) { 0.8 } else { 0.38 }),
                    )
                    .corner_radii(Corners::all(px(3.))),
                );
            }
        }
        let playhead = self.x_of(tracks, self.player.position());
        quads.push(fill(
            Bounds {
                origin: point(playhead - px(1.), area.origin.y + px(NUMBERS - 2.)),
                size: size(px(2.), px(LANE * 2. + LANE_GAP + 4.)),
            },
            theme.foreground,
        ));
        quads
    }

    fn overlays(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(area) = self.area else {
            return Vec::new();
        };
        let theme = cx.theme();
        let tracks = self.tracks(area);
        let mut overlays = Vec::new();
        for version in [Version::Before, Version::After] {
            let Some(side) = self.videos.side(version) else {
                continue;
            };
            overlays.push(
                h_flex()
                    .absolute()
                    .left_0()
                    .top(self.lane_top(area, version) - area.origin.y - px(2.))
                    .gap_1p5()
                    .xs_text()
                    .child(version_dot(version, cx))
                    .child(version.label())
                    .child(
                        div()
                            .font_family(theme.mono_font_family.clone())
                            .text_color(theme.muted_foreground)
                            .child(clock(side.facts.duration, 2)),
                    )
                    .into_any_element(),
            );
        }
        for (ix, change) in self.videos.changes.iter().enumerate() {
            let middle = self.x_of(tracks, (change.from + change.to) / 2.);
            let current = self.current == Some(ix);
            overlays.push(
                div()
                    .absolute()
                    .left(middle - area.origin.x - px(BADGE / 2.))
                    .top(px(2.))
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
        let length = self.videos.length();
        let width = f64::from(f32::from(tracks.size.width));
        let step = TICK_STEPS
            .into_iter()
            .find(|step| step / length.max(f64::EPSILON) * width >= TICK_GAP)
            .unwrap_or(600.);
        let mut tick = 0.;
        let last_label = tracks.origin.x + tracks.size.width - px(28.);
        while tick <= length && self.x_of(tracks, tick) <= last_label {
            overlays.push(
                div()
                    .absolute()
                    .left(self.x_of(tracks, tick) - area.origin.x)
                    .top(px(NUMBERS) + px(LANE * 2. + LANE_GAP * 2.))
                    .text_size(px(11.))
                    .whitespace_nowrap()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(theme.muted_foreground)
                    .child(clock(tick, 0))
                    .into_any_element(),
            );
            tick += step;
        }
        overlays
    }

    fn time_at(&self, x: Pixels) -> Option<f64> {
        let tracks = self.tracks(self.area?);
        let share = f64::from((x - tracks.origin.x) / tracks.size.width).clamp(0., 1.);
        Some(share * self.videos.length())
    }

    fn on_press(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        if let Some(at) = self.time_at(event.position.x) {
            self.scrubbing = true;
            self.current = self
                .videos
                .changes
                .iter()
                .position(|change| (change.from..change.to).contains(&at))
                .or(self.current);
            self.sync_repeat();
            self.seek(at, cx);
        }
    }

    fn on_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.scrubbing && event.pressed_button == Some(MouseButton::Left) {
            if let Some(at) = self.time_at(event.position.x) {
                self.seek(at, cx);
            }
        } else {
            self.scrubbing = false;
        }
    }

    fn details(&self) -> MediaDetails {
        let values = |facts: &VideoFacts| -> Vec<SharedString> {
            vec![
                format!("{} × {}", facts.width, facts.height).into(),
                facts
                    .frame_rate
                    .map_or_else(|| "—".to_owned(), |rate| format!("{} fps", (rate * 100.).round() / 100.))
                    .into(),
                clock(facts.duration, 2).into(),
                facts.video_codec.clone().into(),
                facts.audio.clone().unwrap_or_else(|| "None".to_owned()).into(),
                facts
                    .bit_rate
                    .map_or_else(|| "—".to_owned(), |rate| format!("{:.1} Mb/s", rate as f64 / 1_000_000.))
                    .into(),
                file_size(facts.bytes as usize).into(),
            ]
        };
        let mut details = MediaDetails::new(DETAILS.to_vec());
        for version in [Version::Before, Version::After] {
            if let Some(side) = self.videos.side(version) {
                details = details.row(DetailsRow {
                    version,
                    source: self.sources[usize::from(version == Version::After)].clone(),
                    values: values(&side.facts),
                });
            }
        }
        let deltas = match (&self.videos.before, &self.videos.after) {
            (Some(before), Some(after)) => vec![
                None,
                None,
                Some(format!("{:+.2} s", after.facts.duration - before.facts.duration).replace('-', "−").into()),
                None,
                None,
                None,
                Some(signed_size(after.facts.bytes as i64 - before.facts.bytes as i64).into()),
            ],
            _ => Vec::new(),
        };
        details.deltas(deltas)
    }
}

impl Render for VideoCompare {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playing = self.player.is_playing();
        if playing {
            self.pull_stream_frames(cx);
            window.request_animation_frame();
        } else if !self.streams.is_empty() {
            self.streams.clear();
            self.fetch_frames(self.player.position(), cx);
        }
        let entity = cx.entity();
        let timeline = canvas(
            move |bounds, window, cx| entity.update(cx, |this, cx| this.paint_list(bounds, window, cx)),
            |_, quads, window, _| {
                for quad in quads {
                    window.paint_quad(quad);
                }
            },
        )
        .absolute()
        .size_full();
        let transport = Transport::new(
            self.player.position(),
            self.videos.length(),
            playing,
            self.repeating,
            self.heard(),
            handler(cx.listener(|this, _: &(), _, cx| this.toggle_play(cx))),
            handler(cx.listener(|this, _: &(), _, cx| this.toggle_repeat(cx))),
            handler(cx.listener(|this, version: &Version, _, cx| this.hear(*version, cx))),
        )
        .frame_steps(handler(cx.listener(|this, direction: &Direction, _, cx| this.step_frame(*direction, cx))))
        .trouble(self.trouble.clone());
        let overlays = self.overlays(cx);
        v_flex()
            .size_full()
            .key_context(MEDIA_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &PlayPause, _, cx| this.toggle_play(cx)))
            .on_action(cx.listener(|this, _: &SwitchVersion, _, cx| this.switch_version(cx)))
            .on_action(cx.listener(|this, _: &LoopChange, _, cx| this.toggle_repeat(cx)))
            .child(div().flex_1().min_h_0().child(self.frames.clone()))
            .child(
                div()
                    .flex_none()
                    .px_4()
                    .pt_1()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .id("video-timeline")
                            .relative()
                            .w_full()
                            .h(px(NUMBERS + LANE * 2. + LANE_GAP * 2. + TICKS))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| this.on_press(event, window, cx)))
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| this.on_move(event, cx)))
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.scrubbing = false))
                            .child(timeline)
                            .children(overlays),
                    ),
            )
            .child(transport)
            .child(self.details())
    }
}

