//! File-operation helpers: rewrite `source FILE` references when a
//! sourced file is renamed.
//!
//! When a
//! `.tcl` file is renamed, every dependent file that `source`s it
//! (via a *literal* path) gets its path literal rewritten so the
//! workspace still loads.  `compute_rename_edits` is pure and
//! returns byte-span edits keyed by dependent URI; the server resolves
//! the spans to LSP ranges against each dependent's current text and
//! assembles the `WorkspaceEdit`.

use tcl_lexer::Span;

use crate::workspace_index::WorkspaceIndex;

/// One edit to a dependent document: replace `span` with `new_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEdit {
    /// Dependent document to edit.
    pub uri: String,
    /// Byte span of the `source` path literal to replace.
    pub span: Span,
    /// New path literal (style-preserving — relative stays relative).
    pub new_text: String,
}

/// Compute the `source`-rewrite edits for renaming `old_uri` → `new_uri`.
///
/// Scans every literal `source` target in the index, resolves it
/// against its own document's path (and each workspace root), and — when
/// it points at `old_uri`'s path — emits an edit replacing the literal
/// with the new path (preserving absolute-vs-relative style).
/// `workspace_roots` are `file://` URIs of the workspace folders, tried
/// for a relative literal that doesn't resolve next to its script.
/// Returns an empty vector when nothing references the renamed file.
/// Mirrors `compute_rename_edits` / `_collect_edits_for_rename`.
#[must_use]
pub fn compute_rename_edits(
    old_uri: &str,
    new_uri: &str,
    index: &WorkspaceIndex,
    workspace_roots: &[String],
) -> Vec<RenameEdit> {
    let (Some(old_raw), Some(new_raw)) = (uri_to_path(old_uri), uri_to_path(new_uri)) else {
        return Vec::new(); // non-file URIs are not renameable
    };
    let old_path = normpath(&old_raw);
    let new_path = normpath(&new_raw);
    // Workspace roots as filesystem paths (non-file URIs are dropped).
    let root_paths: Vec<String> = workspace_roots
        .iter()
        .filter_map(|r| uri_to_path(r))
        .collect();

    let mut edits = Vec::new();
    for src in index.sources() {
        if !src.is_literal {
            continue; // conservative: leave substitution paths alone
        }
        let Some(dep_path) = uri_to_path(&src.uri) else {
            continue;
        };
        if !source_resolves_to(&src.raw_path, &dep_path, &root_paths, &old_path) {
            continue;
        }
        let new_text = compute_new_literal(&src.raw_path, &dep_path, &new_path);
        // `src.range` is the path word's lexer span, which for a braced
        // (`{…}`) or quoted (`"…"`) literal covers the *opening*
        // delimiter + content but not the closing one (the inner-end
        // span convention).  Replacing it with the bare new path would
        // drop the opener and orphan the closer (`{old}` → `new}`), so
        // narrow the edit to the path content only — the range length
        // minus the (delimiter-free) `raw_path` length is the opening
        // delimiter width, and the closing delimiter sits past
        // `range.end()`.  Both delimiters are then preserved.
        edits.push(RenameEdit {
            uri: src.uri.clone(),
            span: content_span(src.range, &src.raw_path),
            new_text,
        });
    }
    edits
}

/// The span covering only the path *content* of a `source` literal,
/// given the word's full lexer span and its delimiter-free `raw_path`.
/// For a braced / quoted literal the leading delimiter width is
/// `range_len - raw_path_len`; the trailing delimiter lies past
/// `range.end()` and is left untouched, so a rewrite preserves both.
/// For a bare word the delimiter width is zero (the span is returned
/// unchanged).
fn content_span(range: Span, raw_path: &str) -> Span {
    let range_len = range.end().saturating_sub(range.start());
    let raw_len = u32::try_from(raw_path.len()).unwrap_or(range_len);
    let lead = range_len.saturating_sub(raw_len);
    Span::new(range.start() + lead, range.end())
}

/// Convert a `file://` URI to a filesystem path, or `None` for a
/// non-file URI (`untitled:`, `vscode-vfs:`, …).
fn uri_to_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    // `file:///abs/path` → `/abs/path`; drop a leading authority-less
    // `//` host segment (empty host) by keeping from the first `/`.
    let path = rest.strip_prefix("//").map_or(rest, |after_host| {
        after_host.find('/').map_or("/", |i| &after_host[i..])
    });
    Some(percent_decode(path))
}

/// Minimal percent-decoding for the characters a path realistically
/// carries (`%20` → space, etc.).  Unknown / malformed escapes pass
/// through verbatim.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// True when a literal `source` path resolves to `old_path` under any of
/// the candidate bases Python's `resolve_source_target` tries (for the
/// literal case): the script's own directory first, then each workspace
/// root; an absolute literal resolves only to itself.  The pure analog
/// of the filesystem `isfile` probes — a match against *any* candidate
/// counts, since the renamed file is known to exist at `old_path`.
fn source_resolves_to(raw: &str, dep_path: &str, roots: &[String], old_path: &str) -> bool {
    if raw.starts_with('/') {
        return normpath(raw) == old_path;
    }
    // Relative to the script's own directory.
    let dir = posix_dirname(dep_path);
    if normpath(&posix_join(dir, raw)) == old_path {
        return true;
    }
    // Relative to each workspace root.
    roots
        .iter()
        .any(|root| normpath(&posix_join(root, raw)) == old_path)
}

/// The new path literal preserving the existing style: an absolute
/// literal becomes the new absolute path; a relative one is re-relative
/// to the dependent's directory.
fn compute_new_literal(old_literal: &str, dep_path: &str, new_abs_path: &str) -> String {
    if old_literal.starts_with('/') {
        return new_abs_path.to_string();
    }
    relpath(new_abs_path, posix_dirname(dep_path))
}

// -- posix path helpers (the test environment is Linux) -------------

fn posix_dirname(p: &str) -> &str {
    match p.rfind('/') {
        Some(0) => "/",
        Some(i) => &p[..i],
        None => "",
    }
}

fn posix_join(dir: &str, rel: &str) -> String {
    if dir.is_empty() {
        rel.to_string()
    } else if dir.ends_with('/') {
        format!("{dir}{rel}")
    } else {
        format!("{dir}/{rel}")
    }
}

/// Collapse `.` / `..` / repeated slashes (posix `os.path.normpath`).
fn normpath(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let absolute = p.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for comp in p.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(&"..")) || (!absolute && out.is_empty()) {
                    out.push("..");
                } else {
                    out.pop();
                }
            }
            other => out.push(other),
        }
    }
    let joined = out.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// Relative path from `start` to `target` (posix `os.path.relpath`).
fn relpath(target: &str, start: &str) -> String {
    let target_norm = normpath(target);
    let start_norm = normpath(start);
    let t: Vec<&str> = target_norm.split('/').filter(|c| !c.is_empty()).collect();
    let s: Vec<&str> = start_norm.split('/').filter(|c| !c.is_empty()).collect();
    let common = t.iter().zip(&s).take_while(|(a, b)| a == b).count();
    let ups = s.len() - common;
    let mut parts: Vec<&str> = vec![".."; ups];
    parts.extend(&t[common..]);
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn index_of(uri: &str, src: &str) -> WorkspaceIndex {
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let mut idx = WorkspaceIndex::new();
        idx.add_document(uri, &analysis);
        idx
    }

    #[test]
    fn rewrites_relative_source_literal() {
        // /proj/main.tcl sources lib/old.tcl → rename to lib/new.tcl
        // rewrites the literal to `lib/new.tcl`.
        let idx = index_of("file:///proj/main.tcl", "source lib/old.tcl\n");
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].uri, "file:///proj/main.tcl");
        assert_eq!(edits[0].new_text, "lib/new.tcl");
    }

    #[test]
    fn braced_and_quoted_literals_rewrite_content_only() {
        // A braced `source {lib/old.tcl}` must rewrite only the path
        // content, leaving the `{` / `}` delimiters intact (else the
        // edit would orphan the closing brace).
        for src in [
            "source {lib/old.tcl}\n",
            "source \"lib/old.tcl\"\n",
            "source lib/old.tcl\n",
        ] {
            let idx = index_of("file:///proj/main.tcl", src);
            let edits = compute_rename_edits(
                "file:///proj/lib/old.tcl",
                "file:///proj/lib/new.tcl",
                &idx,
                &[],
            );
            assert_eq!(edits.len(), 1, "{src:?}");
            // The edit's span must cover exactly the old path content.
            let covered = &src[edits[0].span.as_range()];
            assert_eq!(covered, "lib/old.tcl", "span content for {src:?}");
            assert_eq!(edits[0].new_text, "lib/new.tcl", "{src:?}");
            // Applying the edit preserves any surrounding delimiters.
            let mut applied = src.to_string();
            applied.replace_range(edits[0].span.as_range(), &edits[0].new_text);
            let expected = src.replace("lib/old.tcl", "lib/new.tcl");
            assert_eq!(applied, expected, "rewrite for {src:?}");
        }
    }

    #[test]
    fn rewrites_absolute_source_literal_to_new_absolute() {
        let idx = index_of("file:///proj/main.tcl", "source /proj/lib/old.tcl\n");
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "/proj/lib/new.tcl");
    }

    #[test]
    fn ignores_unrelated_and_substituted_sources() {
        // A source of a different file, and a `$var`-substituted source,
        // are both left alone.
        let idx = index_of(
            "file:///proj/main.tcl",
            "source other.tcl\nsource $dynamic\n",
        );
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn resolves_relative_source_via_workspace_root() {
        // `/proj/sub/main.tcl` does `source helper.tcl`, but the file lives
        // at the workspace root `/proj/helper.tcl`, not next to the script.
        // Without a workspace root it isn't matched; with one it is, and
        // the rewrite re-relativises to the script's directory.
        let idx = index_of("file:///proj/sub/main.tcl", "source helper.tcl\n");
        let no_root = compute_rename_edits(
            "file:///proj/helper.tcl",
            "file:///proj/helper2.tcl",
            &idx,
            &[],
        );
        assert!(no_root.is_empty(), "no root → no match: {no_root:?}");

        let with_root = compute_rename_edits(
            "file:///proj/helper.tcl",
            "file:///proj/helper2.tcl",
            &idx,
            &["file:///proj".to_string()],
        );
        assert_eq!(with_root.len(), 1, "{with_root:?}");
        assert_eq!(with_root[0].new_text, "../helper2.tcl");
    }

    #[test]
    fn path_helpers() {
        assert_eq!(normpath("/a/b/../c"), "/a/c");
        assert_eq!(posix_dirname("/a/b/c.tcl"), "/a/b");
        assert_eq!(relpath("/proj/lib/new.tcl", "/proj"), "lib/new.tcl");
        assert_eq!(relpath("/proj/new.tcl", "/proj/sub"), "../new.tcl");
        assert_eq!(uri_to_path("file:///a/b.tcl").as_deref(), Some("/a/b.tcl"));
        assert_eq!(uri_to_path("untitled:foo"), None);
    }
}
