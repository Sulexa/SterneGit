use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

use anyhow::Result;
use gix::{
    ObjectId,
    bstr::ByteSlice as _,
    refs::FullName,
    revision::walk::{Info, Sorting},
    traverse::commit::simple::CommitTimeOrder,
};
use gpui_kit::SharedString;

use super::{
    graph::{self, Graph},
    refs::{self, RefDigest, RefLabel},
    time::format_time,
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
    pub(crate) authored: SharedString,
    pub(crate) committed: SharedString,
    committed_at: i64,
}

impl Commit {
    fn read(info: &Info<'_>) -> Result<Self> {
        let commit = info.object()?;
        let author = commit.author()?;
        let authored = author.time()?;
        let committed = commit.time()?;
        Ok(Self {
            id: info.id,
            short_id: short_id(info.id),
            parents: info.parent_ids.iter().copied().collect(),
            subject: commit
                .message()?
                .summary()
                .to_str_lossy()
                .into_owned()
                .into(),
            author: author.name.to_str_lossy().into_owned().into(),
            authored: format_time(authored.seconds, authored.offset).into(),
            committed: format_time(committed.seconds, committed.offset).into(),
            committed_at: committed.seconds,
        })
    }
}

pub(crate) fn short_id(id: ObjectId) -> SharedString {
    id.to_hex_with_len(SHORT_ID_LENGTH).to_string().into()
}

pub(crate) struct History {
    pub(crate) head_label: Option<SharedString>,
    pub(crate) head_id: Option<ObjectId>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) refs: Vec<RefLabel>,
    pub(crate) graph: Graph,
    pub(crate) truncated: bool,
    pub(crate) elapsed: Duration,
    pub(crate) digest: RefDigest,
    rows: HashMap<ObjectId, usize>,
    refs_by_commit: HashMap<ObjectId, Vec<usize>>,
}

struct Head {
    name: Option<FullName>,
    id: Option<ObjectId>,
}

impl Head {
    fn read(repo: &gix::Repository) -> Result<Self> {
        Ok(Self {
            name: repo.head_name()?,
            id: repo.head_id().ok().map(|id| id.detach()),
        })
    }

    fn label(&self) -> Option<SharedString> {
        match (&self.name, self.id) {
            (Some(name), _) => Some(name.shorten().to_str_lossy().into_owned().into()),
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
        let refs = refs::read_labels(repo, head.name.as_ref())?;
        let walked = walk_newest_first(repo, tips(&refs, head.id))?;
        Ok(Self::new(
            head,
            in_date_order(walked.commits),
            refs,
            walked.truncated,
            digest_never_newer_than_what_is_read,
            started,
        ))
    }

    fn new(
        head: Head,
        commits: Vec<Commit>,
        refs: Vec<RefLabel>,
        truncated: bool,
        digest: RefDigest,
        started: Instant,
    ) -> Self {
        let rows = commits
            .iter()
            .enumerate()
            .map(|(row, commit)| (commit.id, row))
            .collect();
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
            head_id: head.id,
            commits,
            refs,
            graph,
            truncated,
            elapsed: started.elapsed(),
            digest,
            rows,
            refs_by_commit,
        }
    }

    pub(crate) fn row_of(&self, id: &ObjectId) -> Option<usize> {
        self.rows.get(id).copied()
    }

    pub(crate) fn newest(&self) -> Option<ObjectId> {
        self.commits.first().map(|commit| commit.id)
    }

    pub(crate) fn labels_on(&self, id: &ObjectId) -> impl Iterator<Item = &RefLabel> {
        self.refs_by_commit
            .get(id)
            .into_iter()
            .flatten()
            .map(|&ix| &self.refs[ix])
    }
}

fn tips(refs: &[RefLabel], head: Option<ObjectId>) -> Vec<ObjectId> {
    let mut tips: Vec<ObjectId> = refs.iter().map(|label| label.target).chain(head).collect();
    tips.sort_unstable();
    tips.dedup();
    tips
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
            },
            commits,
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

    use super::{fixtures::commit, in_date_order};
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
