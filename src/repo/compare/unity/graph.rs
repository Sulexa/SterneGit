use std::sync::Arc;

use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, Corners, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement as _, Pixels, RenderImage, ScrollEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    canvas,
    component::{
        ActiveTheme as _, Icon, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::{DropdownMenu as _, PopupMenuItem},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, rgb,
};

use super::{
    super::{Version, details::version_dot},
    UnityCompare,
    cells::state_badge,
    inspector::{property_row, sides_row},
    sheet::{Area, Mapping, Sheet, Viewport},
};
use crate::{
    media::{Sketch, Stroke},
    ui::{Look as _, ctrl_wheel_area, list_row},
    unity::{ChangeTarget, GraphEdge, GraphNode, NodeKind, Port, PortType, Shown, State},
};

const BACKGROUND: [u8; 3] = [0x16, 0x16, 0x16];
const GRID: [u8; 4] = [255, 255, 255, 9];
const SAME_EDGE: [u8; 4] = [0x8a, 0x8a, 0x8a, 255];
const RED: [u8; 4] = [0xf8, 0x71, 0x71, 255];
const GREEN: [u8; 4] = [0x4a, 0xde, 0x80, 255];
const YELLOW: [u8; 4] = [0xfa, 0xcc, 0x15, 255];
const BLUE: [u8; 4] = [0x60, 0xa5, 0xfa, 255];
const SIDE_WIDTH: f32 = 340.;
const SIDE_PROPERTY: f32 = 96.;
const GRID_STEP: f64 = 20.;
const HEADER: f64 = 26.;
const PORT_ROW: f64 = 20.;
const LINE_HEIGHT_FOR_WHEEL: f32 = 20.;
const READABLE: f64 = 0.85;
const FOCUS_MARGIN: f64 = 40.;
const UNIT_TEXT: f32 = 11.;
const SMALLEST_TEXT: f32 = 5.5;
const PORTS_FROM: f32 = 0.5;
const ELSEWHERE: f32 = 0.45;

#[derive(Default)]
pub(crate) struct GraphState {
    viewport: Option<Viewport>,
    pub(super) sheet: Sheet,
    bounds: Option<Bounds<Pixels>>,
    mapping: Option<Mapping>,
    dragging: Option<(f64, f64)>,
    generation: u64,
    focused: Option<usize>,
}

impl GraphState {
    fn viewport(&mut self) -> &mut Viewport {
        self.viewport.get_or_insert_with(|| Viewport::new(false))
    }

    pub(crate) fn forget(&mut self) {
        self.generation += 1;
    }

    pub(crate) fn percent(&self) -> Option<f64> {
        let viewport = self.viewport.unwrap_or(Viewport::new(false));
        self.mapping.map(|mapping| viewport.percent(&mapping, 1.))
    }

    pub(crate) fn fits(&self) -> bool {
        self.viewport.is_none_or(|viewport| viewport.fits())
    }

    pub(crate) fn zoom(&mut self, steps: f64) {
        if let Some(mapping) = self.mapping {
            let zoomed = self.viewport().zoom_by(&mapping, steps, None);
            self.mapping = Some(zoomed);
        }
    }

    pub(crate) fn fit(&mut self) {
        self.viewport().fit();
    }
}

fn port_color(kind: PortType) -> [u8; 4] {
    match kind {
        PortType::Flow => [0xe5, 0xe5, 0xe5, 255],
        PortType::Float => [0x60, 0xa5, 0xfa, 255],
        PortType::Integer => [0x22, 0xd3, 0xee, 255],
        PortType::Bool => [0xf4, 0x72, 0xb6, 255],
        PortType::Vector => [0xfb, 0x92, 0x3c, 255],
        PortType::Text => [0xfd, 0xe0, 0x47, 255],
        PortType::Object => [0xa3, 0xa3, 0xa3, 255],
    }
}

fn hsla_of(color: [u8; 4]) -> Hsla {
    rgb((u32::from(color[0]) << 16) | (u32::from(color[1]) << 8) | u32::from(color[2])).into()
}

fn content(compare: &UnityCompare) -> Option<Area> {
    Area::around(
        compare
            .drawn
            .nodes
            .iter()
            .filter(|node| !compare.shown.hides(node.state))
            .flat_map(|node| [node.at, (node.at.0 + node.size.0, node.at.1 + node.size.1)]),
    )
}

fn node_area(node: &GraphNode) -> Area {
    Area {
        left: node.at.0,
        top: node.at.1,
        right: node.at.0 + node.size.0,
        bottom: node.at.1 + node.size.1,
    }
}

fn change_area(compare: &UnityCompare) -> Option<Area> {
    let find = |id: &str| compare.drawn.nodes.iter().find(|node| node.id == id).map(node_area);
    let area = match picked_target(compare)? {
        ChangeTarget::Node(ix) => compare.drawn.nodes.get(ix).map(node_area)?,
        ChangeTarget::Edge(ix) => {
            let edge = compare.drawn.edges.get(ix)?;
            find(&edge.from)?.joined(find(&edge.to)?)
        }
        ChangeTarget::Parameter => return None,
    };
    Some(area.grown(FOCUS_MARGIN))
}

fn picked_target(compare: &UnityCompare) -> Option<ChangeTarget> {
    compare.graph.changes.get(compare.current).map(|change| change.target)
}

fn node_center(node: &GraphNode) -> (f64, f64) {
    (node.at.0 + node.size.0 / 2., node.at.1 + node.size.1 / 2.)
}

fn edge_stroke(edge: &GraphEdge, picked: bool, solo: bool, device: f64) -> Stroke {
    let width = if picked { 3. } else if edge.kind == PortType::Flow { 2. } else { 1.6 } * device as f32;
    match edge.state {
        State::Added => Stroke::solid(width, GREEN),
        State::Removed if solo => Stroke::solid(width, RED),
        State::Removed => Stroke::dashed(width, RED, (5. * device) as f32, (4. * device) as f32),
        State::Changed => Stroke::solid(width, YELLOW),
        State::Same if picked => Stroke::solid(width, BLUE),
        State::Same => Stroke::solid(width, if edge.from_port.is_some() { port_color(edge.kind) } else { SAME_EDGE }),
    }
}

fn draw(compare: &UnityCompare, sketch: &mut Sketch, mapping: &Mapping, device: f64) {
    let step = if GRID_STEP * mapping.scale >= 8. { GRID_STEP } else { GRID_STEP * 5. };
    let origin = mapping.world((0., 0.));
    let end = mapping.world(mapping.size);
    let mut x = (origin.0 / step).floor() * step;
    while x <= end.0 {
        let sx = (mapping.screen((x, 0.)).0 * device) as f32;
        sketch.line((sx, 0.), (sx, sketch.height() as f32), Stroke::solid(device as f32, GRID));
        x += step;
    }
    let mut y = (origin.1 / step).floor() * step;
    while y <= end.1 {
        let sy = (mapping.screen((0., y)).1 * device) as f32;
        sketch.line((0., sy), (sketch.width() as f32, sy), Stroke::solid(device as f32, GRID));
        y += step;
    }
    let picked = picked_target(compare);
    let shown = compare.shown;
    let nodes = &compare.drawn.nodes;
    let find = |id: &str| nodes.iter().find(|node| node.id == id).filter(|node| !shown.hides(node.state));
    let device_point = |(x, y): (f64, f64)| ((x * device) as f32, (y * device) as f32);
    for (ix, edge) in compare.drawn.edges.iter().enumerate() {
        if shown.hides(edge.state) {
            continue;
        }
        let (Some(from), Some(to)) = (find(&edge.from), find(&edge.to)) else {
            continue;
        };
        let stroke = edge_stroke(edge, picked == Some(ChangeTarget::Edge(ix)), shown != Shown::Both, device);
        if compare.graph.has_ports {
            let start = mapping.screen((from.at.0 + from.size.0, from.output_y(edge.from_port.as_deref().unwrap_or_default())));
            let finish = mapping.screen((to.at.0, to.input_y(edge.to_port.as_deref().unwrap_or_default())));
            let bend = if finish.0 > start.0 { ((finish.0 - start.0) / 2.).max(30. * mapping.scale) } else { 26. * mapping.scale };
            sketch.cubic(
                device_point(start),
                device_point((start.0 + bend, start.1)),
                device_point((finish.0 - bend, finish.1)),
                device_point(finish),
                stroke,
            );
            continue;
        }
        if from.id == to.id {
            continue;
        }
        let (a, b) = (mapping.screen(node_center(from)), mapping.screen(node_center(to)));
        let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(1e-6);
        let unit = ((b.0 - a.0) / length, (b.1 - a.1) / length);
        let both_ways = compare
            .drawn
            .edges
            .iter()
            .any(|other| other.from == edge.to && other.to == edge.from && !shown.hides(other.state));
        let shift = if both_ways { 5. } else { 0. };
        let normal = (-unit.1 * shift, unit.0 * shift);
        let (a, b) = ((a.0 + normal.0, a.1 + normal.1), (b.0 + normal.0, b.1 + normal.1));
        sketch.line(device_point(a), device_point(b), stroke);
        let middle = ((a.0 + b.0) / 2., (a.1 + b.1) / 2.);
        let tip = (middle.0 + unit.0 * 7., middle.1 + unit.1 * 7.);
        let back = (middle.0 - unit.0 * 5., middle.1 - unit.1 * 5.);
        let side = (-unit.1 * 5.5, unit.0 * 5.5);
        sketch.triangle(
            [device_point(tip), device_point((back.0 + side.0, back.1 + side.1)), device_point((back.0 - side.0, back.1 - side.1))],
            stroke.color,
        );
    }
}

fn prepare(compare: &mut UnityCompare, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<UnityCompare>) -> Option<(Bounds<Pixels>, Arc<RenderImage>)> {
    let size = (f64::from(f32::from(bounds.size.width)), f64::from(f32::from(bounds.size.height)));
    let area = content(compare);
    let mut mapping = compare.graph_view.viewport().mapping(area, size);
    if compare.graph_view.focused != Some(compare.current) {
        compare.graph_view.focused = Some(compare.current);
        if let Some(target) = change_area(compare)
            && (mapping.scale < READABLE || !mapping.shows(target))
        {
            mapping = compare.graph_view.viewport().focus(&mapping, target, READABLE);
        }
    }
    let changed = compare.graph_view.mapping != Some(mapping) || compare.graph_view.bounds != Some(bounds);
    compare.graph_view.mapping = Some(mapping);
    compare.graph_view.bounds = Some(bounds);
    if changed {
        window.request_animation_frame();
        cx.notify();
    }
    let device = f64::from(window.scale_factor());
    let stamp = mapping.stamp() ^ compare.graph_view.generation.rotate_left(9) ^ (compare.current as u64).rotate_left(37);
    let mut sheet = std::mem::take(&mut compare.graph_view.sheet);
    let image = sheet.image(bounds, stamp, BACKGROUND, window, |sketch, _| draw(compare, sketch, &mapping, device));
    compare.graph_view.sheet = sheet;
    image.map(|image| (bounds, image))
}

fn node_fill(kind: NodeKind) -> u32 {
    match kind {
        NodeKind::Entry => 0x166534,
        NodeKind::AnyState => 0x115e59,
        NodeKind::Exit => 0x7f1d1d,
        NodeKind::DefaultState => 0xb45309,
        NodeKind::State | NodeKind::Machine => 0x3a3a42,
        NodeKind::Event => 0x2f4a1c,
        NodeKind::Variable => 0x2e2a4f,
        NodeKind::Value => 0x1f3347,
        NodeKind::Flow => 0x33333a,
    }
}

fn port_glyph(port: &Port, scale: f32, cx: &App) -> AnyElement {
    let size = (9. * scale).clamp(4., 14.);
    let color = hsla_of(port_color(port.kind));
    if port.kind == PortType::Flow {
        Icon::new(IconName::Play).size(px(size + 2.)).text_color(color).into_any_element()
    } else {
        div()
            .flex_none()
            .size(px(size))
            .rounded_full()
            .border_2()
            .border_color(color)
            .bg(cx.theme().background)
            .into_any_element()
    }
}

fn port_row(input: Option<&Port>, output: Option<&Port>, scale: f32, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let height = (PORT_ROW as f32 * scale).max(1.);
    let left = input.map(|port| {
        h_flex()
            .gap(px(4. * scale))
            .ml(px(-5. * scale))
            .child(port_glyph(port, scale, cx))
            .when(!port.label.is_empty(), |this| this.child(div().text_color(theme.muted_foreground).child(port.label.clone())))
            .when_some(port.literal.clone(), |this, literal| {
                let changed = port.was.is_some();
                this.child(
                    div()
                        .px(px(4. * scale))
                        .rounded(px(3.))
                        .when(changed, |this| this.bg(theme.yellow).text_color(gpui_kit::black()))
                        .when(!changed, |this| this.bg(gpui_kit::white().opacity(0.07)))
                        .child(literal),
                )
            })
    });
    let right = output.map(|port| {
        h_flex()
            .gap(px(4. * scale))
            .mr(px(-5. * scale))
            .when(!port.label.is_empty(), |this| this.child(div().text_color(theme.muted_foreground).child(port.label.clone())))
            .child(port_glyph(port, scale, cx))
    });
    h_flex()
        .h(px(height))
        .w_full()
        .justify_between()
        .child(left.map_or_else(|| div().into_any_element(), IntoElement::into_any_element))
        .child(right.map_or_else(|| div().into_any_element(), IntoElement::into_any_element))
        .into_any_element()
}

fn nodes(compare: &UnityCompare, cx: &mut Context<UnityCompare>) -> Vec<AnyElement> {
    let Some(mapping) = compare.graph_view.mapping else {
        return Vec::new();
    };
    let theme = cx.theme();
    let scale = mapping.scale as f32;
    let text = UNIT_TEXT * scale;
    let titled = text >= SMALLEST_TEXT;
    let picked = picked_target(compare);
    let solo = compare.shown != Shown::Both;
    let mut order: Vec<(usize, &GraphNode)> = compare.drawn.nodes.iter().enumerate().filter(|(_, node)| !compare.shown.hides(node.state)).collect();
    order.sort_by_key(|(_, node)| node.state != State::Removed);
    order
        .into_iter()
        .map(|(ix, node)| {
            let (left, top) = mapping.screen(node.at);
            let width = (node.size.0 * mapping.scale) as f32;
            let height = (node.size.1 * mapping.scale) as f32;
            let ring = match node.state {
                State::Added => Some(theme.green),
                State::Changed => Some(theme.yellow),
                _ => None,
            };
            let change = compare.graph.changes.iter().position(|change| change.target == ChangeTarget::Node(ix));
            let is_unit = compare.graph.has_ports;
            let base = v_flex()
                .id(("unity-node", ix))
                .absolute()
                .left(px(left as f32))
                .top(px(top as f32))
                .w(px(width))
                .h(px(height))
                .rounded(px(6. * scale.min(1.)))
                .border_1()
                .border_color(if node.state == State::Removed { theme.red } else { gpui_kit::white().opacity(0.18) })
                .when(node.state == State::Removed && !solo, |this| this.opacity(0.6))
                .when_some(ring, |this, color| this.border_2().border_color(color))
                .when(picked == Some(ChangeTarget::Node(ix)), |this| this.shadow_md().border_2().border_color(theme.blue))
                .text_size(px(text))
                .line_height(px(text * 1.3))
                .when_some(change, |this, change| this.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.choose_change(change, cx))));
            let title = h_flex()
                .gap(px(5. * scale))
                .min_w_0()
                .children(state_badge(node.state, cx).filter(|_| scale > 0.8))
                .when(titled, |this| {
                    this.child(
                        div()
                            .ellipsis()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .when(node.state == State::Removed && !solo, |this| this.line_through())
                            .child(node.title.clone()),
                    )
                });
            if !is_unit {
                return base
                    .bg(rgb(node_fill(node.kind)))
                    .items_center()
                    .justify_center()
                    .child(title)
                    .into_any_element();
            }
            let rows = (0..node.rows()).map(|row| port_row(node.inputs.get(row), node.outputs.get(row), scale, cx));
            base.bg(rgb(0x26262b))
                .child(
                    h_flex()
                        .flex_none()
                        .h(px((HEADER * mapping.scale) as f32))
                        .px(px(8. * scale))
                        .rounded_t(px(5. * scale.min(1.)))
                        .bg(rgb(node_fill(node.kind)))
                        .child(title),
                )
                .when(scale >= PORTS_FROM, |this| this.children(rows))
                .into_any_element()
        })
        .collect()
}

fn parameters(compare: &UnityCompare, cx: &App) -> Option<AnyElement> {
    let shown: Vec<&(String, String, State)> = compare.drawn.parameters.iter().filter(|(_, _, state)| !compare.shown.hides(*state)).collect();
    if shown.is_empty() {
        return None;
    }
    let theme = cx.theme();
    let right = compare.graph.has_ports;
    Some(
        v_flex()
            .absolute()
            .bottom_2p5()
            .when(right, |this| this.right_2p5())
            .when(!right, |this| this.left_2p5())
            .w(px(220.))
            .px_2p5()
            .py_2()
            .gap_0p5()
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover.opacity(0.92))
            .child(div().xs_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(compare.graph.parameters_title))
            .children(shown.into_iter().map(|(name, kind, state)| {
                h_flex()
                    .gap_1p5()
                    .xs_text()
                    .child(state_badge(*state, cx).unwrap_or_else(|| div().flex_none().w(px(18.)).into_any_element()))
                    .child(div().flex_1().ellipsis().when(*state == State::Removed, |this| this.line_through()).child(name.clone()))
                    .child(div().text_color(theme.muted_foreground).child(kind.clone()))
            }))
            .into_any_element(),
    )
}

fn layer_picker(compare: &UnityCompare, cx: &mut Context<UnityCompare>) -> Option<AnyElement> {
    let layers = compare.layers().to_vec();
    if layers.is_empty() {
        return None;
    }
    let current = compare.layer();
    let entity = cx.entity();
    Some(
        div()
            .absolute()
            .top_2p5()
            .left_2p5()
            .child(
                Button::new("unity-layer")
                    .small()
                    .icon(IconName::Workflow)
                    .label(SharedString::from(layers.get(current).cloned().unwrap_or_default()))
                    .tooltip("The layer shown")
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
                        layers.iter().enumerate().fold(menu, |menu, (ix, name)| {
                            let entity = entity.clone();
                            menu.item(
                                PopupMenuItem::new(SharedString::from(name.clone()))
                                    .checked(ix == current)
                                    .on_click(move |_, _, cx| entity.update(cx, |this, cx| this.set_layer(ix, cx))),
                            )
                        })
                    }),
            )
            .into_any_element(),
    )
}

fn side_panel(compare: &UnityCompare, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let rows: Vec<AnyElement> = compare
        .graph
        .changes
        .iter()
        .enumerate()
        .map(|(ix, change)| {
            list_row(("unity-graph-change", ix), ix == compare.current)
                .px_3()
                .when(compare.shown.hides(change.state), |this| this.opacity(ELSEWHERE))
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap_2()
                        .children(state_badge(change.state, cx))
                        .child(div().flex_none().w(px(66.)).xs_text().text_color(theme.muted_foreground).child(change.kind))
                        .child(div().flex_1().ellipsis().sm_text().child(change.name.clone()))
                        .child(div().flex_none().xs_text().text_color(theme.muted_foreground).child(change.detail.clone())),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.choose_change(ix, cx)))
                .into_any_element()
        })
        .collect();
    let picked = compare.graph.changes.get(compare.current);
    let head = |title: SharedString, note: SharedString| {
        h_flex()
            .flex_none()
            .h(px(32.))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_1().ellipsis().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(title))
            .child(div().xs_text().text_color(theme.muted_foreground).child(note))
    };
    let settings = picked.map(|change| {
        v_flex()
            .flex_1()
            .min_h_0()
            .child(head(change.name.clone().into(), change.kind.into()))
            .when(!change.rows.is_empty(), |this| {
                this.child(sides_row(&[SharedString::default(), SharedString::default()], SIDE_PROPERTY, cx)).child(
                    div()
                        .id("unity-graph-settings")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .children(change.rows.iter().map(|row| property_row(row, SIDE_PROPERTY, 12., false, cx))),
                )
            })
    });
    v_flex()
        .flex_none()
        .w(px(SIDE_WIDTH))
        .h_full()
        .overflow_hidden()
        .border_l_1()
        .border_color(theme.border)
        .child(head("Changes".into(), compare.graph.changes.len().to_string().into()))
        .child(div().id("unity-graph-changes").min_h_0().max_h(gpui_kit::relative(0.5)).overflow_y_scroll().track_scroll(&compare.changes_scroll).children(rows))
        .children(settings)
        .into_any_element()
}

fn press(compare: &mut UnityCompare, event: &MouseDownEvent) {
    if let Some(bounds) = compare.graph_view.bounds {
        compare.graph_view.dragging = Some((
            f64::from(f32::from(event.position.x - bounds.origin.x)),
            f64::from(f32::from(event.position.y - bounds.origin.y)),
        ));
    }
}

fn drag(compare: &mut UnityCompare, event: &MouseMoveEvent, cx: &mut Context<UnityCompare>) {
    let (Some((from_x, from_y)), Some(bounds), Some(mapping)) = (compare.graph_view.dragging, compare.graph_view.bounds, compare.graph_view.mapping) else {
        return;
    };
    if !event.dragging() {
        compare.graph_view.dragging = None;
        return;
    }
    let x = f64::from(f32::from(event.position.x - bounds.origin.x));
    let y = f64::from(f32::from(event.position.y - bounds.origin.y));
    let moved = compare.graph_view.viewport().pan_by(&mapping, x - from_x, y - from_y);
    compare.graph_view.mapping = Some(moved);
    compare.graph_view.dragging = Some((x, y));
    cx.notify();
}

fn wheel_zoom(compare: &mut UnityCompare, event: &ScrollEvent, cx: &mut Context<UnityCompare>) -> bool {
    let (Some(bounds), Some(mapping)) = (compare.graph_view.bounds, compare.graph_view.mapping) else {
        return false;
    };
    let delta = event.delta.pixel_delta(px(LINE_HEIGHT_FOR_WHEEL));
    let at = (
        f64::from(f32::from(event.position.x - bounds.origin.x)),
        f64::from(f32::from(event.position.y - bounds.origin.y)),
    );
    let zoomed = compare.graph_view.viewport().zoom_by(&mapping, if f32::from(delta.y) > 0. { 1. } else { -1. }, Some(at));
    compare.graph_view.mapping = Some(zoomed);
    cx.notify();
    true
}

pub(crate) fn version_picker(view: &Entity<UnityCompare>, shown: Shown, cx: &App) -> AnyElement {
    let choice = |id: &'static str, choice: Shown, version: Option<Version>, tooltip: &'static str| {
        let view = view.clone();
        Button::new(id)
            .ghost()
            .xsmall()
            .selected(shown == choice)
            .child(
                h_flex()
                    .gap_1p5()
                    .children(version.map(|version| version_dot(version, cx)))
                    .child(version.map_or("Both", Version::label)),
            )
            .tooltip(tooltip)
            .on_click(move |_, _, cx| view.update(cx, |view, cx| view.show(choice, cx)))
    };
    h_flex()
        .flex_none()
        .gap_0p5()
        .child(div().mr_1p5().xs_text().text_color(cx.theme().muted_foreground).child("Show"))
        .child(choice("graph-show-before", Shown::Before, Some(Version::Before), "Draw the graph as it was before the change"))
        .child(choice("graph-show-both", Shown::Both, None, "Draw both versions over each other"))
        .child(choice("graph-show-after", Shown::After, Some(Version::After), "Draw the graph as it is after the change"))
        .into_any_element()
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
    let shown = nodes(compare, cx);
    let canvas_area = div()
        .id("unity-graph")
        .relative()
        .flex_1()
        .min_w_0()
        .h_full()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, _| press(this, event)))
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| drag(this, event, cx)))
        .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.graph_view.dragging = None))
        .child(picture)
        .children(shown)
        .children(layer_picker(compare, cx))
        .children(parameters(compare, cx))
        .child(ctrl_wheel_area(cx.entity().downgrade(), wheel_zoom));
    h_flex()
        .size_full()
        .child(canvas_area)
        .child(side_panel(compare, cx))
        .into_any_element()
}
