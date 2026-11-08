use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use gpui_kit::{App, AppContext as _, BorrowAppContext as _, Global};

use crate::{
    net::{curl::Curl, services::cancel_pair},
    update::{
        self, Installation, Release, Staged, Then, UpdateError, another_copy_runs, update_automatically,
        updates_automatically, updates_folder,
    },
};

const FIRST_CHECK_AFTER: Duration = Duration::from_secs(20);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

pub(crate) const RESTART_TO_UPDATE: &str = "Restart to update";
pub(crate) const WHATS_NEW: &str = "What's new";
pub(crate) const UPDATE: &str = "Update";
pub(crate) const DOWNLOADING: &str = "Downloading…";
pub(crate) const OPEN_RELEASE_PAGE: &str = "Open the release page";
pub(crate) const UPDATE_THEN_RESTART: &str = "Update downloads it, then Restart to update installs it.";

pub(crate) fn ready_tip(release: &Release) -> String {
    format!(
        "SterneGit {} is downloaded. Restart to install it now; quitting installs it too.",
        release.version
    )
}

pub(crate) fn update_to(release: &Release) -> String {
    format!("Update to {}", release.version)
}

pub(crate) fn release_out(release: &Release) -> String {
    format!("SterneGit {} is out", release.version)
}

pub(crate) fn downloading(release: &Release) -> String {
    format!("Downloading SterneGit {}…", release.version)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    NotChecked,
    Checking,
    UpToDate,
    Out(Release),
    Downloading { release: Release, asked: bool },
    Ready { release: Release, staged: Staged },
    ReadOnly(Release),
    Failed(UpdateError),
    Installing,
}

pub(crate) struct Updates {
    installation: Installation,
    folder: Option<PathBuf>,
    automatic: bool,
    status: Status,
    checked_at: Option<SystemTime>,
    finished_checks: usize,
}

impl Global for Updates {}

impl Updates {
    pub(crate) fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn installation(&self) -> &Installation {
        &self.installation
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn automatic(&self) -> bool {
        self.automatic
    }

    pub(crate) fn checked_at(&self) -> Option<SystemTime> {
        self.checked_at
    }

    pub(crate) fn finished_checks(&self) -> usize {
        self.finished_checks
    }

    pub(crate) fn is_busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading { .. } | Status::Installing)
    }
}

pub(crate) fn install_updates(cx: &mut App) {
    let installation = Installation::of_this_sternegit();
    let folder = updates_folder();
    let updates = installation.feed().is_some();
    cx.set_global(Updates {
        installation: installation.clone(),
        folder: folder.clone(),
        automatic: updates_automatically(),
        status: Status::NotChecked,
        checked_at: None,
        finished_checks: 0,
    });
    if !updates {
        return;
    }
    cx.on_app_quit(|cx| {
        install_on_quit(cx);
        async {}
    })
    .detach();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(FIRST_CHECK_AFTER).await;
        if let Some(folder) = folder {
            cx.background_spawn(async move { update::tidy(&installation, &folder) }).await;
        }
        loop {
            cx.update(check);
            cx.background_executor().timer(CHECK_EVERY).await;
        }
    })
    .detach();
}

pub(crate) fn check(cx: &mut App) {
    let updates = Updates::of(cx);
    if updates.is_busy() {
        return;
    }
    let (Some(folder), true) = (updates.folder.clone(), updates.installation.feed().is_some()) else {
        return;
    };
    let installation = updates.installation.clone();
    let before = updates.status.clone();
    set_status(Status::Checking, cx);
    cx.spawn(async move |cx| {
        let found = cx
            .background_spawn(async move {
                let (_keep, cancel) = cancel_pair();
                update::newer_release(&Curl::system(), &installation, &folder, cancel).await
            })
            .await;
        cx.update(|cx| checked(found, before, cx));
    })
    .detach();
}

pub(crate) fn retry_after_network_failure(cx: &mut App) {
    if matches!(Updates::of(cx).status, Status::Failed(UpdateError::Network(_))) {
        check(cx);
    }
}

pub(crate) fn download(cx: &mut App) {
    let release = match &Updates::of(cx).status {
        Status::Out(release) => release.clone(),
        _ => return,
    };
    set_status(Status::Downloading { release: release.clone(), asked: true }, cx);
    stage(release, cx);
}

pub(crate) fn restart_to_update(cx: &mut App) {
    let updates = Updates::of(cx);
    let Status::Ready { staged, .. } = updates.status.clone() else {
        return;
    };
    let installation = updates.installation.clone();
    set_status(Status::Installing, cx);
    match update::apply(&staged, &installation, Then::Relaunch) {
        Ok(()) => cx.quit(),
        Err(error) => set_status(Status::Failed(error.into()), cx),
    }
}

pub(crate) fn set_automatic(automatic: bool, cx: &mut App) -> std::io::Result<()> {
    update_automatically(automatic)?;
    cx.update_global::<Updates, _>(|updates, _| updates.automatic = automatic);
    if automatic {
        download(cx);
    }
    Ok(())
}

fn checked(found: Result<Option<Release>, UpdateError>, before: Status, cx: &mut App) {
    let automatic = Updates::of(cx).automatic;
    let succeeded = found.is_ok();
    let next = match (found, before) {
        (Ok(Some(release)), Status::Ready { release: ready, staged }) if ready.version == release.version => {
            Status::Ready { release: ready, staged }
        }
        (Ok(Some(release)), _) if automatic => Status::Downloading { release, asked: false },
        (Ok(Some(release)), _) => Status::Out(release),
        (Ok(None), _) => Status::UpToDate,
        (Err(_), ready @ Status::Ready { .. }) => ready,
        (Err(error), _) => Status::Failed(error),
    };
    cx.update_global::<Updates, _>(|updates, _| {
        updates.finished_checks += 1;
        if succeeded {
            updates.checked_at = Some(SystemTime::now());
        }
        updates.status = next.clone();
    });
    if let Status::Downloading { release, .. } = next {
        stage(release, cx);
    }
}

fn stage(release: Release, cx: &mut App) {
    let updates = Updates::of(cx);
    let Some(folder) = updates.folder.clone() else {
        set_status(Status::Failed(UpdateError::Disk("There's no folder to download into".into())), cx);
        return;
    };
    let installation = updates.installation.clone();
    cx.spawn(async move |cx| {
        let wanted = release.clone();
        let staged = cx
            .background_spawn(async move {
                let (_keep, cancel) = cancel_pair();
                update::stage(&Curl::system(), &wanted, &installation, &folder, cancel).await
            })
            .await;
        let next = match staged {
            Ok(staged) => Status::Ready { release, staged },
            Err(UpdateError::ReadOnlyFolder) => Status::ReadOnly(release),
            Err(error) => Status::Failed(error),
        };
        cx.update(|cx| set_status(next, cx));
    })
    .detach();
}

fn install_on_quit(cx: &mut App) {
    let updates = Updates::of(cx);
    let Status::Ready { staged, .. } = &updates.status else {
        return;
    };
    let Some(exe) = updates.installation.exe() else {
        return;
    };
    if !another_copy_runs(exe) {
        let _ = update::apply(staged, &updates.installation, Then::Stay);
    }
}

fn set_status(status: Status, cx: &mut App) {
    cx.update_global::<Updates, _>(|updates, _| updates.status = status);
}

pub(crate) fn checked_ago(elapsed: Duration) -> String {
    let minutes = elapsed.as_secs() / 60;
    let (hours, days) = (minutes / 60, minutes / (60 * 24));
    let ago = |count: u64, one: &str, many: &str| format!("{count} {} ago", if count == 1 { one } else { many });
    match (days, hours, minutes) {
        (0, 0, 0) => "just now".to_owned(),
        (0, 0, minutes) => ago(minutes, "minute", "minutes"),
        (0, hours, _) => ago(hours, "hour", "hours"),
        (days, _, _) => ago(days, "day", "days"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::checked_ago;

    #[test]
    fn the_last_check_reads_in_the_largest_whole_unit() {
        let after = |seconds: u64| checked_ago(Duration::from_secs(seconds));
        assert_eq!(after(20), "just now");
        assert_eq!(after(60), "1 minute ago");
        assert_eq!(after(4 * 60 + 59), "4 minutes ago");
        assert_eq!(after(3600), "1 hour ago");
        assert_eq!(after(5 * 3600 + 120), "5 hours ago");
        assert_eq!(after(2 * 86_400), "2 days ago");
    }
}
