use gpui_kit::{
    AnyElement, App, ClipboardItem, Context, Hsla, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::PopupMenuItem,
        v_flex,
    },
    div, px,
    prelude::FluentBuilder as _,
};

use super::{
    parts::{Heading, Picker, boxed, callout, chip, crumbs, group, hot_row, mark, page, shown_path, slim_row},
    settings_tab::{Page, SettingsTab},
};
use crate::{
    app::readers::{ReaderPlugin, Readers, approve, choose},
    plugin::{Origin, Standing, Stop},
    read::{Grant, Reach, ReaderId},
    settings::Folder,
    ui::Look as _,
};

const READERS: &str = "Readers";
const READERS_ABOUT: &str = "Readers teach SterneGit file formats it can compare object by object, such as Unity scenes and \
     prefabs. Each one runs in a sandbox: it reads files of the repository being shown and answers what they hold; SterneGit \
     compares and draws everything itself.";
const OPEN_FOLDER: &str = "Open folder";
const BUNDLED: &str = "Bundled";
const INSTALLED: &str = "Installed";
const REVIEW: &str = "Review…";
const SHOW_LOG: &str = "Show log";
const APPROVE: &str = "Approve";
const NOT_NOW: &str = "Not now";
const COPY_LOG: &str = "Copy log";
const NO_LOG: &str = "The reader wrote nothing to its log.";
const READER: &str = "reader";
const FILES_TITLE: &str = "Files it reads";
const FILES_ABOUT: &str = "Read only: it never writes a file, reaches the network or starts a program.";
const KINDS_TITLE: &str = "Kinds of files";
const JOBS_TITLE: &str = "What it does";
const NEVER_TITLE: &str = "Never";
const NEVER: [&str; 3] = ["Writes a file", "Reaches the network", "Starts a program or reads outside the repository"];
const SHARED_TITLE: &str = "Files more than one reader opens";
const SHARED_ABOUT: &str = "SterneGit asked which reader to use the first time; change it here.";
const AS_TEXT: &str = "Show as text";

pub(crate) fn reach_line(reach: Reach) -> &'static str {
    match reach {
        Reach::File => "The file being compared and its side files, such as its .meta",
        Reach::Repository => {
            "Any file of the repository being shown, at the commit being compared, then in the working tree for what that commit lacks"
        }
    }
}

fn reach_short(reach: Reach) -> &'static str {
    match reach {
        Reach::File => "Reads only the file shown",
        Reach::Repository => "Reads the repository being shown",
    }
}

fn job_look(job: &str) -> (IconName, &str, &'static str) {
    match job {
        "compare" => (IconName::Columns2, "Compare two versions", "Reads each version into objects; SterneGit pairs and compares them"),
        "side_files" => (IconName::Link, "Check side files", "Warns when a file and its side files disagree, such as a GUID that changed"),
        "fixes" => (
            IconName::Wrench,
            "Propose fixes",
            "Suggests edits to files; you see them first and SterneGit writes them only when you apply",
        ),
        "merge" => (
            IconName::GitMerge,
            "Merge by object",
            "Merges a conflicted file object by object and leaves you the fields both sides changed",
        ),
        other => (IconName::Check, other, ""),
    }
}

fn standing_look(standing: &Standing<Grant>, cx: &App) -> (&'static str, Hsla) {
    let theme = cx.theme();
    match standing {
        Standing::Ready => ("Ready", theme.green),
        Standing::Updated => ("Updated", theme.blue),
        Standing::Waiting(_) => ("Waiting for approval", theme.yellow),
        Standing::Modified(_) => ("Files changed", theme.yellow),
        Standing::Unusable(_) => ("Can't run", theme.red),
        Standing::Failed(_) => ("Stopped", theme.red),
    }
}

fn origin_label(reader: &ReaderPlugin) -> &'static str {
    match reader.origin {
        Origin::Bundled => BUNDLED,
        Origin::Installed => INSTALLED,
    }
}

fn shown_grant(reader: &ReaderPlugin) -> Option<Grant> {
    match &reader.standing {
        Standing::Waiting(grant) | Standing::Modified(grant) => Some(grant.clone()),
        _ => reader.manifest.as_ref().map(|manifest| manifest.grant.clone()),
    }
}

fn name_of(reader: &ReaderPlugin) -> String {
    reader.manifest.as_ref().map_or_else(|| reader.id.0.clone(), |manifest| manifest.name.clone())
}

fn installed_folder() -> Option<std::path::PathBuf> {
    Folder::Data.path().map(|folder| folder.join("readers"))
}

fn stop_log(stop: &Stop) -> Vec<String> {
    match stop {
        Stop::Trapped { log, .. } => log.clone(),
        _ => Vec::new(),
    }
}

struct Summary {
    id: ReaderId,
    name: String,
    version: Option<String>,
    origin: &'static str,
    standing: Standing<Grant>,
    facts: String,
}

fn summarized(reader: &ReaderPlugin) -> Summary {
    let kinds = reader
        .manifest
        .as_ref()
        .map(|manifest| manifest.kinds.iter().map(|kind| kind.label.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let reach = shown_grant(reader).map(|grant| reach_short(grant.read)).unwrap_or_default();
    Summary {
        id: reader.id.clone(),
        name: name_of(reader),
        version: reader.manifest.as_ref().map(|manifest| manifest.version.clone()),
        origin: origin_label(reader),
        standing: reader.standing.clone(),
        facts: [kinds.as_str(), reach].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" · "),
    }
}

impl SettingsTab {
    pub(super) fn readers_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (summaries, shared) = {
            let readers = Readers::of(cx);
            let summaries: Vec<Summary> = readers.readers().iter().map(summarized).collect();
            let shared: Vec<(String, Vec<(ReaderId, String)>, Option<String>)> = readers
                .shared_extensions()
                .into_iter()
                .map(|(extension, claimers)| {
                    let chosen = readers.chosen_for(&extension).map(str::to_owned);
                    (extension, claimers, chosen)
                })
                .collect();
            (summaries, shared)
        };
        let muted = cx.theme().muted_foreground;
        let heading = Heading::new(READERS).about(READERS_ABOUT).action(
            Button::new("open-readers-folder")
                .icon(IconName::FolderOpen)
                .label(OPEN_FOLDER)
                .on_click(|_, _, cx| {
                    if let Some(folder) = installed_folder() {
                        let _ = std::fs::create_dir_all(&folder);
                        cx.open_with_system(&folder);
                    }
                }),
        );
        let rows = summaries
            .iter()
            .enumerate()
            .map(|(ix, summary)| self.reader_row(ix, summary, cx))
            .collect::<Vec<_>>();
        let shared_rows: Vec<AnyElement> = shared
            .into_iter()
            .enumerate()
            .map(|(ix, (extension, claimers, chosen))| shared_row(ix, extension, claimers, chosen, cx))
            .collect();
        let folder = installed_folder().map_or_else(String::new, |folder| shown_path(&folder));
        page(
            v_flex()
                .gap_6()
                .child(heading.render(cx))
                .when(!rows.is_empty(), |this| this.child(boxed(rows, cx)))
                .when(!shared_rows.is_empty(), |this| this.child(group(SHARED_TITLE, Some(SHARED_ABOUT.into()), shared_rows, cx)))
                .child(
                    h_flex()
                        .gap_1p5()
                        .xs_text()
                        .text_color(muted)
                        .child(Icon::new(IconName::Info).xsmall())
                        .child(format!("To install a reader, copy its folder into {folder} and restart SterneGit.")),
                ),
        )
        .into_any_element()
    }

    fn reader_row(&self, ix: usize, summary: &Summary, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, red) = (theme.muted_foreground, theme.red);
        let (standing, color) = standing_look(&summary.standing, cx);
        let second = match &summary.standing {
            Standing::Unusable(problem) => div().text_color(red).child(problem.clone()),
            Standing::Failed(stop) => div().text_color(red).child(format!("Stopped while starting: {}", stop.told_of(READER))),
            _ => div().text_color(muted).child(summary.facts.clone()),
        };
        let action = match &summary.standing {
            Standing::Waiting(_) | Standing::Modified(_) => Button::new(("review-reader", ix)).primary().small().label(REVIEW).into_any_element(),
            Standing::Failed(_) => Button::new(("reader-log", ix)).small().label(SHOW_LOG).into_any_element(),
            _ => Icon::new(IconName::ChevronRight).small().text_color(muted).into_any_element(),
        };
        let id = summary.id.clone();
        hot_row(("reader-row", ix), cx)
            .cursor_pointer()
            .child(mark(IconName::FileCode, false, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .sm_text()
                            .child(div().font_semibold().child(summary.name.clone()))
                            .children(summary.version.clone().map(|version| div().text_color(muted).child(version)))
                            .child(chip(summary.origin, None, cx))
                            .child(chip(standing, Some(color), cx)),
                    )
                    .child(second.xs_text().ellipsis()),
            )
            .child(action)
            .on_click(cx.listener(move |this, _, _, cx| this.show(Page::Reader(id.clone()), cx)))
            .into_any_element()
    }

    pub(super) fn reader_page(&mut self, id: &ReaderId, cx: &mut Context<Self>) -> AnyElement {
        let readers = Readers::of(cx);
        let Some(reader) = readers.reader(id) else {
            return div().into_any_element();
        };
        let was = readers.approved_version(id);
        let wider = readers.approved_grant(id).is_none_or(|granted| matches!(&reader.standing, Standing::Waiting(asked) | Standing::Modified(asked) if asked.read > granted.read));
        let standing_now = reader.standing.clone();
        let origin = origin_label(reader);
        let folder = shown_path(&reader.folder);
        let grant = shown_grant(reader);
        let manifest = reader.manifest.clone();
        let name = name_of(reader);
        let theme = cx.theme();
        let (muted, yellow, red) = (theme.muted_foreground, theme.yellow, theme.red);
        let version = manifest.as_ref().map_or_else(String::new, |manifest| manifest.version.clone());
        let awaiting = standing_now.awaits_approval();
        let update = matches!((&standing_now, &was), (Standing::Waiting(_), Some(was)) if *was != version);
        let title = match (&standing_now, &was) {
            (Standing::Waiting(_), Some(was)) if *was != version => format!("{name} {version} asks for more than {was} did"),
            (Standing::Modified(_), _) => format!("{name}'s files changed: approve {name} {version}?"),
            (Standing::Waiting(_), _) => format!("Approve {name} {version}?"),
            _ => format!("{name} {version}"),
        };
        let (standing, color) = standing_look(&standing_now, cx);
        let back = cx.entity().downgrade();
        let heading = Heading::new(title).media(mark(IconName::FileCode, true, cx)).subtitle(
            h_flex()
                .min_w_0()
                .gap_1p5()
                .mt_1()
                .child(chip(origin, None, cx))
                .child(chip(standing, Some(color), cx))
                .child(div().flex_1().min_w_0().xs_text().text_color(muted).ellipsis().child(format!("Installed in {folder}"))),
        );
        let mut body = v_flex().gap_6().child(
            v_flex()
                .gap_2()
                .child(crumbs(READERS, name.clone(), move |_, cx| {
                    let _ = back.update(cx, |this, cx| this.show(Page::Readers, cx));
                }, cx))
                .child(heading.render(cx)),
        );
        if awaiting {
            body = body.child(callout(
                IconName::ShieldCheck,
                format!(
                    "Approve only a reader you trust. Once approved, {name} can read the files of any repository you open in \
                     SterneGit. It can't change them, send them anywhere or run anything."
                ),
                yellow,
            ));
        }
        match &standing_now {
            Standing::Unusable(problem) => body = body.child(div().sm_text().text_color(red).child(problem.clone())),
            Standing::Failed(stop) => {
                let log = stop_log(stop);
                let copied = log.join("\n");
                let mono = cx.theme().mono_font_family.clone();
                body = body.child(group(
                    "Why it stopped",
                    Some(stop.told_of(READER).into()),
                    vec![
                        v_flex()
                            .p_3()
                            .gap_0p5()
                            .font_family(mono)
                            .xs_text()
                            .text_color(muted)
                            .when(log.is_empty(), |this| this.child(NO_LOG))
                            .children(log.into_iter().map(|line| div().child(line)))
                            .into_any_element(),
                        slim_row()
                            .justify_end()
                            .child(
                                Button::new("copy-reader-log")
                                    .small()
                                    .icon(IconName::Copy)
                                    .label(COPY_LOG)
                                    .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))),
                            )
                            .into_any_element(),
                    ],
                    cx,
                ));
            }
            _ => {}
        }
        if let Some(grant) = grant {
            body = body.child(grant_list(&grant, manifest.as_deref(), update, wider, cx));
            if let (true, Some(was)) = (awaiting, &was) {
                body = body.child(div().sm_text().text_color(muted).child(format!("Everything else stays as you approved it for {was}.")));
            }
        }
        if awaiting {
            let approved = id.clone();
            let back = cx.entity().downgrade();
            body = body.child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("reader-not-now").label(NOT_NOW).on_click(cx.listener(|this, _, _, cx| this.show(Page::Readers, cx))))
                    .child(
                        Button::new("reader-approve")
                            .primary()
                            .icon(IconName::ShieldCheck)
                            .label(APPROVE)
                            .on_click(move |_, _, cx| {
                                let result = approve(cx, &approved);
                                let _ = back.update(cx, |this, cx| match result {
                                    Ok(()) => this.show(Page::Readers, cx),
                                    Err(error) => this.tell(format!("Couldn't approve it: {error}"), cx),
                                });
                            }),
                    ),
            );
        }
        if let Some(note) = &self.note {
            body = body.child(div().sm_text().text_color(red).child(note.clone()));
        }
        page(body).into_any_element()
    }
}

fn shared_row(ix: usize, extension: String, claimers: Vec<(ReaderId, String)>, chosen: Option<String>, cx: &App) -> AnyElement {
    let shown = match chosen.as_deref() {
        Some("") => AS_TEXT.to_owned(),
        Some(id) => claimers.iter().find(|(claimer, _)| claimer.0 == id).map_or_else(|| id.to_owned(), |(_, name)| name.clone()),
        None => claimers.first().map(|(_, name)| name.clone()).unwrap_or_default(),
    };
    let mono = cx.theme().mono_font_family.clone();
    let picked = extension.clone();
    let picker = Picker::new(("reader-choice", ix), shown).menu(move |menu, _, _| {
        let menu = claimers.iter().fold(menu, |menu, (id, name)| {
            let (id, extension) = (id.clone(), picked.clone());
            menu.item(PopupMenuItem::new(name.clone()).on_click(move |_, _, cx| {
                let _ = choose(cx, &extension, Some(&id));
            }))
        });
        let extension = picked.clone();
        menu.separator().item(PopupMenuItem::new(AS_TEXT).on_click(move |_, _, cx| {
            let _ = choose(cx, &extension, None);
        }))
    });
    slim_row()
        .child(div().flex_1().sm_text().font_family(mono).child(extension))
        .child(div().w(px(220.)).child(picker))
        .into_any_element()
}

fn grant_list(grant: &Grant, manifest: Option<&crate::read::Manifest>, update: bool, wider: bool, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (muted, yellow) = (theme.muted_foreground, theme.yellow);
    let fresh = || update.then(|| chip("New", Some(yellow), cx));
    let line = |icon: IconName, content: AnyElement, side: Option<AnyElement>| {
        slim_row()
            .child(Icon::new(icon).small().text_color(muted))
            .child(div().flex_1().min_w_0().child(content))
            .children(side)
            .into_any_element()
    };
    let mut groups = v_flex().gap_6();
    if !update || wider {
    groups = groups.child(group(
        FILES_TITLE,
        Some(FILES_ABOUT.into()),
        vec![line(
            if grant.read == Reach::Repository { IconName::FolderGit2 } else { IconName::File },
            div().sm_text().child(reach_line(grant.read)).into_any_element(),
            fresh(),
        )],
        cx,
    ));
    }
    if let (false, Some(manifest)) = (update, manifest) {
        let mono = theme.mono_font_family.clone();
        groups = groups.child(group(
            KINDS_TITLE,
            None,
            manifest
                .kinds
                .iter()
                .map(|kind| {
                    line(
                        IconName::FileCode,
                        h_flex()
                            .gap_1p5()
                            .sm_text()
                            .child(kind.label.clone())
                            .child(div().xs_text().text_color(muted).font_family(mono.clone()).child(kind.extensions.join(" ")))
                            .into_any_element(),
                        None,
                    )
                })
                .collect(),
            cx,
        ));
    }
    if !grant.jobs.is_empty() {
        groups = groups.child(group(
            JOBS_TITLE,
            None,
            grant
                .jobs
                .iter()
                .map(|job| {
                    let (icon, label, about) = job_look(job);
                    line(
                        icon,
                        v_flex()
                            .child(div().sm_text().child(SharedString::from(label.to_owned())))
                            .when(!about.is_empty(), |this| this.child(div().xs_text().text_color(muted).child(about)))
                            .into_any_element(),
                        fresh(),
                    )
                })
                .collect(),
            cx,
        ));
    }
    if !update {
        groups = groups.child(group(
            NEVER_TITLE,
            None,
            NEVER.iter().map(|text| line(IconName::Ban, div().sm_text().child(*text).into_any_element(), None)).collect(),
            cx,
        ));
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::{job_look, reach_line};
    use crate::read::Reach;

    #[test]
    fn every_job_a_reader_can_ask_for_has_words_for_it() {
        for job in ["compare", "side_files", "fixes", "merge"] {
            let (_, label, about) = job_look(job);
            assert!(!about.is_empty() && label != job, "{job}");
        }
        assert!(reach_line(Reach::Repository).contains("working tree"));
    }
}
