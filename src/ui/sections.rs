#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SectionRow<K> {
    Header(K, usize),
    Item(K, usize),
}

#[derive(Clone)]
pub(crate) struct Sections<K> {
    sections: Vec<(K, usize)>,
}

impl<K: Copy> Sections<K> {
    pub(crate) fn new(sections: impl IntoIterator<Item = (K, usize)>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
        }
    }

    pub(crate) fn rows(&self) -> usize {
        self.sections.iter().map(|(_, items)| items + 1).sum()
    }

    pub(crate) fn row(&self, ix: usize) -> Option<SectionRow<K>> {
        let mut ix = ix;
        for &(key, items) in &self.sections {
            if ix == 0 {
                return Some(SectionRow::Header(key, items));
            }
            if ix <= items {
                return Some(SectionRow::Item(key, ix - 1));
            }
            ix -= items + 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{SectionRow, Sections};

    #[test]
    fn each_header_comes_before_its_items() {
        let sections = Sections::new([('a', 2), ('b', 0), ('c', 1)]);
        let rows: Vec<_> = (0..sections.rows())
            .filter_map(|ix| sections.row(ix))
            .collect();
        assert_eq!(
            rows,
            [
                SectionRow::Header('a', 2),
                SectionRow::Item('a', 0),
                SectionRow::Item('a', 1),
                SectionRow::Header('b', 0),
                SectionRow::Header('c', 1),
                SectionRow::Item('c', 0),
            ]
        );
        assert_eq!(sections.row(sections.rows()), None);
    }
}
