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

fn hazard(source: &str, analysis: &AnalysisResult) -> Option<RenameRefusal> {
    let family = ["::C".to_owned()];
    method_rename_hazard(
        source,
        tcl_dialect::DialectProfile::plain_tcl(),
        analysis,
        MethodRenameTarget {
            family: &family,
            method: "m",
            is_classmethod: false,
            new_name: "changed",
        },
        &LineIndex::new(source),
    )
}

const CLASSES: &str =
    "oo::class create C {method m {} {}}\noo::class create Other {method m {} {}}\n";

#[test]
fn actual_rename_hazard_keeps_a_compound_variable_receiver_untracked() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    for head in ["$v", "${v}", "\"${v}\""] {
        let source = format!("{CLASSES}set v [Other new]\n{head} m\n");
        let current = analyse(&source, tcl_lexer::LexerConfig::default());
        assert_eq!(
            current.instance_classes.get("v").map(String::as_str),
            Some("::Other")
        );
        assert!(hazard(&source, &current).is_none(), "{head}");
    }
    for head in ["${v}suffix", "${v}[list suffix]", "$arr(k)", "${missing}"] {
        let source = format!("{CLASSES}set v [Other new]\n{head} m\n");
        let current = analyse(&source, tcl_lexer::LexerConfig::default());
        assert!(
            hazard(&source, &current)
                .unwrap()
                .reason
                .contains("not tracked"),
            "{head}"
        );
    }
}

#[test]
fn actual_rename_receiver_names_keep_first_close_and_nested_source_currency() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = format!("{CLASSES}set {{a{{b}}c}} [Other new]\n${{a{{b}}c}} m\n");
    let mut config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::default()
    };
    let nested = analyse(&source, config);
    assert!(hazard(&source, &nested).is_none());
    config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
    let first = analyse(&source, config);
    assert!(
        hazard(&source, &first)
            .unwrap()
            .reason
            .contains("not tracked")
    );
    let valid_first_close = format!("{CLASSES}set \"a{{b\" [Other new]\n${{a{{b}} m\n");
    let first = analyse(&valid_first_close, config);
    assert!(hazard(&valid_first_close, &first).is_none());
    let foreign = source.clone() + "puts foreign\n";
    assert!(
        hazard(&foreign, &nested)
            .unwrap()
            .reason
            .contains("original source")
    );
    let mut missing = nested;
    missing.resolved_input = None;
    assert!(
        hazard(&source, &missing)
            .unwrap()
            .reason
            .contains("original source")
    );
}
