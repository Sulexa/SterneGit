use super::{Sound, sound::decibels};

const WINDOW: f64 = 0.1;
const BANDS: usize = 3;
const LOW_CUTOFF: f64 = 300.;
const HIGH_CUTOFF: f64 = 3000.;
const AUDIBLE_DIFFERENCE_DB: f64 = 2.;
const QUIETEST_HEARD_DB: f64 = -60.;
const BRIDGED_GAP: usize = 2;
const SHORTEST_REGION: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Region {
    pub(crate) from: f64,
    pub(crate) to: f64,
}

pub(crate) fn audible_changes(before: &Sound, after: &Sound) -> Vec<Region> {
    let shared = before.duration().min(after.duration());
    let longest = before.duration().max(after.duration());
    let windows = (shared / WINDOW).ceil() as usize;
    let identical_layout = before.rate == after.rate && before.channels == after.channels;
    let bands_before = band_levels(before, windows);
    let bands_after = band_levels(after, windows);
    let changed: Vec<bool> = (0..windows)
        .map(|window| {
            if identical_layout && same_samples(before, after, window) {
                return false;
            }
            bands_before[window]
                .iter()
                .zip(&bands_after[window])
                .any(|(was, is)| {
                    was.max(*is) > QUIETEST_HEARD_DB && (was - is).abs() > AUDIBLE_DIFFERENCE_DB
                })
        })
        .collect();
    let mut regions = merged(&changed);
    if longest - shared > WINDOW / 2. {
        match regions.last_mut() {
            Some(last) if shared - last.to <= WINDOW * BRIDGED_GAP as f64 => last.to = longest,
            _ => regions.push(Region {
                from: shared,
                to: longest,
            }),
        }
    }
    regions
}

fn merged(changed: &[bool]) -> Vec<Region> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (window, &is_changed) in changed.iter().enumerate() {
        if !is_changed {
            continue;
        }
        match runs.last_mut() {
            Some(run) if window - run.1 <= BRIDGED_GAP + 1 => run.1 = window,
            _ => runs.push((window, window)),
        }
    }
    runs.into_iter()
        .filter(|(first, last)| last - first + 1 >= SHORTEST_REGION)
        .map(|(first, last)| Region {
            from: first as f64 * WINDOW,
            to: (last + 1) as f64 * WINDOW,
        })
        .collect()
}

fn same_samples(before: &Sound, after: &Sound, window: usize) -> bool {
    let first = before.frame_at(window as f64 * WINDOW) * before.channels;
    let last = before.frame_at((window + 1) as f64 * WINDOW) * before.channels;
    let end = last.min(before.samples.len()).min(after.samples.len());
    first < end && before.samples[first..end] == after.samples[first..end]
}

fn band_levels(sound: &Sound, windows: usize) -> Vec<[f64; BANDS]> {
    let rate = f64::from(sound.rate);
    let smoothing = |cutoff: f64| 1. - (-2. * std::f64::consts::PI * cutoff / rate).exp();
    let (low_step, high_step) = (smoothing(LOW_CUTOFF), smoothing(HIGH_CUTOFF));
    let (mut low, mut below_high) = (0f64, 0f64);
    let frames_per_window = (WINDOW * rate).max(1.) as usize;
    let mut levels = vec![[f64::NEG_INFINITY; BANDS]; windows];
    let mut energy = [0f64; BANDS];
    let mut count = 0usize;
    let frames = sound.frames().min(windows * frames_per_window);
    for frame in 0..frames {
        let value = f64::from(sound.mono(frame));
        low += low_step * (value - low);
        below_high += high_step * (value - below_high);
        let bands = [low, below_high - low, value - below_high];
        for (sum, band) in energy.iter_mut().zip(bands) {
            *sum += band * band;
        }
        count += 1;
        if count == frames_per_window || frame + 1 == frames {
            let window = frame / frames_per_window;
            if let Some(level) = levels.get_mut(window) {
                *level = energy.map(|sum| decibels((sum / count as f64).sqrt()));
            }
            energy = [0.; BANDS];
            count = 0;
        }
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::{Region, audible_changes};
    use crate::media::Sound;

    const RATE: u32 = 8_000;

    fn sound(seconds: f64, loudness_at: impl Fn(f64) -> f64) -> Sound {
        let frames = (f64::from(RATE) * seconds) as usize;
        let samples = (0..frames)
            .map(|frame| {
                let t = frame as f64 / f64::from(RATE);
                (loudness_at(t) * (2. * std::f64::consts::PI * 440. * t).sin() * 32767.) as i16
            })
            .collect();
        Sound::from_pcm(RATE, 1, samples)
    }

    fn rounded(regions: &[Region]) -> Vec<(f64, f64)> {
        regions
            .iter()
            .map(|region| ((region.from * 10.).round() / 10., (region.to * 10.).round() / 10.))
            .collect()
    }

    #[test]
    fn the_same_sound_has_nothing_to_hear() {
        let a = sound(3., |_| 0.5);
        let b = sound(3., |_| 0.5);
        assert!(audible_changes(&a, &b).is_empty());
    }

    #[test]
    fn a_louder_stretch_is_marked_where_it_plays() {
        let a = sound(4., |_| 0.2);
        let b = sound(4., |t| if (1.0..2.0).contains(&t) { 0.8 } else { 0.2 });
        assert_eq!(rounded(&audible_changes(&a, &b)), [(1.0, 2.0)]);
    }

    #[test]
    fn the_end_of_the_longer_version_is_a_change() {
        let a = sound(2., |_| 0.3);
        let b = sound(3., |_| 0.3);
        assert_eq!(rounded(&audible_changes(&a, &b)), [(2.0, 3.0)]);
    }

    #[test]
    fn a_tiny_level_change_is_not_heard() {
        let a = sound(2., |_| 0.5);
        let b = sound(2., |_| 0.5 * 1.05);
        assert!(audible_changes(&a, &b).is_empty());
    }
}
