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

/// Render roles emitted by the selected engine for this pattern and invocation.
pub(super) fn regex_hover_text(text: &str, components: &[tcl_regex::PatternComponent]) -> String {
    use tcl_regex::PatternRole as Role;
    let mut parts = vec!["**Regex pattern**\n".to_owned()];
    let mut seen = FxHashSet::default();
    if !components.is_empty() {
        parts.push("| Component | Meaning |".to_owned());
        parts.push("|-----------|---------|".to_owned());
    }
    for component in components {
        let spelling = &text[component.range.clone()];
        if !seen.insert((spelling, component.role)) {
            continue;
        }
        let meaning = match component.role {
            Role::Options => "Pattern options",
            Role::Comment => "Pattern comment",
            Role::Anchor => "Position constraint",
            Role::CharacterClass => "Character class",
            Role::Alternation => "Alternation",
            Role::CaptureOpen => "Group open",
            Role::NoncaptureOpen => "Non-capturing group",
            Role::PositiveLookahead => "Positive lookahead",
            Role::NegativeLookahead => "Negative lookahead",
            Role::GroupClose => "Group close",
            Role::Quantifier => "Repetition",
            Role::Escape => "Escaped character",
            Role::Backreference => "Backreference",
        };
        parts.push(format!("| `{}` | {meaning} |", gfm_table_cell(spelling)));
    }
    parts.join("\n")
}
