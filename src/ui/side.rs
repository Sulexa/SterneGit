use gpui_kit::assets::IconName;

use crate::git::Side;

pub(crate) struct SideLabels {
    pub(crate) files: &'static str,
    pub(crate) state: &'static str,
    pub(crate) verb: &'static str,
    pub(crate) all: &'static str,
    pub(crate) file_tooltip: &'static str,
    pub(crate) open_file_tooltip: &'static str,
    pub(crate) icon: IconName,
}

const UNSTAGED: SideLabels = SideLabels {
    files: "Unstaged files",
    state: "unstaged",
    verb: "Stage",
    all: "Stage all",
    file_tooltip: "Stage file",
    open_file_tooltip: "Stage this file",
    icon: IconName::Plus,
};

const STAGED: SideLabels = SideLabels {
    files: "Staged files",
    state: "staged",
    verb: "Unstage",
    all: "Unstage all",
    file_tooltip: "Unstage file",
    open_file_tooltip: "Unstage this file",
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
