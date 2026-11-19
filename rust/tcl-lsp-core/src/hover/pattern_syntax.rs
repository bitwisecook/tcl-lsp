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
            let description = match group {
                0 => "Entire matched string".to_owned(),
                1 => "First capture group".to_owned(),
                2 => "Second capture group".to_owned(),
                _ => format!("Capture group {group}"),
            };
            parts.push(format!("| `{spelling}` | {description} |"));
        }
    }
    parts.join("\n")
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
pub(super) fn glob_hover_text(
    text: &str,
    components: &[tcl_syntax::glob::GlobComponent],
) -> String {
    use tcl_syntax::glob::GlobRole;
    let mut parts = vec!["**Glob pattern**\n".to_owned()];
    if !components.is_empty() {
        parts.push("| Pattern | Meaning |".to_owned());
        parts.push("|---------|---------|".to_owned());
    }
    let mut seen = FxHashSet::default();
    for component in components {
        let spelling = &text[component.range.clone()];
        if !seen.insert(spelling) {
            continue;
        }
        let meaning = match component.role {
            GlobRole::AnySequence => "Matches any sequence of characters",
            GlobRole::AnyCharacter => "Matches a single character",
            GlobRole::CharacterClass => "Character class",
            GlobRole::Escape => "Escaped character",
        };
        parts.push(format!("| `{}` | {meaning} |", gfm_table_cell(spelling)));
    }
    parts.join("\n")
}

/// Render roles emitted by the selected engine for this pattern and invocation.
pub(super) fn regex_hover_text(text: &str, components: &[tcl_regex::PatternComponent]) -> String {
    use tcl_regex::PatternRole as Role;
    let mut parts = vec!["**Regex pattern**\n".to_owned()];
    let mut seen = FxHashSet::default();
    if components.is_empty() && text.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        parts.push("Literal string (no metacharacters).".to_owned());
    }
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
            Role::Anchor => "Zero-width position anchor",
            Role::CharacterClass => "Character class",
            Role::Alternation => "Alternation",
            Role::CaptureOpen => "Group open",
            Role::NoncaptureOpen => "Non-capturing group",
            Role::PositiveLookahead => "Positive lookahead",
            Role::NegativeLookahead => "Negative lookahead",
            Role::GroupClose => "Group close",
            Role::Quantifier => match spelling {
                "+" | "+?" => "One or more repetitions",
                "*" | "*?" => "Zero or more repetitions",
                "?" | "??" => "Zero or one repetition",
                _ => "Bounded repetition",
            },
            Role::Escape => "Escaped character",
            Role::Backreference => "Backreference",
        };
        parts.push(format!("| `{}` | {meaning} |", gfm_table_cell(spelling)));
    }
    parts.join("\n")
}
