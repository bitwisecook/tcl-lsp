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

use crate::analyser::Analyser;
use tcl_core_types::DiagCode;

#[test]
fn original_tcloo_formals_follow_counted_lifecycle_body_applicability() {
    // naming.tcloo.empty-lifecycle-definition
    // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
    // Original ASCII source-script contract; no entered frame is asserted.
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, expected) in [
            ("oo::class create C {constructor {{a b c}} {}}", 0),
            ("oo::class create C {constructor {{a b c}} { }}", 1),
            ("oo::class create C {constructor {{a b c}} {# body}}", 1),
            ("oo::class create C {constructor {\"a} {}}", 0),
            ("oo::class create C {constructor {\"a} { }}", 1),
            (
                "oo::class create C {}; oo::define C constructor {{a b c}} {}",
                0,
            ),
            (
                "oo::class create C {}; oo::define C constructor {{a b c}} { }",
                1,
            ),
            (
                "oo::class create P {constructor {a b} { }}; oo::class create C {superclass P; constructor {{a b c}} {}}",
                0,
            ),
            ("oo::class create C {method m {{a b c}} {}}", 1),
        ] {
            let analysis = Analyser::new().analyse(source, dialect);
            let diagnostics = analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::E006)
                .collect::<Vec<_>>();
            assert_eq!(
                diagnostics.len(),
                expected,
                "{dialect}: {source}: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn original_tcloo_formal_applicability_preserves_dynamic_body_uncertainty() {
    // naming.tcloo.empty-lifecycle-definition
    // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
    // Dynamic source bodies are not evaluated or promoted to empty values.
    for source in [
        "oo::class create C {constructor {{a b c}} $body}",
        "oo::class create C {constructor {{a b c}} [body]}",
        "oo::class create C {}; oo::define C constructor {{a b c}} $body",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            !analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E006),
            "{source}"
        );
    }
}
