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
//! `[file join …]`, `[file normalize …]` (anchored arguments only — see
//! [`eval_file_normalize`]), literal words, and `~`-prefixed paths.  Variable
//! substitutions (`$auto_path`, `$env(…)`) and list-manipulation
//! commands are *not* evaluated — anything outside the subset returns
//! `None` and the caller skips the entry.  We never guess.
//!
//! The `signature_scan` handler records only the raw literal of
//! each entry; this evaluator resolves those idioms.  It is consulted
//! when a workspace / package index has the analysed file's path (the
//! `[info script]` value) available.
//!
//! # Path space
//!
//! All path arithmetic here happens in Tcl's **slash form**, on every host.
//! That is not a POSIX assumption — it is what Tcl itself does: `filename(n)`
//! specifies that the `file` commands accept either separator on Windows and
//! *return* `/`-separated paths, so `[file dirname C:/x/y]` is `C:/x` in a
//! Windows interpreter exactly as it is written here.  A native path handed in
//! as `info_script` (`C:\repo\user.tcl`, straight out of `Uri::to_file_path`)
//! is converted at the boundary by [`to_tcl_slash_form`], and the result is
//! returned in slash form too — which `PathBuf::from` accepts on Windows.
//!
//! Three absolute roots are recognised (see [`root_len`]): POSIX `/…`, a
//! Windows drive `C:/…`, and a UNC share `//server/share/…`.  A *drive-relative*
//! path (`C:` or `C:foo`, resolved against a per-drive working directory only
//! the running process knows) is outside the subset and abstains, like any
//! other unsupported shape.
//!
//! # A relative result stays relative
//!
//! `lappend auto_path lib` / `set v helper.tcl; source $v` fold to a path
//! with no root.  Tcl resolves such a path against the **interpreter's
//! working directory at run time** (oracle, tclsh 8.6.16 and 9.0.4:
//! `cd other; tclsh ../sub/main.tcl` where `main.tcl` does
//! `set v helper.tcl; source $v` loads `other/helper.tcl`, *not*
//! `sub/helper.tcl` — `[info script]` plays no part), which is a fact no
//! static analysis can know.
//!
//! This evaluator therefore returns such a result **relative**, normalised
//! but unanchored, and each caller applies the base it actually has — the
//! sourcing document's directory for the source graph
//! (`source_graph::resolve_source_target`), the workspace root for document
//! links (`document_links::resolve_path`), the analysed file's own directory
//! for the package database. It deliberately does **not** absolutise against
//! the *analysing process's* current directory: the language server's launch
//! directory is an editor artefact with no relationship to the user's
//! program, so anchoring on it makes the same file resolve differently
//! depending on how the editor happened to be started, and makes a computed
//! path disagree with the literal path right beside it (which every caller
//! already anchors on its own base).

use crate::analyser::types::{AutoPathEntry, AutoPathForm};

/// A parsed word: a literal, or a command substitution
/// `[name arg …]`.
enum Node {
    Lit(String),
    Cmd(String, Vec<Node>),
}

/// Every directory one recorded [`AutoPathEntry`] adds to `auto_path`.
///
/// This is the supported way to read the fact, because the two mutation forms
/// differ in arity — see [`AutoPathForm`].  An [`AutoPathForm::Assign`] record
/// holds a Tcl *list*, so it is split with the shared list grammar
/// ([`tcl_syntax::list::split_list`]) before each element is folded; an
/// [`AutoPathForm::Append`] record is one directory and is folded whole, so a
/// braced `lappend auto_path {p q}` stays the single directory `p q`.
///
/// A *computed* right-hand side is never list-split.  The segmenter
/// reconstructs a substituted word with its `${var}` / `[cmd]` markers intact,
/// so their absence is reliable evidence the word is a plain literal (the same
/// test `SignatureSource::is_literal` uses).  `set auto_path [file dirname
/// [info script]]` is one computed directory, not three list elements, and is
/// handed whole to [`evaluate_auto_path_expr`]; `set auto_path [list /a /b]`
/// is outside the supported subset and contributes nothing.
///
/// A literal word, by contrast, is already a path *value* and is used as one —
/// re-parsing it would split `lappend auto_path {/opt/p q}` into two words and
/// lose a directory Tcl treats as a single element.
///
/// Entries outside the supported subset contribute nothing — we never guess.
#[must_use]
pub fn evaluate_auto_path_entry(entry: &AutoPathEntry, info_script: Option<&str>) -> Vec<String> {
    let raw = entry.raw_path.as_str();
    if raw.contains('$') || raw.contains('[') {
        // Computed: fold it as an expression, as one directory.
        return evaluate_auto_path_expr(raw, info_script)
            .into_iter()
            .collect();
    }
    if entry.form == AutoPathForm::Append {
        return fold_path_value(raw).into_iter().collect();
    }
    let Ok(elements) = tcl_syntax::list::split_list(raw) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|el| fold_path_value(el))
        .collect()
}

/// Fold one already-substituted path *value* (a `~` expansion plus lexical
/// normalisation), abstaining on a drive-relative path.  The shared tail of
/// [`evaluate_auto_path_expr`] and the per-element path of
/// [`evaluate_auto_path_entry`].
///
/// A rootless value stays rootless — see the module docs' "A relative result
/// stays relative".
fn fold_path_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let expanded = expanduser(&to_tcl_slash_form(value));
    if is_drive_relative(&expanded) {
        // `C:` / `C:foo` names a per-drive working directory only the running
        // process knows.  Outside the subset — abstain rather than guess.
        return None;
    }
    Some(normalise_path_value(&expanded))
}

/// Resolve `raw` (a `lappend auto_path` argument) to a normalised path,
/// substituting `info_script` for `[info script]` (the filesystem path
/// of the file being analysed — what Tcl itself would report).
///
/// The result is absolute whenever the expression is anchored — by
/// `[info script]`, by a literal root, or by `~` — and **relative**
/// otherwise, for the caller to anchor (see the module docs' "A relative
/// result stays relative").
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
    // `info script` reports the path as Tcl sees it, which is slash form on
    // every host, so the native path is converted before it enters the fold.
    // A drive-relative script path would make every `file dirname` of it
    // meaningless, so it is dropped rather than folded.
    let info_script = info_script
        .map(to_tcl_slash_form)
        .filter(|p| !is_drive_relative(p));
    let result = eval(&node, info_script.as_deref())?;
    // Expand `~` and normalise (collapsing `..`) so the parent-dir idiom
    // resolves to a real directory.  A rootless result is left for the
    // caller to anchor.
    fold_path_value(&result)
}

/// A native filesystem path as Tcl's slash form.
///
/// Backslashes are separators **only** in a Windows-shaped path — one opening
/// with a `X:` drive prefix or a `\\server` UNC prefix.  On a POSIX host a
/// backslash is an ordinary filename character, so an unshaped path is
/// returned untouched and `/tmp/a\b` keeps its literal `\`.
#[must_use]
pub fn to_tcl_slash_form(p: &str) -> String {
    if is_windows_shaped(p) {
        p.replace('\\', "/")
    } else {
        p.to_owned()
    }
}

/// Does `p` open with a Windows volume prefix — `X:` or `\\` / `//`-plus-host?
fn is_windows_shaped(p: &str) -> bool {
    if p.starts_with("\\\\") {
        return true;
    }
    let mut chars = p.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(c), Some(':')) if c.is_ascii_alphabetic()
    )
}

/// Is `p` (slash form) drive-relative — a `X:` prefix *not* followed by `/`?
///
/// `C:foo` resolves against drive C's own working directory, which a static
/// analysis cannot know, so callers abstain on it.
fn is_drive_relative(p: &str) -> bool {
    is_windows_shaped(p) && !p.starts_with("//") && !p.starts_with("\\\\") && {
        // Windows-shaped and not UNC ⇒ a `X:` drive prefix.
        !p[2..].starts_with('/')
    }
}

/// Byte length of `p`'s absolute root, or `None` when `p` is relative.
///
/// The three roots, all in slash form:
///
/// | Shape | Root | Example |
/// |-------|------|---------|
/// | POSIX / Windows volume-relative | `/` | `/a/b` → 1 |
/// | Windows drive | `X:/` | `C:/a/b` → 3 |
/// | UNC share | `//server/share` | `//srv/sh/a` → 7 |
///
/// A UNC root spans *both* the host and the share, because `//server` alone is
/// not a directory: `file dirname //server/share` is `//server/share` itself.
/// Path arithmetic never rewrites or ascends past the root.
fn root_len(p: &str) -> Option<usize> {
    if is_windows_shaped(p) && !p.starts_with("//") && p[2..].starts_with('/') {
        return Some(3); // `C:/`
    }
    if p.starts_with("//") && !p.starts_with("///") {
        // `//server/share` — take two components; short of that, the whole
        // string is (an incomplete) root.
        let rest = &p[2..];
        let host = rest.find('/').unwrap_or(rest.len());
        let after_host = &rest[host..];
        if after_host.is_empty() {
            return Some(p.len());
        }
        let share = after_host[1..].find('/').unwrap_or(after_host.len() - 1);
        return Some(2 + host + 1 + share);
    }
    p.starts_with('/').then_some(1)
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

/// Evaluate a parsed node to a path string (pre-normalisation).
fn eval(node: &Node, info_script: Option<&str>) -> Option<String> {
    match node {
        Node::Lit(value) => {
            // Reject any word still carrying a variable substitution —
            // we refuse to guess.
            if value.contains('$') {
                None
            } else {
                // A literal may be written natively (`C:\repo\lib`); the fold
                // works in slash form, so convert here too.
                Some(to_tcl_slash_form(value))
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
                    return Some(path_dirname(&eval(&args[1], info_script)?));
                }
                if sub == "normalize" && args.len() == 2 {
                    return eval_file_normalize(&eval(&args[1], info_script)?);
                }
                if sub == "join" && args.len() >= 2 {
                    let mut parts: Vec<String> = Vec::new();
                    for a in &args[1..] {
                        parts.push(eval(a, info_script)?);
                    }
                    if parts.is_empty() {
                        return None;
                    }
                    return Some(path_join(&parts));
                }
            }
            None
        }
    }
}

/// `file normalize` in slash form, or `None` when the argument is not already
/// anchored.
///
/// `file normalize` always *returns* an absolute path: given a relative one it
/// resolves against the interpreter's run-time working directory, which is
/// exactly the fact this module refuses to guess at (module docs, "A relative
/// result stays relative").  So only an anchored argument folds — by a literal
/// root, or by `~`, which [`expanduser`] resolves here for the same reason
/// [`fold_path_value`] does at the top level.
///
/// For an anchored path the command's remaining work is collapsing `.` and
/// `..`, which [`normpath`] already does.  Symbolic links are *not* resolved:
/// tclsh follows them, and this cannot without touching the filesystem — the
/// gap is deliberate, and it only ever leaves a path a reader would recognise
/// rather than one pointing somewhere else.
///
/// This is what makes the corpus idiom fold — `set dir [file normalize [file
/// dirname [info script]]]`, and the equivalent `[file dirname [file normalize
/// [info script]]]`, both of which are anchored by `[info script]`.  Without
/// it every `source [file join $dir …]` in a file written that way abstained
/// (issue #775).
/// A drive-relative `C:foo` is rejected by the same test, since [`root_len`]
/// only counts `X:/` as a root — it names a per-drive working directory, which
/// is no more knowable than the process-wide one.
fn eval_file_normalize(path: &str) -> Option<String> {
    let expanded = expanduser(&to_tcl_slash_form(path));
    root_len(&expanded)?;
    Some(normpath(&expanded))
}

/// `file dirname` in slash form — everything up to (not including) the last
/// `/`, collapsing trailing slashes on the head, but never shortening the path
/// past its root ([`root_len`]).
///
/// The root clamp is what makes the Windows shapes come out right:
/// `C:/x` → `C:/` (not the drive-relative `C:`), and
/// `//server/share/dir` → `//server/share` (not `//server`).
///
/// A rootless single-component path has `.` for its directory, exactly as
/// tclsh 8.6.16 / 9.0.4 answer `file dirname helper.tcl` — not the empty
/// string, and emphatically not the analysing process's working directory.
fn path_dirname(p: &str) -> String {
    let root = root_len(p).unwrap_or(0);
    let head_or_dot = |head: &str| {
        if head.is_empty() {
            ".".to_owned()
        } else {
            head.to_owned()
        }
    };
    if p.len() <= root {
        return head_or_dot(&p[..root]);
    }
    let Some(idx) = p[root..].rfind('/') else {
        // No separator past the root: the head is the root, or `.` for a
        // single-component relative path.
        return head_or_dot(&p[..root]);
    };
    let head = &p[..root + idx];
    if head.len() <= root {
        return head_or_dot(&p[..root]);
    }
    head_or_dot(head.trim_end_matches('/'))
}

/// `file join` in slash form — a later *absolute* component resets the
/// accumulator; otherwise components are joined with a single `/`.
///
/// One Windows subtlety from `filename(n)`: a `/foo` component is
/// *volumerelative*, not absolute, so joined onto a drive- or UNC-rooted
/// accumulator it replaces the tail while keeping the volume —
/// `file join C:/a /b` is `C:/b`, not `/b`.
fn path_join(parts: &[String]) -> String {
    let mut result = parts[0].clone();
    for p in &parts[1..] {
        match (root_len(p), root_len(&result)) {
            // A volume-relative `/foo` onto a volume-rooted accumulator keeps
            // the volume and replaces everything after it.
            (Some(1), Some(vol)) if vol > 1 && !p.starts_with("//") => {
                result.truncate(vol);
                if !result.ends_with('/') {
                    result.push('/');
                }
                result.push_str(p.trim_start_matches('/'));
            }
            (Some(_), _) => result.clone_from(p),
            (None, _) => {
                if result.is_empty() || result.ends_with('/') {
                    result.push_str(p);
                } else {
                    result.push('/');
                    result.push_str(p);
                }
            }
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

/// Normalise `p` (slash form) — collapse `.`, `..` and repeated separators
/// — **without** anchoring a rootless path anywhere.
///
/// An unanchored relative result is Tcl's own "resolve against the
/// interpreter's run-time working directory" (module docs), which is not
/// statically knowable; absolutising it against the *analysing process's*
/// working directory would substitute an unrelated editor artefact for it.
/// The caller — which knows the sourcing document, the workspace root, or
/// both — anchors it instead.
fn normalise_path_value(p: &str) -> String {
    normpath(p)
}

/// Path normalisation in slash form — collapse `.`, `..` and repeated
/// separators, keeping the root ([`root_len`]) verbatim so `..` can never
/// ascend past `/`, a drive (`C:/`) or a UNC share (`//server/share`).
fn normpath(p: &str) -> String {
    if p.is_empty() {
        return ".".to_owned();
    }
    let root = root_len(p);
    let (prefix, tail) = match root {
        Some(n) => (&p[..n], &p[n..]),
        None => ("", p),
    };
    let mut out: Vec<&str> = Vec::new();
    for comp in tail.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp != ".." || (root.is_none() && out.is_empty()) || out.last() == Some(&"..") {
            out.push(comp);
        } else if !out.is_empty() {
            out.pop();
        }
    }
    // The root in canonical form: `/`, `C:/`, or `//server/share` (which
    // already ends in a component, so it takes a separator before the tail).
    let mut result = prefix.trim_end_matches('/').to_owned();
    if root.is_some() && (result.is_empty() || result.ends_with(':')) {
        result.push('/');
    }
    let tail = out.join("/");
    if !tail.is_empty() {
        if !result.is_empty() && !result.ends_with('/') {
            result.push('/');
        }
        result.push_str(&tail);
    }
    if result.is_empty() {
        ".".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Absolute `info_script` keeps the fold rooted; a rootless fold result
    // is returned relative (see the module docs), never anchored on the
    // analysing process's working directory.

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

    /// TP (issue #923 idx 73) — the exact idiom `pix` writes: three nested
    /// `file dirname`s around `[info script]`, one level deeper than the
    /// two-level form the other tests use.  `examples/user.tcl` reaching the
    /// repo root two directories up is what makes the package resolvable
    /// when the workspace root is scoped below it.
    #[test]
    fn triple_nested_dirname_is_the_pix_idiom() {
        assert_eq!(
            evaluate_auto_path_expr(
                "[file dirname [file dirname [file dirname [info script]]]]",
                Some("/proj/lib/examples/user.tcl"),
            )
            .as_deref(),
            Some("/proj"),
        );
    }

    /// The analysing process's working directory must never influence a
    /// fold.  A rootless expression stays rootless (its caller anchors it);
    /// running the same fold from two different directories must not change
    /// the answer.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): `set v helper.tcl; source $v` inside
    /// `sub/main.tcl` run as `cd other && tclsh ../sub/main.tcl` loads
    /// `other/helper.tcl` — resolution is against the interpreter's *run
    /// time* working directory, which a language server cannot know and must
    /// not substitute its own launch directory for.
    #[test]
    fn a_rootless_fold_stays_relative_and_ignores_the_process_cwd() {
        assert_eq!(
            evaluate_auto_path_expr("helper.tcl", None).as_deref(),
            Some("helper.tcl"),
        );
        assert_eq!(
            evaluate_auto_path_expr("[file join lib pkg]", None).as_deref(),
            Some("lib/pkg"),
        );
        // `..` still collapses lexically without a root to ascend past.
        // (tclsh returns the uncollapsed text `lib/../other`; this evaluator
        // normalises deliberately — see `normalise_path_value` — because its
        // consumers compare and scan directories rather than print them, and
        // the two name the same directory.)
        assert_eq!(
            evaluate_auto_path_expr("[file join lib .. other]", None).as_deref(),
            Some("other"),
        );
        // `file dirname` of a single-component relative path is `.` in tclsh
        // 8.6/9.0 — not the process's working directory.
        assert_eq!(
            evaluate_auto_path_expr("[file dirname helper.tcl]", None).as_deref(),
            Some("."),
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

    // `file normalize` — the wrapper the corpus writes around the
    // `[info script]` idiom, and the reason every `source [file join $dir …]`
    // in a file spelled that way used to abstain (issue #775).

    /// Both nestings of the corpus idiom fold, and to the same directory.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): with `[info script]` reporting
    /// `/proj/test/arbitaryTest.tcl`, both spellings answer `/proj/test`.
    #[test]
    fn file_normalize_folds_the_info_script_idiom() {
        for expr in [
            "[file normalize [file dirname [info script]]]",
            "[file dirname [file normalize [info script]]]",
        ] {
            assert_eq!(
                evaluate_auto_path_expr(expr, Some("/proj/test/arbitaryTest.tcl")).as_deref(),
                Some("/proj/test"),
                "{expr} must fold",
            );
        }
    }

    /// `file normalize` collapses `.` and `..` like the rest of the module.
    #[test]
    fn file_normalize_collapses_dot_segments() {
        assert_eq!(
            evaluate_auto_path_expr("[file normalize /proj/test/../lib/.]", None).as_deref(),
            Some("/proj/lib"),
        );
    }

    /// A relative argument does not fold: `file normalize` would resolve it
    /// against the interpreter's working directory, which is not knowable
    /// statically — and inventing the analysing process's own cwd is the one
    /// answer that is always wrong.
    #[test]
    fn file_normalize_of_a_relative_path_is_none() {
        for expr in [
            "[file normalize lib]",
            "[file normalize ../lib]",
            "[file normalize [file dirname helper.tcl]]",
            "[file normalize C:lib]",
        ] {
            assert_eq!(
                evaluate_auto_path_expr(expr, Some("/proj/test/caller.tcl")),
                None,
                "{expr} must abstain",
            );
        }
    }

    /// Anchored by `~` rather than a literal root, expanded the same way the
    /// top-level fold expands it.
    #[test]
    fn file_normalize_expands_tilde() {
        temp_env::with_var("HOME", Some("/home/tester"), || {
            assert_eq!(
                evaluate_auto_path_expr("[file normalize ~/lib/..]", None).as_deref(),
                Some("/home/tester"),
            );
        });
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

    // -----------------------------------------------------------------------
    // Slash-form path arithmetic (PR #1086 finding 1).
    //
    // Pure string math, so the Windows shapes are exercised on any host — the
    // evaluator has no `cfg(windows)` branch to skip.
    // -----------------------------------------------------------------------

    #[test]
    fn dirname_join_normalise_are_root_aware() {
        // POSIX, unchanged.
        assert_eq!(path_dirname("/a/b/c"), "/a/b");
        assert_eq!(path_dirname("/a"), "/");
        assert_eq!(path_dirname("/"), "/");
        // A rootless single component: `.`, exactly as tclsh 8.6.16 / 9.0.4
        // answer `file dirname a` — the empty string this used to return had
        // to be rescued by absolutising against the process's own working
        // directory, which is not something a language server may assume.
        assert_eq!(path_dirname("a"), ".");
        assert_eq!(path_join(&["/a/b".into(), "..".into()]), "/a/b/..");
        assert_eq!(path_join(&["/a".into(), "/b".into()]), "/b");
        assert_eq!(normpath("/a/b/../c"), "/a/c");
        assert_eq!(normpath("/a/./b"), "/a/b");
        // `..` cannot climb out of the POSIX root.
        assert_eq!(normpath("/a/../.."), "/");

        // Windows drive.  `file dirname C:/x` is `C:/`, never the
        // drive-*relative* `C:`.
        assert_eq!(path_dirname("C:/x/y"), "C:/x");
        assert_eq!(path_dirname("C:/x"), "C:/");
        assert_eq!(path_dirname("C:/"), "C:/");
        assert_eq!(path_join(&["C:/a".into(), "b".into()]), "C:/a/b");
        assert_eq!(path_join(&["/a".into(), "C:/b".into()]), "C:/b");
        assert_eq!(normpath("C:/a/../b"), "C:/b");
        assert_eq!(normpath("C:/a/../.."), "C:/");

        // UNC: the root spans host *and* share, and `..` stops there.
        assert_eq!(path_dirname("//server/share/dir"), "//server/share");
        assert_eq!(path_dirname("//server/share"), "//server/share");
        assert_eq!(
            path_join(&["//server/share".into(), "d".into()]),
            "//server/share/d"
        );
        assert_eq!(normpath("//server/share/a/../b"), "//server/share/b");
        assert_eq!(normpath("//server/share/a/../.."), "//server/share");
    }

    /// `filename(n)`: on Windows a leading `/` is *volumerelative*, not
    /// absolute, so joining it onto a volume-rooted path keeps the volume.
    #[test]
    fn join_treats_a_leading_slash_as_volume_relative() {
        assert_eq!(path_join(&["C:/a/b".into(), "/x".into()]), "C:/x");
        assert_eq!(
            path_join(&["//server/share/a".into(), "/x".into()]),
            "//server/share/x",
        );
        // Between two POSIX paths the ordinary "absolute component wins" rule
        // still applies.
        assert_eq!(path_join(&["/a/b".into(), "/x".into()]), "/x");
    }

    #[test]
    fn native_windows_separators_become_slash_form() {
        assert_eq!(to_tcl_slash_form("C:\\repo\\user.tcl"), "C:/repo/user.tcl");
        assert_eq!(
            to_tcl_slash_form("\\\\server\\share\\lib"),
            "//server/share/lib",
        );
        // TN: on POSIX a backslash is an ordinary filename character and must
        // survive untouched.
        assert_eq!(to_tcl_slash_form("/tmp/a\\b"), "/tmp/a\\b");
        assert_eq!(to_tcl_slash_form("/proj/lib"), "/proj/lib");
    }

    /// The P1 itself: `Uri::to_file_path` hands back a native Windows path, and
    /// the `[file dirname [info script]]` idiom must resolve against *it*, not
    /// silently fall through to the process working directory.
    #[test]
    fn info_script_folds_a_native_windows_path() {
        assert_eq!(
            evaluate_auto_path_expr(
                "[file dirname [info script]]",
                Some("C:\\repo\\pkg\\init.tcl"),
            )
            .as_deref(),
            Some("C:/repo/pkg"),
        );
        assert_eq!(
            evaluate_auto_path_expr(
                "[file dirname [file dirname [info script]]]",
                Some("C:\\repo\\pkg\\init.tcl"),
            )
            .as_deref(),
            Some("C:/repo"),
        );
        assert_eq!(
            evaluate_auto_path_expr(
                "[file join [file dirname [info script]] ..]",
                Some("C:/repo/pkg/init.tcl"),
            )
            .as_deref(),
            Some("C:/repo"),
        );
        assert_eq!(
            evaluate_auto_path_expr(
                "[file dirname [info script]]",
                Some("\\\\server\\share\\pkg\\init.tcl"),
            )
            .as_deref(),
            Some("//server/share/pkg"),
        );
        // A literal written with native separators folds too.
        assert_eq!(
            evaluate_auto_path_expr("C:\\lib\\tcl", None).as_deref(),
            Some("C:/lib/tcl"),
        );
    }

    /// TN: a drive-*relative* path names a per-drive working directory a static
    /// analysis cannot know, so it abstains rather than resolving against the
    /// process cwd.
    #[test]
    fn drive_relative_paths_abstain() {
        assert_eq!(evaluate_auto_path_expr("C:lib", None), None);
        assert_eq!(evaluate_auto_path_expr("C:", None), None);
        assert_eq!(
            evaluate_auto_path_expr("[file dirname [info script]]", Some("C:user.tcl")),
            None,
        );
    }

    // -----------------------------------------------------------------------
    // `set` vs `lappend` arity (PR #1086 finding 2).
    // -----------------------------------------------------------------------

    /// Fold every entry a source records, in order.
    fn fold_all(src: &str, info_script: &str) -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        r.auto_path_entries
            .iter()
            .flat_map(|e| evaluate_auto_path_entry(e, Some(info_script)))
            .collect()
    }

    /// TP — `set auto_path {A B}` assigns a **list**, so both directories are
    /// searched.  Oracle (`tclsh8.6` / `tclsh9.0`): `set auto_path {/opt/p1
    /// /opt/p2}; llength $auto_path` → 2, and `package require` finds a package
    /// under either.
    #[test]
    fn set_auto_path_yields_one_dir_per_list_element() {
        assert_eq!(
            fold_all("set auto_path {/opt/p1 /opt/p2}\n", "/w/user.tcl"),
            vec!["/opt/p1".to_owned(), "/opt/p2".to_owned()],
        );
        assert_eq!(
            fold_all("set auto_path \"/opt/x /opt/y\"\n", "/w/user.tcl"),
            vec!["/opt/x".to_owned(), "/opt/y".to_owned()],
        );
    }

    /// TN — a brace-quoted element containing a space is **one** directory
    /// (`llength {/opt/a {/opt/with space} /opt/b}` → 3, oracle-confirmed), and
    /// so is every `lappend` word however many spaces it holds.
    #[test]
    fn braced_elements_and_lappend_words_stay_one_directory_each() {
        assert_eq!(
            fold_all(
                "set auto_path {/opt/a {/opt/with space} /opt/b}\n",
                "/w/user.tcl",
            ),
            vec![
                "/opt/a".to_owned(),
                "/opt/with space".to_owned(),
                "/opt/b".to_owned(),
            ],
        );
        // `lappend auto_path {p q}` appends ONE element — never list-split.
        assert_eq!(
            fold_all("lappend auto_path {/opt/p q}\n", "/w/user.tcl"),
            vec!["/opt/p q".to_owned()],
        );
        // …while `lappend auto_path a b` is already two words, hence two dirs.
        assert_eq!(
            fold_all("lappend auto_path /opt/a /opt/b\n", "/w/user.tcl"),
            vec!["/opt/a".to_owned(), "/opt/b".to_owned()],
        );
    }

    /// A `set` right-hand side carrying a substitution is *not* a literal list:
    /// `set auto_path [file dirname [info script]]` is one computed directory,
    /// and `[list …]` is outside the subset entirely.
    #[test]
    fn a_computed_set_right_hand_side_is_not_list_split() {
        assert_eq!(
            fold_all(
                "set auto_path [file dirname [info script]]\n",
                "/proj/lib/user.tcl",
            ),
            vec!["/proj/lib".to_owned()],
        );
        assert!(
            fold_all("set auto_path [list /a /b]\n", "/w/user.tcl").is_empty(),
            "`[list …]` is outside the supported subset — abstain, never \
             split its text as if it were a list literal",
        );
        assert!(fold_all("set auto_path $env(TCLLIBPATH)\n", "/w/user.tcl").is_empty());
    }
}
