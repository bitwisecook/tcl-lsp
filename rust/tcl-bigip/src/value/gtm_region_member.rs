//! Typed BIG-IP GTM region-member row value.

use std::fmt;

/// One row inside a GTM `region-members` block.
///
/// `kind` is the row's clause type (`subnet` / `country` / `state` /
/// `isp` / `continent` / ...). `value` is the matched literal. `negated`
/// is the `not` prefix. `raw` preserves the original spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct GtmRegionMember {
    /// Clause type.
    pub kind: String,
    /// Matched literal value.
    pub value: String,
    /// Whether the row carries a `not` prefix.
    pub negated: bool,
    /// Original row text (stripped).
    pub raw: String,
}

impl GtmRegionMember {
    /// Parse one row. The input may include the trailing `{ }` body or
    /// omit it. Returns `None` for empty or all-whitespace input, or rows
    /// with fewer than two tokens after the optional `not`.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        let stripped_full = text.trim();
        if stripped_full.is_empty() {
            return None;
        }
        // Drop the trailing ``{ ... }`` body.
        let mut stripped = stripped_full;
        if let Some(brace) = stripped.find('{') {
            stripped = stripped[..brace].trim();
        }
        let mut tokens: Vec<&str> = stripped.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        let mut negated = false;
        if tokens[0] == "not" {
            negated = true;
            tokens.remove(0);
        }
        if tokens.len() < 2 {
            return None;
        }
        let kind = tokens[0].to_owned();
        let value = tokens[1..].join(" ");
        Some(Self {
            kind,
            value,
            negated,
            raw: text.trim().to_owned(),
        })
    }
}

impl fmt::Display for GtmRegionMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return f.write_str(&self.raw);
        }
        let prefix = if self.negated { "not " } else { "" };
        write!(f, "{prefix}{} {} {{ }}", self.kind, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subnet() {
        let m = GtmRegionMember::try_parse("subnet 10.10.1.0/24 { }").unwrap();
        assert_eq!(m.kind, "subnet");
        assert_eq!(m.value, "10.10.1.0/24");
        assert!(!m.negated);
        assert_eq!(m.to_string(), "subnet 10.10.1.0/24 { }");
    }

    #[test]
    fn parse_negated() {
        let m = GtmRegionMember::try_parse("not country JP { }").unwrap();
        assert!(m.negated);
        assert_eq!(m.kind, "country");
        assert_eq!(m.value, "JP");
        assert_eq!(m.to_string(), "not country JP { }");
    }

    #[test]
    fn structured_render() {
        let m = GtmRegionMember {
            kind: "country".to_owned(),
            value: "US".to_owned(),
            negated: false,
            raw: String::new(),
        };
        assert_eq!(m.to_string(), "country US { }");
    }

    #[test]
    fn rejects_empty_and_short() {
        assert!(GtmRegionMember::try_parse("").is_none());
        assert!(GtmRegionMember::try_parse("subnet").is_none());
        assert!(GtmRegionMember::try_parse("not subnet").is_none());
    }
}
