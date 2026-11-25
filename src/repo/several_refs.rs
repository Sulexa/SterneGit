use std::rc::Rc;

use gix::ObjectId;
use gpui_kit::{
    App, Entity, SharedString, Window,
    component::{
        WindowExt as _,
        button::ButtonVariant,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use super::{
    branch_menu::what_deleting_loses, navigation::Target, operation::Outcome, repository::Repository,
};
use crate::{
    git::{History, Pushed, RefKind, RefLabel, Repo, Stash, TagPushed},
    ui::{Handler, Picked, counted, handler, listed},
};

const DELETE: &str = "Delete";

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum RefKey {
    Ref(RefKind, SharedString),
    Stash(ObjectId),
}

impl RefKey {
    pub(crate) fn of(label: &RefLabel) -> Self {
        RefKey::Ref(label.kind, label.name.clone())
    }

    pub(crate) fn same_section(&self, other: &RefKey) -> bool {
        match (self, other) {
            (RefKey::Ref(kind, _), RefKey::Ref(other, _)) => kind == other,
            (RefKey::Stash(_), RefKey::Stash(_)) => true,
            _ => false,
        }
    }

    pub(crate) fn exists_in(&self, history: &History) -> bool {
        match self {
            RefKey::Ref(..) => history.refs.iter().any(|label| RefKey::of(label) == *self),
            RefKey::Stash(id) => history.stashes.iter().any(|stash| stash.id == *id),
        }
    }

    fn target_in(&self, history: &History) -> Option<ObjectId> {
        match self {
            RefKey::Ref(..) => history
                .refs
                .iter()
                .find(|label| RefKey::of(label) == *self)
                .map(|label| label.target),
            RefKey::Stash(id) => Some(*id),
        }
    }
}

pub(crate) fn graph_pick(keys: &Picked<RefKey>, clicked: &RefKey, history: &History) -> Option<Target> {
    let ids: Vec<ObjectId> = history
        .refs
        .iter()
        .filter(|label| keys.contains(&RefKey::of(label)))
        .map(|label| label.target)
        .filter(|id| history.row_of(id).is_some())
        .collect();
    let focus = clicked
        .target_in(history)
        .filter(|id| ids.contains(id))
        .or_else(|| ids.first().copied())?;
    let picked = Picked::with(ids, focus);
    Some(match picked.len() {
        1 => Target::at_commit(focus),
        _ => Target::Commits {
            picked: Rc::new(picked),
            open: None,
        },
    })
}

#[derive(Clone, Default)]
pub(crate) struct RefPick {
    pub(crate) picked: Rc<Picked<RefKey>>,
    made_for: Vec<ObjectId>,
}

impl RefPick {
    pub(crate) fn new(picked: Picked<RefKey>, made_for: &Picked<ObjectId>) -> Self {
        Self {
            picked: Rc::new(picked),
            made_for: sorted(made_for),
        }
    }

    pub(crate) fn current(
        &self,
        shown: &Picked<ObjectId>,
        exists: impl Fn(&RefKey) -> bool,
    ) -> Option<Rc<Picked<RefKey>>> {
        if self.made_for != sorted(shown) {
            return None;
        }
        let mut kept = (*self.picked).clone();
        kept.retain(exists);
        (kept.len() > 0).then(|| Rc::new(kept))
    }
}

fn sorted(ids: &Picked<ObjectId>) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = ids.keys().copied().collect();
    ids.sort();
    ids
}

#[derive(Clone, Debug)]
pub(crate) enum PickedRefs {
    Refs(Vec<RefLabel>),
    Stashes(Vec<Stash>),
}

impl PickedRefs {
    pub(crate) fn of(keys: &Picked<RefKey>, history: &History) -> Option<Self> {
        let refs: Vec<RefLabel> = history
            .refs
            .iter()
            .filter(|label| keys.contains(&RefKey::of(label)))
            .cloned()
            .collect();
        let stashes: Vec<Stash> = history
            .stashes
            .iter()
            .filter(|stash| keys.contains(&RefKey::Stash(stash.id)))
            .cloned()
            .collect();
        match (refs.len(), stashes.len()) {
            (0, 0) => None,
            (0, _) => Some(PickedRefs::Stashes(stashes)),
            _ => Some(PickedRefs::Refs(refs)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefsAction {
    Push,
    Delete,
    DeleteThere,
}

#[derive(Clone)]
pub(crate) struct PickedRefsMenu {
    remote: Option<SharedString>,
    writing: bool,
    on_action: Option<Handler<(PickedRefs, RefsAction)>>,
}

impl PickedRefsMenu {
    pub(crate) fn new(remote: Option<SharedString>, writing: bool) -> Self {
        Self {
            remote,
            writing,
            on_action: None,
        }
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(PickedRefs, RefsAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(&self, menu: PopupMenu, picked: &PickedRefs) -> PopupMenu {
        let mut menu = menu;
        for (ix, group) in self.offered(picked).into_iter().enumerate() {
            if ix > 0 {
                menu = menu.separator();
            }
            for (action, text) in group {
                menu = menu.item(self.item(action, text, picked));
            }
        }
        menu
    }

    fn offered(&self, picked: &PickedRefs) -> Vec<Vec<(RefsAction, String)>> {
        let remote = self.remote.as_deref();
        let groups = match picked {
            PickedRefs::Stashes(stashes) => vec![vec![(RefsAction::Delete, format!("Delete {} stashes…", stashes.len()))]],
            PickedRefs::Refs(labels) => match labels[0].kind {
                RefKind::LocalBranch => {
                    let deletable = deletable(labels).len();
                    vec![
                        vec![(RefsAction::Push, format!("Push {} branches", labels.len()))],
                        (deletable > 0)
                            .then(|| (RefsAction::Delete, format!("Delete {deletable} branches…")))
                            .into_iter()
                            .collect(),
                    ]
                }
                RefKind::RemoteBranch => vec![vec![(
                    RefsAction::Delete,
                    format!("Delete {} branches from {}…", labels.len(), remotes_of(labels)),
                )]],
                RefKind::Tag => {
                    let n = labels.len();
                    let pushed = remote.map(|remote| (RefsAction::Push, format!("Push {n} tags to {remote}")));
                    let there = remote.map(|remote| (RefsAction::DeleteThere, format!("Delete {n} tags from {remote}…")));
                    vec![
                        pushed.into_iter().collect(),
                        [Some((RefsAction::Delete, format!("Delete {n} tags…"))), there]
                            .into_iter()
                            .flatten()
                            .collect(),
                    ]
                }
            },
        };
        groups.into_iter().filter(|group| !group.is_empty()).collect()
    }

    fn item(&self, action: RefsAction, text: String, picked: &PickedRefs) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (picked.clone(), action);
        PopupMenuItem::new(text)
            .disabled(self.writing || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

fn deletable(labels: &[RefLabel]) -> Vec<RefLabel> {
    labels
        .iter()
        .filter(|label| !label.is_head && label.worktree.is_none())
        .cloned()
        .collect()
}

fn remotes_of(labels: &[RefLabel]) -> String {
    let mut remotes = by_remote(labels).into_iter().map(|(remote, _)| remote);
    match (remotes.next(), remotes.next()) {
        (Some(only), None) => only,
        _ => "their remotes".to_owned(),
    }
}

fn by_remote(labels: &[RefLabel]) -> Vec<(String, Vec<String>)> {
    let mut grouped: Vec<(String, Vec<String>)> = Vec::new();
    for remote in labels.iter().filter_map(RefLabel::on_remote) {
        match grouped.iter_mut().find(|(name, _)| name == remote.remote) {
            Some((_, branches)) => branches.push(remote.branch.to_owned()),
            None => grouped.push((remote.remote.to_owned(), vec![remote.branch.to_owned()])),
        }
    }
    grouped
}

fn names(labels: &[RefLabel]) -> Vec<&str> {
    labels.iter().map(|label| label.name.as_str()).collect()
}

#[derive(Clone, Debug)]
pub(crate) enum RefsWrite {
    PushBranches(Vec<RefLabel>),
    DeleteBranches(Vec<RefLabel>),
    PushTags { tags: Vec<RefLabel>, remote: String },
    DeleteTags(Vec<RefLabel>),
    DeleteTagsThere { tags: Vec<RefLabel>, remote: String },
    DeleteStashes(Vec<Stash>),
}

impl RefsWrite {
    pub(crate) fn pushes(&self) -> bool {
        matches!(self, RefsWrite::PushBranches(_) | RefsWrite::PushTags { .. })
    }

    pub(crate) fn failed(&self) -> SharedString {
        match self {
            RefsWrite::PushBranches(_) => "Couldn't push the branches".into(),
            RefsWrite::DeleteBranches(_) => "Couldn't delete the branches".into(),
            RefsWrite::PushTags { remote, .. } => format!("Couldn't push the tags to {remote}").into(),
            RefsWrite::DeleteTags(_) => "Couldn't delete the tags".into(),
            RefsWrite::DeleteTagsThere { remote, .. } => format!("Couldn't delete the tags from {remote}").into(),
            RefsWrite::DeleteStashes(_) => "Couldn't delete the stashes".into(),
        }
    }

    pub(crate) fn run(self, repo: &Repo) -> anyhow::Result<Outcome> {
        match self {
            RefsWrite::PushBranches(labels) => {
                let mut left_out = Vec::new();
                for label in &labels {
                    match repo.push_branch(&label.name)? {
                        Pushed::Behind(_) | Pushed::UpstreamRewritten(_) => left_out.push(label.name.to_string()),
                        Pushed::Sent(_) | Pushed::AlreadyThere(_) | Pushed::Forced(_) => {}
                    }
                }
                Ok(branches_pushed(labels.len(), &left_out))
            }
            RefsWrite::DeleteBranches(labels) => {
                let local: Vec<&str> = labels
                    .iter()
                    .filter(|label| label.kind == RefKind::LocalBranch)
                    .map(|label| label.name.as_str())
                    .collect();
                if !local.is_empty() {
                    repo.delete_branches(&local)?;
                }
                let remotes = by_remote(&labels);
                for (remote, branches) in &remotes {
                    let branches: Vec<&str> = branches.iter().map(String::as_str).collect();
                    repo.delete_remote_branches(remote, &branches)?;
                }
                Ok(Outcome::Done(match local.is_empty() {
                    true => format!("Deleted {} branches from {}", labels.len(), remotes_of(&labels)).into(),
                    false => format!("Deleted {} branches", labels.len()).into(),
                }))
            }
            RefsWrite::PushTags { tags, remote } => {
                let pushed = repo.push_tags(&remote, &names(&tags))?;
                let already = tags
                    .iter()
                    .zip(&pushed)
                    .filter(|(_, pushed)| **pushed == TagPushed::AlreadyThere)
                    .map(|(tag, _)| tag.name.to_string())
                    .collect::<Vec<_>>();
                Ok(Outcome::Done(tags_pushed(tags.len(), &already, &remote).into()))
            }
            RefsWrite::DeleteTags(tags) => {
                repo.delete_tags(&names(&tags))?;
                Ok(Outcome::Done(format!("Deleted {} tags", tags.len()).into()))
            }
            RefsWrite::DeleteTagsThere { tags, remote } => {
                repo.delete_remote_tags(&remote, &names(&tags))?;
                Ok(Outcome::Done(format!("Deleted {} tags from {remote}", tags.len()).into()))
            }
            RefsWrite::DeleteStashes(stashes) => {
                for stash in &stashes {
                    repo.drop_stash(stash.id)?;
                }
                Ok(Outcome::Done(format!("Deleted {} stashes", stashes.len()).into()))
            }
        }
    }
}

fn branches_pushed(n: usize, left_out: &[String]) -> Outcome {
    let pushed = n - left_out.len();
    if left_out.is_empty() {
        return Outcome::Done(format!("Pushed {n} branches").into());
    }
    let behind = match left_out.len() {
        1 => "is behind its upstream, so it was left out. Pull it, then push it again.",
        _ => "are behind their upstreams, so they were left out. Pull them, then push them again.",
    };
    Outcome::Warning {
        title: format!("Pushed {}", counted(pushed, "branch", "branches")).into(),
        message: format!("{} {behind}", listed(left_out)).into(),
    }
}

fn tags_pushed(n: usize, already: &[String], remote: &str) -> String {
    match already.len() {
        0 => format!("Pushed {n} tags to {remote}"),
        all if all == n => format!("{remote} already has these {n} tags"),
        _ => format!(
            "Pushed {} to {remote}; it already had {}",
            counted(n - already.len(), "tag", "tags"),
            listed(already)
        ),
    }
}

struct Question {
    title: String,
    description: String,
    write: RefsWrite,
}

fn question(picked: PickedRefs, action: RefsAction, history: &History, remote: Option<&str>) -> Option<Question> {
    match (picked, action) {
        (PickedRefs::Stashes(stashes), RefsAction::Delete) => Some(Question {
            title: format!("Delete {} stashes?", stashes.len()),
            description: "Git keeps no other copy of their changes, so they are lost.".to_owned(),
            write: RefsWrite::DeleteStashes(stashes),
        }),
        (PickedRefs::Refs(labels), RefsAction::Delete) if labels[0].kind == RefKind::LocalBranch => {
            let gone = deletable(&labels);
            let kept: Vec<&str> = labels
                .iter()
                .filter(|label| label.is_head || label.worktree.is_some())
                .map(|label| label.name.as_str())
                .collect();
            let lost = what_deleting_several_loses(&names(&gone), history.commits_only_on_these(&gone));
            let stays = match kept.len() {
                0 => None,
                1 => Some(format!("{} stays, since it is checked out.", kept[0])),
                _ => Some(format!("{} stay, since they are checked out.", listed(&kept))),
            };
            (!gone.is_empty()).then(|| Question {
                title: format!("Delete {} branches?", gone.len()),
                description: [Some(lost), stays].into_iter().flatten().collect::<Vec<_>>().join(" "),
                write: RefsWrite::DeleteBranches(gone),
            })
        }
        (PickedRefs::Refs(labels), RefsAction::Delete) if labels[0].kind == RefKind::RemoteBranch => {
            let remotes = remotes_of(&labels);
            Some(Question {
                title: format!("Delete {} branches from {remotes}?", labels.len()),
                description: format!(
                    "The branches are removed from {remotes} for everyone who uses {}. \
                     Local branches with the same names stay.",
                    if remotes == "their remotes" { "them" } else { "it" }
                ),
                write: RefsWrite::DeleteBranches(labels),
            })
        }
        (PickedRefs::Refs(tags), RefsAction::Delete) => {
            let lost = what_deleting_several_loses(&names(&tags), history.commits_only_on_these(&tags));
            let description = match remote {
                Some(remote) => format!(
                    "{lost} If {remote} has them too, the next fetch brings them back; \
                     Delete {} tags from {remote}… removes them there.",
                    tags.len()
                ),
                None => lost,
            };
            Some(Question {
                title: format!("Delete {} tags?", tags.len()),
                description,
                write: RefsWrite::DeleteTags(tags),
            })
        }
        (PickedRefs::Refs(tags), RefsAction::DeleteThere) => {
            let remote = remote?.to_owned();
            Some(Question {
                title: format!("Delete {} tags from {remote}?", tags.len()),
                description: format!(
                    "The tags are removed from {remote} for everyone who uses it. \
                     They stay here, so a push would send them back."
                ),
                write: RefsWrite::DeleteTagsThere { tags, remote },
            })
        }
        (PickedRefs::Stashes(_), RefsAction::DeleteThere) | (_, RefsAction::Push) => None,
    }
}

fn what_deleting_several_loses(names: &[&str], commits_only_on_them: Option<usize>) -> String {
    match commits_only_on_them {
        Some(0) => format!(
            "Every commit of {} is on another branch or tag too, so nothing is lost.",
            listed(names)
        ),
        Some(count) => format!(
            "{} on no other branch or tag. Deleting {} loses {}.",
            counted(count, "commit is", "commits are"),
            listed(names),
            if count == 1 { "it" } else { "them" }
        ),
        None => what_deleting_loses(&listed(names), None),
    }
}

pub(crate) fn act(
    repository: Entity<Repository>,
    picked: PickedRefs,
    action: RefsAction,
    window: &mut Window,
    cx: &mut App,
) {
    let (history, remote) = {
        let repository = repository.read(cx);
        (repository.history().cloned(), repository.push_remote())
    };
    let Some(history) = history else {
        return;
    };
    if action == RefsAction::Push {
        let write = match picked {
            PickedRefs::Refs(labels) if labels[0].kind == RefKind::LocalBranch => RefsWrite::PushBranches(labels),
            PickedRefs::Refs(tags) if tags[0].kind == RefKind::Tag => match remote {
                Some(remote) => RefsWrite::PushTags { tags, remote },
                None => return,
            },
            _ => return,
        };
        return repository.update(cx, |repository, cx| repository.write_several(write, cx));
    }
    let Some(question) = question(picked, action, &history, remote.as_deref()) else {
        return;
    };
    let Question {
        title,
        description,
        write,
    } = question;
    window.open_alert_dialog(cx, move |alert, _, _| {
        let repository = repository.clone();
        let write = write.clone();
        alert
            .confirm()
            .title(title.clone())
            .description(description.clone())
            .ok_text(DELETE)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                let write = write.clone();
                repository.update(cx, |repository, cx| repository.write_several(write, cx));
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{
        PickedRefs, PickedRefsMenu, RefKey, RefPick, branches_pushed, remotes_of, tags_pushed,
        what_deleting_several_loses,
    };
    use crate::{
        git::{RefKind, RefLabel, Stash, fixtures::id},
        repo::operation::Outcome,
        ui::Picked,
    };

    fn label(name: &str, kind: RefKind) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head: false,
            upstream: None,
            from_upstream: None,
            worktree: None,
        }
    }

    fn texts(menu: &PickedRefsMenu, picked: &PickedRefs) -> Vec<Vec<String>> {
        menu.offered(picked)
            .into_iter()
            .map(|group| group.into_iter().map(|(_, text)| text).collect())
            .collect()
    }

    #[test]
    fn picked_branches_push_and_delete_all_but_the_checked_out_ones() {
        let menu = PickedRefsMenu::new(Some("origin".into()), false);
        let main = RefLabel {
            is_head: true,
            ..label("main", RefKind::LocalBranch)
        };
        let elsewhere = RefLabel {
            worktree: Some("tree".into()),
            ..label("topic", RefKind::LocalBranch)
        };
        let blame = label("feature/blame", RefKind::LocalBranch);
        let tabs = label("feature/tabs", RefKind::LocalBranch);
        assert_eq!(
            texts(&menu, &PickedRefs::Refs(vec![blame.clone(), tabs.clone()])),
            [vec!["Push 2 branches"], vec!["Delete 2 branches…"]]
        );
        assert_eq!(
            texts(&menu, &PickedRefs::Refs(vec![main.clone(), blame, tabs])),
            [vec!["Push 3 branches"], vec!["Delete 2 branches…"]]
        );
        assert_eq!(texts(&menu, &PickedRefs::Refs(vec![main, elsewhere])), [vec!["Push 2 branches"]]);
    }

    #[test]
    fn picked_remote_branches_are_deleted_from_their_remote() {
        let menu = PickedRefsMenu::new(Some("origin".into()), false);
        let one = vec![label("origin/a", RefKind::RemoteBranch), label("origin/b/c", RefKind::RemoteBranch)];
        assert_eq!(texts(&menu, &PickedRefs::Refs(one.clone())), [vec!["Delete 2 branches from origin…"]]);
        assert_eq!(remotes_of(&one), "origin");
        let two = vec![label("origin/a", RefKind::RemoteBranch), label("upstream/a", RefKind::RemoteBranch)];
        assert_eq!(texts(&menu, &PickedRefs::Refs(two)), [vec!["Delete 2 branches from their remotes…"]]);
    }

    #[test]
    fn picked_tags_push_and_delete_here_or_on_the_remote() {
        let tags = PickedRefs::Refs(vec![label("v1", RefKind::Tag), label("v2", RefKind::Tag)]);
        assert_eq!(
            texts(&PickedRefsMenu::new(Some("origin".into()), false), &tags),
            [vec!["Push 2 tags to origin"], vec!["Delete 2 tags…", "Delete 2 tags from origin…"]]
        );
        assert_eq!(texts(&PickedRefsMenu::new(None, false), &tags), [vec!["Delete 2 tags…"]]);
    }

    #[test]
    fn picked_stashes_are_only_deleted() {
        let stash = |n: u8| Stash {
            id: id(n),
            name: format!("stash@{{{n}}}").into(),
            message: "WIP".into(),
        };
        let picked = PickedRefs::Stashes(vec![stash(1), stash(2)]);
        assert_eq!(texts(&PickedRefsMenu::new(None, false), &picked), [vec!["Delete 2 stashes…"]]);
    }

    #[test]
    fn the_ref_pick_holds_only_while_the_graph_shows_the_commits_it_was_made_for() {
        let keys = Picked::only(RefKey::Ref(RefKind::LocalBranch, "a".into()));
        let pick = RefPick::new(keys, &Picked::only(id(1)));
        assert!(pick.current(&Picked::only(id(1)), |_| true).is_some());
        assert!(pick.current(&Picked::only(id(1)), |_| false).is_none());
        assert!(pick.current(&Picked::only(id(2)), |_| true).is_none());
        assert!(RefPick::default().current(&Picked::default(), |_| true).is_none());
    }

    #[test]
    fn the_delete_question_counts_the_commits_only_the_picked_refs_hold() {
        assert_eq!(
            what_deleting_several_loses(&["a", "b", "c"], Some(2)),
            "2 commits are on no other branch or tag. Deleting a, b and c loses them."
        );
        assert_eq!(
            what_deleting_several_loses(&["a", "b"], Some(0)),
            "Every commit of a and b is on another branch or tag too, so nothing is lost."
        );
    }

    #[test]
    fn the_notifications_say_what_was_left_out_or_already_there() {
        assert!(matches!(branches_pushed(3, &[]), Outcome::Done(done) if done.as_ref() == "Pushed 3 branches"));
        match branches_pushed(3, &["topic".to_owned()]) {
            Outcome::Warning { title, message } => {
                assert_eq!(title.as_ref(), "Pushed 2 branches");
                assert_eq!(
                    message.as_ref(),
                    "topic is behind its upstream, so it was left out. Pull it, then push it again."
                );
            }
            _ => panic!("a branch left out warns"),
        }
        assert_eq!(tags_pushed(2, &[], "origin"), "Pushed 2 tags to origin");
        assert_eq!(tags_pushed(2, &["v1".to_owned(), "v2".to_owned()], "origin"), "origin already has these 2 tags");
        assert_eq!(tags_pushed(3, &["v1".to_owned()], "origin"), "Pushed 2 tags to origin; it already had v1");
    }
}
