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

//! Component tables for `regsub` substitutions, glob patterns and regular expressions.

use rustc_hash::FxHashSet;

/// Capture references and their actual group numbers in a replacement value.
pub(super) fn scan_regsub_backrefs(text: &str) -> Vec<(&str, u8)> {
    tcl_cmd_core::regex::replacement_parts(text.as_bytes())
        .filter_map(|(range, part)| match part {
            tcl_cmd_core::regex::ReplacementPart::Capture(group) => Some((&text[range], group)),
            tcl_cmd_core::regex::ReplacementPart::Literal(_) => None,
        })
        .collect()
}

/// Render the regsub substitution-spec hover markdown.
pub(super) fn regsub_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Substitution spec** (regsub)\n".to_string()];
    let refs = scan_regsub_backrefs(text);
    if refs.is_empty() {
        parts.push("No backreferences found.".to_string());
    } else {
        parts.push("| Reference | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for (spelling, group) in refs {
            let description = if group == 0 {
                "Entire matched string".to_owned()
            } else {
                format!("Capture group {group}")
            };
            parts.push(format!("| `{spelling}` | {description} |"));
        }
    }
    parts.join("\n")
}

/// Glob metacharacter descriptions.
fn glob_meta_desc(c: char) -> Option<&'static str> {
    match c {
        '*' => Some("Matches any sequence of characters"),
        '?' => Some("Matches any single character"),
        '[' => Some("Character class — matches any character inside brackets"),
        _ => None,
    }
}

/// Scan a glob pattern for metacharacters.  Returns a list of
/// `(token, description)` tuples — `*`, `?`, character class
/// `[abc]`, and escape sequences. +
/// `_glob_hover`'s metacharacter walk.
pub(super) fn scan_glob_metachars(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            let key = "escape".to_string();
            if !seen.contains(&key) {
                out.push((format!("\\{next}"), format!("Escaped character `{next}`")));
                seen.insert(key);
            }
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            let start = i;
            let mut end = i + 1;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            let token: String = if end < chars.len() {
                chars[start..=end].iter().collect()
            } else {
                chars[start..end].iter().collect()
            };
            if !seen.contains(&token) {
                let inner: String = chars[start + 1..end].iter().collect();
                out.push((
                    token.clone(),
                    format!("Character class: matches any of `{inner}`"),
                ));
                seen.insert(token);
            }
            i = end + 1;
            continue;
        }
        if let Some(desc) = glob_meta_desc(chars[i]) {
            let key = chars[i].to_string();
            if !seen.contains(&key) {
                out.push((key.clone(), desc.to_string()));
                seen.insert(key);
            }
        }
        i += 1;
    }
    out
}

/// Escape a value for a GFM table cell: a literal `|` closes the cell even
/// inside an inline code span, so it must be backslash-escaped (`\|`, which
/// GFM renders back as `|`).  Applied to the regex/glob hover token and
/// meaning cells, whose content is user-pattern-derived and may contain `|`
/// (e.g. an alternation `foo|bar`) that would otherwise break the row in
/// strict GFM renderers (issue 187).
fn gfm_table_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Render the glob-pattern hover markdown.
pub(super) fn glob_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Glob pattern**\n".to_string()];
    let metas = scan_glob_metachars(text);
    if metas.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Pattern | Meaning |".to_string());
        parts.push("|---------|---------|".to_string());
        for (tok, desc) in metas {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}

/// Regex metacharacter descriptions.
fn regex_meta_desc(token: &str) -> Option<&'static str> {
    match token {
        "^" => Some("Start of line/string anchor"),
        "$" => Some("End of line/string anchor"),
        "." => Some("Match any single character"),
        "*" => Some("Zero or more (greedy)"),
        "+" => Some("One or more (greedy)"),
        "?" => Some("Zero or one (greedy)"),
        "*?" => Some("Zero or more (lazy)"),
        "+?" => Some("One or more (lazy)"),
        "??" => Some("Zero or one (lazy)"),
        "|" => Some("Alternation (OR)"),
        _ => None,
    }
}

/// Regex escape descriptions for common shorthand classes.
fn regex_escape_desc(token: &str) -> Option<&'static str> {
    match token {
        "\\d" => Some("Digit `[0-9]`"),
        "\\D" => Some("Non-digit"),
        "\\s" => Some("Whitespace"),
        "\\S" => Some("Non-whitespace"),
        "\\w" => Some("Word character `[a-zA-Z0-9_]`"),
        "\\W" => Some("Non-word character"),
        "\\b" => Some("Word boundary"),
        "\\B" => Some("Non-word boundary"),
        "\\A" => Some("Start of string"),
        "\\Z" => Some("End of string"),
        "\\z" => Some("End of string (Tcl 9.1+)"),
        "\\n" => Some("Newline"),
        "\\t" => Some("Tab"),
        "\\r" => Some("Carriage return"),
        _ => None,
    }
}

/// One emitted regex-token entry — `(consumed, key, tok, desc)`.
/// `consumed` is the number of source chars the token covers;
/// `key` is the dedup key; `tok` is what the table renders;
/// `desc` is the explanation.  Returned by each sub-scanner so
/// the outer loop in [`scan_regex_components`] stays readable.
type RegexComp = (usize, String, String, String);

/// Scan a regex pattern for metacharacters / classes /
/// escapes.  Handles common cases:
/// anchors, quantifiers, alternation, character classes,
/// shorthand escapes, capture-group parens, lazy
/// quantifiers.
pub(super) fn scan_regex_components(text: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut i = 0;
    while i < chars.len() {
        // The sub-scanners are tried in order: escape and char
        // class are eager (they consume multi-char windows);
        // lazy-quantifier has to run before single-char meta so
        // `*?` doesn't get split.  Group-open also runs before
        // single-char meta so `(?:` / `(` get attributed
        // correctly.
        let token = scan_regex_escape(&chars, i)
            .or_else(|| scan_regex_char_class(&chars, i))
            .or_else(|| scan_regex_lazy_quantifier(&chars, i))
            .or_else(|| scan_regex_group(&chars, i))
            .or_else(|| scan_regex_single_meta(chars[i]));
        match token {
            Some((consumed, key, tok, desc)) => {
                if seen.insert(key) {
                    out.push((tok, desc));
                }
                i += consumed.max(1);
            }
            None => i += 1,
        }
    }
    out
}

/// `\<char>` escape sequences — shorthand classes (`\d`, `\w`),
/// numbered backreferences (`\1`-`\9`), and escaped literals
/// (`\.`, `\*`, …).  Falls back to a generic "Escape sequence"
/// label for unknown payloads.
fn scan_regex_escape(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'\\' {
        return None;
    }
    let next = *chars.get(i + 1)?;
    let tok = format!("\\{next}");
    let desc = if next.is_ascii_digit() {
        format!("Backreference to group {next}")
    } else if let Some(d) = regex_escape_desc(&tok) {
        d.to_string()
    } else if ".*+?(){}[]|^$\\".contains(next) {
        format!("Escaped literal `{next}`")
    } else {
        format!("Escape sequence `{tok}`")
    };
    Some((2, tok.clone(), tok, desc))
}

/// `[...]` character classes, including leading `^` negation
/// and a literal `]` as first char per regex grammar.
/// Consumes the entire class including the closing `]` (or to
/// EOL when the pattern is malformed).
fn scan_regex_char_class(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'[' {
        return None;
    }
    let start = i;
    let mut end = i + 1;
    if chars.get(end) == Some(&'^') {
        end += 1;
    }
    if chars.get(end) == Some(&']') {
        end += 1;
    }
    while end < chars.len() && chars[end] != ']' {
        if chars[end] == '\\' && end + 1 < chars.len() {
            end += 2;
        } else {
            end += 1;
        }
    }
    let (tok_slice, consumed) = if end < chars.len() {
        (&chars[start..=end], end + 1 - start)
    } else {
        (&chars[start..end], end - start)
    };
    let tok: String = tok_slice.iter().collect();
    let inner: String = if tok.starts_with('[') && tok.ends_with(']') {
        tok[1..tok.len() - 1].to_string()
    } else {
        tok[1..].to_string()
    };
    let desc = format!("Character class: matches any of `{inner}`");
    Some((consumed, tok.clone(), tok, desc))
}

/// Lazy quantifiers — `*?`, `+?`, `??`.  Must run before
/// [`scan_regex_single_meta`] so `*` alone doesn't claim the
/// pair.
fn scan_regex_lazy_quantifier(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if !matches!(c, '*' | '+' | '?') {
        return None;
    }
    if chars.get(i + 1) != Some(&'?') {
        return None;
    }
    let tok = format!("{c}?");
    let desc = regex_meta_desc(&tok)?.to_string();
    Some((2, tok.clone(), tok, desc))
}

/// Grouping — `(?:`, `(?=`, `(?!`, `(?>`, and bare `(` / `)`.
fn scan_regex_group(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if c == ')' {
        return Some((1, ")".into(), ")".into(), "Group close".into()));
    }
    if c != '(' {
        return None;
    }
    if chars.get(i + 1) == Some(&'?')
        && let Some(trail) = chars.get(i + 2)
    {
        let pair = match trail {
            ':' => Some(("(?:", "Non-capturing group")),
            '=' => Some(("(?=", "Positive lookahead")),
            '!' => Some(("(?!", "Negative lookahead")),
            '>' => Some(("(?>", "Atomic (possessive) group")),
            _ => None,
        };
        if let Some((tok, desc)) = pair {
            return Some((3, tok.to_string(), tok.to_string(), desc.to_string()));
        }
    }
    Some((1, "(".into(), "(".into(), "Capture group open".into()))
}

/// Single-char metacharacters — `^`, `$`, `.`, `*`, `+`, `?`,
/// `|`.  Anything [`regex_meta_desc`] knows about.
fn scan_regex_single_meta(c: char) -> Option<RegexComp> {
    let key = c.to_string();
    let desc = regex_meta_desc(&key)?.to_string();
    Some((1, key.clone(), key, desc))
}

/// Render a regex-pattern hover markdown.
pub(super) fn regex_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Regex pattern**\n".to_string()];
    let comps = scan_regex_components(text);
    if comps.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Component | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for (tok, desc) in comps {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}
