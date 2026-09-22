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

//! The `SslicTcl` loader's diagnostics, as ordinary document diagnostics.
//!
//! `tcl_sslictcl::dsl::load_with_diagnostics` is the authoring-grade entry
//! point to the `.sslictcl` vocabulary: it recovers past a bad declaration
//! and reports every problem it finds as a [`DslDiagnostic`] carrying a
//! published `SSLIC1xxx` code and a byte [`Span`] into the document. Those
//! are exactly the facts an editor squiggle needs, so this module is a
//! projection and nothing more — it maps one loader diagnostic to one
//! [`Finding`], the shape every producer hands the policy step, and lets
//! the adapters' span lift give it a UTF-16-correct range. It applies no
//! policy: the disabled set and the directives reach these codes through
//! [`crate::diagnostic_policy::apply`] like every other code.
//!
//! Nothing here names a declaration. The vocabulary lives in
//! `tcl_sslictcl::vocabulary` (for the loader) and in the `sslictcl` registry
//! pack (for the editor surfaces); a word added to either is visible here the
//! moment the loader reports on it.
//!
//! [`DslDiagnostic`]: tcl_sslictcl::dsl::DslDiagnostic
//! [`Span`]: tcl_lexer::Span

use tcl_core_types::{DiagCode, Severity};
use tcl_dialect::DialectProfile;
use tcl_sslictcl::dsl::{DslSeverity, load_with_diagnostics};

use crate::diagnostic_policy::{Finding, Producer};

/// The authoring surface package the `sslictcl` environment carries.
///
/// The routing key, read off the resolved environment rather than compared
/// against a dialect *name*: aliases (`sslic-tcl`, `tls-sslictcl`) are folded
/// in by resolution, exactly as the BIG-IP dispatch reads `bigip`.
const SURFACE_PACKAGE: &str = "sslictcl";

/// Whether documents of `dialect` are `SslicTcl` declarations — i.e. whether
/// [`diagnostics`] applies to them.
///
/// The resolved environment's authoring point, not a name test: a document
/// reaches the `sslictcl` environment through its extension, its editor
/// language id, an alias, or the `sslictcl VERSION` content signature, and all
/// four answer the same authoring surface.
#[must_use]
pub fn applies_to(dialect: &DialectProfile) -> bool {
    crate::document_context_for_profile(dialect)
        .authoring_query()
        .packages
        .contains(&SURFACE_PACKAGE)
}

/// Every loader diagnostic `source` produces, as findings.
///
/// Nothing is filtered: the caller decides *whether* to ask
/// ([`applies_to`]), and the policy step decides what shows. This function
/// assumes the document is a `.sslictcl` one and does not re-check.
#[must_use]
pub fn diagnostics(source: &str) -> Vec<Finding> {
    load_with_diagnostics(source)
        .diagnostics
        .into_iter()
        .map(|d| Finding {
            code: d.code,
            span: d.range,
            severity: match d.severity {
                DslSeverity::Error => Severity::Error,
                DslSeverity::Warning => Severity::Warning,
                DslSeverity::Hint => Severity::Hint,
            },
            message: d.message,
            fixes: Vec::new(),
            data: None,
            producer: Producer::SslicTcl,
        })
        .collect()
}

/// The analyser codes a `.sslictcl` document's loader supersedes.
///
/// W123 answers "is this word a command that exists?". The question needs a
/// word in the head position of a script that will be **evaluated**, and the
/// defining property of this environment is that its documents never are —
/// the loader walks the syntax tree and constructs no interpreter, so a
/// `.sslictcl` document contains declarations and no calls at all. The
/// unrecognised word the analyser sees is an unknown *declaration*, and the
/// loader already says so with far better information than an edit-distance
/// guess: `SSLIC1101` where an open block preserves it as a forwards-
/// compatibility extension, `SSLIC1007` where a closed block rejects it.
/// Publishing both puts two hints on one word that disagree about what it is,
/// and the W123 one carries a "did you mean …?" quick-fix that would rewrite
/// a deliberately-preserved extension into a declaration.
///
/// Every other analyser verdict stands: an arity error on a declared member
/// (`hostname a b c` → `E003`) is still an arity error, and the syntax and
/// style codes are unaffected.
///
/// This is a **dialect** policy — it names no declaration, and it holds for
/// every word in the document rather than a list of them. The policy step
/// reads it as the dialect's overlap entry
/// ([`crate::diagnostic_policy::dialect_overlaps`]): the loader owns each
/// code document-wide, whether or not it emitted a finding of its own.
pub const SUPERSEDED_ANALYSER_CODES: &[DiagCode] = &[DiagCode::W123];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_for_dialect;

    const THREE_ERRORS: &str = "sslictcl 1\n\
                                endpoint /Common/a {\n\
                                    hostname a.example.test\n\
                                    hsts {\n\
                                        enabled maybe\n\
                                        nonsense 1\n\
                                    }\n\
                                    chain missing-chain\n\
                                }\n";

    fn codes(source: &str) -> Vec<String> {
        diagnostics(source)
            .into_iter()
            .map(|d| d.code.as_str().to_owned())
            .collect()
    }

    #[test]
    fn the_sslictcl_environment_is_the_one_that_applies() {
        assert!(applies_to(profile_for_dialect("sslictcl")));
        assert!(applies_to(profile_for_dialect("sslic-tcl")));
        for other in ["tcl9.0", "tcl8.6", "spectcl", "f5-irules", "f5-bigip"] {
            assert!(!applies_to(profile_for_dialect(other)), "{other}");
        }
    }

    #[test]
    fn independent_errors_are_all_reported() {
        let codes = codes(THREE_ERRORS);
        for expected in ["SSLIC1007", "SSLIC1009", "SSLIC1011"] {
            assert!(
                codes.contains(&expected.to_owned()),
                "{expected}: {codes:?}"
            );
        }
    }

    #[test]
    fn every_finding_carries_the_loader_span_and_the_sslictcl_producer() {
        use crate::diagnostic_policy::{PolicyBuilder, PolicyLayer, Reason, apply};
        let findings = diagnostics(THREE_ERRORS);
        assert!(findings.iter().all(|f| f.producer == Producer::SslicTcl));
        assert!(
            findings
                .iter()
                .all(|f| f.fixes.is_empty() && f.data.is_none())
        );
        let span = findings
            .iter()
            .find(|f| f.code == DiagCode::Sslic1009)
            .expect("the out-of-domain value is reported")
            .span;
        assert_eq!(
            &THREE_ERRORS[span.start() as usize..span.end() as usize],
            "maybe"
        );
        // Nothing is filtered here; the policy step does that.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &serde_json::json!({ "diagnostics": { "SSLIC1009": false } }),
            )
            .build();
        let report = apply(findings, &policy);
        assert_eq!(
            report.reason_for(DiagCode::Sslic1009, span),
            Some(Reason::Disabled(PolicyLayer::Editor))
        );
        assert!(
            report
                .shown()
                .any(|s| s.finding.code == DiagCode::Sslic1007)
        );
    }

    #[test]
    fn severities_follow_the_loader() {
        let notice = "sslictcl 1\nunknown-declaration {a b}\n";
        let lifted = diagnostics(notice);
        let hint = lifted
            .iter()
            .find(|d| d.code.as_str() == "SSLIC1101")
            .expect("the unknown top-level declaration is a notice");
        assert_eq!(hint.severity, Severity::Hint);
    }
}
