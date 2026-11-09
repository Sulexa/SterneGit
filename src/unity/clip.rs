use std::collections::HashMap;

use super::{
    compare::State,
    names::nicify,
    value::{Lookup, Value, short_number},
    yaml::{Document, Node},
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
    pub(crate) sprite: Option<(String, i64)>,
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

    pub(crate) fn last_time(&self) -> f64 {
        match self {
            Keys::Curve(keys) => keys.last().map_or(0., |key| key.time),
            Keys::Objects(keys) => keys.last().map_or(0., |key| key.time),
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

pub(crate) fn read_clip(documents: &[Document], lookup: &Lookup) -> Option<Clip> {
    let body = &documents.iter().find(|document| document.type_name == "AnimationClip")?.body;
    let mut tracks = Vec::new();
    let editor = body.get("m_EditorCurves").map(Node::items).unwrap_or_default();
    if editor.is_empty() {
        tracks.extend(float_tracks(body.get("m_FloatCurves").map(Node::items).unwrap_or_default()));
        for (source, attribute) in [
            ("m_PositionCurves", "m_LocalPosition"),
            ("m_EulerCurves", "localEulerAnglesRaw"),
            ("m_ScaleCurves", "m_LocalScale"),
        ] {
            tracks.extend(vector_tracks(body.get(source).map(Node::items).unwrap_or_default(), attribute));
        }
    } else {
        tracks.extend(float_tracks(editor));
    }
    tracks.extend(object_tracks(body.get("m_PPtrCurves").map(Node::items).unwrap_or_default(), lookup));
    let events = body
        .get("m_Events")
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .map(|event| {
            let time = event.get("time").and_then(Node::number).unwrap_or(0.);
            let name = event.get("functionName").and_then(Node::text).unwrap_or_default().to_owned();
            (time, name)
        })
        .collect();
    let settings_node = body.get("m_AnimationClipSettings");
    let start = settings_node.and_then(|settings| settings.get("m_StartTime")).and_then(Node::number).unwrap_or(0.);
    let stop = settings_node.and_then(|settings| settings.get("m_StopTime")).and_then(Node::number);
    let last_key = tracks.iter().map(|track| track.keys.last_time()).fold(0., f64::max);
    let length = stop.map_or(last_key, |stop| (stop - start).max(0.));
    let sample_rate = body.get("m_SampleRate").and_then(Node::number).unwrap_or(60.);
    let flag = |key: &str| {
        settings_node
            .and_then(|settings| settings.get(key))
            .and_then(Node::text)
            .map(|text| if text == "1" { "On" } else { "Off" }.to_owned())
            .unwrap_or_else(|| "Off".to_owned())
    };
    let settings = vec![
        ("Length".to_owned(), clock(length, sample_rate)),
        ("Sample rate".to_owned(), short_number(sample_rate)),
        ("Loop time".to_owned(), flag("m_LoopTime")),
        ("Curves".to_owned(), tracks.len().to_string()),
        ("Events".to_owned(), events_len(body).to_string()),
    ];
    Some(Clip {
        tracks,
        events,
        length,
        sample_rate,
        settings,
    })
}

fn events_len(body: &Node) -> usize {
    body.get("m_Events").map_or(0, |events| events.items().len())
}

pub(crate) fn clock(seconds: f64, rate: f64) -> String {
    let frames = (seconds * rate).round() as i64;
    let rate = rate.round().max(1.) as i64;
    format!("{}:{:02} · {} s", frames / rate, frames % rate, short_number(seconds))
}

fn curve_keys(curve: &Node) -> Vec<CurveKey> {
    curve
        .get("m_Curve")
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .map(|key| {
            let number = |name: &str| key.get(name).and_then(Node::number).unwrap_or(0.);
            CurveKey {
                time: number("time"),
                value: number("value"),
                in_slope: number("inSlope"),
                out_slope: number("outSlope"),
            }
        })
        .collect()
}

fn float_tracks(curves: &[Node]) -> Vec<Track> {
    curves
        .iter()
        .filter_map(|entry| {
            let attribute = entry.get("attribute").and_then(Node::text)?;
            let path = entry.get("path").and_then(Node::text).unwrap_or_default();
            let class_id = entry.get("classID").and_then(Node::number).unwrap_or(0.) as u32;
            Some(Track {
                id: format!("{path}|{class_id}|{attribute}"),
                group: group_label(path, class_id),
                name: attribute_label(attribute),
                keys: Keys::Curve(curve_keys(entry.get("curve")?)),
            })
        })
        .collect()
}

fn vector_tracks(curves: &[Node], attribute: &str) -> Vec<Track> {
    let mut out = Vec::new();
    for entry in curves {
        let path = entry.get("path").and_then(Node::text).unwrap_or_default();
        let keys = entry
            .get("curve")
            .and_then(|curve| curve.get("m_Curve"))
            .map(Node::items)
            .unwrap_or_default();
        for axis in ["x", "y", "z"] {
            let axis_keys = keys
                .iter()
                .map(|key| {
                    let part = |name: &str| key.get(name).and_then(|value| value.get(axis)).and_then(Node::number).unwrap_or(0.);
                    CurveKey {
                        time: key.get("time").and_then(Node::number).unwrap_or(0.),
                        value: part("value"),
                        in_slope: part("inSlope"),
                        out_slope: part("outSlope"),
                    }
                })
                .collect();
            let full = format!("{attribute}.{axis}");
            out.push(Track {
                id: format!("{path}|4|{full}"),
                group: group_label(path, 4),
                name: attribute_label(&full),
                keys: Keys::Curve(axis_keys),
            });
        }
    }
    out
}

fn object_tracks(curves: &[Node], lookup: &Lookup) -> Vec<Track> {
    curves
        .iter()
        .filter_map(|entry| {
            let attribute = entry.get("attribute").and_then(Node::text)?;
            let path = entry.get("path").and_then(Node::text).unwrap_or_default();
            let class_id = entry.get("classID").and_then(Node::number).unwrap_or(0.) as u32;
            let keys = entry
                .get("curve")
                .map(Node::items)
                .unwrap_or_default()
                .iter()
                .map(|key| {
                    let reference = key.get("value");
                    let sprite = reference.and_then(|reference| {
                        let guid = reference.get("guid")?.text()?.to_owned();
                        let file_id = reference.get("fileID")?.integer()?;
                        Some((guid, file_id))
                    });
                    ObjectKey {
                        time: key.get("time").and_then(Node::number).unwrap_or(0.),
                        value: reference.and_then(|reference| lookup.reference(reference)).unwrap_or(Value::Nothing("Object".to_owned())),
                        sprite,
                    }
                })
                .collect();
            Some(Track {
                id: format!("{path}|{class_id}|{attribute}"),
                group: group_label(path, class_id),
                name: attribute_label(attribute),
                keys: Keys::Objects(keys),
            })
        })
        .collect()
}

pub(crate) fn class_title(class_id: u32) -> &'static str {
    match class_id {
        1 => "GameObject",
        4 => "Transform",
        20 => "Camera",
        23 => "Mesh Renderer",
        50 => "Rigidbody 2D",
        58 => "Circle Collider 2D",
        61 => "Box Collider 2D",
        82 => "Audio Source",
        95 => "Animator",
        108 => "Light",
        114 => "Script",
        137 => "Skinned Mesh Renderer",
        198 => "Particle System",
        212 => "Sprite Renderer",
        224 => "Rect Transform",
        225 => "Canvas Group",
        _ => "Component",
    }
}

fn group_label(path: &str, class_id: u32) -> String {
    let object = if path.is_empty() { "Root" } else { path.rsplit('/').next().unwrap_or(path) };
    format!("{object} : {}", class_title(class_id))
}

pub(crate) fn attribute_label(attribute: &str) -> String {
    let (base, suffix) = match attribute.rsplit_once('.') {
        Some((base, suffix)) if suffix.len() == 1 => (base, Some(suffix)),
        _ => (attribute, None),
    };
    let base = match base {
        "m_LocalPosition" => "Position".to_owned(),
        "localEulerAnglesRaw" | "localEulerAngles" | "localEulerAnglesBaked" => "Rotation".to_owned(),
        "m_LocalRotation" => "Rotation (quaternion)".to_owned(),
        "m_LocalScale" => "Scale".to_owned(),
        "m_IsActive" => "Active".to_owned(),
        _ => nicify(base),
    };
    match suffix {
        Some(suffix) => format!("{base}.{suffix}"),
        None => base,
    }
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
    use super::{CurveKey, attribute_label, clock, compare_clips, read_clip, value_at};
    use crate::unity::{compare::State, value::Lookup, yaml::documents};

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
        let before = read_clip(&documents(&clip("0.8", "0.5", false)), &Lookup::default());
        let after = read_clip(&documents(&clip("1.1", "0.5833333", true)), &Lookup::default());
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
        assert_eq!(attribute_label("localEulerAnglesRaw.z"), "Rotation.z");
        assert_eq!(attribute_label("m_Sprite"), "Sprite");
    }
}
