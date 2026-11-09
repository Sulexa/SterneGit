use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, Bounds, Context, Corners, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, ParentElement as _, Pixels, RenderImage, ScrollEvent, StatefulInteractiveElement as _, Styled as _,
    Window,
    assets::IconName,
    canvas,
    component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    UnityCompare,
    cells::state_badge,
    inspector::{part_head, property_row, sides_row},
    sheet::{Area, Mapping, Sheet, Viewport},
};
use crate::{
    media::Stroke,
    repo::compare::{Version, VersionTag},
    ui::{Look as _, ctrl_wheel_area},
    unity::{Layout, Marker, ObjectChange, Outline, Spot, State, View},
};

const BACKGROUND: [u8; 3] = [0x0c, 0x0e, 0x13];
const GRID: [u8; 4] = [255, 255, 255, 12];
const GRID_MAJOR: [u8; 4] = [255, 255, 255, 26];
const AXIS: [u8; 4] = [255, 255, 255, 52];
const RED: [u8; 4] = [0xf8, 0x71, 0x71, 255];
const GREEN: [u8; 4] = [0x4a, 0xde, 0x80, 255];
const YELLOW: [u8; 4] = [0xfa, 0xcc, 0x15, 255];
const BLUE: [u8; 4] = [0x60, 0xa5, 0xfa, 255];
const MUTED: [u8; 4] = [0xa3, 0xa3, 0xa3, 255];
const LIGHT: [u8; 4] = [0xfd, 0xe6, 0x8a, 255];
const GHOST: f32 = 0.35;
const MARKER_RADIUS: f64 = 7.;
const RING_PAD: f64 = 5.;
const PICK_PAD: f64 = 9.;
const SMALLEST_GRID_GAP: f64 = 28.;
const CAMERA_ASPECT: f64 = 16. / 9.;
const PICK_PANEL_HEIGHT: f32 = 200.;
const PROPERTY_WIDTH: f32 = 176.;
const LINE_HEIGHT_FOR_WHEEL: f32 = 20.;

pub(crate) struct SceneState {
    viewport: Viewport,
    pub(super) sheet: Sheet,
    bounds: Option<Bounds<Pixels>>,
    mapping: Option<Mapping>,
    boxes: Vec<(i64, [f64; 4])>,
    dragging: Option<(f64, f64)>,
    unit: f64,
    generation: u64,
}

impl SceneState {
    pub(crate) fn new() -> Self {
        Self {
            viewport: Viewport::new(true),
            sheet: Sheet::default(),
            bounds: None,
            mapping: None,
            boxes: Vec::new(),
            dragging: None,
            unit: 100.,
            generation: 0,
        }
    }

    pub(crate) fn forget(&mut self) {
        self.generation += 1;
    }

    pub(crate) fn percent(&self) -> Option<f64> {
        self.mapping.map(|mapping| self.viewport.percent(&mapping, self.unit))
    }

    pub(crate) fn fits(&self) -> bool {
        self.viewport.fits()
    }

    pub(crate) fn zoom(&mut self, steps: f64) {
        if let Some(mapping) = self.mapping {
            self.viewport.zoom_by(&mapping, steps, None);
        }
    }

    pub(crate) fn fit(&mut self) {
        self.viewport.fit();
    }
}

#[derive(Clone)]
struct Placed {
    key: i64,
    state: State,
    before: Option<Spot>,
    after: Option<Spot>,
}

fn placed(compare: &UnityCompare) -> Vec<Placed> {
    let [before, after] = compare.files.layouts();
    let mut keys: Vec<i64> = after.map(|layout| layout.spots.iter().map(|spot| spot.key).collect()).unwrap_or_default();
    for spot in before.map(|layout| layout.spots.as_slice()).unwrap_or_default() {
        if !keys.contains(&spot.key) {
            keys.push(spot.key);
        }
    }
    keys.into_iter()
        .map(|key| Placed {
            key,
            state: compare.comparison.state_of(key),
            before: before.and_then(|layout: &Layout| layout.spot(key)).cloned(),
            after: after.and_then(|layout: &Layout| layout.spot(key)).cloned(),
        })
        .collect()
}

fn sprite_rect(compare: &UnityCompare, sprite: &crate::unity::PlacedSprite) -> Option<(Arc<crate::unity::SpriteImage>, Area)> {
    let image = compare.files.sprite(&sprite.guid, sprite.file_id)?.clone();
    let (width, height) = image.size_in_units();
    let (w, h) = (width * sprite.scale.0, height * sprite.scale.1);
    let left = sprite.at.0 - image.pivot.0 * w;
    let bottom = sprite.at.1 - image.pivot.1 * h;
    Some((image, Area { left, top: bottom, right: left + w, bottom: bottom + h }))
}

fn content(compare: &UnityCompare, items: &[Placed]) -> Option<Area> {
    let mut points = Vec::new();
    for item in items {
        for spot in item.before.iter().chain(&item.after) {
            points.push(spot.at);
            for sprite in &spot.sprites {
                if let Some((_, area)) = sprite_rect(compare, sprite) {
                    points.push((area.left, area.top));
                    points.push((area.right, area.bottom));
                }
            }
            if let Some(size) = spot.camera {
                points.push((spot.at.0 - size * CAMERA_ASPECT, spot.at.1 - size));
                points.push((spot.at.0 + size * CAMERA_ASPECT, spot.at.1 + size));
            }
        }
    }
    Area::around(points)
}

fn grid_step(scale: f64) -> f64 {
    let mut step = 10f64.powf((SMALLEST_GRID_GAP / scale).log10().floor());
    for factor in [1., 2., 5., 10.] {
        if step * factor * scale >= SMALLEST_GRID_GAP {
            step *= factor;
            break;
        }
    }
    step
}

fn spot_box(compare: &UnityCompare, spot: &Spot, mapping: &Mapping) -> [f64; 4] {
    let mut corners = Vec::new();
    for sprite in &spot.sprites {
        if let Some((_, area)) = sprite_rect(compare, sprite) {
            corners.push(mapping.screen((area.left, area.top)));
            corners.push(mapping.screen((area.right, area.bottom)));
        }
    }
    match Area::around(corners) {
        Some(area) if area.right - area.left > 1. => [area.left, area.top, area.right - area.left, area.bottom - area.top],
        _ => {
            let (x, y) = mapping.screen(spot.at);
            [x - MARKER_RADIUS, y - MARKER_RADIUS, MARKER_RADIUS * 2., MARKER_RADIUS * 2.]
        }
    }
}

fn moved(before: &Spot, after: &Spot) -> bool {
    (before.at.0 - after.at.0).abs() > 1e-6 || (before.at.1 - after.at.1).abs() > 1e-6
}

fn looks_different(before: &Spot, after: &Spot) -> bool {
    moved(before, after) || before.sprites != after.sprites
}

fn draw_spot(compare: &UnityCompare, sketch: &mut crate::media::Sketch, spot: &Spot, mapping: &Mapping, device: f64, opacity: f32) {
    let mut sprites: Vec<_> = spot.sprites.iter().collect();
    sprites.sort_by_key(|sprite| sprite.order);
    let mut drew = false;
    for sprite in sprites {
        let Some((image, area)) = sprite_rect(compare, sprite) else {
            continue;
        };
        let (left, top) = mapping.screen((area.left, area.bottom));
        let width = (area.right - area.left) * mapping.scale;
        let height = (area.bottom - area.top) * mapping.scale;
        sketch.image(
            &image.rgba,
            image.width,
            image.height,
            ((left * device) as f32, (top * device) as f32),
            (
                (width * device / f64::from(image.width)) as f32,
                (height * device / f64::from(image.height)) as f32,
            ),
            opacity * sprite.alpha.clamp(0., 1.) as f32,
        );
        drew = true;
    }
    if drew {
        return;
    }
    let (x, y) = mapping.screen(spot.at);
    let center = ((x * device) as f32, (y * device) as f32);
    let radius = (MARKER_RADIUS * device) as f32;
    let faded = |color: [u8; 4]| [color[0], color[1], color[2], (f32::from(color[3]) * opacity) as u8];
    match spot.marker {
        Marker::Light => {
            sketch.circle(center, radius * 0.6, Stroke::solid(2. * device as f32, faded(LIGHT)));
            for ray in 0..8 {
                let angle = f32::from(ray as u8) * std::f32::consts::FRAC_PI_4;
                let (sin, cos) = angle.sin_cos();
                sketch.line(
                    (center.0 + cos * radius * 0.95, center.1 + sin * radius * 0.95),
                    (center.0 + cos * radius * 1.45, center.1 + sin * radius * 1.45),
                    Stroke::solid(2. * device as f32, faded(LIGHT)),
                );
            }
        }
        Marker::Camera => {
            sketch.rect(center.0 - radius, center.1 - radius * 0.7, radius * 1.4, radius * 1.4, 2., Stroke::solid(2. * device as f32, faded(MUTED)));
            sketch.triangle(
                [
                    (center.0 + radius * 0.4, center.1),
                    (center.0 + radius, center.1 - radius * 0.5),
                    (center.0 + radius, center.1 + radius * 0.5),
                ],
                faded(MUTED),
            );
        }
        Marker::Plain => sketch.circle(center, radius * 0.5, Stroke::solid(2. * device as f32, faded(MUTED))),
    }
}

fn ring(sketch: &mut crate::media::Sketch, area: [f64; 4], pad: f64, stroke: Stroke, device: f64) {
    let [x, y, w, h] = area;
    sketch.rect(
        ((x - pad) * device) as f32,
        ((y - pad) * device) as f32,
        ((w + pad * 2.) * device) as f32,
        ((h + pad * 2.) * device) as f32,
        ((pad - 1.).max(1.) * device) as f32,
        stroke,
    );
}

fn draw_outline(sketch: &mut crate::media::Sketch, outline: &Outline, mapping: &Mapping, stroke: Stroke, device: f64) {
    match outline {
        Outline::Circle { center, radius } => {
            let (x, y) = mapping.screen(*center);
            sketch.circle(((x * device) as f32, (y * device) as f32), (radius * mapping.scale * device) as f32, stroke);
        }
        Outline::Box { center, size } => {
            let (x, y) = mapping.screen((center.0 - size.0 / 2., center.1 + size.1 / 2.));
            sketch.rect(
                (x * device) as f32,
                (y * device) as f32,
                (size.0 * mapping.scale * device) as f32,
                (size.1 * mapping.scale * device) as f32,
                0.,
                stroke,
            );
        }
    }
}

fn draw(compare: &UnityCompare, items: &[Placed], sketch: &mut crate::media::Sketch, mapping: &Mapping, device: f64) -> Vec<(i64, [f64; 4])> {
    let width = 1.5 * device as f32;
    let dash = (5. * device) as f32;
    let step = grid_step(mapping.scale);
    let top_left = mapping.world((0., 0.));
    let bottom_right = mapping.world(mapping.size);
    let mut x = (top_left.0 / step).floor() * step;
    while x <= bottom_right.0 {
        let (sx, _) = mapping.screen((x, 0.));
        let color = if x.abs() < step / 2. { AXIS } else if ((x / step).round() as i64) % 5 == 0 { GRID_MAJOR } else { GRID };
        sketch.line(((sx * device) as f32, 0.), ((sx * device) as f32, sketch.height() as f32), Stroke::solid(device as f32, color));
        x += step;
    }
    let mut y = (bottom_right.1.min(top_left.1) / step).floor() * step;
    while y <= bottom_right.1.max(top_left.1) {
        let (_, sy) = mapping.screen((0., y));
        let color = if y.abs() < step / 2. { AXIS } else if ((y / step).round() as i64) % 5 == 0 { GRID_MAJOR } else { GRID };
        sketch.line((0., (sy * device) as f32), (sketch.width() as f32, (sy * device) as f32), Stroke::solid(device as f32, color));
        y += step;
    }
    let frame = |sketch: &mut crate::media::Sketch, spot: &Spot, stroke: Stroke| {
        if let Some(size) = spot.camera {
            let (left, top) = mapping.screen((spot.at.0 - size * CAMERA_ASPECT, spot.at.1 + size));
            sketch.rect(
                (left * device) as f32,
                (top * device) as f32,
                (size * CAMERA_ASPECT * 2. * mapping.scale * device) as f32,
                (size * 2. * mapping.scale * device) as f32,
                0.,
                stroke,
            );
        }
    };
    for item in items {
        match (item.state, &item.before, &item.after) {
            (State::Same, _, Some(after)) => frame(sketch, after, Stroke::solid(width, [MUTED[0], MUTED[1], MUTED[2], 120])),
            (_, before, after) => {
                if let Some(before) = before {
                    frame(sketch, before, Stroke::dashed(width, RED, dash, dash * 0.8));
                }
                if let Some(after) = after {
                    frame(sketch, after, Stroke::solid(width, GREEN));
                }
            }
        }
    }
    for item in items {
        if let (Some(before), Some(after)) = (&item.before, &item.after)
            && item.state != State::Same
            && looks_different(before, after)
        {
            draw_spot(compare, sketch, before, mapping, device, GHOST);
        }
        if let (Some(before), None) = (&item.before, &item.after) {
            draw_spot(compare, sketch, before, mapping, device, GHOST);
        }
        if let Some(after) = &item.after {
            draw_spot(compare, sketch, after, mapping, device, 1.);
        }
    }
    let mut boxes = Vec::new();
    for item in items {
        let after_box = item.after.as_ref().map(|spot| spot_box(compare, spot, mapping));
        let before_box = item.before.as_ref().map(|spot| spot_box(compare, spot, mapping));
        let changed_look = match (&item.before, &item.after) {
            (Some(before), Some(after)) => looks_different(before, after),
            _ => true,
        };
        if item.state != State::Same {
            if let (Some(before_box), true) = (before_box, changed_look) {
                ring(sketch, before_box, RING_PAD, Stroke::dashed(width, RED, dash * 0.8, dash * 0.6), device);
            }
            if let Some(after_box) = after_box {
                let color = if item.state == State::Added { GREEN } else { YELLOW };
                ring(sketch, after_box, RING_PAD, Stroke::solid(width, color), device);
            }
            if let (Some(before), Some(after), Some(from), Some(to)) = (&item.before, &item.after, before_box, after_box)
                && moved(before, after)
            {
                let start = (from[0] + from[2] / 2., from[1] + from[3] / 2.);
                let end = (to[0] + to[2] / 2., to[1] + to[3] / 2.);
                let length = ((end.0 - start.0).powi(2) + (end.1 - start.1).powi(2)).sqrt().max(1e-6);
                let unit = ((end.0 - start.0) / length, (end.1 - start.1) / length);
                let gap_start = from[2].max(from[3]) / 2. + 8.;
                let gap_end = to[2].max(to[3]) / 2. + 12.;
                if length > gap_start + gap_end {
                    let a = (start.0 + unit.0 * gap_start, start.1 + unit.1 * gap_start);
                    let b = (end.0 - unit.0 * gap_end, end.1 - unit.1 * gap_end);
                    sketch.line(((a.0 * device) as f32, (a.1 * device) as f32), ((b.0 * device) as f32, (b.1 * device) as f32), Stroke::dashed(width, YELLOW, dash, dash * 0.8));
                    let tip = ((b.0 + unit.0 * 7.) * device, (b.1 + unit.1 * 7.) * device);
                    let back = (b.0 * device, b.1 * device);
                    let side = (-unit.1 * 4.5 * device, unit.0 * 4.5 * device);
                    sketch.triangle(
                        [
                            (tip.0 as f32, tip.1 as f32),
                            ((back.0 + side.0) as f32, (back.1 + side.1) as f32),
                            ((back.0 - side.0) as f32, (back.1 - side.1) as f32),
                        ],
                        YELLOW,
                    );
                }
            }
            if item.after.is_none()
                && let Some([x, y, w, h]) = before_box
            {
                let (cx, cy) = ((x + w / 2.) * device, (y + h / 2.) * device);
                let arm = 6. * device;
                let stroke = Stroke::solid(2. * device as f32, RED);
                sketch.line(((cx - arm) as f32, (cy - arm) as f32), ((cx + arm) as f32, (cy + arm) as f32), stroke);
                sketch.line(((cx + arm) as f32, (cy - arm) as f32), ((cx - arm) as f32, (cy + arm) as f32), stroke);
            }
            let shapes = |spot: &Option<Spot>| spot.as_ref().map(|spot| spot.shapes.clone()).unwrap_or_default();
            let (old_shapes, new_shapes) = (shapes(&item.before), shapes(&item.after));
            if old_shapes != new_shapes {
                for (_, outline) in &old_shapes {
                    draw_outline(sketch, outline, mapping, Stroke::dashed(width, RED, dash, dash * 0.8), device);
                }
                for (_, outline) in &new_shapes {
                    draw_outline(sketch, outline, mapping, Stroke::solid(width, GREEN), device);
                }
            }
        }
        if Some(item.key) == compare.picked
            && let Some(picked) = after_box.or(before_box)
        {
            ring(sketch, picked, PICK_PAD, Stroke::solid(2. * device as f32, BLUE), device);
        }
        if let Some(found) = after_box.or(before_box) {
            boxes.push((item.key, found));
        }
    }
    boxes
}

fn prepare(compare: &mut UnityCompare, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<UnityCompare>) -> Option<(Bounds<Pixels>, Arc<RenderImage>)> {
    let items = placed(compare);
    let size = (f64::from(f32::from(bounds.size.width)), f64::from(f32::from(bounds.size.height)));
    let mapping = compare.scene.viewport.mapping(content(compare, &items), size);
    let unit = items
        .iter()
        .flat_map(|item| item.after.iter().chain(&item.before))
        .flat_map(|spot| spot.sprites.iter())
        .find_map(|sprite| compare.files.sprite(&sprite.guid, sprite.file_id).map(|image| image.pixels_per_unit))
        .unwrap_or(100.);
    compare.scene.unit = unit;
    let resized = compare.scene.bounds.is_none_or(|old| old.size != bounds.size || old.origin != bounds.origin);
    let remapped = compare.scene.mapping != Some(mapping);
    compare.scene.bounds = Some(bounds);
    compare.scene.mapping = Some(mapping);
    let stamp = mapping.stamp() ^ compare.scene.generation.rotate_left(7) ^ compare.picked.map_or(0, |key| key as u64).rotate_left(41);
    let device = f64::from(window.scale_factor());
    let mut boxes = None;
    let mut sheet = std::mem::take(&mut compare.scene.sheet);
    let image = sheet.image(bounds, stamp, BACKGROUND, window, |sketch, _| {
        boxes = Some(draw(compare, &items, sketch, &mapping, device));
    });
    compare.scene.sheet = sheet;
    if let Some(boxes) = boxes {
        compare.scene.boxes = boxes;
    }
    if resized || remapped {
        window.request_animation_frame();
        cx.notify();
    }
    image.map(|image| (bounds, image))
}

fn note(object: &ObjectChange) -> Option<String> {
    if object.state != State::Changed {
        return None;
    }
    let row = object.parts.iter().flat_map(|part| part.rows.iter()).find(|row| row.state != State::Same)?;
    let text = |value: &Option<crate::unity::Value>| value.as_ref().map(crate::unity::Value::text).unwrap_or_default();
    Some(match (&row.before, &row.after) {
        (Some(_), Some(_)) => format!("{} {} → {}", row.label, text(&row.before), text(&row.after)),
        _ => row.label.clone(),
    })
}

fn labels(compare: &UnityCompare, cx: &App) -> Vec<AnyElement> {
    let Some(mapping) = compare.scene.mapping else {
        return Vec::new();
    };
    let theme = cx.theme();
    let mut out = Vec::new();
    for item in placed(compare) {
        let Some(object) = compare.comparison.objects.iter().find(|object| object.key == item.key) else {
            continue;
        };
        let Some(spot) = item.after.as_ref().or(item.before.as_ref()) else {
            continue;
        };
        let [x, y, w, h] = spot_box(compare, spot, &mapping);
        let camera = spot.camera.map(|size| mapping.screen((spot.at.0 - size * CAMERA_ASPECT, spot.at.1 + size)));
        let (left, top, centered) = match camera {
            Some((left, top)) => (left + 8., top + 8., false),
            None => (x + w / 2., y + h + 10., true),
        };
        out.push(
            h_flex()
                .absolute()
                .left(px(left as f32))
                .top(px(top as f32))
                .when(centered, |this| this.ml(px(-90.)).w(px(180.)).justify_center())
                .child(
                    h_flex()
                        .flex_none()
                        .gap_1p5()
                        .px_1p5()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.popover.opacity(0.88))
                        .text_size(px(11.))
                        .line_height(px(18.))
                        .children(state_badge(object.state, cx))
                        .when(spot.camera.is_some(), |this| this.child(Icon::new(IconName::Video).xsmall()))
                        .child(object.name.clone())
                        .when_some(note(object), |this, note| this.child(div().text_color(theme.muted_foreground).child(note))),
                )
                .into_any_element(),
        );
    }
    out
}

fn picked_panel(compare: &UnityCompare, cx: &App) -> Option<AnyElement> {
    let object = compare.comparison.objects.iter().find(|object| Some(object.key) == compare.picked)?;
    let theme = cx.theme();
    let mut rows: Vec<AnyElement> = Vec::new();
    rows.push(
        h_flex()
            .h(px(34.))
            .px_3()
            .gap_1p5()
            .bg(theme.secondary.opacity(0.55))
            .children(state_badge(object.state, cx))
            .child(Icon::new(IconName::Box).small().text_color(if object.prefab { theme.blue } else { theme.muted_foreground }))
            .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(object.name.clone()))
            .when_some(object.parent.clone(), |this, parent| this.child(div().xs_text().text_color(theme.muted_foreground).child(format!("in {parent}"))))
            .when_some(object.what.clone(), |this, what| this.child(div().xs_text().text_color(theme.muted_foreground).child(what)))
            .into_any_element(),
    );
    if object.state != State::Removed {
        for part in &object.parts {
            rows.push(part_head(part, 28., cx).into_any_element());
            rows.extend(part.rows.iter().map(|row| property_row(row, PROPERTY_WIDTH, 28., false, cx).into_any_element()));
        }
    }
    Some(
        v_flex()
            .flex_none()
            .max_h(px(PICK_PANEL_HEIGHT))
            .border_t_1()
            .border_color(theme.border)
            .child(sides_row(&compare.sources, PROPERTY_WIDTH, cx))
            .child(div().id("unity-scene-pick").flex_1().min_h_0().overflow_y_scroll().children(rows))
            .into_any_element(),
    )
}

fn press(compare: &mut UnityCompare, event: &MouseDownEvent, cx: &mut Context<UnityCompare>) {
    let Some(bounds) = compare.scene.bounds else {
        return;
    };
    let x = f64::from(f32::from(event.position.x - bounds.origin.x));
    let y = f64::from(f32::from(event.position.y - bounds.origin.y));
    let hit = compare
        .scene
        .boxes
        .iter()
        .rev()
        .find(|(_, [left, top, width, height])| x >= left - RING_PAD && x <= left + width + RING_PAD && y >= top - RING_PAD && y <= top + height + RING_PAD)
        .map(|(key, _)| *key);
    match hit {
        Some(key) if compare.comparison.objects.iter().any(|object| object.key == key) => compare.pick(key, cx),
        _ => compare.scene.dragging = Some((x, y)),
    }
}

fn drag(compare: &mut UnityCompare, event: &MouseMoveEvent, cx: &mut Context<UnityCompare>) {
    let (Some((from_x, from_y)), Some(bounds), Some(mapping)) = (compare.scene.dragging, compare.scene.bounds, compare.scene.mapping) else {
        return;
    };
    if !event.dragging() {
        compare.scene.dragging = None;
        return;
    }
    let x = f64::from(f32::from(event.position.x - bounds.origin.x));
    let y = f64::from(f32::from(event.position.y - bounds.origin.y));
    compare.scene.viewport.pan_by(&mapping, x - from_x, y - from_y);
    compare.scene.dragging = Some((x, y));
    cx.notify();
}

fn wheel_zoom(compare: &mut UnityCompare, event: &ScrollEvent, cx: &mut Context<UnityCompare>) -> bool {
    let (Some(bounds), Some(mapping)) = (compare.scene.bounds, compare.scene.mapping) else {
        return false;
    };
    let delta = event.delta.pixel_delta(px(LINE_HEIGHT_FOR_WHEEL));
    let at = (
        f64::from(f32::from(event.position.x - bounds.origin.x)),
        f64::from(f32::from(event.position.y - bounds.origin.y)),
    );
    compare.scene.viewport.zoom_by(&mapping, if f32::from(delta.y) > 0. { 1. } else { -1. }, Some(at));
    cx.notify();
    true
}

pub(super) fn render(compare: &mut UnityCompare, _: &mut Window, cx: &mut Context<UnityCompare>) -> AnyElement {
    let entity = cx.entity();
    let picture = canvas(
        move |bounds, window, cx| entity.update(cx, |this, cx| prepare(this, bounds, window, cx)),
        |_, prepared, window, _| {
            if let Some((bounds, image)) = prepared {
                let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
            }
        },
    )
    .absolute()
    .size_full();
    let theme = cx.theme();
    let view = match compare.files.view {
        View::Front => "Seen from the front, X and Y",
        View::Above => "Seen from above, X and Z",
    };
    let prefab = compare.files.kind == crate::unity::UnityKind::Prefab;
    let hidden_places = if prefab {
        format!("Prefab mode · {view}")
    } else {
        view.to_owned()
    };
    let legend = h_flex()
        .absolute()
        .top_2p5()
        .left_2p5()
        .gap_2()
        .child(VersionTag::new(Version::Before, format!("{} · dashed", compare.sources[0]).into()))
        .child(VersionTag::new(Version::After, compare.sources[1].clone()))
        .child(div().xs_text().text_color(theme.muted_foreground).child(hidden_places));
    let labels = labels(compare, cx);
    v_flex()
        .size_full()
        .child(
            div()
                .id("unity-scene")
                .relative()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| press(this, event, cx)))
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| drag(this, event, cx)))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.scene.dragging = None))
                .child(picture)
                .children(labels)
                .child(legend)
                .child(ctrl_wheel_area(cx.entity().downgrade(), |this: &mut UnityCompare, event: &ScrollEvent, cx: &mut Context<UnityCompare>| wheel_zoom(this, event, cx))),
        )
        .children(picked_panel(compare, cx))
        .into_any_element()
}
