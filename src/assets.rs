use std::borrow::Cow;

use gpui_kit::{AssetSource, SharedString};

gpui_kit::assets::icon_assets!(
    IconsBeyondTheKit,
    [
        GitBranch,
        GitBranchPlus,
        GitMerge,
        Cloud,
        Tag,
        Columns2,
        Rows2,
        List,
        ListTree,
        ArrowDownToLine,
        ArrowUpFromLine,
        Archive,
        ArchiveRestore
    ]
);

pub(crate) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        match IconsBeyondTheKit.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(IconsBeyondTheKit.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
