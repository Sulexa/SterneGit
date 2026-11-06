use std::sync::Arc;

use sternegit_protocol::{PullSummary, StateFilter};
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::{DropdownMenu as _, PopupMenuItem},
        spinner::Spinner,
        tooltip::Tooltip,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::looks::{check_mark, state_mark};
use crate::{
    hosting::{Fix, Trouble},
    ui::{Handler, Look as _, SectionHeader, list_row, tree_line},
};

const TITLE: &str = "Pull requests";
const FILTER_TOOLTIP: &str = "Choose which pull requests to list";
const READING: &str = "Reading pull requests…";
const LOAD_MORE: &str = "Load more";
const ADD_ACCOUNT: &str = "Add account…";
const SIGN_IN_AGAIN: &str = "Sign in again";
const RETRY: &str = "Retry";
const CHOOSING: &str = "Pick an account above to see pull requests";
const FILTERS: [StateFilter; 3] = [StateFilter::Open, StateFilter::Closed, StateFilter::All];

#[derive(Clone)]
pub(crate) enum SectionState {
    Reading,
    Listed {
        pulls: Arc<[PullSummary]>,
        more: bool,
        loading_more: bool,
    },
    Note {
        icon: IconName,
        text: SharedString,
        fix: Option<Fix>,
    },
}

impl SectionState {
    pub(crate) fn no_account(service: &str, server: &str) -> Self {
        SectionState::Note {
            icon: IconName::User,
            text: format!("Add a {service} account to see the pull requests of {server}").into(),
            fix: Some(Fix::SignInAgain),
        }
    }

    pub(crate) fn choosing() -> Self {
        SectionState::Note {
            icon: IconName::User,
            text: CHOOSING.into(),
            fix: None,
        }
    }

    pub(crate) fn signed_out(login: &str) -> Self {
        SectionState::Note {
            icon: IconName::TriangleAlert,
            text: format!("{login} is signed out").into(),
            fix: Some(Fix::SignInAgain),
        }
    }

    pub(crate) fn failed(trouble: &Trouble) -> Self {
        SectionState::Note {
            icon: IconName::CircleX,
            text: format!("{}: {}", trouble.title, trouble.message).into(),
            fix: matches!(trouble.fix, Fix::Retry | Fix::SignInAgain | Fix::SignInWithBrowser).then_some(trouble.fix),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PullsSection {
    pub(crate) state: SectionState,
    pub(crate) filter: StateFilter,
    pub(crate) open: bool,
    pub(crate) selected: Option<u64>,
    pub(crate) needs_an_account: bool,
    pub(crate) on_select: Option<Handler<u64>>,
    pub(crate) on_toggle: Option<Handler<()>>,
    pub(crate) on_filter: Option<Handler<StateFilter>>,
    pub(crate) on_more: Option<Handler<()>>,
    pub(crate) on_fix: Option<Handler<Fix>>,
}

impl PullsSection {
    pub(crate) fn len(&self) -> usize {
        let body = match &self.state {
            _ if !self.open => 0,
            SectionState::Listed { pulls, more, .. } if pulls.is_empty() => 1 + usize::from(*more),
            SectionState::Listed { pulls, more, .. } => pulls.len() + usize::from(*more),
            SectionState::Reading | SectionState::Note { .. } => 1,
        };
        1 + body
    }

    pub(crate) fn row(&self, ix: usize, cx: &App) -> AnyElement {
        if ix == 0 {
            return self.header(cx);
        }
        match &self.state {
            SectionState::Reading => note_row(Lead::Spinner, READING.into(), None, cx),
            SectionState::Note { icon, text, fix } => {
                let action = fix.map(|fix| self.fix_button(fix));
                note_row(Lead::Icon(*icon), text.clone(), action, cx)
            }
            SectionState::Listed { pulls, loading_more, .. } => match pulls.get(ix - 1) {
                Some(pull) => self.pull_row(pull, cx),
                None if pulls.is_empty() && ix == 1 => {
                    note_row(Lead::Nothing, nothing_listed(self.filter).into(), None, cx)
                }
                None => self.more_row(*loading_more, cx),
            },
        }
    }

    fn header(&self, cx: &App) -> AnyElement {
        let count = match &self.state {
            SectionState::Listed { pulls, .. } => Some(pulls.len()),
            SectionState::Reading | SectionState::Note { .. } => None,
        };
        let filter = matches!(self.state, SectionState::Listed { .. }).then(|| self.filter_button(cx));
        SectionHeader::new(TITLE)
            .open(self.open)
            .when_some(count, SectionHeader::count)
            .when_some(filter, SectionHeader::action)
            .when_some(self.on_toggle.clone(), |header, on_toggle| {
                header.on_toggle(move |_, window, cx| on_toggle(&(), window, cx))
            })
            .into_any_element()
    }

    fn filter_button(&self, _: &App) -> AnyElement {
        let chosen = self.filter;
        let on_filter = self.on_filter.clone();
        Button::new("pulls-filter")
            .ghost()
            .xsmall()
            .icon(IconName::ListFilter)
            .when(chosen != StateFilter::Open, |button| button.label(filter_label(chosen)))
            .tooltip(FILTER_TOOLTIP)
            .dropdown_menu(move |menu, _, _| {
                FILTERS.into_iter().fold(menu, |menu, filter| {
                    let on_filter = on_filter.clone();
                    menu.item(
                        PopupMenuItem::new(filter_label(filter))
                            .checked(filter == chosen)
                            .on_click(move |_, window, cx| {
                                if let Some(on_filter) = &on_filter {
                                    on_filter(&filter, window, cx)
                                }
                            }),
                    )
                })
            })
            .into_any_element()
    }

    fn pull_row(&self, pull: &PullSummary, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let state = state_mark(pull.state, pull.draft);
        let number = pull.number;
        let words = row_tooltip(pull);
        let checks = pull.checks.map(|checks| {
            let mark = check_mark(checks);
            Icon::new(mark.icon).xsmall().text_color(mark.tint.color(cx))
        });
        list_row(SharedString::from(format!("pull/{number}")), self.selected == Some(number))
            .px_3()
            .child(
                h_flex()
                    .id(SharedString::from(format!("pull-line/{number}")))
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .child(
                        tree_line(
                            0,
                            None,
                            Icon::new(state.icon).small().text_color(state.tint.color(cx)),
                            &pull.title,
                            false,
                            cx,
                        )
                        .flex_1(),
                    )
                    .children(checks)
                    .child(
                        div()
                            .flex_none()
                            .xs_text()
                            .text_color(muted)
                            .child(format!("#{number}")),
                    )
                    .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx)),
            )
            .when_some(self.on_select.clone(), |row, on_select| {
                row.on_click(move |_, window, cx| on_select(&number, window, cx))
            })
            .into_any_element()
    }

    fn more_row(&self, loading: bool, cx: &App) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        list_row("pulls-more", false)
            .px_3()
            .child(
                h_flex()
                    .w_full()
                    .pl_5()
                    .gap_2()
                    .xs_text()
                    .text_color(muted)
                    .when(loading, |row| row.child(Spinner::new().xsmall()))
                    .child(LOAD_MORE),
            )
            .when_some(self.on_more.clone().filter(|_| !loading), |row, on_more| {
                row.on_click(move |_, window, cx| on_more(&(), window, cx))
            })
            .into_any_element()
    }

    fn fix_button(&self, fix: Fix) -> AnyElement {
        let label = match fix {
            Fix::Retry => RETRY,
            Fix::SignInAgain | Fix::SignInWithBrowser if self.needs_an_account => ADD_ACCOUNT,
            Fix::SignInAgain | Fix::SignInWithBrowser | Fix::CopyLog | Fix::Nothing => SIGN_IN_AGAIN,
        };
        let on_fix = self.on_fix.clone();
        Button::new("pulls-fix")
            .ghost()
            .xsmall()
            .label(label)
            .on_click(move |_, window, cx| {
                if let Some(on_fix) = &on_fix {
                    on_fix(&fix, window, cx)
                }
            })
            .into_any_element()
    }
}

enum Lead {
    Icon(IconName),
    Spinner,
    Nothing,
}

fn note_row(lead: Lead, text: SharedString, action: Option<AnyElement>, cx: &App) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let words = text.clone();
    list_row(SharedString::from(format!("pulls-note/{text}")), false)
        .px_3()
        .child(
            h_flex()
                .id("pulls-note")
                .w_full()
                .min_w_0()
                .gap_1p5()
                .pl(px(18.))
                .xs_text()
                .text_color(muted)
                .child(match lead {
                    Lead::Icon(icon) => Icon::new(icon).xsmall().text_color(muted).into_any_element(),
                    Lead::Spinner => Spinner::new().xsmall().into_any_element(),
                    Lead::Nothing => div().into_any_element(),
                })
                .child(div().flex_1().ellipsis().child(text))
                .children(action)
                .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx)),
        )
        .into_any_element()
}

pub(crate) fn filter_label(filter: StateFilter) -> &'static str {
    match filter {
        StateFilter::Open | StateFilter::Unknown => "Open",
        StateFilter::Closed => "Closed",
        StateFilter::All => "All",
    }
}

fn nothing_listed(filter: StateFilter) -> &'static str {
    match filter {
        StateFilter::Open | StateFilter::Unknown => "No open pull requests",
        StateFilter::Closed => "No closed pull requests",
        StateFilter::All => "No pull requests yet",
    }
}

fn row_tooltip(pull: &PullSummary) -> String {
    format!(
        "#{} by {}: {} into {}",
        pull.number, pull.author.login, pull.head.branch, pull.base.branch
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sternegit_protocol::StateFilter;

    use super::{PullsSection, SectionState};
    use crate::repo::pulls::samples::pull;

    fn section(state: SectionState, open: bool) -> PullsSection {
        PullsSection {
            state,
            filter: StateFilter::Open,
            open,
            selected: None,
            needs_an_account: false,
            on_select: None,
            on_toggle: None,
            on_filter: None,
            on_more: None,
            on_fix: None,
        }
    }

    #[test]
    fn rows_count_the_header_then_pulls_then_load_more() {
        let listed = |count: usize, more: bool| SectionState::Listed {
            pulls: (0..count).map(|ix| pull(ix as u64, "topic", "sternegit-org")).collect::<Arc<[_]>>(),
            more,
            loading_more: false,
        };
        assert_eq!(section(listed(4, false), true).len(), 5);
        assert_eq!(section(listed(4, true), true).len(), 6);
        assert_eq!(section(listed(0, false), true).len(), 2);
        assert_eq!(section(listed(4, true), false).len(), 1);
        assert_eq!(section(SectionState::Reading, true).len(), 2);
        assert_eq!(section(SectionState::choosing(), true).len(), 2);
    }
}
