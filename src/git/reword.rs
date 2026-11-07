use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use gix::{
    ObjectId,
    bstr::{BStr, ByteSlice as _},
    objs::CommitRef,
};
use gpui_kit::SharedString;

use super::{
    History, Repo, RefKind,
    command::{Exit, run, run_with_input},
    short_id,
    text::decode,
};

const SIGNATURES: [&[u8]; 2] = [b"gpgsig", b"gpgsig-sha256"];
const DETACHED: &str = "HEAD";
const BRANCHES: &[u8] = b"refs/heads/";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RewordScope {
    pub(crate) after: usize,
    pub(crate) branches: Vec<SharedString>,
    pub(crate) remotes: Vec<SharedString>,
    pub(crate) tags: Vec<SharedString>,
}

impl History {
    pub(crate) fn reword_scope(&self, id: &ObjectId) -> Option<RewordScope> {
        let target = self.row_of(id)?;
        if self.stash_index(id).is_some() {
            return None;
        }
        let holds = self.holding(target);
        let held_row = |id: &ObjectId| self.row_of(id).filter(|row| holds[*row]);
        let moved: Vec<_> = self
            .refs
            .iter()
            .filter(|label| label.kind == RefKind::LocalBranch)
            .filter_map(|label| Some((label.name.clone(), held_row(&label.target)?)))
            .chain(
                self.head_id
                    .filter(|_| self.branch.is_none())
                    .and_then(|head| Some((DETACHED.into(), held_row(&head)?))),
            )
            .collect();
        if moved.is_empty() {
            return None;
        }
        let mut rewritten = vec![false; self.commits.len()];
        let mut pending: Vec<usize> = moved.iter().map(|(_, row)| *row).collect();
        while let Some(row) = pending.pop() {
            if !holds[row] || std::mem::replace(&mut rewritten[row], true) {
                continue;
            }
            pending.extend(self.commits[row].parents.iter().filter_map(|parent| self.row_of(parent)));
        }
        let named = |kind: RefKind, rows: &[bool]| {
            self.refs
                .iter()
                .filter(|label| label.kind == kind)
                .filter(|label| self.row_of(&label.target).is_some_and(|row| rows[row]))
                .map(|label| label.name.clone())
                .collect()
        };
        Some(RewordScope {
            after: rewritten.iter().filter(|rewritten| **rewritten).count() - 1,
            branches: moved.into_iter().map(|(name, _)| name).collect(),
            remotes: named(RefKind::RemoteBranch, &holds),
            tags: named(RefKind::Tag, &rewritten),
        })
    }

    fn holding(&self, target: usize) -> Vec<bool> {
        let mut holds = vec![false; self.commits.len()];
        holds[target] = true;
        for row in (0..target).rev() {
            holds[row] = self.commits[row]
                .parents
                .iter()
                .any(|parent| self.row_of(parent).is_some_and(|parent| holds[parent]));
        }
        holds
    }
}

struct Tip {
    name: Vec<u8>,
    id: ObjectId,
}

struct Descent<'a> {
    parents: &'a HashMap<ObjectId, Vec<ObjectId>>,
    target: ObjectId,
    holds: HashMap<ObjectId, bool>,
    oldest_first: Vec<ObjectId>,
}

impl Descent<'_> {
    fn holds(&mut self, tip: ObjectId) -> bool {
        let mut pending = vec![(tip, false)];
        while let Some((commit, parents_known)) = pending.pop() {
            if self.holds.contains_key(&commit) {
                continue;
            }
            let Some(parents) = self.parents.get(&commit) else {
                self.holds.insert(commit, commit == self.target);
                continue;
            };
            if parents_known {
                let holds = parents.iter().any(|parent| self.holds[parent]);
                self.holds.insert(commit, holds);
                if holds {
                    self.oldest_first.push(commit);
                }
            } else {
                pending.push((commit, true));
                let unknown = parents.iter().filter(|parent| !self.holds.contains_key(*parent));
                pending.extend(unknown.map(|parent| (*parent, false)));
            }
        }
        self.holds[&tip]
    }
}

impl Repo {
    pub(crate) fn reword(&self, id: ObjectId, message: &str) -> Result<ObjectId> {
        let message = run_with_input(&self.location, &["stripspace"], message.as_bytes())?;
        if message.is_empty() {
            bail!("A commit message can't be empty");
        }
        let repo = self.local();
        let tips = local_tips(&repo)?;
        let parents = walk_above(&repo, &tips, id)?;
        let mut descent = Descent {
            parents: &parents,
            target: id,
            holds: HashMap::new(),
            oldest_first: Vec::new(),
        };
        let moved: Vec<&Tip> = tips.iter().filter(|tip| descent.holds(tip.id)).collect();
        if moved.is_empty() {
            bail!("No local branch holds {}", short_id(id));
        }
        let committer = run(&self.location, &["var", "GIT_COMMITTER_IDENT"], Exit::Success)?;
        let committer = committer.trim_end().as_bstr();
        let mut rewritten = HashMap::new();
        let reworded = copy_commit(&repo, id, &rewritten, committer, Some(message.as_bstr()))?;
        rewritten.insert(id, reworded);
        for commit in descent.oldest_first {
            let copy = copy_commit(&repo, commit, &rewritten, committer, None)?;
            rewritten.insert(commit, copy);
        }
        let reflog = format!("SterneGit: edit the message of {}", short_id(id));
        self.move_tips(&repo, &moved, &rewritten, &reflog)?;
        Ok(reworded)
    }

    fn move_tips(
        &self,
        repo: &gix::Repository,
        moved: &[&Tip],
        rewritten: &HashMap<ObjectId, ObjectId>,
        reflog: &str,
    ) -> Result<()> {
        let mut updates = Vec::new();
        for tip in moved {
            updates.extend_from_slice(b"update ");
            updates.extend_from_slice(&tip.name);
            updates.extend_from_slice(format!(" {} {}\n", rewritten[&tip.id], tip.id).as_bytes());
        }
        let args = ["update-ref", "-m", reflog, "--stdin"];
        match run_with_input(&self.location, &args, &updates) {
            Ok(_) => Ok(()),
            Err(error) => Err(match moved_meanwhile(repo, moved) {
                Some(name) => anyhow!(
                    "{name} moved while the message was saved, so nothing changed. Save again to rewrite it from where it is now."
                ),
                None => error,
            }),
        }
    }
}

fn moved_meanwhile(repo: &gix::Repository, moved: &[&Tip]) -> Option<String> {
    let now = local_tips(repo).ok()?;
    let tip = moved.iter().find(|tip| {
        !now
            .iter()
            .any(|current| current.name == tip.name && current.id == tip.id)
    })?;
    let name = tip.name.strip_prefix(BRANCHES).unwrap_or(&tip.name);
    Some(decode(name))
}

fn local_tips(repo: &gix::Repository) -> Result<Vec<Tip>> {
    let mut tips = Vec::new();
    for reference in repo.references()?.local_branches()? {
        let Ok(reference) = reference else { continue };
        if let Some(id) = reference.try_id() {
            tips.push(Tip {
                name: reference.name().as_bstr().to_vec(),
                id: id.detach(),
            });
        }
    }
    if repo.head_name()?.is_none()
        && let Ok(head) = repo.head_id()
    {
        tips.push(Tip {
            name: DETACHED.into(),
            id: head.detach(),
        });
    }
    Ok(tips)
}

fn walk_above(
    repo: &gix::Repository,
    tips: &[Tip],
    id: ObjectId,
) -> Result<HashMap<ObjectId, Vec<ObjectId>>> {
    let mut parents = HashMap::new();
    if tips.is_empty() {
        return Ok(parents);
    }
    let walk = repo
        .rev_walk(tips.iter().map(|tip| tip.id))
        .with_hidden([id])
        .all()?;
    for info in walk {
        let info = info?;
        parents.insert(info.id, info.parent_ids.iter().copied().collect());
    }
    Ok(parents)
}

fn copy_commit(
    repo: &gix::Repository,
    id: ObjectId,
    rewritten: &HashMap<ObjectId, ObjectId>,
    committer: &BStr,
    message: Option<&BStr>,
) -> Result<ObjectId> {
    let object = repo.find_object(id)?;
    let parents: Vec<String>;
    let mut commit = CommitRef::from_bytes(&object.data, repo.object_hash())?;
    parents = commit
        .parents
        .iter()
        .map(|parent| {
            let parent = ObjectId::from_hex(parent)?;
            Ok(rewritten.get(&parent).copied().unwrap_or(parent).to_string())
        })
        .collect::<Result<_>>()?;
    commit.parents = parents.iter().map(|parent| parent.as_bytes().as_bstr()).collect();
    commit.committer = committer;
    commit
        .extra_headers
        .retain(|(name, _)| !SIGNATURES.contains(&name.as_bytes()));
    if let Some(message) = message {
        commit.message = message;
        commit.encoding = None;
    }
    Ok(repo.write_object(&commit)?.detach())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use std::collections::HashMap;

    use super::{RewordScope, local_tips};
    use crate::git::{command::run_with_input, scratch::ScratchRepo};

    fn id(scratch: &ScratchRepo, revision: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_hex(scratch.git(&["rev-parse", revision])?.trim().as_bytes())?)
    }

    fn log(scratch: &ScratchRepo, format: &str, revisions: &[&str]) -> Result<String> {
        let format = format!("--format={format}");
        let mut args = vec!["log", format.as_str()];
        args.extend_from_slice(revisions);
        scratch.git(&args)
    }

    fn three_on_main(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        for (file, subject) in [("a.txt", "Add a"), ("b.txt", "Add b"), ("c.txt", "Add c")] {
            scratch.write(file, subject)?;
            scratch.git(&["add", file])?;
            scratch.git(&["commit", "--quiet", "-m", subject])?;
        }
        Ok(scratch)
    }

    #[test]
    fn an_older_commit_gets_the_message_and_the_commits_after_it_keep_theirs() -> Result<()> {
        let scratch = three_on_main("reword-older")?;
        let trees = log(&scratch, "%T %an %ae %ad", &["main"])?;
        scratch.write("a.txt", "edited")?;
        scratch.write("b.txt", "staged")?;
        scratch.git(&["add", "b.txt"])?;
        let status = scratch.git(&["status", "--porcelain"])?;
        let (repo, _) = scratch.open()?;
        let reworded = repo.reword(id(&scratch, "main~1")?, "Add the b file  \n\n\nWith a body.  \n")?;
        assert_eq!(id(&scratch, "main~1")?, reworded);
        assert_eq!(log(&scratch, "%B", &["-1", "main~1"])?, "Add the b file\n\nWith a body.\n\n");
        assert_eq!(log(&scratch, "%s", &["main"])?, "Add c\nAdd the b file\nAdd a\n");
        assert_eq!(log(&scratch, "%T %an %ae %ad", &["main"])?, trees);
        assert_eq!(scratch.git(&["status", "--porcelain"])?, status);
        assert_eq!(scratch.git(&["symbolic-ref", "HEAD"])?.trim(), "refs/heads/main");
        Ok(())
    }

    #[test]
    fn every_local_branch_holding_it_moves_and_tags_and_remote_branches_stay() -> Result<()> {
        let scratch = three_on_main("reword-branches")?;
        let first = id(&scratch, "main~2")?;
        let held = id(&scratch, "main~1")?;
        let held_hex = held.to_string();
        scratch.git(&["switch", "--quiet", "--create=topic", &held_hex])?;
        scratch.commit("On topic")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.git(&["merge", "--quiet", "--no-ff", "--no-edit", "topic"])?;
        scratch.git(&["branch", "aside", &first.to_string()])?;
        scratch.git(&["tag", "kept", &held_hex])?;
        scratch.git(&["update-ref", "refs/remotes/origin/main", &held_hex])?;
        let (repo, history) = scratch.open()?;
        assert_eq!(
            history.reword_scope(&held),
            Some(RewordScope {
                after: 3,
                branches: vec!["main".into(), "topic".into()],
                remotes: vec!["origin/main".into()],
                tags: vec!["kept".into()],
            })
        );
        let reworded = repo.reword(held, "Add b, reworded\n")?;
        assert_eq!(id(&scratch, "main^1~1")?, reworded);
        assert_eq!(id(&scratch, "topic~1")?, reworded);
        assert_eq!(id(&scratch, "main^2")?, id(&scratch, "topic")?);
        assert_eq!(id(&scratch, "kept")?, held);
        assert_eq!(id(&scratch, "origin/main")?, held);
        assert_eq!(id(&scratch, "aside")?, first);
        assert_eq!(id(&scratch, "main~3")?, first);
        assert_eq!(log(&scratch, "%s", &["-1", "main^2"])?, "On topic\n");
        Ok(())
    }

    #[test]
    fn a_detached_head_holding_it_moves() -> Result<()> {
        let scratch = three_on_main("reword-detached")?;
        scratch.git(&["switch", "--quiet", "--detach", "main"])?;
        scratch.git(&["branch", "--quiet", "--delete", "--force", "main"])?;
        let (repo, history) = scratch.open()?;
        let head = id(&scratch, "HEAD")?;
        assert_eq!(
            history.reword_scope(&head).map(|scope| scope.branches),
            Some(vec!["HEAD".into()])
        );
        let reworded = repo.reword(head, "Add c, reworded")?;
        assert_eq!(id(&scratch, "HEAD")?, reworded);
        assert!(scratch.git(&["symbolic-ref", "--quiet", "HEAD"]).is_err());
        Ok(())
    }

    #[test]
    fn a_commit_no_local_branch_holds_is_left_alone() -> Result<()> {
        let scratch = three_on_main("reword-unheld")?;
        scratch.git(&["switch", "--quiet", "--create=gone"])?;
        scratch.commit("Only on a tag")?;
        scratch.git(&["tag", "only", "gone"])?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.git(&["branch", "--quiet", "--delete", "--force", "gone"])?;
        let only = id(&scratch, "only")?;
        let (repo, history) = scratch.open()?;
        assert_eq!(history.reword_scope(&only), None);
        assert!(repo.reword(only, "Never written").is_err());
        assert!(repo.reword(id(&scratch, "main")?, "  \n").is_err());
        assert_eq!(log(&scratch, "%s", &["-1", "main"])?, "Add c\n");
        Ok(())
    }

    #[test]
    fn a_branch_that_moved_meanwhile_stops_the_save_and_is_named() -> Result<()> {
        let scratch = three_on_main("reword-moved")?;
        let (repo, _) = scratch.open()?;
        let local = repo.local();
        let tips = local_tips(&local)?;
        let moved: Vec<_> = tips.iter().collect();
        let rewritten = HashMap::from([(id(&scratch, "main")?, id(&scratch, "main~1")?)]);
        scratch.git(&["update-ref", "refs/heads/main", "main~2"])?;
        let error = repo.move_tips(&local, &moved, &rewritten, "test").unwrap_err();
        assert_eq!(
            error.to_string(),
            "main moved while the message was saved, so nothing changed. Save again to rewrite it from where it is now."
        );
        Ok(())
    }

    #[test]
    fn a_signature_goes_and_other_headers_stay() -> Result<()> {
        let scratch = three_on_main("reword-signed")?;
        let raw = scratch.git(&["cat-file", "commit", "main"])?;
        let (headers, message) = raw.split_once("\n\n").unwrap_or_default();
        let signed = format!(
            "{headers}\nencoding ISO-8859-1\nx-kept value\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n abc\n -----END PGP SIGNATURE-----\n\n{message}"
        );
        let written = run_with_input(&scratch.dir, &["hash-object", "-t", "commit", "-w", "--stdin"], signed.as_bytes())?;
        let written = String::from_utf8(written)?;
        scratch.git(&["update-ref", "refs/heads/main", written.trim()])?;
        scratch.git(&["switch", "--quiet", "--create=after"])?;
        scratch.commit("After it")?;
        let (repo, _) = scratch.open()?;
        repo.reword(id(&scratch, "main")?, "Add c, unsigned")?;
        let copy = scratch.git(&["cat-file", "commit", "main"])?;
        assert!(copy.contains("\nx-kept value\n"));
        assert!(!copy.contains("gpgsig"));
        assert!(!copy.contains("encoding"));
        assert!(copy.ends_with("\n\nAdd c, unsigned\n"));
        assert_eq!(id(&scratch, "after~1")?, id(&scratch, "main")?);
        Ok(())
    }
}
