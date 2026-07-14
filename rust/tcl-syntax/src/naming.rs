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

//! Variable and command name normalisation.
//!
//! These helpers live in the compiler-facing crates because they are
//! consumed by the expression parser and lowering — not by the lexer
//! itself.
//!
//! The `::`-qualifier split ([`qualifier_segments`] / [`is_qualified`]) is the
//! **one** canonical source for namespace-name parsing, shared by the compiler
//! (`normalise_qualified_name`) and the WASM runtime's command **and** variable
//! resolvers (`runtime/rust/src/namespace.rs`, the var coordinator) — mirroring
//! C Tcl's `TclGetNamespaceForQualName` segmentation (`tmp/tcl9.0.3`). Byte-based
//! so the runtime (which works in UTF-8 bytes) and the compiler (`&str`) share it
//! without one re-deriving the other.

/// Does `name` contain a `::` namespace separator (i.e. is it qualified)?
#[must_use]
pub fn is_qualified(name: &[u8]) -> bool {
    name.windows(2).any(|w| w == b"::")
}

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
/// A run of **two or more** colons is one separator (all consecutive colons are
/// consumed), while a lone interior colon is an ordinary name character, so
/// `a:::b` → `[a, b]` and `a:b` → `[a:b]`. A trailing separator drops its empty
/// tail (`a::b::` → `[a, b]`); callers that care about the `{}`-named cmd/var a
/// trailing `::` denotes test for it themselves. Mirrors
/// `TclGetNamespaceForQualName`'s component walk (`tclNamesp.c`).
#[must_use]
pub fn qualifier_segments(name: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut seg_start = 0;
    let mut i = 0;
    while i < name.len() {
        if name[i] == b':' && i + 1 < name.len() && name[i + 1] == b':' {
            if i > seg_start {
                out.push(&name[seg_start..i]);
            }
            // C skips the `::` then every subsequent `:`, so a colon run of any
            // length is a single separator.
            i += 2;
            while i < name.len() && name[i] == b':' {
                i += 1;
            }
            seg_start = i;
        } else {
            i += 1;
        }
    }
    if seg_start < name.len() {
        out.push(&name[seg_start..]);
    }
    out
}

/// Does `name` end with a namespace separator (a run of ≥2 colons)? In a
/// command or variable name such a trailing `::` names the `{}` (empty) entity
/// in the qualified namespace (`TclGetNamespaceForQualName`), so the simple name
/// is `""` rather than the last [`qualifier_segments`] element.
#[must_use]
pub fn ends_with_separator(name: &[u8]) -> bool {
    name.len() >= 2 && name[name.len() - 1] == b':' && name[name.len() - 2] == b':'
}

/// Strip a variable reference's substitution sigil (`$`, `${…}`) while
/// **keeping** any array-index suffix — the form an evaluator needs to read the
/// actual variable (`$arr(idx)` → `arr(idx)`, `${v}` → `v`, `$x` → `x`). Unlike
/// [`normalise_var_name`], the `(idx)` is preserved.
///
/// ```
/// use tcl_syntax::naming::var_reference;
/// assert_eq!(var_reference("$arr(idx)"), "arr(idx)");
/// assert_eq!(var_reference("${v}"), "v");
/// assert_eq!(var_reference("$x"), "x");
/// ```
#[must_use]
pub fn var_reference(name: &str) -> &str {
    if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    }
}

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_syntax::naming::normalise_var_name;
/// assert_eq!(normalise_var_name("$foo"), "foo");
/// assert_eq!(normalise_var_name("${bar}"), "bar");
/// assert_eq!(normalise_var_name("$arr(idx)"), "arr");
/// assert_eq!(normalise_var_name("plain"), "plain");
/// ```
#[must_use]
pub fn normalise_var_name(name: &str) -> &str {
    let base = if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    };

    // Strip array index: keep everything before the first `(`.
    match base.find('(') {
        Some(idx) => &base[..idx],
        None => base,
    }
}

/// Return `true` when `${name}` would successfully look up `name` under the
/// given `${…}` delimiting rule.
///
/// `nesting` selects the dialect's `Tcl_ParseVarName` brace-form parser
/// (`LexerConfig::braced_var`):
///
/// - **`true` — Tcl 9.x** (tcl9.0.1 `tclParse.c`, the `braceCount` loop):
///   `\X` consumes 2 source chars, both kept in the lookup name — so a `}`
///   preceded by `\` does not end the span — and `{` / `}` are tracked with
///   a depth counter; only a `}` at depth 0 ends the var-name span.
///   Unreachable: a `}` at depth 0, a trailing lone `\`, or unbalanced `{`.
/// - **`false` — the 8.x family** (8.6.14 `tclParse.c:1466`,
///   tclsh-verified): the name is everything up to the FIRST literal `}`,
///   with no nesting and no backslash processing — so any name containing a
///   `}` is unreachable, and `{` / `\` are ordinary name characters.
///
/// Drives the W215 reachability check.
#[must_use]
pub fn is_brace_substitutable(name: &str, nesting: bool) -> bool {
    if name.is_empty() {
        return true; // `${}` looks up the var literally named "".
    }
    if !nesting {
        // 8.x: the first literal `}` closes the form, so a name containing
        // one can never be spelt; everything else is verbatim.
        return !name.contains('}');
    }
    let b = name.as_bytes();
    let n = b.len();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < n {
        match b[i] {
            b'}' if depth == 0 => return false,
            b'\\' => {
                if i + 1 >= n {
                    return false;
                }
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    depth == 0
}

/// Return `true` when `$name` would lex as a single bare variable token.
///
/// A name is one or more `::`-separated segments of **ASCII** alphanumerics
/// or `_`, with an optional leading `::` — `TclIsBareword` is ASCII-only in
/// every Tcl (8.6.14 and 9.0.1 `tclParse.c`; the man page: "Letters and
/// digits are only the standard ASCII ones"), so `$café` names the variable
/// `caf`. Used to decide between the bare `$name` and brace `${name}` forms
/// in quick fixes — a Unicode-permissive rule here would let a
/// `${café}` → `$café` rewrite silently change which variable is read.
///
/// ```
/// use tcl_syntax::naming::is_bare_var_name;
/// assert!(is_bare_var_name("foo"));
/// assert!(is_bare_var_name("ns::bar"));
/// assert!(is_bare_var_name("::baz"));
/// assert!(!is_bare_var_name("has-dash"));
/// assert!(!is_bare_var_name("café"));
/// assert!(!is_bare_var_name(""));
/// ```
#[must_use]
pub fn is_bare_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let s = name.strip_prefix("::").unwrap_or(name);
    if s.is_empty() {
        return false;
    }
    for segment in s.split("::") {
        if segment.is_empty() {
            return false;
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return false;
        }
    }
    true
}

/// Return `true` when *word* is the braced indirect-array-element idiom
/// `${name}(index)`.
///
/// Tcl parses `${name}(index)` as the brace-form substitution `${name}`
/// (which the lexer ends at the `}`) concatenated with the *literal* text
/// `(index)`.  In a variable-name position (the target of `set` / `incr` /
/// `append` / `lappend` / `unset` / `info exists` / `vwait`) the resulting
/// string `<value-of-name>(index)` names element `index` of the array whose
/// name is held in the scalar `name` — the standard "array name kept in a
/// variable" idiom (e.g. `set ${token}(status) eof`).  The braces are
/// essential: the bare `$name(index)` is a *direct* array reference, a
/// different construct, so this returns `false` for it.
///
/// This is the discriminator that keeps the W216 (brace-then-paren) and W212
/// (substitution-where-name-expected) checks from false-positiving on the
/// indirect idiom.
///
/// ```
/// use tcl_syntax::naming::is_braced_indirect_array_ref;
/// assert!(is_braced_indirect_array_ref("${token}(status)"));
/// assert!(!is_braced_indirect_array_ref("$arr(idx)"));
/// assert!(!is_braced_indirect_array_ref("${x}"));
/// assert!(!is_braced_indirect_array_ref("${}(x)"));
/// ```
#[must_use]
pub fn is_braced_indirect_array_ref(word: &str) -> bool {
    if !word.starts_with("${") {
        return false;
    }
    let bytes = word.as_bytes();
    let n = bytes.len();
    // Walk to the depth-0 `}` that closes the brace-form variable name.
    let mut i = 2usize;
    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    while i < n {
        match bytes[i] {
            b'\\' if i + 1 < n => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    close = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    // No closing brace, or an empty name (`${}(…)`) — not the idiom.  The
    // closing `}` must sit past the `${` and the (non-empty) name.
    let Some(close) = close.filter(|&c| c > 2) else {
        return false;
    };
    // The closing `}` must be immediately followed by `(…)` running to the
    // end of the word.
    let rest = &bytes[close + 1..];
    rest.len() >= 2 && rest[0] == b'(' && *rest.last().unwrap() == b')'
}

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_syntax::naming::normalise_qualified_name;
/// assert_eq!(normalise_qualified_name("foo"), "::foo");
/// assert_eq!(normalise_qualified_name("ns::bar"), "::ns::bar");
/// assert_eq!(normalise_qualified_name("::baz"), "::baz");
/// assert_eq!(normalise_qualified_name(""), "");
/// assert_eq!(normalise_qualified_name("::::x"), "::x");
/// ```
#[must_use]
pub fn normalise_qualified_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Share the one canonical `::` segmentation. Each segment is a subslice of a
    // `&str` split on ASCII `::`, so it is valid UTF-8.
    let parts: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    if parts.is_empty() {
        return "::".to_owned();
    }
    format!("::{}", parts.join("::"))
}

/// Join a namespace `prefix` and a (possibly-relative) `name` into a fully
/// qualified, `::`-rooted name — the canonical Tcl qualification rule:
///
/// * An **absolute** `name` (leading `::`) ignores the prefix entirely —
///   `qualify("::ns", "::other::C")` is `::other::C`, never re-prefixed.
/// * A relative `name` resolves under `prefix`, which may be given rooted
///   (`::a::b`) or unrooted (`a::b`); duplicate separators collapse.
/// * An empty / root prefix roots the name at `::`.
///
/// The one shared join for the analyser / signature-scan / class-lattice
/// qualifiers, so the absolute-name rule cannot drift between them.
#[must_use]
pub fn qualify(prefix: &str, name: &str) -> String {
    if name.starts_with("::") {
        return normalise_qualified_name(name);
    }
    let p = prefix.trim_start_matches("::").trim_end_matches("::");
    if name.is_empty() {
        // The empty-name entity (`proc {} {} {}`): a trailing separator
        // names the `{}` command/variable in the qualified namespace
        // ([`ends_with_separator`]) — `::` at the root, `::p::` inside `p`.
        return if p.is_empty() {
            "::".to_owned()
        } else {
            format!("::{p}::")
        };
    }
    if p.is_empty() {
        normalise_qualified_name(name)
    } else {
        normalise_qualified_name(&format!("{p}::{name}"))
    }
}

/// Candidate qualified names for Tcl's real bareword command/procedure
/// resolution, in priority order: the current namespace first, then global.
///
/// The `namespace path`-free specialisation of
/// [`command_resolution_candidates`] — see there for the full rule.  Kept
/// as the common entry point for consumers that do not model
/// `namespace path` (the static analyser, the optimiser's interprocedural
/// proc-identity resolution).
///
/// ```
/// use tcl_syntax::naming::bareword_resolution_candidates;
/// assert_eq!(bareword_resolution_candidates("::ns", "::foo"), vec!["::foo"]);
/// assert_eq!(
///     bareword_resolution_candidates("::ns", "inner::p"),
///     vec!["::ns::inner::p", "::inner::p"],
/// );
/// assert_eq!(bareword_resolution_candidates("::ns", "foo"), vec!["::ns::foo", "::foo"]);
/// assert_eq!(bareword_resolution_candidates("::", "foo"), vec!["::foo"]);
/// ```
#[must_use]
pub fn bareword_resolution_candidates(namespace: &str, cmd_name: &str) -> Vec<String> {
    command_resolution_candidates::<&str>(namespace, &[], cmd_name)
}

/// Candidate qualified names for Tcl's command resolution, in priority
/// order, including the current namespace's `namespace path`.
///
/// This is the **canonical encoding of C Tcl's command-lookup order**
/// (`Tcl_FindCommand`, `generic/tclNamesp.c`, Tcl 9.0.4) — the algorithm
/// every backend must agree with (behaviour pinned against tclsh 8.6.16
/// and 9.0.4 by `tests/command_resolution_conformance.rs`):
///
/// * An absolute name (`::foo`, `::ns::foo`) is taken as-is — one
///   candidate; the path is **not** consulted.
/// * Any relative name — bare (`foo`) *or* with embedded qualifiers
///   (`inner::p`) — is resolved against the current namespace first, then
///   each `namespace path` entry in order, then the global namespace.
///   The dispatch target is the first candidate that **exists**; mere
///   existence of an intermediate *namespace* does not commit resolution
///   (calling `inner::p` from `::outer` reaches `::inner::p` even when
///   the namespace `::outer::inner` exists but holds no `p`).
/// * No implicit ancestor walk: a bare `helper` inside `::a::b` never
///   reaches `::a::helper` unless `::a` is on the `namespace path`.
///
/// `path` entries may be given rooted (`::pathns`) or as written
/// (`pathns`): a *relative* entry is **current-namespace-relative only**
/// — `namespace path inner` inside `::outer` means `::outer::inner`,
/// never `::inner` (tclsh 8.6.16 / 9.0.4: the set errors `namespace
/// "inner" not found in "::outer"` when `::outer::inner` does not exist,
/// even with `::inner` present — namespace names have no global
/// fallback, unlike command names).  Consumers that do not model
/// `namespace path` pass `&[]` (equivalently, use
/// [`bareword_resolution_candidates`]).
///
/// ```
/// use tcl_syntax::naming::command_resolution_candidates;
/// // path slots between the current namespace and global:
/// assert_eq!(
///     command_resolution_candidates("::c", &["::pathns"], "helper"),
///     vec!["::c::helper", "::pathns::helper", "::helper"],
/// );
/// // a relative path entry is current-namespace-relative (tclsh-pinned):
/// assert_eq!(
///     command_resolution_candidates("::d", &["pathq"], "sub::q"),
///     vec!["::d::sub::q", "::d::pathq::sub::q", "::sub::q"],
/// );
/// // absolute names ignore the path entirely:
/// assert_eq!(
///     command_resolution_candidates("::d", &["::pathq"], "::sub::q"),
///     vec!["::sub::q"],
/// );
/// // duplicates collapse, keeping the highest-priority position:
/// assert_eq!(
///     command_resolution_candidates("::", &["::"], "foo"),
///     vec!["::foo"],
/// );
/// ```
#[must_use]
pub fn command_resolution_candidates<S: AsRef<str>>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
) -> Vec<String> {
    if cmd_name.starts_with("::") {
        return vec![cmd_name.to_owned()];
    }
    let mut out: Vec<String> = Vec::with_capacity(path.len() + 2);
    let push_base = |base: &str, out: &mut Vec<String>| {
        let candidate = if base.is_empty() || base == "::" {
            format!("::{cmd_name}")
        } else {
            let rooted = normalise_qualified_name(base);
            format!("{rooted}::{cmd_name}")
        };
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    };
    push_base(namespace, &mut out);
    for entry in path {
        let entry = entry.as_ref();
        if entry.starts_with("::") {
            push_base(entry, &mut out);
        } else {
            // Relative entry: current-namespace-relative only (see above).
            let based = if namespace.is_empty() || namespace == "::" {
                format!("::{entry}")
            } else {
                format!("{namespace}::{entry}")
            };
            push_base(&based, &mut out);
        }
    }
    push_base("::", &mut out);
    out
}

/// Resolve a command name the way C Tcl's `Tcl_FindCommand` does: walk
/// [`command_resolution_candidates`] in priority order and return the
/// first candidate for which `exists` is true.
///
/// `exists` is the caller's command table — the analyser's collected
/// definitions, a compilation unit's proc map, or a live interpreter's
/// registry.  Returns `None` when no candidate exists (Tcl would raise
/// `invalid command name` / fall through to `unknown`).
///
/// ```
/// use tcl_syntax::naming::resolve_command_with;
/// let defined = ["::inner::p"];
/// let exists = |q: &str| defined.contains(&q);
/// // local candidate absent -> falls back to the global command:
/// assert_eq!(
///     resolve_command_with::<&str, _>("::outer", &[], "inner::p", exists),
///     Some("::inner::p".to_string()),
/// );
/// // nothing defined -> unresolved:
/// assert_eq!(resolve_command_with::<&str, _>("::outer", &[], "nope", exists), None);
/// ```
#[must_use]
pub fn resolve_command_with<S: AsRef<str>, F: FnMut(&str) -> bool>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
    mut exists: F,
) -> Option<String> {
    command_resolution_candidates(namespace, path, cmd_name)
        .into_iter()
        .find(|candidate| exists(candidate))
}

/// Split a Tcl variable reference into `(base, element)`.
///
/// Strips `$` / `${…}` substitution sigils first, then separates the
/// optional `(element)` array-index suffix from the base name.  Returns
/// `(base, None)` for scalar references.  Follows the brace-form rule that
/// `${arr}(foo)` is the scalar `arr` followed by literal `(foo)`, whereas
/// `${arr(foo)}` *is* the array element `arr(foo)`.
///
/// ```
/// use tcl_syntax::naming::split_array_name;
/// assert_eq!(split_array_name("arr"), ("arr", None));
/// assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
/// assert_eq!(split_array_name("${arr}"), ("arr", None));
/// ```
#[must_use]
pub fn split_array_name(name: &str) -> (&str, Option<&str>) {
    // `${…}` brace form: only the chars inside the braces are the reference;
    // an `(idx)` *inside* the braces is an element, one *after* `}` is not.
    if let Some(after) = name.strip_prefix("${")
        && let Some(rel) = after.find('}')
    {
        let inner = &after[..rel];
        if inner.ends_with(')')
            && let Some(idx) = inner.find('(')
        {
            return (&inner[..idx], Some(&inner[idx + 1..inner.len() - 1]));
        }
        return (inner, None);
    }
    // No closing brace — fall through (gated on `"}" in base`).
    let base = name.strip_prefix('$').unwrap_or(name);
    if base.ends_with(')')
        && let Some(idx) = base.find('(')
    {
        return (&base[..idx], Some(&base[idx + 1..base.len() - 1]));
    }
    (base, None)
}

/// True when `word` carries a variable / command substitution anywhere —
/// e.g. `rename ::$c mystr` or `rename foo bar[x]` — so it cannot be
/// resolved to a static command name at compile time.
///
/// ```
/// use tcl_syntax::naming::is_dynamic_word;
/// assert!(!is_dynamic_word("foo"));
/// assert!(is_dynamic_word("::$c"));
/// assert!(is_dynamic_word("bar[x]"));
/// ```
#[must_use]
pub fn is_dynamic_word(word: &str) -> bool {
    word.contains('$') || word.contains('[')
}

/// Shared command-resolution conformance vectors.
///
/// One table of `(namespace, namespace path, defined commands, call,
/// expected winner)` rows — the executable ground truth for
/// [`resolve_command_with`](super::naming::resolve_command_with) and for
/// every backend that re-implements the rule over its own command store
/// (the analyser's post-walk settlement, the bytecode VM's dispatch, the
/// WASM runtime's namespace tree).  Each consumer runs the *same* rows, so
/// a change to the rule that lands in one implementation but not another
/// fails that consumer's conformance test rather than drifting silently.
///
/// The rows themselves are pinned against real tclsh by
/// `tests/command_resolution_conformance.rs` in this crate, which renders
/// each row with [`conformance::vector_script`] and diffs the interpreter's
/// answer against `want`.
pub mod conformance {
    /// One resolution scenario: from `ns` (with `path` as its
    /// `namespace path`), a call to `call` with exactly `defs` defined
    /// must dispatch `want` (`None` = `invalid command name`).
    #[derive(Debug, Clone)]
    pub struct ResolutionVector {
        /// `::`-rooted current namespace (`::` = global).
        pub ns: String,
        /// `namespace path` entries for `ns`, in order (each `::`-rooted).
        pub path: Vec<String>,
        /// Every defined command, `::`-rooted.
        pub defs: Vec<String>,
        /// The call text as written (bare, relative-qualified, or absolute).
        pub call: String,
        /// The `::`-rooted winner, or `None` for `invalid command name`.
        pub want: Option<String>,
        /// 1-based line in the vector file (for failure messages).
        pub line: usize,
    }

    /// The raw vector table (see the file header for the format).
    pub const RAW: &str = include_str!("../tests/data/command_resolution_vectors.txt");

    /// Parse [`RAW`] into vectors.
    ///
    /// # Panics
    /// On a malformed row — the file is repo-controlled test data, so a
    /// parse failure is a bug in the row, not an input condition.
    #[must_use]
    pub fn vectors() -> Vec<ResolutionVector> {
        let split_list = |field: &str| -> Vec<String> {
            if field == "-" {
                Vec::new()
            } else {
                field.split(',').map(|s| s.trim().to_string()).collect()
            }
        };
        let mut out = Vec::new();
        for (idx, raw_line) in RAW.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            assert!(
                fields.len() == 5,
                "vector line {}: expected 5 |-separated fields, got {}: {raw_line:?}",
                idx + 1,
                fields.len(),
            );
            out.push(ResolutionVector {
                ns: fields[0].to_string(),
                path: split_list(fields[1]),
                defs: split_list(fields[2]),
                call: fields[3].to_string(),
                want: (fields[4] != "-").then(|| fields[4].to_string()),
                line: idx + 1,
            });
        }
        assert!(!out.is_empty(), "no conformance vectors parsed");
        out
    }

    /// Render a vector as a runnable Tcl script whose **output** is the
    /// dispatched command's qualified name, or `-` on
    /// `invalid command name`.
    ///
    /// Each defined command becomes a proc returning its own qualified
    /// name; the call runs inside `namespace eval {ns}` after any
    /// `namespace path` is applied.  Shared by the tclsh pin test, the
    /// bytecode-VM conformance test, and the WASM-runtime conformance
    /// test, so all three execute byte-identical scripts.
    #[must_use]
    pub fn vector_script(v: &ResolutionVector) -> String {
        use std::fmt::Write as _;
        let mut script = String::new();
        // Every namespace referenced anywhere must exist up front: the
        // call namespace, each path entry, and each definition's holder
        // (`namespace eval ::a::b {}` creates intermediate levels too).
        let ensure_ns = |ns: &str, script: &mut String| {
            if ns != "::" && !ns.is_empty() {
                let _ = writeln!(script, "namespace eval {ns} {{}}");
            }
        };
        ensure_ns(&v.ns, &mut script);
        for p in &v.path {
            ensure_ns(p, &mut script);
        }
        for def in &v.defs {
            if let Some((holder, _tail)) = def.rsplit_once("::") {
                ensure_ns(holder, &mut script);
            }
            let _ = writeln!(script, "proc {def} {{}} {{ return {def} }}");
        }
        if !v.path.is_empty() {
            let entries = v.path.join(" ");
            let _ = writeln!(
                script,
                "namespace eval {} [list namespace path [list {entries}]]",
                v.ns
            );
        }
        let _ = writeln!(
            script,
            "if {{[catch {{namespace eval {} {{{}}}}} __r]}} {{ set __r - }}",
            v.ns, v.call
        );
        script.push_str("puts $__r\n");
        script
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn qualify_joins_relative_names_under_rooted_and_unrooted_prefixes() {
        assert_eq!(qualify("::ns", "C"), "::ns::C");
        assert_eq!(qualify("ns", "C"), "::ns::C");
        assert_eq!(qualify("::a::b", "c::D"), "::a::b::c::D");
        assert_eq!(qualify("", "C"), "::C");
        assert_eq!(qualify("::", "C"), "::C");
    }

    #[test]
    fn qualify_names_the_empty_entity_with_a_trailing_separator() {
        // `proc {} {} {}` — the empty-name command is the `{}` entity the
        // trailing separator denotes.
        assert_eq!(qualify("", ""), "::");
        assert_eq!(qualify("::ns", ""), "::ns::");
    }

    #[test]
    fn qualify_keeps_absolute_names_absolute() {
        // The class-lattice regression: an absolute name must never be
        // re-prefixed under the current namespace.
        assert_eq!(qualify("::ns", "::other::C"), "::other::C");
        assert_eq!(qualify("ns", "::C"), "::C");
    }
    use super::*;

    #[test]
    fn simple_dollar() {
        assert_eq!(normalise_var_name("$foo"), "foo");
    }

    #[test]
    fn brace_substitutable_cases_tcl9_nesting() {
        // Reachable names under the Tcl 9 rule (nested braces + `\X` pairs).
        assert!(is_brace_substitutable("", true));
        assert!(is_brace_substitutable("foo", true));
        assert!(is_brace_substitutable("a(b)", true)); // `)` is fine in brace form
        assert!(is_brace_substitutable("a{b}c", true)); // balanced inner braces
        assert!(is_brace_substitutable(r"a\}b", true)); // `\}` consumes 2, kept
        // Unreachable names.
        assert!(!is_brace_substitutable("a}b", true)); // `}` at depth 0 ends span early
        assert!(!is_brace_substitutable(r"trail\", true)); // trailing lone backslash
        assert!(!is_brace_substitutable("a{b", true)); // unbalanced `{`
    }

    #[test]
    fn brace_substitutable_cases_tcl8_first_close() {
        // The 8.x family closes at the FIRST literal `}` (8.6.14
        // `tclParse.c:1466`, tclsh-verified: `${a{b}}` reads variable `a{b`).
        assert!(is_brace_substitutable("", false));
        assert!(is_brace_substitutable("foo", false));
        assert!(is_brace_substitutable("a(b)", false));
        // `{` and `\` are ordinary name characters — no pairing, no escapes.
        assert!(is_brace_substitutable("a{b", false));
        assert!(is_brace_substitutable(r"trail\", false));
        // ANY `}` in the name is unreachable — nesting/escapes don't help.
        assert!(!is_brace_substitutable("a{b}c", false));
        assert!(!is_brace_substitutable(r"a\}b", false));
        assert!(!is_brace_substitutable("a}b", false));
    }

    #[test]
    fn split_array_name_forms() {
        assert_eq!(split_array_name("arr"), ("arr", None));
        assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
        // `${arr}(foo)` is scalar `arr` then literal `(foo)` — not an element.
        assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
        assert_eq!(split_array_name("${arr}"), ("arr", None));
        // dynamic index text is preserved verbatim for later classification.
        assert_eq!(split_array_name("a($i)"), ("a", Some("$i")));
        // a `)` with no `(` is not an element.
        assert_eq!(split_array_name("weird)"), ("weird)", None));
    }

    #[test]
    fn braced_dollar() {
        assert_eq!(normalise_var_name("${bar}"), "bar");
    }

    #[test]
    fn array_stripped() {
        assert_eq!(normalise_var_name("$arr(idx)"), "arr");
    }

    #[test]
    fn braced_array_stripped() {
        assert_eq!(normalise_var_name("${arr(idx)}"), "arr");
    }

    #[test]
    fn no_prefix() {
        assert_eq!(normalise_var_name("plain"), "plain");
    }

    #[test]
    fn namespace_qualified() {
        assert_eq!(normalise_var_name("$ns::var"), "ns::var");
    }

    #[test]
    fn empty_string() {
        assert_eq!(normalise_var_name(""), "");
    }

    #[test]
    fn bare_dollar() {
        assert_eq!(normalise_var_name("$"), "");
    }

    // normalise_qualified_name tests

    #[test]
    fn qualified_bare() {
        assert_eq!(normalise_qualified_name("foo"), "::foo");
    }

    #[test]
    fn qualified_already() {
        assert_eq!(normalise_qualified_name("::bar"), "::bar");
    }

    #[test]
    fn qualified_nested() {
        assert_eq!(normalise_qualified_name("ns::sub"), "::ns::sub");
    }

    #[test]
    fn qualified_empty() {
        assert_eq!(normalise_qualified_name(""), "");
    }

    #[test]
    fn qualified_just_colons() {
        assert_eq!(normalise_qualified_name("::"), "::");
    }

    #[test]
    fn qualified_extra_colons() {
        assert_eq!(normalise_qualified_name("::::x"), "::x");
    }

    // qualifier_segments / is_qualified

    #[test]
    fn qualifier_segments_cases() {
        assert_eq!(
            qualifier_segments(b"::a::b::cmd"),
            vec![&b"a"[..], b"b", b"cmd"]
        );
        assert_eq!(qualifier_segments(b"::cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"a::b"), vec![&b"a"[..], b"b"]);
        assert!(qualifier_segments(b"::").is_empty());
        // a trailing separator drops the empty tail; a lone interior colon stays.
        assert_eq!(qualifier_segments(b"a::b::"), vec![&b"a"[..], b"b"]);
        // a run of >=2 colons is one separator (all consecutive colons consumed).
        assert_eq!(qualifier_segments(b"a:::b"), vec![&b"a"[..], b"b"]);
        assert_eq!(qualifier_segments(b"a::::b"), vec![&b"a"[..], b"b"]);
        assert_eq!(
            qualifier_segments(b":::test_ns_1:::::test_ns_2:::"),
            vec![&b"test_ns_1"[..], b"test_ns_2"]
        );
        // a lone interior colon is an ordinary name character.
        assert_eq!(qualifier_segments(b"a:b"), vec![&b"a:b"[..]]);
        assert!(ends_with_separator(b"a::b::"));
        assert!(!ends_with_separator(b"a::b"));
    }

    #[test]
    fn is_qualified_cases() {
        assert!(is_qualified(b"::a"));
        assert!(is_qualified(b"a::b"));
        assert!(is_qualified(b"::"));
        assert!(!is_qualified(b"plain"));
        assert!(!is_qualified(b"a:b"));
        assert!(!is_qualified(b""));
    }
}
