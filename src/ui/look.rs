use gpui_kit::Styled;

pub(crate) trait Look: Styled + Sized {
    fn pane(self) -> Self {
        self.size_full().min_w_0().min_h_0()
    }

    fn fill_height(self) -> Self {
        self.flex_1().min_h_0()
    }

    fn ellipsis(self) -> Self {
        self.min_w_0().truncate()
    }
}

impl<E: Styled> Look for E {}
