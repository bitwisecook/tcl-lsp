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

//! Static mini-evaluator for `lappend auto_path` / `set auto_path`
//! arguments.
//!
//! A common Tcl idiom adds a sibling / parent directory to the package
//! search path using only a handful of side-effect-free built-ins:
//!
//! ```tcl
//! lappend auto_path [file join [file dirname [info script]] ..]
//! lappend auto_path [file dirname [file dirname [info script]]]
//! lappend auto_path /abs/path
//! lappend auto_path ~/lib
//! ```
//!
//! The supported subset is `[info script]`, `[file dirname …]`,
//! `[file join …]`, literal words, and `~`-prefixed paths.  Variable
//! substitutions (`$auto_path`, `$env(…)`) and list-manipulation
//! commands are *not* evaluated — anything outside the subset returns
//! `None` and the caller skips the entry.  We never guess.
//!
//! The `signature_scan` handler records only the raw literal of
//! each entry; this evaluator resolves those idioms.  It is consulted
//! when a workspace / package index has the analysed file's path (the
//! `[info script]` value) available.

/// A parsed word: a literal, or a command substitution
/// `[name arg …]`.
enum Node {
    Lit(String),
    Cmd(String, Vec<Node>),
}

/// Resolve `raw` (a `lappend auto_path` argument) to an absolute path,
/// substituting `info_script` for `[info script]` (the filesystem path
/// of the file being analysed — what Tcl itself would report).
///
/// Returns `None` when any part of the expression is outside the
/// supported subset.
#[must_use]
pub fn evaluate_auto_path_expr(raw: &str, info_script: Option<&str>) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let tokens = tokenise(raw)?;
    let (node, rest) = parse(&tokens, 0);
    let node = node?;
    if rest != tokens.len() {
        return None;
    }
    let result = eval(&node, info_script)?;
    // Expand `~` and make absolute (collapsing `..`) so the parent-dir
    // idiom resolves to a real directory.
    Some(abspath(&expanduser(&result)))
}

/// Split `text` into `[` / `]` / word tokens.  Brace groups are kept
/// as a single word (brackets inside braces are literal); an
/// unterminated brace / quote aborts with `None`.  One quirk: a brace
/// word whose content is exactly `[` reads as an open bracket in
/// `parse` (the `{[}`-as-open-bracket case).
fn tokenise(text: &str) -> Option<Vec<String>> {
    let b: Vec<char> = text.chars().collect();
    let n = b.len();
    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;
    while i < n {
        let ch = b[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '[' || ch == ']' {
            tokens.push(ch.to_string());
            i += 1;
            continue;
        }
        if ch == '{' {
            let mut depth = 1;
            let mut j = i + 1;
            while j < n && depth > 0 {
                if b[j] == '{' {
                    depth += 1;
                } else if b[j] == '}' {
                    depth -= 1;
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            tokens.push(b[i + 1..j - 1].iter().collect());
            i = j;
            continue;
        }
        if ch == '"' {
            let mut j = i + 1;
            while j < n && b[j] != '"' {
                if b[j] == '\\' && j + 1 < n {
                    j += 2;
                    continue;
                }
                j += 1;
            }
            if j >= n {
                return None;
            }
            tokens.push(b[i + 1..j].iter().collect());
            i = j + 1;
            continue;
        }
        let mut j = i;
        while j < n && !b[j].is_whitespace() && b[j] != '[' && b[j] != ']' {
            j += 1;
        }
        tokens.push(b[i..j].iter().collect());
        i = j;
    }
    Some(tokens)
}

/// Parse a single word starting at `pos`; returns `(node, next_pos)`.
fn parse(tokens: &[String], mut pos: usize) -> (Option<Node>, usize) {
    let Some(tok) = tokens.get(pos) else {
        return (None, pos);
    };
    if tok == "[" {
        pos += 1;
        let Some(name) = tokens.get(pos) else {
            return (None, pos);
        };
        let name = name.clone();
        pos += 1;
        let mut args: Vec<Node> = Vec::new();
        while pos < tokens.len() && tokens[pos] != "]" {
            let (node, next) = parse(tokens, pos);
            pos = next;
            match node {
                Some(n) => args.push(n),
                None => return (None, pos),
            }
        }
        if pos >= tokens.len() {
            return (None, pos);
        }
        pos += 1; // consume `]`
        return (Some(Node::Cmd(name, args)), pos);
    }
    if tok == "]" {
        return (None, pos);
    }
    (Some(Node::Lit(tok.clone())), pos + 1)
}

/// Evaluate a parsed node to a path string (pre-`abspath`).
fn eval(node: &Node, info_script: Option<&str>) -> Option<String> {
    match node {
        Node::Lit(value) => {
            // Reject any word still carrying a variable substitution —
            // we refuse to guess.
            if value.contains('$') {
                None
            } else {
                Some(value.clone())
            }
        }
        Node::Cmd(name, args) => {
            if name == "info" && args.len() == 1 {
                return match eval(&args[0], info_script).as_deref() {
                    Some("script") => info_script.map(str::to_owned),
                    _ => None,
                };
            }
            if name == "file" && args.len() >= 2 {
                let sub = eval(&args[0], info_script)?;
                if sub == "dirname" && args.len() == 2 {
                    return Some(posix_dirname(&eval(&args[1], info_script)?));
                }
                if sub == "join" && args.len() >= 2 {
                    let mut parts: Vec<String> = Vec::new();
                    for a in &args[1..] {
                        parts.push(eval(a, info_script)?);
                    }
                    if parts.is_empty() {
                        return None;
                    }
                    return Some(posix_join(&parts));
                }
            }
            None
        }
    }
}

/// POSIX path dirname — everything up to (not including) the last
/// `/`, collapsing trailing slashes on the head unless it is all
/// slashes (the root).
fn posix_dirname(p: &str) -> String {
    let i = p.rfind('/').map_or(0, |idx| idx + 1);
    let head = &p[..i];
    if !head.is_empty() && head.bytes().any(|c| c != b'/') {
        head.trim_end_matches('/').to_owned()
    } else {
        head.to_owned()
    }
}

/// POSIX path join — a later absolute component resets the
/// accumulator; otherwise components are joined with a single `/`.
fn posix_join(parts: &[String]) -> String {
    let mut result = parts[0].clone();
    for p in &parts[1..] {
        if p.starts_with('/') {
            result.clone_from(p);
        } else if result.is_empty() || result.ends_with('/') {
            result.push_str(p);
        } else {
            result.push('/');
            result.push_str(p);
        }
    }
    result
}

/// POSIX path tilde-expansion for the bare `~` and `~/…` forms; a
/// `~user` form is left unchanged (no passwd lookup).  `~` expands to
/// `$HOME` when set.
fn expanduser(p: &str) -> String {
    if !p.starts_with('~') {
        return p.to_owned();
    }
    // Only the bare `~` (followed by `/` or end) is expanded.
    let after = &p[1..];
    if (after.is_empty() || after.starts_with('/'))
        && let Ok(home) = std::env::var("HOME")
    {
        let home = home.trim_end_matches('/');
        return format!("{home}{after}");
    }
    p.to_owned()
}

/// POSIX path absolutisation — make `p` absolute against the current
/// working directory, then normalise (`.` / `..` / `//`).
fn abspath(p: &str) -> String {
    let joined = if p.starts_with('/') {
        p.to_owned()
    } else {
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|c| c.to_str().map(str::to_owned))
            .unwrap_or_else(|| "/".to_owned());
        posix_join(&[cwd, p.to_owned()])
    };
    normpath(&joined)
}

/// POSIX path normalisation — collapse `.` / `..` / repeated slashes.
/// A leading `//` (exactly two) is preserved per POSIX; three or more
/// collapse to one.
fn normpath(p: &str) -> String {
    if p.is_empty() {
        return ".".to_owned();
    }
    let initial_slashes = if p.starts_with('/') {
        if p.starts_with("//") && !p.starts_with("///") {
            2
        } else {
            1
        }
    } else {
        0
    };
    let mut out: Vec<&str> = Vec::new();
    for comp in p.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp != ".." || (initial_slashes == 0 && out.is_empty()) || out.last() == Some(&"..") {
            out.push(comp);
        } else if !out.is_empty() {
            out.pop();
        }
    }
    let mut result = "/".repeat(initial_slashes);
    result.push_str(&out.join("/"));
    if result.is_empty() {
        ".".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Absolute `info_script` keeps `abspath` deterministic (it reduces
    // to `normpath`, independent of the test's cwd).

    #[test]
    fn dirname_of_info_script() {
        assert_eq!(
            evaluate_auto_path_expr("[file dirname [info script]]", Some("/proj/lib/init.tcl"))
                .as_deref(),
            Some("/proj/lib"),
        );
    }

    #[test]
    fn parent_dir_join_idiom_collapses_dotdot() {
        // [file join [file dirname [info script]] ..] → parent of the
        // script's directory.
        assert_eq!(
            evaluate_auto_path_expr(
                "[file join [file dirname [info script]] ..]",
                Some("/proj/lib/pkg/init.tcl"),
            )
            .as_deref(),
            Some("/proj/lib"),
        );
    }

    #[test]
    fn nested_dirname() {
        assert_eq!(
            evaluate_auto_path_expr(
                "[file dirname [file dirname [info script]]]",
                Some("/proj/lib/init.tcl"),
            )
            .as_deref(),
            Some("/proj"),
        );
    }

    #[test]
    fn literal_absolute_path() {
        assert_eq!(
            evaluate_auto_path_expr("/usr/local/lib", None).as_deref(),
            Some("/usr/local/lib"),
        );
    }

    #[test]
    fn variable_substitution_is_unsupported() {
        assert_eq!(evaluate_auto_path_expr("$auto_path", None), None);
        assert_eq!(
            evaluate_auto_path_expr("[file dirname $base]", Some("/a/b.tcl")),
            None,
        );
    }

    #[test]
    fn info_script_without_path_is_none() {
        assert_eq!(
            evaluate_auto_path_expr("[file dirname [info script]]", None),
            None
        );
    }

    #[test]
    fn unsupported_command_is_none() {
        assert_eq!(
            evaluate_auto_path_expr("[lsort {a b}]", Some("/a/b.tcl")),
            None,
        );
    }

    #[test]
    fn tilde_expands_to_home() {
        // `temp_env` scopes the `$HOME` mutation so this crate stays unsafe-free
        // (`set_var` is `unsafe` on edition 2024).
        temp_env::with_var("HOME", Some("/home/tester"), || {
            assert_eq!(
                evaluate_auto_path_expr("~/lib", None).as_deref(),
                Some("/home/tester/lib"),
            );
        });
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(evaluate_auto_path_expr("   ", None), None);
    }

    #[test]
    fn dirname_join_helpers_match_posix() {
        assert_eq!(posix_dirname("/a/b/c"), "/a/b");
        assert_eq!(posix_dirname("/a"), "/");
        assert_eq!(posix_dirname("a"), "");
        assert_eq!(posix_join(&["/a/b".into(), "..".into()]), "/a/b/..");
        assert_eq!(posix_join(&["/a".into(), "/b".into()]), "/b");
        assert_eq!(normpath("/a/b/../c"), "/a/c");
        assert_eq!(normpath("/a/./b"), "/a/b");
    }

    /// TP (issue #923 idx 73) — the raw text the analyser records for a
    /// `lappend auto_path …` argument is exactly what this evaluator folds, so
    /// the package database can consume `AnalysisResult::auto_path_entries`
    /// directly.  Pins the *contract between the two*, which is what the
    /// workspace scan's `extend_resolver_with_document_auto_paths` relies on.
    #[test]
    fn recorded_auto_path_entries_round_trip_through_the_evaluator() {
        let src = "lappend auto_path [file dirname [file dirname [info script]]]\n\
                   lappend auto_path $someVar\n";
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let folded: Vec<Option<String>> = r
            .auto_path_entries
            .iter()
            .map(|e| evaluate_auto_path_expr(&e.raw_path, Some("/a/b/c/user.tcl")))
            .collect();
        assert_eq!(
            folded,
            vec![Some("/a/b".to_owned()), None],
            "the computed idiom folds; a `$var` entry abstains: {:?}",
            r.auto_path_entries,
        );
    }
}
