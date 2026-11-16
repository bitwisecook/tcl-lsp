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

use super::*;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

fn analyse(source: &str, config: tcl_lexer::LexerConfig) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(std::ptr::eq(
        context.as_ref(),
        tcl_registry::model::ingress::static_context_for("tcl9.1"),
    ));
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

fn source(name: &str, operand: &str) -> String {
    format!(
        "oo::class create C {{\n    method tick {{}} {{}}\n    method setup {{}} {{\n        set {{{name}}} [namespace code [list my tick]]\n        after 0 {operand}\n    }}\n}}\n"
    )
}

fn has_stored_target(source: &str, analysis: &AnalysisResult) -> bool {
    references(
        source,
        tcl_dialect::DialectProfile::plain_tcl(),
        1,
        11,
        analysis,
        true,
    )
    .iter()
    .any(|range| range.start_line == 3)
}

#[test]
fn actual_stored_callback_reference_uses_its_retained_brace_configuration() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = source("a{b}c", "${a{b}c}");
    let mut config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::default()
    };
    let nested = analyse(&source, config);
    assert!(has_stored_target(&source, &nested));
    config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
    let first = analyse(&source, config);
    assert!(!has_stored_target(&source, &first));
    assert!(!has_stored_target(
        &(source.clone() + "puts foreign\n"),
        &nested
    ));
    let mut changed = analyse(&source, nested.body_lexer_config.unwrap());
    let config = changed.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    assert!(!has_stored_target(&source, &changed));
}

#[test]
fn actual_callback_references_do_not_borrow_a_prefix_or_array_variable_name() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    for operand in ["${cbé}", "\"${cbé}\""] {
        let source = source("cbé", operand);
        assert!(
            has_stored_target(
                &source,
                &analyse(&source, tcl_lexer::LexerConfig::default())
            ),
            "{operand}"
        );
    }
    for operand in [
        "${cbé}suffix",
        "${cbé}[list suffix]",
        r"\$cbé",
        "${cbé(extra)}",
    ] {
        let source = source("cbé", operand);
        assert!(
            !has_stored_target(
                &source,
                &analyse(&source, tcl_lexer::LexerConfig::default())
            ),
            "{operand}"
        );
    }
}
