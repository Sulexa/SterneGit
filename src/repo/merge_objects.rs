use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

use gpui_kit::{
    AnyElement, Context, EventEmitter, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use sternegit_reader_protocol::{Choice, Conflict, MergeInput, Merged, Pick};

use crate::{
    git::{FileChange, Repo, Revision},
    read::{Reader, RevisionTree, Tree},
    ui::{Look as _, counted},
};

const LABEL_WIDTH: f32 = 180.;

#[derive(Clone)]
pub(crate) struct MergeByReader {
    pub(crate) name: String,
    pub(crate) reader: Arc<Reader>,
    pub(crate) input: MergeInput,
}

impl MergeByReader {
    pub(crate) fn run(&self, repo: &Repo, choices: Vec<Choice>) -> anyhow::Result<Merged> {
        let tree: Arc<dyn Tree> = Arc::new(RevisionTree::new(repo.clone(), Revision::WorkTree, self.input.path.clone()));
        let input = MergeInput {
            choices,
            ..self.input.clone()
        };
        self.reader
            .merge(tree, input, &AtomicBool::new(false))
            .map_err(|error| anyhow::anyhow!("{} couldn't merge this file: {error}", self.name))
    }
}

pub(crate) enum ObjectMergeEvent {
    AsText,
}

pub(crate) struct ObjectMerge {
    change: FileChange,
    by: MergeByReader,
    labels: [SharedString; 2],
    conflicts: Vec<Conflict>,
    merged: usize,
    choices: HashMap<(String, String), Pick>,
}

impl EventEmitter<ObjectMergeEvent> for ObjectMerge {}

fn key(conflict: &Conflict) -> (String, String) {
    (conflict.object.clone(), conflict.field.clone())
}

impl ObjectMerge {
    pub(crate) fn new(change: FileChange, by: MergeByReader, labels: [SharedString; 2], merged: Merged) -> Self {
        Self {
            change,
            by,
            labels,
            conflicts: merged.conflicts,
            merged: merged.merged,
            choices: HashMap::new(),
        }
    }

    pub(crate) fn change(&self) -> &FileChange {
        &self.change
    }

    pub(crate) fn by(&self) -> &MergeByReader {
        &self.by
    }

    pub(crate) fn left(&self) -> usize {
        self.conflicts.iter().filter(|conflict| !self.choices.contains_key(&key(conflict))).count()
    }

    pub(crate) fn total(&self) -> usize {
        self.conflicts.len()
    }

    pub(crate) fn is_resolved(&self) -> bool {
        self.left() == 0
    }

    pub(crate) fn choices(&self) -> Vec<Choice> {
        self.choices
            .iter()
            .map(|((object, field), pick)| Choice {
                object: object.clone(),
                field: field.clone(),
                pick: *pick,
            })
            .collect()
    }

    fn take(&mut self, conflict: (String, String), pick: Pick, cx: &mut Context<Self>) {
        self.choices.insert(conflict, pick);
        cx.notify();
    }

    fn take_object(&mut self, object: &str, pick: Pick, cx: &mut Context<Self>) {
        for conflict in self.conflicts.iter().filter(|conflict| conflict.object == object) {
            self.choices.insert(key(conflict), pick);
        }
        cx.notify();
    }

    fn start_over(&mut self, cx: &mut Context<Self>) {
        self.choices.clear();
        cx.notify();
    }

    fn cell(&self, ix: usize, conflict: &Conflict, pick: Pick, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (blue, muted, mono) = (theme.blue, theme.muted_foreground, theme.mono_font_family.clone());
        let chosen = self.choices.get(&key(conflict)).copied();
        let taken = chosen == Some(pick);
        let dropped = chosen.is_some() && !taken;
        let text = match pick {
            Pick::Current => conflict.current.clone(),
            Pick::Incoming => conflict.incoming.clone(),
        };
        let id = key(conflict);
        let side = if pick == Pick::Current { "current" } else { "incoming" };
        h_flex()
            .id((SharedString::from(format!("merge-{side}")), ix))
            .flex_1()
            .min_w_0()
            .gap_2()
            .px_2()
            .py_0p5()
            .rounded(px(5.))
            .border_1()
            .border_color(if taken { blue } else { gpui_kit::transparent_black() })
            .when(taken, |cell| cell.bg(blue.opacity(0.12)))
            .when(dropped, |cell| cell.opacity(0.5).line_through())
            .cursor_pointer()
            .child(Icon::new(if taken { IconName::CircleCheck } else { IconName::Circle }).xsmall().text_color(if taken { blue } else { muted }))
            .child(div().min_w_0().sm_text().font_family(mono).ellipsis().child(text))
            .on_click(cx.listener(move |this, _, _, cx| this.take(id.clone(), pick, cx)))
            .into_any_element()
    }
}

impl Render for ObjectMerge {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border, yellow, green) = (theme.muted_foreground, theme.border, theme.yellow, theme.green);
        let left = self.left();
        let reader = self.by.name.clone();
        let state = if left > 0 {
            div().sm_text().text_color(yellow).child(format!("{} to choose", counted(left, "field", "fields")))
        } else {
            div().sm_text().text_color(green).child("No conflicts left")
        };
        let bar = h_flex()
            .flex_none()
            .gap_2()
            .px_3()
            .py_1p5()
            .border_b_1()
            .border_color(border)
            .child(Icon::new(IconName::GitMerge).small().text_color(muted))
            .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(format!("Merged by {reader}")))
            .child(state)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .sm_text()
                    .text_color(muted)
                    .ellipsis()
                    .child(format!("{} from both sides merged on their own", counted(self.merged, "change", "changes"))),
            )
            .child(
                Button::new("merge-as-text")
                    .ghost()
                    .xsmall()
                    .icon(IconName::FileText)
                    .label("Merge as text")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ObjectMergeEvent::AsText))),
            )
            .child(
                Button::new("merge-start-over")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Undo2)
                    .label("Start over")
                    .tooltip("Forget every choice")
                    .on_click(cx.listener(|this, _, _, cx| this.start_over(cx))),
            );
        let head = |dot: gpui_kit::Hsla, title: &'static str, label: SharedString| {
            h_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(div().size(px(8.)).rounded(px(2.)).bg(dot))
                .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(title))
                .child(div().sm_text().text_color(muted).ellipsis().child(label))
        };
        let sides = h_flex()
            .flex_none()
            .gap_2()
            .px_3()
            .py_1p5()
            .border_b_1()
            .border_color(border)
            .child(div().w(px(LABEL_WIDTH)).flex_none())
            .child(head(theme.green, "Current", self.labels[0].clone()))
            .child(head(theme.blue, "Incoming", self.labels[1].clone()));
        let mut objects: Vec<(String, Vec<usize>)> = Vec::new();
        for (ix, conflict) in self.conflicts.iter().enumerate() {
            match objects.iter_mut().find(|(object, _)| *object == conflict.object) {
                Some((_, rows)) => rows.push(ix),
                None => objects.push((conflict.object.clone(), vec![ix])),
            }
        }
        let sections = objects
            .into_iter()
            .enumerate()
            .map(|(at, (object, rows))| {
                let first = &self.conflicts[rows[0]];
                let (name, part) = (first.name.clone(), first.part.clone());
                let (take_current, take_incoming) = (object.clone(), object.clone());
                let header = h_flex()
                    .gap_2()
                    .px_3()
                    .pt_2()
                    .pb_1()
                    .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(name))
                    .child(div().xs_text().text_color(muted).child(part))
                    .child(div().flex_1())
                    .child(
                        Button::new(("take-current", at))
                            .ghost()
                            .xsmall()
                            .label("Take current")
                            .on_click(cx.listener(move |this, _, _, cx| this.take_object(&take_current, Pick::Current, cx))),
                    )
                    .child(
                        Button::new(("take-incoming", at))
                            .ghost()
                            .xsmall()
                            .label("Take incoming")
                            .on_click(cx.listener(move |this, _, _, cx| this.take_object(&take_incoming, Pick::Incoming, cx))),
                    );
                let lines = rows
                    .iter()
                    .map(|&ix| {
                        let conflict = self.conflicts[ix].clone();
                        h_flex()
                            .gap_2()
                            .px_3()
                            .py_0p5()
                            .child(div().w(px(LABEL_WIDTH)).flex_none().sm_text().text_color(muted).ellipsis().child(conflict.label.clone()))
                            .child(self.cell(ix, &conflict, Pick::Current, cx))
                            .child(self.cell(ix, &conflict, Pick::Incoming, cx))
                            .into_any_element()
                    })
                    .collect::<Vec<_>>();
                v_flex()
                    .pb_1p5()
                    .border_b_1()
                    .border_color(border)
                    .child(header)
                    .children(lines)
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        v_flex()
            .size_full()
            .child(bar)
            .child(sides)
            .child(v_flex().id("merge-objects").flex_1().min_h_0().overflow_y_scroll().children(sections))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, ffi::OsStr, path::Path, sync::Arc};

    use sternegit_reader_protocol::{MergeInput, Pick};

    use super::{MergeByReader, ObjectMerge, key};
    use crate::{
        git::{Revision, Version, scratch::ScratchRepo},
        read::Reader,
    };

    fn prefab(speed: &str, height: &str) -> String {
        format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Slime\n--- !u!114 &500\nMonoBehaviour:\n  m_GameObject: {{fileID: 100}}\n  moveSpeed: {speed}\n  jumpHeight: {height}\n"
        )
    }

    #[test]
    fn a_conflicted_prefab_merges_by_object_and_writes_the_chosen_fields() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("merge-by-object")?;
        scratch.write("Slime.prefab", &prefab("2.5", "1.2"))?;
        scratch.git(&["add", "-A"])?;
        scratch.commit("base")?;
        scratch.git(&["checkout", "-q", "-b", "hop"])?;
        scratch.write("Slime.prefab", &prefab("4", "1.6"))?;
        scratch.git(&["commit", "-q", "-am", "hop"])?;
        scratch.git(&["checkout", "-q", "-"])?;
        scratch.write("Slime.prefab", &prefab("3", "1.2"))?;
        scratch.git(&["commit", "-q", "-am", "speed"])?;
        let _ = scratch.git(&["merge", "-q", "hop"]);
        let (repo, _) = scratch.open()?;
        let path = OsStr::new("Slime.prefab");
        let text = |version| String::from_utf8_lossy(&repo.file_bytes(Revision::Conflict(version), path).unwrap()).into_owned();
        let wasm = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("readers/unity/dist/unity.wasm"))?;
        let by = MergeByReader {
            name: "Unity".into(),
            reader: Arc::new(Reader::compile(&wasm).unwrap()),
            input: MergeInput {
                kind: "prefab".into(),
                path: "Slime.prefab".into(),
                base: repo.conflict_base(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
                current: text(Version::Current),
                incoming: text(Version::Incoming),
                choices: Vec::new(),
            },
        };
        let merged = by.run(&repo, Vec::new())?;
        assert_eq!(merged.merged, 1, "jumpHeight came from one side only");
        assert_eq!(merged.conflicts.len(), 1);
        let mut merge = ObjectMerge {
            change: crate::git::FileChange::new(b"Slime.prefab", None, crate::git::ChangeKind::Conflicted),
            by,
            labels: Default::default(),
            conflicts: merged.conflicts,
            merged: merged.merged,
            choices: HashMap::new(),
        };
        assert!(!merge.is_resolved());
        let conflict = key(&merge.conflicts[0]);
        merge.choices.insert(conflict, Pick::Incoming);
        assert!(merge.is_resolved());
        let text = merge.by().run(&repo, merge.choices())?.text.expect("every field chosen");
        assert!(text.contains("moveSpeed: 4\n  jumpHeight: 1.6\n"), "{text}");
        Ok(())
    }
}
