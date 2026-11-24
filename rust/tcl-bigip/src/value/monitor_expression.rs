// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed BIG-IP monitor-expression value.

use std::fmt;

use crate::parser::{ShellToken, ShellTokenKind, expression_tokens};

/// `monitor default` — the parent object's default selection.
pub const DEFAULT_WORD: &str = "default";
/// `monitor none` — no monitoring.
pub const NONE_WORD: &str = "none";
/// The `min` of `monitor min N of { … }`.
pub const MIN_WORD: &str = "min";
/// The `of` of `monitor min N of { … }`.
pub const OF_WORD: &str = "of";
/// The conjunction of `monitor M1 and M2`.
pub const AND_WORD: &str = "and";

/// Every word of the `monitor` expression grammar, as
/// [`MonitorExpression::try_parse`] reads it. Static editor grammars project
/// this list (`cargo xtask gen-bigip-grammars`) to paint an expression's own
/// words — a lexical display, never a parse of the expression.
pub const GRAMMAR_WORDS: [&str; 5] = [DEFAULT_WORD, NONE_WORD, MIN_WORD, OF_WORD, AND_WORD];

/// The grammatical shape of a parsed `monitor` expression.
///
/// [`MonitorMode::as_str`] returns the canonical string literal for
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
    /// The canonical string literal for this mode.
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
    /// Parse an LTM monitor expression, keeping its original spelling and
    /// the source extent of each monitor reference.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        let tokens = complete_tokens(text)?;
        let mut expression = if let [token] = tokens.as_slice()
            && let Some(inner) = token.spelling().strip_prefix('"')
            && let Some(inner) = inner.strip_suffix('"')
        {
            let mut expression = Self::parse_tokens(&complete_tokens(inner)?)?;
            let offset = token.range.start + 1;
            for (start, end) in &mut expression.monitor_spans {
                *start += offset;
                *end += offset;
            }
            if expression.mode == MonitorMode::Single {
                expression.monitor_spans[0] = (token.range.start, token.range.end);
            }
            expression
        } else {
            Self::parse_tokens(&tokens)?
        };
        text.clone_into(&mut expression.raw);
        Some(expression)
    }

    fn parse_tokens(tokens: &[ShellToken<'_>]) -> Option<Self> {
        let (first, rest) = tokens.split_first()?;
        let mut expression = Self::default();
        expression.mode = match (first.spelling(), rest) {
            (DEFAULT_WORD, []) => MonitorMode::Default,
            (NONE_WORD, []) => MonitorMode::None,
            (MIN_WORD, [threshold, of, open, members @ .., close])
                if of.spelling() == OF_WORD
                    && open.kind == ShellTokenKind::Open
                    && close.kind == ShellTokenKind::Close =>
            {
                let minimum: u16 = threshold.spelling().parse().ok()?;
                if minimum == 0 || usize::from(minimum) > members.len() {
                    return None;
                }
                for token in members {
                    expression.push_monitor(token)?;
                }
                expression.minimum = Some(i64::from(minimum));
                MonitorMode::MinOf
            }
            _ => {
                expression.push_monitor(first)?;
                for pair in rest.chunks(2) {
                    let [and, monitor] = pair else {
                        return None;
                    };
                    if and.spelling() != AND_WORD {
                        return None;
                    }
                    expression.push_monitor(monitor)?;
                }
                if rest.is_empty() {
                    MonitorMode::Single
                } else {
                    MonitorMode::All
                }
            }
        };
        Some(expression)
    }

    fn push_monitor(&mut self, token: &ShellToken<'_>) -> Option<()> {
        let name = token.word_value()?;
        if name.is_empty() || GRAMMAR_WORDS.contains(&name.as_ref()) {
            return None;
        }
        self.monitors.push(name.into_owned());
        self.monitor_spans
            .push((token.range.start, token.range.end));
        Some(())
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

    /// Number of referenced monitors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.monitors.len()
    }

    /// `true` when there are no referenced monitors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.monitors.is_empty()
    }

    /// Truthy unless `default` mode with empty `raw`.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        self.mode != MonitorMode::Default || !self.raw.is_empty()
    }

    /// Iterate the referenced monitor paths.
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

fn complete_tokens(text: &str) -> Option<Vec<ShellToken<'_>>> {
    let tokens: Vec<_> = expression_tokens(text).collect();
    tokens.iter().all(|token| token.complete).then_some(tokens)
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
    fn measured_ltm_monitor_grammar() {
        let members = ["/Common/http", "/Common/https"];
        for (raw, mode, minimum) in [
            (
                r#""/Common/http" and "/Common/https""#,
                MonitorMode::All,
                None,
            ),
            (
                r#""/Common/http and /Common/https""#,
                MonitorMode::All,
                None,
            ),
            (
                "min 1 of { /Common/http /Common/https }",
                MonitorMode::MinOf,
                Some(1),
            ),
            (
                "min 2 of { /Common/http /Common/https }",
                MonitorMode::MinOf,
                Some(2),
            ),
            (
                r#"  "min 1 of { /Common/http /Common/https }"  "#,
                MonitorMode::MinOf,
                Some(1),
            ),
        ] {
            let parsed = MonitorExpression::try_parse(raw).unwrap();
            assert_eq!(parsed.mode, mode, "{raw}");
            assert_eq!(parsed.minimum, minimum, "{raw}");
            assert_eq!(parsed.references(), members, "{raw}");
            assert_eq!(parsed.raw, raw);
            assert_eq!(parsed.to_string(), raw);
            for ((start, end), member) in parsed.monitor_spans.into_iter().zip(members) {
                assert_eq!(raw[start..end].trim_matches('"'), member);
            }
        }
        for raw in [
            "min 1 of { /Common/http and /Common/https }",
            "/Common/http and",
            "/Common/http or /Common/https",
            "min 0 of { /Common/http /Common/https }",
            "min -1 of { /Common/http and /Common/https }",
            "min 3 of { /Common/http /Common/https }",
            "min 65536 of { /Common/http /Common/https }",
            "min 1 of { /Common/http } ignored }",
            r#""/Common/http and""#,
            r#""/Common/http and /Common/https"#,
        ] {
            assert!(MonitorExpression::try_parse(raw).is_none(), "{raw}");
        }
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
