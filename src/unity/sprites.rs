use sternegit_reader_protocol::Picture;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpriteImage {
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pivot: (f64, f64),
    pub(crate) pixels_per_unit: f64,
    pub(crate) border: [f64; 4],
}

impl SpriteImage {
    pub(crate) fn size_in_units(&self) -> (f64, f64) {
        (f64::from(self.width) / self.pixels_per_unit, f64::from(self.height) / self.pixels_per_unit)
    }
}

pub(crate) fn crop(rgba: &[u8], width: u32, height: u32, picture: &Picture) -> Option<SpriteImage> {
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let [x, y, rect_width, rect_height] = picture
        .rect
        .map_or([0, 0, width, height], |rect| rect.map(|side| side.max(0.) as u32));
    let right = x.saturating_add(rect_width).min(width);
    let top_from_bottom = y.saturating_add(rect_height).min(height);
    if right <= x || top_from_bottom <= y {
        return None;
    }
    let crop_width = right - x;
    let crop_height = top_from_bottom - y;
    let first_row = height - top_from_bottom;
    let mut pixels = Vec::with_capacity((crop_width * crop_height * 4) as usize);
    for row in first_row..first_row + crop_height {
        let start = ((row * width + x) * 4) as usize;
        pixels.extend_from_slice(&rgba[start..start + (crop_width * 4) as usize]);
    }
    Some(SpriteImage {
        rgba: pixels,
        width: crop_width,
        height: crop_height,
        pivot: picture.pivot,
        pixels_per_unit: picture.pixels_per_unit,
        border: picture.border,
    })
}

#[cfg(test)]
mod tests {
    use sternegit_reader_protocol::Picture;

    use super::crop;

    fn pixels(width: u32, height: u32) -> Vec<u8> {
        (0..width * height).flat_map(|ix| [ix as u8, 0, 0, 255]).collect()
    }

    fn picture(rect: Option<[f64; 4]>) -> Picture {
        Picture {
            id: "slime:21300000".into(),
            path: "Slime.png".into(),
            rect,
            pivot: (0.5, 0.),
            pixels_per_unit: 16.,
            border: [0.; 4],
        }
    }

    #[test]
    fn a_picture_without_a_rect_is_the_whole_image_at_its_pixels_per_unit() {
        let sprite = crop(&pixels(4, 2), 4, 2, &picture(None)).expect("whole picture");
        assert_eq!((sprite.width, sprite.height), (4, 2));
        assert_eq!(sprite.pivot, (0.5, 0.));
        assert_eq!(sprite.size_in_units(), (0.25, 0.125));
    }

    #[test]
    fn a_rect_is_cut_counting_rows_from_the_bottom() {
        let sprite = crop(&pixels(4, 2), 4, 2, &picture(Some([2., 1., 2., 1.]))).expect("one cell");
        assert_eq!((sprite.width, sprite.height), (2, 1));
        assert_eq!(sprite.rgba[0], 2);
    }
}
