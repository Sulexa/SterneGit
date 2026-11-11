use std::collections::HashSet;

use anyhow::{Result, bail};
use gix::{ObjectId, bstr::ByteSlice as _};

use super::{
    History, RefKind, Repo, RewordScope,
    history::{Commit, short_id},
    reword::{Copied, Rewrite},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) newest: ObjectId,
    pub(crate) oldest: ObjectId,
    pub(crate) base: Option<ObjectId>,
    pub(crate) merges: bool,
}

impl History {
    pub(crate) fn in_graph_order<'a>(&self, ids: impl IntoIterator<Item = &'a ObjectId>) -> Vec<ObjectId> {
        let mut rows: Vec<usize> = ids.into_iter().filter_map(|id| self.row_of(id)).collect();
        rows.sort_unstable();
        rows.dedup();
        rows.into_iter().map(|row| self.commits[row].id).collect()
    }

    pub(crate) fn pickable(&self) -> Vec<ObjectId> {
        (0..self.commits.len())
            .filter(|&row| self.worktree_on_row(row).is_none())
            .map(|row| &self.commits[row])
            .filter(|commit| self.stash_index(&commit.id).is_none())
            .map(|commit| commit.id)
            .collect()
    }

    pub(crate) fn picked_commits(&self, newest_first: &[ObjectId]) -> Vec<Commit> {
        newest_first
            .iter()
            .filter_map(|id| self.row_of(id))
            .map(|row| self.commits[row].clone())
            .collect()
    }

    pub(crate) fn line_of(&self, newest_first: &[ObjectId]) -> Option<Line> {
        if newest_first.len() < 2 {
            return None;
        }
        let commits = newest_first
            .iter()
            .map(|id| self.row_of(id).map(|row| &self.commits[row]))
            .collect::<Option<Vec<_>>>()?;
        let follows = commits
            .windows(2)
            .all(|pair| pair[0].parents.first() == Some(&pair[1].id));
        let (newest, oldest) = (commits.first()?, commits.last()?);
        follows.then(|| Line {
            newest: newest.id,
            oldest: oldest.id,
            base: oldest.parents.first().copied(),
            merges: commits.iter().any(|commit| commit.parents.len() > 1),
        })
    }

    pub(crate) fn squash_scope(&self, line: &Line, newest_first: &[ObjectId]) -> Option<RewordScope> {
        if line.merges {
            return None;
        }
        let newest = self.reword_scope(&line.newest)?;
        let oldest = self.reword_scope(&line.oldest)?;
        let picked: HashSet<&ObjectId> = newest_first.iter().collect();
        let tags = self
            .refs
            .iter()
            .filter(|label| label.kind == RefKind::Tag)
            .filter(|label| picked.contains(&label.target) || newest.tags.contains(&label.name))
            .map(|label| label.name.clone())
            .collect();
        Some(RewordScope {
            after: newest.after,
            branches: newest.branches,
            remotes: oldest.remotes,
            tags,
        })
    }
}

impl Repo {
    pub(crate) fn squash(&self, newest: ObjectId, oldest: ObjectId, message: &str) -> Result<ObjectId> {
        let message = self.cleaned_message(message)?;
        let repo = self.local();
        let count = commits_in_line(&repo, newest, oldest)?;
        let tree = repo.find_commit(newest)?.tree_id()?.detach();
        let reflog = format!("SterneGit: squash {count} commits into one");
        let copied = Copied {
            message: Some(message.as_bstr()),
            tree: Some(tree),
        };
        self.rewrite_above(newest, oldest, copied, &reflog, Rewrite::Squash)
    }
}

fn commits_in_line(repo: &gix::Repository, newest: ObjectId, oldest: ObjectId) -> Result<usize> {
    let mut at = newest;
    let mut count = 1;
    loop {
        let parents: Vec<ObjectId> = repo
            .find_commit(at)?
            .parent_ids()
            .map(|parent| parent.detach())
            .collect();
        if parents.len() > 1 {
            bail!("{} is a merge, so it can't be squashed", short_id(at));
        }
        if at == oldest {
            return Ok(count);
        }
        let Some(&parent) = parents.first() else {
            bail!("{} doesn't come before {}", short_id(oldest), short_id(newest));
        };
        at = parent;
        count += 1;
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::Line;
    use crate::git::{RewordScope, scratch::ScratchRepo};

    fn id(scratch: &ScratchRepo, revision: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_hex(scratch.git(&["rev-parse", revision])?.trim().as_bytes())?)
    }

    fn ids(scratch: &ScratchRepo, revisions: &[&str]) -> Result<Vec<ObjectId>> {
        revisions.iter().map(|revision| id(scratch, revision)).collect()
    }

    fn four_on_main(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        for (file, subject) in [("a.txt", "Add a"), ("b.txt", "Add b"), ("c.txt", "Add c"), ("d.txt", "Add d")] {
            scratch.write(file, subject)?;
            scratch.git(&["add", file])?;
            scratch.git(&["commit", "--quiet", "-m", subject])?;
        }
        Ok(scratch)
    }

    #[test]
    fn commits_each_the_first_parent_of_the_one_above_make_a_line() -> Result<()> {
        let scratch = four_on_main("line-of")?;
        scratch.git(&["switch", "--quiet", "--create=topic", "main~2"])?;
        scratch.commit("On topic")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let (_, history) = scratch.open()?;
        let line = ids(&scratch, &["main~1", "main~2", "main~3"])?;
        assert_eq!(
            history.line_of(&line),
            Some(Line {
                newest: line[0],
                oldest: line[2],
                base: None,
                merges: false,
            })
        );
        assert_eq!(history.line_of(&ids(&scratch, &["main", "main~2"])?), None);
        assert_eq!(history.line_of(&ids(&scratch, &["topic", "main~1"])?), None);
        assert_eq!(history.line_of(&ids(&scratch, &["main"])?), None);
        let topic = ids(&scratch, &["topic", "main~2"])?;
        assert_eq!(history.line_of(&topic).map(|line| line.base), Some(Some(id(&scratch, "main~3")?)));
        Ok(())
    }

    #[test]
    fn picks_follow_the_graph_and_leave_stashes_out() -> Result<()> {
        let scratch = four_on_main("pickable")?;
        scratch.write("a.txt", "changed")?;
        scratch.git(&["stash", "push", "--quiet"])?;
        let (_, history) = scratch.open()?;
        let shown = ids(&scratch, &["main", "main~1", "main~2", "main~3"])?;
        assert_eq!(history.pickable(), shown);
        let scrambled = [shown[2], shown[0], shown[2], shown[1]];
        assert_eq!(history.in_graph_order(&scrambled), shown[..3]);
        Ok(())
    }

    #[test]
    fn squashing_keeps_the_newest_files_the_oldest_author_and_the_commits_after() -> Result<()> {
        let scratch = four_on_main("squash")?;
        scratch.git(&["tag", "inside", "main~2"])?;
        scratch.git(&["update-ref", "refs/remotes/origin/main", "main~3"])?;
        let tree = scratch.git(&["rev-parse", "main~1^{tree}"])?;
        let author = scratch.git(&["log", "-1", "--format=%an %ae %ad", "main~2"])?;
        let status = scratch.git(&["status", "--porcelain"])?;
        let (repo, history) = scratch.open()?;
        let (newest, oldest) = (id(&scratch, "main~1")?, id(&scratch, "main~2")?);
        let line = history.line_of(&[newest, oldest]).expect("a line");
        assert_eq!(
            history.squash_scope(&line, &[newest, oldest]),
            Some(RewordScope {
                after: 1,
                branches: vec!["main".into()],
                remotes: Vec::new(),
                tags: vec!["inside".into()],
            })
        );
        let squashed = repo.squash(newest, oldest, "Add b and c\n\nBoth files at once.")?;
        assert_eq!(id(&scratch, "main~1")?, squashed);
        assert_eq!(scratch.git(&["log", "--format=%s", "main"])?, "Add d\nAdd b and c\nAdd a\n");
        assert_eq!(scratch.git(&["rev-parse", "main~1^{tree}"])?, tree);
        assert_eq!(scratch.git(&["log", "-1", "--format=%an %ae %ad", "main~1"])?, author);
        assert_eq!(scratch.git(&["log", "-1", "--format=%b", "main~1"])?, "Both files at once.\n\n");
        assert_eq!(id(&scratch, "inside")?, oldest);
        assert_eq!(scratch.git(&["status", "--porcelain"])?, status);
        Ok(())
    }

    #[test]
    fn a_pushed_oldest_commit_is_named_and_a_merge_is_never_squashed() -> Result<()> {
        let scratch = four_on_main("squash-merge")?;
        scratch.git(&["switch", "--quiet", "--create=topic", "main~1"])?;
        scratch.commit("On topic")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.git(&["merge", "--quiet", "--no-ff", "--no-edit", "topic"])?;
        scratch.git(&["update-ref", "refs/remotes/origin/main", "main~2"])?;
        let (repo, history) = scratch.open()?;
        let pushed = ids(&scratch, &["main~1", "main~2"])?;
        let line = history.line_of(&pushed).expect("a line");
        let scope = history.squash_scope(&line, &pushed).expect("a scope");
        assert_eq!(scope.remotes, vec![gpui_kit::SharedString::from("origin/main")]);
        let merged = ids(&scratch, &["main", "main~1"])?;
        let line = history.line_of(&merged).expect("a line through the merge");
        assert!(line.merges);
        assert_eq!(history.squash_scope(&line, &merged), None);
        assert!(repo.squash(merged[0], merged[1], "Never").is_err());
        assert!(repo.squash(id(&scratch, "main~1")?, id(&scratch, "topic")?, "Not a line").is_err());
        Ok(())
    }
}
