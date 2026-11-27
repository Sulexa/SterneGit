use std::collections::HashMap;

use super::{
    compare::State,
    value::{Value, short_number},
};

const SAME: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CurveKey {
    pub(crate) time: f64,
    pub(crate) value: f64,
    pub(crate) in_slope: f64,
    pub(crate) out_slope: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ObjectKey {
    pub(crate) time: f64,
    pub(crate) value: Value,
    pub(crate) picture: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Keys {
    Curve(Vec<CurveKey>),
    Objects(Vec<ObjectKey>),
}

impl Keys {
    fn same_as(&self, other: &Keys) -> bool {
        match (self, other) {
            (Keys::Curve(a), Keys::Curve(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b).all(|(x, y)| {
                        close(x.time, y.time) && close(x.value, y.value) && close(x.in_slope, y.in_slope) && close(x.out_slope, y.out_slope)
                    })
            }
            (Keys::Objects(a), Keys::Objects(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| close(x.time, y.time) && x.value == y.value)
            }
            _ => false,
        }
    }

    pub(crate) fn range(&self) -> Option<(f64, f64)> {
        let Keys::Curve(keys) = self else {
            return None;
        };
        let values = keys.iter().map(|key| key.value).filter(|value| value.is_finite());
        let low = values.clone().fold(f64::INFINITY, f64::min);
        let high = values.fold(f64::NEG_INFINITY, f64::max);
        low.is_finite().then_some((low, high))
    }
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= SAME * a.abs().max(b.abs()).max(1.) || a.is_infinite() && b.is_infinite() && a.signum() == b.signum()
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Track {
    pub(crate) id: String,
    pub(crate) group: String,
    pub(crate) name: String,
    pub(crate) keys: Keys,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Clip {
    pub(crate) tracks: Vec<Track>,
    pub(crate) events: Vec<(f64, String)>,
    pub(crate) length: f64,
    pub(crate) sample_rate: f64,
    pub(crate) settings: Vec<(String, String)>,
}

pub(crate) fn clock(seconds: f64, rate: f64) -> String {
    let frames = (seconds * rate).round() as i64;
    let rate = rate.round().max(1.) as i64;
    format!("{}:{:02} · {} s", frames / rate, frames % rate, short_number(seconds))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TrackChange {
    pub(crate) group: String,
    pub(crate) name: String,
    pub(crate) state: State,
    pub(crate) before: Option<Keys>,
    pub(crate) after: Option<Keys>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ClipComparison {
    pub(crate) tracks: Vec<TrackChange>,
    pub(crate) events: [Vec<(f64, String)>; 2],
    pub(crate) events_changed: bool,
    pub(crate) lengths: [f64; 2],
    pub(crate) sample_rate: f64,
    pub(crate) settings: [Option<Vec<(String, String)>>; 2],
    pub(crate) changes: usize,
}

pub(crate) fn compare_clips(before: Option<&Clip>, after: Option<&Clip>, unchanged: bool) -> ClipComparison {
    let empty = Clip::default();
    let old = before.unwrap_or(&empty);
    let new = after.unwrap_or(&empty);
    let old_tracks: HashMap<&str, &Track> = old.tracks.iter().map(|track| (track.id.as_str(), track)).collect();
    let mut tracks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for track in &new.tracks {
        seen.insert(track.id.as_str());
        let previous = old_tracks.get(track.id.as_str()).copied();
        let state = match previous {
            None => State::Added,
            Some(previous) if previous.keys.same_as(&track.keys) => State::Same,
            Some(_) => State::Changed,
        };
        tracks.push(TrackChange {
            group: track.group.clone(),
            name: track.name.clone(),
            state,
            before: previous.map(|previous| previous.keys.clone()),
            after: Some(track.keys.clone()),
        });
    }
    for track in &old.tracks {
        if !seen.contains(track.id.as_str()) {
            tracks.push(TrackChange {
                group: track.group.clone(),
                name: track.name.clone(),
                state: State::Removed,
                before: Some(track.keys.clone()),
                after: None,
            });
        }
    }
    let changed_tracks = tracks.iter().filter(|track| track.state != State::Same).count();
    if !unchanged && changed_tracks > 0 {
        tracks.retain(|track| track.state != State::Same);
    }
    let events_changed = old.events.len() != new.events.len()
        || old.events.iter().zip(&new.events).any(|(a, b)| !close(a.0, b.0) || a.1 != b.1);
    let settings_changed = old.settings != new.settings && before.is_some() && after.is_some();
    ClipComparison {
        tracks,
        events: [old.events.clone(), new.events.clone()],
        events_changed,
        lengths: [before.map_or(0., |clip| clip.length), after.map_or(0., |clip| clip.length)],
        sample_rate: after.or(before).map_or(60., |clip| clip.sample_rate),
        settings: [before.map(|clip| clip.settings.clone()), after.map(|clip| clip.settings.clone())],
        changes: changed_tracks + usize::from(events_changed) + usize::from(settings_changed && changed_tracks == 0),
    }
}

pub(crate) fn hermite(from: &CurveKey, to: &CurveKey, time: f64) -> f64 {
    if !from.out_slope.is_finite() || !to.in_slope.is_finite() {
        return from.value;
    }
    let span = to.time - from.time;
    if span <= 0. {
        return to.value;
    }
    let t = ((time - from.time) / span).clamp(0., 1.);
    let (t2, t3) = (t * t, t * t * t);
    let h00 = 2. * t3 - 3. * t2 + 1.;
    let h10 = t3 - 2. * t2 + t;
    let h01 = -2. * t3 + 3. * t2;
    let h11 = t3 - t2;
    h00 * from.value + h10 * span * from.out_slope + h01 * to.value + h11 * span * to.in_slope
}

pub(crate) fn value_at(keys: &[CurveKey], time: f64) -> Option<f64> {
    let first = keys.first()?;
    if time <= first.time {
        return Some(first.value);
    }
    for pair in keys.windows(2) {
        if time <= pair[1].time {
            return Some(hermite(&pair[0], &pair[1], time));
        }
    }
    keys.last().map(|key| key.value)
}

#[cfg(test)]
mod tests {
    use super::{Clip, CurveKey, clock, compare_clips, value_at};
    use crate::unity::{
        compare::State,
        ingest::{clip as ingest_clip, tests::read_text},
    };

    fn read_clip(text: &str) -> Option<Clip> {
        read_text("Hop.anim", text).curves.as_ref().map(ingest_clip)
    }

    fn clip(peak: &str, stop: &str, with_alpha: bool) -> String {
        let alpha = if with_alpha {
            "  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: 1
        inSlope: 0
        outSlope: 0
    attribute: m_Color.a
    path:
    classID: 212
    script: {fileID: 0}
"
        } else {
            ""
        };
        format!(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!74 &7400000
AnimationClip:
  m_Name: SlimeJump
  m_SampleRate: 60
  m_EditorCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: 0
        inSlope: 0
        outSlope: 0
      - serializedVersion: 3
        time: 0.25
        value: {peak}
        inSlope: 0
        outSlope: 0
    attribute: m_LocalPosition.y
    path:
    classID: 4
    script: {{fileID: 0}}
{alpha}  m_AnimationClipSettings:
    m_StartTime: 0
    m_StopTime: {stop}
    m_LoopTime: 0
  m_Events:
  - time: 0.45
    functionName: OnLand
"
        )
    }

    #[test]
    fn a_changed_curve_and_an_added_one_are_both_listed() {
        let before = read_clip(&clip("0.8", "0.5", false));
        let after = read_clip(&clip("1.1", "0.5833333", true));
        let comparison = compare_clips(before.as_ref(), after.as_ref(), false);
        let tracks: Vec<(&str, State)> = comparison.tracks.iter().map(|track| (track.name.as_str(), track.state)).collect();
        assert_eq!(tracks, vec![("Position.y", State::Changed), ("Color.a", State::Added)]);
        assert_eq!(comparison.tracks[0].group, "Root : Transform");
        assert!(!comparison.events_changed);
        assert_eq!(comparison.changes, 2);
    }

    #[test]
    fn the_curve_passes_through_its_keys_and_eases_between_them() {
        let keys = [
            CurveKey { time: 0., value: 0., in_slope: 0., out_slope: 0. },
            CurveKey { time: 1., value: 1., in_slope: 0., out_slope: 0. },
        ];
        assert_eq!(value_at(&keys, 0.), Some(0.));
        assert_eq!(value_at(&keys, 1.), Some(1.));
        assert!((value_at(&keys, 0.5).unwrap_or(0.) - 0.5).abs() < 1e-9);
        assert!(value_at(&keys, 0.25).unwrap_or(1.) < 0.25);
    }

    #[test]
    fn times_read_as_seconds_and_frames() {
        assert_eq!(clock(0.5, 60.), "0:30 · 0.5 s");
    }
}
