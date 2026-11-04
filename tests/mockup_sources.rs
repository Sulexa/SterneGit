use std::{fs, path::Path};

#[test]
fn every_source_file_the_mockup_names_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mockup = root.join("docs").join("mockup");
    let mut missing = Vec::new();
    let mut named = 0;
    for entry in fs::read_dir(&mockup).expect("docs/mockup exists") {
        let path = entry.expect("docs/mockup is readable").path();
        if path.extension().is_none_or(|extension| extension != "js") {
            continue;
        }
        let script = fs::read_to_string(&path).expect("mockup scripts are UTF-8");
        for source in sources_named(&script) {
            named += 1;
            if !root.join(source).is_file() {
                let shown = path.file_name().unwrap_or_default().to_string_lossy();
                missing.push(format!("{shown} names {source}"));
            }
        }
    }
    assert!(named > 0, "the mockup names no source files");
    assert!(
        missing.is_empty(),
        "the mockup points at files that moved:\n{}",
        missing.join("\n")
    );
}

#[test]
fn only_source_fields_count() {
    let script = "{ source: 'src/repo/mod.rs', path: 'src/ui/toast.rs' }, { source: 'src/ui/rows.rs' }";
    assert_eq!(sources_named(script), ["src/repo/mod.rs", "src/ui/rows.rs"]);
}

fn sources_named(script: &str) -> Vec<&str> {
    const FIELD: &str = "source: '";
    script
        .match_indices(FIELD)
        .filter_map(|(at, _)| {
            let value = &script[at + FIELD.len()..];
            value.split_once('\'').map(|(path, _)| path)
        })
        .collect()
}
