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

use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};

fn analyser() -> Analyser {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let environment = tcl_registry::model::ingress::resolve_environment("jimtcl");
    let profile = environment.analyser_profile();
    let context = Arc::new(
        environment
            .default_context_registry()
            .with_command_store(Arc::new(tcl_registry::CommandRegistry::build_default())),
    );
    Analyser::new().with_resolved_input(ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    ))
}

fn fixture(
    source: &str,
) -> (
    AnalysisResult,
    Vec<tcl_compiler::segmenter::SegmentedCommand>,
) {
    let result = analyser().analyse(source, "jimtcl");
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        result.resolved_input.as_ref().unwrap().lexer_config(),
    );
    (result, commands)
}

#[test]
fn original_jim_source_layout_keeps_statics_and_actual_body_in_shared_registry() {
    let source = "proc counter {} {{n 0}} {not_a_command_anywhere $n}; counter";
    let (analysis, commands) = fixture(source);
    let procedure = analysis
        .all_procs
        .values()
        .find(|p| p.name == "counter")
        .unwrap();
    assert_eq!(procedure.body_span, commands[0].arg_tokens()[3].span);
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|call| call.name == "not_a_command_anywhere")
    );
}

#[test]
fn original_procedure_source_layout_refuses_foreign_values_tokens_and_expansion() {
    let mut analyser = analyser();
    for (source, selected, foreign) in [
        (
            "proc counter {} {{n 0}} {original_body $n}",
            "original_body",
            "foreign_body",
        ),
        (
            "# another source\nproc counter {} {{n 0}} {foreign_body $n}",
            "foreign_body",
            "original_body",
        ),
        (
            "proc counter {} {{n 0}} {original_body $n}",
            "original_body",
            "foreign_body",
        ),
    ] {
        let result = analyser.analyse(source, "jimtcl");
        let procedure = result
            .all_procs
            .values()
            .find(|p| p.name == "counter")
            .unwrap();
        assert!(
            source[tcl_lexer::word_span_at(source, procedure.body_span).as_range()]
                .contains(selected)
        );
        assert!(
            result
                .command_invocations
                .iter()
                .any(|call| call.name == selected)
        );
        assert!(
            result
                .command_invocations
                .iter()
                .all(|call| call.name != foreign)
        );
    }
    let result = analyser.analyse("proc counter {} {*}$body", "jimtcl");
    assert!(result.all_procs.values().all(|p| p.name != "counter"));
}

#[test]
fn original_procedure_source_layout_keeps_plain_and_rooted_jim_forms() {
    let source = "foreach name {a b} {proc $name {} {{n 0}} {not_a_command_anywhere $n}}";
    let (analysis, _) = fixture(source);
    let start = source.find("{not_a_command_anywhere").unwrap();
    let end = start + "{not_a_command_anywhere $n}".len();
    let expected = tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
    for name in ["a", "b"] {
        let procedure = analysis
            .all_procs
            .values()
            .find(|p| p.name == name)
            .unwrap();
        assert_eq!(procedure.body_span, expected);
    }
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|call| call.name == "not_a_command_anywhere"
                && call.range.start() > expected.start()
                && call.range.end() < expected.end())
    );
    for source in [
        "proc counter {} {return VALUE}",
        "::proc counter {} {{n 0}} {return $n}",
    ] {
        let (analysis, commands) = fixture(source);
        let procedure = analysis
            .all_procs
            .values()
            .find(|p| p.name == "counter")
            .unwrap();
        assert_eq!(
            procedure.body_span,
            commands[0].arg_tokens().last().unwrap().span
        );
    }
}
