use std::path::Path;

use anyhow::Result;

use super::command::{Exit, READ_ONLY, run};

pub(crate) fn paths_changed_since(location: &Path, commit: &str, glob: &str) -> Result<Vec<String>> {
    let output = run(
        location,
        &[READ_ONLY, "-c", "core.quotePath=false", "diff", "--name-only", "--no-renames", "-z", commit, "--", glob],
        Exit::Success,
    )?;
    Ok(output
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect())
}

pub(crate) fn matches_in_commit(location: &Path, commit: &str, regex: &str, paths: &[&str]) -> Result<Vec<(String, String)>> {
    let mut args = vec![READ_ONLY, "grep", "-I", "-o", "-E", "-e", regex, commit, "--"];
    args.extend_from_slice(paths);
    let output = run(location, &args, Exit::SuccessOrDifference)?;
    let prefix = format!("{commit}:");
    Ok(String::from_utf8_lossy(&output)
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .filter_map(|line| line.split_once(':'))
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect())
}

pub(crate) fn matches_in_working_tree(location: &Path, regex: &str, glob: &str) -> Result<Vec<(String, String)>> {
    let output = run(
        location,
        &[READ_ONLY, "grep", "--untracked", "-I", "-o", "-E", "-e", regex, "--", glob],
        Exit::SuccessOrDifference,
    )?;
    Ok(String::from_utf8_lossy(&output)
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{matches_in_commit, matches_in_working_tree};
    use crate::git::scratch::ScratchRepo;

    const GUID: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn a_search_finds_matches_in_files_not_committed_yet() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("grep-working-tree")?;
        scratch.write("slime.png.meta", &format!("fileFormatVersion: 2
guid: {GUID}
"))?;
        scratch.write("Slime.prefab", &format!("m_Sprite: {{fileID: 21300000, guid: {GUID}, type: 3}}
"))?;
        let found = matches_in_working_tree(std::path::Path::new(&scratch.location()), "^guid: [0-9a-f]{32}", "*.meta")?;
        assert_eq!(found, vec![("slime.png.meta".to_owned(), format!("guid: {GUID}"))]);
        let mut users: Vec<String> = matches_in_working_tree(std::path::Path::new(&scratch.location()), GUID, "*")?
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        users.sort();
        assert_eq!(users, ["Slime.prefab", "slime.png.meta"]);
        Ok(())
    }

    #[test]
    fn a_search_in_a_commit_reads_that_commit_only() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("grep-commit")?;
        scratch.write("slime.png.meta", &format!("guid: {GUID}
"))?;
        scratch.git(&["add", "-A"])?;
        scratch.commit("first")?;
        scratch.write("slime.png.meta", "guid: ffffffffffffffffffffffffffffffff
")?;
        let found = matches_in_commit(std::path::Path::new(&scratch.location()), "HEAD", "^guid: [0-9a-f]{32}", &["*.meta"])?;
        assert_eq!(found, vec![("slime.png.meta".to_owned(), format!("guid: {GUID}"))]);
        Ok(())
    }
}
