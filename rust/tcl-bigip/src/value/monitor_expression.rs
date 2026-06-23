//! Typed BIG-IP monitor-expression value.

use std::fmt;

/// The grammatical shape of a parsed `monitor` expression.
///
/// [`MonitorMode::as_str`] returns the exact Python string literal for
/// each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonitorMode {
    /// `monitor default` — use the parent object's default selection.
    Default,
    /// `monitor none` — suppress monitoring entirely.
    None,
    /// One bare monitor reference (`monitor /Common/http`).
    Single,
    /// `monitor M1 and M2 [...]` — every listed monitor must report up.
    All,
    /// `monitor min N of { M1 M2 [...] }` — at least N must report up.
    MinOf,
}

impl MonitorMode {
    /// The Python string literal for this mode.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MonitorMode::Default => "default",
            MonitorMode::None => "none",
            MonitorMode::Single => "single",
            MonitorMode::All => "all",
            MonitorMode::MinOf => "min-of",
        }
    }
}

/// A parsed `monitor` expression.
///
/// `monitors` is the ordered list of monitor full-paths (empty for the
/// `default` and `none` modes). `minimum` is the threshold for the
/// `min-of` mode and `None` otherwise. The original spelling is preserved
/// on `raw` so a round trip doesn't change formatting when no monitor
/// membership changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonitorExpression {
    /// The grammatical mode.
    pub mode: MonitorMode,
    /// Ordered monitor full-paths.
    pub monitors: Vec<String>,
    /// Minimum threshold (only for `min-of`).
    pub minimum: Option<i64>,
    /// Original spelling.
    pub raw: String,
    /// Per-monitor byte spans inside `raw` (local offsets), in order.
    pub monitor_spans: Vec<(usize, usize)>,
}

impl Default for MonitorExpression {
    fn default() -> Self {
        Self {
            mode: MonitorMode::Default,
            monitors: Vec::new(),
            minimum: None,
            raw: String::new(),
            monitor_spans: Vec::new(),
        }
    }
}

impl MonitorExpression {
    /// Parse a TMSH monitor expression. Returns `None` when the text
    /// doesn't match any of the grammatical forms.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        let stripped = text.trim();
        if stripped.is_empty() {
            return None;
        }
        if stripped == "default" {
            return Some(Self {
                mode: MonitorMode::Default,
                raw: stripped.to_owned(),
                ..Default::default()
            });
        }
        if stripped == "none" {
            return Some(Self {
                mode: MonitorMode::None,
                raw: stripped.to_owned(),
                ..Default::default()
            });
        }
        if stripped.starts_with("min ") {
            return Self::parse_min_of(stripped);
        }
        Self::parse_and_chain(stripped)
    }

    fn parse_min_of(text: &str) -> Option<Self> {
        let rest_start = "min ".len();
        let rest = text[rest_start..].trim_start();
        let of_idx = rest.find(" of ")?;
        let minimum: i64 = rest[..of_idx].trim().parse().ok()?;
        let body_start_in_rest = of_idx + " of ".len();
        let body = rest[body_start_in_rest..].trim_start();
        if !body.starts_with('{') || !body.trim_end().ends_with('}') {
            return None;
        }
        // Walk *text* to find each monitor token between { and }.
        let bytes = text.as_bytes();
        let inner_open = text.find('{')?;
        let mut monitors: Vec<String> = Vec::new();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut i = inner_open + 1;
        let length = text.len();
        while i < length && bytes[i] != b'}' {
            if is_py_space(bytes[i]) {
                i += 1;
                continue;
            }
            let tok_start = i;
            while i < length && !is_py_space(bytes[i]) && bytes[i] != b'}' {
                i += 1;
            }
            monitors.push(text[tok_start..i].to_owned());
            spans.push((tok_start, i));
        }
        if monitors.is_empty() {
            return None;
        }
        Some(Self {
            mode: MonitorMode::MinOf,
            monitors,
            minimum: Some(minimum),
            raw: text.to_owned(),
            monitor_spans: spans,
        })
    }

    fn parse_and_chain(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let length = text.len();
        let mut monitors: Vec<String> = Vec::new();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        let mut expect_monitor = true;
        while i < length {
            if is_py_space(bytes[i]) {
                i += 1;
                continue;
            }
            let tok_start = i;
            while i < length && !is_py_space(bytes[i]) {
                i += 1;
            }
            let token = &text[tok_start..i];
            if expect_monitor {
                if token.is_empty() {
                    return None;
                }
                monitors.push(token.to_owned());
                spans.push((tok_start, i));
                expect_monitor = false;
            } else {
                if token != "and" {
                    return None;
                }
                expect_monitor = true;
            }
        }
        if monitors.is_empty() {
            return None;
        }
        let mode = if monitors.len() == 1 {
            MonitorMode::Single
        } else {
            MonitorMode::All
        };
        Some(Self {
            mode,
            monitors,
            minimum: None,
            raw: text.to_owned(),
            monitor_spans: spans,
        })
    }

    /// `true` for the `default` mode.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.mode == MonitorMode::Default
    }

    /// `true` for the `none` mode.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.mode == MonitorMode::None
    }

    /// Canonical scalar spelling of the expression.
    #[must_use]
    pub fn full_path(&self) -> String {
        if self.mode == MonitorMode::Single && !self.monitors.is_empty() {
            return self.monitors[0].clone();
        }
        if self.mode == MonitorMode::Default {
            return "default".to_owned();
        }
        if self.mode == MonitorMode::None {
            return "none".to_owned();
        }
        self.to_string()
    }

    /// Leaf name of the (first) referenced monitor.
    #[must_use]
    pub fn name(&self) -> String {
        match self.monitors.first() {
            None => String::new(),
            Some(m) => m.rsplit('/').next().unwrap_or(m).to_owned(),
        }
    }

    /// The full-paths this expression references.
    #[must_use]
    pub fn references(&self) -> &[String] {
        &self.monitors
    }

    /// Number of referenced monitors (Python `__len__`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.monitors.len()
    }

    /// `true` when there are no referenced monitors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.monitors.is_empty()
    }

    /// Python `__bool__`: truthy unless `default` mode with empty `raw`.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        self.mode != MonitorMode::Default || !self.raw.is_empty()
    }

    /// Iterate the referenced monitor paths (Python `__iter__`).
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.monitors.iter()
    }
}

impl<'a> IntoIterator for &'a MonitorExpression {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.monitors.iter()
    }
}

impl fmt::Display for MonitorExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty()
            && let Some(reparsed) = MonitorExpression::try_parse(&self.raw)
            && reparsed.mode == self.mode
            && reparsed.monitors == self.monitors
            && reparsed.minimum == self.minimum
        {
            return f.write_str(&self.raw);
        }
        match self.mode {
            MonitorMode::Default => f.write_str("default"),
            MonitorMode::None => f.write_str("none"),
            MonitorMode::Single => f.write_str(self.monitors.first().map_or("", String::as_str)),
            MonitorMode::All => f.write_str(&self.monitors.join(" and ")),
            MonitorMode::MinOf => {
                let inner = self.monitors.join(" ");
                let min = self.minimum.unwrap_or(0);
                write!(f, "min {min} of {{ {inner} }}")
            }
        }
    }
}

/// Python `str.isspace()` for the ASCII whitespace characters that appear
/// in monitor expressions.
fn is_py_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_none() {
        let m = MonitorExpression::try_parse("default").unwrap();
        assert_eq!(m.mode, MonitorMode::Default);
        assert_eq!(m.to_string(), "default");
        let m = MonitorExpression::try_parse("none").unwrap();
        assert_eq!(m.mode, MonitorMode::None);
        assert_eq!(m.to_string(), "none");
    }

    #[test]
    fn single() {
        let m = MonitorExpression::try_parse("/Common/http").unwrap();
        assert_eq!(m.mode, MonitorMode::Single);
        assert_eq!(m.monitors, vec!["/Common/http".to_owned()]);
        assert_eq!(m.to_string(), "/Common/http");
        assert_eq!(m.full_path(), "/Common/http");
        assert_eq!(m.name(), "http");
    }

    #[test]
    fn and_chain() {
        let m = MonitorExpression::try_parse("/Common/http and /Common/tcp").unwrap();
        assert_eq!(m.mode, MonitorMode::All);
        assert_eq!(m.monitors.len(), 2);
        assert_eq!(m.to_string(), "/Common/http and /Common/tcp");
    }

    #[test]
    fn min_of() {
        let m = MonitorExpression::try_parse(
            "min 2 of { /Common/gateway_icmp /Common/http /Common/http2 }",
        )
        .unwrap();
        assert_eq!(m.mode, MonitorMode::MinOf);
        assert_eq!(m.minimum, Some(2));
        assert_eq!(m.monitors.len(), 3);
        assert_eq!(
            m.to_string(),
            "min 2 of { /Common/gateway_icmp /Common/http /Common/http2 }"
        );
        // span check
        let (s, e) = m.monitor_spans[0];
        assert_eq!(&m.raw[s..e], "/Common/gateway_icmp");
    }

    #[test]
    fn structured_render_min_of() {
        let m = MonitorExpression {
            mode: MonitorMode::MinOf,
            monitors: vec!["/Common/a".to_owned(), "/Common/b".to_owned()],
            minimum: Some(1),
            raw: String::new(),
            monitor_spans: Vec::new(),
        };
        assert_eq!(m.to_string(), "min 1 of { /Common/a /Common/b }");
    }

    #[test]
    fn rejects_bad_chain() {
        assert!(MonitorExpression::try_parse("/Common/a or /Common/b").is_none());
        assert!(MonitorExpression::try_parse("").is_none());
    }
}
