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

#[cfg(test)]
mod tests {
    use super::nicify;

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
        assert_eq!(nicify("Light2D"), "Light 2D");
    }

}
