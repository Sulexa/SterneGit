use anyhow::{Result, bail};
use gix::{ObjectId, bstr::BStr};

use super::{
    Carried, CheckedOut, Repo,
    command::{Exit, READ_ONLY, run_text},
};

pub(super) const PULL_REFS: &str = "refs/fastgit/pull/";
const REMOTE_URL_KEYS: &str = r"^remote\..*\.url$";
const REMOTE_KEY_PREFIX: &str = "remote.";
const REMOTE_KEY_SUFFIX: &str = ".url";
const BRANCHES: &str = "refs/heads/";
const FORK_BRANCHES: &str = "pr/";
const SECURE_SCHEME: &str = "https://";
const FETCH_QUIETLY: [&str; 3] = ["fetch", "--quiet", "--no-tags"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NamedRemote {
    pub(crate) name: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PullSource {
    pub(crate) remote: String,
    pub(crate) url: Option<String>,
    pub(crate) refspec: String,
    pub(crate) number: u64,
    pub(crate) base_branch: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BranchSpan {
    pub(crate) subjects: Vec<String>,
    pub(crate) files: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct PullHead {
    pub(crate) head: ObjectId,
    pub(crate) merge_base: ObjectId,
    pub(crate) subjects: Vec<String>,
    pub(crate) patch: String,
}

pub(crate) fn pull_ref(number: u64) -> String {
    format!("{PULL_REFS}{number}")
}

pub(crate) fn fork_branch_name(number: u64, head: &str) -> String {
    let mut readable = String::with_capacity(head.len());
    for character in head.chars() {
        let kept = character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-');
        let shown = if kept { character } else { '-' };
        if !(shown == '-' && readable.ends_with('-')) {
            readable.push(shown);
        }
    }
    let readable = readable.trim_matches(|character| matches!(character, '-' | '.'));
    let named = format!("{FORK_BRANCHES}{number}-{readable}");
    let valid = !readable.is_empty()
        && !named.ends_with(".lock")
        && gix::validate::reference::branch_name(BStr::new(&format!("{BRANCHES}{named}"))).is_ok();
    if valid {
        named
    } else {
        format!("{FORK_BRANCHES}{number}")
    }
}

fn plain_ref(refspec: &str) -> bool {
    refspec.starts_with("refs/")
        && !refspec.chars().any(|character| character.is_whitespace() || matches!(character, ':' | '*' | '^' | '~' | '?' | '[' | '\\'))
        && gix::validate::reference::name(BStr::new(refspec)).is_ok()
}

fn plain_branch(branch: &str) -> bool {
    !branch.starts_with('-')
        && gix::validate::reference::branch_name(BStr::new(&format!("{BRANCHES}{branch}"))).is_ok()
}

impl PullSource {
    fn checked(&self) -> Result<()> {
        if !plain_ref(&self.refspec) {
            bail!("The connector named an unusable ref for this pull request: {}", self.refspec);
        }
        if !plain_branch(&self.base_branch) {
            bail!("The pull request's base branch has a name Git refuses: {}", self.base_branch);
        }
        if self.url.as_deref().is_some_and(|url| !url.starts_with(SECURE_SCHEME)) {
            bail!("FastGit fetches pull requests only over https");
        }
        Ok(())
    }

    fn base_tracking(&self) -> String {
        format!("refs/remotes/{}/{}", self.remote, self.base_branch)
    }

    fn head_refspec(&self) -> String {
        format!("+{}:{}", self.refspec, pull_ref(self.number))
    }

    fn base_refspec(&self) -> String {
        format!("+{BRANCHES}{}:{}", self.base_branch, self.base_tracking())
    }
}

impl Repo {
    pub(crate) fn named_remotes(&self) -> Vec<NamedRemote> {
        run_text(
            &self.location,
            &["config", "--get-regexp", REMOTE_URL_KEYS],
            Exit::SuccessOrDifference,
        )
        .map(|listed| named_remotes_in(&listed))
        .unwrap_or_default()
    }

    pub(crate) fn default_branch_of(&self, remote: &str) -> Option<String> {
        let pointer = format!("refs/remotes/{remote}/HEAD");
        let named = run_text(
            &self.location,
            &[READ_ONLY, "symbolic-ref", "--quiet", "--short", &pointer],
            Exit::Success,
        )
        .ok()?;
        named
            .trim()
            .strip_prefix(&format!("{remote}/"))
            .map(str::to_owned)
    }

    pub(crate) fn fetch_pull_head(&self, source: &PullSource) -> Result<PullHead> {
        source.checked()?;
        let (head, base) = (source.head_refspec(), source.base_refspec());
        match &source.url {
            None => {
                self.fetch_refspecs(&source.remote, &[&head, &base])?;
            }
            Some(url) => {
                self.fetch_refspecs(url, &[&head])?;
                self.fetch_refspecs(&source.remote, &[&base])?;
            }
        }
        self.compare_pull_head(&pull_ref(source.number), &source.base_tracking())
    }

    pub(crate) fn check_out_pull_branch(&self, remote: &str, branch: &str) -> Result<(CheckedOut, Carried)> {
        if !plain_branch(branch) {
            bail!("The pull request's branch has a name Git refuses: {branch}");
        }
        let refspec = format!("+{BRANCHES}{branch}:refs/remotes/{remote}/{branch}");
        self.fetch_refspecs(remote, &[&refspec])?;
        self.check_out_remote(&format!("{remote}/{branch}"), branch)
    }

    pub(crate) fn check_out_fork_pull(&self, source: &PullSource, branch: &str) -> Result<Carried> {
        source.checked()?;
        let head = source.head_refspec();
        self.fetch_refspecs(source.url.as_deref().unwrap_or(&source.remote), &[&head])?;
        if self.has_branch(branch) {
            return self.check_out(branch);
        }
        self.create_branch(branch, Some(&pull_ref(source.number)))
    }

    pub(crate) fn branch_span(&self, base: &str, tip: &str) -> Result<BranchSpan> {
        let range = format!("{base}..{tip}");
        let subjects = run_text(&self.location, &[READ_ONLY, "log", "--format=%s", &range, "--"], Exit::Success)?
            .lines()
            .map(str::to_owned)
            .collect();
        let since = format!("{base}...{tip}");
        let files = run_text(&self.location, &[READ_ONLY, "diff", "--name-only", &since, "--"], Exit::Success)?
            .lines()
            .filter(|line| !line.is_empty())
            .count();
        Ok(BranchSpan { subjects, files })
    }

    fn fetch_refspecs(&self, from: &str, refspecs: &[&str]) -> Result<Vec<u8>> {
        let mut args: Vec<&str> = FETCH_QUIETLY.to_vec();
        args.push(from);
        args.extend(refspecs);
        self.reach(&args)
    }

    fn compare_pull_head(&self, head: &str, base: &str) -> Result<PullHead> {
        let commit = |revision: &str| -> Result<ObjectId> {
            let found = run_text(&self.location, &[READ_ONLY, "rev-parse", "--verify", "--quiet", revision], Exit::Success)?;
            Ok(ObjectId::from_hex(found.trim().as_bytes())?)
        };
        let head = commit(&format!("{head}^{{commit}}"))?;
        let found = run_text(&self.location, &[READ_ONLY, "merge-base", base, &head.to_string()], Exit::Success)?;
        let merge_base = ObjectId::from_hex(found.trim().as_bytes())?;
        let range = format!("{merge_base}..{head}");
        let subjects = run_text(&self.location, &[READ_ONLY, "log", "--reverse", "--format=%s", &range], Exit::Success)?
            .lines()
            .map(str::to_owned)
            .collect();
        let patch = self.range_patch(merge_base, head)?;
        Ok(PullHead {
            head,
            merge_base,
            subjects,
            patch,
        })
    }
}

fn named_remotes_in(listed: &str) -> Vec<NamedRemote> {
    listed
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter_map(|(key, url)| {
            let name = key.strip_prefix(REMOTE_KEY_PREFIX)?.strip_suffix(REMOTE_KEY_SUFFIX)?;
            Some(NamedRemote {
                name: name.to_owned(),
                url: url.trim().to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{NamedRemote, PullSource, fork_branch_name, named_remotes_in, plain_ref};
    use crate::git::scratch::ScratchRepo;

    fn source(remote: &str, number: u64) -> PullSource {
        PullSource {
            remote: remote.to_owned(),
            url: None,
            refspec: format!("refs/pull/{number}/head"),
            number,
            base_branch: "main".to_owned(),
        }
    }

    #[test]
    fn fork_branches_are_named_after_the_number_and_a_readable_head() {
        assert_eq!(fork_branch_name(9, "patch-1"), "pr/9-patch-1");
        assert_eq!(fork_branch_name(12, "feature/diff view"), "pr/12-feature-diff-view");
        assert_eq!(fork_branch_name(3, "..."), "pr/3");
        assert_eq!(fork_branch_name(4, "ééé"), "pr/4");
    }

    #[test]
    fn only_plain_refs_are_fetched() {
        assert!(plain_ref("refs/pull/12/head"));
        assert!(plain_ref("refs/merge-requests/4/head"));
        assert!(!plain_ref("refs/heads/main:refs/heads/main"));
        assert!(!plain_ref("--upload-pack=evil"));
        assert!(!plain_ref("refs/pull/*/head"));
        assert!(!plain_ref("main"));
    }

    #[test]
    fn remotes_are_read_with_their_names_and_push_urls_are_left_out() {
        let listed = "remote.origin.url https://github.com/fastgit-org/fastgit.git\nremote.origin.pushurl git@github.com:me/fastgit.git\nremote.my.fork.url https://github.com/me/fastgit\n";
        assert_eq!(
            named_remotes_in(listed),
            [
                NamedRemote { name: "origin".into(), url: "https://github.com/fastgit-org/fastgit.git".into() },
                NamedRemote { name: "my.fork".into(), url: "https://github.com/me/fastgit".into() },
            ]
        );
    }

    #[test]
    fn a_connector_url_other_than_https_is_refused() {
        let mut unsafe_source = source("origin", 1);
        unsafe_source.url = Some("ext::sh -c touch% /tmp/owned".into());
        assert!(unsafe_source.checked().is_err());
        unsafe_source.url = Some("https://dev.azure.com/org/project/_git/fork".into());
        assert!(unsafe_source.checked().is_ok());
    }

    fn with_a_pull_request(label: &str) -> Result<(ScratchRepo, ScratchRepo, String)> {
        let origin = ScratchRepo::bare(&format!("{label}-origin"))?;
        let author = ScratchRepo::new(&format!("{label}-author"))?;
        author.write("shared.txt", "one\n")?;
        author.git(&["add", "shared.txt"])?;
        author.commit("first")?;
        author.git(&["remote", "add", "origin", &origin.location()])?;
        author.git(&["push", "--quiet", "origin", "main"])?;
        author.git(&["switch", "--quiet", "--create=topic"])?;
        author.write("shared.txt", "one\ntwo\n")?;
        author.git(&["commit", "--quiet", "--all", "-m", "Add two"])?;
        author.write("added.txt", "new\n")?;
        author.git(&["add", "added.txt"])?;
        author.git(&["commit", "--quiet", "-m", "Add a file"])?;
        let head = author.head()?;
        author.git(&["push", "--quiet", "origin", "topic:refs/pull/7/head"])?;
        author.git(&["switch", "--quiet", "main"])?;
        author.commit("main moves on")?;
        author.git(&["push", "--quiet", "origin", "main"])?;
        let reviewer = ScratchRepo::tracking(&format!("{label}-reviewer"), &origin)?;
        Ok((origin, reviewer, head))
    }

    #[test]
    fn a_pull_request_head_is_fetched_and_compared_with_where_it_branched_off() -> Result<()> {
        let (_origin, reviewer, head) = with_a_pull_request("pull-head")?;
        let (repo, _) = reviewer.open()?;
        let fetched = repo.fetch_pull_head(&source("origin", 7))?;
        assert_eq!(fetched.head.to_string(), head);
        assert_eq!(fetched.subjects, ["Add two", "Add a file"]);
        assert!(fetched.patch.contains("+two"));
        assert!(fetched.patch.contains("b/added.txt"));
        assert!(!fetched.patch.contains("main moves on"));
        assert_eq!(reviewer.git(&["rev-parse", "refs/fastgit/pull/7"])?.trim(), head);
        Ok(())
    }

    #[test]
    fn a_fork_pull_request_is_checked_out_as_its_own_branch() -> Result<()> {
        let (_origin, reviewer, head) = with_a_pull_request("fork-check-out")?;
        let (repo, _) = reviewer.open()?;
        repo.check_out_fork_pull(&source("origin", 7), "pr/7-topic")?;
        assert_eq!(reviewer.git(&["symbolic-ref", "--short", "HEAD"])?.trim(), "pr/7-topic");
        assert_eq!(reviewer.head()?, head);
        assert_eq!(reviewer.git(&["rev-parse", "pr/7-topic"])?.trim(), head);
        Ok(())
    }

    #[test]
    fn a_branch_span_lists_its_own_commits_newest_first_and_counts_its_files() -> Result<()> {
        let (_origin, reviewer, _) = with_a_pull_request("branch-span")?;
        let (repo, _) = reviewer.open()?;
        repo.fetch_pull_head(&source("origin", 7))?;
        let span = repo.branch_span("origin/main", "refs/fastgit/pull/7")?;
        assert_eq!(span.subjects, ["Add a file", "Add two"]);
        assert_eq!(span.files, 2);
        Ok(())
    }

    #[test]
    fn a_missing_pull_request_fails_with_git_s_reason() -> Result<()> {
        let (_origin, reviewer, _) = with_a_pull_request("pull-missing")?;
        let (repo, _) = reviewer.open()?;
        let failed = repo.fetch_pull_head(&source("origin", 99)).unwrap_err();
        assert!(format!("{failed:#}").contains("refs/pull/99/head"));
        Ok(())
    }
}
