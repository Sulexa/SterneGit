use super::{PixelChange, PixelDifference, Picture};

const CHECKER_DARK: [u8; 3] = [0x1a, 0x1a, 0x1a];
const CHECKER_LIGHT: [u8; 3] = [0x26, 0x26, 0x26];
const DIMMED: f32 = 0.42;
const MOST_SAMPLES_ACROSS: u32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Backdrop {
    #[default]
    Checkerboard,
    Black,
    White,
    Panel,
}

impl Backdrop {
    pub(crate) const ALL: [Backdrop; 4] = [
        Backdrop::Checkerboard,
        Backdrop::Black,
        Backdrop::White,
        Backdrop::Panel,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Backdrop::Checkerboard => "Checkerboard",
            Backdrop::Black => "Black",
            Backdrop::White => "White",
            Backdrop::Panel => "Panel color",
        }
    }

    fn color(self, x: u32, y: u32, surface: &Surface) -> [f32; 3] {
        let rgb = match self {
            Backdrop::Checkerboard => {
                let square = surface.checker.max(1);
                if (x / square + y / square) % 2 == 0 {
                    CHECKER_DARK
                } else {
                    CHECKER_LIGHT
                }
            }
            Backdrop::Black => [0, 0, 0],
            Backdrop::White => [255, 255, 255],
            Backdrop::Panel => surface.panel,
        };
        rgb.map(|channel| f32::from(channel) / 255.)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) scale: f64,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Surface {
    pub(crate) backdrop: Backdrop,
    pub(crate) panel: [u8; 3],
    pub(crate) checker: u32,
}

pub(crate) enum Layer<'a> {
    Picture(&'a Picture),
    Stretched {
        picture: &'a Picture,
        onto: &'a Picture,
    },
    Onion {
        before: &'a Picture,
        after: &'a Picture,
        mix: f32,
    },
    Difference {
        after: &'a Picture,
        difference: &'a PixelDifference,
        colors: [[u8; 3]; 3],
    },
}

impl Layer<'_> {
    fn reference(&self) -> (u32, u32) {
        let picture = match self {
            Layer::Picture(picture) | Layer::Stretched { onto: picture, .. } => picture,
            Layer::Onion { after, .. } | Layer::Difference { after, .. } => after,
        };
        (picture.width, picture.height)
    }
}

struct Footprint {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

struct Source<'a> {
    picture: &'a Picture,
    ratio_x: f64,
    ratio_y: f64,
}

impl<'a> Source<'a> {
    fn new(picture: &'a Picture, reference: (u32, u32)) -> Self {
        Self {
            picture,
            ratio_x: f64::from(picture.width) / f64::from(reference.0.max(1)),
            ratio_y: f64::from(picture.height) / f64::from(reference.1.max(1)),
        }
    }

    fn sample(&self, at: &Footprint) -> [f32; 4] {
        let (width, height) = (self.picture.width, self.picture.height);
        let span = |from: f64, to: f64, ratio: f64, limit: u32| {
            let start = ((from * ratio).floor().max(0.) as u32).min(limit - 1);
            let end = ((to * ratio).ceil() as u32).clamp(start + 1, limit);
            (start, end)
        };
        let (x0, x1) = span(at.x0, at.x1, self.ratio_x, width);
        let (y0, y1) = span(at.y0, at.y1, self.ratio_y, height);
        let step_x = ((x1 - x0) / MOST_SAMPLES_ACROSS).max(1);
        let step_y = ((y1 - y0) / MOST_SAMPLES_ACROSS).max(1);
        let mut sum = [0f32; 4];
        let mut count = 0f32;
        for y in (y0..y1).step_by(step_y as usize) {
            for x in (x0..x1).step_by(step_x as usize) {
                let [r, g, b, a] = self.picture.pixel(x, y).map(|channel| f32::from(channel) / 255.);
                sum[0] += r * a;
                sum[1] += g * a;
                sum[2] += b * a;
                sum[3] += a;
                count += 1.;
            }
        }
        sum.map(|channel| channel / count)
    }
}

pub(crate) fn rasterize(layer: &Layer, placement: Placement, surface: Surface) -> Vec<u8> {
    let row = placement.width as usize * 4;
    let mut out = vec![0; row * placement.height as usize];
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let rows_per_band = (placement.height as usize).div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (band, rows) in out.chunks_mut((rows_per_band * row).max(1)).enumerate() {
            let top = (band * rows_per_band) as u32;
            scope.spawn(move || paint_rows(layer, placement, surface, top, rows));
        }
    });
    out
}

fn paint_rows(layer: &Layer, placement: Placement, surface: Surface, top: u32, rows: &mut [u8]) {
    let reference = layer.reference();
    let sources = match layer {
        Layer::Picture(picture) | Layer::Stretched { picture, .. } => (Source::new(picture, reference), None),
        Layer::Onion { before, after, .. } => (
            Source::new(after, reference),
            Some(Source::new(before, reference)),
        ),
        Layer::Difference { after, .. } => (Source::new(after, reference), None),
    };
    let mut pixels = rows.chunks_exact_mut(4);
    for y in top..placement.height {
        for x in 0..placement.width {
            let Some(pixel) = pixels.next() else {
                return;
            };
            let footprint = footprint(&placement, x, y);
            let base = surface.backdrop.color(x, y, &surface);
            let rgb = match layer {
                Layer::Picture(_) | Layer::Stretched { .. } => over(sources.0.sample(&footprint), base),
                Layer::Onion { mix, .. } => {
                    let below = sources
                        .1
                        .as_ref()
                        .map_or(base, |before| over(before.sample(&footprint), base));
                    let above = sources.0.sample(&footprint);
                    over(above.map(|channel| channel * mix), below)
                }
                Layer::Difference {
                    difference, colors, ..
                } => match change_in(difference, &footprint, reference) {
                    Some(change) => colors[change].map(|channel| f32::from(channel) / 255.),
                    None => {
                        let [r, g, b, a] = sources.0.sample(&footprint);
                        let gray = (0.299 * r + 0.587 * g + 0.114 * b) * DIMMED;
                        over([gray, gray, gray, a], base)
                    }
                },
            };
            let [r, g, b] = rgb.map(|channel| (channel.clamp(0., 1.) * 255.).round() as u8);
            pixel.copy_from_slice(&[b, g, r, 255]);
        }
    }
}

fn footprint(placement: &Placement, x: u32, y: u32) -> Footprint {
    let scale = placement.scale.max(f64::EPSILON);
    let (x, y) = (f64::from(x), f64::from(y));
    if scale >= 1. {
        let cx = ((x + 0.5 - placement.left) / scale).floor();
        let cy = ((y + 0.5 - placement.top) / scale).floor();
        Footprint {
            x0: cx,
            x1: cx + 1.,
            y0: cy,
            y1: cy + 1.,
        }
    } else {
        Footprint {
            x0: (x - placement.left) / scale,
            x1: (x + 1. - placement.left) / scale,
            y0: (y - placement.top) / scale,
            y1: (y + 1. - placement.top) / scale,
        }
    }
}

fn change_in(difference: &PixelDifference, at: &Footprint, reference: (u32, u32)) -> Option<usize> {
    let clamp = |value: f64, limit: u32| (value.max(0.) as u32).min(limit);
    let x0 = clamp(at.x0.floor(), reference.0 - 1);
    let y0 = clamp(at.y0.floor(), reference.1 - 1);
    let x1 = clamp(at.x1.ceil(), reference.0).max(x0 + 1);
    let y1 = clamp(at.y1.ceil(), reference.1).max(y0 + 1);
    let step_x = ((x1 - x0) / MOST_SAMPLES_ACROSS).max(1) as usize;
    let step_y = ((y1 - y0) / MOST_SAMPLES_ACROSS).max(1) as usize;
    for y in (y0..y1).step_by(step_y) {
        for x in (x0..x1).step_by(step_x) {
            match difference.at(x, y) {
                PixelChange::Same => {}
                PixelChange::Added => return Some(0),
                PixelChange::Changed => return Some(1),
                PixelChange::Removed => return Some(2),
            }
        }
    }
    None
}

fn over(color: [f32; 4], base: [f32; 3]) -> [f32; 3] {
    let [r, g, b, a] = color;
    [
        r + base[0] * (1. - a),
        g + base[1] * (1. - a),
        b + base[2] * (1. - a),
    ]
}

#[cfg(test)]
mod tests {
    use super::{Backdrop, Layer, Placement, Surface, rasterize};
    use crate::media::{Picture, PixelDifference};

    const PANEL: Surface = Surface {
        backdrop: Backdrop::Black,
        panel: [10, 10, 10],
        checker: 8,
    };

    fn gray(level: u8) -> [u8; 4] {
        [level, level, level, 255]
    }

    fn picture(width: u32, pixels: &[[u8; 4]]) -> Picture {
        let height = pixels.len() as u32 / width;
        Picture::from_rgba(width, height, pixels.concat())
    }

    fn red_channel(bgra: &[u8]) -> Vec<u8> {
        bgra.chunks(4).map(|pixel| pixel[2]).collect()
    }

    fn at(left: f64, top: f64, scale: f64, width: u32, height: u32) -> Placement {
        Placement {
            left,
            top,
            scale,
            width,
            height,
        }
    }

    #[test]
    fn zooming_in_keeps_each_pixel_square() {
        let picture = picture(2, &[gray(0), gray(255), gray(100), gray(50)]);
        let out = rasterize(&Layer::Picture(&picture), at(0., 0., 2., 4, 4), PANEL);
        assert_eq!(
            red_channel(&out),
            [0, 0, 255, 255, 0, 0, 255, 255, 100, 100, 50, 50, 100, 100, 50, 50]
        );
    }

    #[test]
    fn zooming_out_averages_the_pixels_behind_each_one() {
        let picture = picture(2, &[gray(0), gray(200), gray(200), gray(0)]);
        let out = rasterize(&Layer::Picture(&picture), at(0., 0., 0.5, 1, 1), PANEL);
        assert_eq!(red_channel(&out), [100]);
    }

    #[test]
    fn transparent_pixels_show_the_backdrop() {
        let picture = picture(1, &[[255, 0, 0, 0]]);
        let white = Surface {
            backdrop: Backdrop::White,
            ..PANEL
        };
        let out = rasterize(&Layer::Picture(&picture), at(0., 0., 1., 1, 1), white);
        assert_eq!(out, [255, 255, 255, 255]);
    }

    #[test]
    fn a_stretched_picture_fills_the_frame_of_the_other() {
        let before = picture(1, &[gray(10)]);
        let after = picture(2, &[gray(200), gray(200)]);
        let layer = Layer::Stretched {
            picture: &before,
            onto: &after,
        };
        assert_eq!(red_channel(&rasterize(&layer, at(0., 0., 1., 2, 1), PANEL)), [10, 10]);
    }

    #[test]
    fn every_row_lands_where_it_belongs() {
        let clear = picture(1, &[[0, 0, 0, 0]]);
        let checkerboard = Surface {
            backdrop: Backdrop::Checkerboard,
            checker: 1,
            ..PANEL
        };
        let (width, height) = (3, 37);
        let out = rasterize(&Layer::Picture(&clear), at(0., 0., 1., width, height), checkerboard);
        let expected: Vec<u8> = (0..height)
            .flat_map(|y| (0..width).map(move |x| if (x + y) % 2 == 0 { 0x1a } else { 0x26 }))
            .collect();
        assert_eq!(red_channel(&out), expected);
    }

    #[test]
    fn the_difference_paints_only_what_changed() {
        let before = picture(2, &[gray(10), gray(10)]);
        let after = picture(2, &[gray(10), gray(200)]);
        let difference = PixelDifference::between(&before, &after);
        let layer = Layer::Difference {
            after: &after,
            difference: &difference,
            colors: [[0, 255, 0], [250, 200, 0], [255, 0, 0]],
        };
        let out = rasterize(&layer, at(0., 0., 1., 2, 1), PANEL);
        assert_eq!(red_channel(&out), [4, 250]);
    }
}
