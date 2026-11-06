use std::path::PathBuf;

use gpui_kit::{
    AppContext as _, Context, EventEmitter, IntoElement, ParentElement as _, Render, SharedString, Styled as _,
    Task, Window,
    component::{
        ActiveTheme as _, StyledExt as _,
        button::Button,
        spinner::Spinner,
        v_flex,
    },
    div, px,
};

use crate::{
    accounts::AccountId,
    app::connectors::change_account,
    git::{self, CloneRequest, Progress, Stop},
    ui::{Look as _, thousands},
};

const CANCEL: &str = "Cancel";
const CLOSE: &str = "Close tab";

pub(crate) struct CloneJob {
    pub(crate) request: CloneRequest,
    pub(crate) shown: String,
    pub(crate) uses_key_of: Option<AccountId>,
}

pub(crate) enum CloneEvent {
    Cloned(PathBuf),
    Abandoned,
}

pub(crate) struct CloneView {
    shown: String,
    folder: PathBuf,
    progress: Option<Progress>,
    failure: Option<SharedString>,
    stop: Stop,
    _run: Option<Task<()>>,
}

impl EventEmitter<CloneEvent> for CloneView {}

impl CloneView {
    pub(crate) fn new(job: CloneJob, cx: &mut Context<Self>) -> Self {
        let stop = Stop::default();
        let (sender, receiver) = smol::channel::unbounded::<Progress>();
        let request = job.request.clone();
        let folder = request.folder.clone();
        let stopping = stop.clone();
        let owner = job.uses_key_of.clone();
        let run = cx.spawn(async move |this, cx| {
            let cloning = cx.background_spawn(async move {
                git::clone(&request, move |step| drop(sender.try_send(step)), &stopping)
            });
            while let Ok(step) = receiver.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.progress = Some(step);
                    cx.notify();
                });
            }
            let result = cloning.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => {
                    if let Some(owner) = &owner {
                        let folder = this.folder.clone();
                        let _ = change_account(cx, owner, |account| {
                            if !account.ssh_repositories.contains(&folder) {
                                account.ssh_repositories.push(folder);
                            }
                        });
                    }
                    cx.emit(CloneEvent::Cloned(this.folder.clone()));
                }
                Err(error) => {
                    this.failure = Some(format!("{error:#}").into());
                    cx.notify();
                }
            });
        });
        Self {
            shown: job.shown,
            folder,
            progress: None,
            failure: None,
            stop,
            _run: Some(run),
        }
    }

    pub(crate) fn name(&self) -> SharedString {
        git::folder_name(&self.folder)
    }

    pub(crate) fn is_running(&self) -> bool {
        self.failure.is_none()
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.stop.now();
        cx.emit(CloneEvent::Abandoned);
    }
}

impl Drop for CloneView {
    fn drop(&mut self) {
        self.stop.now();
    }
}

impl Render for CloneView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, red) = (theme.muted_foreground, theme.red);
        let mono = theme.mono_font_family.clone();
        let into = self.folder.display().to_string();
        let progress = self.progress.as_ref().map(|step| {
            let counts = match (step.done, step.total) {
                (Some(done), Some(total)) => format!(" ({} of {})", thousands(done as usize), thousands(total as usize)),
                _ => String::new(),
            };
            let percent = step.percent.map_or_else(String::new, |percent| format!(": {percent}%"));
            format!("{}{percent}{counts}", step.phase)
        });
        let body = match &self.failure {
            None => v_flex()
                .items_center()
                .gap_3()
                .child(Spinner::new())
                .child(div().sm_text().child(format!("Cloning {} into {into}…", self.shown)))
                .children(progress.map(|line| div().xs_text().font_family(mono).text_color(muted).child(line)))
                .child(
                    Button::new("cancel-clone")
                        .label(CANCEL)
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                ),
            Some(failure) => v_flex()
                .items_center()
                .gap_3()
                .max_w(px(640.))
                .child(div().font_medium().child(format!("Couldn't clone {}", self.shown)))
                .child(div().sm_text().text_center().text_color(red).child(failure.clone()))
                .child(
                    Button::new("close-failed-clone")
                        .label(CLOSE)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(CloneEvent::Abandoned))),
                ),
        };
        v_flex().size_full().items_center().justify_center().p_6().child(body)
    }
}
