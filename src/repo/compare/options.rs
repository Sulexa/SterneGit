use gpui_kit::{
    App, Global, ParentElement as _, Styled as _,
    component::{
        ActiveTheme as _,
        diff::DiffInlineUnit,
        menu::{PopupMenu, PopupMenuItem},
        v_flex,
    },
    div, px,
};

use crate::{git::Whitespace, ui::Look as _};

const MENU_WIDTH: f32 = 300.;
const CONTEXT_LINES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextOptions {
    words: bool,
    wrap: bool,
    whole_file: bool,
    ignore_whitespace: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            words: true,
            wrap: true,
            whole_file: false,
            ignore_whitespace: false,
        }
    }
}

impl Global for TextOptions {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextOption {
    Words,
    Wrap,
    WholeFile,
    IgnoreWhitespace,
}

impl TextOption {
    const ALL: [TextOption; 4] = [
        TextOption::Words,
        TextOption::Wrap,
        TextOption::WholeFile,
        TextOption::IgnoreWhitespace,
    ];

    fn label(self) -> &'static str {
        match self {
            TextOption::Words => "Highlight changed words",
            TextOption::Wrap => "Wrap long lines",
            TextOption::WholeFile => "Show the whole file",
            TextOption::IgnoreWhitespace => "Ignore whitespace",
        }
    }

    fn about(self) -> &'static str {
        match self {
            TextOption::Words => "Marks what changed inside a line",
            TextOption::Wrap => "Breaks lines that are wider than the pane",
            TextOption::WholeFile => "Keeps unchanged lines instead of folding them",
            TextOption::IgnoreWhitespace => {
                "Hides changes to spaces and tabs; parts can't be staged then"
            }
        }
    }
}

impl TextOptions {
    pub(crate) fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    pub(crate) fn wraps(self) -> bool {
        self.wrap
    }

    pub(crate) fn inline_unit(self) -> Option<DiffInlineUnit> {
        self.words.then_some(DiffInlineUnit::Word)
    }

    pub(crate) fn context_lines(self) -> Option<usize> {
        (!self.whole_file).then_some(CONTEXT_LINES)
    }

    pub(crate) fn whitespace(self) -> Whitespace {
        if self.ignore_whitespace {
            Whitespace::Ignored
        } else {
            Whitespace::Compared
        }
    }

    fn is_on(self, option: TextOption) -> bool {
        match option {
            TextOption::Words => self.words,
            TextOption::Wrap => self.wrap,
            TextOption::WholeFile => self.whole_file,
            TextOption::IgnoreWhitespace => self.ignore_whitespace,
        }
    }

    fn toggled(mut self, option: TextOption) -> Self {
        let flag = match option {
            TextOption::Words => &mut self.words,
            TextOption::Wrap => &mut self.wrap,
            TextOption::WholeFile => &mut self.whole_file,
            TextOption::IgnoreWhitespace => &mut self.ignore_whitespace,
        };
        *flag = !*flag;
        self
    }
}

pub(crate) fn text_options_menu(menu: PopupMenu, cx: &App) -> PopupMenu {
    let current = TextOptions::current(cx);
    TextOption::ALL.into_iter().fold(
        menu.min_w(px(MENU_WIDTH)).label("Every text comparison"),
        |menu, option| {
            menu.item(
                PopupMenuItem::element(move |_, cx| {
                    v_flex().child(option.label()).child(
                        div()
                            .xs_text()
                            .text_color(cx.theme().muted_foreground)
                            .child(option.about()),
                    )
                })
                .checked(current.is_on(option))
                .on_click(move |_, window, cx| {
                    cx.set_global(TextOptions::current(cx).toggled(option));
                    window.refresh();
                }),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{TextOption, TextOptions};
    use crate::git::Whitespace;

    #[test]
    fn words_and_wrapping_start_on_and_folding_and_whitespace_start_as_git_shows_them() {
        let options = TextOptions::default();
        assert!(options.inline_unit().is_some());
        assert!(options.wraps());
        assert_eq!(options.context_lines(), Some(3));
        assert_eq!(options.whitespace(), Whitespace::Compared);
    }

    #[test]
    fn each_choice_flips_only_itself() {
        let options = TextOptions::default()
            .toggled(TextOption::WholeFile)
            .toggled(TextOption::IgnoreWhitespace);
        assert_eq!(options.context_lines(), None);
        assert_eq!(options.whitespace(), Whitespace::Ignored);
        assert!(options.wraps());
        assert!(options.toggled(TextOption::Words).inline_unit().is_none());
    }
}
