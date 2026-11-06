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
use tcl_compiler::command_binding::compiled_preflight::ordered_locals::*;
use tcl_compiler::command_binding::formal_topology::OriginalFormalTopology;
use tcl_compiler::command_binding::{CommandAllocationSite, SourceExecutionContext};
use tcl_core_types::NameBytes;
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_syntax::naming::{
    NativeCompiledVariableEnvironment, NativeCompiledVariableLookup, NativeCompiledVariableProtocol,
};

mod original_native_request_fixtures {
    use crate as tcl_codegen;
    // tcl-lsp-no-smoke-include
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/native_compiled_fixture.rs"
    ));
}

mod original_native_request_tests {
    use std::hash::{Hash, Hasher};
    use std::sync::Arc;
    use tcl_compiler::command_binding::CommandAllocationSite;
    use tcl_compiler::command_binding::compiled_preflight::ordered_locals::*;

    fn retained_locals(
        fixture: &super::original_native_request_fixtures::CompiledFixture,
    ) -> SourceCompilerLocalInventory {
        let module = fixture.module();
        assert!(module.source_metadata_input.is_none());
        assert!(
            module
                .source_entry
                .native_entry
                .as_ref()
                .unwrap()
                .compiled_local_layout
                .is_none()
        );
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let bindings = retained
            .positioned_for_module(module, module.resolved_registry())
            .unwrap();
        let visits: Vec<_> = bindings.compiler_invocations.values().flatten().collect();
        assert_ne!(
            visits,
            [] as [&tcl_compiler::command_binding::compiler_inventory::SourceCompilerInvocation; 0]
        );
        assert!(visits.iter().all(|visit| {
            visit
                .table
                .state
                .source_variables
                .original_formal_topology
                .is_none()
        }));
        visits
            .iter()
            .filter_map(|visit| visit.source_locals.as_deref())
            .filter(|locals| matches!(locals.formals, SourceCompilerFormalOwner::Native(_)))
            .max_by_key(|locals| locals.locals.len())
            .expect("actual request-specific compiler local traversal")
            .clone()
    }

    fn assert_original_expression_boundaries(
        fixture: &super::original_native_request_fixtures::CompiledFixture,
        locals: &SourceCompilerLocalInventory,
    ) {
        use tcl_registry::native_expression_program::NativeExpressionTree;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let module = fixture.module();
        let bindings = module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_for_module(module, module.resolved_registry())
            .unwrap();
        let (site, program) = bindings
            .compiler_invocations
            .values()
            .flatten()
            .find_map(|visit| {
                let preparation = visit.structured.as_deref()?;
                let NativeInstructionPlan::Expression(expression) = &preparation.recipe else {
                    return None;
                };
                Some((&preparation.compilation_site, expression.program.as_ref()?))
            })
            .expect("actual selected original expression preparation");
        let accepts =
            |inventory: &SourceCompilerLocalInventory,
             program: &tcl_registry::native_expression_program::NativeExpressionProgram,
             site: &CommandAllocationSite| {
                inventory.retains_existing_expression(
                    program,
                    site,
                    locals.config,
                    &locals.frame,
                    locals.compilation,
                )
            };
        assert!(accepts(locals, program, site));
        let mut missing = locals.clone();
        missing.locals.clear();
        assert!(
            !accepts(&missing, program, site),
            "no new slot may be minted"
        );
        let mut foreign = site.clone();
        foreign.source = Arc::new(site.source.as_ref().clone());
        assert!(!accepts(locals, program, &foreign));
        let mut changed = program.clone();
        changed.source.push(b' ');
        assert!(!accepts(locals, &changed, site));
        let mut different_recipe = program.clone();
        different_recipe.context.native_syntax =
            tcl_syntax::expr::parser::NativeExprSyntax::Unknown;
        assert!(!accepts(locals, &different_recipe, site));
        let mut rejected = program.clone();
        rejected.tree = NativeExpressionTree::Rejected {
            message: Vec::new(),
            error_code: None,
        };
        assert!(!accepts(locals, &rejected, site));
    }

    fn fingerprint(locals: &SourceCompilerLocalInventory) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        locals.hash(&mut state);
        state.finish()
    }

    #[test]
    fn original_native_request_formals_keep_actual_scope_and_withdraw_after_owner_retirement() {
        let source = "proc p {a b} {set n 1; return [expr {$a + $b}]}";
        let mut fixture =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let locals = retained_locals(&fixture);
        assert_original_expression_boundaries(&fixture, &locals);
        assert!(locals.owns(
            &locals.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        for name in [b"a".as_slice(), b"b".as_slice(), b"n".as_slice()] {
            assert_eq!(
                locals
                    .primary_for(name, locals.compilation)
                    .unwrap()
                    .as_bytes(),
                name
            );
        }
        let (_, body) = fixture.body();
        assert_eq!(body.params_raw, "");
        let before = fingerprint(&locals);
        fixture.drop_owner();
        assert_eq!(
            fingerprint(&locals),
            before,
            "retirement cannot mutate an immutable key"
        );
        assert!(!locals.owns(
            &locals.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        assert!(locals.primary_for(b"a", locals.compilation).is_none());
    }

    #[test]
    fn original_native_request_formals_refuse_foreign_scope_and_keep_existing_body_boundaries() {
        let source = "proc p {a b} {set n 1; return [expr {$a + $b}]}";
        let fixture =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let foreign =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let locals = retained_locals(&fixture);
        let other = retained_locals(&foreign);
        assert_ne!(locals.formals, other.formals);
        assert!(!locals.owns(
            &other.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut reboxed = locals.chunk.clone();
        reboxed.source = Arc::new(locals.chunk.source.as_ref().clone());
        assert!(!locals.owns(
            &reboxed,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut changed = locals.config;
        changed.leading_bom = if changed.leading_bom == tcl_lexer::LeadingBom::Content {
            tcl_lexer::LeadingBom::Skip
        } else {
            tcl_lexer::LeadingBom::Content
        };
        assert!(!locals.owns(
            &locals.chunk,
            changed,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut module = fixture.module().clone();
        let mut changed_source = module.source.bytes().to_vec();
        changed_source.push(b' ');
        module.source = tcl_lexer::SourceImage::from_bytes(changed_source, module.source.channel());
        assert!(fixture.refuses_module(&module));
        assert!(fixture.refuses_foreign_entry(&foreign));
        super::original_native_request_fixtures::assert_declaration_and_refusal_boundaries();
        super::original_native_request_fixtures::assert_dispatched_binding_boundaries();
    }
}
