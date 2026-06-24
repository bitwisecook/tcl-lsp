//! Typed BIG-IP source-address-translation mode value.

use std::fmt;

/// The variant of a [`SnatMode`].
///
/// [`SnatModeKind::as_str`] returns the canonical string literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnatModeKind {
    /// SNAT disabled (`type none`).
    None,
    /// Auto-selected self-IP (`type automap`).
    Automap,
    /// Named SNAT pool (`type snat` + `pool <full-path>`).
    Snat,
    /// Reserved address-list variant.
    AddressList,
}

impl SnatModeKind {
    /// The canonical string literal for this kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SnatModeKind::None => "none",
            SnatModeKind::Automap => "automap",
            SnatModeKind::Snat => "snat",
            SnatModeKind::AddressList => "address-list",
        }
    }
}

/// A parsed source-address-translation mode.
///
/// `raw` preserves the original brace-body text so a no-op round trip
/// doesn't change spacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnatMode {
    /// The mode variant.
    pub kind: SnatModeKind,
    /// SNAT pool reference (for the `snat` kind).
    pub pool_path: String,
    /// Address list (for the `address-list` kind).
    pub addresses: Vec<String>,
    /// Original body text (stripped).
    pub raw: String,
}

impl Default for SnatMode {
    fn default() -> Self {
        Self {
            kind: SnatModeKind::None,
            pool_path: String::new(),
            addresses: Vec::new(),
            raw: String::new(),
        }
    }
}

impl SnatMode {
    /// Parse a `source-address-translation` body, with or without the
    /// surrounding braces. Returns `None` for empty input or a malformed
    /// `type snat` clause without a pool.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut body = trimmed;
        if let Some(rest) = body.strip_prefix('{') {
            body = rest;
        }
        if let Some(rest) = body.strip_suffix('}') {
            body = rest;
        }
        let tokens: Vec<&str> = body.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        let mut kind = SnatModeKind::None;
        let mut pool_path = String::new();
        let mut idx = 0;
        while idx < tokens.len() {
            let tok = tokens[idx];
            if tok == "type" && idx + 1 < tokens.len() {
                match tokens[idx + 1] {
                    "none" => kind = SnatModeKind::None,
                    "automap" => kind = SnatModeKind::Automap,
                    "snat" => kind = SnatModeKind::Snat,
                    _ => return None,
                }
                idx += 2;
                continue;
            }
            if tok == "pool" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut pool_path);
                idx += 2;
                continue;
            }
            idx += 1;
        }
        if kind == SnatModeKind::Snat && pool_path.is_empty() {
            return None;
        }
        Some(Self {
            kind,
            pool_path,
            addresses: Vec::new(),
            raw: trimmed.to_owned(),
        })
    }

    /// `true` for the `none` kind.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.kind == SnatModeKind::None
    }

    /// `true` for the `automap` kind.
    #[must_use]
    pub fn is_automap(&self) -> bool {
        self.kind == SnatModeKind::Automap
    }

    /// `true` for the `snat` kind.
    #[must_use]
    pub fn is_snat(&self) -> bool {
        self.kind == SnatModeKind::Snat
    }

    /// Return the full-paths this mode references (the SNAT pool, for
    /// `snat`). Empty otherwise.
    #[must_use]
    pub fn references(&self) -> Vec<String> {
        if self.kind == SnatModeKind::Snat && !self.pool_path.is_empty() {
            return vec![self.pool_path.clone()];
        }
        Vec::new()
    }
}

impl fmt::Display for SnatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return f.write_str(&self.raw);
        }
        match self.kind {
            SnatModeKind::None => f.write_str("{ type none }"),
            SnatModeKind::Automap => f.write_str("{ type automap }"),
            SnatModeKind::Snat => write!(f, "{{ type snat pool {} }}", self.pool_path),
            SnatModeKind::AddressList => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forms() {
        let m = SnatMode::try_parse("type none").unwrap();
        assert!(m.is_none());
        let m = SnatMode::try_parse("{ type automap }").unwrap();
        assert!(m.is_automap());
        assert_eq!(m.to_string(), "{ type automap }");
        let m = SnatMode::try_parse("type snat pool /Common/snatpool_a").unwrap();
        assert!(m.is_snat());
        assert_eq!(m.pool_path, "/Common/snatpool_a");
        assert_eq!(m.references(), vec!["/Common/snatpool_a".to_owned()]);
    }

    #[test]
    fn structured_render() {
        let m = SnatMode {
            kind: SnatModeKind::Snat,
            pool_path: "/Common/p".to_owned(),
            ..Default::default()
        };
        assert_eq!(m.to_string(), "{ type snat pool /Common/p }");
        let m = SnatMode::default();
        assert_eq!(m.to_string(), "{ type none }");
    }

    #[test]
    fn rejects_bad() {
        assert!(SnatMode::try_parse("").is_none());
        assert!(SnatMode::try_parse("type snat").is_none());
        assert!(SnatMode::try_parse("type bogus").is_none());
    }
}
