// Release builds are windowed apps: no console window opens beside them on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod graph;
mod repository;

use std::{borrow::Cow, path::PathBuf};

use gpui_kit::{
    AppContext as _, AssetSource, SharedString, WindowBounds, WindowOptions, component::TitleBar,
    px, size,
};

const USAGE: &str = "Usage: fastgit [--check] [repository]";

// gpui-kit's default bundle carries only the icons its own components use.
// Embed the few extra ones FastGit draws rather than the whole Lucide catalog.
gpui_kit::assets::icon_assets!(GitIcons, [GitBranch, GitMerge, Cloud, Tag]);

struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        match GitIcons.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(GitIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

fn main() {
    let mut check = false;
    let mut path = None;
    for argument in std::env::args_os().skip(1) {
        if argument == "--help" || argument == "-h" {
            println!(
                "{USAGE}\nOpens the repository containing the given path, or the current directory.\n--check reads the history without opening a window and reports how long it took."
            );
            return;
        } else if argument == "--check" {
            check = true;
        } else if path.is_none() && !argument.to_string_lossy().starts_with('-') {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }

    if check {
        let path = path.unwrap_or_else(|| PathBuf::from("."));
        match repository::load(&path) {
            Ok(history) => println!(
                "{}: {} commits{}, {} refs, {} lanes wide, read in {:.1?}",
                history.name,
                history.commits.len(),
                if history.truncated { " (newest only)" } else { "" },
                history.refs.len(),
                history.graph.width,
                history.elapsed
            ),
            Err(error) => {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        return;
    }

    // With no argument, start in the current directory only when it is a repository.
    let path = path.or_else(|| {
        std::env::current_dir()
            .ok()
            .filter(|dir| gix::discover(dir).is_ok())
    });

    gpui_kit::application()
        .with_assets(Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1440.), px(900.)), cx)),
                window_min_size: Some(size(px(960.), px(600.))),
                app_id: Some("fastgit".into()),
                ..TitleBar::window_options()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| app::FastGit::new(path, window, cx))
            })
            .expect("failed to open the FastGit window");
            cx.activate(true);
        });
}
