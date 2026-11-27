use super::{names::file_name, value::Lookup};

const FILE_ID: &str = "fileID: ";
const GUID: &str = "guid: ";

pub(crate) fn hint(line: &str, lookup: &Lookup) -> Option<String> {
    let mut names = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find(FILE_ID) {
        let tail = &rest[start + FILE_ID.len()..];
        let end = tail.find(['}', ',']).unwrap_or(tail.len());
        let id = tail[..end].trim().parse::<i64>().ok();
        let flow_end = tail.find('}').unwrap_or(tail.len());
        let flow = &tail[..flow_end];
        let guid = flow.find(GUID).map(|at| flow[at + GUID.len()..].split([',', '}']).next().unwrap_or_default().trim());
        let name = match (guid, id) {
            (Some(guid), _) if !guid.is_empty() => lookup.asset_path(guid).map(|path| file_name(path).to_owned()),
            (_, Some(id)) if id != 0 => lookup.objects.get(&id).map(|(name, kind)| {
                if kind == "GameObject" { name.clone() } else { format!("{name} ({kind})") }
            }),
            _ => None,
        };
        if let Some(name) = name
            && !names.contains(&name)
        {
            names.push(name);
        }
        rest = &tail[end.min(tail.len())..];
    }
    (!names.is_empty()).then(|| names.join(", "))
}
