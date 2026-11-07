gpui_kit::actions!(
    sternegit,
    [
        OlderCommit,
        NewerCommit,
        Refresh,
        Fetch,
        OpenRepository,
        CloneRepository,
        NewRepository,
        OpenSettings,
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
        ZoomIn,
        ZoomOut,
        ResetZoom,
        About,
        Quit
    ]
);

pub(crate) const WINDOW_CONTEXT: &str = "SterneGit";
pub(crate) const HISTORY_CONTEXT: &str = "History";
pub(crate) const MEDIA_CONTEXT: &str = "Media";
