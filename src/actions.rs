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
        PreviousChange,
        NextChange,
        PreviousFile,
        NextFile,
        WholeWindow,
        PlayPause,
        SwitchVersion,
        LoopChange,
        CloseTab,
        NextTab,
        PreviousTab,
        About,
        Quit
    ]
);

pub(crate) const WINDOW_CONTEXT: &str = "FastGit";
pub(crate) const HISTORY_CONTEXT: &str = "History";
pub(crate) const MEDIA_CONTEXT: &str = "Media";
