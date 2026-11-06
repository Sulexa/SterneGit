use std::path::Path;

use anyhow::Result;
use gix::{ObjectId, state::InProgress};

use super::{
    Carried, Repo,
    command::{Exit, run},
    history::{Commit, short_id},
    refs::{RefKind, RefLabel, read_labels},
};

const REBASE_FOLDERS: [&str; 2] = ["rebase-merge", "rebase-apply"];
const CHERRY_PICK_HEAD: &str = "CHERRY_PICK_HEAD";
const REVERT_HEAD: &str = "REVERT_HEAD";
const BRANCHES: &str = "refs/heads/";
const QUOTE: char = '\'';

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Underway {
    Rebase(Rebasing),
    Merge(Merging),
    CherryPick(Picking),
    Revert(Picking),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Picking {
    pub(crate) branch: String,
    pub(crate) commit: Replaying,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Rebasing {
    pub(crate) branch: String,
    pub(crate) onto: String,
    pub(crate) step: usize,
    pub(crate) total: usize,
    pub(crate) replaying: Option<Replaying>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Replaying {
    pub(crate) id: String,
    pub(crate) subject: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Merging {
    pub(crate) from: String,
    pub(crate) into: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Continued {
    Finished(Carried),
    Emptied(Carried),
    StoppedAgain,
}

impl Underway {
    fn command(&self) -> &'static str {
        match self {
            Underway::Rebase(_) => "rebase",
            Underway::Merge(_) => "merge",
            Underway::CherryPick(_) => "cherry-pick",
            Underway::Revert(_) => "revert",
        }
    }

    fn finish_args(&self) -> [&'static str; 2] {
        [self.command(), "--continue"]
    }

    fn abort_args(&self) -> [&'static str; 2] {
        [self.command(), "--abort"]
    }

    fn skip_args(&self) -> [&'static str; 2] {
        [self.command(), "--skip"]
    }

    fn picks(&self) -> bool {
        matches!(self, Underway::CherryPick(_) | Underway::Revert(_))
    }
}

impl Repo {
    pub(crate) fn underway(&self) -> Option<Underway> {
        underway_in(&self.local())
    }

    pub(crate) fn continue_underway(&self) -> Result<Continued> {
        let Some(underway) = self.underway() else {
            return Ok(Continued::Finished(Carried::Along));
        };
        let before = self.latest_stash();
        match run(&self.location, &underway.finish_args(), Exit::Success) {
            Ok(_) => self.done_with(&underway, before).map(Continued::Finished),
            Err(_) if self.underway().is_some() && self.has_conflicts() => {
                Ok(Continued::StoppedAgain)
            }
            Err(_) if underway.picks() && self.underway().is_some() => {
                self.skip_underway()?;
                self.done_with(&underway, before).map(Continued::Emptied)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn abort_underway(&self) -> Result<Carried> {
        let Some(underway) = self.underway() else {
            return Ok(Carried::Along);
        };
        let before = self.latest_stash();
        run(&self.location, &underway.abort_args(), Exit::Success)?;
        self.done_with(&underway, before)
    }

    pub(super) fn skip_underway(&self) -> Result<()> {
        match self.underway() {
            Some(underway) => run(&self.location, &underway.skip_args(), Exit::Success).map(drop),
            None => Ok(()),
        }
    }

    fn done_with(&self, underway: &Underway, before: Option<ObjectId>) -> Result<Carried> {
        if underway.picks() {
            self.bring_back_set_aside()
        } else {
            Ok(self.carried_since(before))
        }
    }
}

pub(super) fn underway_in(repo: &gix::Repository) -> Option<Underway> {
    match repo.state()? {
        InProgress::Rebase | InProgress::RebaseInteractive | InProgress::ApplyMailboxRebase => {
            rebasing(repo).map(Underway::Rebase)
        }
        InProgress::Merge => merging(repo).map(Underway::Merge),
        InProgress::CherryPick | InProgress::CherryPickSequence => {
            picking(repo, CHERRY_PICK_HEAD).map(Underway::CherryPick)
        }
        InProgress::Revert | InProgress::RevertSequence => {
            picking(repo, REVERT_HEAD).map(Underway::Revert)
        }
        _ => None,
    }
}

fn picking(repo: &gix::Repository, head_file: &str) -> Option<Picking> {
    let hex = read_trimmed(&repo.git_dir().join(head_file))?;
    let id = ObjectId::from_hex(hex.as_bytes()).ok()?;
    let subject = Commit::find(repo, id).map_or_else(|_| String::new(), |commit| commit.subject.to_string());
    Some(Picking {
        branch: head_branch(repo),
        commit: Replaying {
            id: short_id(id).to_string(),
            subject,
        },
    })
}

fn head_branch(repo: &gix::Repository) -> String {
    repo.head_name()
        .ok()
        .flatten()
        .map_or_else(|| "HEAD".to_owned(), |name| name.shorten().to_string())
}

pub(super) fn rebased_branch(repo: &gix::Repository) -> Option<String> {
    rebasing(repo).map(|rebasing| rebasing.branch)
}

fn rebasing(repo: &gix::Repository) -> Option<Rebasing> {
    let folder = REBASE_FOLDERS
        .iter()
        .map(|name| repo.git_dir().join(name))
        .find(|folder| folder.is_dir())?;
    let read = |name: &str| read_trimmed(&folder.join(name));
    let branch = read("head-name").map_or_else(
        || "HEAD".to_owned(),
        |name| name.strip_prefix(BRANCHES).unwrap_or(&name).to_owned(),
    );
    let onto = read("onto")
        .and_then(|hex| ObjectId::from_hex(hex.as_bytes()).ok())
        .map(|id| name_for(repo, id, &branch))
        .unwrap_or_default();
    let number = |first: &str, second: &str| {
        read(first)
            .or_else(|| read(second))
            .and_then(|text| text.parse().ok())
            .unwrap_or(0)
    };
    Some(Rebasing {
        step: number("msgnum", "next"),
        total: number("end", "last"),
        replaying: read("done").and_then(|done| last_replayed(&done)),
        branch,
        onto,
    })
}

fn merging(repo: &gix::Repository) -> Option<Merging> {
    let git_dir = repo.git_dir();
    let head = read_trimmed(&git_dir.join("MERGE_HEAD"))?;
    let into = head_branch(repo);
    let from = read_trimmed(&git_dir.join("MERGE_MSG"))
        .and_then(|message| quoted_name(&message))
        .or_else(|| {
            let id = ObjectId::from_hex(head.lines().next()?.as_bytes()).ok()?;
            Some(name_for(repo, id, &into))
        })
        .unwrap_or_default();
    Some(Merging { from, into })
}

fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
}

fn last_replayed(done: &str) -> Option<Replaying> {
    let line = done
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;
    let mut words = line.splitn(3, ' ');
    let _command = words.next()?;
    let id = words.next()?;
    let id = ObjectId::from_hex(id.as_bytes())
        .map(short_id)
        .map_or_else(|_| id.to_owned(), |short| short.to_string());
    Some(Replaying {
        id,
        subject: words.next().unwrap_or_default().to_owned(),
    })
}

fn quoted_name(message: &str) -> Option<String> {
    let first = message.lines().next()?;
    let (_, rest) = first.split_once(QUOTE)?;
    let (name, _) = rest.split_once(QUOTE)?;
    Some(name.to_owned())
}

fn name_for(repo: &gix::Repository, id: ObjectId, branch: &str) -> String {
    let labels = read_labels(repo, None).unwrap_or_default();
    let upstream = upstream_name(repo, branch);
    let pointing = |label: &&RefLabel| label.target == id && label.name.as_str() != branch;
    let named = |kind: RefKind| {
        labels
            .iter()
            .filter(pointing)
            .find(|label| label.kind == kind)
    };
    labels
        .iter()
        .filter(pointing)
        .find(|label| Some(label.name.as_str()) == upstream.as_deref())
        .or_else(|| named(RefKind::LocalBranch))
        .or_else(|| named(RefKind::RemoteBranch))
        .or_else(|| named(RefKind::Tag))
        .map_or_else(|| short_id(id).to_string(), |label| label.name.to_string())
}

fn upstream_name(repo: &gix::Repository, branch: &str) -> Option<String> {
    let config = repo.config_snapshot();
    let remote = config.string(format!("branch.{branch}.remote").as_str())?;
    let merge = config.string(format!("branch.{branch}.merge").as_str())?;
    let merge = merge.to_string();
    Some(format!(
        "{remote}/{}",
        merge.strip_prefix(BRANCHES).unwrap_or(&merge)
    ))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Continued, Merging, Rebasing, Replaying, Underway, last_replayed, quoted_name};
    use crate::git::{Carried, Rebased, scratch::ScratchRepo};

    fn diverged(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["branch", "topic"])?;
        scratch.write("shared.txt", "main\n")?;
        scratch.git(&["commit", "--quiet", "-am", "Change it on main"])?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        scratch.write("shared.txt", "topic\n")?;
        scratch.git(&["commit", "--quiet", "-am", "Change it on topic"])?;
        scratch.write("other.txt", "two\n")?;
        scratch.git(&["add", "other.txt"])?;
        scratch.commit("Add another file")?;
        Ok(scratch)
    }

    #[test]
    fn a_stopped_rebase_names_both_branches_and_the_commit_it_replays() -> Result<()> {
        let scratch = diverged("underway-rebase")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.underway(), None);
        assert_eq!(repo.rebase_onto("main")?.0, Rebased::Stopped);
        let Some(Underway::Rebase(rebasing)) = repo.underway() else {
            panic!("a rebase should be underway");
        };
        let replayed = scratch.git(&["rev-parse", "--short=7", "topic~1"])?;
        assert_eq!(
            rebasing,
            Rebasing {
                branch: "topic".into(),
                onto: "main".into(),
                step: 1,
                total: 2,
                replaying: Some(Replaying {
                    id: replayed.trim().into(),
                    subject: "Change it on topic".into(),
                }),
            }
        );
        Ok(())
    }

    #[test]
    fn continuing_after_the_conflicts_are_staged_finishes_the_rebase() -> Result<()> {
        let scratch = diverged("underway-continue")?;
        let (repo, _) = scratch.open()?;
        repo.rebase_onto("main")?;
        assert_eq!(repo.continue_underway()?, Continued::StoppedAgain);
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(
            repo.continue_underway()?,
            Continued::Finished(Carried::Along)
        );
        assert_eq!(repo.underway(), None);
        assert_eq!(
            scratch.git(&["log", "--format=%s", "-3"])?,
            "Add another file\nChange it on topic\nChange it on main\n"
        );
        Ok(())
    }

    #[test]
    fn a_commit_the_resolution_empties_is_left_out() -> Result<()> {
        let scratch = diverged("underway-empty")?;
        let (repo, _) = scratch.open()?;
        repo.rebase_onto("main")?;
        scratch.git(&["checkout", "--ours", "shared.txt"])?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(
            repo.continue_underway()?,
            Continued::Finished(Carried::Along)
        );
        assert_eq!(
            scratch.git(&["log", "--format=%s", "-2"])?,
            "Add another file\nChange it on main\n"
        );
        Ok(())
    }

    #[test]
    fn aborting_puts_the_branch_back_and_the_stashed_changes_with_it() -> Result<()> {
        let scratch = diverged("underway-abort")?;
        let before = scratch.head()?;
        scratch.write("other.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.rebase_onto("main")?.0, Rebased::Stopped);
        assert_eq!(repo.abort_underway()?, Carried::Along);
        assert_eq!(repo.underway(), None);
        assert_eq!(scratch.head()?, before);
        assert_eq!(std::fs::read_to_string(scratch.dir.join("other.txt"))?, "draft\n");
        Ok(())
    }

    #[test]
    fn a_stopped_merge_names_what_comes_in() -> Result<()> {
        let scratch = diverged("underway-merge")?;
        let (repo, _) = scratch.open()?;
        repo.merge("main")?;
        assert_eq!(
            repo.underway(),
            Some(Underway::Merge(Merging {
                from: "main".into(),
                into: "topic".into(),
            }))
        );
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(
            repo.continue_underway()?,
            Continued::Finished(Carried::Along)
        );
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%s"])?.trim(),
            "Merge branch 'main' into topic"
        );
        Ok(())
    }

    #[test]
    fn the_replayed_commit_and_the_merged_name_come_from_gits_files() {
        assert_eq!(
            last_replayed("pick 2205c5f Fix it\n# comment\npick abc1234 Show conflicted files\n\n"),
            Some(Replaying {
                id: "abc1234".into(),
                subject: "Show conflicted files".into(),
            })
        );
        assert_eq!(
            quoted_name("Merge remote-tracking branch 'origin/main' into topic\n\n# Conflicts:"),
            Some("origin/main".into())
        );
        assert_eq!(quoted_name("Squashed"), None);
    }
}
