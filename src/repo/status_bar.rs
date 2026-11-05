use gix::ObjectId;
use gpui_kit::{
    App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    component::{ActiveTheme as _, h_flex, status_bar::StatusBar},
    div, px,
};

use super::{Repository, file_history::HistoryPlace};
use crate::{
    git::HISTORY_LIMIT,
    ui::{Look as _, thousands},
};

const FAILURE_WIDTH: f32 = 480.;

#[derive(IntoElement)]
pub(crate) struct RepoStatusBar {
    summary: String,
    position: String,
    fetching: bool,
    fetch_failure: Option<SharedString>,
}

impl RepoStatusBar {
    pub(crate) fn new(repository: &Repository, selected: Option<ObjectId>) -> Self {
        let history = repository.history();
        let summary = match history {
            _ if repository.is_reading_history() => "Reading history…".to_string(),
            Some(history) => {
                let mut summary = format!(
                    "{} commits · {} refs · read in {} ms",
                    thousands(history.commits.len()),
                    thousands(history.refs.len()),
                    history.elapsed.as_millis()
                );
                if history.truncated {
                    summary.push_str(&format!(" · newest {} shown", thousands(HISTORY_LIMIT)));
                }
                summary
            }
            None => String::new(),
        };
        let position = history
            .zip(selected)
            .and_then(|(history, id)| Some((history.row_of(&id)?, history.commits.len())))
            .map(|(row, count)| format!("Commit {} of {}", thousands(row + 1), thousands(count)))
            .unwrap_or_default();
        Self {
            summary,
            position,
            fetching: repository.is_fetching(),
            fetch_failure: repository.fetch_failure().map(|failure| failure.message()),
        }
    }
}

impl RepoStatusBar {
    pub(crate) fn in_file_history(mut self, place: Option<HistoryPlace>) -> Self {
        if let Some(place) = place {
            self.position = format!(
                "Commit {} of {} changing {}",
                thousands(place.row + 1),
                thousands(place.count),
                place.name
            );
        }
        self
    }
}

impl RenderOnce for RepoStatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let fetch = if self.fetching {
            Some(div().text_color(muted).child("Fetching…"))
        } else {
            self.fetch_failure.map(|failure| {
                div()
                    .max_w(px(FAILURE_WIDTH))
                    .truncate()
                    .text_color(cx.theme().red)
                    .child(SharedString::from(format!("Fetch failed: {failure}")))
            })
        };
        StatusBar::new()
            .left(div().xs_text().text_color(muted).child(self.summary))
            .right(
                h_flex()
                    .gap_4()
                    .xs_text()
                    .children(fetch)
                    .child(div().text_color(muted).child(self.position)),
            )
    }
}
