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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

mod assignment_folds;
mod cell_updates;
mod definition_values;
mod existence;
mod fold_trust;
mod foreach_lists;
mod lattice;
mod refinements;
mod solver;
mod statements;

use crate::cfg::{Block, BlockId, Function, Terminator};
use crate::expr_ast::ExprNode;

/// The Tcl 8.6 registry: a fixture here is release-invariant unless it
/// names another release.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
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

/// Convenience wrapper over [`sccp`] for tests with no `Module` in
/// hand — no traced variables, no dynamic variable trace.
fn sccp_no_traces(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
) -> SccpResult {
    sccp(
        cfg,
        ssa,
        param_constants,
        policy,
        TraceInputs {
            source_metadata_input: None,
            registry: &registry(),
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
    )
}

/// [`sccp_no_traces`] under a module that mutates no command binding —
/// the shared per-unit lattice's stance for an untouched file, as
/// [`evaluate_pristine`] states it for one statement. Without the trust
/// fact no resolved command answers, so a test of a route states it.
fn sccp_pristine(cfg: &CfgFunction, ssa: &SsaFunction, policy: FoldPolicy) -> SccpResult {
    let registry = registry();
    sccp_with_builtin_folds(
        cfg,
        ssa,
        None,
        policy,
        &HashSet::new(),
        TraceInputs {
            source_metadata_input: None,
            registry: &registry,
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &crate::command_binding::ModuleCommandMutations::default(),
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
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

/// The existence facts each `puts` reading `name` finds, in source
/// order.
fn puts_reads(f: &crate::compilation_unit::FunctionUnit, name: &str) -> Vec<Existence> {
    let symbol = f.ssa.var_symbol(name).expect("the place is an SSA symbol");
    let mut reads: Vec<(u32, Existence)> = f
        .sccp
        .existence_reads
        .iter()
        .filter(|((_, _, read), _)| *read == symbol)
        .filter_map(|(&(block, index, _), &fact)| {
            let statement = f
                .cfg
                .blocks
                .get(&block)?
                .statements
                .get(usize::try_from(index).ok()?)?;
            matches!(statement, Statement::Call { command, .. }
                    if command.trim_start_matches("::") == "puts")
            .then(|| (statement.span().start(), fact))
        })
        .collect();
    reads.sort_by_key(|&(start, _)| start);
    reads.into_iter().map(|(_, fact)| fact).collect()
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

fn folds_for(dialect: &str) -> BuiltinFoldInputs<'static> {
    use std::sync::OnceLock;
    static MUTATIONS: OnceLock<crate::command_binding::ModuleCommandMutations> = OnceLock::new();
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    BuiltinFoldInputs {
        source_metadata_input: None,
        registry: tcl_registry::model::ingress::static_context_for(dialect).commands(),
        mutations: MUTATIONS.get_or_init(Default::default),
        dialect: Some(environment.analyser_profile()),
        defining_class: None,
        registry_engine: false,
        trust: FoldTrust::ObservedBindings,
        proven_pure_parameters: false,
    }
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
    crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false)
}

/// The value the solver gives the last definition of `name` in `f`.
fn last_value(f: &crate::compilation_unit::FunctionUnit, name: &str) -> LatticeValue {
    let symbol = f
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    f.sccp
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, value)| value.clone())
        .expect("a definition")
}

/// The values the solver gives the last definitions of `names` in the
/// function `name` of `source`.
/// The lattice value of `name`'s last definition in `::p` of `source`
/// under `dialect`. A statement the lowering did not write has no source
/// metadata, so these read the value a real unit settles.
fn settled(dialect: &str, source: &str, name: &str) -> LatticeValue {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    last_value(unit.function("::p").expect("::p is analysed"), name)
}

/// The lattice value of version `version` of `name` in `::p` of `source`
/// under `dialect`, for a name a later arm or merge defines again.
fn settled_at(dialect: &str, source: &str, name: &str, version: u32) -> LatticeValue {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    let function = unit.function("::p").expect("::p is analysed");
    let symbol = function
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    function
        .sccp
        .values
        .get(&(symbol, version))
        .cloned()
        .expect("the version is defined")
}

/// [`settled`] for a body: a procedure in a Tcl release, an event handler
/// under `f5-irules`, where a procedure is only run through `call`.
fn settled_body(dialect: &str, body: &str, name: &str) -> LatticeValue {
    if dialect != "f5-irules" {
        return settled(dialect, &format!("proc p {{}} {{{body}}}"), name);
    }
    let source = format!("when RULE_INIT {{{body}}}");
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        &source, registry, false, dialect,
    );
    last_value(
        unit.function("::when::RULE_INIT")
            .expect("the handler is analysed"),
        name,
    )
}
