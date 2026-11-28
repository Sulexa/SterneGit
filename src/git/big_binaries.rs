use std::path::Path;

use gix::ObjectId;

use super::{
    Repo,
    command::{Exit, READ_ONLY, first_bytes, run},
};

const PEEKED_ABOVE: u64 = 16 * 1024 * 1024;
const GIT_DEFAULT_BIG_FILE: u64 = 512 * 1024 * 1024;
const BYTES_GIT_JUDGES_BY: usize = 8000;
const NO_BLOB: ObjectId = ObjectId::Sha1([0; 20]);

impl Repo {
    pub(crate) fn skipping_big_binaries_of_commit(&self, id: &str) -> Vec<String> {
        self.skipping_big_binaries(&[
            "show",
            "--format=",
            "--first-parent",
            "--raw",
            "--no-renames",
            "--no-abbrev",
            id,
            "--",
        ])
    }

    pub(crate) fn skipping_big_binaries_between(&self, base: &str, head: &str) -> Vec<String> {
        self.skipping_big_binaries(&["diff", "--raw", "--no-renames", "--no-abbrev", base, head, "--"])
    }

    fn skipping_big_binaries(&self, raw_diff: &[&str]) -> Vec<String> {
        let mut args = vec![READ_ONLY];
        args.extend(raw_diff);
        let Ok(raw) = run(&self.location, &args, Exit::Success) else {
            return Vec::new();
        };
        let local = self.local();
        let git_limit = local
            .config_snapshot()
            .integer("core.bigFileThreshold")
            .and_then(|limit| u64::try_from(limit).ok())
            .unwrap_or(GIT_DEFAULT_BIG_FILE);
        let mut big: Vec<(ObjectId, u64)> = changed_blobs(&raw)
            .filter_map(|id| Some((id, local.find_header(id).ok()?.size())))
            .filter(|&(_, size)| size > PEEKED_ABOVE && size <= git_limit)
            .collect();
        big.sort_unstable();
        big.dedup();
        match threshold_reading_every_text(&self.location, &big) {
            Some(threshold) => vec!["-c".to_owned(), format!("core.bigFileThreshold={threshold}")],
            None => Vec::new(),
        }
    }
}

fn changed_blobs(raw: &[u8]) -> impl Iterator<Item = ObjectId> + '_ {
    raw.split(|&byte| byte == b'\n')
        .filter_map(|line| line.strip_prefix(b":"))
        .filter_map(|line| line.split(|&byte| byte == b'\t').next())
        .flat_map(|modes_and_ids| modes_and_ids.split(|&byte| byte == b' ').skip(2).take(2))
        .filter_map(|id| ObjectId::from_hex(id).ok())
        .filter(|id| *id != NO_BLOB)
}

fn threshold_reading_every_text(location: &Path, big: &[(ObjectId, u64)]) -> Option<u64> {
    let binary = starts_like_binary_each(location, big);
    if !binary.iter().any(|&is_binary| is_binary) {
        return None;
    }
    let largest_text = big
        .iter()
        .zip(&binary)
        .filter(|&(_, &is_binary)| !is_binary)
        .map(|(&(_, size), _)| size)
        .max();
    Some(largest_text.unwrap_or(PEEKED_ABOVE))
}

fn starts_like_binary_each(location: &Path, big: &[(ObjectId, u64)]) -> Vec<bool> {
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    big.chunks(threads)
        .flat_map(|batch| {
            std::thread::scope(|scope| {
                let peeks: Vec<_> = batch
                    .iter()
                    .map(|(id, _)| scope.spawn(move || starts_like_binary(location, *id)))
                    .collect();
                peeks
                    .into_iter()
                    .map(|peek| peek.join().unwrap_or(false))
                    .collect::<Vec<_>>()
            })
        })
        .collect()
}

fn starts_like_binary(location: &Path, id: ObjectId) -> bool {
    let id = id.to_string();
    first_bytes(location, &[READ_ONLY, "cat-file", "blob", &id], BYTES_GIT_JUDGES_BY)
        .is_ok_and(|start| start.contains(&0))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use crate::git::{Repo, scratch::ScratchRepo};

    const BIG: usize = 17 * 1024 * 1024;

    fn big_text() -> String {
        "a line of a big text file
".repeat(BIG / 26 + 1)
    }

    fn big_binary() -> Vec<u8> {
        let mut bytes = vec![b'x'; BIG];
        bytes[10] = 0;
        bytes
    }

    fn committed(scratch: &ScratchRepo) -> Result<(Repo, ObjectId)> {
        scratch.git(&["add", "--all"])?;
        scratch.commit("Add files")?;
        let id = ObjectId::from_hex(scratch.head()?.as_bytes())?;
        Ok((scratch.open()?.0, id))
    }

    #[test]
    fn small_changes_leave_git_as_it_is() -> Result<()> {
        let scratch = ScratchRepo::new("big-none")?;
        scratch.write("a.txt", "one
")?;
        let (repo, id) = committed(&scratch)?;
        assert!(repo.skipping_big_binaries_of_commit(&id.to_string()).is_empty());
        Ok(())
    }

    #[test]
    fn a_big_text_file_alone_leaves_git_as_it_is() -> Result<()> {
        let scratch = ScratchRepo::new("big-text")?;
        scratch.write("scene.txt", &big_text())?;
        let (repo, id) = committed(&scratch)?;
        assert!(repo.skipping_big_binaries_of_commit(&id.to_string()).is_empty());
        Ok(())
    }

    #[test]
    fn a_big_binary_is_skipped_while_a_bigger_text_file_is_still_read() -> Result<()> {
        let scratch = ScratchRepo::new("big-both")?;
        let text = big_text() + &"more
".repeat(1024 * 1024);
        scratch.write("scene.txt", &text)?;
        std::fs::write(std::path::Path::new(&scratch.location()).join("music.wav"), big_binary())?;
        let (repo, id) = committed(&scratch)?;
        let threshold = format!("core.bigFileThreshold={}", text.len());
        assert_eq!(repo.skipping_big_binaries_of_commit(&id.to_string()), ["-c", threshold.as_str()]);
        let patch = repo.commit_details(id)?.patch;
        assert!(patch.contains("
+more
"));
        assert!(patch.contains("Binary files /dev/null and b/music.wav differ"));
        Ok(())
    }
}
