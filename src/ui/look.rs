use gpui_kit::{Styled, rems};

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

    fn xs_text(self) -> Self {
        self.text_xs().line_height(rems(1.))
    }

    fn sm_text(self) -> Self {
        self.text_sm().line_height(rems(1.25))
    }

    fn base_text(self) -> Self {
        self.text_base().line_height(rems(1.5))
    }

    fn lg_text(self) -> Self {
        self.text_lg().line_height(rems(1.75))
    }
}

impl<E: Styled> Look for E {}
