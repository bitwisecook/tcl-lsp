//! Token-bounded source rewriter — the **rename** half of the BIG-IP source
//! rewriter (`rename_object` + its helpers `_build_name_pattern` /
//! `_other_kind_header_spans`, and [`RenameReport`]).
//!
//! `rename_object` rewrites every token-bounded occurrence of an object's
//! full-path — the stanza header, every property-value reference, and iRule
//! body command-argument literals — but leaves the *header* of any other-kind
//! stanza that happens to share the same full-path untouched (the `kind_scope`
//! guard). Everything is text-level: no parser round-trip, so comments,
//! whitespace, key order, and unknown stanzas all survive.
//!
//! The reference pattern uses look-behind / look-ahead assertions
//! (`(?<![A-Za-z0-9_/.\-])…(?![A-Za-z0-9_/.\-])`) that the `regex` crate
//! cannot express; the token-boundary match is reproduced here with a manual
//! byte scan so the occurrence count and rewritten text stay byte-identical to
//! the reference. The other-kind header scan keeps the `regex` crate (its
//! pattern has no look-around).
//!
//! Secret redaction (`redact_secrets` / `RedactReport`) is a different verb
//! and is intentionally **not** part of this module.

use regex::Regex;

/// Result of a [`rename_object`] call.
///
/// Carries `old`, `new`, `occurrences`, plus the rewritten text the applier
/// splices back in.
#[derive(Debug, Clone)]
pub struct RenameReport {
    pub old: String,
    pub new: String,
    /// Total token-bounded replacements made.
    pub occurrences: usize,
    pub new_source: String,
}

/// Whether `b` is a BIG-IP identifier character — the class the
/// look-behind / look-ahead bounds the match with: `[A-Za-z0-9_/.\-]`.
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'.' | b'-')
}

/// Find every token-bounded occurrence of `old` in `source`.
///
/// A match at byte offset `i` is token-bounded when the byte before it (if
/// any) and the byte after the match (if any) are both **not** identifier
/// characters. Matches do not overlap (the scan runs left-to-right,
/// resuming past each match), which matters when `old` could otherwise overlap
/// itself; BIG-IP full-paths don't self-overlap, but the non-overlapping scan
/// is reproduced for fidelity.
fn find_name_matches(source: &str, old: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let needle = old.as_bytes();
    let n = bytes.len();
    let m = needle.len();
    let mut spans = Vec::new();
    if m == 0 || m > n {
        return spans;
    }
    let mut i = 0usize;
    while i + m <= n {
        if &bytes[i..i + m] == needle {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_ok = i + m == n || !is_ident_byte(bytes[i + m]);
            if before_ok && after_ok {
                spans.push((i, i + m));
                i += m;
                continue;
            }
        }
        i += 1;
    }
    spans
}

/// Byte-spans of the path tokens in *other-kind* stanza headers that share the
/// full-path `old`.
///
/// A rename scoped to one kind must leave the header of any other-kind stanza
/// that happens to share `old` untouched. The header form is
/// `<module> <kind…> <full-path> {` on a single line; the kind segment may be
/// multi-word, so the regex consumes everything between `<module>` and the
/// trailing `<full-path> {` as the kind, then joins module + kind to compare
/// against `kind_scope`. A header whose kind equals `kind_scope` — or is a
/// TMSH sub-kind of it (`ltm profile tcp` under `ltm profile`) — is *not*
/// protected, so its name rewrites along with the references.
fn other_kind_header_spans(source: &str, old: &str, kind_scope: &str) -> Vec<(usize, usize)> {
    // `^[ \t]*(\w+)[ \t]+([^\n]+?)[ \t]+(<escaped old>)[ \t]*\{` (MULTILINE).
    let pattern = format!(
        r"(?m)^[ \t]*(\w+)[ \t]+([^\n]+?)[ \t]+({})[ \t]*\{{",
        regex::escape(old)
    );
    let header_re = Regex::new(&pattern).expect("valid header regex");
    let mut spans = Vec::new();
    for caps in header_re.captures_iter(source) {
        let module = caps.get(1).unwrap().as_str();
        let kind_mid = caps.get(2).unwrap().as_str().trim();
        let header_kind = format!("{module} {kind_mid}");
        if header_kind == kind_scope {
            continue;
        }
        // Family match — a `kind_scope` of `ltm profile` collapses sub-kinds
        // (`ltm profile tcp`, …) into one navigable kind; treat the header as
        // the same kind so it rewrites along with the references.
        if header_kind.starts_with(&format!("{kind_scope} ")) {
            continue;
        }
        let path = caps.get(3).unwrap();
        spans.push((path.start(), path.end()));
    }
    spans
}

/// Rename `old` to `new` everywhere in `source` (text-level).
///
/// Only token-bounded full-path references are rewritten, so substring
/// collisions don't fire (`/Common/foo` won't match `/Common/foobar`). When
/// `kind_scope` is non-empty, the *header* of any other-kind stanza that
/// shares `old` is left untouched; references everywhere else (this kind's
/// body, iRule bodies, unrelated stanza bodies) are still rewritten because
/// BIG-IP's reference graph is name-only. An empty `kind_scope` is the legacy
/// global rewrite used by the `rename()` builtin / `f5 rename`.
///
/// # Errors
/// Returns an error message when `old` or `new` is empty, matching the
/// reference `ValueError`.
pub fn rename_object(
    source: &str,
    old: &str,
    new: &str,
    kind_scope: &str,
) -> Result<RenameReport, String> {
    if old.is_empty() {
        return Err("old name is empty".to_owned());
    }
    if new.is_empty() {
        return Err("new name is empty".to_owned());
    }

    let protected = if kind_scope.is_empty() {
        Vec::new()
    } else {
        other_kind_header_spans(source, old, kind_scope)
    };

    let matches = find_name_matches(source, old);
    let mut out = String::with_capacity(source.len());
    let mut count = 0usize;
    let mut cursor = 0usize;
    for (start, end) in matches {
        let in_protected = protected
            .iter()
            .any(|&(p_start, p_end)| p_start <= start && start < p_end);
        out.push_str(&source[cursor..start]);
        if in_protected {
            // Leave the other-kind header's path token verbatim.
            out.push_str(&source[start..end]);
        } else {
            out.push_str(new);
            count += 1;
        }
        cursor = end;
    }
    out.push_str(&source[cursor..]);

    // The reference re-parses the rewritten SCF as a sanity check and raises on
    // failure; the Rust `parse_bigip_conf` is infallible, so the field-edit
    // path likewise omits the reparse. Nothing to validate here.

    Ok(RenameReport {
        old: old.to_owned(),
        new: new.to_owned(),
        occurrences: count,
        new_source: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bounded_does_not_touch_longer_name() {
        let src = "ltm pool /Common/foo {\n}\nltm pool /Common/foobar {\n}\n";
        let r = rename_object(src, "/Common/foo", "/Common/baz", "").unwrap();
        assert_eq!(r.occurrences, 1);
        assert!(r.new_source.contains("/Common/baz {"));
        assert!(r.new_source.contains("/Common/foobar {"));
    }

    #[test]
    fn rewrites_header_and_references() {
        let src = "ltm pool /Common/p {\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";
        let r = rename_object(src, "/Common/p", "/Common/q", "").unwrap();
        assert_eq!(r.occurrences, 2);
        assert!(r.new_source.contains("ltm pool /Common/q {"));
        assert!(r.new_source.contains("pool /Common/q"));
    }

    #[test]
    fn kind_scope_protects_other_kind_header() {
        // A pool and a virtual share `/Common/shared`; scope to the pool.
        let src = "ltm pool /Common/shared {\n}\nltm virtual /Common/shared {\n    pool /Common/shared\n}\n";
        let r = rename_object(src, "/Common/shared", "/Common/renamed", "ltm pool").unwrap();
        // The pool header + the body reference rewrite; the virtual header
        // stays (other kind).
        assert!(r.new_source.contains("ltm pool /Common/renamed {"));
        assert!(r.new_source.contains("ltm virtual /Common/shared {"));
        assert!(r.new_source.contains("pool /Common/renamed"));
        assert_eq!(r.occurrences, 2);
    }

    #[test]
    fn empty_names_error() {
        assert!(rename_object("x", "", "y", "").is_err());
        assert!(rename_object("x", "y", "", "").is_err());
    }
}
