use std::ffi::OsStr;

use anyhow::{Context as _, Result};
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    ChangeKind, Commit, Repo,
    command::{Exit, READ_ONLY, UNIQUE_LINES_FIRST, run},
    text::decode,
};

const FOLLOWED_LOG: &[&str] = &[
    READ_ONLY,
    "--literal-pathspecs",
    "log",
    "--follow",
    "--date-order",
    "--no-show-signature",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    UNIQUE_LINES_FIRST,
    "--format=%x01%H",
    "--raw",
    "--numstat",
    "-z",
];

const COMMIT_MARK: u8 = 0x01;

const BINARY_COUNT: &[u8] = b"-";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineCounts {
    pub(crate) added: usize,
    pub(crate) removed: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct FileCommit {
    pub(crate) commit: Commit,
    pub(crate) kind: ChangeKind,
    pub(crate) path: SharedString,
    pub(crate) original_path: Option<SharedString>,
    pub(crate) lines: Option<LineCounts>,
}

#[derive(Debug, PartialEq, Eq)]
struct Step {
    id: ObjectId,
    kind: ChangeKind,
    path: SharedString,
    original_path: Option<SharedString>,
    lines: Option<LineCounts>,
}

impl Repo {
    pub(crate) fn file_history(&self, path: &OsStr, tips: &[ObjectId]) -> Result<Vec<FileCommit>> {
        if tips.is_empty() {
            return Ok(Vec::new());
        }
        let tips: Vec<String> = tips.iter().map(ToString::to_string).collect();
        let mut args: Vec<&OsStr> = FOLLOWED_LOG.iter().map(OsStr::new).collect();
        args.extend(tips.iter().map(OsStr::new));
        args.extend([OsStr::new("--"), path]);
        let output = run(&self.location, &args, Exit::Success)?;
        let repo = self.local();
        followed_steps(&output)?
            .into_iter()
            .map(|step| {
                Ok(FileCommit {
                    commit: Commit::find(&repo, step.id)?,
                    kind: step.kind,
                    path: step.path,
                    original_path: step.original_path,
                    lines: step.lines,
                })
            })
            .collect()
    }
}

fn followed_steps(output: &[u8]) -> Result<Vec<Step>> {
    let mut tokens = output
        .split(|&byte| byte == 0)
        .map(|token| token.strip_prefix(b"\n").unwrap_or(token))
        .peekable();
    let mut steps = Vec::new();
    while let Some(token) = tokens.next() {
        let Some(hex) = token.strip_prefix(&[COMMIT_MARK]) else {
            continue;
        };
        let id = ObjectId::from_hex(hex).context("git log wrote an unexpected commit id")?;
        let Some(raw) = tokens.next_if(|token| token.starts_with(b":")) else {
            continue;
        };
        let kind = raw
            .rsplit(|&byte| byte == b' ')
            .next()
            .and_then(|status| status.first())
            .and_then(|&code| ChangeKind::from_status_code(code))
            .unwrap_or(ChangeKind::Modified);
        let moved = matches!(kind, ChangeKind::Renamed | ChangeKind::Copied);
        let original_path = moved.then(|| tokens.next()).flatten().map(name);
        let path = tokens.next().map(name).unwrap_or_default();
        let counts = tokens.next().unwrap_or_default();
        if moved && counts.ends_with(b"\t") {
            tokens.next();
            tokens.next();
        }
        steps.push(Step {
            id,
            kind,
            path,
            original_path,
            lines: line_counts(counts),
        });
    }
    Ok(steps)
}

fn name(bytes: &[u8]) -> SharedString {
    decode(bytes).into()
}

fn line_counts(numstat: &[u8]) -> Option<LineCounts> {
    let mut fields = numstat.splitn(3, |&byte| byte == b'\t');
    let mut count = || {
        fields
            .next()
            .filter(|field| *field != BINARY_COUNT)
            .and_then(|field| std::str::from_utf8(field).ok()?.parse().ok())
    };
    Some(LineCounts {
        added: count()?,
        removed: count()?,
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use anyhow::Result;
    use gix::ObjectId;

    use super::{LineCounts, Step, followed_steps};
    use crate::git::{
        ChangeKind,
        fixtures::id,
        scratch::{INSERTED_LINES, ScratchRepo, lookalike_blocks, lookalike_blocks_with_insertions},
    };

    #[test]
    fn blocks_inserted_between_lookalike_blocks_count_as_added_lines_only() -> Result<()> {
        let scratch = ScratchRepo::new("lookalike-history")?;
        scratch.write("shop.prefab", &lookalike_blocks())?;
        scratch.git(&["add", "shop.prefab"])?;
        scratch.commit("Add the shop")?;
        scratch.write("shop.prefab", &lookalike_blocks_with_insertions())?;
        scratch.git(&["commit", "--quiet", "-am", "Insert offers"])?;
        let (repo, history) = scratch.open()?;
        let head: Vec<ObjectId> = history.head_id.into_iter().collect();
        let newest = repo.file_history(OsStr::new("shop.prefab"), &head)?[0].lines;
        assert_eq!(
            newest,
            Some(LineCounts {
                added: INSERTED_LINES,
                removed: 0
            })
        );
        Ok(())
    }

    fn summary(
        repo: &crate::git::Repo,
        path: &str,
        tips: &[ObjectId],
    ) -> Result<Vec<(String, ChangeKind, String, Option<String>)>> {
        Ok(repo
            .file_history(OsStr::new(path), tips)?
            .into_iter()
            .map(|step| {
                (
                    step.commit.subject.to_string(),
                    step.kind,
                    step.path.to_string(),
                    step.original_path.map(|path| path.to_string()),
                )
            })
            .collect())
    }

    #[test]
    fn the_history_follows_the_file_through_a_rename_newest_first() -> Result<()> {
        let scratch = ScratchRepo::new("file-history")?;
        scratch.write("a.txt", "one\ntwo\nthree\nfour\nfive\nsix\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.git(&["commit", "--quiet", "-m", "Add a"])?;
        scratch.write("a.txt", "one\ntwo\nthree\nfour\nfive\nsix\nseven\n")?;
        scratch.git(&["commit", "--quiet", "-am", "Grow a"])?;
        scratch.write("other.txt", "unrelated\n")?;
        scratch.git(&["add", "other.txt"])?;
        scratch.git(&["commit", "--quiet", "-m", "Leave a alone"])?;
        std::fs::create_dir(scratch.dir.join("sub"))?;
        scratch.git(&["mv", "a.txt", "sub/b.txt"])?;
        scratch.git(&["commit", "--quiet", "-m", "Move a to b"])?;
        scratch.write("sub/b.txt", "one\nTWO\nthree\nfour\nfive\nsix\nseven\n")?;
        scratch.git(&["commit", "--quiet", "-am", "Edit b"])?;
        let (repo, history) = scratch.open()?;
        let head: Vec<ObjectId> = history.head_id.into_iter().collect();
        assert_eq!(
            summary(&repo, "sub/b.txt", &head)?,
            [
                (
                    "Edit b".into(),
                    ChangeKind::Modified,
                    "sub/b.txt".into(),
                    None
                ),
                (
                    "Move a to b".into(),
                    ChangeKind::Renamed,
                    "sub/b.txt".into(),
                    Some("a.txt".into())
                ),
                ("Grow a".into(), ChangeKind::Modified, "a.txt".into(), None),
                ("Add a".into(), ChangeKind::Added, "a.txt".into(), None),
            ]
        );
        let counts: Vec<_> = repo
            .file_history(OsStr::new("sub/b.txt"), &head)?
            .into_iter()
            .map(|step| step.lines)
            .collect();
        assert_eq!(
            counts,
            [
                Some(LineCounts {
                    added: 1,
                    removed: 1
                }),
                Some(LineCounts {
                    added: 0,
                    removed: 0
                }),
                Some(LineCounts {
                    added: 1,
                    removed: 0
                }),
                Some(LineCounts {
                    added: 6,
                    removed: 0
                }),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_commit_on_another_branch_joins_the_history_when_it_is_a_tip() -> Result<()> {
        let scratch = ScratchRepo::new("file-history-tips")?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.git(&["commit", "--quiet", "-m", "Add a"])?;
        scratch.git(&["checkout", "--quiet", "-b", "side"])?;
        scratch.write("a.txt", "one\nside\n")?;
        scratch.git(&["commit", "--quiet", "-am", "Change a on the side"])?;
        let side = ObjectId::from_hex(scratch.head()?.as_bytes())?;
        scratch.git(&["checkout", "--quiet", "main"])?;
        let (repo, history) = scratch.open()?;
        let head = history.head_id.expect("main has a commit");
        let subjects = |tips: &[ObjectId]| -> Result<Vec<String>> {
            Ok(summary(&repo, "a.txt", tips)?
                .into_iter()
                .map(|row| row.0)
                .collect())
        };
        assert_eq!(subjects(&[head])?, ["Add a"]);
        assert_eq!(subjects(&[head, side])?, ["Change a on the side", "Add a"]);
        assert!(subjects(&[])?.is_empty());
        Ok(())
    }

    #[test]
    fn a_binary_file_has_no_line_counts_and_a_deletion_keeps_its_path() -> Result<()> {
        let mut output = Vec::new();
        for (byte, raw, counts) in [
            (
                2u8,
                ":100644 000000 1981a8c 0000000 D\0pic.png\0",
                "-\t-\tpic.png\0",
            ),
            (
                1u8,
                ":000000 100644 0000000 1981a8c A\0pic.png\0",
                "-\t-\tpic.png\0",
            ),
        ] {
            output.push(super::COMMIT_MARK);
            output.extend(id(byte).to_string().bytes());
            output.extend(b"\0\n");
            output.extend(raw.bytes());
            output.extend(counts.bytes());
        }
        let steps = followed_steps(&output)?;
        assert_eq!(
            steps,
            [
                Step {
                    id: id(2),
                    kind: ChangeKind::Deleted,
                    path: "pic.png".into(),
                    original_path: None,
                    lines: None
                },
                Step {
                    id: id(1),
                    kind: ChangeKind::Added,
                    path: "pic.png".into(),
                    original_path: None,
                    lines: None
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn a_commit_without_a_change_listed_is_left_out() -> Result<()> {
        let mut output = vec![super::COMMIT_MARK];
        output.extend(id(3).to_string().bytes());
        output.extend(b"\0\n");
        output.push(super::COMMIT_MARK);
        output.extend(id(1).to_string().bytes());
        output.extend(
            b"\0\n:100644 100644 2019eda 2019eda R087\0a.txt\0b.txt\x003\t1\t\0a.txt\0b.txt\0",
        );
        let steps = followed_steps(&output)?;
        assert_eq!(
            steps,
            [Step {
                id: id(1),
                kind: ChangeKind::Renamed,
                path: "b.txt".into(),
                original_path: Some("a.txt".into()),
                lines: Some(LineCounts {
                    added: 3,
                    removed: 1
                }),
            }]
        );
        Ok(())
    }
}
