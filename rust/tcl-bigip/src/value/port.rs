//! `Port` + `PortRange` value types.

use super::error::ValueError;
use std::fmt;

/// F5 port wildcards: `"any"` / `"*"` / `"0"` all mean "match any port".
fn is_any_token(lower: &str) -> bool {
    matches!(lower, "any" | "*" | "0")
}

/// A single TCP/UDP port (0-65535) or the wildcard `any`.
///
/// Port 0 has special semantics in F5: it's the wildcard "match any
/// port". We store wildcard as `port == 0` and remember whether the user
/// spelt it `"any"` / `"*"` / `"0"` so [`Display`](fmt::Display)
/// round-trips the original spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Port {
    /// 0..=65535; 0 means wildcard.
    pub port: u16,
    /// Original textual form (`"80"` / `"any"` / `"*"` / `"0"`).
    pub spelling: String,
}

impl Port {
    /// Construct a port directly from its number and spelling.
    #[must_use]
    pub fn new(port: u16, spelling: impl Into<String>) -> Self {
        Self {
            port,
            spelling: spelling.into(),
        }
    }

    /// Parse `text` as a port number or wildcard token.
    ///
    /// # Errors
    /// Returns [`ValueError`] on empty, non-numeric, or out-of-range input.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("Port: empty input".to_owned()));
        }
        let lower = text.to_lowercase();
        if is_any_token(&lower) {
            return Ok(Self {
                port: 0,
                spelling: text.to_owned(),
            });
        }
        let value: i64 = parse_decimal_int(text)
            .ok_or_else(|| ValueError(format!("Port: not a number ({})", py_repr(text))))?;
        if !(0..=65535).contains(&value) {
            return Err(ValueError(format!("Port: out of range ({value})")));
        }
        Ok(Self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            port: value as u16,
            spelling: text.to_owned(),
        })
    }

    /// Like [`Port::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when this port matches every port (wildcard).
    #[must_use]
    pub fn is_any(&self) -> bool {
        self.port == 0
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.spelling.is_empty() {
            return f.write_str(&self.spelling);
        }
        if self.port == 0 {
            return f.write_str("any");
        }
        write!(f, "{}", self.port)
    }
}

/// An inclusive port range (e.g. `1024-65535`).
///
/// Single-port ranges are accepted (`80-80`) but a bare port should use
/// [`Port`] directly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PortRange {
    /// Inclusive low bound.
    pub low: u16,
    /// Inclusive high bound.
    pub high: u16,
}

impl PortRange {
    /// Parse a `LOW-HIGH` port range.
    ///
    /// # Errors
    /// Single ports without a hyphen, non-numeric input, out-of-range
    /// bounds, and reverse ranges (`high < low`) all return [`ValueError`].
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if !text.contains('-') {
            return Err(ValueError(format!(
                "PortRange: missing '-' in {}",
                py_repr(text)
            )));
        }
        // Python str.partition splits on the FIRST '-'.
        let (low_text, high_text) = text.split_once('-').unwrap_or((text, ""));
        let parse_part = |s: &str| parse_decimal_int(s.trim());
        let (Some(low), Some(high)) = (parse_part(low_text), parse_part(high_text)) else {
            return Err(ValueError(format!(
                "PortRange: not numeric ({})",
                py_repr(text)
            )));
        };
        if !(0..=65535).contains(&low) || !(0..=65535).contains(&high) {
            return Err(ValueError(format!(
                "PortRange: out of range ({})",
                py_repr(text)
            )));
        }
        if high < low {
            return Err(ValueError(format!(
                "PortRange: reverse range ({})",
                py_repr(text)
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self {
            low: low as u16,
            high: high as u16,
        })
    }

    /// Like [`PortRange::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when `port` lies inside the inclusive range.
    #[must_use]
    pub fn contains_port(&self, port: u16) -> bool {
        self.low <= port && port <= self.high
    }

    /// `true` when the wrapped [`Port`] lies inside the inclusive range.
    #[must_use]
    pub fn contains(&self, port: &Port) -> bool {
        self.contains_port(port.port)
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.low, self.high)
    }
}

/// Parse a base-10 integer exactly as Python's `int(text, 10)` does for
/// the spellings these parsers feed it: optional sign, ASCII digits, with
/// surrounding ASCII whitespace tolerated. Returns `None` on anything else.
pub(crate) fn parse_decimal_int(text: &str) -> Option<i64> {
    let trimmed = text.trim_matches(|c: char| c.is_ascii_whitespace());
    if trimmed.is_empty() {
        return None;
    }
    let (sign, digits) = match trimmed.strip_prefix(['+', '-']) {
        Some(rest) => (&trimmed[..1], rest),
        None => ("", trimmed),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    format!("{sign}{digits}").parse::<i64>().ok()
}

/// Render `text` the way Python's `repr()` would for a short single-line
/// string: wrapped in single quotes (double quotes when the value itself
/// contains a single quote and no double quote), matching the `{text!r}`
/// interpolation used in the Python error messages.
pub(crate) fn py_repr(text: &str) -> String {
    let has_single = text.contains('\'');
    let has_double = text.contains('"');
    let (quote, escape_quote) = if has_single && !has_double {
        ('"', '"')
    } else {
        ('\'', '\'')
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == escape_quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numeric_port() {
        let p = Port::parse("80").unwrap();
        assert_eq!(p.port, 80);
        assert_eq!(p.to_string(), "80");
        assert!(!p.is_any());
    }

    #[test]
    fn parses_wildcard_spellings() {
        for tok in ["any", "*", "0", "ANY"] {
            let p = Port::parse(tok).unwrap();
            assert_eq!(p.port, 0);
            assert!(p.is_any());
            assert_eq!(p.to_string(), tok);
        }
    }

    #[test]
    fn rejects_bad_ports() {
        assert_eq!(Port::parse("").unwrap_err().0, "Port: empty input");
        assert_eq!(
            Port::parse("abc").unwrap_err().0,
            "Port: not a number ('abc')"
        );
        assert_eq!(
            Port::parse("70000").unwrap_err().0,
            "Port: out of range (70000)"
        );
        assert!(Port::try_parse("nope").is_none());
    }

    #[test]
    fn renders_wildcard_without_spelling() {
        assert_eq!(Port::new(0, "").to_string(), "any");
        assert_eq!(Port::new(443, "").to_string(), "443");
    }

    #[test]
    fn parses_port_range() {
        let r = PortRange::parse("1024-65535").unwrap();
        assert_eq!((r.low, r.high), (1024, 65535));
        assert_eq!(r.to_string(), "1024-65535");
        assert!(r.contains_port(2000));
        assert!(!r.contains_port(80));
    }

    #[test]
    fn rejects_bad_ranges() {
        assert_eq!(
            PortRange::parse("80").unwrap_err().0,
            "PortRange: missing '-' in '80'"
        );
        assert_eq!(
            PortRange::parse("a-b").unwrap_err().0,
            "PortRange: not numeric ('a-b')"
        );
        assert_eq!(
            PortRange::parse("100-50").unwrap_err().0,
            "PortRange: reverse range ('100-50')"
        );
        assert_eq!(
            PortRange::parse("0-70000").unwrap_err().0,
            "PortRange: out of range ('0-70000')"
        );
    }
}
