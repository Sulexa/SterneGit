use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, PathPromptOptions,
    Render, SharedString, Styled as _, Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};

use super::{
    accounts_page::service_label,
    hosted::{Hosted, connect_origin, creating_accounts, remember_use},
    parts::{Note, field},
};
use crate::{
    git::{init, is_empty_or_missing},
    ui::Look as _,
};

const WIDTH: f32 = 520.;
const TITLE: &str = "New repository";
const ABOUT: &str = "FastGit makes the folder, runs git init in it and opens it in a new tab.";
const NAME: &str = "Name";
const FOLDER: &str = "Folder";
const BROWSE: &str = "Browse…";
const ALSO_ON: &str = "Also create it on";
const OWNER: &str = "Owner";
const VISIBILITY: &str = "Visibility";
const DESCRIPTION: &str = "Description";
const CANCEL: &str = "Cancel";
const CREATE: &str = "Create";
const NOT_EMPTY: &str = "This folder already holds files: choose another name or folder.";
const BAD_NAME: &str = "A name can't hold / \\ : * ? \" < > or |.";

type OnCreated = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;

struct NewRepositoryForm {
    name: Entity<InputState>,
    parent: Entity<InputState>,
    hosting: bool,
    hosted: Entity<Hosted>,
    problem: Option<SharedString>,
    busy: Option<Task<()>>,
    on_created: OnCreated,
    _subscriptions: Vec<Subscription>,
}

impl NewRepositoryForm {
    fn folder(&self, cx: &App) -> Option<PathBuf> {
        let name = self.name.read(cx).value().trim().to_owned();
        let parent = self.parent.read(cx).value().trim().to_owned();
        (!name.is_empty() && !parent.is_empty()).then(|| Path::new(&parent).join(name))
    }

    fn local_problem(&self, cx: &App) -> Option<&'static str> {
        let name = self.name.read(cx).value();
        if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Some(BAD_NAME);
        }
        self.folder(cx).filter(|folder| !is_empty_or_missing(folder)).map(|_| NOT_EMPTY)
    }

    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Create it in".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(chosen) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.parent.update(cx, |state, cx| state.set_value(chosen.display().to_string(), window, cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = self.folder(cx) else {
            return;
        };
        if self.local_problem(cx).is_some() || self.busy.is_some() {
            return;
        }
        let name = self.name.read(cx).value().trim().to_owned();
        let creation = match self.hosting {
            true => match self.hosted.read(cx).creation(&name, cx) {
                Some(creation) => Some(creation),
                None => {
                    self.problem = Some("Choose an account to create it on.".into());
                    cx.notify();
                    return;
                }
            },
            false => None,
        };
        let account = creation.as_ref().map(|creation| (creation.account().clone(), creation.owner().to_owned()));
        let made = folder.clone();
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let created = match creation {
                        Some(creation) => Some(creation.create().await?),
                        None => None,
                    };
                    init(&made).map_err(|error| SharedString::from(format!("{error:#}")))?;
                    if let Some(created) = &created {
                        connect_origin(&made, created).map_err(|error| SharedString::from(format!("{error:#}")))?;
                    }
                    Ok::<_, SharedString>(created.is_some_and(|created| created.ssh_command.is_some()))
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match result {
                    Ok(over_ssh) => {
                        if let Some((account, owner)) = &account {
                            remember_use(account, owner, &folder, over_ssh, cx);
                        }
                        window.close_dialog(cx);
                        (this.on_created)(folder.clone(), window, cx);
                    }
                    Err(problem) => {
                        this.problem = Some(problem);
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }
}

impl Render for NewRepositoryForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let has_accounts = !creating_accounts(cx).is_empty();
        let creates = self
            .folder(cx)
            .map(|folder| Note::Plain(format!("Creates {}", folder.display()).into()));
        let local_problem = self.local_problem(cx);
        let folder_note = local_problem.map(|problem| Note::Problem(problem.into())).or(creates);
        let hosting = self.hosting;
        let hosted = self.hosted.clone();
        let picker = hosted.update(cx, |hosted, cx| hosted.account_picker("new-repository-account", !hosting, cx));
        let mut form = v_flex()
            .gap_4()
            .child(div().sm_text().text_color(muted).child(ABOUT))
            .child(field(NAME, Input::new(&self.name), None, cx))
            .child(field(
                FOLDER,
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.parent)))
                    .child(Button::new("new-repository-browse").label(BROWSE).on_click(cx.listener(|this, _, window, cx| this.browse(window, cx)))),
                folder_note,
                cx,
            ));
        if has_accounts {
            form = form.child(
                h_flex()
                    .gap_2p5()
                    .child(
                        Checkbox::new("new-repository-hosted")
                            .label(ALSO_ON)
                            .checked(hosting)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.hosting = *checked;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1().child(picker)),
            );
        }
        if hosting && has_accounts {
            let (owner, visibility, service) = hosted.update(cx, |hosted, cx| {
                let service = hosted
                    .chosen_account(cx)
                    .map_or_else(String::new, |account| service_label(&account, cx));
                (hosted.owner_picker(cx), hosted.visibility_choice(cx), service)
            });
            let description = hosted.read(cx).description_input();
            form = form
                .child(field(OWNER, owner, None, cx))
                .child(field(VISIBILITY, visibility, None, cx))
                .child(field(
                    DESCRIPTION,
                    description,
                    Some(Note::Plain(
                        format!("FastGit creates it empty on {service} and adds it as origin. Your first push sends your branch there.").into(),
                    )),
                    cx,
                ));
        }
        let busy = self.busy.is_some();
        let ready = self.folder(cx).is_some() && local_problem.is_none();
        form.children(self.problem.clone().map(|problem| div().xs_text().text_color(cx.theme().red).child(problem)))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("new-repository-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("new-repository-create")
                            .primary()
                            .icon(IconName::FolderPlus)
                            .label(CREATE)
                            .loading(busy)
                            .disabled(!ready || busy)
                            .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
                    ),
            )
    }
}

pub(crate) fn open_new_repository(
    parent: PathBuf,
    window: &mut Window,
    cx: &mut App,
    on_created: impl Fn(PathBuf, &mut Window, &mut App) + 'static,
) {
    let on_created: OnCreated = Rc::new(on_created);
    let form = cx.new(|cx| {
        let name = cx.new(|cx| InputState::new(window, cx));
        let parent = cx.new(|cx| InputState::new(window, cx).default_value(parent.display().to_string()));
        let hosted = cx.new(|cx| Hosted::new(window, cx));
        let subscriptions = vec![
            cx.subscribe(&name, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
            cx.subscribe(&parent, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
            cx.observe(&hosted, |_, _, cx| cx.notify()),
        ];
        NewRepositoryForm {
            name,
            parent,
            hosting: false,
            hosted,
            problem: None,
            busy: None,
            on_created,
            _subscriptions: subscriptions,
        }
    });
    let name = form.read(cx).name.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        let created = form.clone();
        dialog.w(px(WIDTH)).title(TITLE).child(form.clone()).on_ok(move |_, window, cx| {
            created.update(cx, |form, cx| form.create(window, cx));
            false
        })
    });
    window.defer(cx, move |window, cx| name.update(cx, |state, cx| state.focus(window, cx)));
}
