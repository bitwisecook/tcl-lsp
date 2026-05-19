//! Document-links provider — Rust port of
//! `lsp/features/document_links.py`.
//!
//! Detects `source <path>` invocations in the document and
//! surfaces each path argument as a clickable link.  When a
//! `workspace_root` is provided, relative paths resolve
//! against it; absolute paths surface as-is.
//!
//! What is *deferred*:
//!
//! * `package require <pkg>` resolution — needs a package
//!   index (Tcl's `auto_path` / pkgIndex.tcl scan).  Lands
//!   alongside the workspace-init chunk that builds that
//!   index.
//! * Variable-interpolated paths (`source [file join $dir
//!   init.tcl]`) — requires resolving the variable's value,
//!   which the workspace-index follow-up will surface from
//!   `RULE_INIT` / global `set` calls.
//! * Workspace-folder enumeration that lets a `source` link
//!   resolve across multiple roots; the single
//!   `workspace_root` parameter is sufficient for the
//!   common single-root case.
//!
//! What landed:
//!
//! * Tilde expansion (`~/path/to/file`) — via the
//!   `home` argument plumbed in by the server from the env.
//! * Literal `[file join a b c]` — recognised when every
//!   sub-arg is a simple bareword / quoted string; the joined
//!   path resolves against `workspace_root` like any other
//!   relative arg.

use tcl_compiler::segmenter::segment_commands;
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
    /// Target URI string.
    pub target: String,
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
pub fn document_links(source: &str, workspace_root: Option<&str>) -> Vec<DocumentLink> {
    let home = std::env::var("HOME").ok();
    document_links_with_home(source, workspace_root, home.as_deref())
}

/// Same as [`document_links`] but lets callers pass an
/// explicit `home` directory string (for testability under
/// `#![forbid(unsafe_code)]`, where we can't mutate
/// `std::env`).  Production callers should use the
/// zero-argument [`document_links`].
#[must_use]
pub fn document_links_with_home(
    source: &str,
    workspace_root: Option<&str>,
    home: Option<&str>,
) -> Vec<DocumentLink> {
    let line_index = LineIndex::new(source);
    let mut links = Vec::new();

    for seg in segment_commands(source) {
        if seg.texts.is_empty() {
            continue;
        }
        if seg.texts[0] != "source" {
            continue;
        }
        // `source` may take optional flags (`-encoding NAME`)
        // before the path argument; locate the first non-flag
        // arg.  Python's resolver does the same.
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
        let path_owned = if let Some(joined) = literal_file_join(path.as_str()) {
            joined
        } else {
            // Skip non-literal paths (variable substitution /
            // command substitution / multi-token).  The
            // `single_token_word[idx]` is `false` for those.
            if let Some(&single) = seg.single_token_word.get(idx) {
                if !single {
                    continue;
                }
            }
            path.clone()
        };
        let Some(target) = resolve_path(&path_owned, workspace_root, home) else {
            continue;
        };
        let arg_tok = seg.argv.get(idx);
        let Some(arg_tok) = arg_tok else { continue };
        let start = line_index.position_at(arg_tok.span.start());
        let end = line_index.position_at(arg_tok.span.end());
        links.push(DocumentLink {
            start_line: start.line,
            start_character: start.character,
            end_line: end.line,
            end_character: end.character,
            target,
        });
    }

    links
}

/// Try to interpret `arg` as a literal `[file join …]`
/// command substitution and return the joined path.  Returns
/// `None` if the shape doesn't match — typically because the
/// argument is a different command, or because one of the
/// sub-arguments contains a substitution that we can't resolve
/// statically.
///
/// Mirrors the literal-only branch of Python's `_resolve_path`
/// logic that recognises `[file join …]` source-arg expressions.
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
    Some(format!("file://{resolved}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_links_for_non_source_commands() {
        assert!(document_links("set x 1\n", None).is_empty());
        assert!(document_links("puts hello\n", None).is_empty());
    }

    #[test]
    fn absolute_path_surfaces_as_link() {
        let src = "source /usr/lib/tcl/init.tcl\n";
        let links = document_links(src, None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/lib/tcl/init.tcl");
    }

    #[test]
    fn relative_path_resolves_against_workspace_root() {
        let src = "source helper.tcl\n";
        let links = document_links(src, Some("/home/user/project"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/project/helper.tcl");
    }

    #[test]
    fn relative_path_without_root_produces_no_link() {
        let src = "source helper.tcl\n";
        let links = document_links(src, None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn encoding_flag_skipped_before_path() {
        let src = "source -encoding utf-8 /tmp/foo.tcl\n";
        let links = document_links(src, None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///tmp/foo.tcl");
    }

    #[test]
    fn link_range_anchors_at_path_argument() {
        // `source ` is 7 chars; the path starts at col 7.
        let src = "source /tmp/foo.tcl\n";
        let links = document_links(src, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].start_character, 7);
        // End col covers the path (12 chars: `/tmp/foo.tcl`).
        assert_eq!(links[0].end_character, 19);
    }

    #[test]
    fn dynamic_path_produces_no_link() {
        // `source $somevar` — variable substitution, not a
        // literal path.  Skipped.
        let src = "source $somevar\n";
        let links = document_links(src, None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn double_dash_terminator_skipped() {
        let src = "source -- /tmp/x.tcl\n";
        let links = document_links(src, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///tmp/x.tcl");
    }

    #[test]
    fn trailing_slash_on_workspace_root_handled() {
        let src = "source helper.tcl\n";
        let links = document_links(src, Some("/home/user/"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/helper.tcl");
    }

    #[test]
    fn tilde_expansion_uses_supplied_home() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(src, None, Some("/test-home"));
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///test-home/lib/init.tcl");
    }

    #[test]
    fn tilde_without_home_produces_no_link() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(src, None, None);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        let src = "source ~\n";
        let links = document_links_with_home(src, None, Some("/test-home"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///test-home");
    }

    // -- S-document-links-rich: literal `[file join …]` --------------

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
        let links = document_links(src, Some("/home/user/project"));
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(
            links[0].target,
            "file:///home/user/project/lib/helper.tcl",
        );
    }

    #[test]
    fn source_with_absolute_file_join_segment_surfaces_link() {
        let src = "source [file join /usr/local/lib tcl init.tcl]\n";
        let links = document_links(src, None);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/local/lib/tcl/init.tcl");
    }
}
