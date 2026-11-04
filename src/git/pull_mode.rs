use anyhow::Result;

use super::{
    Repo,
    command::{Exit, run},
};

const SAVED_UNDER: &str = "fastgit.pullMode";
const GIT_REBASES_ON_PULL: &str = "pull.rebase";
const GIT_FAST_FORWARDS_ON_PULL: &str = "pull.ff";
const GIT_FALSE: [&str; 5] = ["false", "no", "off", "0", ""];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PullMode {
    #[default]
    FastForwardIfPossible,
    FastForwardOnly,
    Rebase,
    FetchOnly,
}

impl PullMode {
    pub(crate) const ALL: [PullMode; 4] = [
        PullMode::FastForwardIfPossible,
        PullMode::FastForwardOnly,
        PullMode::Rebase,
        PullMode::FetchOnly,
    ];

    pub(crate) fn pull_args(self) -> Option<&'static [&'static str]> {
        match self {
            PullMode::FastForwardIfPossible => Some(&["--no-rebase", "--ff"]),
            PullMode::FastForwardOnly => Some(&["--no-rebase", "--ff-only"]),
            PullMode::Rebase => Some(&["--rebase"]),
            PullMode::FetchOnly => None,
        }
    }

    fn saved_as(self) -> &'static str {
        match self {
            PullMode::FastForwardIfPossible => "merge",
            PullMode::FastForwardOnly => "ff-only",
            PullMode::Rebase => "rebase",
            PullMode::FetchOnly => "fetch",
        }
    }

    fn saved(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.saved_as().eq_ignore_ascii_case(value.trim()))
    }

    fn following_git(rebase: Option<&str>, fast_forward: Option<&str>) -> Self {
        let rebases = rebase.is_some_and(|value| {
            !GIT_FALSE
                .iter()
                .any(|no| no.eq_ignore_ascii_case(value.trim()))
        });
        let fast_forward_only =
            fast_forward.is_some_and(|value| value.trim().eq_ignore_ascii_case("only"));
        if rebases {
            PullMode::Rebase
        } else if fast_forward_only {
            PullMode::FastForwardOnly
        } else {
            PullMode::FastForwardIfPossible
        }
    }
}

impl Repo {
    pub(crate) fn pull_mode(&self) -> PullMode {
        let repo = self.local();
        let config = repo.config_snapshot();
        let value = |key: &str| config.string(key).map(|value| value.to_string());
        value(SAVED_UNDER)
            .as_deref()
            .and_then(PullMode::saved)
            .unwrap_or_else(|| {
                PullMode::following_git(
                    value(GIT_REBASES_ON_PULL).as_deref(),
                    value(GIT_FAST_FORWARDS_ON_PULL).as_deref(),
                )
            })
    }

    pub(crate) fn save_pull_mode(&self, mode: PullMode) -> Result<()> {
        run(
            &self.location,
            &["config", "--local", SAVED_UNDER, mode.saved_as()],
            Exit::Success,
        )
        .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::PullMode;
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn without_a_saved_mode_pull_follows_gits_own_settings() -> Result<()> {
        let scratch = ScratchRepo::new("pull-mode-git")?;
        scratch.commit("first")?;
        assert_eq!(scratch.open()?.0.pull_mode(), PullMode::FastForwardIfPossible);
        scratch.git(&["config", "pull.ff", "only"])?;
        assert_eq!(scratch.open()?.0.pull_mode(), PullMode::FastForwardOnly);
        scratch.git(&["config", "pull.rebase", "true"])?;
        assert_eq!(scratch.open()?.0.pull_mode(), PullMode::Rebase);
        scratch.git(&["config", "pull.rebase", "false"])?;
        assert_eq!(scratch.open()?.0.pull_mode(), PullMode::FastForwardOnly);
        Ok(())
    }

    #[test]
    fn a_saved_mode_wins_over_gits_settings_and_survives_a_reopen() -> Result<()> {
        let scratch = ScratchRepo::new("pull-mode-saved")?;
        scratch.commit("first")?;
        scratch.git(&["config", "pull.rebase", "true"])?;
        for mode in PullMode::ALL {
            scratch.open()?.0.save_pull_mode(mode)?;
            assert_eq!(scratch.open()?.0.pull_mode(), mode);
        }
        assert_eq!(scratch.git(&["config", "pull.rebase"])?.trim(), "true");
        Ok(())
    }

    #[test]
    fn an_unknown_saved_mode_falls_back_to_gits_settings() {
        assert_eq!(PullMode::saved(" Rebase "), Some(PullMode::Rebase));
        assert_eq!(PullMode::saved("squash"), None);
        assert_eq!(
            PullMode::following_git(Some("merges"), None),
            PullMode::Rebase
        );
        assert_eq!(
            PullMode::following_git(Some("off"), Some("true")),
            PullMode::FastForwardIfPossible
        );
    }
}
