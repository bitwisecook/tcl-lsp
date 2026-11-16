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

#[test]
fn logical_source_occurrences_do_not_lend_generic_flow_or_current_tree_facts() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .unwrap()
        .default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = input.borrowed_context_registry().commands();
    for (source, expected) in [
        ("proc f {p} {set anchor 1; puts $missing}", true),
        ("proc f {p} {unknown_host; puts $missing}", false),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let procedure = module
            .procedures
            .values()
            .next()
            .expect("genuine Logical declaration");
        let tokens = procedure.body.statements[0].tokens().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let report = binding.declaration_flow_report(registry).unwrap();
        assert!(!report.owns_original_procedure(procedure, &module.source));
        let reads = binding
            .original_declared_procedure_read_occurrences(
                tokens,
                &input,
                procedure,
                &module.source,
                registry,
            )
            .expect("genuine original-source occurrence purpose");
        assert_eq!(
            reads.occurrences().any(|(name, _)| name == "missing"),
            expected
        );
        let mut changed_tree = procedure.clone();
        changed_tree.body.statements.clear();
        assert!(!report.owns_original_procedure(&changed_tree, &module.source));
        assert!(
            binding
                .original_declared_procedure_read_occurrences(
                    tokens,
                    &input,
                    &changed_tree,
                    &module.source,
                    registry,
                )
                .is_some(),
            "original source occurrences do not claim current tree custody"
        );
        let mut foreign = procedure.clone();
        foreign.declaration_site = None;
        assert!(
            binding
                .original_declared_procedure_read_occurrences(
                    tokens,
                    &input,
                    &foreign,
                    &module.source,
                    registry,
                )
                .is_none()
        );
        assert!(
            binding
                .original_declared_procedure_read_occurrences(
                    tokens,
                    &input,
                    procedure,
                    &tcl_lexer::SourceImage::document(&format!("# foreign\n{source}")),
                    registry,
                )
                .is_none()
        );
        let other = tcl_registry::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .default_context_registry();
        assert!(
            binding
                .original_declared_procedure_read_occurrences(
                    tokens,
                    &input,
                    procedure,
                    &module.source,
                    other.commands(),
                )
                .is_none()
        );
    }
}
