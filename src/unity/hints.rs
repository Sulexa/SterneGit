use super::{
    names::file_name,
    value::Lookup,
    yaml::Document,
};

const FILE_ID: &str = "fileID: ";
const GUID: &str = "guid: ";
const HINT: &str = "  # ";

pub(crate) struct Scopes<'a> {
    before: &'a [Document],
    after: &'a [Document],
    lookup: &'a Lookup,
}

impl<'a> Scopes<'a> {
    pub(crate) fn new(before: &'a [Document], after: &'a [Document], lookup: &'a Lookup) -> Self {
        Self { before, after, lookup }
    }

    fn scope_at(&self, documents: &[Document], line: usize) -> Option<String> {
        let document = documents.iter().find(|document| document.first_line <= line && line <= document.last_line)?;
        let (name, kind) = self.lookup.objects.get(&document.file_id)?;
        Some(if kind == "GameObject" || name == kind { name.clone() } else { format!("{name} › {kind}") })
    }

    fn hint(&self, line: &str) -> Option<String> {
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
                (Some(guid), _) if !guid.is_empty() => self.lookup.asset_path(guid).map(|path| file_name(path).to_owned()),
                (_, Some(id)) if id != 0 => self.lookup.objects.get(&id).map(|(name, kind)| {
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

    pub(crate) fn annotate(&self, patch: &str) -> String {
        let mut out = String::with_capacity(patch.len() + patch.len() / 8);
        let lines: Vec<&str> = patch.split_inclusive('\n').collect();
        let mut ix = 0;
        while ix < lines.len() {
            let line = lines[ix];
            if let Some(header) = line.strip_prefix("@@ ")
                && let Some((ranges, _)) = header.split_once(" @@")
            {
                let (mut old, mut new) = starts(ranges);
                let mut end = ix + 1;
                while end < lines.len() && !lines[end].starts_with("@@ ") && !lines[end].starts_with("diff --git") {
                    end += 1;
                }
                let mut scope = None;
                for body in &lines[ix + 1..end] {
                    match body.as_bytes().first() {
                        Some(b'+') => {
                            scope = self.scope_at(self.after, new);
                            break;
                        }
                        Some(b'-') => {
                            scope = self.scope_at(self.before, old);
                            break;
                        }
                        Some(b' ') => {
                            old += 1;
                            new += 1;
                        }
                        _ => {}
                    }
                }
                out.push_str("@@ ");
                out.push_str(ranges);
                out.push_str(" @@");
                if let Some(scope) = scope {
                    out.push(' ');
                    out.push_str(&scope);
                }
                out.push('\n');
                for body in &lines[ix + 1..end] {
                    out.push_str(&self.with_hint(body));
                }
                ix = end;
                continue;
            }
            out.push_str(line);
            ix += 1;
        }
        out
    }

    fn with_hint(&self, line: &str) -> String {
        let marked = matches!(line.as_bytes().first(), Some(b'+' | b'-' | b' '));
        let hint = marked.then(|| self.hint(line)).flatten();
        match hint {
            Some(hint) => {
                let (text, ending) = match line.strip_suffix("\r\n") {
                    Some(text) => (text, "\r\n"),
                    None => match line.strip_suffix('\n') {
                        Some(text) => (text, "\n"),
                        None => (line, ""),
                    },
                };
                format!("{text}{HINT}{hint}{ending}")
            }
            None => line.to_owned(),
        }
    }
}

fn starts(ranges: &str) -> (usize, usize) {
    let mut parts = ranges.split_whitespace();
    let read = |part: Option<&str>| {
        part.map(|part| part.trim_start_matches(['-', '+']))
            .and_then(|part| part.split(',').next())
            .and_then(|start| start.parse::<usize>().ok())
            .unwrap_or(1)
    };
    let old = read(parts.next());
    let new = read(parts.next());
    (old, new)
}

#[cfg(test)]
mod tests {
    use super::Scopes;
    use crate::unity::{scene::object_names, value::Lookup, yaml::documents};

    const AFTER: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Component:
  - component: {fileID: 500}
  m_Name: Slime
--- !u!50 &500
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_Mass: 1.4
  m_Sprite: {fileID: 21300000, guid: aaaabbbbccccddddeeeeffff00001111, type: 3}
";

    #[test]
    fn each_part_is_named_after_its_object_and_references_get_their_names() {
        let docs = documents(AFTER);
        let mut lookup = Lookup::default();
        lookup.assets.insert("aaaabbbbccccddddeeeeffff00001111".to_owned(), "Assets/Art/slime.png".to_owned());
        lookup.objects = object_names(&docs, &lookup.assets);
        let patch = "diff --git a/S.prefab b/S.prefab\n--- a/S.prefab\n+++ b/S.prefab\n@@ -9,3 +9,3 @@ Rigidbody2D:\n   m_GameObject: {fileID: 100}\n-  m_Mass: 1\n+  m_Mass: 1.4\n   m_Sprite: {fileID: 21300000, guid: aaaabbbbccccddddeeeeffff00001111, type: 3}\n";
        let annotated = Scopes::new(&docs, &docs, &lookup).annotate(patch);
        assert!(annotated.contains("@@ -9,3 +9,3 @@ Slime › Rigidbody 2D\n"), "{annotated}");
        assert!(annotated.contains("   m_GameObject: {fileID: 100}  # Slime\n"), "{annotated}");
        assert!(annotated.contains("type: 3}  # slime.png\n"), "{annotated}");
        assert!(annotated.contains("+  m_Mass: 1.4\n"));
    }
}
