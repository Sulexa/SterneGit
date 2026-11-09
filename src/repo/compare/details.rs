use gpui_kit::{
    AnyElement, App, Hsla, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex},
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::ui::Look as _;

const ROW: f32 = 20.;
const TAG_HEIGHT: f32 = 22.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Version {
    Before,
    After,
}

impl Version {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Version::Before => "Before",
            Version::After => "After",
        }
    }

    fn color(self, cx: &App) -> Hsla {
        match self {
            Version::Before => cx.theme().red,
            Version::After => cx.theme().green,
        }
    }
}

pub(crate) fn version_dot(version: Version, cx: &App) -> AnyElement {
    div()
        .flex_none()
        .size(px(8.))
        .rounded(px(2.))
        .bg(version.color(cx))
        .into_any_element()
}

#[derive(IntoElement)]
pub(crate) struct VersionTag {
    version: Version,
    source: SharedString,
    heard: bool,
}

impl VersionTag {
    pub(crate) fn new(version: Version, source: SharedString) -> Self {
        Self {
            version,
            source,
            heard: false,
        }
    }

    pub(crate) fn heard(mut self, heard: bool) -> Self {
        self.heard = heard;
        self
    }
}

impl RenderOnce for VersionTag {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .flex_none()
            .h(px(TAG_HEIGHT))
            .px_2()
            .gap_1p5()
            .rounded(px(4.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover.opacity(0.88))
            .xs_text()
            .child(version_dot(self.version, cx))
            .child(self.version.label())
            .when(!self.source.is_empty(), |this| {
                this.child(div().text_color(theme.muted_foreground).child(self.source))
            })
            .when(self.heard, |this| {
                this.child(Icon::new(IconName::Headphones).xsmall())
            })
    }
}

pub(crate) fn import_table(changes: &crate::unity::ImportChanges, sources: &[SharedString; 2]) -> MediaDetails {
    let note = format!("{} · {} unchanged", changes.file, crate::ui::thousands(changes.same));
    let mut table = MediaDetails::named("Import settings", changes.columns.iter().cloned().map(SharedString::from).collect()).note(note);
    for (version, values, source) in [
        (Version::Before, &changes.before, &sources[0]),
        (Version::After, &changes.after, &sources[1]),
    ] {
        table = table.row(DetailsRow {
            version,
            source: source.clone(),
            values: values.iter().cloned().map(SharedString::from).collect(),
        });
    }
    table
}

pub(crate) struct DetailsRow {
    pub(crate) version: Version,
    pub(crate) source: SharedString,
    pub(crate) values: Vec<SharedString>,
}

#[derive(IntoElement)]
pub(crate) struct MediaDetails {
    title: SharedString,
    note: Option<SharedString>,
    columns: Vec<SharedString>,
    rows: Vec<DetailsRow>,
    deltas: Vec<Option<SharedString>>,
}

impl MediaDetails {
    pub(crate) fn new(columns: Vec<&'static str>) -> Self {
        Self::named("Details", columns.into_iter().map(SharedString::from).collect())
    }

    pub(crate) fn named(title: impl Into<SharedString>, columns: Vec<SharedString>) -> Self {
        Self {
            title: title.into(),
            note: None,
            columns,
            rows: Vec::new(),
            deltas: Vec::new(),
        }
    }

    pub(crate) fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub(crate) fn row(mut self, row: DetailsRow) -> Self {
        self.rows.push(row);
        self
    }

    pub(crate) fn deltas(mut self, deltas: Vec<Option<SharedString>>) -> Self {
        self.deltas = deltas;
        self
    }
}

impl RenderOnce for MediaDetails {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let compared = self.rows.len() == 2;
        let before: Vec<SharedString> = self
            .rows
            .iter()
            .find(|row| row.version == Version::Before)
            .map(|row| row.values.clone())
            .unwrap_or_default();
        let heading = |text: SharedString| {
            div()
                .h(px(ROW))
                .text_color(muted)
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .child(text)
        };
        let who = v_flex()
            .flex_none()
            .child(
                h_flex()
                    .h(px(ROW))
                    .gap_1p5()
                    .text_color(muted)
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child(Icon::new(IconName::ChevronDown).xsmall())
                    .child(self.title.clone())
                    .children(self.note.clone().map(|note| {
                        div().font_weight(gpui_kit::FontWeight::NORMAL).child(note)
                    })),
            )
            .children(self.rows.iter().map(|row| {
                h_flex()
                    .h(px(ROW))
                    .gap_1p5()
                    .child(version_dot(row.version, cx))
                    .child(row.version.label())
                    .child(div().text_color(muted).child(row.source.clone()))
            }));
        let columns = self.columns.iter().enumerate().map(|(ix, name)| {
            v_flex()
                .flex_none()
                .child(heading(name.clone()))
                .children(self.rows.iter().map(|row| {
                    let value = row.values.get(ix).cloned().unwrap_or_default();
                    let changed = compared && row.version == Version::After && before.get(ix) != Some(&value);
                    let delta = self.deltas.get(ix).cloned().flatten().filter(|_| changed);
                    h_flex()
                        .h(px(ROW))
                        .gap_1p5()
                        .child(
                            div()
                                .when(changed, |this| this.text_color(theme.yellow))
                                .child(value),
                        )
                        .children(delta.map(|delta| div().text_color(muted).child(delta)))
                }))
        });
        h_flex()
            .id("media-details")
            .flex_none()
            .w_full()
            .items_start()
            .gap_4()
            .px_3()
            .pt_1p5()
            .pb_2()
            .overflow_x_scroll()
            .border_t_1()
            .border_color(theme.border)
            .xs_text()
            .whitespace_nowrap()
            .child(who)
            .children(columns)
    }
}
