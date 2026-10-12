// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Namespace prefixes of Unicode analytical reporting QNames.

/// Selected geometry of a reporting QName's namespace presentation.
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

/// Select a reporting QName's literal namespace prefix, defaulting to `::`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_qname_namespaces_keep_purpose_and_literal_text() {
        // Implementation contract: naming.analysis.reported-qname-namespace-prefix
        // docs/design/analysis/name-resolution-proofs/reported-qname-namespace-prefix.md
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
