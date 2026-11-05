pub(crate) fn expand(
    template: &str,
    mut value_of: impl FnMut(&str) -> Option<String>,
) -> Result<String, String> {
    let unbalanced = || format!("{template} has an unbalanced {{ or }}");
    let mut expanded = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest[open..].starts_with('}') {
            return Err(unbalanced());
        }
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or_else(unbalanced)?;
        let name = &after[..close];
        if name.is_empty() || name.contains('{') {
            return Err(unbalanced());
        }
        let value = value_of(name)
            .ok_or_else(|| format!("{template} names {{{name}}}, which isn't defined here"))?;
        expanded.push_str(&rest[..open]);
        expanded.push_str(&value);
        rest = &after[close + 1..];
    }
    expanded.push_str(rest);
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::expand;

    fn known(name: &str) -> Option<String> {
        match name {
            "server" => Some("https://git.example.com".into()),
            "web" => Some("https://git.example.com/".into()),
            _ => None,
        }
    }

    #[test]
    fn placeholders_are_replaced_by_their_values() {
        assert_eq!(expand("{server}/api/v3/", known).unwrap(), "https://git.example.com/api/v3/");
        assert_eq!(
            expand("{web}settings/tokens/new?scopes=repo", known).unwrap(),
            "https://git.example.com/settings/tokens/new?scopes=repo"
        );
        assert_eq!(expand("https://api.github.com/", known).unwrap(), "https://api.github.com/");
    }

    #[test]
    fn an_unknown_placeholder_is_named_in_the_error() {
        assert_eq!(
            expand("{token}/x", known),
            Err("{token}/x names {token}, which isn't defined here".into())
        );
    }

    #[test]
    fn unbalanced_braces_are_refused() {
        for template in ["{server/x", "server}/x", "{}/x", "{{server}}"] {
            assert!(expand(template, known).is_err(), "{template}");
        }
    }
}
