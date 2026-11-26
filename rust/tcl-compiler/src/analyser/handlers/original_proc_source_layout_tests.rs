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

use crate::analyser::{Analyser, ResolvedAnalysisInput};
use std::sync::Arc;

fn fixture(
    source: &str,
) -> (
    Analyser,
    crate::analyser::AnalysisResult,
    Vec<crate::segmenter::SegmentedCommand>,
) {
    let environment = tcl_registry::model::ingress::resolve_environment("jimtcl");
    let profile = environment.analyser_profile();
    let context = Arc::new(
        environment
            .default_context_registry()
            .with_command_store(Arc::new(tcl_registry::CommandRegistry::build_default())),
    );
    assert!(context.commands().profile().is_none());
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    let analysis = analyser
        .analyse_and_retain_result_for_test(source, "jimtcl")
        .clone();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        analyser.file_lexer_config(),
    );
    (analyser, analysis, commands)
}

pub fn original_jim_source_layout_keeps_statics_and_actual_body_in_shared_registry() {
    let source = "proc counter {} {{n 0}} {not_a_command_anywhere $n}; counter";
    let (analyser, analysis, commands) = fixture(source);
    let command = &commands[0];
    let args = command.args();
    let args_text: Vec<_> = args.iter().map(String::as_str).collect();
    assert!(
        analyser
            .analysis_context()
            .commands()
            .procedure_definition_words("proc", &args_text)
            .is_none()
    );
    let layout = analyser
        .original_procedure_source_words_at(command.argv[0], args, command.arg_tokens())
        .expect("same original authored Jim declaration shape");
    assert_eq!(
        layout,
        tcl_registry::ProcedureWords {
            name: 0,
            params: 1,
            statics: Some(2),
            body: 3
        }
    );
    let procedure = analysis
        .all_procs
        .values()
        .find(|procedure| procedure.name == "counter")
        .unwrap();
    assert_eq!(procedure.body_span, command.arg_tokens()[3].span);
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|invocation| invocation.name == "not_a_command_anywhere")
    );
}

pub fn original_procedure_source_layout_refuses_foreign_values_tokens_and_expansion() {
    let source = "proc counter {} {{n 0}} {return $n}";
    let (mut analyser, _analysis, commands) = fixture(source);
    let command = &commands[0];
    assert!(
        analyser
            .original_procedure_source_words_at(
                command.argv[0],
                command.args(),
                command.arg_tokens()
            )
            .is_some()
    );
    let mut args = command.args().to_vec();
    "return FOREIGN".clone_into(&mut args[3]);
    assert!(
        analyser
            .original_procedure_source_words_at(command.argv[0], &args, command.arg_tokens())
            .is_none()
    );
    let mut tokens = command.arg_tokens().to_vec();
    tokens.swap(2, 3);
    assert!(
        analyser
            .original_procedure_source_words_at(command.argv[0], command.args(), &tokens)
            .is_none()
    );
    analyser.source = format!("{}\n# changed complete source", analyser.source).into();
    assert!(
        analyser
            .original_procedure_source_words_at(
                command.argv[0],
                command.args(),
                command.arg_tokens()
            )
            .is_none()
    );
    let source = "proc counter {} {*}$body";
    let (analyser, _analysis, commands) = fixture(source);
    let command = &commands[0];
    assert!(
        analyser
            .original_procedure_source_words_at(
                command.argv[0],
                command.args(),
                command.arg_tokens()
            )
            .is_none()
    );
}

pub fn original_procedure_source_layout_keeps_plain_and_rooted_jim_forms() {
    let source = "foreach name {a b} {proc $name {} {{n 0}} {not_a_command_anywhere $n}}";
    let (_analyser, analysis, _commands) = fixture(source);
    let start = source.find("{not_a_command_anywhere").unwrap();
    let end = start + "{not_a_command_anywhere $n}".len();
    let expected = tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
    for name in ["a", "b"] {
        let procedure = analysis
            .all_procs
            .values()
            .find(|procedure| procedure.name == name)
            .expect("both finite foreach declarations retain their genuine source body");
        assert_eq!(procedure.body_span, expected);
    }
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|invocation| invocation.name == "not_a_command_anywhere"
                && invocation.range.start() > expected.start()
                && invocation.range.end() < expected.end())
    );
    for source in [
        "proc counter {} {return VALUE}",
        "::proc counter {} {{n 0}} {return $n}",
    ] {
        let (analyser, analysis, commands) = fixture(source);
        let command = &commands[0];
        let layout = analyser
            .original_procedure_source_words_at(
                command.argv[0],
                command.args(),
                command.arg_tokens(),
            )
            .unwrap();
        assert_eq!(layout.body, command.args().len() - 1);
        let procedure = analysis
            .all_procs
            .values()
            .find(|procedure| procedure.name == "counter")
            .unwrap();
        assert_eq!(procedure.body_span, command.arg_tokens()[layout.body].span);
    }
}
