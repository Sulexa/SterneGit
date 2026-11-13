use gpui_kit::{AnyElement, IntoElement, component::menu::ContextMenuExt};

use super::{branch_menu::BranchMenu, tag_menu::TagMenu};
use crate::git::{RefKind, RefLabel};

#[derive(Clone, Default)]
pub(crate) struct RefMenus {
    pub(crate) branch: Option<BranchMenu>,
    pub(crate) tag: Option<TagMenu>,
}

impl RefMenus {
    pub(crate) fn attach<E>(&self, element: E, label: &RefLabel) -> AnyElement
    where
        E: ContextMenuExt + IntoElement + 'static,
    {
        let right_clicked = label.clone();
        match (label.kind, &self.branch, &self.tag) {
            (RefKind::LocalBranch | RefKind::RemoteBranch, Some(menu), _) => {
                let menu = menu.clone();
                element
                    .context_menu(move |popup, window, cx| menu.build(popup, &right_clicked, window, cx))
                    .into_any_element()
            }
            (RefKind::Tag, _, Some(menu)) => {
                let menu = menu.clone();
                element
                    .context_menu(move |popup, _, _| menu.build(popup, &right_clicked))
                    .into_any_element()
            }
            _ => element.into_any_element(),
        }
    }
}
