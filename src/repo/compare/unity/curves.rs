use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, Bounds, Context, Corners, InteractiveElement as _, IntoElement, ParentElement as _, Pixels,
    RenderImage, SharedString, StatefulInteractiveElement as _, Styled as _, Window, canvas,
    component::{ActiveTheme as _, h_flex, v_flex},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{UnityCompare, cells::state_badge};
use crate::{
    media::{Sketch, Stroke},
    repo::compare::{DetailsRow, MediaDetails, Version, VersionTag},
    ui::Look as _,
    unity::{CurveKey, Keys, State, TrackChange, short_number, value_at},
};

const BACKGROUND: [u8; 3] = [0x0a, 0x0a, 0x0a];
const LINE: [u8; 4] = [0x26, 0x26, 0x26, 255];
const TICK: [u8; 4] = [0x52, 0x52, 0x52, 255];
const RED: [u8; 4] = [0xf8, 0x71, 0x71, 255];
const GREEN: [u8; 4] = [0x4a, 0xde, 0x80, 255];
const WHITE: [u8; 4] = [0xfa, 0xfa, 0xfa, 255];
const NAME_WIDTH: f32 = 196.;
const RULER: f32 = 26.;
const CURVE_LANE: f32 = 76.;
const SPRITE_LANE: f32 = 66.;
const EVENT_LANE: f32 = 52.;
const SIDES: f32 = 36.;
const LANE_PAD: f64 = 8.;
const THUMB: f64 = 26.;
const SMALLEST_FRAME_GAP: f64 = 7.;
const SMALLEST_LABEL_GAP: f64 = 44.;
const LETTER: f32 = 6.5;

fn lane_height(track: &TrackChange) -> f32 {
    match track.after.as_ref().or(track.before.as_ref()) {
        Some(Keys::Objects(_)) => SPRITE_LANE,
        _ => CURVE_LANE,
    }
}

fn frames_shown(compare: &UnityCompare) -> f64 {
    let rate = compare.clip.sample_rate.max(1.);
    let longest = compare.clip.lengths[0].max(compare.clip.lengths[1]);
    let frames = (longest * rate * 1.12).max(10.);
    (frames / 10.).ceil() * 10.
}

fn tick_step(per_frame: f64) -> f64 {
    for step in [1., 2., 5., 10., 20., 50., 100., 200., 500., 1000.] {
        if step * per_frame >= SMALLEST_FRAME_GAP {
            return step;
        }
    }
    2000.
}

fn range(track: &TrackChange) -> (f64, f64) {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for keys in track.before.iter().chain(&track.after) {
        if let Some((a, b)) = keys.range() {
            low = low.min(a);
            high = high.max(b);
        }
    }
    if !low.is_finite() {
        return (-1., 1.);
    }
    if (high - low).abs() < 1e-9 {
        return (low - 1., high + 1.);
    }
    let pad = (high - low) * 0.1;
    (low - pad, high + pad)
}

fn curve_points(keys: &[CurveKey], x_of: impl Fn(f64) -> f64, y_of: impl Fn(f64) -> f64, width: f64, rate: f64) -> Vec<(f32, f32)> {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return Vec::new();
    };
    let start = x_of(first.time).max(0.);
    let end = x_of(last.time).min(width);
    let mut points = Vec::new();
    let mut x = start;
    while x <= end + 0.5 {
        let time = first.time + (x - x_of(first.time)) / (x_of(last.time) - x_of(first.time)).max(1e-9) * (last.time - first.time);
        if let Some(value) = value_at(keys, time) {
            points.push((x as f32, y_of(value) as f32));
        }
        x += 1.;
    }
    let _ = rate;
    points
}

fn draw(compare: &UnityCompare, sketch: &mut Sketch, device: f64, width: f64) {
    let rate = compare.clip.sample_rate.max(1.);
    let frames = frames_shown(compare);
    let per_frame = width / frames;
    let x_of = |seconds: f64| seconds * rate * per_frame * device;
    let full = f64::from(sketch.width());
    let step = tick_step(per_frame);
    let mut frame = 0.;
    while frame <= frames {
        let x = (frame * per_frame * device) as f32;
        let major = (frame / step).round() as i64 % 2 == 0;
        let top = if major { 3. } else { 16. };
        sketch.line((x, (top * device) as f32), (x, (f64::from(RULER) * device) as f32), Stroke::solid(device as f32, TICK));
        frame += step;
    }
    let mut top = f64::from(RULER);
    sketch.line((0., (top * device) as f32), (full as f32, (top * device) as f32), Stroke::solid(device as f32, LINE));
    for (ix, length) in compare.clip.lengths.iter().enumerate() {
        let x = x_of(*length) as f32;
        let color = if ix == 0 { RED } else { GREEN };
        sketch.line((x, 0.), (x, sketch.height() as f32), Stroke::dashed(device as f32, [color[0], color[1], color[2], 90], 3. * device as f32, 4. * device as f32));
        sketch.line((x, 0.), (x, (f64::from(RULER) * device) as f32), Stroke::solid(2. * device as f32, color));
    }
    for track in &compare.clip.tracks {
        let height = f64::from(lane_height(track));
        let lane_top = top;
        match track.after.as_ref().or(track.before.as_ref()) {
            Some(Keys::Curve(_)) => {
                let (low, high) = range(track);
                let y_of = |value: f64| (lane_top + LANE_PAD + (1. - (value - low) / (high - low)) * (height - 2. * LANE_PAD)) * device;
                let other_keys = |keys: &Option<Keys>| match keys {
                    Some(Keys::Curve(keys)) => keys.clone(),
                    _ => Vec::new(),
                };
                let (before, after) = (other_keys(&track.before), other_keys(&track.after));
                let same = |key: &CurveKey, others: &[CurveKey]| others.iter().any(|other| (other.time - key.time).abs() < 1e-6 && (other.value - key.value).abs() < 1e-6);
                if !before.is_empty() {
                    let points = curve_points(&before, x_of, y_of, full, rate);
                    sketch.polyline(&points, Stroke::dashed((2. * device) as f32, [RED[0], RED[1], RED[2], 220], (5. * device) as f32, (4. * device) as f32));
                }
                if !after.is_empty() {
                    let points = curve_points(&after, x_of, y_of, full, rate);
                    sketch.polyline(&points, Stroke::solid((2. * device) as f32, GREEN));
                }
                for key in &before {
                    let fill = if same(key, &after) { WHITE } else { RED };
                    sketch.diamond((x_of(key.time) as f32, y_of(key.value) as f32), (4.5 * device) as f32, fill, Some(([10, 10, 10, 255], device as f32)));
                }
                for key in &after {
                    let fill = if same(key, &before) { WHITE } else { GREEN };
                    sketch.diamond((x_of(key.time) as f32, y_of(key.value) as f32), (4.5 * device) as f32, fill, Some(([10, 10, 10, 255], device as f32)));
                }
            }
            Some(Keys::Objects(_)) => {
                for (row, keys) in [&track.before, &track.after].into_iter().enumerate() {
                    let Some(Keys::Objects(keys)) = keys else {
                        continue;
                    };
                    let color = if row == 0 { RED } else { GREEN };
                    let row_top = lane_top + 3. + row as f64 * (height / 2.);
                    for key in keys {
                        let center = (x_of(key.time) / device).max(THUMB / 2. + 1.);
                        let left = center - THUMB / 2.;
                        sketch.fill_rect((left * device) as f32, (row_top * device) as f32, (THUMB * device) as f32, (THUMB * device) as f32, [255, 255, 255, 8]);
                        if let Some(image) = key.sprite.as_ref().and_then(|(guid, file_id)| compare.files.sprite(guid, *file_id)) {
                            let fit = (THUMB - 4.) / f64::from(image.width.max(image.height));
                            let (w, h) = (f64::from(image.width) * fit, f64::from(image.height) * fit);
                            sketch.image(
                                &image.rgba,
                                image.width,
                                image.height,
                                (((center - w / 2.) * device) as f32, ((row_top + THUMB - 2. - h) * device) as f32),
                                ((fit * device) as f32, (fit * device) as f32),
                                1.,
                            );
                        }
                        sketch.rect((left * device) as f32, (row_top * device) as f32, (THUMB * device) as f32, (THUMB * device) as f32, (3. * device) as f32, Stroke::solid(device as f32, color));
                    }
                }
            }
            None => {}
        }
        top += height;
        sketch.line((0., (top * device) as f32), (full as f32, (top * device) as f32), Stroke::solid(device as f32, LINE));
    }
    if compare.clip.events_changed || !compare.clip.events[1].is_empty() {
        for (row, events) in compare.clip.events.iter().enumerate() {
            let color = if row == 0 { RED } else { GREEN };
            for (time, _) in events {
                let x = x_of(*time);
                let y = (top + 5. + row as f64 * (f64::from(EVENT_LANE) / 2.)) * device;
                sketch.fill_rect(x as f32, y as f32, (6. * device) as f32, (16. * device) as f32, color);
            }
        }
        top += f64::from(EVENT_LANE);
        sketch.line((0., (top * device) as f32), (full as f32, (top * device) as f32), Stroke::solid(device as f32, LINE));
    }
}

fn prepare(compare: &mut UnityCompare, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<UnityCompare>) -> Option<(Bounds<Pixels>, Arc<RenderImage>)> {
    let width = f64::from(f32::from(bounds.size.width));
    let device = f64::from(window.scale_factor());
    if compare.curves_bounds != Some(bounds) {
        compare.curves_bounds = Some(bounds);
        window.request_animation_frame();
        cx.notify();
    }
    let stamp = width.to_bits() ^ (compare.clip.tracks.len() as u64).rotate_left(17) ^ (compare.current as u64).rotate_left(33);
    let mut sheet = std::mem::take(&mut compare.curves);
    let image = sheet.image(bounds, stamp, BACKGROUND, window, |sketch, _| draw(compare, sketch, device, width));
    compare.curves = sheet;
    image.map(|image| (bounds, image))
}

fn name_cell(badge: Option<AnyElement>, group: SharedString, name: SharedString, struck: bool, height: f32, current: bool, cx: &App) -> AnyElement {
    let theme = cx.theme();
    h_flex()
        .w_full()
        .h(px(height))
        .px_3()
        .gap_2()
        .border_b_1()
        .border_color(theme.border)
        .when(current, |this| this.bg(theme.list_active))
        .children(badge)
        .child(
            v_flex()
                .min_w_0()
                .child(div().xs_text().ellipsis().text_color(theme.muted_foreground).child(group))
                .child(div().sm_text().when(struck, |this| this.line_through().text_color(theme.muted_foreground)).child(name)),
        )
        .into_any_element()
}

fn overlay_labels(compare: &UnityCompare, cx: &App) -> Vec<AnyElement> {
    let Some(bounds) = compare.curves_bounds else {
        return Vec::new();
    };
    let theme = cx.theme();
    let width = f64::from(f32::from(bounds.size.width));
    let rate = compare.clip.sample_rate.max(1.);
    let frames = frames_shown(compare);
    let per_frame = width / frames;
    let tick = tick_step(per_frame);
    let mut step = tick * 2.;
    while step * per_frame < SMALLEST_LABEL_GAP {
        step += tick * 2.;
    }
    let mut labels = Vec::new();
    let mut frame = 0.;
    while frame <= frames {
        let seconds = (frame / rate).floor() as i64;
        let rest = frame as i64 - seconds * rate.round() as i64;
        labels.push(
            div()
                .absolute()
                .top(px(2.))
                .left(px((frame * per_frame) as f32 + 4.))
                .text_size(px(11.))
                .text_color(theme.muted_foreground)
                .child(format!("{seconds}:{rest:02}"))
                .into_any_element(),
        );
        frame += step;
    }
    let mut top = RULER;
    for track in &compare.clip.tracks {
        let height = lane_height(track);
        if matches!(track.after.as_ref().or(track.before.as_ref()), Some(Keys::Curve(_))) {
            let (low, high) = range(track);
            for (value, at) in [(high, top + 2.), (low, top + height - 14.)] {
                labels.push(
                    div()
                        .absolute()
                        .top(px(at))
                        .left(px(4.))
                        .text_size(px(10.))
                        .text_color(theme.muted_foreground)
                        .child(short_number(value))
                        .into_any_element(),
                );
            }
        }
        top += height;
    }
    for (row, events) in compare.clip.events.iter().enumerate() {
        for (ix, (time, name)) in events.iter().enumerate() {
            let flag = (time * rate * per_frame) as f32;
            let width = name.chars().count() as f32 * LETTER;
            let left = if ix % 2 == 0 && flag - width - 4. > 0. { flag - width - 4. } else { flag + 10. };
            labels.push(
                div()
                    .absolute()
                    .top(px(top + 4. + row as f32 * (EVENT_LANE / 2.)))
                    .left(px(left))
                    .xs_text()
                    .child(name.clone())
                    .into_any_element(),
            );
        }
    }
    labels
}

fn details(compare: &UnityCompare) -> Option<MediaDetails> {
    let [before, after] = &compare.clip.settings;
    let columns: Vec<SharedString> = after.as_ref().or(before.as_ref())?.iter().map(|(name, _)| SharedString::from(name.clone())).collect();
    let mut table = MediaDetails::named("Clip", columns);
    for (version, settings, source) in [(Version::Before, before, &compare.sources[0]), (Version::After, after, &compare.sources[1])] {
        if let Some(settings) = settings {
            table = table.row(DetailsRow {
                version,
                source: source.clone(),
                values: settings.iter().map(|(_, value)| SharedString::from(value.clone())).collect(),
            });
        }
    }
    Some(table)
}

pub(super) fn render(compare: &mut UnityCompare, _: &mut Window, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let current_track = compare
        .clip
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.state != State::Same)
        .nth(compare.current)
        .map(|(ix, _)| ix);
    let mut names: Vec<AnyElement> = vec![
        h_flex()
            .h(px(RULER))
            .px_3()
            .border_b_1()
            .border_color(theme.border)
            .xs_text()
            .text_color(theme.muted_foreground)
            .child(format!("{} samples a second", short_number(compare.clip.sample_rate)))
            .into_any_element(),
    ];
    for (ix, track) in compare.clip.tracks.iter().enumerate() {
        names.push(name_cell(
            state_badge(track.state, cx),
            track.group.clone().into(),
            track.name.clone().into(),
            track.state == State::Removed,
            lane_height(track),
            current_track == Some(ix),
            cx,
        ));
    }
    let has_events = compare.clip.events_changed || !compare.clip.events[1].is_empty();
    if has_events {
        let badge = state_badge(if compare.clip.events_changed { State::Changed } else { State::Same }, cx);
        names.push(name_cell(badge, "Animation events".into(), "Events".into(), false, EVENT_LANE, current_track.is_none() && compare.clip.events_changed, cx));
    }
    let entity = cx.entity();
    let plot = canvas(
        move |bounds, window, cx| entity.update(cx, |this, cx| prepare(this, bounds, window, cx)),
        |_, prepared, window, _| {
            if let Some((bounds, image)) = prepared {
                let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
            }
        },
    )
    .absolute()
    .size_full();
    let lanes_height = RULER
        + compare.clip.tracks.iter().map(lane_height).sum::<f32>()
        + if has_events { EVENT_LANE } else { 0. };
    let sides = h_flex()
        .flex_none()
        .h(px(SIDES))
        .px_3()
        .gap_2()
        .child(VersionTag::new(Version::Before, format!("{} · dashed", compare.sources[0]).into()))
        .child(VersionTag::new(Version::After, compare.sources[1].clone()))
        .child(div().xs_text().text_color(theme.muted_foreground).child("A diamond is a key: white when both versions have it"));
    let lanes = h_flex()
        .w_full()
        .h(px(lanes_height))
        .items_start()
        .child(v_flex().flex_none().w(px(NAME_WIDTH)).border_r_1().border_color(theme.border).children(names))
        .child(div().relative().flex_1().min_w_0().h_full().child(plot).children(overlay_labels(compare, cx)));
    let empty = compare.clip.tracks.is_empty() && !has_events;
    v_flex()
        .size_full()
        .child(sides)
        .child(if empty {
            crate::ui::hint("No animated properties changed.", cx)
        } else {
            div()
                .id("unity-curves")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .border_t_1()
                .border_color(theme.border)
                .child(lanes)
                .into_any_element()
        })
        .children(details(compare))
        .into_any_element()
}
