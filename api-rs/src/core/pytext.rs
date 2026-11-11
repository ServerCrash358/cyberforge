//! Python's text rules, so the Rust server treats every input exactly like the Python one did.
//!
//! Rust's `char::is_whitespace` and `is_alphanumeric` differ from Python's `str.isspace` and
//! `str.isalnum` on a few characters (Python counts U+001C..U+001F as space; Rust counts Hindi
//! vowel signs as alphanumeric, Python does not). These helpers follow Python.

use unicode_general_category::{GeneralCategory, get_general_category};

/// Python `str.isspace` for one character.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python `str.isalnum` for one character: a letter (L*) or a number (N*).
pub fn is_alnum(c: char) -> bool {
    use GeneralCategory as G;
    matches!(
        get_general_category(c),
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
            | G::DecimalNumber
            | G::LetterNumber
            | G::OtherNumber
    )
}

/// Python `str.strip()` with no arguments.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// Python `str.split()` with no arguments.
pub fn split(s: &str) -> impl Iterator<Item = &str> {
    s.split(is_space).filter(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_follows_python() {
        assert!(is_space(' ') && is_space('\t') && is_space('\u{1f}') && is_space('\u{3000}'));
        assert!(!is_space('a') && !is_space('\u{200b}'));
    }

    #[test]
    fn alnum_follows_python() {
        assert!(is_alnum('a') && is_alnum('7') && is_alnum('क') && is_alnum('½'));
        // Devanagari vowel sign AA (Mc) and virama (Mn): Python isalnum() is False.
        assert!(!is_alnum('\u{093e}') && !is_alnum('\u{094d}'));
        assert!(!is_alnum('_') && !is_alnum('-') && !is_alnum(' '));
    }

    #[test]
    fn strip_and_split() {
        assert_eq!(strip("\u{1c} ab c \n"), "ab c");
        assert_eq!(split("  a\tb  c ").collect::<Vec<_>>(), ["a", "b", "c"]);
    }
}
