use std::sync::Arc;

use gpui_kit::{App, Bounds, Pixels, RenderImage, Window};

use crate::media::Sketch;

const ZOOM_STEP: f64 = 1.25;
const LEAST_ZOOM: f64 = 1e-4;
const MOST_ZOOM: f64 = 1e5;
const FIT_MARGIN: f64 = 0.08;

#[derive(Default)]
pub(crate) struct Sheet {
    key: Option<(u32, u32, u64)>,
    image: Option<Arc<RenderImage>>,
}

impl Sheet {
    pub(crate) fn image(
        &mut self,
        bounds: Bounds<Pixels>,
        stamp: u64,
        background: [u8; 3],
        window: &mut Window,
        draw: impl FnOnce(&mut Sketch, f32),
    ) -> Option<Arc<RenderImage>> {
        let scale = window.scale_factor();
        let width = (f32::from(bounds.size.width) * scale).round().max(1.) as u32;
        let height = (f32::from(bounds.size.height) * scale).round().max(1.) as u32;
        let key = (width, height, stamp);
        if self.key == Some(key) {
            return self.image.clone();
        }
        let mut sketch = Sketch::new(width, height, background);
        draw(&mut sketch, scale);
        let buffer = image::RgbaImage::from_raw(width, height, sketch.into_bgra())?;
        let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
        if let Some(old) = self.image.replace(image.clone()) {
            let _ = window.drop_image(old);
        }
        self.key = Some(key);
        Some(image)
    }

    pub(crate) fn forget(&mut self, cx: &mut App) {
        self.key = None;
        let Some(image) = self.image.take() else {
            return;
        };
        cx.spawn(async move |cx| cx.update(|cx| cx.drop_image(image, None))).detach();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Area {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
}

impl Area {
    pub(crate) fn around(points: impl IntoIterator<Item = (f64, f64)>) -> Option<Area> {
        let mut area: Option<Area> = None;
        for (x, y) in points {
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            area = Some(match area {
                None => Area { left: x, top: y, right: x, bottom: y },
                Some(area) => Area {
                    left: area.left.min(x),
                    top: area.top.min(y),
                    right: area.right.max(x),
                    bottom: area.bottom.max(y),
                },
            });
        }
        area
    }

    fn width(&self) -> f64 {
        (self.right - self.left).max(1e-6)
    }

    fn height(&self) -> f64 {
        (self.bottom - self.top).max(1e-6)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Viewport {
    pub(crate) zoom: Option<f64>,
    pub(crate) center: Option<(f64, f64)>,
    pub(crate) y_up: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Mapping {
    pub(crate) scale: f64,
    pub(crate) center: (f64, f64),
    pub(crate) size: (f64, f64),
    pub(crate) y_up: bool,
}

impl Mapping {
    pub(crate) fn screen(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let sx = (x - self.center.0) * self.scale + self.size.0 / 2.;
        let dy = (y - self.center.1) * self.scale;
        let sy = if self.y_up { self.size.1 / 2. - dy } else { self.size.1 / 2. + dy };
        (sx, sy)
    }

    pub(crate) fn world(&self, (sx, sy): (f64, f64)) -> (f64, f64) {
        let x = (sx - self.size.0 / 2.) / self.scale + self.center.0;
        let dy = if self.y_up { self.size.1 / 2. - sy } else { sy - self.size.1 / 2. };
        (x, dy / self.scale + self.center.1)
    }

    pub(crate) fn stamp(&self) -> u64 {
        self.scale.to_bits() ^ self.center.0.to_bits().rotate_left(13) ^ self.center.1.to_bits().rotate_left(29)
    }
}

impl Viewport {
    pub(crate) fn new(y_up: bool) -> Self {
        Self {
            zoom: None,
            center: None,
            y_up,
        }
    }

    pub(crate) fn mapping(&self, content: Option<Area>, size: (f64, f64)) -> Mapping {
        let content = content.unwrap_or(Area { left: -1., top: -1., right: 1., bottom: 1. });
        let usable = (size.0 * (1. - 2. * FIT_MARGIN), size.1 * (1. - 2. * FIT_MARGIN));
        let fit = (usable.0 / content.width()).min(usable.1 / content.height()).clamp(LEAST_ZOOM, MOST_ZOOM);
        let center = self.center.unwrap_or(((content.left + content.right) / 2., (content.top + content.bottom) / 2.));
        Mapping {
            scale: self.zoom.unwrap_or(fit),
            center,
            size,
            y_up: self.y_up,
        }
    }

    pub(crate) fn fits(&self) -> bool {
        self.zoom.is_none() && self.center.is_none()
    }

    pub(crate) fn fit(&mut self) {
        self.zoom = None;
        self.center = None;
    }

    pub(crate) fn zoom_by(&mut self, mapping: &Mapping, steps: f64, at: Option<(f64, f64)>) {
        let scale = (mapping.scale * ZOOM_STEP.powf(steps)).clamp(LEAST_ZOOM, MOST_ZOOM);
        let anchor = at.unwrap_or((mapping.size.0 / 2., mapping.size.1 / 2.));
        let under = mapping.world(anchor);
        let zoomed = Mapping { scale, ..*mapping };
        let landed = zoomed.world(anchor);
        self.zoom = Some(scale);
        self.center = Some((mapping.center.0 + under.0 - landed.0, mapping.center.1 + under.1 - landed.1));
    }

    pub(crate) fn pan_by(&mut self, mapping: &Mapping, dx: f64, dy: f64) {
        let shift_y = if self.y_up { -dy } else { dy };
        self.zoom = Some(mapping.scale);
        self.center = Some((mapping.center.0 - dx / mapping.scale, mapping.center.1 - shift_y / mapping.scale));
    }

    pub(crate) fn percent(&self, mapping: &Mapping, unit: f64) -> f64 {
        mapping.scale / unit * 100.
    }
}

#[cfg(test)]
mod tests {
    use super::{Area, Viewport};

    #[test]
    fn fitting_puts_the_content_in_the_middle_with_a_margin() {
        let viewport = Viewport::new(true);
        let content = Area::around([(0., 0.), (10., 5.)]);
        let mapping = viewport.mapping(content, (1000., 500.));
        let (x, y) = mapping.screen((5., 2.5));
        assert!((x - 500.).abs() < 1e-9 && (y - 250.).abs() < 1e-9);
        let (left, top) = mapping.screen((0., 5.));
        assert!(left > 0. && top > 0.);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_in_place() {
        let mut viewport = Viewport::new(false);
        let mapping = viewport.mapping(Area::around([(0., 0.), (100., 100.)]), (400., 400.));
        let under = mapping.world((120., 80.));
        viewport.zoom_by(&mapping, 2., Some((120., 80.)));
        let zoomed = viewport.mapping(Area::around([(0., 0.), (100., 100.)]), (400., 400.));
        let after = zoomed.world((120., 80.));
        assert!((under.0 - after.0).abs() < 1e-9 && (under.1 - after.1).abs() < 1e-9);
    }
}
