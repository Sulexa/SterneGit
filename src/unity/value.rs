use crate::media::file_size;

const SAME_NUMBER: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph {
    Object,
    Asset,
    Script,
    Image,
    Sound,
    Material,
    Prefab,
    Clip,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reference {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) glyph: Glyph,
    pub(crate) guid: Option<String>,
    pub(crate) missing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Digest(u64);

impl Digest {
    pub(crate) fn from_value(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Text(String),
    Long { start: String, more: usize, digest: Digest },
    Data { bytes: usize, digest: Digest },
    Number(String),
    Flag(bool),
    Vector(Vec<(&'static str, String)>),
    Color { rgba: [f32; 4], hdr: Option<f32> },
    Reference(Reference),
    Nothing(String),
    Count(usize),
}

impl Value {
    pub(crate) fn same_as(&self, other: &Value) -> Sameness {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => numbers(a, b),
            (Value::Vector(a), Value::Vector(b)) if a.len() == b.len() => a
                .iter()
                .zip(b)
                .map(|((axis, x), (other_axis, y))| if axis == other_axis { numbers(x, y) } else { Sameness::Different })
                .fold(Sameness::Equal, Sameness::and),
            (Value::Color { rgba: a, .. }, Value::Color { rgba: b, .. }) => a
                .iter()
                .zip(b)
                .map(|(x, y)| float_sameness(f64::from(*x), f64::from(*y)))
                .fold(Sameness::Equal, Sameness::and),
            (a, b) if a == b => Sameness::Equal,
            _ => Sameness::Different,
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Value::Text(text) | Value::Number(text) => text.clone(),
            Value::Long { start, .. } => format!("{start}…"),
            Value::Data { bytes, .. } => format!("{} of data", file_size(*bytes)),
            Value::Flag(on) => if *on { "On" } else { "Off" }.to_owned(),
            Value::Vector(axes) => axes
                .iter()
                .map(|(axis, value)| format!("{} {value}", axis.to_uppercase()))
                .collect::<Vec<_>>()
                .join("  "),
            Value::Color { rgba, hdr } => {
                let hex = hex_color(*rgba);
                match hdr {
                    Some(intensity) => format!("{hex} · Intensity {}", short_number(f64::from(*intensity))),
                    None => hex,
                }
            }
            Value::Reference(reference) => format!("{} ({})", reference.name, reference.kind),
            Value::Nothing(kind) => format!("None ({kind})"),
            Value::Count(count) => format!("{count} {}", if *count == 1 { "item" } else { "items" }),
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sameness {
    Equal,
    Rounding,
    Different,
}

impl Sameness {
    fn and(self, other: Sameness) -> Sameness {
        match (self, other) {
            (Sameness::Different, _) | (_, Sameness::Different) => Sameness::Different,
            (Sameness::Rounding, _) | (_, Sameness::Rounding) => Sameness::Rounding,
            _ => Sameness::Equal,
        }
    }
}

fn numbers(a: &str, b: &str) -> Sameness {
    if a == b {
        return Sameness::Equal;
    }
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => float_sameness(x, y),
        _ => Sameness::Different,
    }
}

fn float_sameness(x: f64, y: f64) -> Sameness {
    if x == y {
        Sameness::Equal
    } else if (x - y).abs() <= SAME_NUMBER * x.abs().max(y.abs()).max(1.) {
        Sameness::Rounding
    } else {
        Sameness::Different
    }
}

pub(crate) fn short_number(value: f64) -> String {
    let rounded = (value * 1000.).round() / 1000.;
    let text = format!("{rounded}");
    if text == "-0" { "0".to_owned() } else { text }
}

pub(crate) fn hex_color(rgba: [f32; 4]) -> String {
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
    let [r, g, b, a] = rgba.map(byte);
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) value: Value,
    pub(crate) noise: bool,
}

#[cfg(test)]
mod tests {
    use super::{Field, Sameness, Value};
    use crate::unity::{
        compare::{State, changed_rows},
        ingest::tests::model_of,
    };

    fn read_fields(body: &str) -> Vec<Field> {
        let text = format!("%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &1
MonoBehaviour:
  m_Name: Table
{body}");
        let model = model_of("Table.asset", &text);
        model.items[0].parts[0].fields.iter().filter(|field| field.path != "m_Name").cloned().collect()
    }

    #[test]
    fn long_hex_reads_as_data_compared_whole_and_shown_by_size() {
        let data = |changed_at: usize| {
            let hex: String = (0..4_096).map(|at| if at == changed_at { 'a' } else { '0' }).collect();
            read_fields(&format!("  _typelessdata: {hex}
")).remove(0).value
        };
        let before = data(3_000);
        assert!(matches!(before, Value::Data { bytes: 2_048, .. }));
        assert_eq!(before.text(), "2.0 KB of data");
        assert_eq!(before.same_as(&data(3_000)), Sameness::Equal);
        assert_eq!(before.same_as(&data(3_001)), Sameness::Different);
    }

    #[test]
    fn floats_that_only_round_differently_are_not_a_real_change() {
        let one = Value::Number("1".to_owned());
        assert_eq!(one.same_as(&Value::Number("0.99999994".to_owned())), Sameness::Rounding);
        assert_eq!(one.same_as(&Value::Number("1.4".to_owned())), Sameness::Different);
        assert_eq!(one.same_as(&Value::Number("1".to_owned())), Sameness::Equal);
    }

    #[test]
    fn list_entries_with_an_id_pair_by_id_so_removing_one_leaves_the_rest_unchanged() {
        let table = |entries: &[(&str, &str)]| {
            let listed: String = entries.iter().map(|(id, words)| format!("  - m_Id: {id}
    m_Localized: {words}
")).collect();
            read_fields(&format!("  m_TableData:
{listed}"))
        };
        let before = table(&[("1", "Language"), ("2", "Send data"), ("3", "General")]);
        let after = table(&[("1", "Language"), ("3", "General"), ("4", "Privacy")]);
        let (rows, _) = changed_rows(Some(&before), Some(&after), Default::default());
        let changed: Vec<(String, State)> = rows
            .iter()
            .filter(|row| row.depth == 2)
            .map(|row| (row.before.as_ref().or(row.after.as_ref()).unwrap().text(), row.state))
            .collect();
        assert_eq!(
            changed,
            [
                ("2".to_owned(), State::Removed),
                ("Send data".to_owned(), State::Removed),
                ("4".to_owned(), State::Added),
                ("Privacy".to_owned(), State::Added),
            ]
        );
    }
}
