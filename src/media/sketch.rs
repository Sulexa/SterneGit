use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::mem;
use std::ops::{Add, Mul, Range, Sub};

const CURVE_TOLERANCE: f32 = 0.2;
const MOST_CURVE_STEPS: f32 = 4096.0;
const SHORTEST_DASH: f32 = 0.5;
const LONGEST_DASH: f32 = 1.0e6;
const EMPTY_SPAN: (usize, usize) = (usize::MAX, 0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stroke {
    pub(crate) width: f32,
    pub(crate) color: [u8; 4],
    pub(crate) dash: Option<(f32, f32)>,
}

impl Stroke {
    pub(crate) fn solid(width: f32, color: [u8; 4]) -> Self {
        Stroke { width, color, dash: None }
    }

    pub(crate) fn dashed(width: f32, color: [u8; 4], on: f32, off: f32) -> Self {
        Stroke { width, color, dash: Some((on, off)) }
    }
}

pub(crate) struct Sketch {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    coverage: Coverage,
}

impl Sketch {
    pub(crate) fn new(width: u32, height: u32, background: [u8; 3]) -> Self {
        let count = (width as usize)
            .checked_mul(height as usize)
            .filter(|count| count.checked_mul(4).is_some());
        let Some(count) = count else {
            return Sketch { width: 0, height: 0, pixels: Vec::new(), coverage: Coverage::default() };
        };
        let [red, green, blue] = background;
        let pixels = [blue, green, red, 255].repeat(count);
        Sketch { width, height, pixels, coverage: Coverage::default() }
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn fill_rect(&mut self, left: f32, top: f32, width: f32, height: f32, color: [u8; 4]) {
        let placed = [left, top, width, height].iter().all(|value| value.is_finite());
        if !placed || !(width > 0.0 && height > 0.0) || color[3] == 0 {
            return;
        }
        let (right, bottom) = (left + width, top + height);
        let columns: Vec<(usize, f32)> =
            pixel_span(left, right, self.width).map(|x| (x, overlap(x, left, right))).collect();
        for y in pixel_span(top, bottom, self.height) {
            let row_cover = alpha_of(color) * overlap(y, top, bottom);
            for &(x, column_cover) in &columns {
                self.blend(x, y, rgb_of(color), row_cover * column_cover);
            }
        }
    }

    pub(crate) fn line(&mut self, from: (f32, f32), to: (f32, f32), stroke: Stroke) {
        self.stroke_path(&[Point::from(from), Point::from(to)], false, stroke);
    }

    pub(crate) fn polyline(&mut self, points: &[(f32, f32)], stroke: Stroke) {
        let path: Vec<Point> = points.iter().copied().map(Point::from).collect();
        self.stroke_path(&path, false, stroke);
    }

    pub(crate) fn cubic(
        &mut self,
        from: (f32, f32),
        first: (f32, f32),
        second: (f32, f32),
        to: (f32, f32),
        stroke: Stroke,
    ) {
        let controls = [from, first, second, to].map(Point::from);
        if controls.iter().all(|control| control.is_finite()) {
            self.stroke_path(&cubic_path(controls), false, stroke);
        }
    }

    pub(crate) fn circle(&mut self, center: (f32, f32), radius: f32, stroke: Stroke) {
        let center = Point::from(center);
        if center.is_finite() && radius > 0.0 && radius.is_finite() {
            self.stroke_path(&circle_path(center, radius), true, stroke);
        }
    }

    pub(crate) fn rect(
        &mut self,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
        corner: f32,
        stroke: Stroke,
    ) {
        if width > 0.0 && height > 0.0 {
            self.stroke_path(&rounded_rect_path(left, top, width, height, corner), true, stroke);
        }
    }

    pub(crate) fn diamond(
        &mut self,
        center: (f32, f32),
        half: f32,
        fill: [u8; 4],
        outline: Option<([u8; 4], f32)>,
    ) {
        if !(half > 0.0) {
            return;
        }
        let Point { x, y } = Point::from(center);
        let corners = [
            Point::new(x, y - half),
            Point::new(x + half, y),
            Point::new(x, y + half),
            Point::new(x - half, y),
        ];
        self.fill_convex(&corners, fill);
        if let Some((color, width)) = outline {
            let path = [corners[0], corners[1], corners[2], corners[3], corners[0]];
            self.stroke_path(&path, true, Stroke::solid(width, color));
        }
    }

    pub(crate) fn triangle(&mut self, corners: [(f32, f32); 3], fill: [u8; 4]) {
        self.fill_convex(&corners.map(Point::from), fill);
    }

    pub(crate) fn image(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        at: (f32, f32),
        scale: (f32, f32),
        opacity: f32,
    ) {
        self.tinted_image(rgba, width, height, at, scale, [1.0, 1.0, 1.0, opacity]);
    }

    pub(crate) fn tinted_image(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        at: (f32, f32),
        scale: (f32, f32),
        tint: [f32; 4],
    ) {
        let opacity = tint[3];
        let (columns, rows) = (width as usize, height as usize);
        let needed = columns.checked_mul(rows).and_then(|count| count.checked_mul(4));
        let fits = needed.is_some_and(|needed| needed > 0 && rgba.len() >= needed);
        let placed = [at.0, at.1, scale.0, scale.1].iter().all(|value| value.is_finite());
        if !fits || !placed || !(scale.0 > 0.0 && scale.1 > 0.0 && opacity > 0.0) {
            return;
        }
        let strength = opacity.min(1.0) / 255.0;
        let across = samples(at.0, scale.0, columns, self.width);
        for (y, row) in samples(at.1, scale.1, rows, self.height) {
            for &(x, column) in &across {
                let texel = &rgba[(row * columns + column) * 4..][..4];
                let shade = |channel: usize| (f32::from(texel[channel]) * tint[channel].clamp(0.0, 1.0)) as u8;
                self.blend(x, y, [shade(0), shade(1), shade(2)], texel[3] as f32 * strength);
            }
        }
    }

    pub(crate) fn into_bgra(self) -> Vec<u8> {
        self.pixels
    }

    fn stroke_path(&mut self, path: &[Point], closed: bool, stroke: Stroke) {
        let size = (self.width as usize, self.height as usize);
        if size.0 == 0 || size.1 == 0 || path.len() < 2 || !path.iter().all(|point| point.is_finite()) {
            return;
        }
        let (Some(pen), Some(pattern)) = (Pen::of(stroke, size), Pattern::of(stroke.dash)) else {
            return;
        };
        let runs = trace(path, closed, pattern, Bounds::around(size, pen.reach));
        if runs.is_empty() {
            return;
        }
        self.coverage.ensure(size.0, size.1);
        for run in &runs {
            for piece in run.pieces() {
                self.coverage.cover(piece, pen);
            }
        }
        self.coverage.paint(&mut self.pixels, stroke.color);
    }

    fn fill_convex(&mut self, corners: &[Point], color: [u8; 4]) {
        if color[3] == 0 || !corners.iter().all(|corner| corner.is_finite()) {
            return;
        }
        let Some(edges) = convex_edges(corners) else {
            return;
        };
        let low = corners.iter().fold(corners[0], |low, &corner| low.min(corner));
        let high = corners.iter().fold(corners[0], |high, &corner| high.max(corner));
        let columns = pixel_span(low.x - 1.0, high.x + 1.0, self.width);
        for y in pixel_span(low.y - 1.0, high.y + 1.0, self.height) {
            for x in columns.clone() {
                let center = Point::new(x as f32 + 0.5, y as f32 + 0.5);
                let outside = edges.iter().map(|edge| edge.distance(center)).fold(f32::MIN, f32::max);
                self.blend(x, y, rgb_of(color), alpha_of(color) * (0.5 - outside).clamp(0.0, 1.0));
            }
        }
    }

    fn blend(&mut self, x: usize, y: usize, rgb: [u8; 3], alpha: f32) {
        let index = (y * self.width as usize + x) * 4;
        blend_into(&mut self.pixels[index..index + 4], rgb, alpha);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    fn dot(self, other: Point) -> f32 {
        self.x * other.x + self.y * other.y
    }

    fn cross(self, other: Point) -> f32 {
        self.x * other.y - self.y * other.x
    }

    fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    fn lerp(self, other: Point, share: f32) -> Point {
        if share == 1.0 { other } else { self + (other - self) * share }
    }

    fn min(self, other: Point) -> Point {
        Point::new(self.x.min(other.x), self.y.min(other.y))
    }

    fn max(self, other: Point) -> Point {
        Point::new(self.x.max(other.x), self.y.max(other.y))
    }
}

impl From<(f32, f32)> for Point {
    fn from((x, y): (f32, f32)) -> Self {
        Point { x, y }
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Point {
    type Output = Point;

    fn mul(self, factor: f32) -> Point {
        Point::new(self.x * factor, self.y * factor)
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Bounds {
    fn around((width, height): (usize, usize), margin: f32) -> Self {
        Bounds { left: -margin, top: -margin, right: width as f32 + margin, bottom: height as f32 + margin }
    }

    fn clip(self, from: Point, to: Point) -> Option<(f32, f32)> {
        let delta = to - from;
        let limits = [
            (-delta.x, from.x - self.left),
            (delta.x, self.right - from.x),
            (-delta.y, from.y - self.top),
            (delta.y, self.bottom - from.y),
        ];
        let (mut enter, mut leave) = (0.0f32, 1.0f32);
        for (direction, room) in limits {
            if direction == 0.0 {
                if room < 0.0 {
                    return None;
                }
            } else if direction < 0.0 {
                enter = enter.max(room / direction);
            } else {
                leave = leave.min(room / direction);
            }
        }
        (enter <= leave).then_some((enter, leave))
    }

    fn hold(self, point: Point) -> Point {
        Point::new(point.x.clamp(self.left, self.right), point.y.clamp(self.top, self.bottom))
    }
}

#[derive(Clone, Copy)]
struct Pen {
    half: f32,
    strength: f32,
    reach: f32,
}

impl Pen {
    fn of(stroke: Stroke, (width, height): (usize, usize)) -> Option<Pen> {
        if !(stroke.width > 0.0) || stroke.color[3] == 0 {
            return None;
        }
        let widest = 4.0 * (width + height) as f32 + 16.0;
        let thickness = stroke.width.clamp(1.0, widest);
        Some(Pen { half: thickness / 2.0, strength: stroke.width.min(1.0), reach: thickness / 2.0 + 1.0 })
    }

    fn cover(self, distance: f32) -> f32 {
        (self.half + 0.5 - distance).clamp(0.0, 1.0) * self.strength
    }
}

#[derive(Clone, Copy)]
enum Pattern {
    Solid,
    Dashed { on: f32, off: f32 },
}

impl Pattern {
    fn of(dash: Option<(f32, f32)>) -> Option<Pattern> {
        let Some((on, off)) = dash else {
            return Some(Pattern::Solid);
        };
        if !(on > 0.0) {
            return None;
        }
        if !(off > 0.0) || on == f32::INFINITY {
            return Some(Pattern::Solid);
        }
        Some(Pattern::Dashed {
            on: on.clamp(SHORTEST_DASH, LONGEST_DASH),
            off: off.clamp(SHORTEST_DASH, LONGEST_DASH),
        })
    }
}

#[derive(Clone, Copy)]
struct Piece {
    from: Point,
    to: Point,
    round_from: bool,
    round_to: bool,
}

struct Run {
    points: Vec<Point>,
    from_start: bool,
    looped: bool,
}

impl Run {
    fn pieces(&self) -> impl Iterator<Item = Piece> {
        let last = self.points.len() - 2;
        self.points.windows(2).enumerate().map(move |(index, pair)| Piece {
            from: pair[0],
            to: pair[1],
            round_from: self.looped || index > 0,
            round_to: self.looped || index < last,
        })
    }
}

fn trace(path: &[Point], closed: bool, pattern: Pattern, visible: Bounds) -> Vec<Run> {
    let mut tracer = Tracer::new(pattern);
    for pair in path.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let length = (to - from).length();
        if !(length > 0.0 && length.is_finite()) {
            continue;
        }
        match visible.clip(from, to) {
            None => tracer.skip(length),
            Some((enter, leave)) => {
                tracer.skip(enter * length);
                tracer.draw(visible.hold(from.lerp(to, enter)), visible.hold(from.lerp(to, leave)));
                tracer.skip((1.0 - leave) * length);
            }
        }
    }
    tracer.finish(closed)
}

struct Tracer {
    pattern: Pattern,
    on: bool,
    left: f32,
    moved: bool,
    current: Vec<Point>,
    current_from_start: bool,
    runs: Vec<Run>,
}

impl Tracer {
    fn new(pattern: Pattern) -> Self {
        let left = match pattern {
            Pattern::Solid => f32::INFINITY,
            Pattern::Dashed { on, .. } => on,
        };
        Tracer {
            pattern,
            on: true,
            left,
            moved: false,
            current: Vec::new(),
            current_from_start: false,
            runs: Vec::new(),
        }
    }

    fn skip(&mut self, length: f32) {
        if !(length > 0.0) {
            return;
        }
        self.moved = true;
        self.end_run();
        if let Pattern::Dashed { on, off } = self.pattern {
            let period = on + off;
            let phase = if self.on { on - self.left } else { period - self.left };
            let phase = (phase + length).rem_euclid(period);
            self.on = phase < on;
            self.left = if self.on { on - phase } else { period - phase };
            if !(self.left > 0.0) {
                self.toggle();
            }
        }
    }

    fn draw(&mut self, from: Point, to: Point) {
        let length = (to - from).length();
        if !(length > 0.0) {
            return;
        }
        let mut travelled = 0.0;
        while length - travelled > 0.0 {
            let start = from.lerp(to, travelled / length);
            if self.left > length - travelled {
                if self.on {
                    self.extend(start, to);
                }
                self.left -= length - travelled;
                break;
            }
            travelled += self.left;
            if self.on {
                self.extend(start, from.lerp(to, travelled / length));
            }
            self.toggle();
        }
        self.moved = true;
    }

    fn toggle(&mut self) {
        if let Pattern::Dashed { on, off } = self.pattern {
            if self.on {
                self.end_run();
            }
            self.on = !self.on;
            self.left = if self.on { on } else { off };
        }
    }

    fn extend(&mut self, start: Point, end: Point) {
        if self.current.is_empty() {
            self.current_from_start = !self.moved;
            self.current.push(start);
        }
        if self.current.last() != Some(&end) {
            self.current.push(end);
        }
        self.moved = true;
    }

    fn end_run(&mut self) {
        let points = mem::take(&mut self.current);
        if points.len() >= 2 {
            self.runs.push(Run { points, from_start: self.current_from_start, looped: false });
        }
    }

    fn finish(mut self, closed: bool) -> Vec<Run> {
        let mut tail = mem::take(&mut self.current);
        if tail.len() < 2 {
            return self.runs;
        }
        if closed && self.current_from_start {
            self.runs.push(Run { points: tail, from_start: true, looped: true });
        } else if closed && self.runs.first().is_some_and(|head| head.from_start) {
            let head = &mut self.runs[0];
            let skip_joint = usize::from(head.points.first() == tail.last());
            tail.extend_from_slice(&head.points[skip_joint..]);
            head.points = tail;
            head.from_start = false;
        } else {
            self.runs.push(Run { points: tail, from_start: self.current_from_start, looped: false });
        }
        self.runs
    }
}

#[derive(Default)]
struct Coverage {
    width: usize,
    height: usize,
    levels: Vec<u8>,
    spans: Vec<(usize, usize)>,
    rows: Option<(usize, usize)>,
}

impl Coverage {
    fn ensure(&mut self, width: usize, height: usize) {
        if self.width != width || self.height != height || self.levels.len() != width * height {
            *self = Coverage {
                width,
                height,
                levels: vec![0; width * height],
                spans: vec![EMPTY_SPAN; height],
                rows: None,
            };
        }
    }

    fn cover(&mut self, piece: Piece, pen: Pen) {
        let Some(footprint) = Footprint::of(piece, pen) else {
            return;
        };
        let (from, to) = (piece.from, piece.to);
        let top = clamp_index((from.y.min(to.y) - pen.reach).floor(), self.height);
        let bottom = clamp_index((from.y.max(to.y) + pen.reach).ceil(), self.height);
        for y in top..bottom {
            let center_y = y as f32 + 0.5;
            let Some((left, right)) = row_reach(from, to, center_y, pen.reach) else {
                continue;
            };
            let start = clamp_index(left.floor(), self.width);
            let end = clamp_index(right.ceil(), self.width);
            if start >= end {
                continue;
            }
            for (offset, level) in self.row(y, start, end).iter_mut().enumerate() {
                let center = Point::new((start + offset) as f32 + 0.5, center_y);
                *level = (*level).max((footprint.cover(center) * 255.0 + 0.5) as u8);
            }
        }
    }

    fn row(&mut self, y: usize, start: usize, end: usize) -> &mut [u8] {
        let span = &mut self.spans[y];
        *span = (span.0.min(start), span.1.max(end));
        self.rows = Some(self.rows.map_or((y, y), |(top, bottom)| (top.min(y), bottom.max(y))));
        let offset = y * self.width;
        &mut self.levels[offset + start..offset + end]
    }

    fn paint(&mut self, pixels: &mut [u8], color: [u8; 4]) {
        let Some((top, bottom)) = self.rows.take() else {
            return;
        };
        let rgb = rgb_of(color);
        let strength = color[3] as f32 / 65025.0;
        for y in top..=bottom {
            let (start, end) = mem::replace(&mut self.spans[y], EMPTY_SPAN);
            if start >= end {
                continue;
            }
            for index in y * self.width + start..y * self.width + end {
                let level = mem::take(&mut self.levels[index]);
                if level > 0 {
                    blend_into(&mut pixels[index * 4..index * 4 + 4], rgb, strength * level as f32);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Footprint {
    start: Point,
    direction: Point,
    length: f32,
    round_start: bool,
    round_end: bool,
    pen: Pen,
}

impl Footprint {
    fn of(piece: Piece, pen: Pen) -> Option<Self> {
        let delta = piece.to - piece.from;
        let length = delta.length();
        let (direction, length) = if length > 1e-6 {
            (delta * (1.0 / length), length)
        } else if piece.round_from || piece.round_to {
            (Point::new(1.0, 0.0), 0.0)
        } else {
            return None;
        };
        Some(Footprint {
            start: piece.from,
            direction,
            length,
            round_start: piece.round_from,
            round_end: piece.round_to,
            pen,
        })
    }

    fn cover(&self, center: Point) -> f32 {
        let offset = center - self.start;
        let along = offset.dot(self.direction);
        if along < 0.0 && self.round_start {
            return self.pen.cover(offset.length());
        }
        if along > self.length && self.round_end {
            return self.pen.cover((offset - self.direction * self.length).length());
        }
        let head = if self.round_start { 1.0 } else { (along + 0.5).clamp(0.0, 1.0) };
        let tail = if self.round_end { 1.0 } else { (self.length - along + 0.5).clamp(0.0, 1.0) };
        self.pen.cover(offset.cross(self.direction).abs()) * head * tail
    }
}

fn row_reach(from: Point, to: Point, center_y: f32, reach: f32) -> Option<(f32, f32)> {
    let (low, high) = (center_y - reach, center_y + reach);
    let rise = to.y - from.y;
    let (enter, leave) = if rise.abs() < 1e-6 {
        if from.y < low || from.y > high {
            return None;
        }
        (0.0, 1.0)
    } else {
        let first = (low - from.y) / rise;
        let second = (high - from.y) / rise;
        (first.min(second).max(0.0), first.max(second).min(1.0))
    };
    if enter > leave {
        return None;
    }
    let start = from.x + (to.x - from.x) * enter;
    let end = from.x + (to.x - from.x) * leave;
    Some((start.min(end) - reach, start.max(end) + reach))
}

struct Edge {
    normal: Point,
    offset: f32,
}

impl Edge {
    fn between(from: Point, to: Point, inside: Point) -> Option<Edge> {
        let along = to - from;
        let length = along.length();
        if !(length > 1e-6) {
            return None;
        }
        let normal = Point::new(along.y, -along.x) * (1.0 / length);
        let normal = if normal.dot(inside - from) > 0.0 { normal * -1.0 } else { normal };
        Some(Edge { normal, offset: normal.dot(from) })
    }

    fn distance(&self, point: Point) -> f32 {
        self.normal.dot(point) - self.offset
    }
}

fn convex_edges(corners: &[Point]) -> Option<Vec<Edge>> {
    if corners.len() < 3 || !(twice_area(corners).abs() > 1e-6) {
        return None;
    }
    let centroid = corners.iter().fold(Point::new(0.0, 0.0), |sum, &corner| sum + corner)
        * (1.0 / corners.len() as f32);
    let next = corners.iter().cycle().skip(1);
    Some(corners.iter().zip(next).filter_map(|(&from, &to)| Edge::between(from, to, centroid)).collect())
}

fn twice_area(corners: &[Point]) -> f32 {
    let next = corners.iter().cycle().skip(1);
    corners.iter().zip(next).map(|(&from, &to)| from.cross(to)).sum()
}

fn cubic_path(controls: [Point; 4]) -> Vec<Point> {
    let [start, first, second, end] = controls;
    let bend = (start - first * 2.0 + second).length().max((first - second * 2.0 + end).length());
    let steps = curve_steps((0.75 * bend / CURVE_TOLERANCE).sqrt());
    (0..=steps).map(|step| bezier(controls, step as f32 / steps as f32)).collect()
}

fn bezier([start, first, second, end]: [Point; 4], share: f32) -> Point {
    let rest = 1.0 - share;
    start * (rest * rest * rest)
        + first * (3.0 * rest * rest * share)
        + second * (3.0 * rest * share * share)
        + end * (share * share * share)
}

fn circle_path(center: Point, radius: f32) -> Vec<Point> {
    let steps = arc_steps(radius, TAU).max(8);
    let mut path: Vec<Point> =
        (0..steps).map(|step| center + polar(radius, TAU * step as f32 / steps as f32)).collect();
    path.push(path[0]);
    path
}

fn rounded_rect_path(left: f32, top: f32, width: f32, height: f32, corner: f32) -> Vec<Point> {
    let radius = corner.max(0.0).min(width.min(height) / 2.0);
    let (right, bottom) = (left + width, top + height);
    let (inner_left, inner_top) = (left + radius, top + radius);
    let (inner_right, inner_bottom) = (right - radius, bottom - radius);
    let tangents = [
        Point::new(inner_left, top),
        Point::new(inner_right, top),
        Point::new(right, inner_top),
        Point::new(right, inner_bottom),
        Point::new(inner_right, bottom),
        Point::new(inner_left, bottom),
        Point::new(left, inner_bottom),
        Point::new(left, inner_top),
    ];
    let centers = [
        Point::new(inner_right, inner_top),
        Point::new(inner_right, inner_bottom),
        Point::new(inner_left, inner_bottom),
        Point::new(inner_left, inner_top),
    ];
    let steps = arc_steps(radius, FRAC_PI_2);
    let mut path = vec![tangents[0]];
    for (quarter, center) in centers.into_iter().enumerate() {
        push_distinct(&mut path, tangents[2 * quarter + 1]);
        let start = FRAC_PI_2 * quarter as f32 - FRAC_PI_2;
        for step in 1..steps {
            let angle = start + FRAC_PI_2 * step as f32 / steps as f32;
            push_distinct(&mut path, center + polar(radius, angle));
        }
        push_distinct(&mut path, tangents[(2 * quarter + 2) % tangents.len()]);
    }
    path
}

fn push_distinct(path: &mut Vec<Point>, point: Point) {
    if path.last() != Some(&point) {
        path.push(point);
    }
}

fn polar(radius: f32, angle: f32) -> Point {
    Point::new(radius * angle.cos(), radius * angle.sin())
}

fn arc_steps(radius: f32, sweep: f32) -> usize {
    let angle = if radius > CURVE_TOLERANCE { 2.0 * (1.0 - CURVE_TOLERANCE / radius).acos() } else { PI };
    curve_steps(sweep / angle)
}

fn curve_steps(estimate: f32) -> usize {
    if estimate.is_finite() {
        estimate.ceil().clamp(1.0, MOST_CURVE_STEPS) as usize
    } else {
        MOST_CURVE_STEPS as usize
    }
}

fn samples(origin: f32, scale: f32, count: usize, limit: u32) -> Vec<(usize, usize)> {
    let limit = limit as usize;
    let end = origin + count as f32 * scale;
    let first = clamp_index((origin - 0.5).ceil(), limit);
    let last = clamp_index((end - 0.5).ceil(), limit);
    (first..last)
        .map(|pixel| {
            let source = ((pixel as f32 + 0.5 - origin) / scale).floor().max(0.0) as usize;
            (pixel, source.min(count - 1))
        })
        .collect()
}

fn pixel_span(low: f32, high: f32, limit: u32) -> Range<usize> {
    let limit = limit as usize;
    clamp_index(low.floor(), limit)..clamp_index(high.ceil(), limit)
}

fn clamp_index(value: f32, limit: usize) -> usize {
    (value.clamp(0.0, limit as f32) as usize).min(limit)
}

fn overlap(index: usize, low: f32, high: f32) -> f32 {
    let start = index as f32;
    ((start + 1.0).min(high) - start.max(low)).clamp(0.0, 1.0)
}

fn rgb_of(color: [u8; 4]) -> [u8; 3] {
    [color[0], color[1], color[2]]
}

fn alpha_of(color: [u8; 4]) -> f32 {
    color[3] as f32 / 255.0
}

fn blend_into(pixel: &mut [u8], [red, green, blue]: [u8; 3], alpha: f32) {
    if !(alpha > 0.0) {
        return;
    }
    if alpha >= 1.0 {
        pixel[..3].copy_from_slice(&[blue, green, red]);
        return;
    }
    for (channel, source) in pixel.iter_mut().zip([blue, green, red]) {
        let below = *channel as f32;
        *channel = (below + (source as f32 - below) * alpha + 0.5) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    const BLACK: [u8; 3] = [0, 0, 0];
    const LIT: [u8; 3] = [255, 255, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];

    fn rgb(sketch: &Sketch, x: usize, y: usize) -> [u8; 3] {
        let index = (y * sketch.width() as usize + x) * 4;
        [sketch.pixels[index + 2], sketch.pixels[index + 1], sketch.pixels[index]]
    }

    fn brightness(sketch: &Sketch, x: usize, y: usize) -> u8 {
        rgb(sketch, x, y)[0]
    }

    fn is_blank(sketch: &Sketch, x: usize, y: usize) -> bool {
        rgb(sketch, x, y) == BLACK
    }

    #[test]
    fn the_background_fills_every_pixel() {
        let sketch = Sketch::new(7, 5, [10, 20, 30]);
        assert_eq!((sketch.width(), sketch.height()), (7, 5));
        let bytes = sketch.into_bgra();
        assert_eq!(bytes.len(), 7 * 5 * 4);
        assert!(bytes.chunks(4).all(|pixel| pixel == [30, 20, 10, 255]));
    }

    #[test]
    fn into_bgra_writes_blue_green_red_and_opaque_alpha_row_by_row() {
        let mut sketch = Sketch::new(2, 2, [1, 2, 3]);
        sketch.fill_rect(1.0, 0.0, 1.0, 1.0, [200, 100, 50, 255]);
        sketch.fill_rect(0.0, 1.0, 1.0, 1.0, [7, 8, 9, 255]);
        let bytes = sketch.into_bgra();
        assert_eq!(bytes, [3, 2, 1, 255, 50, 100, 200, 255, 9, 8, 7, 255, 3, 2, 1, 255]);
    }

    #[test]
    fn a_two_pixel_horizontal_line_colours_its_rows_and_not_the_rows_far_away() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.line((4.0, 20.0), (60.0, 20.0), Stroke::solid(2.0, WHITE));
        for x in 4..60 {
            assert_eq!(rgb(&sketch, x, 19), LIT);
            assert_eq!(rgb(&sketch, x, 20), LIT);
            assert!(is_blank(&sketch, x, 18));
            assert!(is_blank(&sketch, x, 21));
        }
        for x in 0..64 {
            assert!(is_blank(&sketch, x, 10));
            assert!(is_blank(&sketch, x, 30));
        }
        assert!(is_blank(&sketch, 3, 20));
        assert!(is_blank(&sketch, 60, 20));
    }

    #[test]
    fn a_one_pixel_line_through_pixel_centres_covers_exactly_one_row() {
        let mut sketch = Sketch::new(40, 20, BLACK);
        sketch.line((0.0, 10.5), (32.0, 10.5), Stroke::solid(1.0, WHITE));
        for x in 0..32 {
            assert_eq!(rgb(&sketch, x, 10), LIT);
            assert!(is_blank(&sketch, x, 9));
            assert!(is_blank(&sketch, x, 11));
        }
        assert!(is_blank(&sketch, 32, 10));
    }

    #[test]
    fn a_diagonal_line_is_smoothed_at_its_edges() {
        let mut sketch = Sketch::new(40, 40, BLACK);
        sketch.line((0.0, 0.0), (40.0, 40.0), Stroke::solid(1.0, WHITE));
        let partial = (0..40).flat_map(|y| (0..40).map(move |x| (x, y))).filter(|&(x, y)| {
            let level = brightness(&sketch, x, y);
            level > 0 && level < 255
        });
        assert!(partial.count() > 20);
        assert!(is_blank(&sketch, 30, 10));
    }

    #[test]
    fn a_dashed_line_leaves_gaps_between_its_dashes() {
        let mut sketch = Sketch::new(128, 32, BLACK);
        sketch.line((0.0, 10.0), (100.0, 10.0), Stroke::dashed(2.0, WHITE, 6.0, 6.0));
        assert_eq!(rgb(&sketch, 3, 10), LIT);
        assert!(is_blank(&sketch, 9, 10));
        assert_eq!(rgb(&sketch, 15, 10), LIT);
        assert!(is_blank(&sketch, 21, 10));
        let dashes = (1..100).filter(|&x| is_blank(&sketch, x - 1, 10) && !is_blank(&sketch, x, 10)).count();
        assert!(dashes >= 7);
    }

    #[test]
    fn a_dashed_polyline_carries_its_phase_around_corners() {
        let mut sketch = Sketch::new(16, 48, BLACK);
        let path = [(0.5, 10.5), (4.5, 10.5), (4.5, 40.5)];
        sketch.polyline(&path, Stroke::dashed(1.0, WHITE, 6.0, 6.0));
        assert_eq!(rgb(&sketch, 2, 10), LIT);
        assert_eq!(rgb(&sketch, 4, 11), LIT);
        assert!(is_blank(&sketch, 4, 15));
        assert_eq!(rgb(&sketch, 4, 21), LIT);
        assert!(is_blank(&sketch, 4, 27));
        assert_eq!(rgb(&sketch, 4, 33), LIT);
    }

    #[test]
    fn a_dashed_rect_carries_its_phase_around_corners() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.rect(10.5, 10.5, 43.0, 40.0, 0.0, Stroke::dashed(1.0, WHITE, 5.0, 5.0));
        assert_eq!(rgb(&sketch, 12, 10), LIT);
        assert!(is_blank(&sketch, 17, 10));
        assert_eq!(rgb(&sketch, 22, 10), LIT);
        assert_eq!(rgb(&sketch, 53, 11), LIT);
        assert!(is_blank(&sketch, 53, 14));
        assert_eq!(rgb(&sketch, 53, 19), LIT);
    }

    #[test]
    fn a_circle_outline_colours_the_ring_and_leaves_the_centre() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.circle((32.0, 32.0), 20.0, Stroke::solid(2.0, WHITE));
        for (x, y) in [(51, 31), (12, 32), (32, 12), (31, 51)] {
            assert!(brightness(&sketch, x, y) > 200, "ring pixel {x},{y} is {:?}", rgb(&sketch, x, y));
        }
        for (x, y) in [(32, 32), (32, 22), (40, 40), (25, 30)] {
            assert!(is_blank(&sketch, x, y));
        }
        for (x, y) in [(0, 0), (63, 63), (32, 2), (60, 32)] {
            assert!(is_blank(&sketch, x, y));
        }
    }

    #[test]
    fn a_dashed_circle_leaves_gaps_around_its_ring() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.circle((32.0, 32.0), 20.0, Stroke::dashed(2.0, WHITE, 4.0, 4.0));
        let ring = (0..360).map(|degree| {
            let angle = (degree as f32).to_radians();
            let x = (32.0 + 20.0 * angle.cos()).floor() as usize;
            let y = (32.0 + 20.0 * angle.sin()).floor() as usize;
            brightness(&sketch, x, y)
        });
        let (lit, dark) = ring.fold((0, 0), |(lit, dark), level| match level {
            200..=255 => (lit + 1, dark),
            0..=30 => (lit, dark + 1),
            _ => (lit, dark),
        });
        assert!(lit > 60 && dark > 60, "lit {lit}, dark {dark}");
    }

    #[test]
    fn a_rect_outline_leaves_its_inside_untouched() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.rect(10.5, 10.5, 30.0, 20.0, 0.0, Stroke::solid(1.0, WHITE));
        for x in 10..=40 {
            assert_eq!(rgb(&sketch, x, 10), LIT);
            assert_eq!(rgb(&sketch, x, 30), LIT);
        }
        for y in 10..=30 {
            assert_eq!(rgb(&sketch, 10, y), LIT);
            assert_eq!(rgb(&sketch, 40, y), LIT);
        }
        for y in 11..30 {
            for x in 11..40 {
                assert!(is_blank(&sketch, x, y));
            }
        }
        assert!(is_blank(&sketch, 9, 20));
        assert!(is_blank(&sketch, 41, 20));
    }

    #[test]
    fn a_rounded_rect_leaves_its_outer_corner_pixels_untouched() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.rect(10.5, 10.5, 30.0, 30.0, 8.0, Stroke::solid(1.0, WHITE));
        assert_eq!(rgb(&sketch, 25, 10), LIT);
        assert_eq!(rgb(&sketch, 10, 25), LIT);
        for (x, y) in [(10, 10), (40, 10), (10, 40), (40, 40), (25, 25)] {
            assert!(is_blank(&sketch, x, y));
        }
    }

    #[test]
    fn a_half_transparent_fill_blends_with_what_is_below() {
        let mut sketch = Sketch::new(8, 8, BLACK);
        sketch.fill_rect(0.0, 0.0, 4.0, 4.0, [255, 255, 255, 128]);
        assert_eq!(rgb(&sketch, 1, 1), [128, 128, 128]);
        assert!(is_blank(&sketch, 5, 5));
        let mut grey = Sketch::new(4, 4, [100, 100, 100]);
        grey.fill_rect(0.0, 0.0, 4.0, 4.0, [200, 0, 100, 128]);
        assert_eq!(rgb(&grey, 2, 2), [150, 50, 100]);
    }

    #[test]
    fn a_fill_edge_between_pixels_covers_each_side_by_half() {
        let mut sketch = Sketch::new(4, 1, BLACK);
        sketch.fill_rect(0.5, 0.0, 1.0, 1.0, WHITE);
        assert_eq!(rgb(&sketch, 0, 0), [128, 128, 128]);
        assert_eq!(rgb(&sketch, 1, 0), [128, 128, 128]);
        assert!(is_blank(&sketch, 2, 0));
    }

    #[test]
    fn a_translucent_polyline_does_not_darken_its_corners() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        let path = [(10.5, 10.5), (40.5, 10.5), (40.5, 40.5)];
        sketch.polyline(&path, Stroke::solid(3.0, [255, 255, 255, 128]));
        let middle = brightness(&sketch, 25, 10);
        assert!((127..=129).contains(&middle));
        assert_eq!(brightness(&sketch, 40, 10), middle);
        assert_eq!(brightness(&sketch, 40, 25), middle);
        assert!(sketch.pixels.chunks(4).all(|pixel| pixel[..3].iter().all(|&channel| channel <= middle)));
    }

    #[test]
    fn a_translucent_circle_has_no_seam_where_it_closes() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.circle((32.0, 32.0), 20.0, Stroke::solid(3.0, [255, 255, 255, 128]));
        assert!(sketch.pixels.chunks(4).all(|pixel| pixel[..3].iter().all(|&channel| channel <= 129)));
        assert!(brightness(&sketch, 52, 32) >= 120);
    }

    #[test]
    fn an_image_at_scale_four_makes_square_blocks_with_crisp_edges() {
        let mut sketch = Sketch::new(12, 12, BLACK);
        let texels = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]].concat();
        sketch.image(&texels, 2, 2, (2.0, 2.0), (4.0, 4.0), 1.0);
        for y in 2..10 {
            for x in 2..10 {
                let texel = ((y - 2) / 4 * 2 + (x - 2) / 4) * 4;
                assert_eq!(rgb(&sketch, x, y), [texels[texel], texels[texel + 1], texels[texel + 2]]);
            }
        }
        for edge in 0..12 {
            assert!(is_blank(&sketch, 1, edge));
            assert!(is_blank(&sketch, 10, edge));
            assert!(is_blank(&sketch, edge, 1));
            assert!(is_blank(&sketch, edge, 10));
        }
    }

    #[test]
    fn an_image_is_clipped_where_it_leaves_the_bitmap() {
        let mut sketch = Sketch::new(6, 6, BLACK);
        let texels = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]].concat();
        sketch.image(&texels, 2, 2, (-3.0, -3.0), (4.0, 4.0), 1.0);
        assert_eq!(rgb(&sketch, 0, 0), [255, 0, 0]);
        assert_eq!(rgb(&sketch, 1, 0), [0, 255, 0]);
        assert_eq!(rgb(&sketch, 4, 4), [255, 255, 255]);
        assert!(is_blank(&sketch, 5, 5));
    }

    #[test]
    fn half_opacity_halves_an_images_effect() {
        let white = [255, 255, 255, 255];
        let mut full = Sketch::new(4, 4, BLACK);
        full.image(&white, 1, 1, (0.0, 0.0), (2.0, 2.0), 1.0);
        let mut half = Sketch::new(4, 4, BLACK);
        half.image(&white, 1, 1, (0.0, 0.0), (2.0, 2.0), 0.5);
        assert_eq!(rgb(&full, 1, 1), LIT);
        assert!((127..=128).contains(&brightness(&half, 1, 1)));
        assert!(is_blank(&half, 2, 2));
        let mut faint = Sketch::new(4, 4, BLACK);
        faint.image(&[255, 255, 255, 128], 1, 1, (0.0, 0.0), (2.0, 2.0), 0.5);
        assert!((63..=65).contains(&brightness(&faint, 0, 0)));
    }

    #[test]
    fn a_triangle_fills_its_inside_and_leaves_the_outside() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.triangle([(10.0, 10.0), (50.0, 10.0), (10.0, 50.0)], WHITE);
        assert_eq!(rgb(&sketch, 15, 15), LIT);
        assert_eq!(rgb(&sketch, 10, 10), LIT);
        assert!(is_blank(&sketch, 45, 45));
        assert!(is_blank(&sketch, 5, 5));
        assert!(is_blank(&sketch, 9, 30));
    }

    #[test]
    fn a_diamond_draws_its_fill_inside_its_outline() {
        let mut sketch = Sketch::new(64, 64, BLACK);
        sketch.diamond((32.0, 32.0), 10.0, [255, 0, 0, 255], Some(([0, 0, 255, 255], 2.0)));
        assert_eq!(rgb(&sketch, 32, 32), [255, 0, 0]);
        assert!(rgb(&sketch, 31, 22)[2] > 150);
        assert!(rgb(&sketch, 41, 31)[2] > 150);
        assert!(is_blank(&sketch, 32, 10));
        assert!(is_blank(&sketch, 24, 24));
    }

    #[test]
    fn a_cubic_passes_near_its_end_points() {
        let mut sketch = Sketch::new(128, 128, BLACK);
        sketch.cubic((10.0, 10.5), (40.0, 10.5), (60.0, 90.5), (90.0, 90.5), Stroke::solid(1.0, WHITE));
        assert!(brightness(&sketch, 10, 10) > 150);
        assert!(brightness(&sketch, 89, 90) > 150);
        assert!(brightness(&sketch, 49, 50).max(brightness(&sketch, 50, 50)) > 100);
        assert!(is_blank(&sketch, 90, 10));
        assert!(is_blank(&sketch, 10, 90));
        assert!(brightness(&sketch, 8, 10) < 8);
        assert!(brightness(&sketch, 9, 10) < 8);
        assert!(brightness(&sketch, 91, 90) < 8);
    }

    #[test]
    fn shapes_entirely_outside_the_bitmap_change_nothing() {
        let mut sketch = Sketch::new(32, 32, BLACK);
        let before = sketch.pixels.clone();
        let stroke = Stroke::solid(2.0, WHITE);
        sketch.line((-50.0, -50.0), (-10.0, -5.0), stroke);
        sketch.line((40.0, 0.0), (90.0, 30.0), stroke);
        sketch.polyline(&[(-20.0, 10.0), (-20.0, 50.0), (-5.0, 60.0)], Stroke::dashed(1.0, WHITE, 3.0, 2.0));
        sketch.cubic((100.0, 100.0), (150.0, 90.0), (90.0, 150.0), (120.0, 120.0), stroke);
        sketch.circle((-40.0, 16.0), 20.0, stroke);
        sketch.rect(40.0, 40.0, 10.0, 10.0, 2.0, stroke);
        sketch.diamond((-10.0, -10.0), 5.0, WHITE, Some((WHITE, 2.0)));
        sketch.triangle([(40.0, 0.0), (60.0, 0.0), (50.0, 20.0)], WHITE);
        sketch.fill_rect(-30.0, 0.0, 30.0, 32.0, WHITE);
        sketch.image(&[255; 16], 2, 2, (33.0, 0.0), (4.0, 4.0), 1.0);
        sketch.image(&[255; 16], 2, 2, (-9.0, 0.0), (4.0, 4.0), 1.0);
        assert!(sketch.pixels == before);
    }

    #[test]
    fn nan_and_zero_sized_inputs_change_nothing() {
        let nan = f32::NAN;
        let mut sketch = Sketch::new(32, 32, BLACK);
        let before = sketch.pixels.clone();
        let stroke = Stroke::solid(2.0, WHITE);
        sketch.line((nan, 0.0), (10.0, 10.0), stroke);
        sketch.line((5.0, 5.0), (5.0, 5.0), stroke);
        for width in [0.0, -1.0, nan] {
            sketch.line((0.0, 0.0), (20.0, 20.0), Stroke::solid(width, WHITE));
        }
        sketch.line((0.0, 0.0), (20.0, 20.0), Stroke::solid(2.0, [255, 255, 255, 0]));
        sketch.line((0.0, 0.0), (20.0, 20.0), Stroke::dashed(2.0, WHITE, 0.0, 4.0));
        sketch.line((0.0, 0.0), (20.0, 20.0), Stroke::dashed(2.0, WHITE, nan, 4.0));
        sketch.polyline(&[], stroke);
        sketch.polyline(&[(3.0, 3.0)], stroke);
        sketch.polyline(&[(1.0, 1.0), (nan, 5.0), (9.0, 9.0)], stroke);
        sketch.cubic((nan, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0), stroke);
        sketch.cubic((4.0, 4.0), (4.0, 4.0), (4.0, 4.0), (4.0, 4.0), stroke);
        for radius in [0.0, -3.0, nan, f32::INFINITY] {
            sketch.circle((16.0, 16.0), radius, stroke);
        }
        sketch.circle((nan, 16.0), 5.0, stroke);
        sketch.rect(4.0, 4.0, 0.0, 10.0, 1.0, stroke);
        sketch.rect(4.0, 4.0, 10.0, -1.0, 1.0, stroke);
        sketch.rect(nan, 4.0, 10.0, 10.0, 1.0, stroke);
        sketch.rect(4.0, 4.0, f32::INFINITY, 10.0, 1.0, stroke);
        sketch.diamond((16.0, 16.0), 0.0, WHITE, Some((WHITE, 2.0)));
        sketch.diamond((16.0, 16.0), nan, WHITE, Some((WHITE, 2.0)));
        sketch.diamond((nan, 16.0), 4.0, WHITE, Some((WHITE, 2.0)));
        sketch.diamond((16.0, 16.0), 4.0, [0, 0, 0, 0], Some((WHITE, 0.0)));
        sketch.triangle([(1.0, 1.0), (10.0, 10.0), (20.0, 20.0)], WHITE);
        sketch.triangle([(nan, 1.0), (10.0, 1.0), (5.0, 9.0)], WHITE);
        sketch.triangle([(2.0, 2.0), (2.0, 2.0), (2.0, 2.0)], WHITE);
        sketch.fill_rect(2.0, 2.0, 0.0, 5.0, WHITE);
        sketch.fill_rect(2.0, 2.0, -4.0, 5.0, WHITE);
        sketch.fill_rect(nan, 2.0, 4.0, 5.0, WHITE);
        sketch.fill_rect(2.0, 2.0, 4.0, 5.0, [255, 255, 255, 0]);
        sketch.image(&[], 0, 0, (0.0, 0.0), (1.0, 1.0), 1.0);
        sketch.image(&[255; 4], 2, 2, (0.0, 0.0), (1.0, 1.0), 1.0);
        for scale in [(0.0, 4.0), (4.0, -1.0), (nan, 4.0), (4.0, f32::INFINITY)] {
            sketch.image(&[255; 16], 2, 2, (0.0, 0.0), scale, 1.0);
        }
        sketch.image(&[255; 16], 2, 2, (nan, 0.0), (4.0, 4.0), 1.0);
        for opacity in [nan, 0.0, -1.0] {
            sketch.image(&[255; 16], 2, 2, (0.0, 0.0), (4.0, 4.0), opacity);
        }
        sketch.image(&[255, 255, 255, 0], 1, 1, (0.0, 0.0), (4.0, 4.0), 1.0);
        assert!(sketch.pixels == before);
    }

    #[test]
    fn a_zero_sized_sketch_accepts_every_call() {
        for (width, height) in [(0, 0), (0, 10), (10, 0)] {
            let mut sketch = Sketch::new(width, height, LIT);
            let stroke = Stroke::dashed(2.0, WHITE, 3.0, 1.0);
            sketch.fill_rect(0.0, 0.0, 5.0, 5.0, WHITE);
            sketch.line((0.0, 0.0), (5.0, 5.0), stroke);
            sketch.polyline(&[(0.0, 0.0), (5.0, 5.0), (0.0, 9.0)], stroke);
            sketch.cubic((0.0, 0.0), (5.0, 0.0), (0.0, 5.0), (5.0, 5.0), stroke);
            sketch.circle((2.0, 2.0), 2.0, stroke);
            sketch.rect(0.0, 0.0, 4.0, 4.0, 1.0, stroke);
            sketch.diamond((2.0, 2.0), 2.0, WHITE, Some((WHITE, 1.0)));
            sketch.triangle([(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)], WHITE);
            sketch.image(&[255; 4], 1, 1, (0.0, 0.0), (3.0, 3.0), 1.0);
            assert!(sketch.into_bgra().is_empty());
        }
    }

    #[test]
    fn enormous_coordinates_and_widths_are_clipped_without_hanging() {
        let mut sketch = Sketch::new(64, 32, BLACK);
        sketch.line((-1.0e7, 10.5), (1.0e7, 10.5), Stroke::solid(1.0, WHITE));
        assert_eq!(rgb(&sketch, 32, 10), LIT);
        let dashed = Stroke::dashed(3.0, WHITE, 0.01, 0.01);
        sketch.line((-1.0e30, 20.5), (1.0e30, 20.5), dashed);
        sketch.polyline(&[(-1.0e30, -1.0e30), (1.0e30, 1.0e30), (-1.0e30, 1.0e30)], dashed);
        sketch.polyline(&[(-1.0e38, 0.0), (1.0e38, 0.0)], dashed);
        sketch.circle((32.0, 16.0), 1.0e30, Stroke::dashed(2.0, WHITE, 2.0, 2.0));
        sketch.circle((32.0, 16.0), 1.0e4, dashed);
        sketch.cubic((-1.0e30, 0.0), (1.0e30, 1.0e30), (-1.0e30, 1.0e30), (1.0e30, 0.0), dashed);
        sketch.cubic((-3.0e38, 0.0), (3.0e38, 3.0e38), (-3.0e38, 3.0e38), (3.0e38, 0.0), dashed);
        sketch.rect(-1.0e30, -1.0e30, 2.0e30, 2.0e30, 1.0e29, dashed);
        sketch.diamond((32.0, 16.0), 1.0e30, WHITE, Some((WHITE, 1.0e30)));
        sketch.triangle([(-1.0e30, -1.0e30), (1.0e30, 0.0), (0.0, 1.0e30)], WHITE);
        sketch.fill_rect(-1.0e30, -1.0e30, 2.0e30, 2.0e30, [0, 0, 0, 255]);
        sketch.line((0.0, 5.0), (60.0, 5.0), Stroke::solid(1.0e30, WHITE));
        sketch.line((0.0, 5.0), (60.0, 5.0), Stroke::dashed(f32::INFINITY, WHITE, 1.0, f32::INFINITY));
        sketch.image(&[9, 9, 9, 255], 1, 1, (0.0, 0.0), (1.0e30, 1.0e30), 1.0);
        sketch.image(&[9, 9, 9, 255], 1, 1, (-1.0e30, -1.0e30), (1.0e-30, 1.0e-30), 1.0);
        assert_eq!(rgb(&sketch, 63, 31), [9, 9, 9]);
    }

    #[test]
    fn five_thousand_short_segments_on_a_4k_sketch_finish_quickly() {
        let started = Instant::now();
        let mut sketch = Sketch::new(3840, 2160, [24, 24, 28]);
        let mut seed = 0x2545_f491_u32;
        let mut next = move |limit: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % limit
        };
        for index in 0..5000 {
            let from = (next(3840) as f32, next(2160) as f32);
            let to = (from.0 + next(41) as f32 - 20.0, from.1 + next(41) as f32 - 20.0);
            let stroke = match index % 4 {
                0 => Stroke::dashed(1.5, WHITE, 4.0, 3.0),
                1 => Stroke::solid(2.0, [255, 200, 0, 160]),
                _ => Stroke::solid(1.0, WHITE),
            };
            sketch.line(from, to, stroke);
        }
        let bytes = sketch.into_bgra();
        assert_eq!(bytes.len(), 3840 * 2160 * 4);
        assert!(bytes.chunks(4).any(|pixel| pixel == [255, 255, 255, 255]));
        assert!(started.elapsed() < Duration::from_secs(20), "took {:?}", started.elapsed());
    }
}
