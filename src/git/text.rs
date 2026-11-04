use std::ffi::OsStr;

const WINDOWS_1252_FROM_0X80: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

pub(crate) fn decode(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    let mut text = String::with_capacity(bytes.len() + bytes.len() / 2);
    for line in bytes.split_inclusive(|&byte| byte == b'\n') {
        match std::str::from_utf8(line) {
            Ok(line) => text.push_str(line),
            Err(_) => text.extend(line.iter().copied().map(windows_1252)),
        }
    }
    text
}

#[cfg(unix)]
pub(crate) fn os_name<'a>(bytes: &'a [u8], _shown: &'a str) -> &'a OsStr {
    use std::os::unix::ffi::OsStrExt as _;
    OsStr::from_bytes(bytes)
}

#[cfg(not(unix))]
pub(crate) fn os_name<'a>(bytes: &'a [u8], shown: &'a str) -> &'a OsStr {
    OsStr::new(std::str::from_utf8(bytes).unwrap_or(shown))
}

fn windows_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9F => WINDOWS_1252_FROM_0X80[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn utf8_text_is_kept_as_it_is() {
        let text = "café crème, déjà vu, 漢字 ✓";
        assert_eq!(decode(text.as_bytes()), text);
    }

    #[test]
    fn latin_1_text_reads_as_its_accented_letters() {
        assert_eq!(
            decode(b"caf\xE9 cr\xE8me br\xFBl\xE9e \xE0 la fran\xE7aise, \xE2ge"),
            "café crème brûlée à la française, âge"
        );
    }

    #[test]
    fn windows_1252_punctuation_reads_as_itself() {
        assert_eq!(
            decode(b"\x80 5, \x93quoted\x94, \xABguillemets\xBB, \x9Cuvre\x85"),
            "€ 5, “quoted”, «guillemets», œuvre…"
        );
    }

    #[test]
    fn each_line_keeps_its_own_encoding() {
        assert_eq!(decode(b"caf\xC3\xA9\ncaf\xE9\n"), "café\ncafé\n");
    }
}
