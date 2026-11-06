use gpui_kit::{
    BorderStyle, Bounds, ContentMask, Hsla, IntoElement, ParentElement as _, PathBuilder, Pixels,
    Point, Styled as _, Window, canvas, div, fill, point, px, size, transparent_black,
};

use super::{
    palette::{LANE_COLORS, Palette},
    rows::ROW_HEIGHT,
};
use crate::git::{Edge, Row};

const LANE_WIDTH: Pixels = px(12.);
const GRAPH_MARGIN: Pixels = px(6.);
const MAX_VISIBLE_LANES: usize = 32;
const NODE_RADIUS: Pixels = px(4.5);
const MERGE_NODE_SCALE: f32 = 0.7;
const HEAD_RING_GAP: Pixels = px(3.);
const LINE_WIDTH: Pixels = px(2.);
const RING_WIDTH: Pixels = px(1.5);
const STASH_CORNER: Pixels = px(1.5);
const CHANGES_DASH: Pixels = px(3.);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphNode {
    Commit,
    Head,
    Stash,
    Changes,
}

impl GraphNode {
    fn line_leaves_below(self, middle: Pixels) -> Pixels {
        match self {
            GraphNode::Stash | GraphNode::Changes => middle + NODE_RADIUS,
            GraphNode::Commit | GraphNode::Head => middle,
        }
    }
}

pub(crate) fn graph_width<'a>(visible: impl IntoIterator<Item = &'a Row>) -> Pixels {
    let lanes = visible
        .into_iter()
        .map(|row| row.lanes as usize)
        .max()
        .unwrap_or(1)
        .clamp(1, MAX_VISIBLE_LANES);
    LANE_WIDTH * lanes as f32 + GRAPH_MARGIN * 2.
}

fn lane_x(bounds: &Bounds<Pixels>, lane: u16) -> Pixels {
    bounds.left() + GRAPH_MARGIN + LANE_WIDTH * (lane as f32 + 0.5)
}

pub(crate) fn graph_cell(
    row: Row,
    node: GraphNode,
    palette: &Palette,
    width: Pixels,
) -> impl IntoElement {
    let lanes = palette.lanes;
    cell(width, move |bounds, window| {
        paint_row(&row, node, &lanes, bounds, window)
    })
}

pub(crate) fn uncommitted_cell(color: Hsla) -> impl IntoElement {
    cell(LANE_WIDTH + GRAPH_MARGIN * 2., move |bounds, window| {
        let center = point(lane_x(&bounds, 0), bounds.center().y);
        paint_ring(window, center, NODE_RADIUS, color);
    })
}

fn cell(
    width: Pixels,
    paint: impl FnOnce(Bounds<Pixels>, &mut Window) + 'static,
) -> impl IntoElement {
    div().flex_none().w(width).h(ROW_HEIGHT).child(
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| paint(bounds, window),
        )
        .size_full(),
    )
}

fn paint_row(
    row: &Row,
    shape: GraphNode,
    lanes: &[Hsla; LANE_COLORS],
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let x = |lane: u16| lane_x(&bounds, lane);
    let color = |lane: u16| lanes[lane as usize % LANE_COLORS];
    let (top, middle, bottom) = (bounds.top(), bounds.center().y, bounds.bottom());
    let node = row.node_lane;
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for edge in &row.edges {
            let mut path = PathBuilder::stroke(LINE_WIDTH);
            let lane = match *edge {
                Edge::Pass(lane) => {
                    path.move_to(point(x(lane), top));
                    path.line_to(point(x(lane), bottom));
                    lane
                }
                Edge::In { from } => {
                    path.move_to(point(x(from), top));
                    if from == node {
                        path.line_to(point(x(node), middle));
                    } else {
                        bend_into_commit(&mut path, point(x(from), middle), point(x(node), middle));
                    }
                    from
                }
                Edge::Out { to } => {
                    let leaves = shape.line_leaves_below(middle);
                    path.move_to(point(x(node), leaves));
                    if to == node && shape == GraphNode::Changes {
                        dash_down(&mut path, point(x(node), leaves), bottom);
                    } else if to == node {
                        path.line_to(point(x(node), bottom));
                    } else {
                        bend_down_to_parent(&mut path, point(x(to), middle), point(x(to), bottom));
                    }
                    to
                }
            };
            if let Ok(path) = path.build() {
                window.paint_path(path, color(lane));
            }
        }

        let center = point(x(node), middle);
        if shape == GraphNode::Stash {
            paint_square(window, center, color(node));
            return;
        }
        if shape == GraphNode::Changes {
            paint_dashed_ring(window, center, NODE_RADIUS, color(node));
            return;
        }
        let radius = if row.is_merge {
            NODE_RADIUS * MERGE_NODE_SCALE
        } else {
            NODE_RADIUS
        };
        window.paint_quad(
            fill(
                Bounds::centered_at(center, size(radius * 2., radius * 2.)),
                color(node),
            )
            .corner_radii(radius),
        );
        if shape == GraphNode::Head {
            paint_ring(window, center, NODE_RADIUS + HEAD_RING_GAP, color(node));
        }
    });
}

fn paint_square(window: &mut Window, center: Point<Pixels>, color: Hsla) {
    window.paint_quad(
        fill(
            Bounds::centered_at(center, size(NODE_RADIUS * 2., NODE_RADIUS * 2.)),
            transparent_black(),
        )
        .corner_radii(STASH_CORNER)
        .border_widths(RING_WIDTH)
        .border_color(color),
    );
}

fn dash_down(path: &mut PathBuilder, from: Point<Pixels>, bottom: Pixels) {
    let mut top = from.y;
    while top < bottom {
        let end = top + CHANGES_DASH;
        path.move_to(point(from.x, top));
        path.line_to(point(from.x, if end < bottom { end } else { bottom }));
        top = end + CHANGES_DASH;
    }
}

fn paint_dashed_ring(window: &mut Window, center: Point<Pixels>, radius: Pixels, color: Hsla) {
    let mut ring = fill(
        Bounds::centered_at(center, size(radius * 2., radius * 2.)),
        transparent_black(),
    )
    .corner_radii(radius)
    .border_widths(RING_WIDTH)
    .border_color(color);
    ring.border_style = BorderStyle::Dashed;
    window.paint_quad(ring);
}

fn bend_into_commit(path: &mut PathBuilder, corner: Point<Pixels>, commit: Point<Pixels>) {
    path.cubic_bezier_to(commit, corner, corner);
}

fn bend_down_to_parent(path: &mut PathBuilder, corner: Point<Pixels>, lane_bottom: Point<Pixels>) {
    path.cubic_bezier_to(lane_bottom, corner, corner);
}

fn paint_ring(window: &mut Window, center: Point<Pixels>, radius: Pixels, color: Hsla) {
    window.paint_quad(
        fill(
            Bounds::centered_at(center, size(radius * 2., radius * 2.)),
            transparent_black(),
        )
        .corner_radii(radius)
        .border_widths(RING_WIDTH)
        .border_color(color),
    );
}
