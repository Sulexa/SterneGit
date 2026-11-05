use gpui_kit::{
    App, Entity, Window,
    component::{
        WindowExt as _,
        button::ButtonVariant,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use super::repository::Repository;
use crate::{
    git::Stash,
    ui::{Handler, handler},
};

const DELETE: &str = "Delete";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StashAction {
    Apply,
    Pop,
    Delete,
}

impl StashAction {
    pub(crate) fn done(self, stash: &Stash) -> String {
        match self {
            StashAction::Apply => format!("Applied {}; the stash is kept", stash.name),
            StashAction::Pop => format!("Popped {}", stash.name),
            StashAction::Delete => format!("Deleted {}", stash.name),
        }
    }

    pub(crate) fn failed(self, stash: &Stash) -> String {
        let verb = match self {
            StashAction::Apply => "apply",
            StashAction::Pop => "pop",
            StashAction::Delete => "delete",
        };
        format!("Couldn't {verb} {}", stash.name)
    }

    fn text(self, stash: &Stash) -> String {
        match self {
            StashAction::Apply => format!("Apply {}", stash.name),
            StashAction::Pop => format!("Pop {}", stash.name),
            StashAction::Delete => format!("Delete {}…", stash.name),
        }
    }
}

#[derive(Clone)]
pub(crate) struct StashMenu {
    writing: bool,
    on_action: Option<Handler<(Stash, StashAction)>>,
}

impl StashMenu {
    pub(crate) fn new(writing: bool) -> Self {
        Self {
            writing,
            on_action: None,
        }
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(Stash, StashAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(&self, menu: PopupMenu, stash: &Stash) -> PopupMenu {
        menu.item(self.item(StashAction::Apply, stash))
            .item(self.item(StashAction::Pop, stash))
            .separator()
            .item(self.item(StashAction::Delete, stash))
    }

    fn item(&self, action: StashAction, stash: &Stash) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (stash.clone(), action);
        PopupMenuItem::new(action.text(stash))
            .disabled(self.writing || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

fn what_deleting_loses(stash: &Stash) -> String {
    format!(
        "Git keeps no other copy of “{}”, so its changes are lost.",
        stash.message
    )
}

pub(crate) fn confirm_delete(
    repository: Entity<Repository>,
    stash: Stash,
    window: &mut Window,
    cx: &mut App,
) {
    let title = format!("Delete {}?", stash.name);
    let description = what_deleting_loses(&stash);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let repository = repository.clone();
        let stash = stash.clone();
        alert
            .confirm()
            .title(title.clone())
            .description(description.clone())
            .ok_text(DELETE)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                repository.update(cx, |repository, cx| {
                    repository.act_on_stash(stash.clone(), StashAction::Delete, cx)
                });
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{StashAction, what_deleting_loses};
    use crate::git::{Stash, fixtures::id};

    fn older() -> Stash {
        Stash {
            id: id(1),
            name: "stash@{1}".into(),
            message: "On feature/diff-view: Try a wider gutter".into(),
        }
    }

    #[test]
    fn every_item_names_the_stash() {
        let texts: Vec<String> = [StashAction::Apply, StashAction::Pop, StashAction::Delete]
            .into_iter()
            .map(|action| action.text(&older()))
            .collect();
        assert_eq!(
            texts,
            ["Apply stash@{1}", "Pop stash@{1}", "Delete stash@{1}…"]
        );
    }

    #[test]
    fn applying_says_the_stash_is_kept() {
        assert_eq!(
            StashAction::Apply.done(&older()),
            "Applied stash@{1}; the stash is kept"
        );
        assert_eq!(StashAction::Pop.failed(&older()), "Couldn't pop stash@{1}");
    }

    #[test]
    fn the_delete_question_quotes_the_message() {
        assert_eq!(
            what_deleting_loses(&older()),
            "Git keeps no other copy of “On feature/diff-view: Try a wider gutter”, so its changes are lost."
        );
    }
}
