use std::rc::Rc;

use gpui_kit::{
    App, ClipboardItem, Entity, SharedString, Window,
    component::{
        WindowExt as _,
        button::ButtonVariant,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use super::{
    branch_menu::{DeleteQuestion, what_deleting_loses},
    repository::Repository,
};
use crate::{
    git::{History, RefLabel},
    ui::{Handler, handler},
};

const BRANCH_HERE: &str = "Create branch here…";
const COPY_NAME: &str = "Copy name";
const NAME_COPIED: &str = "Tag name copied";
const DELETE: &str = "Delete";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TagAction {
    CheckOut,
    Push,
    BranchHere,
    CopyName,
    Delete,
    DeleteThere,
}

#[derive(Clone)]
pub(crate) struct TagMenu {
    remote: Option<SharedString>,
    writing: bool,
    underway: bool,
    on_action: Option<Handler<(RefLabel, TagAction)>>,
}

impl TagMenu {
    pub(crate) fn new(remote: Option<SharedString>, writing: bool, underway: bool) -> Self {
        Self {
            remote,
            writing,
            underway,
            on_action: None,
        }
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(RefLabel, TagAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(&self, menu: PopupMenu, label: &RefLabel) -> PopupMenu {
        self.offered()
            .into_iter()
            .enumerate()
            .fold(menu, |menu, (ix, group)| {
                let menu = if ix > 0 { menu.separator() } else { menu };
                group
                    .into_iter()
                    .fold(menu, |menu, action| menu.item(self.item(action, label)))
            })
    }

    fn offered(&self) -> Vec<Vec<TagAction>> {
        use TagAction::*;
        let there = self.remote.is_some();
        [
            Some(vec![CheckOut]),
            there.then(|| vec![Push]),
            Some(vec![BranchHere]),
            Some(vec![CopyName]),
            Some([Some(Delete), there.then_some(DeleteThere)].into_iter().flatten().collect()),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn available(&self, action: TagAction) -> bool {
        match action {
            TagAction::CopyName => true,
            TagAction::CheckOut | TagAction::BranchHere => !self.writing && !self.underway,
            TagAction::Push | TagAction::Delete | TagAction::DeleteThere => !self.writing,
        }
    }

    fn text(&self, action: TagAction, label: &RefLabel) -> String {
        let remote = self.remote.as_deref().unwrap_or_default();
        let name = &label.name;
        match action {
            TagAction::CheckOut => format!("Check out {name}"),
            TagAction::Push => format!("Push {name} to {remote}"),
            TagAction::BranchHere => BRANCH_HERE.to_owned(),
            TagAction::CopyName => COPY_NAME.to_owned(),
            TagAction::Delete => format!("Delete {name}…"),
            TagAction::DeleteThere => format!("Delete {name} from {remote}…"),
        }
    }

    fn item(&self, action: TagAction, label: &RefLabel) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (label.clone(), action);
        PopupMenuItem::new(self.text(action, label))
            .disabled(!self.available(action) || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

pub(crate) fn copy_tag_name(label: &RefLabel, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(label.name.to_string()));
    window.push_notification(NAME_COPIED, cx);
}

fn delete_question(label: &RefLabel, history: &History, remote: Option<&str>) -> DeleteQuestion {
    let name = &label.name;
    let lost = what_deleting_loses(name, history.commits_only_on(label));
    let description = match remote {
        Some(remote) => format!(
            "{lost} If {remote} has {name} too, the next fetch brings it back; \
             Delete {name} from {remote}… removes it there."
        ),
        None => lost,
    };
    DeleteQuestion {
        title: format!("Delete {name}?"),
        description,
    }
}

fn delete_there_question(label: &RefLabel, remote: &str) -> DeleteQuestion {
    let name = &label.name;
    DeleteQuestion {
        title: format!("Delete {name} from {remote}?"),
        description: format!(
            "The tag is removed from {remote} for everyone who uses it. \
             {name} stays here, so a push would send it back."
        ),
    }
}

pub(crate) fn confirm_delete(
    repository: Entity<Repository>,
    label: RefLabel,
    window: &mut Window,
    cx: &mut App,
) {
    let question = {
        let repository = repository.read(cx);
        let Some(history) = repository.history() else {
            return;
        };
        delete_question(&label, history, repository.push_remote().as_deref())
    };
    ask(question, window, cx, move |cx| {
        repository.update(cx, |repository, cx| repository.delete_tag(label.clone(), cx))
    });
}

pub(crate) fn confirm_delete_there(
    repository: Entity<Repository>,
    label: RefLabel,
    remote: String,
    window: &mut Window,
    cx: &mut App,
) {
    let question = delete_there_question(&label, &remote);
    ask(question, window, cx, move |cx| {
        repository.update(cx, |repository, cx| {
            repository.delete_tag_there(label.clone(), remote.clone(), cx)
        })
    });
}

fn ask(question: DeleteQuestion, window: &mut Window, cx: &mut App, on_delete: impl Fn(&mut App) + 'static) {
    let on_delete = Rc::new(on_delete);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let on_delete = on_delete.clone();
        alert
            .confirm()
            .title(question.title.clone())
            .description(question.description.clone())
            .ok_text(DELETE)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                on_delete(cx);
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{TagAction::*, TagMenu, delete_question, delete_there_question};
    use crate::git::{
        RefKind, RefLabel,
        fixtures::{id, linear_history},
    };

    fn tag(name: &str) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind: RefKind::Tag,
            target: id(1),
            is_head: false,
            upstream: None,
            from_upstream: None,
            worktree: None,
        }
    }

    fn texts(menu: &TagMenu, label: &RefLabel) -> Vec<Vec<String>> {
        menu.offered()
            .into_iter()
            .map(|group| group.into_iter().map(|action| menu.text(action, label)).collect())
            .collect()
    }

    #[test]
    fn with_a_remote_a_tag_is_pushed_and_deleted_there_by_name() {
        let menu = TagMenu::new(Some("origin".into()), false, false);
        assert_eq!(
            texts(&menu, &tag("v0.2.0")),
            [
                vec!["Check out v0.2.0"],
                vec!["Push v0.2.0 to origin"],
                vec!["Create branch here…"],
                vec!["Copy name"],
                vec!["Delete v0.2.0…", "Delete v0.2.0 from origin…"],
            ]
        );
    }

    #[test]
    fn without_a_remote_there_is_nothing_to_push_to_or_delete_from() {
        let menu = TagMenu::new(None, false, false);
        assert_eq!(menu.offered(), [vec![CheckOut], vec![BranchHere], vec![CopyName], vec![Delete]]);
    }

    #[test]
    fn while_git_writes_only_copy_name_stays_and_a_waiting_operation_blocks_switching() {
        let writing = TagMenu::new(Some("origin".into()), true, false);
        let usable = |menu: &TagMenu| {
            menu.offered()
                .into_iter()
                .flatten()
                .filter(|action| menu.available(*action))
                .collect::<Vec<_>>()
        };
        assert_eq!(usable(&writing), [CopyName]);
        let underway = TagMenu::new(Some("origin".into()), false, true);
        assert_eq!(usable(&underway), [Push, CopyName, Delete, DeleteThere]);
    }

    #[test]
    fn the_delete_questions_say_a_fetch_brings_the_tag_back_and_who_loses_it_there() {
        let history = linear_history(&[1, 2], Some(1));
        let v1 = tag("v1");
        let here = delete_question(&v1, &history, Some("origin"));
        assert_eq!(here.title, "Delete v1?");
        assert!(here.description.ends_with(
            "If origin has v1 too, the next fetch brings it back; Delete v1 from origin… removes it there."
        ));
        let alone = delete_question(&v1, &history, None);
        assert!(!alone.description.contains("fetch"));
        let there = delete_there_question(&v1, "origin");
        assert_eq!(there.title, "Delete v1 from origin?");
        assert_eq!(
            there.description,
            "The tag is removed from origin for everyone who uses it. v1 stays here, so a push would send it back."
        );
    }
}
