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

#[test]
fn source_read_selector_uses_original_occurrence_and_current_function_owner() {
    let source = "set same TOP; puts $same\nproc a {} {set same 123; return $same}\nproc b {} {set same TEXT; return $same}\n";
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
        .expect("actual complete Logical source input");
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    assert_eq!(unit.ir_module.source_metadata_input.as_ref(), Some(&input));
    let reads = source
        .match_indices("$same")
        .map(|(offset, _)| {
            original_source_read_at_unit(
                &unit,
                tcl_lexer::Span::new(
                    u32::try_from(offset).expect("fixture source offset fits u32"),
                    u32::try_from(offset).expect("fixture source offset fits u32") + 5,
                ),
                context.commands(),
            )
            .expect("actual original read has its own current source/SSA producer")
        })
        .collect::<Vec<_>>();
    assert_eq!(reads.len(), 3);
    assert!(std::ptr::eq(
        reads[0].function(),
        std::ptr::from_ref(&unit.top_level)
    ));
    assert!(!std::ptr::eq(reads[0].function(), reads[1].function()));
    assert!(!std::ptr::eq(reads[1].function(), reads[2].function()));
    for read in &reads {
        assert_eq!(read.function().ssa.var_name(read.symbol()), "same");
        assert!(std::ptr::eq(
            unit.function_for_body(read.function().body_id).unwrap(),
            read.function()
        ));
    }
    let span = reads[1].span();
    let body = reads[1].function().body_id.unwrap();
    assert!(
        original_source_read_at_unit(
            &unit,
            tcl_lexer::Span::new(span.start() + 1, span.end()),
            context.commands()
        )
        .is_none()
    );
    let mut stale = unit.clone();
    stale.source.push(' ');
    assert!(original_source_read_at_unit(&stale, span, context.commands()).is_none());
    for formals in [false, true] {
        let mut changed = unit.clone();
        let procedure = changed
            .ir_module
            .procedures
            .get_mut(&body)
            .or_else(|| changed.ir_module.body_units.get_mut(&body))
            .unwrap();
        if formals {
            procedure.params.push("added".into());
        } else {
            procedure.body_source = Some("return OTHER".into());
        }
        assert!(original_source_read_at_unit(&changed, span, context.commands()).is_none());
    }
    let mut missing = unit.clone();
    if let Some(function) = missing
        .procedures
        .get_mut(&body)
        .or_else(|| missing.body_units.get_mut(&body))
    {
        function.source_metadata_input = None;
    } else {
        panic!("actual original procedure function");
    }
    assert!(original_source_read_at_unit(&missing, span, context.commands()).is_none());
}
