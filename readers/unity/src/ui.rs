use super::{scene::script_guid, yaml::Document};

pub(crate) const DEFAULT_SCREEN: [f64; 2] = [1920., 1080.];
pub(crate) const REFERENCE_PIXELS_PER_UNIT: f64 = 100.;
const LETTER_WIDTH: f64 = 0.55;
const LINE_HEIGHT: f64 = 1.25;
const UNSET: f64 = -1.;

pub(crate) type Read<'a> = &'a dyn Fn(&str) -> Option<f64>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Script {
    Image,
    RawImage,
    Text,
    TextMeshPro,
    Horizontal,
    Vertical,
    Grid,
    LayoutElement,
    Fitter,
    Scaler,
}

pub(crate) fn script_of(document: &Document) -> Option<Script> {
    let class = document
        .body
        .get("m_EditorClassIdentifier")
        .and_then(super::yaml::Node::text)
        .and_then(|class| class.rsplit(['.', ':']).next())
        .unwrap_or_default();
    let by_class = match class {
        "Image" => Some(Script::Image),
        "RawImage" => Some(Script::RawImage),
        "Text" => Some(Script::Text),
        "TextMeshProUGUI" => Some(Script::TextMeshPro),
        "HorizontalLayoutGroup" => Some(Script::Horizontal),
        "VerticalLayoutGroup" => Some(Script::Vertical),
        "GridLayoutGroup" => Some(Script::Grid),
        "LayoutElement" => Some(Script::LayoutElement),
        "ContentSizeFitter" => Some(Script::Fitter),
        "CanvasScaler" => Some(Script::Scaler),
        _ => None,
    };
    by_class.or_else(|| {
        Some(match script_guid(&document.body)? {
            "fe87c0e1cc204ed48ad3b37840f39efc" => Script::Image,
            "1344c3c82d62a2a41a3576d8abb8e3ea" => Script::RawImage,
            "5f7201a12d95ffc409449d95f23cf332" => Script::Text,
            "f4688fdb7df04437aeb418b961361dc5" => Script::TextMeshPro,
            "30649d3a9faa99c48a7b1166b86bf2a0" => Script::Horizontal,
            "59f8146938fff824cb5fd77236b75775" => Script::Vertical,
            "8a8695521f0d02e499659fee002a26c2" => Script::Grid,
            "306cc8c2b49d7114eaa3623786fc2126" => Script::LayoutElement,
            "3245ec927659c4140ac4f8d17403cc18" => Script::Fitter,
            "0cd44c1031e13a943bb63640046fad76" => Script::Scaler,
            _ => return None,
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) min: [f64; 2],
    pub(crate) size: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RectFields {
    pub(crate) anchor_min: [f64; 2],
    pub(crate) anchor_max: [f64; 2],
    pub(crate) anchored: [f64; 2],
    pub(crate) size_delta: [f64; 2],
    pub(crate) pivot: [f64; 2],
}

fn pair(read: Read, key: &str, default: [f64; 2]) -> [f64; 2] {
    [read(&format!("{key}.x")).unwrap_or(default[0]), read(&format!("{key}.y")).unwrap_or(default[1])]
}

impl RectFields {
    pub(crate) fn read(read: Read) -> Self {
        Self {
            anchor_min: pair(read, "m_AnchorMin", [0.5; 2]),
            anchor_max: pair(read, "m_AnchorMax", [0.5; 2]),
            anchored: pair(read, "m_AnchoredPosition", [0.; 2]),
            size_delta: pair(read, "m_SizeDelta", [100.; 2]),
            pivot: pair(read, "m_Pivot", [0.5; 2]),
        }
    }

    fn span(&self, parent: Rect, axis: usize) -> f64 {
        (self.anchor_max[axis] - self.anchor_min[axis]) * parent.size[axis]
    }

    pub(crate) fn place(&self, parent: Rect) -> ([f64; 2], Rect) {
        let mut at = [0.; 2];
        let mut size = [0.; 2];
        for axis in 0..2 {
            let low = parent.min[axis] + self.anchor_min[axis] * parent.size[axis];
            let span = self.span(parent, axis);
            size[axis] = span + self.size_delta[axis];
            at[axis] = low + span * self.pivot[axis] + self.anchored[axis];
        }
        (at, self.own(size))
    }

    pub(crate) fn own(&self, size: [f64; 2]) -> Rect {
        Rect {
            min: [-self.pivot[0] * size[0], -self.pivot[1] * size[1]],
            size,
        }
    }

    pub(crate) fn sized(mut self, axis: usize, size: f64, parent: Rect) -> Self {
        self.size_delta[axis] = size - self.span(parent, axis);
        self
    }

    fn driven(mut self, axis: usize, at: f64, size: Option<f64>) -> Self {
        self.anchor_min = [0., 1.];
        self.anchor_max = [0., 1.];
        let size = match size {
            Some(size) => {
                self.size_delta[axis] = size;
                size
            }
            None => self.size_delta[axis],
        };
        self.anchored[axis] = if axis == 0 { at + size * self.pivot[axis] } else { -at - size * (1. - self.pivot[axis]) };
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sizes {
    pub(crate) min: f64,
    pub(crate) preferred: f64,
    pub(crate) flexible: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Offer {
    pub(crate) priority: f64,
    pub(crate) min: [f64; 2],
    pub(crate) preferred: [f64; 2],
    pub(crate) flexible: [f64; 2],
}

impl Offer {
    pub(crate) fn element(read: Read) -> Option<Self> {
        if read("m_IgnoreLayout") == Some(1.) {
            return None;
        }
        Some(Self {
            priority: read("m_LayoutPriority").unwrap_or(1.),
            min: [read("m_MinWidth").unwrap_or(UNSET), read("m_MinHeight").unwrap_or(UNSET)],
            preferred: [read("m_PreferredWidth").unwrap_or(UNSET), read("m_PreferredHeight").unwrap_or(UNSET)],
            flexible: [read("m_FlexibleWidth").unwrap_or(UNSET), read("m_FlexibleHeight").unwrap_or(UNSET)],
        })
    }

    pub(crate) fn of(sizes: [Sizes; 2]) -> Self {
        Self {
            priority: 0.,
            min: [sizes[0].min, sizes[1].min],
            preferred: [sizes[0].preferred, sizes[1].preferred],
            flexible: [sizes[0].flexible, sizes[1].flexible],
        }
    }
}

pub(crate) fn sizes_from(offers: &[Offer]) -> [Sizes; 2] {
    let pick = |value: &dyn Fn(&Offer) -> f64, fallback: f64| {
        let mut best = fallback;
        let mut priority = f64::MIN;
        for offer in offers {
            let found = value(offer);
            if found < 0. {
                continue;
            }
            if offer.priority > priority {
                best = found;
                priority = offer.priority;
            } else if offer.priority == priority && found > best {
                best = found;
            }
        }
        best
    };
    [0, 1].map(|axis| {
        let min = pick(&|offer| offer.min[axis], 0.);
        let preferred = pick(&|offer| offer.preferred[axis], 0.).max(min);
        let flexible = pick(&|offer| offer.flexible[axis], 0.);
        Sizes { min, preferred, flexible }
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Padding {
    fn read(read: Read) -> Self {
        Self {
            left: read("m_Padding.m_Left").unwrap_or(0.),
            right: read("m_Padding.m_Right").unwrap_or(0.),
            top: read("m_Padding.m_Top").unwrap_or(0.),
            bottom: read("m_Padding.m_Bottom").unwrap_or(0.),
        }
    }

    fn total(&self, axis: usize) -> f64 {
        if axis == 0 { self.left + self.right } else { self.top + self.bottom }
    }

    fn start(&self, axis: usize) -> f64 {
        if axis == 0 { self.left } else { self.top }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Child {
    pub(crate) fields: RectFields,
    pub(crate) sizes: [Sizes; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Group {
    Line {
        vertical: bool,
        padding: Padding,
        spacing: f64,
        alignment: usize,
        control: [bool; 2],
        expand: [bool; 2],
        reverse: bool,
    },
    Grid {
        padding: Padding,
        cell: [f64; 2],
        spacing: [f64; 2],
        corner: usize,
        by_columns: bool,
        alignment: usize,
        constraint: usize,
        count: usize,
    },
}

fn clamp(value: f64, low: f64, high: f64) -> f64 {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

impl Group {
    pub(crate) fn read(script: Script, read: Read) -> Option<Self> {
        let flag = |key: &str, default: bool| read(key).map_or(default, |value| value != 0.);
        let alignment = read("m_ChildAlignment").unwrap_or(0.).clamp(0., 8.) as usize;
        let padding = Padding::read(read);
        match script {
            Script::Horizontal | Script::Vertical => Some(Group::Line {
                vertical: script == Script::Vertical,
                padding,
                spacing: read("m_Spacing").unwrap_or(0.),
                alignment,
                control: [flag("m_ChildControlWidth", true), flag("m_ChildControlHeight", true)],
                expand: [flag("m_ChildForceExpandWidth", true), flag("m_ChildForceExpandHeight", true)],
                reverse: flag("m_ReverseArrangement", false),
            }),
            Script::Grid => Some(Group::Grid {
                padding,
                cell: pair(read, "m_CellSize", [100.; 2]),
                spacing: pair(read, "m_Spacing", [0.; 2]),
                corner: read("m_StartCorner").unwrap_or(0.).clamp(0., 3.) as usize,
                by_columns: read("m_StartAxis") == Some(1.),
                alignment,
                constraint: read("m_Constraint").unwrap_or(0.) as usize,
                count: read("m_ConstraintCount").unwrap_or(2.).max(1.) as usize,
            }),
            _ => None,
        }
    }

    fn alignment(&self) -> usize {
        match self {
            Group::Line { alignment, .. } | Group::Grid { alignment, .. } => *alignment,
        }
    }

    fn padding(&self) -> Padding {
        match self {
            Group::Line { padding, .. } | Group::Grid { padding, .. } => *padding,
        }
    }

    fn along(&self, axis: usize) -> f64 {
        let alignment = self.alignment();
        if axis == 0 { (alignment % 3) as f64 * 0.5 } else { (alignment / 3) as f64 * 0.5 }
    }

    fn start(&self, rect: Rect, axis: usize, needed: f64) -> f64 {
        let padding = self.padding();
        let surplus = rect.size[axis] - (needed + padding.total(axis));
        padding.start(axis) + surplus * self.along(axis)
    }

    fn child_sizes(child: &Child, axis: usize, control: bool, expand: bool) -> Sizes {
        let mut sizes = if control {
            child.sizes[axis]
        } else {
            let size = child.fields.size_delta[axis];
            Sizes {
                min: size,
                preferred: size,
                flexible: 0.,
            }
        };
        if expand {
            sizes.flexible = sizes.flexible.max(1.);
        }
        sizes
    }

    fn line_totals(&self, axis: usize, children: &[Child]) -> Sizes {
        let Group::Line {
            vertical,
            padding,
            spacing,
            control,
            expand,
            ..
        } = *self
        else {
            return Sizes::default();
        };
        let main = (axis == 1) == vertical;
        let edge = padding.total(axis);
        let (mut min, mut preferred, mut flexible) = (edge, edge, 0.);
        for child in children {
            let sizes = Self::child_sizes(child, axis, control[axis], expand[axis]);
            if main {
                min += sizes.min + spacing;
                preferred += sizes.preferred + spacing;
                flexible += sizes.flexible;
            } else {
                min = f64::max(min, sizes.min + edge);
                preferred = f64::max(preferred, sizes.preferred + edge);
                flexible = f64::max(flexible, sizes.flexible);
            }
        }
        if main && !children.is_empty() {
            min -= spacing;
            preferred -= spacing;
        }
        Sizes {
            min,
            preferred: preferred.max(min),
            flexible,
        }
    }

    pub(crate) fn sizes(&self, children: &[Child]) -> [Sizes; 2] {
        match *self {
            Group::Line { .. } => [self.line_totals(0, children), self.line_totals(1, children)],
            Group::Grid {
                padding,
                cell,
                spacing,
                constraint,
                count,
                ..
            } => {
                let total = children.len();
                let rows_for = |columns: usize| total.div_ceil(columns.max(1));
                let (min_columns, columns) = match constraint {
                    1 => (count, count),
                    2 => (rows_for(count), rows_for(count)),
                    _ => (1, (total as f64).sqrt().ceil() as usize),
                };
                let rows = match constraint {
                    1 => rows_for(count),
                    2 => count,
                    _ => rows_for(columns),
                };
                let extent = |axis: usize, cells: usize| padding.total(axis) + (cell[axis] + spacing[axis]) * cells as f64 - spacing[axis];
                [
                    Sizes {
                        min: extent(0, min_columns),
                        preferred: extent(0, columns),
                        flexible: 0.,
                    },
                    Sizes {
                        min: extent(1, rows),
                        preferred: extent(1, rows),
                        flexible: 0.,
                    },
                ]
            }
        }
    }

    pub(crate) fn arrange(&self, rect: Rect, children: &[Child]) -> Vec<RectFields> {
        let mut placed: Vec<RectFields> = children.iter().map(|child| child.fields).collect();
        match *self {
            Group::Line {
                vertical,
                padding,
                spacing,
                control,
                expand,
                reverse,
                ..
            } => {
                for axis in 0..2 {
                    let size = rect.size[axis];
                    let along = self.along(axis);
                    if (axis == 1) != vertical {
                        let inner = size - padding.total(axis);
                        for (ix, child) in children.iter().enumerate() {
                            let sizes = Self::child_sizes(child, axis, control[axis], expand[axis]);
                            let needed = clamp(inner, sizes.min, if sizes.flexible > 0. { size } else { sizes.preferred });
                            let start = self.start(rect, axis, needed);
                            placed[ix] = if control[axis] {
                                placed[ix].driven(axis, start, Some(needed))
                            } else {
                                let offset = (needed - child.fields.size_delta[axis]) * along;
                                placed[ix].driven(axis, start + offset, None)
                            };
                        }
                        continue;
                    }
                    let totals = self.line_totals(axis, children);
                    let mut at = padding.start(axis);
                    let mut share = 0.;
                    let surplus = size - totals.preferred;
                    if surplus > 0. {
                        if totals.flexible == 0. {
                            at = self.start(rect, axis, totals.preferred - padding.total(axis));
                        } else {
                            share = surplus / totals.flexible;
                        }
                    }
                    let lerp = if totals.min == totals.preferred { 0. } else { ((size - totals.min) / (totals.preferred - totals.min)).clamp(0., 1.) };
                    let order: Vec<usize> = if reverse { (0..children.len()).rev().collect() } else { (0..children.len()).collect() };
                    for ix in order {
                        let child = &children[ix];
                        let sizes = Self::child_sizes(child, axis, control[axis], expand[axis]);
                        let length = sizes.min + (sizes.preferred - sizes.min) * lerp + sizes.flexible * share;
                        placed[ix] = if control[axis] {
                            placed[ix].driven(axis, at, Some(length))
                        } else {
                            let offset = (length - child.fields.size_delta[axis]) * along;
                            placed[ix].driven(axis, at + offset, None)
                        };
                        at += length + spacing;
                    }
                }
            }
            Group::Grid {
                padding,
                cell,
                spacing,
                corner,
                by_columns,
                constraint,
                count,
                ..
            } => {
                let total = children.len();
                if total == 0 {
                    return placed;
                }
                let fitting = |axis: usize| {
                    let step = cell[axis] + spacing[axis];
                    if step <= 0. {
                        usize::MAX
                    } else {
                        (((rect.size[axis] - padding.total(axis) + spacing[axis] + 0.001) / step).floor() as usize).max(1)
                    }
                };
                let (mut columns, mut rows) = (1, 1);
                match constraint {
                    1 => {
                        columns = count;
                        if total > columns {
                            rows = total.div_ceil(columns);
                        }
                    }
                    2 => {
                        rows = count;
                        if total > rows {
                            columns = total.div_ceil(rows);
                        }
                    }
                    _ => {
                        columns = fitting(0);
                        rows = fitting(1);
                    }
                }
                let per_line = if by_columns { rows } else { columns }.max(1);
                let (shown_columns, shown_rows) = if by_columns {
                    (columns.clamp(1, total.div_ceil(per_line).max(1)), rows.clamp(1, total))
                } else {
                    (columns.clamp(1, total), rows.clamp(1, total.div_ceil(per_line).max(1)))
                };
                let needed = [
                    shown_columns as f64 * cell[0] + (shown_columns as f64 - 1.) * spacing[0],
                    shown_rows as f64 * cell[1] + (shown_rows as f64 - 1.) * spacing[1],
                ];
                let start = [self.start(rect, 0, needed[0]), self.start(rect, 1, needed[1])];
                for (ix, fields) in placed.iter_mut().enumerate() {
                    let (mut column, mut row) = if by_columns { (ix / per_line, ix % per_line) } else { (ix % per_line, ix / per_line) };
                    if corner % 2 == 1 {
                        column = shown_columns.saturating_sub(1 + column);
                    }
                    if corner / 2 == 1 {
                        row = shown_rows.saturating_sub(1 + row);
                    }
                    *fields = fields
                        .driven(0, start[0] + (cell[0] + spacing[0]) * column as f64, Some(cell[0]))
                        .driven(1, start[1] + (cell[1] + spacing[1]) * row as f64, Some(cell[1]));
                }
            }
        }
        placed
    }

    pub(crate) fn controls(&self, axis: usize) -> bool {
        match self {
            Group::Line { control, .. } => control[axis],
            Group::Grid { .. } => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fit {
    pub(crate) axes: [u8; 2],
}

impl Fit {
    pub(crate) fn read(read: Read) -> Self {
        Self {
            axes: [read("m_HorizontalFit").unwrap_or(0.) as u8, read("m_VerticalFit").unwrap_or(0.) as u8],
        }
    }

    pub(crate) fn size(&self, axis: usize, sizes: [Sizes; 2]) -> Option<f64> {
        match self.axes[axis] {
            1 => Some(sizes[axis].min),
            2 => Some(sizes[axis].preferred),
            _ => None,
        }
    }
}

pub(crate) fn plain_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let tail = &rest[open..];
        match tail.find('>') {
            Some(close) if !tail[1..close].contains('<') => {
                if tail[1..close].eq_ignore_ascii_case("br") {
                    out.push('\n');
                }
                rest = &tail[close + 1..];
            }
            _ => {
                out.push('<');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub(crate) fn text_sizes(text: &str, size: f64) -> [Sizes; 2] {
    let lines: Vec<&str> = text.split('\n').collect();
    let widest = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    [
        Sizes {
            min: 0.,
            preferred: widest as f64 * size * LETTER_WIDTH,
            flexible: UNSET,
        },
        Sizes {
            min: 0.,
            preferred: lines.len() as f64 * size * LINE_HEIGHT,
            flexible: UNSET,
        },
    ]
}

pub(crate) fn text_alignment(script: Script, read: Read) -> [f64; 2] {
    if script == Script::Text {
        let alignment = read("m_FontData.m_Alignment").unwrap_or(0.).clamp(0., 8.) as usize;
        return [(alignment % 3) as f64 * 0.5, (alignment / 3) as f64 * 0.5];
    }
    let combined = read("m_textAlignment").filter(|value| *value > 0. && *value < 65535.).unwrap_or(257.) as u32;
    let horizontal = read("m_HorizontalAlignment").map_or(combined & 0xff, |value| value as u32);
    let vertical = read("m_VerticalAlignment").map_or(combined & 0xff00, |value| value as u32);
    let across = match horizontal {
        2 | 32 => 0.5,
        4 => 1.,
        _ => 0.,
    };
    let down = match vertical {
        512 | 4096 => 0.5,
        1024 | 2048 => 1.,
        _ => 0.,
    };
    [across, down]
}

#[cfg(test)]
mod tests {
    use super::{Child, Group, Padding, Rect, RectFields, Sizes, plain_text};

    fn fields(size: [f64; 2]) -> RectFields {
        RectFields {
            anchor_min: [0., 1.],
            anchor_max: [0., 1.],
            anchored: [0.; 2],
            size_delta: size,
            pivot: [0.5; 2],
        }
    }

    #[test]
    fn stretched_children_fill_their_parent_and_pinned_ones_keep_their_size() {
        let parent = Rect { min: [-960., -540.], size: [1920., 1080.] };
        let stretched = RectFields {
            anchor_min: [0.; 2],
            anchor_max: [1.; 2],
            anchored: [0.; 2],
            size_delta: [-40., -40.],
            pivot: [0.5; 2],
        };
        let (at, rect) = stretched.place(parent);
        assert_eq!(at, [0., 0.]);
        assert_eq!(rect, Rect { min: [-940., -520.], size: [1880., 1040.] });
        let corner = RectFields {
            anchor_min: [1., 0.],
            anchor_max: [1., 0.],
            anchored: [-50., 30.],
            size_delta: [100., 60.],
            pivot: [1., 0.],
        };
        let (at, rect) = corner.place(parent);
        assert_eq!(at, [910., -510.]);
        assert_eq!(rect.size, [100., 60.]);
    }

    #[test]
    fn a_vertical_group_stacks_its_children_from_the_top() {
        let group = Group::Line {
            vertical: true,
            padding: Padding::default(),
            spacing: 10.,
            alignment: 1,
            control: [false, false],
            expand: [false, false],
            reverse: false,
        };
        let rect = Rect { min: [-150., -150.], size: [300., 300.] };
        let children = [Child { fields: fields([200., 50.]), sizes: [Sizes::default(); 2] }; 3];
        let placed = group.arrange(rect, &children);
        let tops: Vec<f64> = placed.iter().map(|fields| fields.place(rect).0[1]).collect();
        assert_eq!(tops, vec![125., 65., 5.]);
        let lefts: Vec<f64> = placed.iter().map(|fields| fields.place(rect).0[0]).collect();
        assert_eq!(lefts, vec![0., 0., 0.]);
    }

    #[test]
    fn a_grid_fills_rows_of_cells() {
        let group = Group::Grid {
            padding: Padding::default(),
            cell: [100., 100.],
            spacing: [0., 0.],
            corner: 0,
            by_columns: false,
            alignment: 0,
            constraint: 1,
            count: 2,
        };
        let rect = Rect { min: [0., -200.], size: [200., 200.] };
        let children = [Child { fields: fields([10., 10.]), sizes: [Sizes::default(); 2] }; 3];
        let centers: Vec<[f64; 2]> = group.arrange(rect, &children).iter().map(|fields| fields.place(rect).0).collect();
        assert_eq!(centers, vec![[50., -50.], [150., -50.], [50., -150.]]);
    }

    #[test]
    fn rich_text_tags_are_dropped() {
        assert_eq!(plain_text("Found a bug?<br><b>Join</b> us <3"), "Found a bug?\nJoin us <3");
    }
}
