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
use crate::analyser::Analyser;
use tcl_syntax::formal_params::FormalByteArgumentBinding;

#[test]
fn original_procedure_arguments_keep_actual_formals_prefixes_and_child_anchors() {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    for source in [
        "proc target {captured written} {}; target {*}{FIXED VALUE}",
        "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias {*}{VALUE}",
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.command_invocations.clear();
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let call = commands.last().unwrap();
        let prototype =
            original_procedure_arguments(source, &analysis, call).expect("actual source prototype");
        assert_eq!(prototype.formals().parameters().len(), 2);
        assert_eq!(prototype.arguments().len(), 2);
        assert_eq!(
            prototype
                .formals()
                .name_field(1)
                .unwrap()
                .original_input()
                .bytes(),
            b"written"
        );
        assert!(
            prototype
                .bindings()
                .contains(&FormalByteArgumentBinding::Value {
                    parameter: 1,
                    argument: 1
                })
        );
        let operand = prototype.operands()[1]
            .as_ref()
            .expect("actual expansion child");
        assert!(operand.word().is_none());
        assert_eq!(operand.input().unwrap().bytes(), b"VALUE");
        assert_eq!(&source[operand.span().as_range()], "VALUE");
        if source.contains("interp alias") {
            assert!(prototype.operands()[0].is_none());
        }
        assert!(prototype.matches_source(&tcl_lexer::SourceImage::document(source), config));
        assert!(prototype.matches_registry(analysis.resolved_registry().unwrap()));
        assert!(original_procedure_arguments(&format!("#{source}"), &analysis, call).is_none());
    }
    let source = "proc target {first {second DEFAULT}} {}; target ONLY";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    );
    let prototype =
        original_procedure_arguments(source, &analysis, commands.last().unwrap()).unwrap();
    assert!(
        prototype
            .bindings()
            .contains(&FormalByteArgumentBinding::Default { parameter: 1 })
    );
    assert_eq!(prototype.operands().len(), 1);
}

#[test]
fn original_procedure_arguments_keep_missing_original_correspondence_terminal() {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    let source = "proc target {value} {}; target ONE";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let call = commands.last().unwrap();
    assert!(original_procedure_arguments(source, &analysis, call).is_some());
    analysis
        .command_invocations
        .iter_mut()
        .find(|row| row.range.start() == call.argv[0].span.start())
        .unwrap()
        .original_lookup = None;
    assert!(original_procedure_arguments(source, &analysis, call).is_none());
    let source = "proc target {value} {}; set values ONE; target {*}$values";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    );
    let prototype =
        original_procedure_arguments(source, &analysis, commands.last().unwrap()).unwrap();
    assert_eq!(
        prototype.arguments()[0].literal_bytes(),
        Some(b"ONE".as_slice())
    );
    assert!(
        prototype.operands()[0].is_none(),
        "a frozen list value cannot borrow its earlier setter's source extent"
    );
}
