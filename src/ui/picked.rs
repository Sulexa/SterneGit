use std::{collections::HashSet, hash::Hash, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    Only,
    Toggle,
    Run,
    AddRun,
}

impl Pick {
    pub(crate) fn new(adding: bool, run: bool) -> Self {
        match (adding, run) {
            (false, false) => Pick::Only,
            (true, false) => Pick::Toggle,
            (false, true) => Pick::Run,
            (true, true) => Pick::AddRun,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Picked<K: Hash + Eq> {
    keys: HashSet<K>,
    anchor: Option<K>,
}

impl<K: Hash + Eq> Default for Picked<K> {
    fn default() -> Self {
        Self {
            keys: HashSet::new(),
            anchor: None,
        }
    }
}

impl<K: Clone + Hash + Eq> Picked<K> {
    pub(crate) fn only(key: K) -> Self {
        Self {
            keys: HashSet::from([key.clone()]),
            anchor: Some(key),
        }
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.keys.contains(key)
    }

    pub(crate) fn after(&self, key: K, pick: Pick, shown: &[K]) -> Self {
        let on_screen = shown.iter().collect::<HashSet<_>>();
        let mut kept = self
            .keys
            .iter()
            .filter(|kept| on_screen.contains(kept))
            .cloned()
            .collect::<HashSet<_>>();
        let anchor = self.anchor.as_ref().filter(|anchor| on_screen.contains(anchor));
        match (pick, anchor) {
            (Pick::Only, _) | (Pick::Run, None) => Self::only(key),
            (Pick::Toggle, _) | (Pick::AddRun, None) => {
                if !kept.remove(&key) {
                    kept.insert(key.clone());
                }
                Self {
                    keys: kept,
                    anchor: Some(key),
                }
            }
            (Pick::Run | Pick::AddRun, Some(anchor)) => {
                if pick == Pick::Run {
                    kept.clear();
                }
                kept.extend(run_between(shown, anchor, &key).iter().cloned());
                Self {
                    keys: kept,
                    anchor: Some(anchor.clone()),
                }
            }
        }
    }

    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        self.keys.retain(|key| keep(key));
        if !self.anchor.as_ref().is_some_and(&keep) {
            self.anchor = None;
        }
    }
}

pub(crate) struct PickedFor<K: Hash + Eq> {
    made_for: Option<K>,
    picked: Rc<Picked<K>>,
}

impl<K: Hash + Eq> Default for PickedFor<K> {
    fn default() -> Self {
        Self {
            made_for: None,
            picked: Rc::default(),
        }
    }
}

impl<K: Clone + Hash + Eq> PickedFor<K> {
    pub(crate) fn new(made_for: Option<K>, picked: Picked<K>) -> Self {
        Self {
            made_for,
            picked: Rc::new(picked),
        }
    }

    pub(crate) fn current(&self, open: Option<&K>) -> Rc<Picked<K>> {
        if self.made_for.as_ref() == open {
            self.picked.clone()
        } else {
            Rc::new(open.cloned().map(Picked::only).unwrap_or_default())
        }
    }

    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        Rc::make_mut(&mut self.picked).retain(keep);
    }
}

fn run_between<'a, K: PartialEq>(shown: &'a [K], from: &K, to: &K) -> &'a [K] {
    let at = |key: &K| shown.iter().position(|shown| shown == key);
    match (at(from), at(to)) {
        (Some(from), Some(to)) => &shown[from.min(to)..=from.max(to)],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::{Pick, Picked, PickedFor};

    const SHOWN: [&str; 5] = ["a", "b", "c", "d", "e"];

    fn keys(picked: &Picked<&'static str>) -> Vec<&'static str> {
        let mut keys = SHOWN
            .into_iter()
            .chain(["x", "y"])
            .filter(|key| picked.contains(key))
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }

    fn clicks(steps: &[(&'static str, Pick)]) -> Picked<&'static str> {
        steps
            .iter()
            .fold(Picked::default(), |picked, &(key, pick)| picked.after(key, pick, &SHOWN))
    }

    #[test]
    fn modifiers_choose_the_pick() {
        assert_eq!(Pick::new(false, false), Pick::Only);
        assert_eq!(Pick::new(true, false), Pick::Toggle);
        assert_eq!(Pick::new(false, true), Pick::Run);
        assert_eq!(Pick::new(true, true), Pick::AddRun);
    }

    #[test]
    fn a_plain_click_picks_that_key_alone() {
        let picked = clicks(&[("a", Pick::Only), ("c", Pick::Toggle), ("d", Pick::Only)]);
        assert_eq!(keys(&picked), ["d"]);
    }

    #[test]
    fn a_toggle_adds_a_key_then_takes_it_out() {
        let picked = clicks(&[("a", Pick::Only), ("c", Pick::Toggle)]);
        assert_eq!(keys(&picked), ["a", "c"]);
        assert_eq!(keys(&picked.after("a", Pick::Toggle, &SHOWN)), ["c"]);
    }

    #[test]
    fn a_run_picks_every_key_shown_between_the_last_click_and_this_one() {
        assert_eq!(keys(&clicks(&[("b", Pick::Only), ("d", Pick::Run)])), ["b", "c", "d"]);
        assert_eq!(keys(&clicks(&[("d", Pick::Only), ("b", Pick::Run)])), ["b", "c", "d"]);
    }

    #[test]
    fn a_second_run_starts_where_the_first_did_and_replaces_it() {
        let picked = clicks(&[("c", Pick::Only), ("e", Pick::Run), ("a", Pick::Run)]);
        assert_eq!(keys(&picked), ["a", "b", "c"]);
    }

    #[test]
    fn an_added_run_keeps_what_was_picked() {
        let picked = clicks(&[("a", Pick::Only), ("d", Pick::Toggle), ("e", Pick::AddRun)]);
        assert_eq!(keys(&picked), ["a", "d", "e"]);
    }

    #[test]
    fn a_run_with_nothing_clicked_before_picks_the_key_alone() {
        assert_eq!(keys(&clicks(&[("c", Pick::Run)])), ["c"]);
        assert_eq!(keys(&clicks(&[("c", Pick::AddRun)])), ["c"]);
    }

    #[test]
    fn a_click_drops_keys_that_are_no_longer_shown() {
        let elsewhere = Picked::only("x").after("y", Pick::Toggle, &["x", "y"]);
        assert_eq!(keys(&elsewhere), ["x", "y"]);
        let picked = elsewhere.after("b", Pick::Toggle, &SHOWN);
        assert_eq!(keys(&picked), ["b"]);
        assert_eq!(keys(&elsewhere.after("d", Pick::Run, &SHOWN)), ["d"]);
    }

    #[test]
    fn retaining_drops_keys_and_forgets_a_dropped_anchor() {
        let mut picked = clicks(&[("b", Pick::Only), ("d", Pick::Run)]);
        picked.retain(|key| *key != "b");
        assert_eq!(keys(&picked), ["c", "d"]);
        assert_eq!(keys(&picked.after("e", Pick::Run, &SHOWN)), ["e"]);
    }

    #[test]
    fn a_pick_holds_while_the_file_it_was_made_for_stays_open() {
        let made = PickedFor::new(Some("a"), clicks(&[("a", Pick::Only), ("c", Pick::Toggle)]));
        assert_eq!(keys(&made.current(Some(&"a"))), ["a", "c"]);
        assert_eq!(keys(&made.current(Some(&"b"))), ["b"]);
        assert_eq!(keys(&made.current(None)), Vec::<&str>::new());
    }
}
