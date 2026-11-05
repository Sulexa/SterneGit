use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, Bounds, Context, Corners, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement as _, Pixels, Render,
    RenderImage, ScrollEvent, SharedString, Styled as _, Window,
    assets::IconName,
    canvas,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    point, px, size,
};

use super::{
    Direction, Position,
    details::{DetailsRow, MediaDetails, Version, VersionTag, version_dot},
};
use crate::{
    media::{Area, Backdrop, Layer, Picture, PixelDifference, Placement, Surface, rasterize},
    ui::{Look as _, thousands},
};

const FIT_SHARE: f64 = 0.9;
const MOST_FIT: f64 = 32.;
const MOST_ZOOM: f64 = 64.;
const LEAST_ZOOM: f64 = 0.02;
const ZOOM_STEP: f64 = 1.25;
const AREA_ROOM: f64 = 3.;
const CHECKER: f32 = 8.;
const TAG_INSET: f32 = 10.;
const HANDLE_REACH: f32 = 18.;
const KNOB: f32 = 30.;
const BOX_MARGIN: f32 = 4.;
const BADGE: f32 = 18.;
const LINE_HEIGHT_FOR_WHEEL: f32 = 20.;
const DETAILS: [&str; 4] = ["Size", "Color", "Format", "File size"];
const MIX_TRACK: f32 = 180.;
const MIX_LABEL: f32 = 64.;
const MIX_BOTTOM: f32 = 14.;
const MIX_PILL: f32 = 28.;
const MIX_THUMB: f32 = 16.;
const MIX_REACH: f32 = 10.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PictureMode {
    Split,
    Swipe,
    Onion,
    Difference,
}

impl PictureMode {
    pub(crate) const ALL: [PictureMode; 4] = [
        PictureMode::Split,
        PictureMode::Swipe,
        PictureMode::Onion,
        PictureMode::Difference,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            PictureMode::Split => "picture-split",
            PictureMode::Swipe => "picture-swipe",
            PictureMode::Onion => "picture-onion",
            PictureMode::Difference => "picture-difference",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            PictureMode::Split => "Split",
            PictureMode::Swipe => "Swipe",
            PictureMode::Onion => "Onion skin",
            PictureMode::Difference => "Difference",
        }
    }

    pub(crate) fn icon(self) -> IconName {
        match self {
            PictureMode::Split => IconName::Columns2,
            PictureMode::Swipe => IconName::SquareSplitHorizontal,
            PictureMode::Onion => IconName::Blend,
            PictureMode::Difference => IconName::Diff,
        }
    }

    pub(crate) fn tooltip(self) -> &'static str {
        match self {
            PictureMode::Split => "Show both versions side by side",
            PictureMode::Swipe => "Drag a line between the two versions",
            PictureMode::Onion => "Fade one version into the other",
            PictureMode::Difference => "Show only the pixels that changed",
        }
    }
}

pub(crate) struct Pictures {
    pub(crate) before: Option<Arc<Picture>>,
    pub(crate) after: Option<Arc<Picture>>,
    pub(crate) difference: Option<Arc<PixelDifference>>,
}

impl Pictures {
    pub(crate) fn between(before: Option<Picture>, after: Option<Picture>) -> Self {
        Self::shared(before.map(Arc::new), after.map(Arc::new))
    }

    pub(crate) fn shared(before: Option<Arc<Picture>>, after: Option<Arc<Picture>>) -> Self {
        let difference = match (&before, &after) {
            (Some(before), Some(after)) => Some(Arc::new(PixelDifference::between(before, after))),
            _ => None,
        };
        Self {
            before,
            after,
            difference,
        }
    }

    pub(crate) fn facts(&self) -> SharedString {
        let describe = |picture: &Picture| format!("{} × {} {}", picture.width, picture.height, picture.format);
        match (&self.before, &self.after) {
            (Some(before), Some(after)) if (before.width, before.height) != (after.width, after.height) => {
                format!("{} → {}", describe(before), describe(after)).into()
            }
            (_, Some(picture)) | (Some(picture), None) => describe(picture).into(),
            (None, None) => SharedString::default(),
        }
    }

    fn both(&self) -> Option<(&Arc<Picture>, &Arc<Picture>)> {
        self.before.as_ref().zip(self.after.as_ref())
    }

    fn reference(&self) -> Option<&Arc<Picture>> {
        self.after.as_ref().or(self.before.as_ref())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Zoom {
    Fit,
    Scale(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Pan {
        from: gpui_kit::Point<Pixels>,
        focus: (f64, f64),
    },
    Swipe,
    Mix,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Shown {
    pane: Bounds<Pixels>,
    image: Bounds<Pixels>,
    scale: f64,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RasterKey {
    pane: usize,
    mode: PictureMode,
    generation: u64,
    placement: (u64, u64, u64, u32, u32),
    surface: Surface,
    mix: u32,
    split: u32,
}

struct Rendered {
    key: RasterKey,
    image: Arc<RenderImage>,
}

pub(crate) struct PictureCompare {
    pictures: Pictures,
    shows_details: bool,
    generation: u64,
    sources: [SharedString; 2],
    mode: PictureMode,
    zoom: Zoom,
    focus: (f64, f64),
    swipe: f64,
    mix: f64,
    backdrop: Backdrop,
    current_area: Option<usize>,
    probe: Option<(u32, u32)>,
    drag: Option<Drag>,
    viewport: Option<Bounds<Pixels>>,
    rendered: Vec<Rendered>,
}

impl PictureCompare {
    pub(crate) fn new(pictures: Pictures, sources: [SharedString; 2]) -> Self {
        Self {
            pictures,
            shows_details: true,
            generation: 0,
            sources,
            mode: PictureMode::Split,
            zoom: Zoom::Fit,
            focus: (0.5, 0.5),
            swipe: 0.5,
            mix: 0.5,
            backdrop: Backdrop::default(),
            current_area: None,
            probe: None,
            drag: None,
            viewport: None,
            rendered: Vec::new(),
        }
    }

    pub(crate) fn without_details(mut self) -> Self {
        self.shows_details = false;
        self
    }

    pub(crate) fn replace(&mut self, pictures: Pictures, sources: [SharedString; 2], cx: &mut Context<Self>) {
        self.pictures = pictures;
        self.sources = sources;
        self.generation += 1;
        self.probe = None;
        if !self.compares_both() {
            self.mode = PictureMode::Split;
        }
        let areas = self.pictures.difference.as_ref().map_or(0, |difference| difference.areas.len());
        self.current_area = self.current_area.filter(|at| *at < areas);
        cx.notify();
    }

    pub(crate) fn facts(&self) -> SharedString {
        self.pictures.facts()
    }

    pub(crate) fn mode(&self) -> PictureMode {
        self.mode
    }

    pub(crate) fn compares_both(&self) -> bool {
        self.pictures.both().is_some()
    }

    pub(crate) fn backdrop(&self) -> Backdrop {
        self.backdrop
    }

    pub(crate) fn fits(&self) -> bool {
        self.zoom == Zoom::Fit
    }

    pub(crate) fn set_mode(&mut self, mode: PictureMode, cx: &mut Context<Self>) {
        if self.mode != mode && self.compares_both() {
            self.mode = mode;
            self.probe = None;
            self.drag = None;
            cx.notify();
        }
    }

    pub(crate) fn set_backdrop(&mut self, backdrop: Backdrop, cx: &mut Context<Self>) {
        self.backdrop = backdrop;
        cx.notify();
    }

    pub(crate) fn zoom_percent(&self) -> Option<f64> {
        self.frames().first().map(|shown| shown.scale * 100.)
    }

    pub(crate) fn fit(&mut self, cx: &mut Context<Self>) {
        self.zoom = Zoom::Fit;
        self.focus = (0.5, 0.5);
        cx.notify();
    }

    pub(crate) fn zoom_by(&mut self, factor: f64, cx: &mut Context<Self>) {
        if let Some(scale) = self.frames().first().map(|shown| shown.scale) {
            self.zoom = Zoom::Scale((scale * factor).clamp(LEAST_ZOOM, MOST_ZOOM));
            cx.notify();
        }
    }

    pub(crate) fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom_by(ZOOM_STEP, cx);
    }

    pub(crate) fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom_by(1. / ZOOM_STEP, cx);
    }

    pub(crate) fn change_position(&self) -> Option<Position> {
        let count = self.pictures.difference.as_ref()?.areas.len();
        (count > 0).then(|| Position {
            index: self.current_area.unwrap_or(0).min(count - 1),
            count,
        })
    }

    pub(crate) fn step_change(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let Some(count) = self.change_position().map(|position| position.count) else {
            return;
        };
        let next = match (self.current_area, direction) {
            (None, Direction::Forward) => 0,
            (None, Direction::Back) => count - 1,
            (Some(at), Direction::Forward) => (at + 1) % count,
            (Some(at), Direction::Back) => (at + count - 1) % count,
        };
        self.current_area = Some(next);
        self.zoom_onto_area(next);
        cx.notify();
    }

    pub(crate) fn forget_images(&mut self, cx: &mut App) {
        let images: Vec<Arc<RenderImage>> = self.rendered.drain(..).map(|rendered| rendered.image).collect();
        if images.is_empty() {
            return;
        }
        cx.spawn(async move |cx| {
            cx.update(|cx| {
                for image in images {
                    cx.drop_image(image, None);
                }
            })
        })
        .detach();
    }

    fn zoom_onto_area(&mut self, at: usize) {
        let (Some(area), Some(reference), Some(viewport)) = (
            self.pictures.difference.as_ref().and_then(|difference| difference.areas.get(at).copied()),
            self.pictures.reference().cloned(),
            self.viewport,
        ) else {
            return;
        };
        let panes = self.panes(viewport);
        let Some(pane) = panes.first() else {
            return;
        };
        let room = |length: Pixels, area: u32| f64::from(f32::from(length)) / (f64::from(area.max(1)) * AREA_ROOM);
        let fit = self.fit_scale(&panes);
        let scale = room(pane.size.width, area.width)
            .min(room(pane.size.height, area.height))
            .clamp(fit.min(MOST_FIT), MOST_FIT);
        self.zoom = Zoom::Scale(scale);
        self.focus = (
            (f64::from(area.x) + f64::from(area.width) / 2.) / f64::from(reference.width),
            (f64::from(area.y) + f64::from(area.height) / 2.) / f64::from(reference.height),
        );
    }

    fn shown_pictures(&self) -> Vec<Arc<Picture>> {
        match (self.mode, self.pictures.both()) {
            (PictureMode::Split, Some((before, after))) => vec![before.clone(), after.clone()],
            _ => self.pictures.reference().cloned().into_iter().collect(),
        }
    }

    fn panes(&self, viewport: Bounds<Pixels>) -> Vec<Bounds<Pixels>> {
        let count = self.shown_pictures().len().max(1);
        let width = viewport.size.width / count as f32;
        (0..count)
            .map(|ix| Bounds {
                origin: point(viewport.origin.x + width * ix as f32, viewport.origin.y),
                size: size(width, viewport.size.height),
            })
            .collect()
    }

    fn fit_scale(&self, panes: &[Bounds<Pixels>]) -> f64 {
        self.shown_pictures()
            .iter()
            .zip(panes)
            .map(|(picture, pane)| {
                let across = f64::from(f32::from(pane.size.width)) / f64::from(picture.width.max(1));
                let down = f64::from(f32::from(pane.size.height)) / f64::from(picture.height.max(1));
                across.min(down) * FIT_SHARE
            })
            .fold(MOST_FIT, f64::min)
    }

    fn frames(&self) -> Vec<Shown> {
        let Some(viewport) = self.viewport else {
            return Vec::new();
        };
        let panes = self.panes(viewport);
        let scale = match self.zoom {
            Zoom::Fit => self.fit_scale(&panes),
            Zoom::Scale(scale) => scale,
        };
        self.shown_pictures()
            .iter()
            .zip(panes)
            .map(|(picture, pane)| {
                let width = f64::from(picture.width) * scale;
                let height = f64::from(picture.height) * scale;
                let center = pane.center();
                let left = f64::from(f32::from(center.x)) - self.focus.0 * width;
                let top = f64::from(f32::from(center.y)) - self.focus.1 * height;
                Shown {
                    pane,
                    image: Bounds {
                        origin: point(px(left as f32), px(top as f32)),
                        size: size(px(width as f32), px(height as f32)),
                    },
                    scale,
                    width: picture.width,
                    height: picture.height,
                }
            })
            .collect()
    }

    fn prepare(&mut self, viewport: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Vec<(Bounds<Pixels>, Arc<RenderImage>)> {
        if self.viewport != Some(viewport) {
            self.viewport = Some(viewport);
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        let device = f64::from(window.scale_factor());
        let colors = |color: Hsla| {
            let rgb = color.to_rgb();
            [rgb.r, rgb.g, rgb.b].map(|channel| (channel.clamp(0., 1.) * 255.).round() as u8)
        };
        let theme = cx.theme();
        let surface = Surface {
            backdrop: self.backdrop,
            panel: colors(theme.background),
            checker: (CHECKER * window.scale_factor()).round() as u32,
        };
        let change_colors = [colors(theme.green), colors(theme.yellow), colors(theme.red)];
        let mix = self.mix as f32;
        let mut kept = Vec::new();
        let mut painted = Vec::new();
        let pictures = self.shown_pictures();
        for (ix, shown) in self.frames().into_iter().enumerate() {
            let visible = shown.image.intersect(&shown.pane);
            if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
                continue;
            }
            let device_of = |value: Pixels| f64::from(f32::from(value)) * device;
            let left = device_of(visible.origin.x).floor();
            let top = device_of(visible.origin.y).floor();
            let right = device_of(visible.origin.x + visible.size.width).ceil();
            let bottom = device_of(visible.origin.y + visible.size.height).ceil();
            let placement = Placement {
                left: device_of(shown.image.origin.x) - left,
                top: device_of(shown.image.origin.y) - top,
                scale: shown.scale * device,
                width: (right - left) as u32,
                height: (bottom - top) as u32,
            };
            if placement.width == 0 || placement.height == 0 {
                continue;
            }
            let split = ((device_of(shown.image.origin.x) + self.swipe * shown.image.size.width.as_f32() as f64 * device) - left)
                .clamp(0., f64::from(placement.width)) as u32;
            let key = RasterKey {
                pane: ix,
                mode: self.mode,
                generation: self.generation,
                placement: (
                    placement.left.to_bits(),
                    placement.top.to_bits(),
                    placement.scale.to_bits(),
                    placement.width,
                    placement.height,
                ),
                surface,
                mix: if self.mode == PictureMode::Onion { mix.to_bits() } else { 0 },
                split: if self.mode == PictureMode::Swipe { split } else { 0 },
            };
            let target = Bounds::from_corners(
                point(px((left / device) as f32), px((top / device) as f32)),
                point(px((right / device) as f32), px((bottom / device) as f32)),
            );
            if let Some(at) = self.rendered.iter().position(|rendered| rendered.key == key) {
                let rendered = self.rendered.remove(at);
                painted.push((target, rendered.image.clone()));
                kept.push(rendered);
                continue;
            }
            let picture = pictures.get(ix).or(pictures.first());
            let Some(picture) = picture else {
                continue;
            };
            let layer = match (self.mode, self.pictures.both(), self.pictures.difference.as_ref()) {
                (PictureMode::Swipe, Some((before, after)), _) => Layer::Swipe { before, after, split },
                (PictureMode::Onion, Some((before, after)), _) => Layer::Onion { before, after, mix },
                (PictureMode::Difference, Some((_, after)), Some(difference)) => Layer::Difference {
                    after,
                    difference,
                    colors: change_colors,
                },
                _ => Layer::Picture(picture),
            };
            let bgra = rasterize(&layer, placement, surface);
            let Some(buffer) = image::RgbaImage::from_raw(placement.width, placement.height, bgra) else {
                continue;
            };
            let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
            painted.push((target, image.clone()));
            kept.push(Rendered { key, image });
        }
        for stale in std::mem::replace(&mut self.rendered, kept) {
            let _ = window.drop_image(stale.image);
        }
        painted
    }

    fn shown_at(&self, position: gpui_kit::Point<Pixels>) -> Option<Shown> {
        self.frames().into_iter().find(|shown| shown.pane.contains(&position))
    }

    fn on_wheel(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let Some(shown) = self.shown_at(event.position) else {
            return;
        };
        let delta = event.delta.pixel_delta(px(LINE_HEIGHT_FOR_WHEEL));
        if event.modifiers.secondary(cx) {
            let factor = if f32::from(delta.y) > 0. { ZOOM_STEP } else { 1. / ZOOM_STEP };
            let scale = (shown.scale * factor).clamp(LEAST_ZOOM, MOST_ZOOM);
            let center = shown.pane.center();
            let offset = (
                f64::from(f32::from(event.position.x - center.x)),
                f64::from(f32::from(event.position.y - center.y)),
            );
            let width = f64::from(shown.width);
            let height = f64::from(shown.height);
            let under = (
                self.focus.0 + offset.0 / (width * shown.scale),
                self.focus.1 + offset.1 / (height * shown.scale),
            );
            self.focus = (
                under.0 - offset.0 / (width * scale),
                under.1 - offset.1 / (height * scale),
            );
            self.zoom = Zoom::Scale(scale);
        } else {
            let (dx, dy) = if event.modifiers.shift {
                (f32::from(delta.y), 0.)
            } else {
                (f32::from(delta.x), f32::from(delta.y))
            };
            self.pan_by(&shown, f64::from(dx), f64::from(dy));
        }
        cx.notify();
    }

    fn pan_by(&mut self, shown: &Shown, dx: f64, dy: f64) {
        if self.zoom == Zoom::Fit {
            self.zoom = Zoom::Scale(shown.scale);
        }
        self.focus = (
            (self.focus.0 - dx / (f64::from(shown.width) * shown.scale)).clamp(0., 1.),
            (self.focus.1 - dy / (f64::from(shown.height) * shown.scale)).clamp(0., 1.),
        );
    }

    fn mix_track(&self) -> Option<Bounds<Pixels>> {
        let viewport = self.viewport?;
        let bottom = viewport.origin.y + viewport.size.height - px(MIX_BOTTOM);
        Some(Bounds {
            origin: point(viewport.center().x - px(MIX_TRACK / 2.), bottom - px(MIX_PILL)),
            size: size(px(MIX_TRACK), px(MIX_PILL)),
        })
    }

    fn mix_to(&mut self, position: gpui_kit::Point<Pixels>) -> bool {
        let Some(track) = self.mix_track() else {
            return false;
        };
        self.mix = f64::from((position.x - track.origin.x) / track.size.width).clamp(0., 1.);
        true
    }

    fn on_press(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let on_mix = self.mode == PictureMode::Onion
            && self.compares_both()
            && self.mix_track().is_some_and(|track| track.dilate(px(MIX_REACH)).contains(&event.position));
        if on_mix && self.mix_to(event.position) {
            self.drag = Some(Drag::Mix);
            cx.notify();
            return;
        }
        let Some(shown) = self.shown_at(event.position) else {
            return;
        };
        if self.mode == PictureMode::Swipe && self.compares_both() {
            self.swipe_to(&shown, event.position);
            self.drag = Some(Drag::Swipe);
        } else {
            self.drag = Some(Drag::Pan {
                from: event.position,
                focus: self.focus,
            });
        }
        cx.notify();
    }

    fn swipe_to(&mut self, shown: &Shown, position: gpui_kit::Point<Pixels>) {
        let across = (position.x - shown.image.origin.x) / shown.image.size.width;
        self.swipe = f64::from(across).clamp(0., 1.);
    }

    fn on_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.drag == Some(Drag::Mix) && event.pressed_button == Some(MouseButton::Left) {
            self.mix_to(event.position);
            cx.notify();
            return;
        }
        let Some(shown) = self.shown_at(event.position) else {
            return;
        };
        match self.drag.filter(|_| event.pressed_button == Some(MouseButton::Left)) {
            Some(Drag::Pan { from, focus }) => {
                self.focus = focus;
                let moved = event.position - from;
                self.pan_by(&shown, f64::from(f32::from(moved.x)), f64::from(f32::from(moved.y)));
                cx.notify();
            }
            Some(Drag::Swipe) => {
                self.swipe_to(&shown, event.position);
                cx.notify();
            }
            Some(Drag::Mix) => {}
            None if self.mode == PictureMode::Difference => {
                let probe = pixel_under(&shown, event.position);
                if probe != self.probe {
                    self.probe = probe;
                    cx.notify();
                }
            }
            None => {}
        }
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        if self.drag.take().is_some() {
            cx.notify();
        }
    }

    fn overlays(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(viewport) = self.viewport else {
            return Vec::new();
        };
        let frames = self.frames();
        let relative = |at: gpui_kit::Point<Pixels>| at - viewport.origin;
        let mut overlays = Vec::new();
        let tag = |version: Version, source: &SharedString| VersionTag::new(version, source.clone());
        match (self.mode, self.pictures.both()) {
            (PictureMode::Split, Some(_)) => {
                for (shown, (version, source)) in frames.iter().zip([
                    (Version::Before, &self.sources[0]),
                    (Version::After, &self.sources[1]),
                ]) {
                    let corner = relative(shown.pane.origin);
                    overlays.push(
                        div()
                            .absolute()
                            .left(corner.x + px(TAG_INSET))
                            .top(corner.y + px(TAG_INSET))
                            .child(tag(version, source))
                            .into_any_element(),
                    );
                }
            }
            (_, Some(_)) => {
                overlays.push(
                    div()
                        .absolute()
                        .left(px(TAG_INSET))
                        .top(px(TAG_INSET))
                        .child(tag(Version::Before, &self.sources[0]))
                        .into_any_element(),
                );
                overlays.push(
                    div()
                        .absolute()
                        .right(px(TAG_INSET))
                        .top(px(TAG_INSET))
                        .child(tag(Version::After, &self.sources[1]))
                        .into_any_element(),
                );
            }
            (_, None) => {
                let version = if self.pictures.after.is_some() { Version::After } else { Version::Before };
                let source = &self.sources[usize::from(version == Version::After)];
                overlays.push(
                    div()
                        .absolute()
                        .left(px(TAG_INSET))
                        .top(px(TAG_INSET))
                        .child(tag(version, source))
                        .into_any_element(),
                );
            }
        }
        let Some(shown) = frames.first().copied() else {
            return overlays;
        };
        let image = Bounds {
            origin: relative(shown.image.origin),
            size: shown.image.size,
        };
        match self.mode {
            PictureMode::Swipe if self.compares_both() => overlays.push(self.swipe_handle(image, cx)),
            PictureMode::Onion if self.compares_both() => overlays.push(self.onion_slider(cx)),
            PictureMode::Difference if self.compares_both() => {
                overlays.extend(self.change_boxes(image, shown.scale, cx));
                overlays.extend(self.legend(cx));
                overlays.extend(self.probe_tip(image, shown.scale, cx));
            }
            _ => {}
        }
        overlays
    }

    fn swipe_handle(&self, image: Bounds<Pixels>, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let x = image.origin.x + image.size.width * self.swipe as f32;
        div()
            .absolute()
            .left(x - px(1.))
            .top(image.origin.y - px(HANDLE_REACH))
            .w(px(2.))
            .h(image.size.height + px(HANDLE_REACH * 2.))
            .bg(theme.foreground)
            .child(
                div()
                    .absolute()
                    .left(px(1. - KNOB / 2.))
                    .top((image.size.height + px(HANDLE_REACH * 2.)) / 2. - px(KNOB / 2.))
                    .size(px(KNOB))
                    .rounded_full()
                    .bg(theme.foreground)
                    .text_color(theme.background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::MoveHorizontal).small()),
            )
            .into_any_element()
    }

    fn onion_slider(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        h_flex()
            .absolute()
            .bottom(px(14.))
            .left_0()
            .right_0()
            .justify_center()
            .child(
                h_flex()
                    .gap_3()
                    .px_3()
                    .py_1p5()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .xs_text()
                    .child(
                        h_flex()
                            .w(px(MIX_LABEL))
                            .justify_end()
                            .gap_1p5()
                            .child(version_dot(Version::Before, cx))
                            .child("Before"),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(MIX_TRACK))
                            .h(px(MIX_THUMB))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .right_0()
                                    .top(px((MIX_THUMB - 6.) / 2.))
                                    .h(px(6.))
                                    .rounded(px(3.))
                                    .bg(theme.foreground.opacity(0.2)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px((MIX_THUMB - 6.) / 2.))
                                    .w(px(MIX_TRACK * self.mix as f32))
                                    .h(px(6.))
                                    .rounded(px(3.))
                                    .bg(theme.foreground),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left(px(MIX_TRACK * self.mix as f32 - MIX_THUMB / 2.))
                                    .size(px(MIX_THUMB))
                                    .rounded_full()
                                    .bg(theme.foreground)
                                    .border_3()
                                    .border_color(theme.foreground.opacity(0.5)),
                            ),
                    )
                    .child(
                        h_flex()
                            .w(px(MIX_LABEL))
                            .gap_1p5()
                            .child("After")
                            .child(version_dot(Version::After, cx)),
                    ),
            )
            .into_any_element()
    }

    fn change_boxes(&self, image: Bounds<Pixels>, scale: f64, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(difference) = &self.pictures.difference else {
            return Vec::new();
        };
        let theme = cx.theme();
        let current = self.current_area;
        difference
            .areas
            .iter()
            .enumerate()
            .map(|(ix, area)| {
                let bounds = area_bounds(image, scale, *area);
                let is_current = current == Some(ix);
                div()
                    .absolute()
                    .left(bounds.origin.x - px(BOX_MARGIN))
                    .top(bounds.origin.y - px(BOX_MARGIN))
                    .w(bounds.size.width + px(BOX_MARGIN * 2.))
                    .h(bounds.size.height + px(BOX_MARGIN * 2.))
                    .rounded(px(3.))
                    .border_color(if is_current { theme.foreground } else { theme.foreground.opacity(0.55) })
                    .map(|this| if is_current { this.border_2() } else { this.border_1().border_dashed() })
                    .child(
                        div()
                            .absolute()
                            .left(px(-BADGE / 2. - 1.))
                            .top(px(-BADGE / 2. - 1.))
                            .size(px(BADGE))
                            .rounded_full()
                            .border_1()
                            .border_color(if is_current { theme.foreground } else { theme.border })
                            .bg(if is_current { theme.foreground } else { theme.secondary })
                            .text_color(if is_current { theme.background } else { theme.foreground })
                            .text_size(px(11.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(format!("{}", ix + 1)),
                    )
                    .into_any_element()
            })
            .collect()
    }

    fn legend(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let difference = self.pictures.difference.as_ref()?;
        let theme = cx.theme();
        let counts = difference.counts;
        let key = |color: Hsla, count: usize, label: &'static str| {
            h_flex()
                .gap_1p5()
                .child(div().size(px(10.)).rounded(px(2.)).bg(color))
                .child(format!("{} {label}", thousands(count)))
        };
        Some(
            h_flex()
                .absolute()
                .left(px(14.))
                .bottom(px(14.))
                .gap_3()
                .px_3()
                .py_1p5()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .xs_text()
                .text_color(theme.muted_foreground)
                .child(key(theme.green, counts.added, "added"))
                .child(key(theme.yellow, counts.changed, "changed"))
                .child(key(theme.red, counts.removed, "removed"))
                .child(format!("{} of the picture", share(difference.share())))
                .into_any_element(),
        )
    }

    fn probe_tip(&self, image: Bounds<Pixels>, scale: f64, cx: &Context<Self>) -> Option<AnyElement> {
        let (x, y) = self.probe?;
        let theme = cx.theme();
        let pixel = area_bounds(image, scale, Area { x, y, width: 1, height: 1 });
        let swatch = |rgba: Option<[u8; 4]>, version: Version| {
            let text: SharedString = match rgba {
                Some([r, g, b, a]) if a == 255 => format!("#{r:02x}{g:02x}{b:02x}").into(),
                Some([r, g, b, a]) => format!("#{r:02x}{g:02x}{b:02x}{a:02x}").into(),
                None => "none".into(),
            };
            let fill = rgba.map(|[r, g, b, a]| gpui_kit::Rgba {
                r: f32::from(r) / 255.,
                g: f32::from(g) / 255.,
                b: f32::from(b) / 255.,
                a: f32::from(a) / 255.,
            });
            h_flex()
                .gap_2()
                .child(
                    div()
                        .size(px(12.))
                        .rounded(px(2.))
                        .border_1()
                        .border_color(theme.border)
                        .map(|this| match fill {
                            Some(fill) => this.bg(fill),
                            None => this,
                        }),
                )
                .child(div().font_family(theme.mono_font_family.clone()).child(text))
                .child(div().text_color(theme.muted_foreground).child(version.label()))
        };
        let before = self.pictures.before.as_ref().map(|picture| {
            let reference = self.pictures.reference().map_or((1, 1), |r| (r.width, r.height));
            let bx = (u64::from(x) * u64::from(picture.width) / u64::from(reference.0.max(1))) as u32;
            let by = (u64::from(y) * u64::from(picture.height) / u64::from(reference.1.max(1))) as u32;
            picture.pixel(bx.min(picture.width - 1), by.min(picture.height - 1))
        });
        let after = self.pictures.after.as_ref().map(|picture| picture.pixel(x, y));
        Some(
            div()
                .absolute()
                .left_0()
                .top_0()
                .size_full()
                .child(
                    div()
                        .absolute()
                        .left(pixel.origin.x)
                        .top(pixel.origin.y)
                        .size(pixel.size.width.max(px(2.)))
                        .border_2()
                        .border_color(theme.foreground),
                )
                .child(
                    v_flex()
                        .absolute()
                        .left(pixel.origin.x + pixel.size.width + px(12.))
                        .top(pixel.origin.y - px(10.))
                        .gap_1()
                        .px_2p5()
                        .py_1p5()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.popover)
                        .xs_text()
                        .child(div().child(format!("Pixel {x}, {y}")))
                        .child(swatch(before, Version::Before))
                        .child(swatch(after, Version::After)),
                )
                .into_any_element(),
        )
    }

    fn details(&self) -> MediaDetails {
        let values = |picture: &Picture| -> Vec<SharedString> {
            vec![
                picture.size().into(),
                picture.color.into(),
                picture.format.into(),
                file_size(picture.bytes).into(),
            ]
        };
        let mut details = MediaDetails::new(DETAILS.to_vec());
        if let Some(before) = &self.pictures.before {
            details = details.row(DetailsRow {
                version: Version::Before,
                source: self.sources[0].clone(),
                values: values(before),
            });
        }
        if let Some(after) = &self.pictures.after {
            details = details.row(DetailsRow {
                version: Version::After,
                source: self.sources[1].clone(),
                values: values(after),
            });
        }
        let delta = self
            .pictures
            .both()
            .map(|(before, after)| signed_size(after.bytes as i64 - before.bytes as i64).into());
        details.deltas(vec![None, None, None, delta])
    }
}

impl Render for PictureCompare {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let pictures = canvas(
            move |bounds, window, cx| entity.update(cx, |this, cx| this.prepare(bounds, window, cx)),
            |_, painted, window, _| {
                for (bounds, image) in painted {
                    let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                }
            },
        )
        .absolute()
        .size_full();
        let divider = (self.mode == PictureMode::Split && self.compares_both()).then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(gpui_kit::relative(0.5))
                .w(px(1.))
                .bg(cx.theme().border)
        });
        let overlays = self.overlays(cx);
        v_flex()
            .size_full()
            .child(
                div()
                    .id("pictures")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| this.on_wheel(event, cx)))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| this.on_press(event, cx)))
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| this.on_move(event, cx)))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.release(cx)))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| this.release(cx)))
                    .child(pictures)
                    .children(divider)
                    .children(overlays),
            )
            .when(self.shows_details, |this| this.child(self.details()))
    }
}

fn pixel_under(shown: &Shown, position: gpui_kit::Point<Pixels>) -> Option<(u32, u32)> {
    let x = f64::from(f32::from(position.x - shown.image.origin.x)) / shown.scale;
    let y = f64::from(f32::from(position.y - shown.image.origin.y)) / shown.scale;
    (x >= 0. && y >= 0. && x < f64::from(shown.width) && y < f64::from(shown.height))
        .then(|| (x as u32, y as u32))
}

fn area_bounds(image: Bounds<Pixels>, scale: f64, area: Area) -> Bounds<Pixels> {
    let at = |value: u32| px((f64::from(value) * scale) as f32);
    Bounds {
        origin: point(image.origin.x + at(area.x), image.origin.y + at(area.y)),
        size: size(at(area.width), at(area.height)),
    }
}

fn share(fraction: f64) -> String {
    let percent = fraction * 100.;
    if percent > 0. && percent < 1. {
        "under 1 %".to_owned()
    } else {
        format!("{} %", percent.round())
    }
}

pub(crate) fn file_size(bytes: usize) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.;
    let mut unit = 0;
    while value >= 1024. && unit + 1 < UNITS.len() {
        value /= 1024.;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub(crate) fn signed_size(delta: i64) -> String {
    let sign = if delta < 0 { "−" } else { "+" };
    format!("{sign}{}", file_size(delta.unsigned_abs() as usize))
}

#[cfg(test)]
mod tests {
    use super::{file_size, share, signed_size};

    #[test]
    fn sizes_read_in_the_largest_unit_that_fits() {
        assert_eq!(file_size(742), "742 B");
        assert_eq!(file_size(2 * 1024 * 1024 + 400 * 1024), "2.4 MB");
        assert_eq!(signed_size(171), "+171 B");
        assert_eq!(signed_size(-2048), "−2.0 KB");
    }

    #[test]
    fn tiny_shares_do_not_round_to_nothing() {
        assert_eq!(share(0.0004), "under 1 %");
        assert_eq!(share(0.09), "9 %");
        assert_eq!(share(0.), "0 %");
    }
}
