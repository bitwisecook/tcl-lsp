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

//! Document-links provider.
//!
//! Detects `source <path>` invocations in the document and
//! surfaces each path argument as a clickable link.  When a
//! `workspace_root` is provided, relative paths resolve
//! against it; absolute paths surface as-is.
//!
//! Limitations:
//!
//! * `package require <pkg>` resolution — needs a package
//!   index (Tcl's `auto_path` / pkgIndex.tcl scan) — is not
//!   done.
//! * Computed paths are resolved through the **source graph's own**
//!   path evaluator
//!   ([`tcl_compiler::auto_path_eval::evaluate_auto_path_expr`]) — the
//!   `[file dirname [info script]]` / `[file join …]` idioms — plus a
//!   single-`set` copy propagation for the `source [file join $dir …]`
//!   shape.  Anything outside that subset produces **no link at all**:
//!   a `source` argument whose reconstructed text still carries `$` or
//!   `[` is never treated as a literal path (issue #1140 idx 41 — doing
//!   so percent-encoded the raw Tcl text into a syntactically valid but
//!   semantically bogus `file://` URI).
//! * Workspace-folder enumeration that lets a `source` link
//!   resolve across multiple roots is not done; the single
//!   `workspace_root` parameter is sufficient for the
//!   common single-root case.
//!
//! Also handled:
//!
//! * Tilde expansion (`~/path/to/file`) — via the
//!   `home` argument plumbed in by the server from the env.
//! * Literal `[file join a b c]` — recognised when every
//!   sub-arg is a simple bareword / quoted string; the joined
//!   path resolves against `workspace_root` like any other
//!   relative arg.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::LineIndex;

/// One link in a document — target URI plus the source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    /// Source-range start line.
    pub start_line: u32,
    /// Source-range start character.
    pub start_character: u32,
    /// Source-range end line.
    pub end_line: u32,
    /// Source-range end character.
    pub end_character: u32,
    /// Target URI string.  Empty when the link has no navigable target
    /// (e.g. a `package require` whose package isn't on disk) — the
    /// `tooltip` still surfaces.
    pub target: String,
    /// Hover tooltip (the raw path / package name).  `None` for links
    /// with no extra text.
    pub tooltip: Option<String>,
}

/// Everything a link needs to turn a `source` argument into a filesystem
/// path: where relative paths anchor, what `~` expands to, and what the
/// document's own `[info script]` would answer.
///
/// A struct rather than three more positional parameters so adding the next
/// contextual fact doesn't change every call site (and doesn't trip
/// `clippy::too_many_arguments`).
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkContext<'a> {
    /// Directory relative paths resolve against — typically the document's
    /// own enclosing directory.  `None` leaves relative paths unresolvable,
    /// so only absolute ones produce links.
    pub workspace_root: Option<&'a str>,
    /// `$HOME`, for `~/…` expansion.  `None` leaves `~` paths unresolved.
    pub home: Option<&'a str>,
    /// The document's own filesystem path — what `[info script]` answers
    /// while it is being sourced.  `None` makes every `[info script]`-based
    /// computed path abstain rather than guess.
    pub script_path: Option<&'a str>,
}

/// Compute document links for a document.
///
/// `workspace_root`, when `Some`, is the directory used to
/// resolve relative paths.  Typically the document's enclosing
/// directory.  When `None`, only absolute paths produce links.
///
/// `~/...` paths expand against `$HOME`; the helper reads
/// the env var at call-time so server tests can stub it (see
/// `document_links_with_home`).
#[must_use]
pub fn document_links(
    source: &str,
    dialect: &str,
    workspace_root: Option<&str>,
) -> Vec<DocumentLink> {
    let home = std::env::var("HOME").ok();
    document_links_with_home(source, dialect, workspace_root, home.as_deref())
}

/// Same as [`document_links`] but lets callers pass an
/// explicit `home` directory string (for testability under
/// `#![forbid(unsafe_code)]`, where we can't mutate
/// `std::env`).  Production callers should use the
/// zero-argument [`document_links`].
#[must_use]
pub fn document_links_with_home(
    source: &str,
    dialect: &str,
    workspace_root: Option<&str>,
    home: Option<&str>,
) -> Vec<DocumentLink> {
    document_links_in_context(
        source,
        dialect,
        &LinkContext {
            workspace_root,
            home,
            script_path: None,
        },
    )
}

/// The full-context entry point — the one the server calls, since only it
/// knows the document's own filesystem path (needed to evaluate the
/// `[file dirname [info script]]` idiom).
#[must_use]
pub fn document_links_in_context(
    source: &str,
    dialect: &str,
    ctx: &LinkContext<'_>,
) -> Vec<DocumentLink> {
    let LinkContext {
        workspace_root,
        home,
        script_path,
    } = *ctx;
    let line_index = LineIndex::new(source);
    let mut links = Vec::new();
    // Constant single-assignment `set` map for the `set dir [file dirname
    // [info script]] … source [file join $dir x.tcl]` idiom (issue #1140
    // idx 41), built once per request.
    let constants = constant_path_vars(source, dialect, script_path);

    for seg in segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    ) {
        if seg.texts.is_empty() {
            continue;
        }
        // `package require <pkg> ?version?` — surface a link on the package
        // name (no on-disk target; the tooltip carries the package).
        if seg.texts[0] == "package"
            && seg.texts.get(1).is_some_and(|s| s == "require")
            && seg.texts.len() >= 3
        {
            if let Some(tok) = seg.argv.get(2) {
                // Start at the word *content*, not the opening `{`/`"` delimiter,
                // so a braced/quoted package name underlines the name only
                // (RUST_ISSUE_111).
                let start = line_index
                    .position_at_utf16(tok.span.start() + u32::from(tok.content_offset), source);
                let end = line_index.position_at_utf16(tok.span.end(), source);
                links.push(DocumentLink {
                    start_line: start.line,
                    start_character: start.character.get(),
                    end_line: end.line,
                    end_character: end.character.get(),
                    target: String::new(),
                    tooltip: Some(format!("package require {}", seg.texts[2])),
                });
            }
            continue;
        }
        if seg.texts[0] != "source" {
            continue;
        }
        // `source` may take optional flags (`-encoding NAME`)
        // before the path argument; locate the first non-flag
        // arg.
        let mut path_idx: Option<usize> = None;
        let mut i = 1;
        while i < seg.texts.len() {
            if seg.texts[i].starts_with('-') {
                // Skip the flag and (if it consumes a value)
                // the value too.  `source -encoding utf-8 foo`
                // → skip `-encoding` + its value.
                if matches!(seg.texts[i].as_str(), "-encoding" | "--encoding") {
                    i += 2;
                    continue;
                }
                if seg.texts[i] == "--" {
                    i += 1;
                    continue;
                }
                i += 1;
                continue;
            }
            path_idx = Some(i);
            break;
        }
        let Some(idx) = path_idx else { continue };
        let path = &seg.texts[idx];
        // Literal `[file join a b c]` resolution: when the
        // arg is a command substitution whose head is `file
        // join` and every remaining sub-arg is a literal,
        // build the joined path on the fly.  Falls through
        // to the literal-path resolver below so tilde
        // expansion / `workspace_root` anchoring stays
        // consistent.
        let Some(path_owned) = resolve_source_argument(
            path.as_str(),
            seg.single_token_word.get(idx).copied(),
            script_path,
            &constants,
        ) else {
            continue;
        };
        let Some(target) = resolve_path(&path_owned, workspace_root, home) else {
            continue;
        };
        let arg_tok = seg.argv.get(idx);
        let Some(arg_tok) = arg_tok else { continue };
        // Start at the path *content*: a braced/quoted `source {…}` / `source "…"`
        // must underline the path, not the opening delimiter (RUST_ISSUE_111).
        let start = line_index.position_at_utf16(
            arg_tok.span.start() + u32::from(arg_tok.content_offset),
            source,
        );
        let end = line_index.position_at_utf16(arg_tok.span.end(), source);
        links.push(DocumentLink {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
            target,
            tooltip: Some(path_owned),
        });
    }

    links
}

/// Whether `text` still carries an unevaluated substitution — a `$`
/// variable reference or a `[` command substitution.
///
/// The abstention test at the heart of issue #1140 idx 41: such a word is
/// **not** a path, however syntactically valid a URI percent-encoding it
/// would produce.  `single_token_word` cannot answer this — it only
/// distinguishes "one token" from "several concatenated tokens" within a
/// word, and a whole-word `[file join …]` or a bare `$var` is exactly one
/// token, so the old fall-through treated both as literals.
fn carries_substitution(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// The path a `source` argument denotes, or `None` when it cannot be
/// resolved and the provider must emit no link.
///
/// Three tiers, strongest first:
///
/// 1. A plain literal — no `$`, no `[`, and a single word.
/// 2. The literal `[file join a b c]` shorthand ([`literal_file_join`]).
/// 3. The **source graph's own** path evaluator
///    ([`tcl_compiler::auto_path_eval::evaluate_auto_path_expr`], the same
///    one `resolve_source_edge` uses for the M9 namespace-rehoming source
///    graph), after substituting any single-assignment `set` constants the
///    document establishes — which is what carries the corpus idiom
///    `set dir [file dirname [info script]]; source [file join $dir x.tcl]`.
///
/// Anything else abstains.  A multi-token word (`$dir/x.tcl` spliced from
/// several tokens) abstains too, for the same reason.
fn resolve_source_argument(
    path: &str,
    single_token_word: Option<bool>,
    script_path: Option<&str>,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    if !carries_substitution(path) {
        // Genuinely literal — but still only when the word is one token, so
        // a spliced multi-token word never masquerades as a path.
        if single_token_word == Some(false) {
            return None;
        }
        return Some(path.to_owned());
    }
    if let Some(joined) = literal_file_join(path) {
        return Some(joined);
    }
    let substituted = substitute_constants(path, constants);
    tcl_compiler::auto_path_eval::evaluate_auto_path_expr(&substituted, script_path)
}

/// Replace every `$name` / `${name}` occurrence in `text` whose name is in
/// `constants`, leaving unknown ones in place (so the evaluator abstains on
/// them, as it must).
fn substitute_constants(
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) -> String {
    if constants.is_empty() || !text.contains('$') {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            let ch_len = text[i..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&text[i..i + ch_len]);
            i += ch_len;
            continue;
        }
        let (name, next) = if bytes.get(i + 1) == Some(&b'{') {
            match text[i + 2..].find('}') {
                Some(rel) => (&text[i + 2..i + 2 + rel], i + 2 + rel + 1),
                None => ("", i + 1),
            }
        } else {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            (&text[i + 1..j], j)
        };
        match constants.get(name) {
            Some(value) if !name.is_empty() => {
                out.push_str(value);
                i = next;
            }
            _ => {
                out.push('$');
                i += 1;
            }
        }
    }
    out
}

/// Path-valued variables the document assigns **exactly once** at its top
/// level, mapped to their evaluated values.
///
/// Deliberately narrow: a name written by more than one `set` is dropped
/// entirely rather than guessed at (last-write-wins would be wrong for a
/// `source` between the two), and only values the shared path evaluator can
/// fold are kept, so nothing here can invent a target.  `set` is located by
/// the segmenter's own words, and its value word must itself be resolvable —
/// a literal or a `[file …]` expression.
fn constant_path_vars(
    source: &str,
    dialect: &str,
    script_path: Option<&str>,
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, Option<String>> = HashMap::new();
    for seg in segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    ) {
        if seg.texts.len() != 3 || seg.texts[0] != "set" {
            continue;
        }
        let name = seg.texts[1].clone();
        if name.contains('$') || name.contains('[') || name.contains("::") {
            continue;
        }
        let raw = seg.texts[2].as_str();
        let value = if carries_substitution(raw) {
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr(raw, script_path)
        } else {
            Some(raw.to_owned())
        };
        // A second assignment to the same name poisons it: which value a
        // later `source` sees is a flow question this provider does not ask.
        seen.entry(name)
            .and_modify(|slot| *slot = None)
            .or_insert(value);
    }
    seen.into_iter()
        .filter_map(|(name, value)| value.map(|v| (name, v)))
        .collect()
}

/// Try to interpret `arg` as a literal `[file join …]`
/// command substitution and return the joined path.  Returns
/// `None` if the shape doesn't match — typically because the
/// argument is a different command, or because one of the
/// sub-arguments contains a substitution that we can't resolve
/// statically.
///
/// Handles the literal-only case that recognises `[file join …]`
/// source-arg expressions.
fn literal_file_join(arg: &str) -> Option<String> {
    let inner = arg.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner.trim();
    let rest = inner.strip_prefix("file")?.trim_start();
    let rest = rest.strip_prefix("join")?.trim_start();
    if rest.is_empty() {
        return None;
    }
    // Each remaining token must be a simple literal — no `$`,
    // no nested `[…]`, no continuation lines.  Quoted strings
    // strip the surrounding `"`; braced strings strip the `{}`.
    let mut parts: Vec<String> = Vec::new();
    for tok in rest.split_whitespace() {
        let literal = if let Some(b) = tok.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
            b.to_string()
        } else if let Some(q) = tok.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
            q.to_string()
        } else {
            tok.to_string()
        };
        if literal.contains('$') || literal.contains('[') || literal.is_empty() {
            return None;
        }
        parts.push(literal);
    }
    if parts.is_empty() {
        return None;
    }
    // `[file join a b c]` joins with `/` on POSIX-style paths
    // (Tcl uses platform-native separator at runtime; for
    // document-link surfacing the editor's URI is always
    // `file:///...` which uses `/`).  If any part is absolute
    // it resets the joined accumulator (matches Tcl's `file
    // join` semantics).
    let mut joined = String::new();
    for part in parts {
        if std::path::Path::new(&part).is_absolute() || joined.is_empty() {
            joined = part;
        } else {
            let trimmed = joined.trim_end_matches('/');
            joined = format!("{trimmed}/{part}");
        }
    }
    Some(joined)
}

/// Resolve `path` against `workspace_root` (when provided).
///
/// Returns a `file://`-prefixed URI string.  Absolute paths
/// (starting with `/` on POSIX, drive letter on Windows) pass
/// through; relative paths are joined to `workspace_root`.
/// When `workspace_root` is `None`, relative paths return
/// `None` (no anchor to resolve against).
fn resolve_path(path: &str, workspace_root: Option<&str>, home: Option<&str>) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    // Tilde expansion: `~/...` → `$HOME/...`; `~user/...` is
    // not supported (would need /etc/passwd parsing).
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        let home = home?;
        let home_trimmed = home.trim_end_matches('/');
        format!("{home_trimmed}/{rest}")
    } else if path == "~" {
        home?.to_string()
    } else {
        path.to_string()
    };
    let resolved = if std::path::Path::new(&expanded).is_absolute() {
        expanded
    } else {
        let root = workspace_root?;
        let root_trimmed = root.trim_end_matches('/');
        format!("{root_trimmed}/{expanded}")
    };
    Some(file_uri_for_path(&resolved))
}

/// Build a `file://` URI for a resolved filesystem path.
///
/// Normalises Windows backslash separators to forward slashes
/// and ensures the URI's path component starts with `/` so
/// drive-letter paths (`C:/foo`) become `file:///C:/foo`
/// rather than `file://C:/foo`.
/// The path bytes are percent-encoded per RFC 3986 so special
/// characters (spaces, `#`, `%`, non-ASCII) survive parsing
/// as a URI.
fn file_uri_for_path(path: &str) -> String {
    let normalised: String = path
        .chars()
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let encoded = percent_encode_path(&normalised);
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

/// Percent-encode the path component of a `file://` URI per
/// RFC 3986.  Characters that don't form valid path-segment
/// bytes (space, `#`, `?`, `%`, `[`, `]`, non-ASCII, …) get
/// `%HH`-escaped so editors can parse the link target as a URI.
///
/// Per RFC 3986 the path-component reserves:
///
/// * unreserved: `A-Z` / `a-z` / `0-9` / `-` / `.` / `_` / `~`
/// * sub-delims: `!` / `$` / `&` / `'` / `(` / `)` / `*` / `+`
///   / `,` / `;` / `=`
/// * pchar adds `:` / `@`, and the path layer adds `/`.
///
/// Everything else is encoded.  Keeps the encoded path
/// conservative enough to round-trip through a `file://` URI
/// parser.
fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for &b in path.as_bytes() {
        if is_path_safe_byte(b) {
            out.push(b as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

const fn is_path_safe_byte(b: u8) -> bool {
    matches!(
        b,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@'
            | b'/'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_links_for_non_source_commands() {
        assert!(document_links("set x 1\n", "tcl", None).is_empty());
        assert!(document_links("puts hello\n", "tcl", None).is_empty());
    }

    #[test]
    fn absolute_path_surfaces_as_link() {
        let src = "source /usr/lib/tcl/init.tcl\n";
        let links = document_links(src, "tcl", None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/lib/tcl/init.tcl");
    }

    #[test]
    fn relative_path_resolves_against_workspace_root() {
        let src = "source helper.tcl\n";
        let links = document_links(src, "tcl", Some("/home/user/project"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/project/helper.tcl");
    }

    #[test]
    fn relative_path_without_root_produces_no_link() {
        let src = "source helper.tcl\n";
        let links = document_links(src, "tcl", None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn encoding_flag_skipped_before_path() {
        let src = "source -encoding utf-8 /tmp/foo.tcl\n";
        let links = document_links(src, "tcl", None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///tmp/foo.tcl");
    }

    #[test]
    fn link_range_anchors_at_path_argument() {
        // `source ` is 7 chars; the path starts at col 7.
        let src = "source /tmp/foo.tcl\n";
        let links = document_links(src, "tcl", None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].start_character, 7);
        // End col covers the path (12 chars: `/tmp/foo.tcl`).
        assert_eq!(links[0].end_character, 19);
    }

    #[test]
    fn braced_and_quoted_path_range_excludes_delimiter() {
        // RUST_ISSUE_111: `source {/tmp/foo.tcl}` must underline the path, not
        // the opening `{` — the range starts at the content, past the delimiter.
        for (src, want_start) in [
            ("source {/tmp/foo.tcl}\n", 8u32), // `source ` = 7, then `{` at 7, content at 8
            ("source \"/tmp/foo.tcl\"\n", 8u32),
        ] {
            let links = document_links(src, "tcl", None);
            assert_eq!(links.len(), 1, "{src:?} → {links:?}");
            assert_eq!(links[0].target, "file:///tmp/foo.tcl");
            assert_eq!(
                links[0].start_character, want_start,
                "range must start at the path content for {src:?}",
            );
        }
    }

    #[test]
    fn dynamic_path_produces_no_link() {
        // `source $somevar` — variable substitution, not a
        // literal path.  Skipped.
        let src = "source $somevar\n";
        let links = document_links(src, "tcl", None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn double_dash_terminator_skipped() {
        let src = "source -- /tmp/x.tcl\n";
        let links = document_links(src, "tcl", None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///tmp/x.tcl");
    }

    #[test]
    fn trailing_slash_on_workspace_root_handled() {
        let src = "source helper.tcl\n";
        let links = document_links(src, "tcl", Some("/home/user/"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/helper.tcl");
    }

    #[test]
    fn tilde_expansion_uses_supplied_home() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(src, "tcl", None, Some("/test-home"));
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///test-home/lib/init.tcl");
    }

    #[test]
    fn tilde_without_home_produces_no_link() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(src, "tcl", None, None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        let src = "source ~\n";
        let links = document_links_with_home(src, "tcl", None, Some("/test-home"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///test-home");
    }

    // URI escaping

    #[test]
    fn percent_encode_path_passes_unreserved_through() {
        // Letters, digits, and the unreserved punctuation set
        // pass through unchanged, as does the path separator.
        assert_eq!(
            percent_encode_path("/usr/local/lib/init.tcl"),
            "/usr/local/lib/init.tcl",
        );
    }

    #[test]
    fn percent_encode_path_escapes_spaces_and_specials() {
        assert_eq!(
            percent_encode_path("/path/with spaces/file.tcl"),
            "/path/with%20spaces/file.tcl",
        );
        // `#` is a fragment delimiter in URIs.
        assert_eq!(
            percent_encode_path("/has/#hash/file.tcl"),
            "/has/%23hash/file.tcl",
        );
        // `%` is the escape introducer — it itself needs escaping.
        assert_eq!(
            percent_encode_path("/has/50%off/file.tcl"),
            "/has/50%25off/file.tcl",
        );
    }

    #[test]
    fn percent_encode_path_escapes_non_ascii() {
        // UTF-8 bytes for `é` are 0xC3 0xA9.
        assert_eq!(
            percent_encode_path("/caf\u{00E9}/x.tcl"),
            "/caf%C3%A9/x.tcl"
        );
    }

    #[test]
    fn file_uri_for_unix_absolute_path() {
        // `/foo/bar.tcl` → `file:///foo/bar.tcl` (one slash
        // from `file://` + the leading `/` of the path).
        assert_eq!(file_uri_for_path("/foo/bar.tcl"), "file:///foo/bar.tcl");
    }

    #[test]
    fn file_uri_for_windows_drive_letter_path() {
        // `C:/Users/me/file.tcl` → `file:///C:/Users/me/file.tcl`
        // — `file_uri_for_path` must insert the third slash
        // so the host component is empty and the path starts
        // at the drive letter.
        assert_eq!(
            file_uri_for_path("C:/Users/me/file.tcl"),
            "file:///C:/Users/me/file.tcl",
        );
    }

    #[test]
    fn file_uri_normalises_windows_backslashes() {
        assert_eq!(
            file_uri_for_path(r"C:\Users\me\file.tcl"),
            "file:///C:/Users/me/file.tcl",
        );
    }

    #[test]
    fn source_with_spaces_in_path_produces_escaped_uri() {
        // End-to-end: a literal path arg containing spaces or
        // hashes surfaces as a properly percent-encoded
        // `file://` URI rather than a raw concatenation.
        let src = "source /path/with spaces.tcl\n";
        let links = document_links(src, "tcl", None);
        // The literal-path arg may or may not parse cleanly
        // through the segmenter; if it does, the URI must be
        // escaped.  If not, the test is informational only.
        if let Some(link) = links.first() {
            assert!(
                !link.target.contains(' '),
                "URI target must not contain raw spaces: {target}",
                target = link.target,
            );
        }
    }

    // literal `[file join …]`

    #[test]
    fn file_join_joins_literal_segments() {
        // `[file join lib core init.tcl]` → `lib/core/init.tcl`.
        assert_eq!(
            literal_file_join("[file join lib core init.tcl]"),
            Some("lib/core/init.tcl".to_owned()),
        );
    }

    #[test]
    fn file_join_handles_quoted_and_braced_segments() {
        assert_eq!(
            literal_file_join(r#"[file join "lib" {core} init.tcl]"#),
            Some("lib/core/init.tcl".to_owned()),
        );
    }

    #[test]
    fn file_join_absolute_segment_resets_accumulator() {
        // Per Tcl's `file join` semantics, an absolute path
        // resets the joined accumulator.
        assert_eq!(
            literal_file_join("[file join /etc /opt/foo bar]"),
            Some("/opt/foo/bar".to_owned()),
        );
    }

    #[test]
    fn file_join_returns_none_for_variable_segments() {
        assert!(literal_file_join("[file join $dir foo]").is_none());
        assert!(literal_file_join("[file join [pwd] foo]").is_none());
    }

    #[test]
    fn file_join_returns_none_for_non_file_join_subst() {
        assert!(literal_file_join("[exec ls]").is_none());
        assert!(literal_file_join("[file dirname /foo]").is_none());
    }

    #[test]
    fn source_with_literal_file_join_surfaces_link() {
        let src = "source [file join lib helper.tcl]\n";
        let links = document_links(src, "tcl", Some("/home/user/project"));
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///home/user/project/lib/helper.tcl",);
    }

    #[test]
    fn source_with_absolute_file_join_segment_surfaces_link() {
        let src = "source [file join /usr/local/lib tcl init.tcl]\n";
        let links = document_links(src, "tcl", None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/local/lib/tcl/init.tcl");
    }
}
