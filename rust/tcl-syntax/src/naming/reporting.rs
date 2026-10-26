// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Literal namespace geometry for Unicode analytical reporting keys.

/// Selected geometry of a reporting `QName`'s namespace presentation.
/// Each purpose supplies metadata only, independently of retained source or
/// execution namespace identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportedQNameNamespacePurpose {
    /// Search beyond one leading root marker.
    AfterLeadingRoot,
    /// Root a relative namespace prefix.
    RootRelativePrefix,
    /// Retain the literal namespace prefix.
    LiteralPrefix,
}

/// Select a reporting `QName`'s literal namespace prefix, defaulting to `::`.
/// The last literal `::` pair supplies the split under the selected purpose;
/// overlapping colon pairs and all other Unicode/NUL text remain intact.
/// This query supplies no written-name grammar, constructed-key partition,
/// Native component path, source spelling or lookup authority.
#[must_use]
pub fn reported_qname_namespace(qname: &str, purpose: ReportedQNameNamespacePurpose) -> String {
    let search = match purpose {
        ReportedQNameNamespacePurpose::AfterLeadingRoot => {
            qname.strip_prefix("::").unwrap_or(qname)
        }
        ReportedQNameNamespacePurpose::RootRelativePrefix
        | ReportedQNameNamespacePurpose::LiteralPrefix => qname,
    };
    let Some(index) = search.rfind("::") else {
        return "::".to_owned();
    };
    let prefix = &qname[..qname.len() - search.len() + index];
    if prefix.is_empty() {
        "::".to_owned()
    } else if purpose == ReportedQNameNamespacePurpose::RootRelativePrefix
        && !prefix.starts_with("::")
    {
        format!("::{prefix}")
    } else {
        prefix.to_owned()
    }
}

/// Compare analytical namespace report keys for equality or containment.
/// The optional root marker is removed once for equality; strict containment
/// keeps the original keys for its independent literal-prefix projection.
/// This supplies no namespace component identity, lookup or editable source.
#[must_use]
pub fn reported_namespace_contains(parent: &str, child: &str) -> bool {
    super::unroot_rooted_key(parent).unwrap_or(parent)
        == super::unroot_rooted_key(child).unwrap_or(child)
        || reported_namespace_strictly_contains(parent, child)
}

/// Compare analytical namespace report keys for a strict literal prefix.
/// One leading root marker is removed from either key. The root contains every
/// non-root report key; other prefixes require a following `::` boundary.
/// Literal colons, trailing separators and all other bytes remain intact.
/// This supplies no namespace component identity, lookup or editable source.
#[must_use]
pub fn reported_namespace_strictly_contains(parent: &str, child: &str) -> bool {
    let parent = super::unroot_rooted_key(parent).unwrap_or(parent);
    let child = super::unroot_rooted_key(child).unwrap_or(child);
    if child.is_empty() {
        return false;
    }
    parent.is_empty()
        || child
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_reported_containment_roots_each_original_key_once() {
        for (parent, child, contains) in [
            (":::", ":::", true),
            (":::", "::::::", true),
            (":::", "::", false),
            (":::", "::other", false),
            ("", "::", true),
            ("::", "", true),
            ("p", "::p", true),
            ("p", "::pq", false),
        ] {
            assert_eq!(reported_namespace_contains(parent, child), contains);
        }
    }

    #[test]
    fn reported_namespace_containment_preserves_literal_key_boundaries() {
        for (parent, child, contains) in [
            ("::p", "::p::q", true),
            ("p", "::p::q::r", true),
            ("::p", "::pq::r", false),
            ("::p", "::p", false),
            ("", "::p", true),
            ("::", ":::", true),
            (":::", "::", false),
            ("::", "", false),
            ("::p::", "::p::q", false),
            ("::p::", "::p::::q", true),
            ("::p:", "::p:::q", true),
            ("::é", "::é::λ", true),
            ("::p\0", "::p\0::q", true),
        ] {
            assert_eq!(
                reported_namespace_strictly_contains(parent, child),
                contains
            );
        }
    }

    #[test]
    fn reported_qname_namespaces_keep_purpose_and_literal_text() {

        // Shared analytical name contract:
        // docs/design/contracts/shared-utility-contracts-rust.md (names / namespaces).
        // Fixed metadata projections only; no Native namespace or lookup assertion.
        for (qname, procedure, uplevel, constructor) in [
            ("", "::", "::", "::"),
            ("p", "::", "::", "::"),
            ("::", "::", "::", "::"),
            ("::p", "::", "::", "::"),
            ("n::p", "n", "::n", "n"),
            ("::n::p", "::n", "::n", "::n"),
            (":::p", "::", ":::", ":"),
            (":::", "::", ":::", ":"),
            ("::::p", "::", "::", "::"),
            ("::::::p", "::::", "::::", "::::"),
            ("::a:::p", "::a:", "::a:", "::a:"),
            ("n::", "n", "::n", "n"),
            ("::n::", "::n", "::n", "::n"),
            ("é::λ", "é", "::é", "é"),
            ("::é::λ", "::é", "::é", "::é"),
            ("n\0::p", "n\0", "::n\0", "n\0"),
        ] {
            for (purpose, expected) in [
                (ReportedQNameNamespacePurpose::AfterLeadingRoot, procedure),
                (ReportedQNameNamespacePurpose::RootRelativePrefix, uplevel),
                (ReportedQNameNamespacePurpose::LiteralPrefix, constructor),
            ] {
                assert_eq!(reported_qname_namespace(qname, purpose), expected);
            }
        }
    }
}
