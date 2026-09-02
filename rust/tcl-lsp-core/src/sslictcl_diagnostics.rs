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
//! [`analyser::Diagnostic`], the type every other whole-file finding in this
//! crate already speaks, and lets the server's existing span lift give it a
//! UTF-16-correct range.
//!
//! Nothing here names a declaration. The vocabulary lives in
//! `tcl_sslictcl::vocabulary` (for the loader) and in the `sslictcl` registry
//! pack (for the editor surfaces); a word added to either is visible here the
//! moment the loader reports on it.
//!
//! [`DslDiagnostic`]: tcl_sslictcl::dsl::DslDiagnostic
//! [`Span`]: tcl_lexer::Span
//! [`analyser::Diagnostic`]: tcl_compiler::analyser::Diagnostic

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use tcl_compiler::analyser::{Diagnostic, Severity};
use tcl_dialect::DialectProfile;
use tcl_sslictcl::dsl::{DslSeverity, load_with_diagnostics};

/// The authoring surface package the `sslictcl` environment carries.
///
/// The routing key, read off the resolved environment rather than compared
/// against a dialect *name*: aliases (`sslic-tcl`, `tls-sslictcl`) are folded
/// in by resolution, exactly as the BIG-IP dispatch reads `bigip`.
const SURFACE_PACKAGE: &str = "sslictcl";

/// Sentinel line for a file-wide `# tcl-lsp: disable=…` directive in the
/// analyser's `suppressed_lines` map.
const FILE_SUPPRESS_KEY: i32 = -1;

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

/// Every loader diagnostic `source` produces, as analyser diagnostics.
///
/// `disabled` is the resolved `tclLsp.diagnostics.<CODE> = false` set and
/// `suppressed` the analyser's `# noqa` / `# tcl-lsp: disable=…` map, applied
/// with the same contract every other code obeys: a `"*"` entry suppresses
/// everything, and the file-level bucket applies document-wide.
///
/// The caller decides *whether* to ask ([`applies_to`]); this function assumes
/// the document is a `.sslictcl` one and does not re-check.
#[must_use]
pub fn diagnostics<H: BuildHasher, I: BuildHasher, J: BuildHasher>(
    source: &str,
    disabled: &HashSet<String, J>,
    suppressed: &HashMap<i32, HashSet<String, I>, H>,
) -> Vec<Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new_lsp(source);
    load_with_diagnostics(source)
        .diagnostics
        .into_iter()
        .filter(|d| !disabled.contains(d.code.as_str()))
        .filter(|d| {
            let line = i32::try_from(line_index.position_at_utf16(d.range.start(), source).line)
                .unwrap_or(i32::MAX);
            !is_suppressed(d.code.as_str(), line, suppressed)
        })
        .map(|d| {
            Diagnostic::new(
                d.code,
                d.range,
                d.message,
                match d.severity {
                    DslSeverity::Error => Severity::Error,
                    DslSeverity::Warning => Severity::Warning,
                    DslSeverity::Hint => Severity::Hint,
                },
            )
        })
        .collect()
}

/// The shared `# noqa` / file-directive suppression contract.
fn is_suppressed<H: BuildHasher, I: BuildHasher>(
    code: &str,
    line: i32,
    suppressed: &HashMap<i32, HashSet<String, I>, H>,
) -> bool {
    let hit = |key: i32| {
        suppressed
            .get(&key)
            .is_some_and(|codes| codes.contains("*") || codes.contains(code))
    };
    hit(FILE_SUPPRESS_KEY) || hit(line)
}

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
        diagnostics(
            source,
            &HashSet::<String, std::hash::RandomState>::new(),
            &HashMap::<i32, HashSet<String>, std::hash::RandomState>::new(),
        )
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
            assert!(codes.contains(&expected.to_owned()), "{expected}: {codes:?}");
        }
    }

    #[test]
    fn a_disabled_code_is_dropped_and_the_others_kept() {
        let disabled: HashSet<String> = ["SSLIC1009".to_owned()].into_iter().collect();
        let kept: Vec<String> = diagnostics(
            THREE_ERRORS,
            &disabled,
            &HashMap::<i32, HashSet<String>, std::hash::RandomState>::new(),
        )
        .into_iter()
        .map(|d| d.code.as_str().to_owned())
        .collect();
        assert!(!kept.contains(&"SSLIC1009".to_owned()), "{kept:?}");
        assert!(kept.contains(&"SSLIC1007".to_owned()), "{kept:?}");
    }

    #[test]
    fn a_file_directive_suppresses_document_wide() {
        let mut suppressed: HashMap<i32, HashSet<String>> = HashMap::new();
        suppressed.insert(FILE_SUPPRESS_KEY, ["*".to_owned()].into_iter().collect());
        assert!(
            diagnostics(
                THREE_ERRORS,
                &HashSet::<String, std::hash::RandomState>::new(),
                &suppressed,
            )
            .is_empty()
        );
    }

    #[test]
    fn severities_follow_the_loader() {
        let notice = "sslictcl 1\nunknown-declaration {a b}\n";
        let lifted = diagnostics(
            notice,
            &HashSet::<String, std::hash::RandomState>::new(),
            &HashMap::<i32, HashSet<String>, std::hash::RandomState>::new(),
        );
        let hint = lifted
            .iter()
            .find(|d| d.code.as_str() == "SSLIC1101")
            .expect("the unknown top-level declaration is a notice");
        assert_eq!(hint.severity, Severity::Hint);
    }
}
