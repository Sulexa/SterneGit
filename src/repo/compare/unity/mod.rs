mod cells;
mod curves;
mod graph;
mod guid;
mod inspector;
mod options;
mod scene;
mod sheet;

use std::{collections::HashSet, sync::Arc};

use gpui_kit::{
    AnyElement, App, Bounds, Context, EventEmitter, IntoElement, Pixels, Render, ScrollHandle, SharedString,
    Subscription, Window, assets::IconName,
};

pub(crate) use graph::version_picker;
pub(crate) use options::{UnityOptions, unity_options_menu};

use self::{
    graph::GraphState,
    scene::SceneState,
    sheet::Sheet,
};
use super::{Direction, Position};
use crate::unity::{
    ClipComparison, Comparison, GraphComparison, GuidChange, Shape, Shown, State, UnityFiles, UnityKind, clock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnityMode {
    Main,
    Scene,
    Text,
}

impl UnityMode {
    pub(crate) fn id(self) -> &'static str {
        match self {
            UnityMode::Main => "unity-main",
            UnityMode::Scene => "unity-scene",
            UnityMode::Text => "unity-text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModeLook {
    pub(crate) mode: UnityMode,
    pub(crate) label: &'static str,
    pub(crate) icon: IconName,
    pub(crate) tooltip: &'static str,
}

pub(crate) fn modes_for(shape: Shape, kind: UnityKind) -> Vec<ModeLook> {
    let main = match shape {
        Shape::Objects => ModeLook {
            mode: UnityMode::Main,
            label: "Inspector",
            icon: IconName::ListTree,
            tooltip: "Compare objects, components and properties",
        },
        Shape::Clip => ModeLook {
            mode: UnityMode::Main,
            label: "Curves",
            icon: IconName::Spline,
            tooltip: "Draw both versions of every animated property",
        },
        Shape::Controller => ModeLook {
            mode: UnityMode::Main,
            label: "Graph",
            icon: IconName::Workflow,
            tooltip: "Compare the state machine",
        },
        Shape::Graph => ModeLook {
            mode: UnityMode::Main,
            label: "Graph",
            icon: IconName::Workflow,
            tooltip: "Compare the nodes and their connections",
        },
    };
    let scene = ModeLook {
        mode: UnityMode::Scene,
        label: "Scene",
        icon: IconName::Map,
        tooltip: "See what moved, appeared or went away",
    };
    let text = if shape == Shape::Graph {
        ModeLook {
            mode: UnityMode::Text,
            label: "JSON",
            icon: IconName::Braces,
            tooltip: "Compare the graph data, one value per line",
        }
    } else {
        ModeLook {
            mode: UnityMode::Text,
            label: "YAML",
            icon: IconName::Braces,
            tooltip: "Compare the text Unity saved",
        }
    };
    if kind.has_hierarchy() && shape == Shape::Objects { vec![main, scene, text] } else { vec![main, text] }
}

pub(crate) enum UnityEvent {
    KeepOldGuid(GuidChange),
    PointToNewGuid(GuidChange),
}

pub(crate) struct UnityCompare {
    files: Arc<UnityFiles>,
    sources: [SharedString; 2],
    mode: UnityMode,
    options: UnityOptions,
    comparison: Comparison,
    clip: ClipComparison,
    graph: GraphComparison,
    drawn: GraphComparison,
    shown: Shown,
    layer: usize,
    current: usize,
    unfolded: HashSet<i64>,
    collapsed: HashSet<i64>,
    changed_only: bool,
    picked: Option<i64>,
    scroll: ScrollHandle,
    changes_scroll: ScrollHandle,
    scene: SceneState,
    curves: Sheet,
    curves_bounds: Option<Bounds<Pixels>>,
    graph_view: GraphState,
    _options: Subscription,
}

impl EventEmitter<UnityEvent> for UnityCompare {}

impl UnityCompare {
    pub(crate) fn new(files: Arc<UnityFiles>, sources: [SharedString; 2], mode: UnityMode, cx: &mut Context<Self>) -> Self {
        let options = UnityOptions::current(cx);
        let subscription = cx.observe_global::<UnityOptions>(|this, cx| {
            this.options = UnityOptions::current(cx);
            this.recompute();
            cx.notify();
        });
        let mode = if modes_for(files.shape, files.kind).iter().any(|look| look.mode == mode) { mode } else { UnityMode::Main };
        let mut this = Self {
            scene: SceneState::new(),
            files,
            sources,
            mode,
            options,
            comparison: Comparison::default(),
            clip: ClipComparison::default(),
            graph: GraphComparison::default(),
            drawn: GraphComparison::default(),
            shown: Shown::Both,
            layer: 0,
            current: 0,
            unfolded: HashSet::new(),
            collapsed: HashSet::new(),
            changed_only: false,
            picked: None,
            scroll: ScrollHandle::new(),
            changes_scroll: ScrollHandle::new(),
            curves: Sheet::default(),
            curves_bounds: None,
            graph_view: GraphState::default(),
            _options: subscription,
        };
        this.recompute();
        this.picked = this.first_picked();
        this
    }

    fn recompute(&mut self) {
        let options = self.options.options();
        match self.files.shape {
            Shape::Objects => self.comparison = self.files.compare(options),
            Shape::Clip => self.clip = self.files.clip(options),
            Shape::Controller | Shape::Graph => {
                self.graph = self.files.graph(self.layer, options);
                self.drawn = self.graph.as_shown(self.shown);
            }
        }
        self.current = self.current.min(self.count().saturating_sub(1));
        self.scene.forget();
        self.graph_view.forget();
        self.curves_bounds = None;
    }

    fn first_picked(&self) -> Option<i64> {
        self.comparison.objects.first().map(|object| object.key)
    }

    pub(crate) fn replace(&mut self, files: Arc<UnityFiles>, sources: [SharedString; 2], cx: &mut Context<Self>) {
        self.files = files;
        self.sources = sources;
        self.recompute();
        if self.picked.is_none_or(|picked| !self.comparison.objects.iter().any(|object| object.key == picked)) {
            self.picked = self.first_picked();
        }
        cx.notify();
    }

    pub(crate) fn files(&self) -> &Arc<UnityFiles> {
        &self.files
    }

    pub(crate) fn mode(&self) -> UnityMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: UnityMode, cx: &mut Context<Self>) {
        if mode != UnityMode::Text && self.mode != mode {
            self.mode = mode;
            cx.notify();
        }
    }

    pub(crate) fn shown(&self) -> Option<Shown> {
        (self.mode == UnityMode::Main && matches!(self.files.shape, Shape::Controller | Shape::Graph)).then_some(self.shown)
    }

    pub(crate) fn show(&mut self, shown: Shown, cx: &mut Context<Self>) {
        if self.shown != shown {
            self.shown = shown;
            self.drawn = self.graph.as_shown(shown);
            self.graph_view.forget();
            cx.notify();
        }
    }

    pub(crate) fn hidden(&self) -> usize {
        match self.files.shape {
            Shape::Objects => self.comparison.hidden,
            Shape::Controller | Shape::Graph => self.graph.hidden,
            Shape::Clip => 0,
        }
    }

    fn count(&self) -> usize {
        match self.files.shape {
            Shape::Objects => self.comparison.anchors.len(),
            Shape::Clip => self.clip.tracks.iter().filter(|track| track.state != State::Same).count() + usize::from(self.clip.events_changed),
            Shape::Controller | Shape::Graph => self.graph.changes.len(),
        }
    }

    pub(crate) fn change_position(&self) -> Option<Position> {
        let count = self.count();
        (count > 0).then(|| Position {
            index: self.current.min(count - 1),
            count,
        })
    }

    pub(crate) fn step_change(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let count = self.count();
        if count == 0 {
            return;
        }
        self.current = match direction {
            Direction::Back => (self.current + count - 1) % count,
            Direction::Forward => (self.current + 1) % count,
        };
        self.show_current();
        cx.notify();
    }

    fn show_current(&mut self) {
        if matches!(self.files.shape, Shape::Controller | Shape::Graph) {
            self.changes_scroll.scroll_to_item(self.current);
            return;
        }
        if self.files.shape != Shape::Objects {
            return;
        }
        let Some(anchor) = self.comparison.anchors.get(self.current).copied() else {
            return;
        };
        if let Some(object) = self.comparison.objects.get(anchor.object) {
            self.picked = Some(object.key);
            if object.state == State::Removed && anchor.part.is_some() {
                self.unfolded.insert(object.key);
            }
        }
        if let Some(row) = inspector::flat_index(self, anchor) {
            self.scroll.scroll_to_top_of_item(row.saturating_sub(1));
        }
    }

    pub(crate) fn facts(&self) -> String {
        let kind = self.files.kind.label();
        match self.files.shape {
            Shape::Objects if self.files.kind == UnityKind::Meta => match &self.files.guid_change {
                Some(_) => "Import settings · the GUID changed".to_owned(),
                None => "Import settings".to_owned(),
            },
            Shape::Objects if self.files.kind.has_hierarchy() => {
                let count = self.comparison.changed_objects();
                format!("{kind} · {} changed", crate::ui::counted(count, "object", "objects"))
            }
            Shape::Objects => self
                .comparison
                .objects
                .first()
                .and_then(|object| object.what.clone())
                .map_or_else(|| kind.to_owned(), |what| what.to_owned()),
            Shape::Clip => {
                let rate = self.clip.sample_rate;
                let [before, after] = self.clip.lengths;
                let short = |seconds: f64| clock(seconds, rate).split(' ').next().unwrap_or_default().to_owned();
                match (self.files.before.is_some(), self.files.after.is_some()) {
                    (true, true) if (before - after).abs() > 1e-6 => format!("{kind} · {} → {}", short(before), short(after)),
                    _ => format!("{kind} · {}", short(after.max(before))),
                }
            }
            Shape::Controller => {
                let layer = self.graph.layers.get(self.layer).cloned().unwrap_or_default();
                format!("{kind} · {layer}")
            }
            Shape::Graph if !self.graph.has_ports => "State graph · Visual Scripting".to_owned(),
            Shape::Graph => "Script graph · Visual Scripting".to_owned(),
        }
    }

    pub(crate) fn has_zoom(&self) -> bool {
        self.mode == UnityMode::Scene || matches!(self.files.shape, Shape::Controller | Shape::Graph)
    }

    pub(crate) fn zoom_percent(&self) -> Option<f64> {
        match self.mode {
            UnityMode::Scene => self.scene.percent(),
            _ => self.graph_view.percent(),
        }
    }

    pub(crate) fn fits(&self) -> bool {
        match self.mode {
            UnityMode::Scene => self.scene.fits(),
            _ => self.graph_view.fits(),
        }
    }

    pub(crate) fn zoom(&mut self, steps: f64, cx: &mut Context<Self>) {
        match self.mode {
            UnityMode::Scene => self.scene.zoom(steps),
            _ => self.graph_view.zoom(steps),
        }
        cx.notify();
    }

    pub(crate) fn fit(&mut self, cx: &mut Context<Self>) {
        match self.mode {
            UnityMode::Scene => self.scene.fit(),
            _ => self.graph_view.fit(),
        }
        cx.notify();
    }

    pub(crate) fn layers(&self) -> &[String] {
        &self.graph.layers
    }

    pub(crate) fn layer(&self) -> usize {
        self.layer
    }

    pub(crate) fn set_layer(&mut self, layer: usize, cx: &mut Context<Self>) {
        self.layer = layer;
        self.current = 0;
        self.recompute();
        self.graph_view.fit();
        cx.notify();
    }

    fn pick(&mut self, key: i64, cx: &mut Context<Self>) {
        self.picked = Some(key);
        if let Some(at) = self
            .comparison
            .anchors
            .iter()
            .position(|anchor| self.comparison.objects.get(anchor.object).is_some_and(|object| object.key == key))
        {
            self.current = at;
        }
        cx.notify();
    }

    fn toggle_unfolded(&mut self, key: i64, cx: &mut Context<Self>) {
        if !self.unfolded.remove(&key) {
            self.unfolded.insert(key);
        }
        cx.notify();
    }

    fn toggle_tree(&mut self, key: i64, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&key) {
            self.collapsed.insert(key);
        }
        cx.notify();
    }

    fn reveal(&mut self, key: i64, cx: &mut Context<Self>) {
        self.pick(key, cx);
        if let Some(object) = self.comparison.objects.iter().position(|object| object.key == key)
            && let Some(row) = inspector::flat_index(
                self,
                crate::unity::Anchor {
                    object,
                    part: None,
                    row: None,
                },
            )
        {
            self.scroll.scroll_to_top_of_item(row);
        }
    }

    fn choose_change(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.current = ix;
        cx.notify();
    }

    pub(crate) fn forget_images(&mut self, cx: &mut App) {
        self.scene.sheet.forget(cx);
        self.curves.forget(cx);
        self.graph_view.sheet.forget(cx);
    }
}

impl Render for UnityCompare {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match (self.files.shape, self.mode) {
            (Shape::Objects, UnityMode::Scene) => scene::render(self, window, cx),
            (Shape::Objects, _) => inspector::render(self, window, cx),
            (Shape::Clip, _) => curves::render(self, window, cx),
            (Shape::Controller | Shape::Graph, _) => graph::render(self, window, cx),
        };
        content
    }
}
