//! Reading a repository.
//!
//! History and refs come from gix inside the process, so a refresh spawns no
//! processes however large the history is. A commit's patch comes from
//! `git show`: one spawn per selected commit, off every hot path, producing
//! output every Git user already trusts.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, anyhow, bail};
use gix::{
    ObjectId, ThreadSafeRepository, bstr::ByteSlice as _, refs::Category,
    revision::walk::Sorting, traverse::commit::simple::CommitTimeOrder,
};
use gpui_kit::{SharedString, component::diff::DiffFile};

use crate::graph::{self, Graph};

/// The graph only lays out the rows on screen, so a large window stays smooth.
pub(crate) const HISTORY_LIMIT: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Clone, Debug)]
pub(crate) struct RefLabel {
    pub(crate) name: SharedString,
    pub(crate) kind: RefKind,
    pub(crate) target: ObjectId,
    /// Whether HEAD points at this branch.
    pub(crate) is_head: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Commit {
    pub(crate) id: ObjectId,
    pub(crate) short_id: SharedString,
    pub(crate) parents: Vec<ObjectId>,
    pub(crate) subject: SharedString,
    pub(crate) author: SharedString,
    pub(crate) authored: SharedString,
    /// The commit date. The history is ordered by it, so the list shows it.
    pub(crate) committed: SharedString,
    time: i64,
}

pub(crate) struct History {
    pub(crate) name: SharedString,
    /// The checked-out branch, or where a detached HEAD points.
    pub(crate) head: Option<SharedString>,
    pub(crate) head_id: Option<ObjectId>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) rows: HashMap<ObjectId, usize>,
    /// Sorted by kind, then name.
    pub(crate) refs: Vec<RefLabel>,
    pub(crate) refs_by_commit: HashMap<ObjectId, Vec<usize>>,
    pub(crate) graph: Graph,
    /// Whether older commits exist beyond [`HISTORY_LIMIT`].
    pub(crate) truncated: bool,
    pub(crate) elapsed: Duration,
    repo: ThreadSafeRepository,
    /// Where `git` runs: the work tree, or the Git directory of a bare repository.
    location: PathBuf,
}

pub(crate) struct CommitDetails {
    /// The message below the subject line.
    pub(crate) body: SharedString,
    pub(crate) files: Vec<DiffFile>,
}

pub(crate) fn load(path: &Path) -> Result<History> {
    let started = Instant::now();
    let mut repo = gix::discover(path)
        .with_context(|| format!("{} isn't inside a Git repository", path.display()))?;
    // Walking by commit time looks each commit up twice; a small cache keeps
    // that from decoding it twice.
    repo.object_cache_size_if_unset(4 * 1024 * 1024);

    let location = repo.workdir().unwrap_or_else(|| repo.git_dir());
    // A repository found from "." would otherwise be named ".".
    let location = std::path::absolute(location).unwrap_or_else(|_| location.to_owned());
    let name = location
        .file_name()
        .map_or_else(|| location.display().to_string(), |name| name.to_string_lossy().into_owned())
        .into();

    let head_name = repo.head_name()?;
    let head_id = repo.head_id().ok().map(|id| id.detach());
    let head = match (&head_name, head_id) {
        (Some(name), _) => Some(name.shorten().to_str_lossy().into_owned().into()),
        (None, Some(id)) => Some(format!("detached at {}", id.to_hex_with_len(7)).into()),
        (None, None) => None,
    };

    let mut refs = Vec::new();
    for reference in repo.references()?.all()? {
        // One broken ref shouldn't hide the rest of the history.
        let Ok(mut reference) = reference else { continue };
        let Some((category, short_name)) = reference.name().category_and_short_name() else {
            continue;
        };
        let kind = match category {
            Category::LocalBranch => RefKind::LocalBranch,
            Category::RemoteBranch => RefKind::RemoteBranch,
            Category::Tag => RefKind::Tag,
            _ => continue,
        };
        // `origin/HEAD` only names another remote branch.
        if kind == RefKind::RemoteBranch && short_name.ends_with(b"/HEAD") {
            continue;
        }
        let name = short_name.to_str_lossy().into_owned().into();
        let is_head = kind == RefKind::LocalBranch
            && head_name
                .as_ref()
                .is_some_and(|head| head.as_bstr() == reference.name().as_bstr());
        // Tags can point at trees or blobs; only commits belong in the graph.
        let Ok(commit) = reference.peel_to_commit() else { continue };
        refs.push(RefLabel {
            name,
            kind,
            target: commit.id,
            is_head,
        });
    }
    refs.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.as_str().cmp(b.name.as_str())));

    let mut tips: Vec<ObjectId> = refs.iter().map(|label| label.target).chain(head_id).collect();
    tips.sort_unstable();
    tips.dedup();

    let mut commits = Vec::new();
    let mut truncated = false;
    if !tips.is_empty() {
        let walk = repo
            .rev_walk(tips)
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
            // Every commit gets decoded for its message anyway.
            .use_commit_graph(false)
            .all()?;
        for info in walk {
            let info = info?;
            if commits.len() == HISTORY_LIMIT {
                truncated = true;
                break;
            }
            let commit = info.object()?;
            let author = commit.author()?;
            let authored = author.time()?;
            let committed = commit.time()?;
            commits.push(Commit {
                id: info.id,
                short_id: info.id.to_hex_with_len(7).to_string().into(),
                parents: info.parent_ids.iter().copied().collect(),
                subject: commit.message()?.summary().to_str_lossy().into_owned().into(),
                author: author.name.to_str_lossy().into_owned().into(),
                authored: format_time(authored.seconds, authored.offset).into(),
                committed: format_time(committed.seconds, committed.offset).into(),
                time: committed.seconds,
            });
        }
    }

    let commits = in_date_order(commits);
    let rows = commits.iter().enumerate().map(|(row, commit)| (commit.id, row)).collect();
    let graph = graph::layout(commits.iter().map(|commit| (&commit.id, commit.parents.as_slice())));
    let mut refs_by_commit: HashMap<ObjectId, Vec<usize>> = HashMap::new();
    for (ix, label) in refs.iter().enumerate() {
        refs_by_commit.entry(label.target).or_default().push(ix);
    }

    Ok(History {
        name,
        head,
        head_id,
        commits,
        rows,
        refs,
        refs_by_commit,
        graph,
        truncated,
        elapsed: started.elapsed(),
        repo: repo.into_sync(),
        location,
    })
}

impl History {
    /// The work tree, or the Git directory of a bare repository.
    pub(crate) fn location(&self) -> &Path {
        &self.location
    }

    /// Returns a job that reads one commit's message and patch, to run off the
    /// UI thread.
    pub(crate) fn details_job(
        &self,
        id: ObjectId,
    ) -> impl FnOnce() -> Result<CommitDetails> + Send + 'static {
        let repo = self.repo.clone();
        let location = self.location.clone();
        move || {
            let repo = repo.to_thread_local();
            let commit = repo.find_commit(id)?;
            let message = commit.message_raw_sloppy().to_str_lossy();
            let body = message
                .split_once("\n\n")
                .map_or("", |(_, body)| body.trim())
                .to_owned()
                .into();
            // A merge is compared with its first parent: a two-sided diff can't
            // show Git's combined format.
            let patch = git(
                &location,
                &[
                    "show",
                    "--no-show-signature",
                    "--no-color",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--format=",
                    "--first-parent",
                    "--patch",
                    "--find-renames",
                    "--src-prefix=a/",
                    "--dst-prefix=b/",
                    &id.to_string(),
                    "--",
                ],
            )?;
            let files = DiffFile::parse(&patch).map_err(|error| anyhow!("{error}"))?;
            Ok(CommitDetails { body, files })
        }
    }
}

fn git(location: &Path, args: &[&str]) -> Result<String> {
    let mut command = Command::new("git");
    command.arg("--no-pager").arg("-C").arg(location).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // A windowed app would otherwise flash a console window for every call.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .context("Couldn't run git. Is it installed and on PATH?")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    String::from_utf8(output.stdout).map_err(|_| anyhow!("This commit's patch isn't UTF-8."))
}

/// Orders commits newest first while keeping every child above its parents,
/// like `git log --date-order`. Commit times alone can't promise that: a clock
/// that ran behind gives a child an older time than its parent.
fn in_date_order(commits: Vec<Commit>) -> Vec<Commit> {
    let rows: HashMap<ObjectId, usize> =
        commits.iter().enumerate().map(|(row, commit)| (commit.id, row)).collect();
    let mut unplaced_children = vec![0usize; commits.len()];
    for commit in &commits {
        for parent in &commit.parents {
            if let Some(&row) = rows.get(parent) {
                unplaced_children[row] += 1;
            }
        }
    }
    let mut ready: BinaryHeap<(i64, Reverse<usize>)> = unplaced_children
        .iter()
        .enumerate()
        .filter(|(_, count)| **count == 0)
        .map(|(row, _)| (commits[row].time, Reverse(row)))
        .collect();
    let mut order = Vec::with_capacity(commits.len());
    while let Some((_, Reverse(row))) = ready.pop() {
        order.push(row);
        for parent in &commits[row].parents {
            if let Some(&parent_row) = rows.get(parent) {
                unplaced_children[parent_row] -= 1;
                if unplaced_children[parent_row] == 0 {
                    ready.push((commits[parent_row].time, Reverse(parent_row)));
                }
            }
        }
    }
    let mut slots: Vec<Option<Commit>> = commits.into_iter().map(Some).collect();
    order.into_iter().filter_map(|row| slots[row].take()).collect()
}

/// Formats a timestamp in the author's own time zone, as `git log` does.
fn format_time(seconds: i64, offset: i32) -> String {
    let local = seconds + i64::from(offset);
    let (year, month, day) = civil_from_days(local.div_euclid(86_400));
    let minutes = local.rem_euclid(86_400) / 60;
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", minutes / 60, minutes % 60)
}

/// Converts days since 1970-01-01 to a proleptic Gregorian date, after
/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = (if month_index < 10 { month_index + 3 } else { month_index - 9 }) as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(byte: u8, parents: &[u8], time: i64) -> Commit {
        let id = |byte: u8| ObjectId::from_bytes_or_panic(&[byte; 20]);
        Commit {
            id: id(byte),
            short_id: SharedString::default(),
            parents: parents.iter().copied().map(id).collect(),
            subject: SharedString::default(),
            author: SharedString::default(),
            authored: SharedString::default(),
            committed: SharedString::default(),
            time,
        }
    }

    #[test]
    fn times_use_the_authors_offset() {
        assert_eq!(format_time(0, 0), "1970-01-01 00:00");
        assert_eq!(format_time(1_793_707_200, 0), "2026-11-03 12:00");
        assert_eq!(format_time(1_793_707_200, 3_600), "2026-11-03 13:00");
        assert_eq!(format_time(1_793_707_200, -13 * 3_600), "2026-11-02 23:00");
        assert_eq!(format_time(1_709_164_800, 0), "2024-02-29 00:00");
    }

    #[test]
    fn a_skewed_clock_cannot_put_a_parent_above_its_child() {
        // The walk met the parent (time 10) before its second child, whose
        // clock was behind (time 5).
        let walked = vec![commit(1, &[3], 20), commit(3, &[], 10), commit(2, &[3], 5)];
        let ordered: Vec<u8> = in_date_order(walked)
            .iter()
            .map(|commit| commit.id.as_bytes()[0])
            .collect();
        assert_eq!(ordered, [1, 2, 3]);
    }
}
