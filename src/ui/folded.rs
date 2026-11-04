use std::{borrow::Borrow, collections::HashSet, hash::Hash};

pub(crate) struct Folded<G>(HashSet<G>);

impl<G> Default for Folded<G> {
    fn default() -> Self {
        Self(HashSet::new())
    }
}

impl<G: Hash + Eq> Folded<G> {
    pub(crate) fn toggle(&mut self, group: G) {
        if !self.0.remove(&group) {
            self.0.insert(group);
        }
    }

    pub(crate) fn is_open<Q>(&self, group: &Q) -> bool
    where
        G: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        !self.0.contains(group)
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
}
