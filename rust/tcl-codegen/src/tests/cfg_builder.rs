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

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tcl_compiler::cfg::{Block, BlockId, CfgModule, Function, LoopNode, Terminator};
use tcl_compiler::cfg_builder::cfg_lower::matches_foreach_header;
use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::summary_candidates::OriginalProcedureSummaryOwners;
use tcl_compiler::cfg_builder::upvar_info::{FrameReach, UpvarInfo};
use tcl_compiler::command_binding::ModuleCommandBindings;
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::{CommandBindingSite, CommandTokens, Module, Script, Statement};
use tcl_compiler::registry_invocation::OwnedInvocationMetadataContext as CfgMetadataContext;
use tcl_compiler::var_refs::{VarReferenceScanner, VarScanOptions};
use tcl_lexer::{Span, TokenType};
use tcl_registry::EffectiveRegistrySemantics;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, Traits};

use tcl_compiler::cfg_builder::*;
use tcl_compiler::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};
use tcl_compiler::ir::{ForeachIterator, IfClause, SwitchArm, SwitchMode};

fn native_registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

#[test]
fn folded_eval_keeps_a_separate_executable_compilation_boundary() {
    let registry = native_registry();
    let module = tcl_compiler::lowering::lower_to_ir(
        "proc p {} {eval {set later CHILD}; set later PARENT; return $later}",
        &registry,
    );
    let procedure = module
        .procedures
        .get(&fixture_id(&module.procedures, "::p"))
        .expect("authored fixture body");
    let block = procedure
        .body
        .statements
        .iter()
        .find(|statement| {
            matches!(
                statement,
                Statement::Block {
                    error_context: Some(
                        tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation
                    ),
                    ..
                }
            )
        })
        .expect("folded logical eval body");
    let original = block.tokens().expect("original invocation");
    let executable = build_cfg_codegen_with_registry(&module, false, &registry);
    let function = executable
        .procedures
        .get(&fixture_id(&executable.procedures, "::p"))
        .expect("authored fixture body");
    let boundary = function
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find_map(|statement| match statement {
            Statement::Barrier {
                tokens: Some(tokens),
                args,
                ..
            } if tokens.argv_texts == original.argv_texts => Some((tokens, args)),
            _ => None,
        })
        .expect("runtime eval invocation");
    assert_eq!(boundary.0, original);
    assert_eq!(boundary.1, &original.argv_texts[1..]);
    assert!(!function.blocks.values().flat_map(|block| &block.statements)
            .any(|statement| matches!(statement, Statement::AssignConst { value, .. } if value == "CHILD")));
    let analysis = build_cfg_with_registry(&module, false, &registry);
    assert!(
        analysis
            .procedures
            .get(&fixture_id(&analysis.procedures, "::p"))
            .expect("authored fixture body")
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|statement| {
                if matches!(statement, Statement::AssignConst { value, .. } if value == "CHILD") {
                    return true;
                }
                statement.tokens().is_some_and(|tokens| {
                    tcl_compiler::registry_invocation::advisory_value_assignments(&registry, tokens)
                        .iter()
                        .any(|assignment| {
                            assignment.name == "later" && assignment.value.legacy_text() == "CHILD"
                        })
                })
            })
    );
    let assembly = crate::codegen::codegen_module(&executable, &module, &registry)
        .expect("original source namespace");
    let body = assembly.procedures.values().next().expect("compiled p");
    assert!(body.lvt.find_bytes(b"later").is_some());
    assert!(
        !body
            .literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"CHILD")
    );
    assert!(
        body.literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"set later CHILD")
    );
}
