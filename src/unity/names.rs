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
    let last = full.rsplit(['.', '+']).next().unwrap_or(full);
    let last = last.split(['`', ',']).next().unwrap_or(last);
    match last {
        "Single" => "Float".to_owned(),
        "Int32" => "Integer".to_owned(),
        "Boolean" => "Boolean".to_owned(),
        _ => nicify(last),
    }
}

#[cfg(test)]
mod tests {
    use super::{UnityKind, component_title, file_stem, nicify, short_type};

    #[test]
    fn field_names_read_like_the_inspector_shows_them() {
        assert_eq!(nicify("m_LocalPosition"), "Local Position");
        assert_eq!(nicify("moveSpeed"), "Move Speed");
        assert_eq!(nicify("_BaseColor"), "Base Color");
        assert_eq!(nicify("spritePixelsToUnits"), "Sprite Pixels To Units");
        assert_eq!(nicify("HDRColor"), "HDR Color");
        assert_eq!(nicify("kMaxCount"), "Max Count");
        assert_eq!(nicify("ifTrue"), "If True");
    }

    #[test]
    fn two_d_stays_together_after_its_number() {
        assert_eq!(nicify("CircleCollider2D"), "Circle Collider 2D");
        assert_eq!(component_title("Rigidbody2D"), "Rigidbody 2D");
        assert_eq!(nicify("Light2D"), "Light 2D");
    }

    #[test]
    fn unity_files_are_known_by_their_extension() {
        assert_eq!(UnityKind::of("Assets/Slime.prefab"), Some(UnityKind::Prefab));
        assert_eq!(UnityKind::of("Assets/Scenes/Level1.unity"), Some(UnityKind::Scene));
        assert_eq!(UnityKind::of("Assets/slime.png.meta"), Some(UnityKind::Meta));
        assert_eq!(UnityKind::of("ProjectSettings/QualitySettings.asset"), Some(UnityKind::Asset));
        assert_eq!(UnityKind::of("src/main.rs"), None);
    }

    #[test]
    fn type_names_keep_only_their_last_part() {
        assert_eq!(short_type("System.Single"), "Float");
        assert_eq!(short_type("UnityEngine.Vector2"), "Vector 2");
        assert_eq!(file_stem("Assets/Scripts/SlimeController.cs"), "SlimeController");
    }
}
