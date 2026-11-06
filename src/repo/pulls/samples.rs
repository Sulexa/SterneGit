use fastgit_protocol::{Person, PullState, PullSummary, RepoRef, Tip};

pub(crate) fn repo(owner: &str) -> RepoRef {
    RepoRef {
        owner: owner.into(),
        name: "fastgit".into(),
    }
}

pub(crate) fn pull(number: u64, head: &str, owner: &str) -> PullSummary {
    let tip = |branch: &str, owner: &str| Tip {
        branch: branch.into(),
        sha: "8ff3473".repeat(5) + "00000",
        repo: Some(repo(owner)),
    };
    PullSummary {
        number,
        title: "Render side-by-side diffs".into(),
        state: PullState::Open,
        draft: false,
        author: Person {
            login: "sam-lee".into(),
            name: None,
            avatar_url: None,
        },
        base: tip("main", "fastgit-org"),
        head: tip(head, owner),
        review: None,
        checks: None,
        created_at: "2026-11-03T18:50:00Z".into(),
        updated_at: "2026-11-04T08:15:00Z".into(),
        web_url: format!("https://github.com/fastgit-org/fastgit/pull/{number}"),
    }
}
