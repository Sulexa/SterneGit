use std::{borrow::Borrow, collections::HashSet, hash::Hash};

pub(crate) struct Folded<G> {
    toggled: HashSet<G>,
    starts_open: bool,
}

impl<G> Default for Folded<G> {
    fn default() -> Self {
        Self {
            toggled: HashSet::new(),
            starts_open: true,
        }
    }
}

impl<G: Hash + Eq> Folded<G> {
    pub(crate) fn all_folded_but(opened: impl IntoIterator<Item = G>) -> Self {
        Self {
            toggled: opened.into_iter().collect(),
            starts_open: false,
        }
    }

    pub(crate) fn toggle(&mut self, group: G) {
        if !self.toggled.remove(&group) {
            self.toggled.insert(group);
        }
    }

    pub(crate) fn is_open<Q>(&self, group: &Q) -> bool
    where
        G: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.starts_open != self.toggled.contains(group)
    }

    pub(crate) fn toggled(&self) -> impl Iterator<Item = &G> {
        self.toggled.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::Folded;

    #[test]
    fn everything_starts_open() {
        assert!(Folded::<String>::default().is_open("src"));
    }

    #[test]
    fn toggling_folds_then_opens_again() {
        let mut folded = Folded::default();
        folded.toggle("src".to_owned());
        assert!(!folded.is_open("src"));
        assert!(folded.is_open("docs"));
        folded.toggle("src".to_owned());
        assert!(folded.is_open("src"));
    }

    #[test]
    fn all_folded_but_opens_only_the_given_groups() {
        let folded = Folded::all_folded_but(["src".to_owned()]);
        assert!(folded.is_open("src"));
        assert!(!folded.is_open("docs"));
    }

    #[test]
    fn toggling_in_an_all_folded_set_opens_then_folds_again() {
        let mut folded = Folded::all_folded_but([]);
        folded.toggle("docs".to_owned());
        assert!(folded.is_open("docs"));
        assert_eq!(folded.toggled().collect::<Vec<_>>(), ["docs"]);
        folded.toggle("docs".to_owned());
        assert!(!folded.is_open("docs"));
        assert_eq!(folded.toggled().count(), 0);
    }
}
