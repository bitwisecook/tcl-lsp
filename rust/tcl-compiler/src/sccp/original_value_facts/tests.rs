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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;

mod assignment_folds;
mod definition_values;
mod existence_branches;
mod fold_trust;
mod foreach_lists;
mod lattice;
mod semantic_projection;
mod value_solver;

fn logical_callback_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (context, unit)
}

use crate::cfg::{Block, BlockId, Function, Terminator};
use crate::expr_ast::ExprNode;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// [`evaluate_fixture_def_with_folds`] under a module that mutates no command
/// binding — the stance the shared per-unit lattice takes for an
/// untouched file ([`FoldTrust::ObservedBindings`] over
/// [`crate::command_binding::ModuleCommandMutations`]'s `Default`).
///
/// Plain [`evaluate_fixture_def`] holds no whole-module command view at all and
/// therefore folds no builtin command substitution, so a fold-arm test
/// has to state the trust fact it folds under.
fn evaluate_pristine<S: std::hash::BuildHasher>(
    stmt: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        values,
        ssa,
        policy,
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry(),
            mutations: &crate::command_binding::ModuleCommandMutations::default(),
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// Convenience wrapper over [`crate::sccp::sccp`] for tests with no `Module` in
/// hand — no traced variables, no dynamic variable trace.
fn sccp_no_traces(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
) -> SccpResult {
    crate::sccp::sccp(
        cfg,
        ssa,
        param_constants,
        policy,
        TraceInputs {
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
            source_metadata_input: None,
            registry: &registry(),
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
        },
    )
}

/// Intern `name` and insert a fresh block; returns its [`BlockId`].
fn block(f: &mut Function, name: &str) -> BlockId {
    let id = f.intern_block(name);
    f.blocks.insert(id, Block::new(name));
    id
}

fn id_of(f: &Function, name: &str) -> BlockId {
    f.block_id(name).expect("interned")
}

fn goto(target: BlockId) -> Terminator {
    Terminator::Goto { target, span: None }
}

fn branch(cond: ExprNode, tt: BlockId, ft: BlockId) -> Terminator {
    Terminator::Branch {
        condition: cond,
        true_target: tt,
        false_target: ft,
        span: None,
        condition_base: None,
    }
}

fn literal(text: &str) -> ExprNode {
    ExprNode::Literal {
        text: text.into(),
        start: 0,
        end: u32::try_from(text.len()).unwrap_or(0),
    }
}

// Driver.

use crate::ir::Statement;
use crate::ssa::{SsaBlock, SsaStatement};
use tcl_lexer::Span;

/// A block-less SSA function used purely as a variable-name interner for
/// the hand-built statement / lattice tests.
fn bare_ssa() -> SsaFunction {
    SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
}

fn assign_const_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn empty_ssa_block(name: &str) -> SsaBlock {
    SsaBlock {
        name: name.into(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: HashMap::new(),
        exit_versions: HashMap::new(),
    }
}

/// Build an [`SsaFunction`] over `f`'s interner with the given per-name SSA
/// blocks; any CFG block not listed gets an empty SSA block.
fn make_ssa(f: &Function, named: Vec<(&str, SsaBlock)>) -> SsaFunction {
    let mut ssa = SsaFunction::trivial("::top", f.entry, f.block_names().to_vec());
    let mut provided: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    for (name, blk) in named {
        let id = id_of(f, name);
        provided.insert(id);
        ssa.blocks.insert(id, blk);
    }
    for id in f.blocks.keys() {
        if !provided.contains(id) {
            ssa.blocks.insert(*id, empty_ssa_block(f.block_name(*id)));
        }
    }
    ssa
}

// AssignValue + command-substitution folding.

fn assign_value_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignValue {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_needs_backsubst: false,
            tokens: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn cu(src: &str) -> crate::compilation_unit::CompilationUnit {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    crate::compilation_unit::CompilationUnit::build_for_profile(src, &registry, false, profile)
}
