use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

use anyhow::Result;
use gix::{
    ObjectId,
    refs::FullName,
    revision::walk::{Info, Sorting},
    traverse::commit::simple::CommitTimeOrder,
};
use gpui_kit::SharedString;

use super::{
    graph::{self, Graph},
    refs::{self, RefDigest, RefLabel, ShownRef},
    stash::{Stash, stash_ids_newest_first, stash_name},
    text::decode,
    time::format_time,
    underway::rebased_branch,
};

pub(crate) const HISTORY_LIMIT: usize = 10_000;
const SHORT_ID_LENGTH: usize = 7;

#[derive(Clone, Debug)]
pub(crate) struct Commit {
    pub(crate) id: ObjectId,
    pub(crate) short_id: SharedString,
    pub(crate) parents: Vec<ObjectId>,
    pub(crate) subject: SharedString,
    pub(crate) author: SharedString,
    pub(crate) author_email: SharedString,
    pub(crate) authored: SharedString,
    pub(crate) committed: SharedString,
    committed_at: i64,
}

impl Commit {
    fn read(info: &Info<'_>) -> Result<Self> {
        Self::of(info.id, info.parent_ids.iter().copied().collect(), &info.object()?)
    }

    pub(crate) fn find(repo: &gix::Repository, id: ObjectId) -> Result<Self> {
        let commit = repo.find_commit(id)?;
        let parents = commit.parent_ids().map(|parent| parent.detach()).collect();
        Self::of(id, parents, &commit)
    }

    fn joined_to_its_base_only(mut self) -> Self {
        self.parents.truncate(1);
        self
    }

    fn of(id: ObjectId, parents: Vec<ObjectId>, commit: &gix::Commit<'_>) -> Result<Self> {
        let author = commit.author()?;
        let authored = author.time()?;
        let committed = commit.time()?;
        Ok(Self {
            id,
            short_id: short_id(id),
            parents,
            subject: decode(&commit.message()?.summary()).into(),
            author: decode(author.name).into(),
            author_email: decode(author.email).into(),
            authored: format_time(authored.seconds, authored.offset).into(),
            committed: format_time(committed.seconds, committed.offset).into(),
            committed_at: committed.seconds,
        })
    }
}

pub(crate) fn short_id(id: ObjectId) -> SharedString {
    id.to_hex_with_len(SHORT_ID_LENGTH).to_string().into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowKind {
    Commit,
    Stash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowPlace {
    pub(crate) kind: RowKind,
    pub(crate) number: usize,
    pub(crate) count: usize,
}

pub(crate) struct History {
    pub(crate) head_label: Option<SharedString>,
    pub(crate) branch: Option<SharedString>,
    pub(crate) head_id: Option<ObjectId>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) refs: Vec<RefLabel>,
    pub(crate) stashes: Vec<Stash>,
    pub(crate) graph: Graph,
    pub(crate) truncated: bool,
    pub(crate) elapsed: Duration,
    pub(crate) digest: RefDigest,
    rows: HashMap<ObjectId, usize>,
    stash_rows: Vec<usize>,
    refs_by_commit: HashMap<ObjectId, Vec<usize>>,
}

struct Head {
    name: Option<FullName>,
    id: Option<ObjectId>,
    rebasing: Option<String>,
}

impl Head {
    fn read(repo: &gix::Repository) -> Result<Self> {
        Ok(Self {
            name: repo.head_name()?,
            id: repo.head_id().ok().map(|id| id.detach()),
            rebasing: rebased_branch(repo),
        })
    }

    fn branch(&self) -> Option<SharedString> {
        self.name
            .as_ref()
            .map(|name| decode(name.shorten()).into())
    }

    fn label(&self) -> Option<SharedString> {
        match (&self.name, self.id) {
            _ if let Some(branch) = &self.rebasing => Some(format!("rebasing {branch}").into()),
            (Some(name), _) => Some(decode(name.shorten()).into()),
            (None, Some(id)) => Some(format!("detached at {}", short_id(id)).into()),
            (None, None) => None,
        }
    }
}

struct Walked {
    commits: Vec<Commit>,
    truncated: bool,
}

impl History {
    pub(crate) fn read(repo: &gix::Repository, started: Instant) -> Result<Self> {
        let digest_never_newer_than_what_is_read = RefDigest::of(repo)?;
        let head = Head::read(repo)?;
        let mut refs = refs::read_labels(repo, head.name.as_ref())?;
        refs::count_from_upstreams(repo, &mut refs);
        let (stashes, stashed): (Vec<Stash>, Vec<Commit>) = read_stashes(repo).into_iter().unzip();
        let bases = stashed.iter().filter_map(|stash| stash.parents.first().copied());
        let mut walked = walk_newest_first(repo, tips(&refs, head.id, bases))?;
        add_stash_rows(&mut walked.commits, stashed);
        Ok(Self::new(
            head,
            in_date_order(walked.commits),
            refs,
            stashes,
            walked.truncated,
            digest_never_newer_than_what_is_read,
            started,
        ))
    }

    fn new(
        head: Head,
        commits: Vec<Commit>,
        refs: Vec<RefLabel>,
        stashes: Vec<Stash>,
        truncated: bool,
        digest: RefDigest,
        started: Instant,
    ) -> Self {
        let rows: HashMap<ObjectId, usize> = commits
            .iter()
            .enumerate()
            .map(|(row, commit)| (commit.id, row))
            .collect();
        let mut stash_rows: Vec<usize> = stashes
            .iter()
            .filter_map(|stash| rows.get(&stash.id).copied())
            .collect();
        stash_rows.sort_unstable();
        stash_rows.dedup();
        let graph = graph::layout(
            commits
                .iter()
                .map(|commit| (&commit.id, commit.parents.as_slice())),
        );
        let mut refs_by_commit: HashMap<ObjectId, Vec<usize>> = HashMap::new();
        for (ix, label) in refs.iter().enumerate() {
            refs_by_commit.entry(label.target).or_default().push(ix);
        }
        Self {
            head_label: head.label(),
            branch: head.branch(),
            head_id: head.id,
            commits,
            refs,
            stashes,
            graph,
            truncated,
            elapsed: started.elapsed(),
            digest,
            rows,
            stash_rows,
            refs_by_commit,
        }
    }

    pub(crate) fn row_of(&self, id: &ObjectId) -> Option<usize> {
        self.rows.get(id).copied()
    }

    pub(crate) fn newest(&self) -> Option<ObjectId> {
        self.commits.first().map(|commit| commit.id)
    }

    pub(crate) fn stash_index(&self, id: &ObjectId) -> Option<usize> {
        self.stashes.iter().position(|stash| stash.id == *id)
    }

    pub(crate) fn stash_on(&self, id: &ObjectId) -> Option<&Stash> {
        self.stash_index(id).map(|index| &self.stashes[index])
    }

    pub(crate) fn commit_count(&self) -> usize {
        self.commits.len() - self.stash_rows.len()
    }

    pub(crate) fn place_of(&self, id: &ObjectId) -> Option<RowPlace> {
        if let Some(index) = self.stash_index(id) {
            return Some(RowPlace {
                kind: RowKind::Stash,
                number: index + 1,
                count: self.stashes.len(),
            });
        }
        let row = self.row_of(id)?;
        let stash_rows_above = self.stash_rows.partition_point(|&stash_row| stash_row < row);
        Some(RowPlace {
            kind: RowKind::Commit,
            number: row - stash_rows_above + 1,
            count: self.commit_count(),
        })
    }

    pub(crate) fn labels_on(&self, id: &ObjectId) -> impl Iterator<Item = &RefLabel> {
        self.refs_by_commit
            .get(id)
            .into_iter()
            .flatten()
            .map(|&ix| &self.refs[ix])
    }

    pub(crate) fn shown_on(&self, id: &ObjectId) -> Vec<ShownRef<'_>> {
        let labels: Vec<&RefLabel> = self.labels_on(id).collect();
        refs::fold_upstreams_in(&labels)
    }
}

fn tips(
    refs: &[RefLabel],
    head: Option<ObjectId>,
    stash_bases: impl IntoIterator<Item = ObjectId>,
) -> Vec<ObjectId> {
    let mut tips: Vec<ObjectId> = refs
        .iter()
        .map(|label| label.target)
        .chain(head)
        .chain(stash_bases)
        .collect();
    tips.sort_unstable();
    tips.dedup();
    tips
}

fn read_stashes(repo: &gix::Repository) -> Vec<(Stash, Commit)> {
    stash_ids_newest_first(repo)
        .into_iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let commit = Commit::find(repo, id).ok()?.joined_to_its_base_only();
            let stash = Stash {
                id,
                name: stash_name(index).into(),
                message: commit.subject.clone(),
            };
            Some((stash, commit))
        })
        .collect()
}

fn add_stash_rows(commits: &mut Vec<Commit>, stashed: Vec<Commit>) {
    for stash in stashed {
        if !commits.iter().any(|commit| commit.id == stash.id) {
            commits.push(stash);
        }
    }
}

fn walk_newest_first(repo: &gix::Repository, tips: Vec<ObjectId>) -> Result<Walked> {
    let mut commits = Vec::new();
    if tips.is_empty() {
        return Ok(Walked {
            commits,
            truncated: false,
        });
    }
    let walk = repo
        .rev_walk(tips)
        .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .use_commit_graph(false)
        .all()?;
    for info in walk {
        let info = info?;
        if commits.len() == HISTORY_LIMIT {
            return Ok(Walked {
                commits,
                truncated: true,
            });
        }
        commits.push(Commit::read(&info)?);
    }
    Ok(Walked {
        commits,
        truncated: false,
    })
}

fn in_date_order(commits: Vec<Commit>) -> Vec<Commit> {
    let rows: HashMap<ObjectId, usize> = commits
        .iter()
        .enumerate()
        .map(|(row, commit)| (commit.id, row))
        .collect();
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
        .map(|(row, _)| (commits[row].committed_at, Reverse(row)))
        .collect();
    let mut order = Vec::with_capacity(commits.len());
    while let Some((_, Reverse(row))) = ready.pop() {
        order.push(row);
        for parent in &commits[row].parents {
            if let Some(&parent_row) = rows.get(parent) {
                unplaced_children[parent_row] -= 1;
                if unplaced_children[parent_row] == 0 {
                    ready.push((commits[parent_row].committed_at, Reverse(parent_row)));
                }
            }
        }
    }
    let mut slots: Vec<Option<Commit>> = commits.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|row| slots[row].take())
        .collect()
}

#[cfg(test)]
pub(crate) mod fixtures {
    use std::time::Instant;

    use gix::ObjectId;

    use super::{Commit, Head, History};
    use crate::git::RefDigest;

    pub(crate) fn id(byte: u8) -> ObjectId {
        ObjectId::from_bytes_or_panic(&[byte; 20])
    }

    pub(crate) fn commit(byte: u8, parents: &[u8], committed_at: i64) -> Commit {
        Commit {
            id: id(byte),
            short_id: Default::default(),
            parents: parents.iter().copied().map(id).collect(),
            subject: Default::default(),
            author: Default::default(),
            author_email: Default::default(),
            authored: Default::default(),
            committed: Default::default(),
            committed_at,
        }
    }

    pub(crate) fn linear_history(newest_first: &[u8], head: Option<u8>) -> History {
        let commits = newest_first
            .iter()
            .enumerate()
            .map(|(row, &byte)| {
                let parent = newest_first.get(row + 1).copied();
                commit(byte, parent.as_slice(), -(row as i64))
            })
            .collect();
        History::new(
            Head {
                name: None,
                id: head.map(id),
                rebasing: None,
            },
            commits,
            Vec::new(),
            Vec::new(),
            false,
            RefDigest::default(),
            Instant::now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{RowKind, RowPlace, fixtures::commit, in_date_order};
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn a_skewed_clock_cannot_put_a_parent_above_its_child() {
        let walked = vec![commit(1, &[3], 20), commit(3, &[], 10), commit(2, &[3], 5)];
        let ordered: Vec<u8> = in_date_order(walked)
            .iter()
            .map(|commit| commit.id.as_bytes()[0])
            .collect();
        assert_eq!(ordered, [1, 2, 3]);
    }

    #[test]
    fn a_latin_1_commit_message_reads_as_its_accented_letters() -> Result<()> {
        let scratch = ScratchRepo::new("latin-1-message")?;
        std::fs::write(
            scratch.dir.join("message.txt"),
            b"Caf\xE9 cr\xE8me\n\nCorps du message, \xE2g\xE9\n",
        )?;
        scratch.git(&[
            "-c",
            "i18n.commitEncoding=ISO-8859-1",
            "commit",
            "--allow-empty",
            "--no-verify",
            "--quiet",
            "-F",
            "message.txt",
        ])?;
        let (repo, history) = scratch.open()?;
        let commit = &history.commits[0];
        assert_eq!(&*commit.subject, "Café crème");
        assert_eq!(
            &*repo.commit_details(commit.id)?.body,
            "Corps du message, âgé"
        );
        Ok(())
    }

    #[test]
    fn a_stash_is_one_row_joined_to_the_commit_it_was_made_on() -> Result<()> {
        let scratch = ScratchRepo::new("stash-row")?;
        scratch.write("tracked.txt", "one\n")?;
        scratch.git(&["add", "tracked.txt"])?;
        scratch.commit("first")?;
        scratch.write("tracked.txt", "two\n")?;
        scratch.git(&["add", "tracked.txt"])?;
        scratch.write("untracked.txt", "new\n")?;
        scratch.git(&["stash", "push", "--include-untracked", "--quiet"])?;
        let base = scratch.head()?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits.len(), 2, "no row for the index or the untracked files");
        let stash = &history.commits[0];
        assert_eq!(history.stash_index(&stash.id), Some(0));
        let parents: Vec<String> = stash.parents.iter().map(ToString::to_string).collect();
        assert_eq!(parents, [base]);
        assert!(!history.graph.rows[0].is_merge);
        Ok(())
    }

    #[test]
    fn commit_positions_skip_the_stash_rows() -> Result<()> {
        let scratch = ScratchRepo::new("stash-place")?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "two\n")?;
        scratch.git(&["stash", "push", "--quiet"])?;
        let (_, history) = scratch.open()?;
        let (stash, first) = (history.commits[0].id, history.commits[1].id);
        assert_eq!(history.commit_count(), 1);
        assert_eq!(
            history.place_of(&stash),
            Some(RowPlace {
                kind: RowKind::Stash,
                number: 1,
                count: 1
            })
        );
        assert_eq!(
            history.place_of(&first),
            Some(RowPlace {
                kind: RowKind::Commit,
                number: 1,
                count: 1
            })
        );
        Ok(())
    }

    #[test]
    fn a_stash_keeps_the_commit_it_was_made_on_after_its_branch_is_gone() -> Result<()> {
        let scratch = ScratchRepo::new("stash-orphan-base")?;
        scratch.commit("first")?;
        scratch.git(&["checkout", "--quiet", "-b", "topic"])?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("on topic")?;
        scratch.write("a.txt", "two\n")?;
        scratch.git(&["stash", "push", "--quiet"])?;
        scratch.git(&["checkout", "--quiet", "main"])?;
        scratch.git(&["branch", "--quiet", "-D", "topic"])?;
        let (_, history) = scratch.open()?;
        let stash = history.row_of(&history.stashes[0].id).expect("the stash has a row");
        let base = history.commits[stash].parents[0];
        assert!(history.row_of(&base).is_some());
        assert_eq!(history.commits.len(), 3);
        Ok(())
    }

    #[test]
    fn a_detached_head_is_labelled_with_its_short_id() -> Result<()> {
        let scratch = ScratchRepo::new("detached")?;
        scratch.commit("first")?;
        scratch.commit("second")?;
        scratch.git(&["checkout", "--quiet", "--detach", "HEAD~1"])?;
        let (_, history) = scratch.open()?;
        let head = history.head_id.expect("HEAD points at a commit");
        assert_eq!(
            history.head_label.as_deref(),
            Some(format!("detached at {}", super::short_id(head)).as_str())
        );
        assert_eq!(history.commits.len(), 2);
        Ok(())
    }
}
