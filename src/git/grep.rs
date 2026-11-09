use std::collections::HashMap;

use anyhow::Result;

use super::{
    Repo,
    command::{Exit, READ_ONLY, run},
};

const GUIDS_PER_SEARCH: usize = 400;
const OWNER_MARK: &str = ":guid: ";

impl Repo {
    pub(crate) fn meta_files_owning(&self, guids: &[String]) -> Result<HashMap<String, String>> {
        let mut owners = HashMap::new();
        for chunk in guids.chunks(GUIDS_PER_SEARCH) {
            let pattern = format!("^guid: ({})", chunk.join("|"));
            let output = run(
                self.location(),
                &[READ_ONLY, "grep", "--untracked", "-I", "-o", "-E", "-e", &pattern, "--", "*.meta"],
                Exit::SuccessOrDifference,
            )?;
            for line in String::from_utf8_lossy(&output).lines() {
                if let Some((meta, guid)) = line.rsplit_once(OWNER_MARK) {
                    let asset = meta.strip_suffix(".meta").unwrap_or(meta);
                    owners.entry(guid.trim().to_owned()).or_insert_with(|| asset.to_owned());
                }
            }
        }
        Ok(owners)
    }

    pub(crate) fn files_mentioning(&self, text: &str) -> Result<Vec<String>> {
        let output = run(
            self.location(),
            &[READ_ONLY, "grep", "--untracked", "-I", "-l", "-F", "-e", text],
            Exit::SuccessOrDifference,
        )?;
        Ok(String::from_utf8_lossy(&output)
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn each_guid_is_owned_by_the_meta_file_that_declares_it() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("guid-owners")?;
        scratch.write("slime.png.meta", "fileFormatVersion: 2\nguid: 0123456789abcdef0123456789abcdef\n")?;
        scratch.write(
            "Slime.prefab",
            "--- !u!1 &1\nm_Sprite: {fileID: 21300000, guid: 0123456789abcdef0123456789abcdef, type: 3}\n",
        )?;
        let (repo, _) = scratch.open()?;
        let owners = repo.meta_files_owning(&["0123456789abcdef0123456789abcdef".to_owned()])?;
        assert_eq!(
            owners.get("0123456789abcdef0123456789abcdef").map(String::as_str),
            Some("slime.png")
        );
        Ok(())
    }

    #[test]
    fn files_that_mention_a_guid_are_found_even_before_they_are_committed() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("guid-users")?;
        scratch.write("a.prefab", "guid: feedfeedfeedfeedfeedfeedfeedfeed\n")?;
        scratch.write("b.prefab", "nothing here\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.files_mentioning("feedfeedfeedfeedfeedfeedfeedfeed")?,
            vec!["a.prefab".to_owned()]
        );
        Ok(())
    }
}
