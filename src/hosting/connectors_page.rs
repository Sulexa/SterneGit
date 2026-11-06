use gpui_kit::{
    AnyElement, App, ClipboardItem, Context, Hsla, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use super::{
    parts::{Heading, boxed, callout, chip, crumbs, group, hot_row, mark, page, shown_path, slim_row},
    settings_tab::{Page, SettingsTab},
};
use crate::{
    app::connectors::{Connectors, approve},
    connect::{Connector, Grant, Manifest, Origin, Standing},
    net::services::ConnectorId,
    settings::Folder,
    ui::{Look as _, counted},
};

const CONNECTORS: &str = "Connectors";
const CONNECTORS_ABOUT: &str = "Connectors let SterneGit talk to hosting services. Each one runs in a sandbox: \
     it reaches only the servers listed for it, and SterneGit adds the token to its requests, so it never sees one.";
const OPEN_FOLDER: &str = "Open folder";
const BUNDLED: &str = "Bundled";
const INSTALLED: &str = "Installed";
const REVIEW: &str = "Review…";
const SHOW_LOG: &str = "Show log";
const APPROVE: &str = "Approve";
const NOT_NOW: &str = "Not now";
const COPY_LOG: &str = "Copy log";
const REACH: &str = "Servers it can reach";
const REACH_ABOUT: &str = "Requests anywhere else fail before they leave SterneGit.";
const WITH_TOKEN: &str = "With your token";
const WITHOUT_TOKEN: &str = "Without a token";
const IMAGES: &str = "Pictures from";
const REMOTES: &str = "Remotes it recognizes";
const REMOTES_ABOUT: &str = "Repositories with these remotes use its accounts.";
const SIGN_IN_PAGES: &str = "Sign-in pages";
const FEATURES: &str = "What it offers";
const SERVER_SLOT: &str = "server you add";
const HOST_SLOT: &str = "the server of each account you add";
const NO_LOG: &str = "The connector wrote nothing to its log.";

enum Piece {
    Literal(String),
    Slot(&'static str),
}

fn pieces(template: &str, manifest: Option<&Manifest>) -> Vec<Piece> {
    let literal_endpoint = |name: &str| {
        manifest?
            .services
            .iter()
            .flat_map(|service| &service.endpoints)
            .find(|(endpoint, template)| endpoint == name && !template.contains('{'))
            .map(|(_, template)| template.clone())
    };
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        let name = &rest[open + 1..open + close];
        if open > 0 {
            pieces.push(Piece::Literal(rest[..open].to_owned()));
        }
        pieces.push(match literal_endpoint(name) {
            Some(endpoint) => Piece::Literal(endpoint),
            None if name.ends_with(".host") => Piece::Slot(HOST_SLOT),
            None => Piece::Slot(SERVER_SLOT),
        });
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Literal(rest.to_owned()));
    }
    pieces
}

fn shown_template(template: &str, manifest: Option<&Manifest>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (muted, mono) = (theme.muted_foreground, theme.mono_font_family.clone());
    h_flex().flex_1().min_w_0().sm_text().children(pieces(template, manifest).into_iter().map(move |piece| match piece {
        Piece::Literal(text) => div().font_family(mono.clone()).font_semibold().child(text),
        Piece::Slot(text) => div().italic().text_color(muted).child(text),
    }))
}

fn feature_label(feature: &str) -> String {
    match feature {
        "repositories" => "Browse and clone repositories".to_owned(),
        "repositories.create" => "Create and publish repositories".to_owned(),
        "ssh_keys" => "Add SSH keys".to_owned(),
        "pulls" => "Pull requests: list and view".to_owned(),
        "pulls.create" => "Pull requests: create".to_owned(),
        "pulls.review" => "Pull requests: review".to_owned(),
        "pulls.resolve" => "Pull requests: resolve threads".to_owned(),
        "pulls.merge" => "Pull requests: merge".to_owned(),
        other => other.to_owned(),
    }
}

fn reaches(grant: &Grant) -> String {
    let mut hosts: Vec<String> = Vec::new();
    for template in &grant.endpoints {
        if template.contains('{') {
            continue;
        }
        if let Some(host) = url::Url::parse(template).ok().and_then(|url| url.host_str().map(str::to_owned)) {
            if !hosts.contains(&host) {
                hosts.push(host);
            }
        }
    }
    let typed = grant.endpoints.iter().any(|template| template.contains('{'));
    match (hosts.is_empty(), typed) {
        (false, true) => format!("Reaches {} and servers you add", hosts.join(", ")),
        (false, false) => format!("Reaches {}", hosts.join(", ")),
        (true, _) => "Reaches the servers you add".to_owned(),
    }
}

fn standing_look(standing: &Standing, cx: &App) -> (&'static str, Hsla) {
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

fn origin_label(connector: &Connector) -> &'static str {
    match connector.origin {
        Origin::Bundled => BUNDLED,
        Origin::Installed => INSTALLED,
    }
}

fn shown_grant(connector: &Connector) -> Option<Grant> {
    match &connector.standing {
        Standing::Waiting(grant) | Standing::Modified(grant) => Some(grant.clone()),
        _ => connector.manifest.as_deref().map(Grant::asked_by),
    }
}

struct Summary {
    id: ConnectorId,
    name: String,
    version: Option<String>,
    origin: &'static str,
    standing: Standing,
    reach: Option<String>,
    accounts: usize,
}

fn summarized(connector: &Connector, accounts: usize) -> Summary {
    Summary {
        id: connector.id.clone(),
        name: connector
            .manifest
            .as_ref()
            .map_or_else(|| connector.id.0.clone(), |manifest| manifest.name.clone()),
        version: connector.manifest.as_ref().map(|manifest| manifest.version.clone()),
        origin: origin_label(connector),
        standing: connector.standing.clone(),
        reach: shown_grant(connector).map(|grant| reaches(&grant)),
        accounts,
    }
}

fn installed_folder() -> Option<std::path::PathBuf> {
    Folder::Data.path().map(|folder| folder.join("connectors"))
}

impl SettingsTab {
    pub(super) fn connectors_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let summaries: Vec<Summary> = {
            let connectors = Connectors::of(cx);
            let accounts = connectors.accounts();
            connectors
                .connectors()
                .iter()
                .map(|connector| {
                    let count = accounts.iter().filter(|account| account.connector == connector.id.0).count();
                    summarized(connector, count)
                })
                .collect()
        };
        let muted = cx.theme().muted_foreground;
        let heading = Heading::new(CONNECTORS).about(CONNECTORS_ABOUT).action(
            Button::new("open-connectors-folder")
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
            .map(|(ix, summary)| self.connector_row(ix, summary, cx))
            .collect::<Vec<_>>();
        let folder = installed_folder().map_or_else(String::new, |folder| shown_path(&folder));
        page(
            v_flex()
                .gap_6()
                .child(heading.render(cx))
                .when(!rows.is_empty(), |this| this.child(boxed(rows, cx)))
                .child(
                    h_flex()
                        .gap_1p5()
                        .xs_text()
                        .text_color(muted)
                        .child(Icon::new(IconName::Info).xsmall())
                        .child(format!("To install a connector, copy its folder into {folder} and restart SterneGit.")),
                ),
        )
        .into_any_element()
    }

    fn connector_row(&self, ix: usize, summary: &Summary, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, red) = (theme.muted_foreground, theme.red);
        let (standing, color) = standing_look(&summary.standing, cx);
        let accounts = summary.accounts;
        let name = summary.name.clone();
        let version = summary.version.clone();
        let second = match &summary.standing {
            Standing::Unusable(problem) => div().text_color(red).child(problem.clone()),
            Standing::Failed(stop) => div().text_color(red).child(format!("Stopped while starting: {stop}")),
            _ => {
                let mut text = summary.reach.clone().unwrap_or_default();
                if accounts > 0 {
                    text.push_str(&format!(" · {}", counted(accounts, "account", "accounts")));
                }
                div().text_color(muted).child(text)
            }
        };
        let id = summary.id.clone();
        let action = match &summary.standing {
            Standing::Waiting(_) | Standing::Modified(_) => Button::new(("review-connector", ix))
                .primary()
                .small()
                .label(REVIEW)
                .into_any_element(),
            Standing::Failed(_) => Button::new(("connector-log", ix)).small().label(SHOW_LOG).into_any_element(),
            _ => Icon::new(IconName::ChevronRight).small().text_color(muted).into_any_element(),
        };
        hot_row(("connector-row", ix), cx)
            .cursor_pointer()
            .child(mark(IconName::Plug, false, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .sm_text()
                            .child(div().font_semibold().child(name))
                            .children(version.map(|version| div().text_color(muted).child(version)))
                            .child(chip(summary.origin, None, cx))
                            .child(chip(standing, Some(color), cx)),
                    )
                    .child(second.xs_text().ellipsis()),
            )
            .child(action)
            .on_click(cx.listener(move |this, _, _, cx| this.show(Page::Connector(id.clone()), cx)))
            .into_any_element()
    }

    pub(super) fn connector_page(&mut self, id: &ConnectorId, cx: &mut Context<Self>) -> AnyElement {
        let connectors = Connectors::of(cx);
        let Some(connector) = connectors.connector(id) else {
            return div().into_any_element();
        };
        let was = connectors.approved_version(id);
        let standing_now = connector.standing.clone();
        let origin = origin_label(connector);
        let folder = shown_path(&connector.folder);
        let grant = shown_grant(connector);
        let manifest = connector.manifest.clone();
        let theme = cx.theme();
        let (muted, yellow, red) = (theme.muted_foreground, theme.yellow, theme.red);
        let name = manifest.as_ref().map_or_else(|| id.0.clone(), |manifest| manifest.name.clone());
        let version = manifest.as_ref().map_or_else(String::new, |manifest| manifest.version.clone());
        let awaiting = standing_now.awaits_approval();
        let title = match (&standing_now, &was) {
            (Standing::Waiting(_), Some(was)) if *was != version => {
                format!("{name} {version} asks for more than {was} did")
            }
            (Standing::Modified(_), _) => format!("{name}'s files changed: approve {name} {version}?"),
            (Standing::Waiting(_), _) => format!("Approve {name} {version}?"),
            _ => format!("{name} {version}"),
        };
        let (standing, color) = standing_look(&standing_now, cx);
        let back = cx.entity().downgrade();
        let heading = Heading::new(title).media(mark(IconName::Plug, true, cx)).subtitle(
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
                .child(crumbs(CONNECTORS, name.clone(), move |_, cx| {
                    let _ = back.update(cx, |this, cx| this.show(Page::Connectors, cx));
                }, cx))
                .child(heading.render(cx)),
        );
        if awaiting {
            body = body.child(callout(
                IconName::ShieldCheck,
                format!(
                    "Approve only a connector you trust. Once approved, it acts on your {name} accounts as far as their tokens allow, \
                     through SterneGit. It can't read your files, your SSH keys or your other accounts."
                ),
                yellow,
            ));
        }
        match &standing_now {
            Standing::Unusable(problem) => {
                body = body.child(div().sm_text().text_color(red).child(problem.clone()));
            }
            Standing::Failed(stop) => {
                let log = Connectors::of(cx).log_lines(id);
                let copied = log.join("\n");
                let mono = cx.theme().mono_font_family.clone();
                body = body.child(group(
                    "Why it stopped",
                    Some(stop.to_string().into()),
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
                                Button::new("copy-connector-log")
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
            body = body.child(self.grant_list(&grant, manifest.as_deref(), cx));
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
                    .child(
                        Button::new("connector-not-now")
                            .label(NOT_NOW)
                            .on_click(cx.listener(|this, _, _, cx| this.show(Page::Connectors, cx))),
                    )
                    .child(
                        Button::new("connector-approve")
                            .primary()
                            .icon(IconName::ShieldCheck)
                            .label(APPROVE)
                            .on_click(move |_, _, cx| {
                                let result = approve(cx, &approved);
                                let _ = back.update(cx, |this, cx| match result {
                                    Ok(()) => this.show(Page::Connectors, cx),
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

    fn grant_list(&self, grant: &Grant, manifest: Option<&Manifest>, cx: &App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let carries_token = |template: &str| {
            manifest.is_some_and(|manifest| {
                manifest.services.iter().any(|service| {
                    service
                        .endpoints
                        .iter()
                        .any(|(name, known)| known == template && service.token_endpoints.contains(name))
                })
            })
        };
        let line = |icon: IconName, template: &str, side: Option<&'static str>| {
            slim_row()
                .child(Icon::new(icon).small().text_color(muted))
                .child(shown_template(template, manifest, cx))
                .children(side.map(|side| div().flex_none().xs_text().text_color(muted).child(side)))
                .into_any_element()
        };
        let mut groups = v_flex().gap_6();
        let mut add = |title: &'static str, about: Option<&'static str>, rows: Vec<AnyElement>| {
            if !rows.is_empty() {
                groups = std::mem::replace(&mut groups, v_flex()).child(group(title, about.map(SharedString::from), rows, cx));
            }
        };
        add(
            REACH,
            Some(REACH_ABOUT),
            grant
                .endpoints
                .iter()
                .map(|template| {
                    let side = if carries_token(template) { WITH_TOKEN } else { WITHOUT_TOKEN };
                    line(IconName::Globe, template, Some(side))
                })
                .collect(),
        );
        add(IMAGES, None, grant.images.iter().map(|template| line(IconName::Image, template, Some(WITHOUT_TOKEN))).collect());
        add(REMOTES, Some(REMOTES_ABOUT), grant.remote_hosts.iter().map(|host| line(IconName::Server, host, None)).collect());
        add(SIGN_IN_PAGES, None, grant.sign_in.iter().map(|template| line(IconName::LogIn, template, None)).collect());
        add(
            FEATURES,
            None,
            grant
                .features
                .iter()
                .map(|feature| {
                    slim_row()
                        .child(Icon::new(IconName::Check).small().text_color(muted))
                        .child(div().sm_text().child(feature_label(feature)))
                        .into_any_element()
                })
                .collect(),
        );
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::{Piece, pieces, reaches};
    use crate::connect::{Grant, Manifest, manifest_samples};

    fn shown(template: &str, manifest: Option<&Manifest>) -> String {
        pieces(template, manifest)
            .into_iter()
            .map(|piece| match piece {
                Piece::Literal(text) => text,
                Piece::Slot(text) => format!("<{text}>"),
            })
            .collect()
    }

    #[test]
    fn typed_servers_show_as_slots_and_literal_endpoints_are_spelled_out() {
        let github = Manifest::parse(manifest_samples::github().to_string().as_bytes()).unwrap();
        assert_eq!(shown("{server}/api/v3/", Some(&github)), "<server you add>/api/v3/");
        assert_eq!(shown("{server.host}", None), "<the server of each account you add>");
        assert_eq!(shown("https://api.github.com/", Some(&github)), "https://api.github.com/");
        assert!(shown("{web}settings/tokens/new", Some(&github)).starts_with("https://github.com/settings/tokens/new"));
    }

    #[test]
    fn the_reach_summary_lists_fixed_hosts_then_typed_servers() {
        let github = Manifest::parse(manifest_samples::github().to_string().as_bytes()).unwrap();
        assert_eq!(reaches(&Grant::asked_by(&github)), "Reaches api.github.com, github.com and servers you add");
        assert_eq!(reaches(&Grant::default()), "Reaches the servers you add");
    }
}
