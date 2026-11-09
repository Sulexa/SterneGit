use std::sync::Arc;

use gpui_kit::{
    App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    details::{DetailsRow, MediaDetails, Version},
    picture::{file_size, signed_size},
};
use crate::ui::{Handler, Look as _};

const NOTICE_WIDTH: f32 = 520.;
const NOT_COMMITTED: &str = "Not committed yet";

pub(crate) struct FileFacts {
    pub(crate) size: u64,
    pub(crate) last_change: Option<String>,
}

pub(crate) struct OtherFile {
    pub(crate) type_name: SharedString,
    pub(crate) before: Option<FileFacts>,
    pub(crate) after: Option<FileFacts>,
    pub(crate) diff_tool: Option<String>,
    pub(crate) note: Option<SharedString>,
}

impl OtherFile {
    pub(crate) fn facts(&self) -> SharedString {
        match (&self.before, &self.after) {
            (Some(before), Some(after)) => format!(
                "{} → {}",
                file_size(before.size as usize),
                file_size(after.size as usize)
            )
            .into(),
            (Some(only), None) | (None, Some(only)) => file_size(only.size as usize).into(),
            (None, None) => SharedString::default(),
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct OtherFileView {
    file: Arc<OtherFile>,
    imports: Option<crate::unity::ImportChanges>,
    sources: [SharedString; 2],
    on_open: Handler<Version>,
    on_difftool: Handler<()>,
}

impl OtherFileView {
    pub(crate) fn new(
        file: Arc<OtherFile>,
        sources: [SharedString; 2],
        on_open: Handler<Version>,
        on_difftool: Handler<()>,
    ) -> Self {
        Self {
            file,
            imports: None,
            sources,
            on_open,
            on_difftool,
        }
    }

    pub(crate) fn imports(mut self, imports: Option<crate::unity::ImportChanges>) -> Self {
        self.imports = imports;
        self
    }

    fn details(&self) -> MediaDetails {
        let row = |version: Version, facts: &FileFacts, source: &SharedString| DetailsRow {
            version,
            source: source.clone(),
            values: vec![
                self.file.type_name.clone(),
                file_size(facts.size as usize).into(),
                facts
                    .last_change
                    .clone()
                    .unwrap_or_else(|| NOT_COMMITTED.to_owned())
                    .into(),
            ],
        };
        let mut details = MediaDetails::new(vec!["Type", "File size", "Last changed"]);
        if let Some(before) = &self.file.before {
            details = details.row(row(Version::Before, before, &self.sources[0]));
        }
        if let Some(after) = &self.file.after {
            details = details.row(row(Version::After, after, &self.sources[1]));
        }
        let delta = self
            .file
            .before
            .as_ref()
            .zip(self.file.after.as_ref())
            .map(|(before, after)| signed_size(after.size as i64 - before.size as i64).into());
        details.deltas(vec![None, delta, None])
    }
}

impl RenderOnce for OtherFileView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let open = |id: &'static str, version: Version| {
            let on_open = self.on_open.clone();
            Button::new(id)
                .icon(IconName::ExternalLink)
                .label(format!("Open {}", version.label()))
                .tooltip(match version {
                    Version::Before => "Open a copy of the version before the change with its default app",
                    Version::After => "Open this version with its default app",
                })
                .on_click(move |_, window, cx| on_open(&version, window, cx))
        };
        let both = self.file.before.is_some() && self.file.after.is_some();
        let difftool = self.file.diff_tool.clone().filter(|_| both).map(|tool| {
            let on_difftool = self.on_difftool.clone();
            Button::new("difftool")
                .primary()
                .label(format!("Compare in {tool}"))
                .tooltip("git difftool, with the tool set in diff.tool")
                .on_click(move |_, window, cx| on_difftool(&(), window, cx))
        });
        let title = format!("SterneGit can't draw {} files", self.file.type_name);
        v_flex()
            .size_full()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_6()
                    .child(Icon::new(IconName::FileQuestionMark).large().text_color(theme.muted_foreground))
                    .child(div().base_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .max_w(px(NOTICE_WIDTH))
                            .text_center()
                            .sm_text()
                            .text_color(theme.muted_foreground)
                            .child("Open each version in the app that made it, or hand both to the diff tool set in Git's diff.tool."),
                    )
                    .when_some(self.file.note.clone(), |this, note| {
                        this.child(
                            div()
                                .max_w(px(NOTICE_WIDTH))
                                .text_center()
                                .sm_text()
                                .text_color(theme.yellow)
                                .child(note),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .mt_1()
                            .when(self.file.before.is_some(), |this| this.child(open("open-before", Version::Before)))
                            .when(self.file.after.is_some(), |this| this.child(open("open-after", Version::After)))
                            .children(difftool),
                    ),
            )
            .child(self.details())
            .when_some(self.imports.as_ref(), |this, imports| this.child(super::details::import_table(imports, &self.sources)))
    }
}
