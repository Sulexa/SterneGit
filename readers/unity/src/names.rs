#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum UnityKind {
    Prefab,
    Scene,
    Material,
    AnimationClip,
    AnimatorController,
    Asset,
    Meta,
}

impl UnityKind {
    pub(crate) fn of(path: &str) -> Option<Self> {
        let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
        Some(match extension.as_str() {
            "prefab" => UnityKind::Prefab,
            "unity" => UnityKind::Scene,
            "mat" => UnityKind::Material,
            "anim" => UnityKind::AnimationClip,
            "controller" | "overridecontroller" => UnityKind::AnimatorController,
            "asset" | "physicmaterial" | "physicsmaterial2d" | "mask" | "flare" | "guiskin" | "fontsettings"
            | "mixer" | "playable" | "signal" | "lighting" | "renderTexture" | "rendertexture" | "cubemap"
            | "spriteatlas" | "terrainlayer" | "brush" => UnityKind::Asset,
            "meta" => UnityKind::Meta,
            _ => return None,
        })
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            UnityKind::Prefab => "Prefab",
            UnityKind::Scene => "Scene",
            UnityKind::Material => "Material",
            UnityKind::AnimationClip => "Animation clip",
            UnityKind::AnimatorController => "Animator controller",
            UnityKind::Asset => "Asset",
            UnityKind::Meta => "Import settings",
        }
    }

    pub(crate) fn has_hierarchy(self) -> bool {
        matches!(self, UnityKind::Prefab | UnityKind::Scene)
    }
}

pub(crate) fn nicify(name: &str) -> String {
    thread_local! {
        static NICE: std::cell::RefCell<std::collections::HashMap<String, String>> = Default::default();
    }
    NICE.with(|nice| {
        if let Some(found) = nice.borrow().get(name) {
            return found.clone();
        }
        let made = nicified(name);
        nice.borrow_mut().insert(name.to_owned(), made.clone());
        made
    })
}

fn nicified(name: &str) -> String {
    let trimmed = name
        .strip_prefix("m_")
        .or_else(|| name.strip_prefix('_'))
        .or_else(|| {
            name.strip_prefix('k')
                .filter(|rest| rest.chars().next().is_some_and(char::is_uppercase))
        })
        .unwrap_or(name);
    let chars: Vec<char> = trimmed.chars().filter(|&c| c != '_').collect();
    let mut out = String::with_capacity(chars.len() + 4);
    for (ix, &c) in chars.iter().enumerate() {
        let previous = ix.checked_sub(1).map(|at| chars[at]);
        let next = chars.get(ix + 1).copied();
        let starts_word = match previous {
            None => false,
            Some(previous) => {
                (c.is_uppercase() && (previous.is_lowercase() || previous.is_ascii_digit() && !is_dimension(c, next)))
                    || (c.is_uppercase() && previous.is_uppercase() && next.is_some_and(char::is_lowercase))
                    || (c.is_ascii_digit() && previous.is_alphabetic())
            }
        };
        if starts_word {
            out.push(' ');
        }
        if ix == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn is_dimension(c: char, next: Option<char>) -> bool {
    c == 'D' && !next.is_some_and(char::is_lowercase)
}

pub(crate) fn file_stem(path: &str) -> &str {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

pub(crate) fn component_title(type_name: &str) -> String {
    match type_name {
        "MonoBehaviour" => "Script".to_owned(),
        "GameObject" => "GameObject".to_owned(),
        "Rigidbody2D" => "Rigidbody 2D".to_owned(),
        "RectTransform" => "Rect Transform".to_owned(),
        _ => nicify(type_name),
    }
}

pub(crate) fn short_type(full: &str) -> String {
    let name = outside_brackets(full).next().unwrap_or(full).trim();
    let (base, arguments) = name.find('[').map_or((name, ""), |open| (&name[..open], &name[open..]));
    let base = base.split('`').next().unwrap_or(base);
    let last = base.rsplit(['.', '+']).next().unwrap_or(base);
    let short = match last {
        "Single" => "Float".to_owned(),
        "Int32" => "Integer".to_owned(),
        "Boolean" => "Boolean".to_owned(),
        _ => nicify(last),
    };
    if arguments.starts_with("[]") {
        return format!("Array of {short}");
    }
    let inner = arguments.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or_default();
    let of: Vec<String> = outside_brackets(inner)
        .map(str::trim)
        .map(|argument| argument.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or(argument))
        .filter(|argument| !argument.is_empty())
        .map(short_type)
        .collect();
    if of.is_empty() { short } else { format!("{short} of {}", of.join(" and ")) }
}

fn outside_brackets(text: &str) -> impl Iterator<Item = &str> {
    let mut depth = 0_i32;
    text.split(move |letter| {
        match letter {
            '[' => depth += 1,
            ']' => depth -= 1,
            _ => {}
        }
        letter == ',' && depth == 0
    })
}
