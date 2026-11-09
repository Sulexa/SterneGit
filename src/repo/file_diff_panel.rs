use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

use gix::ObjectId;
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, ClickEvent, Context, Entity, EventEmitter,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        diff::{
            Diff, DiffChangeIndicator, DiffChangeMark, DiffFile, DiffHoverHighlight,
            DiffHunkSeparator, DiffMode, DiffState,
        },
        h_flex,
        menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    Repository,
    compare::{
        ChangeRuler, ChangeStepper, Direction, FileBar, FileFacts, FilePlace, OtherFile,
        OtherFileView, PictureCompare, PictureMode, Pictures, Position, SoundCompare, SoundMode,
        Sounds, TemporaryCopy, TextOptions, Version, VersionTag, VideoCompare, VideoSide, Videos,
        ViewBar, text_options_menu,
    },
    hunk_actions::{HunkActions, Offers, Pressed, line_at, line_span},
    merge_editor::MergeEditor,
    navigation::OpenFile,
    patch_view::{
        PatchStats, PatchTexts, RedrawAfterMeasuring, change_kind, parse_patch, patch_content,
        renamed_from, WorkingPatch, working_patch,
    },
};
use crate::{
    git::{
        Baseline, ChangeKind, Conflicted, DifftoolTarget, FileChange, FilePatch, Hunks, Part,
        PartAction, Repo, Revision, Scope, Side, Version as Kept, Whitespace,
    },
    media::{
        Backdrop, MediaKind, Picture, Sound, VideoFacts, extension, ffmpeg_installed, media_kind,
        soundtrack, thumbnails, visible_changes,
    },
    ui::{self, Job, Load, Look as _, SideUi as _, handler, run_in_background},
};

const TEXTS: PatchTexts = PatchTexts {
    loading: "Reading changes…",
    failed: "Couldn't compare this file",
    empty: "No changes to show: only whitespace, line endings or file mode differ.",
};

const ROWS_ABOVE_A_CHANGE: usize = 8;
const SIDES_HEIGHT: f32 = 32.;

pub(crate) enum FileDiffEvent {
    Close,
    WholeWindow,
    StepFile(Direction),
    Resolve(FileChange, Resolution),
}

pub(crate) enum Resolution {
    Text(Vec<u8>),
    Keep(Kept),
}

enum Merging {
    Reading,
    Read(FileChange, String),
    Open(Entity<MergeEditor>),
}

enum Compared {
    WorkingFile(OpenFile),
    CommitFile {
        id: ObjectId,
        baseline: Baseline,
        short_id: SharedString,
        file: DiffFile,
    },
}

struct WorkingParts {
    side: Side,
    hunks: Arc<Hunks>,
}

struct Heading {
    kind: ChangeKind,
    path: SharedString,
    original: Option<SharedString>,
    state: SharedString,
}

enum Media {
    Pictures(Entity<PictureCompare>),
    Sounds(Entity<SoundCompare>),
    Videos(Entity<VideoCompare>),
    Other(Arc<OtherFile>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MediaView {
    Pictures,
    Sounds,
    Videos,
    Other,
}

#[derive(Clone)]
struct VersionSource {
    revision: Revision,
    path: OsString,
}

#[derive(Clone)]
struct Sides {
    before: Option<VersionSource>,
    after: Option<VersionSource>,
}

#[derive(Clone)]
struct MediaRequest {
    view: MediaView,
    sides: Sides,
    sources: [SharedString; 2],
    extension: Option<String>,
    difftool: Option<DifftoolTarget>,
    path: OsString,
}

pub(crate) struct FileDiffPanel {
    repository: Entity<Repository>,
    compared: Option<Compared>,
    patch: Load<PatchStats>,
    media: Load<Media>,
    media_view: Option<MediaView>,
    opened: Option<MediaRequest>,
    stored_with_lfs: bool,
    rereading: Job,
    opening: Job,
    diff: Entity<DiffState>,
    parts: Option<WorkingParts>,
    pressed: Option<Pressed>,
    stepped: Option<usize>,
    place: FilePlace,
    whitespace: Whitespace,
    redraw_after_measuring: RedrawAfterMeasuring,
    merging: Option<Merging>,
    merge_read: Job,
    merge_observer: Option<Subscription>,
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<FileDiffEvent> for FileDiffPanel {}

impl FileDiffPanel {
    pub(crate) fn new(repository: Entity<Repository>, cx: &mut Context<Self>) -> Self {
        let options = TextOptions::current(cx);
        let diff = cx.new(|cx| {
            DiffState::new([], cx)
                .with_mode(DiffMode::Split)
                .with_inline_unit(options.inline_unit())
                .with_context_lines(options.context_lines())
        });
        let subscriptions = [
            cx.observe(&diff, |_, _, cx| cx.notify()),
            cx.observe_global::<TextOptions>(|this, cx| this.apply_options(cx)),
        ];
        Self {
            repository,
            compared: None,
            patch: Load::Idle,
            media: Load::Idle,
            media_view: None,
            opened: None,
            stored_with_lfs: false,
            rereading: Job::default(),
            opening: Job::default(),
            diff,
            parts: None,
            pressed: None,
            stepped: None,
            place: FilePlace::default(),
            whitespace: options.whitespace(),
            redraw_after_measuring: RedrawAfterMeasuring::default(),
            merging: None,
            merge_read: Job::default(),
            merge_observer: None,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn use_repository(&mut self, repository: Entity<Repository>) {
        self.repository = repository;
    }

    pub(crate) fn place(&mut self, place: FilePlace, cx: &mut Context<Self>) {
        if self.place != place {
            self.place = place;
            cx.notify();
        }
    }

    pub(crate) fn open_working_file(
        &mut self,
        repo: &Repo,
        change: FileChange,
        side: Side,
        cx: &mut Context<Self>,
    ) {
        let file = OpenFile {
            side,
            path: change.path.clone(),
        };
        let staged_too = side == Side::Unstaged
            && self
                .repository
                .read(cx)
                .changes()
                .is_some_and(|changes| changes.find(Side::Staged, &change.path).is_some());
        let reload = matches!(&self.compared, Some(Compared::WorkingFile(shown)) if *shown == file);
        if change.kind == ChangeKind::Conflicted {
            self.open_conflict(repo, change, file, reload, cx);
            return;
        }
        self.leave_merge();
        if let Some(view) = media_view(&change.path) {
            let request = working_request(view, &change, side, staged_too);
            self.compared = Some(Compared::WorkingFile(file));
            self.open_media(repo, request, reload, cx);
            return;
        }
        self.leave_media(cx);
        let binary = working_request(MediaView::Other, &change, side, staged_too);
        let repo_for_binary = repo.clone();
        let repo = repo.clone();
        let whitespace = TextOptions::current(cx).whitespace();
        let task = run_in_background(
            cx,
            move || {
                let patch = repo.file_patch_with(&change, side, whitespace)?;
                working_patch(spread_working(&repo, patch, &change, side))
            },
            move |this, result: anyhow::Result<WorkingPatch>, cx| {
                if let Ok(patch) = &result
                    && patch.files.first().is_some_and(DiffFile::is_binary)
                {
                    this.open_media(&repo_for_binary, binary, false, cx);
                    return;
                }
                let stats = result.map(|patch| {
                    let stats = PatchStats::of(&patch.files);
                    this.parts = patch.hunks.map(|hunks| WorkingParts {
                        side,
                        hunks: Arc::new(hunks),
                    });
                    this.diff.update(cx, |state, cx| {
                        if reload {
                            state.refresh_files(patch.files, cx)
                        } else {
                            state.set_files(patch.files, cx)
                        }
                    });
                    if !reload {
                        this.jump_to_change(0, cx);
                    }
                    this.redraw_after_measuring.files_changed();
                    stats
                });
                if stats.is_err() {
                    this.parts = None;
                }
                this.patch.finish(stats);
                cx.notify();
            },
        );
        self.rereading.cancel();
        if reload {
            self.patch.reload(task);
        } else {
            self.parts = None;
            self.stepped = None;
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            self.patch.load(task);
        }
        self.whitespace = whitespace;
        self.compared = Some(Compared::WorkingFile(file));
        cx.notify();
    }

    fn open_conflict(
        &mut self,
        repo: &Repo,
        change: FileChange,
        file: OpenFile,
        reload: bool,
        cx: &mut Context<Self>,
    ) {
        self.compared = Some(Compared::WorkingFile(file));
        if reload
            && let Some(Merging::Open(editor)) = &self.merging
            && editor.read(cx).is_touched()
        {
            return;
        }
        let keep_shown = reload && matches!(self.merging, Some(Merging::Open(_)));
        if !keep_shown {
            self.leave_media(cx);
            self.rereading.cancel();
            self.parts = None;
            self.stepped = None;
            self.patch.clear();
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            self.merging = Some(Merging::Reading);
        }
        let reading = repo.clone();
        let repo = repo.clone();
        let task = run_in_background(
            cx,
            move || reading.read_conflicted(change.exact_path()).map(|read| (change, read)),
            move |this, result, cx| {
                this.merge_read.finish();
                match result {
                    Ok((change, Conflicted::Text(text))) => {
                        this.merging = Some(Merging::Read(change, text));
                    }
                    Ok((change, Conflicted::Versions { current, incoming })) => {
                        this.leave_merge();
                        let request = conflict_request(&change, current, incoming);
                        this.open_media(&repo, request, reload, cx);
                    }
                    Err(error) => {
                        this.leave_merge();
                        this.patch = Load::Failed(ui::Failure::git_reason(&error));
                    }
                }
                cx.notify();
            },
        );
        self.merge_read.run(task);
        cx.notify();
    }

    fn leave_merge(&mut self) {
        self.merging = None;
        self.merge_observer = None;
        self.merge_read.cancel();
    }

    fn merge_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Entity<MergeEditor>> {
        if let Some(Merging::Read(change, text)) = &self.merging {
            let (change, text) = (change.clone(), text.clone());
            let editor = cx.new(|cx| MergeEditor::new(change, &text, window, cx));
            self.merge_observer = Some(cx.observe(&editor, |_, _, cx| cx.notify()));
            self.merging = Some(Merging::Open(editor));
        }
        match &self.merging {
            Some(Merging::Open(editor)) => Some(editor.clone()),
            _ => None,
        }
    }

    fn render_merge(&mut self, heading: Heading, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let editor = self.merge_editor(window, cx);
        let writing = self.repository.read(cx).is_writing();
        let (resolved, facts, position) = match &editor {
            Some(editor) => {
                let editor = editor.read(cx);
                let facts = format!(
                    "{} of {} resolved",
                    editor.resolved_count(),
                    ui::counted(editor.total(), "conflict", "conflicts")
                );
                (editor.is_resolved(), Some(facts), editor.position())
            }
            None => (false, None, None),
        };
        let mark = Button::new("mark-resolved")
            .xsmall()
            .icon(IconName::Check)
            .label("Mark resolved")
            .when(resolved, |button| button.primary())
            .tooltip(if resolved {
                "Save the result and stage the file"
            } else {
                "Resolve every conflict in the result first"
            })
            .disabled(writing || !resolved || editor.is_none())
            .on_click(cx.listener(|this, _, _, cx| this.mark_resolved(cx)));
        let muted = cx.theme().muted_foreground;
        let mut file_bar = self.file_bar(heading, Some(mark.into_any_element()), cx);
        if let Some(facts) = facts {
            file_bar = file_bar.facts(div().text_color(muted).child(facts).into_any_element());
        }
        let view_bar = ViewBar::new()
            .stepper(
                ChangeStepper::new(
                    position,
                    handler(cx.listener(|this, direction: &Direction, window, cx| {
                        this.step_change(*direction, window, cx)
                    })),
                )
                .noun("Conflict"),
            )
            .tool(
                Button::new("text-options")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Ellipsis)
                    .tooltip("View options")
                    .dropdown_menu_with_anchor(Anchor::TopRight, |menu, _, cx| {
                        text_options_menu(menu, cx)
                    }),
            );
        let content = match editor {
            Some(editor) => editor.into_any_element(),
            None => ui::loading("Reading the conflicts…"),
        };
        v_flex()
            .pane()
            .child(file_bar)
            .child(view_bar)
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn mark_resolved(&mut self, cx: &mut Context<Self>) {
        let Some(Merging::Open(editor)) = &self.merging else {
            return;
        };
        let editor = editor.read(cx);
        if !editor.is_resolved() {
            return;
        }
        let event = FileDiffEvent::Resolve(
            editor.change().clone(),
            Resolution::Text(editor.resolved_bytes()),
        );
        cx.emit(event);
    }

    fn keep(&mut self, version: Kept, cx: &mut Context<Self>) {
        let Some(Compared::WorkingFile(open)) = &self.compared else {
            return;
        };
        let change = self
            .repository
            .read(cx)
            .changes()
            .and_then(|changes| changes.find(open.side, &open.path))
            .cloned();
        if let Some(change) = change {
            cx.emit(FileDiffEvent::Resolve(change, Resolution::Keep(version)));
        }
    }

    fn keep_buttons(&self, cx: &mut Context<Self>) -> AnyElement {
        let writing = self.repository.read(cx).is_writing();
        let keep = |id: &'static str, label: &'static str, tooltip: &'static str, version: Kept, cx: &mut Context<Self>| {
            Button::new(id)
                .xsmall()
                .label(label)
                .tooltip(tooltip)
                .disabled(writing)
                .on_click(cx.listener(move |this, _, _, cx| this.keep(version, cx)))
        };
        h_flex()
            .gap_1()
            .child(keep(
                "keep-current",
                "Keep current",
                "Stage the current version of the file",
                Kept::Current,
                cx,
            ))
            .child(keep(
                "keep-incoming",
                "Keep incoming",
                "Stage the incoming version of the file",
                Kept::Incoming,
                cx,
            ))
            .into_any_element()
    }

    pub(crate) fn open_commit_file(
        &mut self,
        id: ObjectId,
        baseline: Baseline,
        short_id: SharedString,
        file: DiffFile,
        cx: &mut Context<Self>,
    ) {
        self.leave_merge();
        let view = media_view(file.path()).or_else(|| file.is_binary().then_some(MediaView::Other));
        if let Some(view) = view {
            let reload = self.shows_commit_file(id, file.path());
            let request = commit_request(view, &file, id, baseline.commit(), &short_id);
            self.compared = Some(Compared::CommitFile { id, baseline, short_id, file });
            if let Some(repo) = self.repository.read(cx).repo().cloned() {
                self.open_media(&repo, request, reload, cx);
            }
            return;
        }
        self.leave_media(cx);
        let same = self.shows_commit_file(id, file.path());
        let stats = PatchStats::of(std::slice::from_ref(&file));
        self.diff.update(cx, |state, cx| {
            if same {
                state.refresh_files([file.clone()], cx)
            } else {
                state.set_files([file.clone()], cx)
            }
        });
        self.redraw_after_measuring.files_changed();
        self.patch = Load::Ready(stats);
        self.parts = None;
        if !same {
            self.stepped = None;
        }
        self.compared = Some(Compared::CommitFile { id, baseline, short_id, file });
        self.reread_commit_file(!same, cx);
        cx.notify();
    }

    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        if self.compared.take().is_some() {
            self.leave_media(cx);
            self.leave_merge();
            self.parts = None;
            self.pressed = None;
            self.stepped = None;
            self.rereading.cancel();
            self.patch.clear();
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            cx.notify();
        }
    }

    pub(crate) fn step_change(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Merging::Open(editor)) = &self.merging {
            let editor = editor.clone();
            editor.update(cx, |editor, cx| editor.step(direction, window, cx));
            return;
        }
        match self.media.value() {
            Some(Media::Pictures(view)) => {
                let view = view.clone();
                view.update(cx, |view, cx| view.step_change(direction, cx));
                return;
            }
            Some(Media::Sounds(view)) => {
                let view = view.clone();
                view.update(cx, |view, cx| view.step_change(direction, cx));
                return;
            }
            Some(Media::Videos(view)) => {
                let view = view.clone();
                view.update(cx, |view, cx| view.step_change(direction, cx));
                return;
            }
            _ => {}
        }
        let state = self.diff.read(cx);
        let marks = state.change_marks();
        let Some(current) = self.change_position(&marks, state.top_row(), state.visible_rows())
        else {
            return;
        };
        let target = match direction {
            Direction::Back => (current.index + current.count - 1) % current.count,
            Direction::Forward => (current.index + 1) % current.count,
        };
        self.jump_to_change(target, cx);
    }

    fn open_media(&mut self, repo: &Repo, mut request: MediaRequest, reload: bool, cx: &mut Context<Self>) {
        let mut note = None;
        if request.view == MediaView::Videos && !ffmpeg_installed() {
            request.view = MediaView::Other;
            note = Some(SharedString::from("Install FFmpeg, with ffmpeg and ffprobe on PATH, to compare videos frame by frame."));
        }
        self.rereading.cancel();
        if !reload {
            self.patch.clear();
            self.parts = None;
            self.stepped = None;
            self.diff.update(cx, |state, cx| state.set_files([], cx));
        }
        self.media_view = Some(request.view);
        self.opened = Some(request.clone());
        let repo = repo.clone();
        let MediaRequest {
            view,
            sides,
            sources,
            extension,
            ..
        } = request;
        let task = run_in_background(
            cx,
            move || match view {
                MediaView::Pictures => read_pictures(&repo, sides, extension.as_deref()).map(Read::Pictures),
                MediaView::Sounds => read_sounds(&repo, sides, extension.as_deref()).map(Read::Sounds),
                MediaView::Videos => read_videos(&repo, sides, extension.as_deref()).map(Read::Videos),
                MediaView::Other => read_other(&repo, sides, extension.as_deref(), note).map(Read::Other),
            },
            move |this, result, cx| {
                match result {
                    Ok(Read::Sounds((sounds, stored_with_lfs))) => {
                        this.stored_with_lfs = stored_with_lfs;
                        this.forget_media(cx);
                        let view = cx.new(|cx| SoundCompare::new(sounds, sources, cx));
                        this.media.finish(Ok(Media::Sounds(view)));
                    }
                    Ok(Read::Videos((videos, stored_with_lfs))) => {
                        this.stored_with_lfs = stored_with_lfs;
                        this.forget_media(cx);
                        let view = cx.new(|cx| VideoCompare::new(videos, sources, cx));
                        this.media.finish(Ok(Media::Videos(view)));
                    }
                    Ok(Read::Other((file, stored_with_lfs))) => {
                        this.stored_with_lfs = stored_with_lfs;
                        this.forget_media(cx);
                        this.media.finish(Ok(Media::Other(Arc::new(file))));
                    }
                    Ok(Read::Pictures((pictures, stored_with_lfs))) => {
                        this.stored_with_lfs = stored_with_lfs;
                        let kept = match this.media.value() {
                            Some(Media::Pictures(view)) if reload => Some(view.clone()),
                            _ => None,
                        };
                        let view = match kept {
                            Some(view) => {
                                view.update(cx, |view, cx| view.replace(pictures, sources, cx));
                                view
                            }
                            None => {
                                this.forget_media(cx);
                                cx.new(|_| PictureCompare::new(pictures, sources))
                            }
                        };
                        this.media.finish(Ok(Media::Pictures(view)));
                    }
                    Err(error) => {
                        this.forget_media(cx);
                        this.media.finish(Err(error));
                    }
                }
                cx.notify();
            },
        );
        if reload && self.media.value().is_some() {
            self.media.reload(task);
        } else {
            self.forget_media(cx);
            self.stored_with_lfs = false;
            self.media.load(task);
        }
        cx.notify();
    }

    fn forget_media(&mut self, cx: &mut Context<Self>) {
        if let Some(Media::Pictures(view)) = self.media.value() {
            let view = view.clone();
            view.update(cx, |view, cx| view.forget_images(cx));
        }
    }

    fn leave_media(&mut self, cx: &mut Context<Self>) {
        self.forget_media(cx);
        self.media.clear();
        self.media_view = None;
        self.opened = None;
        self.opening.cancel();
        self.stored_with_lfs = false;
    }

    fn open_version(&mut self, version: Version, cx: &mut Context<Self>) {
        let Some(repo) = self.repository.read(cx).repo().cloned() else {
            return;
        };
        let Some(opened) = &self.opened else {
            return;
        };
        let source = match version {
            Version::Before => opened.sides.before.clone(),
            Version::After => opened.sides.after.clone(),
        };
        let Some(source) = source else {
            return;
        };
        if source.revision == Revision::WorkTree {
            cx.open_with_system(&repo.location().join(&source.path));
            return;
        }
        let task = run_in_background(
            cx,
            move || copy_version(&repo, &source, version),
            |this, result: anyhow::Result<PathBuf>, cx| {
                this.opening.finish();
                if let Ok(path) = result {
                    cx.open_with_system(&path);
                }
            },
        );
        self.opening.run(task);
    }

    fn run_difftool(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.repository.read(cx).repo().cloned() else {
            return;
        };
        let Some((target, path)) = self
            .opened
            .as_ref()
            .and_then(|opened| Some((opened.difftool.clone()?, opened.path.clone())))
        else {
            return;
        };
        let task = run_in_background(
            cx,
            move || repo.run_difftool(&target, &path),
            |this, _, _| this.opening.finish(),
        );
        self.opening.run(task);
    }

    fn picture_bar(&self, view: &Entity<PictureCompare>, cx: &mut Context<Self>) -> ViewBar {
        let picture = view.read(cx);
        let current = picture.mode();
        let position = picture.change_position();
        let percent = picture.zoom_percent();
        let fits = picture.fits();
        let backdrop = picture.backdrop();
        let mut bar = ViewBar::new();
        if picture.compares_both() {
            for mode in PictureMode::ALL {
                let view = view.clone();
                bar = bar.mode(
                    mode.id(),
                    mode.label(),
                    mode.icon(),
                    mode.tooltip(),
                    mode == current,
                    move |_, _, cx| view.update(cx, |view, cx| view.set_mode(mode, cx)),
                );
            }
            let view = view.clone();
            bar = bar.stepper(ChangeStepper::new(
                position,
                handler(move |direction: &Direction, _, cx| {
                    view.update(cx, |view, cx| view.step_change(*direction, cx))
                }),
            ));
        }
        let zoom_out = view.clone();
        let zoom_in = view.clone();
        let fit = view.clone();
        let choose_backdrop = view.clone();
        let muted = cx.theme().muted_foreground;
        bar.tool(
            Button::new("zoom-out")
                .ghost()
                .xsmall()
                .icon(IconName::Minus)
                .tooltip("Zoom out")
                .on_click(move |_, _, cx| zoom_out.update(cx, |view, cx| view.zoom_out(cx))),
        )
        .tool(
            div()
                .min_w(px(ZOOM_TEXT_WIDTH))
                .text_center()
                .xs_text()
                .text_color(muted)
                .child(percent.map(|percent| format!("{} %", ui::thousands(percent.round() as usize))).unwrap_or_default()),
        )
        .tool(
            Button::new("zoom-in")
                .ghost()
                .xsmall()
                .icon(IconName::Plus)
                .tooltip("Zoom in, or hold Ctrl and turn the wheel")
                .on_click(move |_, _, cx| zoom_in.update(cx, |view, cx| view.zoom_in(cx))),
        )
        .tool(
            Button::new("zoom-fit")
                .ghost()
                .xsmall()
                .label("Fit")
                .selected(fits)
                .tooltip("Make the pictures as large as the pane allows")
                .on_click(move |_, _, cx| fit.update(cx, |view, cx| view.fit(cx))),
        )
        .tool(
            Button::new("picture-backdrop")
                .ghost()
                .xsmall()
                .tooltip("What shows behind transparent pixels")
                .child(backdrop_swatch(backdrop, cx))
                .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                    Backdrop::ALL.into_iter().fold(menu, |menu, choice| {
                        let view = choose_backdrop.clone();
                        menu.item(
                            PopupMenuItem::new(choice.label())
                                .checked(choice == backdrop)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |view, cx| view.set_backdrop(choice, cx))
                                }),
                        )
                    })
                }),
        )
    }

    fn video_bar(&self, video: &Entity<VideoCompare>, cx: &mut Context<Self>) -> ViewBar {
        let frames = video.read(cx).frames().clone();
        let position = video.read(cx).change_position();
        let mut bar = self.picture_bar(&frames, cx).without_stepper();
        let stepping = video.clone();
        bar = bar.stepper(ChangeStepper::new(
            position,
            handler(move |direction: &Direction, _, cx| {
                stepping.update(cx, |video, cx| video.step_change(*direction, cx))
            }),
        ));
        bar
    }

    fn sound_bar(&self, view: &Entity<SoundCompare>, cx: &mut Context<Self>) -> ViewBar {
        let sound = view.read(cx);
        let current = sound.mode();
        let position = sound.change_position();
        let fits = sound.fits();
        let mut bar = ViewBar::new();
        if sound.compares_both() {
            for mode in SoundMode::ALL {
                let view = view.clone();
                bar = bar.mode(
                    mode.id(),
                    mode.label(),
                    mode.icon(),
                    mode.tooltip(),
                    mode == current,
                    move |_, _, cx| view.update(cx, |view, cx| view.set_mode(mode, cx)),
                );
            }
            let view = view.clone();
            bar = bar.stepper(ChangeStepper::new(
                position,
                handler(move |direction: &Direction, _, cx| {
                    view.update(cx, |view, cx| view.step_change(*direction, cx))
                }),
            ));
        }
        let zoom_out = view.clone();
        let zoom_in = view.clone();
        let fit = view.clone();
        bar.tool(
            Button::new("time-zoom-out")
                .ghost()
                .xsmall()
                .icon(IconName::Minus)
                .tooltip("Show more time")
                .on_click(move |_, _, cx| zoom_out.update(cx, |view, cx| view.zoom_out(cx))),
        )
        .tool(
            Button::new("time-zoom-in")
                .ghost()
                .xsmall()
                .icon(IconName::Plus)
                .tooltip("Show less time, or hold Ctrl and turn the wheel")
                .on_click(move |_, _, cx| zoom_in.update(cx, |view, cx| view.zoom_in(cx))),
        )
        .tool(
            Button::new("time-fit")
                .ghost()
                .xsmall()
                .label("Fit")
                .selected(fits)
                .tooltip("Show the whole length")
                .on_click(move |_, _, cx| fit.update(cx, |view, cx| view.fit(cx))),
        )
    }

    fn media_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = self.media_view;
        match &self.media {
            Load::Idle => div().into_any_element(),
            Load::Loading { .. } if view == Some(MediaView::Pictures) => ui::loading("Reading pictures…"),
            Load::Loading { .. } if view == Some(MediaView::Sounds) => ui::loading("Decoding both versions…"),
            Load::Loading { .. } if view == Some(MediaView::Videos) => ui::loading("Reading both videos with FFmpeg…"),
            Load::Failed(failure) if view == Some(MediaView::Videos) => {
                ui::failure("Couldn't compare these videos", failure.message(), cx)
            }
            Load::Loading { .. } => ui::loading("Reading the file…"),
            Load::Failed(failure) if view == Some(MediaView::Pictures) => {
                ui::failure("Couldn't compare these pictures", failure.message(), cx)
            }
            Load::Failed(failure) if view == Some(MediaView::Sounds) => {
                ui::failure("Couldn't compare these sounds", failure.message(), cx)
            }
            Load::Failed(failure) => ui::failure("Couldn't read this file", failure.message(), cx),
            Load::Refreshing { value, .. } | Load::Ready(value) => match value {
                Media::Pictures(view) => view.clone().into_any_element(),
                Media::Sounds(view) => view.clone().into_any_element(),
                Media::Videos(view) => view.clone().into_any_element(),
                Media::Other(file) => {
                    let sources = self
                        .opened
                        .as_ref()
                        .map(|opened| opened.sources.clone())
                        .unwrap_or_default();
                    OtherFileView::new(
                        file.clone(),
                        sources,
                        handler(cx.listener(|this, version: &Version, _, cx| {
                            this.open_version(*version, cx)
                        })),
                        handler(cx.listener(|this, _: &(), _, cx| this.run_difftool(cx))),
                    )
                    .into_any_element()
                }
            },
        }
    }

    pub(crate) fn shows_commit_file(&self, id: ObjectId, path: &SharedString) -> bool {
        matches!(&self.compared, Some(Compared::CommitFile { id: shown, file, .. }) if *shown == id && file.path() == path)
    }

    fn reread_commit_file(&mut self, from_the_top: bool, cx: &mut Context<Self>) {
        let Some(Compared::CommitFile { id, baseline, file, .. }) = &self.compared else {
            return;
        };
        let Some(repo) = self.repository.read(cx).repo().cloned() else {
            return;
        };
        let (id, baseline) = (*id, *baseline);
        let path = file.path().clone();
        let original = renamed_from(file).cloned();
        let kind = change_kind(file);
        let whitespace = TextOptions::current(cx).whitespace();
        let task = run_in_background(
            cx,
            move || -> anyhow::Result<Option<DiffFile>> {
                let paths: Vec<&OsStr> = std::iter::once(&path)
                    .chain(&original)
                    .map(|path| OsStr::new(path.as_ref()))
                    .collect();
                let mut text = repo.commit_file_patch(id, baseline, &paths, whitespace)?;
                if spreads(kind) {
                    text = repo.with_whole_file(text, Revision::Commit(id), OsStr::new(path.as_ref()));
                }
                Ok(parse_patch(&text)?
                    .into_iter()
                    .find(|file| *file.path() == path))
            },
            move |this, result, cx| {
                this.rereading.finish();
                let Ok(reread) = result else {
                    return;
                };
                let Some(Compared::CommitFile { file: shown, .. }) = &mut this.compared else {
                    return;
                };
                let files: Vec<DiffFile> = reread.iter().cloned().collect();
                if let Some(reread) = reread {
                    *shown = reread;
                }
                this.patch = Load::Ready(PatchStats::of(&files));
                this.diff.update(cx, |state, cx| {
                    if from_the_top {
                        state.set_files(files, cx)
                    } else {
                        state.refresh_files(files, cx)
                    }
                });
                if from_the_top {
                    this.jump_to_change(0, cx);
                }
                this.redraw_after_measuring.files_changed();
                cx.notify();
            },
        );
        self.whitespace = whitespace;
        self.rereading.run(task);
    }

    fn apply_options(&mut self, cx: &mut Context<Self>) {
        let options = TextOptions::current(cx);
        self.diff.update(cx, |state, cx| {
            state.set_inline_unit(options.inline_unit(), cx);
            state.set_context_lines(options.context_lines(), cx);
        });
        if options.whitespace() != self.whitespace {
            self.reread(cx);
        }
        cx.notify();
    }

    fn reread(&mut self, cx: &mut Context<Self>) {
        match &self.compared {
            Some(Compared::WorkingFile(open)) => {
                let open = open.clone();
                let repository = self.repository.read(cx);
                let change = repository
                    .changes()
                    .and_then(|changes| changes.find(open.side, &open.path))
                    .cloned();
                if let (Some(repo), Some(change)) = (repository.repo().cloned(), change) {
                    self.open_working_file(&repo, change, open.side, cx);
                }
            }
            Some(Compared::CommitFile { .. }) => self.reread_commit_file(false, cx),
            None => {}
        }
    }

    fn jump_to_change(&mut self, change: usize, cx: &mut Context<Self>) {
        let state = self.diff.read(cx);
        let Some(mark) = state.change_marks().get(change).copied() else {
            return;
        };
        let to = mark.first_row.saturating_sub(ROWS_ABOVE_A_CHANGE);
        self.stepped = Some(change);
        self.diff.update(cx, |state, cx| state.scroll_to_row(to, cx));
    }

    fn change_position(
        &self,
        marks: &[DiffChangeMark],
        top: usize,
        shown: std::ops::Range<usize>,
    ) -> Option<Position> {
        let count = marks.len();
        if count == 0 {
            return None;
        }
        let index = match self.stepped {
            Some(stepped)
                if marks
                    .get(stepped)
                    .is_some_and(|mark| shown.contains(&mark.first_row)) =>
            {
                stepped
            }
            _ => marks
                .partition_point(|mark| mark.first_row < top)
                .min(count - 1),
        };
        Some(Position { index, count })
    }

    fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        self.stepped = None;
        self.diff.update(cx, |state, cx| state.set_mode(mode, cx));
    }

    fn move_across(&mut self, cx: &mut Context<Self>) {
        let Some(Compared::WorkingFile(open)) = &self.compared else {
            return;
        };
        let open = open.clone();
        let change = self
            .repository
            .read(cx)
            .changes()
            .and_then(|changes| changes.find(open.side, &open.path))
            .cloned();
        if let Some(change) = change {
            self.repository.update(cx, |repository, cx| {
                repository.move_across(open.side, Scope::Files(vec![change]), cx)
            });
        }
    }

    fn apply(&mut self, action: PartAction, part: &Part, cx: &mut Context<Self>) {
        let Some(patch) = self
            .parts
            .as_ref()
            .and_then(|parts| parts.hunks.patch(part, action))
        else {
            return;
        };
        self.repository.update(cx, |repository, cx| {
            repository.apply_part(action, patch, cx)
        });
    }

    fn hunk_actions(&self, cx: &mut Context<Self>) -> Option<HunkActions> {
        let parts = self
            .parts
            .as_ref()
            .filter(|_| matches!(self.compared, Some(Compared::WorkingFile(_))))?;
        let selection = self
            .diff
            .read(cx)
            .selected_lines()
            .map(|range| line_span(&range));
        let enabled = !self.repository.read(cx).is_writing() && !self.patch.is_loading();
        let on_apply = handler(
            cx.listener(|this, (action, part): &(PartAction, Part), _, cx| {
                this.apply(*action, part, cx)
            }),
        );
        Some(HunkActions::new(
            Offers::new(parts.hunks.clone(), parts.side, selection),
            enabled,
            on_apply,
        ))
    }

    fn diff_view(&self, actions: Option<HunkActions>, cx: &mut Context<Self>) -> AnyElement {
        let diff = Diff::new(&self.diff)
            .line_number(true)
            .soft_wrap(TextOptions::current(cx).wraps())
            .header_visible(false)
            .hunk_separator(DiffHunkSeparator::Scope)
            .change_indicator(DiffChangeIndicator::Bars)
            .border_0();
        let Some(actions) = actions else {
            return diff.into_any_element();
        };
        let panel = cx.entity().downgrade();
        let on_line = panel.clone();
        let on_hunk = panel.clone();
        let buttons = actions.clone();
        let diff = diff
            .hover_highlight(DiffHoverHighlight::Line)
            .on_line_secondary_press(move |position, _, cx| {
                let pressed = Pressed::Line(line_at(position));
                let _ = on_line.update(cx, |this, _| this.pressed = Some(pressed));
            })
            .on_hunk_secondary_press(move |_, hunk, _, cx| {
                let _ = on_hunk.update(cx, |this, _| this.pressed = Some(Pressed::Hunk(hunk)));
            })
            .render_hunk_suffix(move |_, hunk, _, _| buttons.buttons(hunk));
        div()
            .id("working-diff")
            .size_full()
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, _, _| {
                if event.button == MouseButton::Right {
                    this.pressed = None;
                }
            }))
            .child(diff)
            .context_menu(move |menu, _, cx| {
                let at = panel.upgrade().and_then(|panel| panel.read(cx).pressed);
                actions.menu(menu, at)
            })
            .into_any_element()
    }

    fn text_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let actions = self.hunk_actions(cx);
        let diff_view = self.diff_view(actions, cx);
        let sides = self.sides_row(cx);
        let state = self.diff.read(cx);
        let ruler = ChangeRuler::new(
            state.change_marks(),
            state.row_count(),
            state.visible_rows(),
            handler(cx.listener(|this, change: &usize, _, cx| this.jump_to_change(*change, cx))),
        );
        h_flex()
            .size_full()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .children(sides)
                    .child(div().flex_1().min_h_0().child(diff_view)),
            )
            .child(ruler)
            .into_any_element()
    }

    fn text_sides(&self, cx: &App) -> Option<[SharedString; 2]> {
        match self.compared.as_ref()? {
            Compared::WorkingFile(open) => {
                let changes = self.repository.read(cx).changes()?;
                let change = changes.find(open.side, &open.path)?;
                let staged_too = open.side == Side::Unstaged
                    && changes.find(Side::Staged, &open.path).is_some();
                Some(working_sides(change, open.side, staged_too).1)
            }
            Compared::CommitFile {
                id,
                baseline,
                short_id,
                file,
            } => Some(commit_sides(file, *id, baseline.commit(), short_id).1),
        }
    }

    fn sides_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let [before, after] = self.text_sides(cx)?;
        let split = self.diff.read(cx).mode() == DiffMode::Split;
        let border = cx.theme().border;
        let before = VersionTag::new(Version::Before, before);
        let after = VersionTag::new(Version::After, after);
        let row = h_flex()
            .flex_none()
            .w_full()
            .h(px(SIDES_HEIGHT))
            .border_b_1()
            .border_color(border);
        Some(if split {
            let half = || h_flex().flex_1().min_w_0().h_full().px_2();
            row.child(half().child(before))
                .child(half().border_l_1().border_color(border).child(after))
                .into_any_element()
        } else {
            row.px_2().gap_2().child(before).child(after).into_any_element()
        })
    }

    fn move_button(&self, open: &OpenFile, listed: bool, cx: &mut Context<Self>) -> AnyElement {
        let labels = open.side.labels();
        let writing = self.repository.read(cx).is_writing();
        Button::new("diff-stage")
            .xsmall()
            .icon(labels.icon)
            .label(labels.verb)
            .tooltip(labels.open_file_tooltip)
            .disabled(writing || !listed)
            .on_click(cx.listener(|this, _, _, cx| this.move_across(cx)))
            .into_any_element()
    }

    fn file_bar(&self, heading: Heading, move_button: Option<AnyElement>, cx: &mut Context<Self>) -> FileBar {
        let shown_path = match &heading.original {
            Some(original) => SharedString::from(format!("{original} → {}", heading.path)),
            None => heading.path.clone(),
        };
        let stats = self.patch.value().copied();
        let facts = match (self.media_view, self.media.value()) {
            (_, Some(Media::Pictures(view))) => Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(view.read(cx).facts())
                    .into_any_element(),
            ),
            (_, Some(Media::Sounds(view))) => Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(view.read(cx).facts())
                    .into_any_element(),
            ),
            (_, Some(Media::Videos(view))) => Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(view.read(cx).facts())
                    .into_any_element(),
            ),
            (_, Some(Media::Other(file))) => Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(file.facts())
                    .into_any_element(),
            ),
            (Some(_), None) => None,
            (None, _) => stats.map(|stats| {
                h_flex()
                    .gap_2()
                    .child(ui::additions(stats.additions, cx))
                    .child(ui::deletions(stats.deletions, cx))
                    .into_any_element()
            }),
        };
        let bar = FileBar::new(
            heading.kind,
            shown_path,
            heading.state,
            self.place,
            handler(cx.listener(|_, _: &(), _, cx| cx.emit(FileDiffEvent::WholeWindow))),
            handler(cx.listener(|_, direction: &Direction, _, cx| {
                cx.emit(FileDiffEvent::StepFile(*direction))
            })),
            handler(cx.listener(|_, _: &(), _, cx| cx.emit(FileDiffEvent::Close))),
        )
        .move_button(move_button)
        .stored_with_lfs(self.stored_with_lfs);
        match facts {
            Some(facts) => bar.facts(facts),
            None => bar,
        }
    }

    fn view_bar(&self, cx: &mut Context<Self>) -> ViewBar {
        let state = self.diff.read(cx);
        let mode = state.mode();
        let position =
            self.change_position(&state.change_marks(), state.top_row(), state.visible_rows());
        let choose = |mode: DiffMode| {
            cx.listener(move |this: &mut Self, _: &ClickEvent, _, cx| this.set_mode(mode, cx))
        };
        ViewBar::new()
            .mode(
                "diff-split",
                "Split",
                IconName::Columns2,
                "Compare side by side",
                mode == DiffMode::Split,
                choose(DiffMode::Split),
            )
            .mode(
                "diff-unified",
                "Unified",
                IconName::Rows2,
                "Show changes in one column",
                mode == DiffMode::Unified,
                choose(DiffMode::Unified),
            )
            .stepper(ChangeStepper::new(
                position,
                handler(cx.listener(|this, direction: &Direction, window, cx| {
                    this.step_change(*direction, window, cx)
                })),
            ))
            .tool(
                Button::new("text-options")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Ellipsis)
                    .tooltip("View options")
                    .dropdown_menu_with_anchor(Anchor::TopRight, |menu, _, cx| {
                        text_options_menu(menu, cx)
                    }),
            )
    }
}

impl Render for FileDiffPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.redraw_after_measuring.request(window);
        let (heading, move_button) = match &self.compared {
            None => return div().into_any_element(),
            Some(Compared::WorkingFile(open)) => {
                let change = self
                    .repository
                    .read(cx)
                    .changes()
                    .and_then(|changes| changes.find(open.side, &open.path))
                    .cloned();
                let listed = change.is_some();
                let conflicted = change
                    .as_ref()
                    .is_some_and(|change| change.kind == ChangeKind::Conflicted);
                let heading = working_heading(open, change);
                if self.merging.is_some() {
                    return self.render_merge(heading, window, cx);
                }
                let button = if conflicted {
                    self.keep_buttons(cx)
                } else {
                    self.move_button(open, listed, cx)
                };
                (heading, Some(button))
            }
            Some(Compared::CommitFile { short_id, file, .. }) => {
                (commit_heading(short_id, file), None)
            }
        };
        let mut file_bar = self.file_bar(heading, move_button, cx);
        let shown_pictures = match self.media.value() {
            Some(Media::Pictures(view)) => Some(view.clone()),
            _ => None,
        };
        let shown_sounds = match self.media.value() {
            Some(Media::Sounds(view)) => Some(view.clone()),
            _ => None,
        };
        let shown_videos = match self.media.value() {
            Some(Media::Videos(view)) => Some(view.clone()),
            _ => None,
        };
        let (view_bar, content) = match (self.media_view, shown_pictures, shown_sounds) {
            (_, Some(view), _) => (Some(self.picture_bar(&view, cx)), self.media_content(cx)),
            (_, _, Some(view)) => (Some(self.sound_bar(&view, cx)), self.media_content(cx)),
            (Some(MediaView::Videos), None, None) => match shown_videos {
                Some(video) => (Some(self.video_bar(&video, cx)), self.media_content(cx)),
                None => (Some(ViewBar::new()), self.media_content(cx)),
            },
            (Some(MediaView::Pictures | MediaView::Sounds), None, None) => (Some(ViewBar::new()), self.media_content(cx)),
            (Some(MediaView::Other), None, None) => (None, self.media_content(cx)),
            (None, _, _) => {
                file_bar = file_bar.view(self.view_bar(cx));
                let text_view = self
                    .patch
                    .value()
                    .is_some()
                    .then(|| self.text_view(cx));
                let content = patch_content(
                    &self.patch,
                    |stats| *stats,
                    &TEXTS,
                    |_| text_view.unwrap_or_else(|| div().into_any_element()),
                    cx,
                );
                (None, content)
            }
        };
        v_flex()
            .pane()
            .child(file_bar)
            .children(view_bar)
            .child(div().fill_height().child(content))
            .into_any_element()
    }
}

const ZOOM_TEXT_WIDTH: f32 = 52.;
const SWATCH: f32 = 12.;
const SWATCH_SQUARE: f32 = 4.;

enum Read {
    Pictures((Pictures, bool)),
    Sounds((Sounds, bool)),
    Videos((Videos, bool)),
    Other((OtherFile, bool)),
}

fn media_view(path: &str) -> Option<MediaView> {
    match media_kind(path)? {
        MediaKind::Picture => Some(MediaView::Pictures),
        MediaKind::Sound => Some(MediaView::Sounds),
        MediaKind::Video => Some(MediaView::Videos),
    }
}

fn working_request(view: MediaView, change: &FileChange, side: Side, staged_too: bool) -> MediaRequest {
    let (sides, sources) = working_sides(change, side, staged_too);
    MediaRequest {
        view,
        sides,
        sources,
        extension: extension(&change.path),
        difftool: Some(match side {
            Side::Unstaged => DifftoolTarget::Unstaged,
            Side::Staged => DifftoolTarget::Staged,
        }),
        path: change.exact_path().to_owned(),
    }
}

fn commit_request(
    view: MediaView,
    file: &DiffFile,
    id: ObjectId,
    parent: Option<ObjectId>,
    short_id: &SharedString,
) -> MediaRequest {
    let (sides, sources) = commit_sides(file, id, parent, short_id);
    MediaRequest {
        view,
        sides,
        sources,
        extension: extension(file.path()),
        difftool: parent.map(|parent| DifftoolTarget::Commit { parent, id }),
        path: OsString::from(file.path().as_ref()),
    }
}

fn copy_version(repo: &Repo, source: &VersionSource, version: Version) -> anyhow::Result<PathBuf> {
    let bytes = repo.file_version(source.revision, &source.path)?.bytes;
    let name = Path::new(&source.path)
        .file_name()
        .map_or_else(|| OsString::from("file"), ToOwned::to_owned);
    let folder = std::env::temp_dir()
        .join("sternegit")
        .join(format!("{}-{}", version.label().to_lowercase(), std::process::id()));
    std::fs::create_dir_all(&folder)?;
    let copy = folder.join(name);
    std::fs::write(&copy, bytes)?;
    Ok(copy)
}

fn read_sounds(repo: &Repo, sides: Sides, extension: Option<&str>) -> anyhow::Result<(Sounds, bool)> {
    let mut stored_with_lfs = false;
    let mut read = |source: Option<VersionSource>| -> anyhow::Result<Option<Sound>> {
        let Some(source) = source else {
            return Ok(None);
        };
        let version = repo.file_version(source.revision, &source.path)?;
        stored_with_lfs |= version.stored_with_lfs;
        Sound::decode(version.bytes, extension).map(Some)
    };
    let before = read(sides.before)?;
    let after = read(sides.after)?;
    Ok((Sounds::between(before, after), stored_with_lfs))
}

fn video_on_disk(
    repo: &Repo,
    source: &VersionSource,
    extension: Option<&str>,
) -> anyhow::Result<(PathBuf, Option<TemporaryCopy>, bool)> {
    if source.revision == Revision::WorkTree {
        return Ok((repo.location().join(&source.path), None, false));
    }
    let bytes = repo.file_bytes(source.revision, &source.path)?;
    if let Some(object) = repo.lfs_object(&bytes) {
        return Ok((object, None, true));
    }
    let bytes = repo.resolve_lfs(bytes)?.bytes;
    static COPIES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let number = COPIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let folder = std::env::temp_dir().join("sternegit");
    std::fs::create_dir_all(&folder)?;
    let copy = folder.join(format!(
        "video-{}-{number}.{}",
        std::process::id(),
        extension.unwrap_or("video")
    ));
    std::fs::write(&copy, bytes)?;
    Ok((copy.clone(), Some(TemporaryCopy(copy)), false))
}

fn read_videos(repo: &Repo, sides: Sides, extension: Option<&str>) -> anyhow::Result<(Videos, bool)> {
    let mut stored_with_lfs = false;
    let mut thumbs = Vec::new();
    let mut read = |source: Option<VersionSource>| -> anyhow::Result<Option<VideoSide>> {
        let Some(source) = source else {
            return Ok(None);
        };
        let (file, copy, lfs) = video_on_disk(repo, &source, extension)?;
        stored_with_lfs |= lfs;
        let facts = VideoFacts::probe(&file)?;
        let sound = soundtrack(&file)?.map(Arc::new);
        thumbs.push((thumbnails(&file)?, facts.duration));
        Ok(Some(VideoSide {
            file,
            facts,
            sound,
            _copy: copy,
        }))
    };
    let before = read(sides.before)?;
    let after = read(sides.after)?;
    let changes = match (before.is_some() && after.is_some(), thumbs.as_slice()) {
        (true, [(before, before_length), (after, after_length)]) => {
            visible_changes(before, after, (*before_length, *after_length))
        }
        _ => Vec::new(),
    };
    Ok((
        Videos {
            before,
            after,
            changes,
        },
        stored_with_lfs,
    ))
}

fn read_other(
    repo: &Repo,
    sides: Sides,
    extension: Option<&str>,
    note: Option<SharedString>,
) -> anyhow::Result<(OtherFile, bool)> {
    let mut stored_with_lfs = false;
    let mut facts = |source: Option<VersionSource>| -> anyhow::Result<Option<FileFacts>> {
        let Some(source) = source else {
            return Ok(None);
        };
        let size = repo.file_size(source.revision, &source.path)?;
        stored_with_lfs |= size.stored_with_lfs;
        Ok(Some(FileFacts {
            size: size.bytes,
            last_change: repo.last_change(source.revision, &source.path)?,
        }))
    };
    let before = facts(sides.before)?;
    let after = facts(sides.after)?;
    let type_name = extension.map_or_else(|| "these".to_owned(), str::to_uppercase);
    Ok((
        OtherFile {
            type_name: type_name.into(),
            before,
            after,
            diff_tool: repo.diff_tool(),
            note,
        },
        stored_with_lfs,
    ))
}

fn backdrop_swatch(backdrop: Backdrop, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let square = |light: bool| {
        div()
            .size(px(SWATCH_SQUARE))
            .bg(if light { gpui_kit::white() } else { gpui_kit::hsla(0., 0., 0.45, 1.) })
    };
    let fill = match backdrop {
        Backdrop::Checkerboard => None,
        Backdrop::Black => Some(gpui_kit::black()),
        Backdrop::White => Some(gpui_kit::white()),
        Backdrop::Panel => Some(theme.background),
    };
    let swatch = div()
        .size(px(SWATCH))
        .rounded(px(2.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.ring);
    match fill {
        Some(fill) => swatch.bg(fill).into_any_element(),
        None => swatch
            .child(
                h_flex()
                    .child(v_flex().child(square(true)).child(square(false)).child(square(true)))
                    .child(v_flex().child(square(false)).child(square(true)).child(square(false)))
                    .child(v_flex().child(square(true)).child(square(false)).child(square(true))),
            )
            .into_any_element(),
    }
}

fn exact_original(change: &FileChange) -> OsString {
    change
        .exact_paths()
        .nth(1)
        .unwrap_or(change.exact_path())
        .to_owned()
}

fn working_sides(change: &FileChange, side: Side, staged_too: bool) -> (Sides, [SharedString; 2]) {
    let new = || change.exact_path().to_owned();
    let deleted = change.kind == ChangeKind::Deleted;
    let added = matches!(change.kind, ChangeKind::Added | ChangeKind::Untracked);
    match side {
        Side::Unstaged => (
            Sides {
                before: (!added).then(|| VersionSource {
                    revision: Revision::Index,
                    path: new(),
                }),
                after: (!deleted).then(|| VersionSource {
                    revision: Revision::WorkTree,
                    path: new(),
                }),
            },
            [
                if staged_too { "Staged" } else { "HEAD" }.into(),
                "Working copy".into(),
            ],
        ),
        Side::Staged => (
            Sides {
                before: (!added).then(|| VersionSource {
                    revision: Revision::Head,
                    path: exact_original(change),
                }),
                after: (!deleted).then(|| VersionSource {
                    revision: Revision::Index,
                    path: new(),
                }),
            },
            ["HEAD".into(), "Staged".into()],
        ),
    }
}

fn commit_sides(
    file: &DiffFile,
    id: ObjectId,
    parent: Option<ObjectId>,
    short_id: &SharedString,
) -> (Sides, [SharedString; 2]) {
    let kind = change_kind(file);
    let path = OsString::from(file.path().as_ref());
    let original = renamed_from(file).map_or_else(|| path.clone(), |original| OsString::from(original.as_ref()));
    let before = parent
        .filter(|_| kind != ChangeKind::Added)
        .map(|parent| VersionSource {
            revision: Revision::Commit(parent),
            path: original,
        });
    let after = (kind != ChangeKind::Deleted).then(|| VersionSource {
        revision: Revision::Commit(id),
        path,
    });
    let parent_name = parent.map_or_else(SharedString::default, crate::git::short_id);
    (Sides { before, after }, [parent_name, short_id.clone()])
}

fn read_pictures(repo: &Repo, sides: Sides, extension: Option<&str>) -> anyhow::Result<(Pictures, bool)> {
    let mut stored_with_lfs = false;
    let mut read = |source: Option<VersionSource>| -> anyhow::Result<Option<Picture>> {
        let Some(source) = source else {
            return Ok(None);
        };
        let version = repo.file_version(source.revision, &source.path)?;
        stored_with_lfs |= version.stored_with_lfs;
        Picture::decode(&version.bytes, extension).map(Some)
    };
    let before = read(sides.before)?;
    let after = read(sides.after)?;
    Ok((Pictures::between(before, after), stored_with_lfs))
}

fn spreads(kind: ChangeKind) -> bool {
    matches!(
        kind,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Copied
    )
}

fn new_side(side: Side) -> Revision {
    match side {
        Side::Unstaged => Revision::WorkTree,
        Side::Staged => Revision::Index,
    }
}

fn spread_working(repo: &Repo, patch: FilePatch, change: &FileChange, side: Side) -> FilePatch {
    match patch {
        FilePatch::Diff { text, hunks } if spreads(change.kind) => FilePatch::Diff {
            text: repo.with_whole_file(text, new_side(side), change.exact_path()),
            hunks,
        },
        patch => patch,
    }
}

fn working_heading(open: &OpenFile, change: Option<FileChange>) -> Heading {
    let kind = change
        .as_ref()
        .map_or(ChangeKind::Modified, |change| change.kind);
    let state = if kind == ChangeKind::Conflicted {
        "conflicted"
    } else {
        open.side.labels().state
    };
    Heading {
        kind,
        path: open.path.clone(),
        original: change.and_then(|change| change.original_path),
        state: state.into(),
    }
}

fn conflict_request(change: &FileChange, current: bool, incoming: bool) -> MediaRequest {
    let side = |present: bool, version: Kept| {
        present.then(|| VersionSource {
            revision: Revision::Conflict(version),
            path: change.exact_path().to_owned(),
        })
    };
    MediaRequest {
        view: media_view(&change.path).unwrap_or(MediaView::Other),
        sides: Sides {
            before: side(current, Kept::Current),
            after: side(incoming, Kept::Incoming),
        },
        sources: ["Current".into(), "Incoming".into()],
        extension: extension(&change.path),
        difftool: None,
        path: change.exact_path().to_owned(),
    }
}

fn commit_heading(short_id: &SharedString, file: &DiffFile) -> Heading {
    Heading {
        kind: change_kind(file),
        path: file.path().clone(),
        original: renamed_from(file).cloned(),
        state: format!("in {short_id}").into(),
    }
}
