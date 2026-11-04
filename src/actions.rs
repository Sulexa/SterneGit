gpui_kit::actions!(
    fastgit,
    [
        OlderCommit,
        NewerCommit,
        Refresh,
        Fetch,
        OpenRepository,
        CopyHash,
        CloseDiff,
        CloseTab,
        NextTab,
        PreviousTab,
        About,
        Quit
    ]
);

pub(crate) const WINDOW_CONTEXT: &str = "FastGit";
pub(crate) const HISTORY_CONTEXT: &str = "History";
