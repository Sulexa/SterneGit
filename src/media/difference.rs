use std::collections::VecDeque;

use super::Picture;

const MOST_AREAS: usize = 99;
const CELLS_ACROSS: u32 = 48;
const SMALLEST_CELL: u32 = 4;
const LOSSY_NOISE: u8 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelChange {
    Same,
    Added,
    Changed,
    Removed,
}

impl PixelChange {
    fn between(before: [u8; 4], after: [u8; 4], noise: u8) -> Self {
        let close = before
            .iter()
            .zip(&after)
            .all(|(was, is)| was.abs_diff(*is) <= noise);
        match (before[3], after[3]) {
            (0, 0) => PixelChange::Same,
            (0, _) => PixelChange::Added,
            (_, 0) => PixelChange::Removed,
            _ if close => PixelChange::Same,
            _ => PixelChange::Changed,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChangeCounts {
    pub(crate) added: usize,
    pub(crate) changed: usize,
    pub(crate) removed: usize,
}

impl ChangeCounts {
    pub(crate) fn total(self) -> usize {
        self.added + self.changed + self.removed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Area {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Area {
    fn of_pixel(x: u32, y: u32) -> Self {
        Self {
            x,
            y,
            width: 1,
            height: 1,
        }
    }

    fn joined(self, other: Area) -> Area {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Area {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

pub(crate) struct PixelDifference {
    pub(crate) width: u32,
    pub(crate) height: u32,
    changes: Vec<PixelChange>,
    pub(crate) counts: ChangeCounts,
    pub(crate) areas: Vec<Area>,
}

impl PixelDifference {
    pub(crate) fn between(before: &Picture, after: &Picture) -> Self {
        let (width, height) = (after.width, after.height);
        let mut changes = Vec::with_capacity((width * height) as usize);
        let mut counts = ChangeCounts::default();
        let noise = if before.lossy || after.lossy { LOSSY_NOISE } else { 0 };
        for y in 0..height {
            let before_y = scaled(y, height, before.height);
            for x in 0..width {
                let before_x = scaled(x, width, before.width);
                let change = PixelChange::between(before.pixel(before_x, before_y), after.pixel(x, y), noise);
                match change {
                    PixelChange::Same => {}
                    PixelChange::Added => counts.added += 1,
                    PixelChange::Changed => counts.changed += 1,
                    PixelChange::Removed => counts.removed += 1,
                }
                changes.push(change);
            }
        }
        let areas = areas(&changes, width, height);
        Self {
            width,
            height,
            changes,
            counts,
            areas,
        }
    }

    pub(crate) fn at(&self, x: u32, y: u32) -> PixelChange {
        self.changes[(y * self.width + x) as usize]
    }

    pub(crate) fn share(&self) -> f64 {
        let pixels = (self.width as usize * self.height as usize).max(1);
        self.counts.total() as f64 / pixels as f64
    }
}

fn scaled(at: u32, from: u32, to: u32) -> u32 {
    if from == to {
        at
    } else {
        ((u64::from(at) * u64::from(to)) / u64::from(from.max(1))).min(u64::from(to.saturating_sub(1))) as u32
    }
}

fn areas(changes: &[PixelChange], width: u32, height: u32) -> Vec<Area> {
    let cell = (width.max(height) / CELLS_ACROSS).max(SMALLEST_CELL);
    let across = width.div_ceil(cell) as usize;
    let down = height.div_ceil(cell) as usize;
    let mut boxes: Vec<Option<Area>> = vec![None; across * down];
    for y in 0..height {
        for x in 0..width {
            if changes[(y * width + x) as usize] == PixelChange::Same {
                continue;
            }
            let at = (y / cell) as usize * across + (x / cell) as usize;
            let pixel = Area::of_pixel(x, y);
            boxes[at] = Some(boxes[at].map_or(pixel, |area| area.joined(pixel)));
        }
    }
    let mut seen = vec![false; boxes.len()];
    let mut found = Vec::new();
    for start in 0..boxes.len() {
        if seen[start] || boxes[start].is_none() {
            continue;
        }
        seen[start] = true;
        let mut area = boxes[start].expect("only changed cells start an area");
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            let (column, row) = (at % across, at / across);
            let neighbours = [
                (column > 0).then(|| at - 1),
                (column + 1 < across).then(|| at + 1),
                (row > 0).then(|| at - across),
                (row + 1 < down).then(|| at + across),
            ];
            for next in neighbours.into_iter().flatten() {
                if let (false, Some(next_box)) = (seen[next], boxes[next]) {
                    seen[next] = true;
                    area = area.joined(next_box);
                    queue.push_back(next);
                }
            }
        }
        found.push(area);
    }
    found.sort_by_key(|area| (area.y, area.x));
    found.truncate(MOST_AREAS);
    found
}

#[cfg(test)]
mod tests {
    use super::{Area, PixelChange, PixelDifference};
    use crate::media::Picture;

    const CLEAR: [u8; 4] = [0, 0, 0, 0];
    const GREEN: [u8; 4] = [0, 200, 0, 255];
    const RED: [u8; 4] = [200, 0, 0, 255];

    fn picture(width: u32, height: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> Picture {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend(paint(x, y));
            }
        }
        Picture::from_rgba(width, height, pixels)
    }

    #[test]
    fn each_pixel_says_whether_it_appeared_changed_or_went_transparent() {
        let before = picture(3, 1, |x, _| [CLEAR, GREEN, GREEN][x as usize]);
        let after = picture(3, 1, |x, _| [GREEN, RED, CLEAR][x as usize]);
        let difference = PixelDifference::between(&before, &after);
        assert_eq!(difference.at(0, 0), PixelChange::Added);
        assert_eq!(difference.at(1, 0), PixelChange::Changed);
        assert_eq!(difference.at(2, 0), PixelChange::Removed);
        assert_eq!(difference.counts.total(), 3);
    }

    #[test]
    fn far_apart_changes_make_separate_areas_in_reading_order() {
        let before = picture(64, 64, |_, _| GREEN);
        let after = picture(64, 64, |x, y| {
            let crown = (40..44).contains(&x) && (2..5).contains(&y);
            let mouth = (10..14).contains(&x) && (50..52).contains(&y);
            if crown || mouth { RED } else { GREEN }
        });
        let difference = PixelDifference::between(&before, &after);
        assert_eq!(
            difference.areas,
            [
                Area { x: 40, y: 2, width: 4, height: 3 },
                Area { x: 10, y: 50, width: 4, height: 2 },
            ]
        );
    }

    #[test]
    fn compression_noise_in_a_lossy_picture_is_not_a_change() {
        let before = picture(2, 1, |_, _| [100, 100, 100, 255]);
        let mut after = picture(2, 1, |x, _| if x == 0 { [110, 95, 104, 255] } else { [200, 100, 100, 255] });
        assert_eq!(PixelDifference::between(&before, &after).counts.total(), 2);
        after.lossy = true;
        let difference = PixelDifference::between(&before, &after);
        assert_eq!(difference.at(0, 0), PixelChange::Same);
        assert_eq!(difference.at(1, 0), PixelChange::Changed);
    }

    #[test]
    fn identical_pictures_have_no_areas() {
        let before = picture(8, 8, |_, _| GREEN);
        let difference = PixelDifference::between(&before, &picture(8, 8, |_, _| GREEN));
        assert!(difference.areas.is_empty());
        assert_eq!(difference.share(), 0.0);
    }

    #[test]
    fn a_resized_picture_is_compared_at_the_new_size() {
        let before = picture(2, 2, |x, _| if x == 0 { GREEN } else { RED });
        let after = picture(4, 4, |x, _| if x < 2 { GREEN } else { RED });
        let difference = PixelDifference::between(&before, &after);
        assert_eq!((difference.width, difference.height), (4, 4));
        assert_eq!(difference.counts.total(), 0);
    }
}
