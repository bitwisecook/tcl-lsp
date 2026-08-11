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
//! [`eval_file_normalize`]), literal words, and `~`-prefixed paths.
//! Variable references (`$dir`, `${dir}`, `pre_$dir`) are resolved through
//! whatever resolver the **caller** supplies — this module owns what a path
//! expression *means*, the caller owns what a variable *is worth*, and the
//! two compose through [`evaluate_auto_path_expr_with_resolver`].  A
//! variable the resolver cannot answer (including every variable, for the
//! resolver-less [`evaluate_auto_path_expr`]), an array element, or a
//! list-manipulation command returns `None` and the caller skips the
//! entry.  We never guess.
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
use crate::segmenter::segment_commands_with_offset_and_config;
use std::collections::HashMap;

/// A parsed word: a literal, a substitutable word, or a command
/// substitution `[name arg …]`.
enum Node {
    /// A word taken verbatim — a braced group (Tcl performs no substitution
    /// inside braces), or any word with no `$` in it.
    Lit(String),
    /// A bare or double-quoted word containing `$` — Tcl *would* substitute
    /// here, so [`eval`] folds it through the caller's variable resolver
    /// ([`crate::text::fold_interpolation_single`]), never textually: a
    /// resolved value containing spaces or brackets is still exactly one
    /// word, where splicing it into the raw text and re-parsing would split
    /// it (`$d` = `my dir` inside `[file join $d x]` is `my dir/x`, not
    /// `my/dir/x`).
    Subst(String),
    Cmd(String, Vec<Node>),
}

/// One lexical token: a structural bracket, or a word that remembers whether
/// its spelling suppresses substitution (braced) or permits it (bare/quoted).
enum Tok {
    Open,
    Close,
    Word { text: String, verbatim: bool },
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
    evaluate_auto_path_entry_with_constants(entry, info_script, &HashMap::new())
}

/// [`evaluate_auto_path_entry`], with the document's single-assignment
/// constants ([`constant_path_assignments`], folded) answering variable
/// references — which is what carries the corpus idiom
/// `set libDir [file join $dir lib]; lappend auto_path $libDir` (issue #775;
/// georgtree/SpiceGenTcl's own `SpiceGenTcl.tcl` registers its `lib`
/// directory exactly this way, and contributed nothing without it).
#[must_use]
pub fn evaluate_auto_path_entry_with_constants<S: std::hash::BuildHasher>(
    entry: &AutoPathEntry,
    info_script: Option<&str>,
    constants: &HashMap<String, String, S>,
) -> Vec<String> {
    let raw = entry.raw_path.as_str();
    if raw.contains('$') || raw.contains('[') {
        // Computed: fold it as an expression, as one directory.
        return evaluate_auto_path_expr_with_constants(raw, info_script, constants)
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
    evaluate_auto_path_expr_with_constants(raw, info_script, &HashMap::new())
}

/// Whether `text` still carries an unevaluated substitution — a `$` variable
/// reference or a `[` command substitution.
///
/// The segmenter reconstructs a word with its `${var}` / `[cmd]` markers
/// intact, so their absence is reliable evidence the word is a plain literal
/// (the same test `SignatureSource::is_literal` uses).  A word that still
/// carries one is *not* a path, however plausible it looks spelled out.
#[must_use]
pub fn carries_substitution(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// [`evaluate_auto_path_expr_with_resolver`] over a plain constant map — the
/// convenience form for callers whose variable facts are already a
/// `name → value` table, such as [`constant_path_vars`]'s.
///
/// Names absent from `constants` stay unresolved, so the fold abstains on
/// them exactly as it does on any other unevaluated substitution — passing an
/// empty map is precisely [`evaluate_auto_path_expr`].
#[must_use]
pub fn evaluate_auto_path_expr_with_constants<S: std::hash::BuildHasher>(
    raw: &str,
    info_script: Option<&str>,
    constants: &HashMap<String, String, S>,
) -> Option<String> {
    evaluate_auto_path_expr_with_resolver(raw, info_script, &|name| constants.get(name).cloned())
}

/// The full evaluator: fold `raw` with variable references answered by
/// `resolve_var` — the provenance/semantics split that makes this a shared
/// lower layer rather than one consumer's helper.
///
/// This module owns what a path expression **means** (`[file join …]`,
/// `[file dirname …]`, `[file normalize …]`, `[info script]`, `~`); the
/// caller owns what a variable **is worth** at the point being analysed, and
/// says so through `resolve_var`.  A simple consumer passes a single-
/// assignment constant map ([`evaluate_auto_path_expr_with_constants`]); a
/// consumer with real flow information (the analyser's constant-string
/// lattice, an SCCP-backed value table) passes its own resolver and gets the
/// same path semantics with better provenance — the evaluator does not care
/// where the values came from, only that the caller vouches for them.
///
/// Resolution happens on the parsed expression, **never** by splicing values
/// into the raw text and re-lexing: a value containing spaces is still one
/// word (`$d` = `my dir` in `[file join $d x]` folds to `my dir/x`, where
/// textual substitution would re-parse it as two words and answer
/// `my/dir/x`), and a value containing `[` or `$` is data, not syntax to be
/// re-interpreted.
///
/// An unresolved variable — `resolve_var` answering `None` — abstains the
/// whole fold, the module-wide rule: we never guess.
#[must_use]
pub fn evaluate_auto_path_expr_with_resolver(
    raw: &str,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
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
    let result = eval(&node, info_script.as_deref(), resolve_var)?;
    // Expand `~` and normalise (collapsing `..`) so the parent-dir idiom
    // resolves to a real directory.  A rootless result is left for the
    // caller to anchor.
    fold_path_value(&result)
}

/// Path-valued variables `source` assigns **exactly once** at its top level,
/// mapped to their folded values — the map
/// [`evaluate_auto_path_expr_with_constants`] consumes.
///
/// Assignments fold in document order, each one seeing the constants
/// established before it, so a chain resolves:
///
/// ```tcl
/// set dir       [file dirname [file normalize [info script]]]
/// set sourceDir [file join $dir src]
/// source [file join $sourceDir generalClasses.tcl]
/// ```
///
/// Without the chain only `dir` folds, and every `source` built on
/// `$sourceDir` abstains — the shape that leaves a whole package's worth of
/// `source` lines unresolved (issue #775; georgtree/SpiceGenTcl's own
/// `SpiceGenTcl.tcl` is seventeen such lines).
///
/// Deliberately narrow in two ways it must stay narrow:
///
/// * A name written by more than one top-level `set` is dropped **outright**,
///   counted in a pre-pass rather than as the walk goes.  Last-write-wins
///   would be wrong for a `source` sitting between the two writes, and
///   first-write-wins wrong for one after both — so a re-assigned name
///   contributes nothing, and contributes nothing to anything computed from
///   it either.
/// * Only values this evaluator can fold are kept, so nothing here can invent
///   a target that the single-expression path would have refused.
///
/// Only top-level `set`s are seen at all: a body is one braced word to the
/// segmenter, so an assignment inside a `proc` or an `if` never enters the
/// map and never has to be reasoned about.
#[must_use]
pub fn constant_path_vars(
    source: &str,
    dialect: &str,
    info_script: Option<&str>,
) -> HashMap<String, String> {
    fold_constant_assignments(&constant_path_assignments(source, dialect), info_script)
}

/// The **raw** assignment facts of [`constant_path_vars`]: every top-level
/// `set name value` with a plain scalar name, in document order, values
/// unfolded.  Multi-write poisoning happens at fold time
/// ([`fold_constant_assignments`]), not here, so the raw list can be
/// accumulated chunk by chunk without any chunk needing the whole document's
/// write counts.
///
/// Split out from the fold so the fact can be *recorded* where only the text
/// is known and *folded* where the document's own filesystem path is (the
/// analyser stores this on `AnalysisResult`, the workspace index carries it,
/// and the source-graph edge resolver folds it lazily per parent).  The raw
/// form is deliberately path-independent: the same text yields the same
/// assignments wherever the file lives, which is what makes it safe to cache
/// alongside the rest of the analysis.
#[must_use]
pub fn constant_path_assignments(source: &str, dialect: &str) -> Vec<(String, String)> {
    constant_path_assignments_from_commands(&segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    ))
}

/// [`constant_path_assignments`] over already-segmented commands — the form
/// the analyser's walk uses, which has the segmentation in hand and must not
/// pay for a second one.
#[must_use]
pub fn constant_path_assignments_from_commands(
    commands: &[crate::segmenter::SegmentedCommand],
) -> Vec<(String, String)> {
    commands
        .iter()
        .filter(|seg| {
            seg.texts.len() == 3
                && seg.texts[0] == "set"
                && !seg.texts[1].contains('$')
                && !seg.texts[1].contains('[')
                && !seg.texts[1].contains("::")
        })
        .map(|seg| (seg.texts[1].clone(), seg.texts[2].clone()))
        .collect()
}

/// Fold [`constant_path_assignments`]' raw facts into the `name → value` map
/// [`evaluate_auto_path_expr_with_constants`] consumes.
///
/// Chaining: each assignment folds with the constants established before it,
/// in document order, so a value naming a *later* constant does not fold.
///
/// Poisoning: a name written by more than one recorded assignment is dropped
/// **outright**, counted in a pre-pass here rather than as the walk goes.
/// Last-write-wins would be wrong for a `source` sitting between the two
/// writes, and first-write-wins wrong for one after both — so a re-assigned
/// name contributes nothing, and contributes nothing to anything computed
/// from it either.
#[must_use]
pub fn fold_constant_assignments(
    assignments: &[(String, String)],
    info_script: Option<&str>,
) -> HashMap<String, String> {
    let mut writes: HashMap<&str, usize> = HashMap::new();
    for (name, _) in assignments {
        *writes.entry(name.as_str()).or_default() += 1;
    }
    let mut constants: HashMap<String, String> = HashMap::new();
    for (name, raw) in assignments {
        if writes.get(name.as_str()) != Some(&1) {
            continue;
        }
        // A literal keeps its own text: a path with spaces (`set d "my dir"`)
        // is one path value, not the several words the expression parser
        // would split it into.
        let value = if carries_substitution(raw) {
            evaluate_auto_path_expr_with_constants(raw, info_script, &constants)
        } else {
            Some(raw.clone())
        };
        if let Some(value) = value {
            constants.insert(name.clone(), value);
        }
    }
    constants
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

/// Split `text` into bracket / word tokens.  Brace groups are kept as a
/// single **verbatim** word (Tcl performs no substitution inside braces, and
/// brackets inside braces are literal); bare and double-quoted words are
/// substitutable.  An unterminated brace / quote aborts with `None`.
fn tokenise(text: &str) -> Option<Vec<Tok>> {
    let b: Vec<char> = text.chars().collect();
    let n = b.len();
    let mut tokens: Vec<Tok> = Vec::new();
    let mut i = 0;
    while i < n {
        let ch = b[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '[' || ch == ']' {
            tokens.push(if ch == '[' { Tok::Open } else { Tok::Close });
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
            tokens.push(Tok::Word {
                text: b[i + 1..j - 1].iter().collect(),
                verbatim: true,
            });
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
            tokens.push(Tok::Word {
                text: b[i + 1..j].iter().collect(),
                verbatim: false,
            });
            i = j + 1;
            continue;
        }
        let mut j = i;
        while j < n && !b[j].is_whitespace() && b[j] != '[' && b[j] != ']' {
            j += 1;
        }
        tokens.push(Tok::Word {
            text: b[i..j].iter().collect(),
            verbatim: false,
        });
        i = j;
    }
    Some(tokens)
}

/// Parse a single word starting at `pos`; returns `(node, next_pos)`.
///
/// A substitutable word containing `$` parses as [`Node::Subst`]; everything
/// else word-shaped is [`Node::Lit`].  A braced word is always `Lit`, even
/// when it spells `$` or `[` — braces suppress substitution in Tcl, so
/// `{$d}` names a file literally called `$d` and `{[}` is a literal `[`
/// (this used to mis-read as an open bracket when tokens were bare strings).
fn parse(tokens: &[Tok], mut pos: usize) -> (Option<Node>, usize) {
    let Some(tok) = tokens.get(pos) else {
        return (None, pos);
    };
    match tok {
        Tok::Open => {
            pos += 1;
            let Some(Tok::Word { text: name, .. }) = tokens.get(pos) else {
                return (None, pos);
            };
            let name = name.clone();
            pos += 1;
            let mut args: Vec<Node> = Vec::new();
            while pos < tokens.len() && !matches!(tokens[pos], Tok::Close) {
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
            (Some(Node::Cmd(name, args)), pos)
        }
        Tok::Close => (None, pos),
        Tok::Word { text, verbatim } => {
            let node = if !verbatim && text.contains('$') {
                Node::Subst(text.clone())
            } else {
                Node::Lit(text.clone())
            };
            (Some(node), pos + 1)
        }
    }
}

/// Evaluate a parsed node to a path string (pre-normalisation), answering
/// variable references through `resolve_var`.
fn eval(
    node: &Node,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    match node {
        Node::Lit(value) => {
            // A verbatim word carrying `$` would name a file with a literal
            // dollar in it — legal, vanishingly rare, and impossible to
            // distinguish from an authoring mistake, so abstain.
            if value.contains('$') {
                None
            } else {
                // A literal may be written natively (`C:\repo\lib`); the fold
                // works in slash form, so convert here too.
                Some(to_tcl_slash_form(value))
            }
        }
        // Resolution happens on the parsed word, via the interpolation
        // folder's own segment grammar (which also rejects array-indexed
        // names and stray `[`) — never by splicing text and re-lexing.
        Node::Subst(word) => crate::text::fold_interpolation_single(word, |name| resolve_var(name))
            .map(|v| to_tcl_slash_form(&v)),
        Node::Cmd(name, args) => {
            if name == "info" && args.len() == 1 {
                return match eval(&args[0], info_script, resolve_var).as_deref() {
                    Some("script") => info_script.map(str::to_owned),
                    _ => None,
                };
            }
            if name == "file" && args.len() >= 2 {
                let sub = eval(&args[0], info_script, resolve_var)?;
                if sub == "dirname" && args.len() == 2 {
                    return Some(path_dirname(&eval(&args[1], info_script, resolve_var)?));
                }
                if sub == "normalize" && args.len() == 2 {
                    return eval_file_normalize(&eval(&args[1], info_script, resolve_var)?);
                }
                if sub == "join" && args.len() >= 2 {
                    let mut parts: Vec<String> = Vec::new();
                    for a in &args[1..] {
                        parts.push(eval(a, info_script, resolve_var)?);
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

    // Chained single-assignment constants (issue #775).

    /// A directory reached through an intermediate folds, so the whole chain
    /// resolves — georgtree/SpiceGenTcl's own `SpiceGenTcl.tcl` shape.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/SpiceGenTcl.tcl`, `dir`
    /// is `/proj` and `sourceDir` is `/proj/src`.
    #[test]
    fn constants_fold_through_a_chain() {
        let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set libDir [file join $dir lib]\n\
                   set sourceDir [file join $dir src]\n";
        let constants = constant_path_vars(src, "tcl", Some("/proj/SpiceGenTcl.tcl"));
        assert_eq!(constants.get("dir").map(String::as_str), Some("/proj"));
        assert_eq!(
            constants.get("libDir").map(String::as_str),
            Some("/proj/lib")
        );
        assert_eq!(
            constants.get("sourceDir").map(String::as_str),
            Some("/proj/src"),
        );
        assert_eq!(
            evaluate_auto_path_expr_with_constants(
                "[file join $sourceDir ngspice x.tcl]",
                Some("/proj/SpiceGenTcl.tcl"),
                &constants,
            )
            .as_deref(),
            Some("/proj/src/ngspice/x.tcl"),
        );
    }

    /// Order matters: a value naming a constant established *later* does not
    /// fold, because at that point in the file the name has no value.
    #[test]
    fn a_constant_used_before_it_is_assigned_does_not_fold() {
        let src = "set early [file join $late x]\n\
                   set late /opt\n";
        let constants = constant_path_vars(src, "tcl", None);
        assert_eq!(constants.get("late").map(String::as_str), Some("/opt"));
        assert!(!constants.contains_key("early"), "{constants:?}");
    }

    /// A re-assigned name contributes nothing — and nothing computed from it
    /// contributes either, since the poison is counted in a pre-pass rather
    /// than as the walk goes.  Which value a later `source` sees is a flow
    /// question this map does not answer.
    #[test]
    fn a_reassigned_name_poisons_itself_and_its_dependents() {
        let src = "set dir /opt/a\n\
                   set sub [file join $dir sub]\n\
                   set dir /opt/b\n";
        let constants = constant_path_vars(src, "tcl", None);
        assert!(!constants.contains_key("dir"), "{constants:?}");
        assert!(
            !constants.contains_key("sub"),
            "a value computed from a poisoned name must not survive: {constants:?}",
        );
    }

    /// A resolved value containing spaces is **one** path element, exactly as
    /// `file join {my dir} x` treats its arguments (filename(n): join takes
    /// arguments, not words — a space is just a character).  Oracle, tclsh
    /// 8.6.14: `set d "my dir"; file join $d x` → `my dir/x`, and a `source`
    /// through such a join really loads from the spaced directory.  This is
    /// the case that makes node-based resolution obligatory: splicing
    /// `my dir` into the raw text and re-lexing splits it into two words and
    /// answers `my/dir/x`.
    #[test]
    fn a_constant_with_spaces_folds_as_one_join_element() {
        let src = "set d {my dir}\nset sub [file join $d x]\n";
        let constants = constant_path_vars(src, "tcl", None);
        assert_eq!(constants.get("d").map(String::as_str), Some("my dir"));
        assert_eq!(constants.get("sub").map(String::as_str), Some("my dir/x"));
        assert_eq!(
            evaluate_auto_path_expr_with_constants("[file join $d y.tcl]", None, &constants)
                .as_deref(),
            Some("my dir/y.tcl"),
        );
    }

    /// A mixed word — literal text around the reference — folds through the
    /// same interpolation grammar, so `source $base/lib/x.tcl` resolves.
    #[test]
    fn a_mixed_word_resolves_through_the_resolver() {
        let constants: HashMap<String, String> = [("base".to_owned(), "/opt".to_owned())]
            .into_iter()
            .collect();
        assert_eq!(
            evaluate_auto_path_expr_with_constants("$base/lib/x.tcl", None, &constants).as_deref(),
            Some("/opt/lib/x.tcl"),
        );
        assert_eq!(
            evaluate_auto_path_expr_with_constants("${base}2/x.tcl", None, &constants).as_deref(),
            Some("/opt2/x.tcl"),
        );
    }

    /// Braces suppress substitution: `{$d}` names a file with a literal `$`
    /// in it, and the fold abstains on it even when the resolver knows `d` —
    /// resolving there would be inventing a substitution Tcl never performs.
    #[test]
    fn a_braced_dollar_word_is_never_resolved() {
        let constants: HashMap<String, String> =
            [("d".to_owned(), "/opt".to_owned())].into_iter().collect();
        assert_eq!(
            evaluate_auto_path_expr_with_constants("[file join {$d} x]", None, &constants),
            None,
        );
    }

    /// The resolver entry point is the lower-layer contract: any caller with
    /// its own variable facts — the analyser's constant-string lattice, an
    /// SCCP value table — plugs in here without this module knowing it.
    #[test]
    fn a_custom_resolver_supplies_variable_provenance() {
        let folded =
            evaluate_auto_path_expr_with_resolver("[file join $root pkg]", None, &|name| {
                (name == "root").then(|| "/from/lattice".to_owned())
            });
        assert_eq!(folded.as_deref(), Some("/from/lattice/pkg"));
        // An unresolved name abstains the whole fold — never a guess.
        assert_eq!(
            evaluate_auto_path_expr_with_resolver("[file join $other pkg]", None, &|_| None),
            None,
        );
    }

    /// Assignments inside a body are not top-level and never enter the map:
    /// the segmenter sees the `proc` body as one braced word.
    #[test]
    fn a_set_inside_a_body_is_not_a_top_level_constant() {
        let src = "proc p {} {\n    set inner /opt\n}\n";
        let constants = constant_path_vars(src, "tcl", None);
        assert!(!constants.contains_key("inner"), "{constants:?}");
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
    /// An `auto_path` entry naming a chained constant contributes its
    /// directory — georgtree/SpiceGenTcl's exact registration shape (issue
    /// #775):
    ///
    /// ```tcl
    /// set dir [file dirname [file normalize [info script]]]
    /// set libDir [file join $dir lib]
    /// lappend auto_path $libDir
    /// ```
    #[test]
    fn an_auto_path_entry_through_a_chained_constant_contributes_its_directory() {
        let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set libDir [file join $dir lib]\n\
                   lappend auto_path $libDir\n";
        let constants = constant_path_vars(src, "tcl", Some("/proj/SpiceGenTcl.tcl"));
        let entry = AutoPathEntry {
            raw_path: "$libDir".to_owned(),
            range: tcl_lexer::Span::new(0, 0),
            form: AutoPathForm::Append,
        };
        assert_eq!(
            evaluate_auto_path_entry_with_constants(
                &entry,
                Some("/proj/SpiceGenTcl.tcl"),
                &constants,
            ),
            vec!["/proj/lib".to_owned()],
        );
        // The constant-less form still contributes nothing — the wiring is
        // opt-in per consumer, never a change to the bare entry point.
        assert!(evaluate_auto_path_entry(&entry, Some("/proj/SpiceGenTcl.tcl")).is_empty());
    }

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
