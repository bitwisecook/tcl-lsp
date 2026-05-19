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
//! * Tilde expansion (`~/path/to/file`) — needs the user's
//!   home directory, plumbed in by the server from
//!   workspace folders.
//! * `[file join ...]` / variable-interpolated paths — the
//!   minimal port resolves only literal path arguments.
//! * Workspace-folder enumeration that lets a `source` link
//!   resolve across multiple roots; the single
//!   `workspace_root` parameter is sufficient for the
//!   common single-root case.

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
        // Skip non-literal paths (variable substitution / command
        // substitution / multi-token).  `single_token_word[idx]`
        // is `false` for those.
        if let Some(&single) = seg.single_token_word.get(idx) {
            if !single {
                continue;
            }
        }
        let Some(target) = resolve_path(path, workspace_root, home) else {
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
}
