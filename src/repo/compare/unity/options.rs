use gpui_kit::{
    App, Global, ParentElement as _, Styled as _,
    component::{
        ActiveTheme as _,
        menu::{PopupMenu, PopupMenuItem},
        v_flex,
    },
    div, px,
};

use crate::{ui::Look as _, unity::Options};

const MENU_WIDTH: f32 = 320.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UnityOptions {
    unchanged: bool,
    hide_noise: bool,
    file_ids: bool,
}

impl Default for UnityOptions {
    fn default() -> Self {
        Self {
            unchanged: false,
            hide_noise: true,
            file_ids: false,
        }
    }
}

impl Global for UnityOptions {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnityOption {
    Unchanged,
    HideNoise,
    FileIds,
}

impl UnityOption {
    const ALL: [UnityOption; 3] = [UnityOption::Unchanged, UnityOption::HideNoise, UnityOption::FileIds];

    fn label(self) -> &'static str {
        match self {
            UnityOption::Unchanged => "Show unchanged properties",
            UnityOption::HideNoise => "Hide what Unity changes on its own",
            UnityOption::FileIds => "Show file IDs",
        }
    }

    fn about(self) -> &'static str {
        match self {
            UnityOption::Unchanged => "Lists every property of a changed component",
            UnityOption::HideNoise => "Float rounding, reordered overrides, version bumps",
            UnityOption::FileIds => "Names each object and component by its fileID too",
        }
    }
}

impl UnityOptions {
    pub(crate) fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    pub(crate) fn options(self) -> Options {
        Options {
            unchanged: self.unchanged,
            hide_noise: self.hide_noise,
            file_ids: self.file_ids,
        }
    }

    fn is_on(self, option: UnityOption) -> bool {
        match option {
            UnityOption::Unchanged => self.unchanged,
            UnityOption::HideNoise => self.hide_noise,
            UnityOption::FileIds => self.file_ids,
        }
    }

    fn toggled(mut self, option: UnityOption) -> Self {
        let flag = match option {
            UnityOption::Unchanged => &mut self.unchanged,
            UnityOption::HideNoise => &mut self.hide_noise,
            UnityOption::FileIds => &mut self.file_ids,
        };
        *flag = !*flag;
        self
    }
}

pub(crate) fn unity_options_menu(menu: PopupMenu, cx: &App) -> PopupMenu {
    let current = UnityOptions::current(cx);
    UnityOption::ALL.into_iter().fold(
        menu.min_w(px(MENU_WIDTH)).label("Every Unity comparison"),
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
                    cx.set_global(UnityOptions::current(cx).toggled(option));
                    window.refresh();
                }),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{UnityOption, UnityOptions};

    #[test]
    fn unity_noise_is_hidden_and_only_changed_properties_show_at_first() {
        let options = UnityOptions::default().options();
        assert!(options.hide_noise);
        assert!(!options.unchanged);
        assert!(!options.file_ids);
        assert!(UnityOptions::default().toggled(UnityOption::FileIds).options().file_ids);
    }
}
