pub(crate) struct Tabs<T> {
    items: Vec<T>,
    active: usize,
}

impl<T> Default for Tabs<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            active: 0,
        }
    }
}

impl<T> Tabs<T> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn active(&self) -> Option<&T> {
        self.items.get(self.active)
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active
    }

    pub(crate) fn position(&self, predicate: impl FnMut(&T) -> bool) -> Option<usize> {
        self.items.iter().position(predicate)
    }

    pub(crate) fn push(&mut self, item: T) -> usize {
        self.items.push(item);
        self.items.len() - 1
    }

    pub(crate) fn activate(&mut self, ix: usize) -> bool {
        let exists = ix < self.items.len();
        if exists {
            self.active = ix;
        }
        exists
    }

    pub(crate) fn close(&mut self, ix: usize) -> bool {
        if ix >= self.items.len() {
            return false;
        }
        self.items.remove(ix);
        let active_shifted_left = ix < self.active;
        let active_fell_off_the_end = self.active >= self.items.len();
        if active_shifted_left || active_fell_off_the_end {
            self.active = self.active.saturating_sub(1);
        }
        true
    }

    pub(crate) fn cycle(&mut self, delta: isize) -> bool {
        let count = self.items.len();
        if count < 2 {
            return false;
        }
        self.active = (self.active as isize + delta).rem_euclid(count as isize) as usize;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Tabs;

    fn tabs(names: &[&'static str], active: usize) -> Tabs<&'static str> {
        let mut tabs = Tabs::default();
        for name in names {
            tabs.push(*name);
        }
        tabs.activate(active);
        tabs
    }

    #[test]
    fn closing_a_tab_left_of_the_active_one_keeps_the_same_tab_active() {
        let mut tabs = tabs(&["a", "b", "c"], 1);
        assert!(tabs.close(0));
        assert_eq!(tabs.active(), Some(&"b"));
    }

    #[test]
    fn closing_a_tab_right_of_the_active_one_keeps_the_same_tab_active() {
        let mut tabs = tabs(&["a", "b", "c"], 1);
        assert!(tabs.close(2));
        assert_eq!(tabs.active(), Some(&"b"));
    }

    #[test]
    fn closing_the_active_tab_activates_the_one_that_takes_its_place() {
        let mut tabs = tabs(&["a", "b", "c"], 1);
        assert!(tabs.close(1));
        assert_eq!(tabs.active(), Some(&"c"));
    }

    #[test]
    fn closing_the_active_last_tab_activates_the_new_last_one() {
        let mut tabs = tabs(&["a", "b", "c"], 2);
        assert!(tabs.close(2));
        assert_eq!(tabs.active(), Some(&"b"));
    }

    #[test]
    fn closing_the_only_tab_leaves_nothing_active() {
        let mut tabs = tabs(&["a"], 0);
        assert!(tabs.close(0));
        assert!(tabs.is_empty());
        assert_eq!(tabs.active(), None);
    }

    #[test]
    fn cycling_wraps_around_both_ends() {
        let mut tabs = tabs(&["a", "b", "c"], 2);
        assert!(tabs.cycle(1));
        assert_eq!(tabs.active(), Some(&"a"));
        assert!(tabs.cycle(-1));
        assert_eq!(tabs.active(), Some(&"c"));
    }

    #[test]
    fn a_single_tab_does_not_cycle() {
        let mut tabs = tabs(&["a"], 0);
        assert!(!tabs.cycle(1));
        assert_eq!(tabs.active(), Some(&"a"));
    }

    #[test]
    fn indexes_past_the_end_change_nothing() {
        let mut tabs = tabs(&["a", "b"], 1);
        assert!(!tabs.activate(2));
        assert!(!tabs.close(5));
        assert_eq!(tabs.active(), Some(&"b"));
    }
}
