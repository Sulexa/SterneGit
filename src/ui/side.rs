use gpui_kit::assets::IconName;

use crate::git::Side;

pub(crate) struct SideLabels {
    pub(crate) files: &'static str,
    pub(crate) state: &'static str,
    pub(crate) verb: &'static str,
    pub(crate) all: &'static str,
    pub(crate) file_tooltip: &'static str,
    pub(crate) folder_tooltip: &'static str,
    pub(crate) open_file_tooltip: &'static str,
    pub(crate) empty: &'static str,
    pub(crate) icon: IconName,
}

const UNSTAGED: SideLabels = SideLabels {
    files: "Unstaged files",
    state: "unstaged",
    verb: "Stage",
    all: "Stage all",
    file_tooltip: "Stage file",
    folder_tooltip: "Stage folder",
    open_file_tooltip: "Stage this file",
    empty: "Every change is staged.",
    icon: IconName::Plus,
};

const STAGED: SideLabels = SideLabels {
    files: "Staged files",
    state: "staged",
    verb: "Unstage",
    all: "Unstage all",
    file_tooltip: "Unstage file",
    folder_tooltip: "Unstage folder",
    open_file_tooltip: "Unstage this file",
    empty: "Stage files to commit them.",
    icon: IconName::Minus,
};

pub(crate) trait SideUi {
    fn labels(self) -> &'static SideLabels;
}

impl SideUi for Side {
    fn labels(self) -> &'static SideLabels {
        match self {
            Side::Unstaged => &UNSTAGED,
            Side::Staged => &STAGED,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct BySide<T> {
    unstaged: T,
    staged: T,
}

impl<T> BySide<T> {
    pub(crate) fn new(mut make: impl FnMut(Side) -> T) -> Self {
        Self {
            unstaged: make(Side::Unstaged),
            staged: make(Side::Staged),
        }
    }

    pub(crate) fn of(&self, side: Side) -> &T {
        match side {
            Side::Unstaged => &self.unstaged,
            Side::Staged => &self.staged,
        }
    }

    pub(crate) fn of_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Unstaged => &mut self.unstaged,
            Side::Staged => &mut self.staged,
        }
    }
}
