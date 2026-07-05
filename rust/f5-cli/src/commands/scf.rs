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

//! tmsh → SCF normalisation.
//!
//! The config verbs accept `tmsh create` / `tmsh modify` script output as well
//! as SCF. Without this, the SCF parser would read `tmsh` as the module name and
//! record generic objects instead of the real pools / virtuals / etc. `to_scf`
//! strips the top-level `tmsh create`/`modify` prefixes (only at brace depth
//! zero, so an embedded `tmsh create …` inside an iRule body survives).

const TMSH_PREFIXES: &[&str] = &["tmsh create ", "tmsh modify "];

/// Normalise tmsh output to SCF when it looks like tmsh, else return it
/// unchanged.
#[must_use]
pub fn to_scf(text: &str) -> String {
    if looks_like_tmsh_output(text) {
        normalise_tmsh_to_scf(text)
    } else {
        text.to_owned()
    }
}

/// True when the first non-blank, non-comment line starts with a tmsh prefix.
fn looks_like_tmsh_output(text: &str) -> bool {
    for line in text.lines() {
        let stripped = line.trim_start();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        return TMSH_PREFIXES.iter().any(|p| stripped.starts_with(p));
    }
    false
}

/// Strip `tmsh create`/`modify` prefixes from stanza headers at brace depth
/// zero.
fn normalise_tmsh_to_scf(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth: i32 = 0;
    for line in text.split_inclusive('\n') {
        let mut rendered = line.to_owned();
        if depth == 0 {
            let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            let (indent, rest) = line.split_at(indent_len);
            for prefix in TMSH_PREFIXES {
                if let Some(stripped) = rest.strip_prefix(prefix) {
                    rendered = format!("{indent}{stripped}");
                    break;
                }
            }
        }
        depth += net_brace_delta(&rendered);
        out.push_str(&rendered);
    }
    out
}

/// `{` minus `}` count outside quoted strings and comments: a `#` outside a
/// string starts a Tcl-style comment.
fn net_brace_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
        } else if !in_string {
            match c {
                b'#' => break,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    depth
}
