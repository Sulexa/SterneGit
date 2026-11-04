use anyhow::Result;

use super::{Repo, command::run_utf8};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fetched {
    NoRemotes,
    AllRemotes,
}

impl Repo {
    pub(crate) fn fetch(&self) -> Result<Fetched> {
        if self.local().remote_names().is_empty() {
            return Ok(Fetched::NoRemotes);
        }
        run_utf8(&self.location, &["fetch", "--all", "--prune", "--quiet"])?;
        Ok(Fetched::AllRemotes)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::Fetched;
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn a_repository_without_remotes_fetches_nothing() -> Result<()> {
        let scratch = ScratchRepo::new("no-remotes")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.fetch()?, Fetched::NoRemotes);
        Ok(())
    }
}
