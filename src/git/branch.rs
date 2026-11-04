use anyhow::Result;
use gix::bstr::BStr;

use super::{
    RefKind, RefLabel, Repo,
    command::{Exit, run},
};

const BRANCHES: &str = "refs/heads/";
const NAMES_GIT_RESERVES: [&str; 1] = ["@"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameProblem {
    Empty,
    Taken,
    Refused,
}

pub(crate) fn new_branch_name<'a>(typed: &'a str, refs: &[RefLabel]) -> Result<&'a str, NameProblem> {
    let name = typed.trim();
    if name.is_empty() {
        return Err(NameProblem::Empty);
    }
    let full = format!("{BRANCHES}{name}");
    let refused = name.starts_with('-')
        || NAMES_GIT_RESERVES.contains(&name)
        || gix::validate::reference::branch_name(BStr::new(&full)).is_err();
    if refused {
        return Err(NameProblem::Refused);
    }
    let taken = refs
        .iter()
        .any(|label| label.kind == RefKind::LocalBranch && label.name.as_str() == name);
    if taken {
        return Err(NameProblem::Taken);
    }
    Ok(name)
}

impl Repo {
    pub(crate) fn create_branch(&self, name: &str) -> Result<()> {
        run(
            &self.location,
            &["switch", "--quiet", &format!("--create={name}")],
            Exit::Success,
        )
        .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{NameProblem, new_branch_name};
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn a_new_branch_starts_where_head_is_and_is_checked_out() -> Result<()> {
        let scratch = ScratchRepo::new("create-branch")?;
        scratch.commit("first")?;
        scratch.write("draft.txt", "kept\n")?;
        let head = scratch.head()?;
        let (repo, history) = scratch.open()?;
        let name = new_branch_name(" feature/toolbar ", &history.refs).expect("a fine name");
        repo.create_branch(name)?;
        assert_eq!(
            scratch.git(&["symbolic-ref", "--short", "HEAD"])?.trim(),
            "feature/toolbar"
        );
        assert_eq!(scratch.head()?, head);
        assert!(scratch.dir.join("draft.txt").exists());
        Ok(())
    }

    #[test]
    fn names_git_refuses_or_that_exist_cannot_make_a_branch() -> Result<()> {
        let scratch = ScratchRepo::new("branch-names")?;
        scratch.commit("first")?;
        scratch.git(&["branch", "topic/tabs"])?;
        scratch.git(&["tag", "v1"])?;
        let (_, history) = scratch.open()?;
        let check = |name: &str| new_branch_name(name, &history.refs).map(str::to_owned);
        assert_eq!(check("  "), Err(NameProblem::Empty));
        assert_eq!(check("topic/tabs"), Err(NameProblem::Taken));
        assert_eq!(check("main"), Err(NameProblem::Taken));
        for refused in [
            "my branch",
            "a..b",
            "-f",
            "HEAD",
            "@",
            "x.lock",
            "end/",
            "what?",
            "star*",
            "tilde~1",
            "caret^",
            "colon:",
            "back\\slash",
            "[bracket",
            "at@{1}",
        ] {
            assert_eq!(check(refused), Err(NameProblem::Refused), "{refused}");
        }
        assert_eq!(check("v1").as_deref(), Ok("v1"));
        assert_eq!(check("fix/merge-2").as_deref(), Ok("fix/merge-2"));
        Ok(())
    }
}
