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
use crate::cfg::{Block, Function, Terminator};
use crate::expr_ast::ExprNode;
use tcl_lexer::Span;

mod construction;
mod core_types;
mod defs;
mod dominators;
mod effect_markers;
mod instance_options;
mod invocation_context;
mod logical_source_roles;
mod phi_placement;
mod source_may_effects;
mod uses;
mod word_scan;

use invocation_context::original_setter_definition_point;

fn logical_role_metadata_fixture() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    logical_metadata_fixture("info exists selected")
}

fn logical_metadata_fixture(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut info = registry.get("info").unwrap().clone();
    info.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(info);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
        .expect("genuine positive supplied Logical input");
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (context, unit)
}

fn original_operand_metadata_fixture() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::environment_ingress::RetainedNativeUnit,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut setter = registry.get("set").unwrap().clone();
    setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(setter);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set selected VALUE",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        Arc::clone(&context),
    );
    (
        context,
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
    )
}

/// Intern `name` into `func` and insert a fresh block for it, returning
/// the [`BlockId`]. The shared test idiom for building a CFG by hand.
fn block(func: &mut Function, name: &str) -> BlockId {
    let id = func.intern_block(name);
    func.blocks.insert(id, Block::new(name));
    id
}

fn make_goto(target: BlockId) -> Terminator {
    Terminator::Goto { target, span: None }
}

fn make_branch(cond: &str, t: BlockId, f: BlockId) -> Terminator {
    Terminator::Branch {
        condition: ExprNode::Raw { text: cond.into() },
        true_target: t,
        false_target: f,
        span: None,
        condition_base: None,
    }
}

fn make_return() -> Terminator {
    Terminator::Return {
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        braced: false,
    }
}

fn call(span: Span, command: &str, args: &[&str], defs: &[&str]) -> Statement {
    Statement::Call {
        span,
        command: command.into(),
        canonical_command: None,
        args: args.iter().map(|arg| (*arg).into()).collect(),
        defs: defs.iter().map(|name| (*name).into()).collect(),
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

/// `(func, [entry, then, else, end])` for a diamond CFG:
/// entry → branch → then/else → end → return.
fn diamond_cfg() -> Function {
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let then = block(&mut func, "then");
    let els = block(&mut func, "else");
    let end = block(&mut func, "end");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
    func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
    func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
    func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
    func
}

/// Build a loop CFG: entry → header → branch → body → header / end
fn loop_cfg() -> Function {
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let header = block(&mut func, "header");
    let body = block(&mut func, "body");
    let end = block(&mut func, "end");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
    func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
    func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
    func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
    func
}

/// Resolve a block name to its id in `func` (test convenience).
fn id_of(func: &Function, name: &str) -> BlockId {
    func.block_id(name).expect("block name interned")
}

// uses_of tests

fn default_registry() -> CommandRegistry {
    CommandRegistry::build_default()
}
