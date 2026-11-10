use gpui_kit::{App, Context, Empty, Global, IntoElement, Pixels, Render, Window, px};

pub(super) const HIERARCHY_LEAST: Pixels = px(200.);
const HIERARCHY_MOST: Pixels = px(640.);
pub(super) const INSPECTOR_LEAST: Pixels = px(480.);
const PROPERTY_LEAST: Pixels = px(120.);
const PROPERTY_MOST: Pixels = px(480.);
const VALUE_LEAST: Pixels = px(120.);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct InspectorWidths {
    pub(super) hierarchy: Pixels,
    pub(super) property: Pixels,
}

impl Default for InspectorWidths {
    fn default() -> Self {
        Self {
            hierarchy: px(232.),
            property: px(176.),
        }
    }
}

impl Global for InspectorWidths {}

impl InspectorWidths {
    pub(super) fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    pub(super) fn with_hierarchy(self, wanted: Pixels, whole: Pixels) -> Self {
        let hierarchy = wanted.min(HIERARCHY_MOST).min(whole - INSPECTOR_LEAST).max(HIERARCHY_LEAST);
        Self { hierarchy, ..self }
    }

    pub(super) fn with_property(self, wanted: Pixels, inspector: Pixels) -> Self {
        let property = wanted.min(PROPERTY_MOST).min(inspector - VALUE_LEAST * 2.).max(PROPERTY_LEAST);
        Self { property, ..self }
    }
}

#[derive(Clone, Copy)]
pub(super) struct HierarchyEdge;

#[derive(Clone, Copy)]
pub(super) struct PropertyEdge;

impl Render for HierarchyEdge {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Render for PropertyEdge {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::px;

    use super::InspectorWidths;

    #[test]
    fn the_hierarchy_follows_the_pointer_between_200_and_640_px() {
        let widths = InspectorWidths::default();
        assert_eq!(widths.with_hierarchy(px(300.), px(1400.)).hierarchy, px(300.));
        assert_eq!(widths.with_hierarchy(px(40.), px(1400.)).hierarchy, px(200.));
        assert_eq!(widths.with_hierarchy(px(900.), px(1400.)).hierarchy, px(640.));
    }

    #[test]
    fn the_hierarchy_leaves_the_inspector_480_px_unless_that_would_shrink_it_below_200() {
        let widths = InspectorWidths::default();
        assert_eq!(widths.with_hierarchy(px(500.), px(800.)).hierarchy, px(320.));
        assert_eq!(widths.with_hierarchy(px(500.), px(500.)).hierarchy, px(200.));
    }

    #[test]
    fn the_property_column_leaves_each_value_column_120_px() {
        let widths = InspectorWidths::default();
        assert_eq!(widths.with_property(px(260.), px(900.)).property, px(260.));
        assert_eq!(widths.with_property(px(60.), px(900.)).property, px(120.));
        assert_eq!(widths.with_property(px(700.), px(900.)).property, px(480.));
        assert_eq!(widths.with_property(px(400.), px(560.)).property, px(320.));
    }

    #[test]
    fn dragging_one_width_keeps_the_other() {
        let widths = InspectorWidths::default().with_property(px(250.), px(900.)).with_hierarchy(px(300.), px(1400.));
        assert_eq!((widths.hierarchy, widths.property), (px(300.), px(250.)));
    }
}
