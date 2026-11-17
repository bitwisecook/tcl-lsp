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

//! Whether a procedure body may be run again from its start: the proof a
//! splice of it needs, so a failure inside the splice can be presented by
//! invoking the procedure instead.

use super::{CommandRegistry, Script, SourceInvocationBinding, Statement};
use crate::ir::{CommandTokens, Module};
use tcl_registry::world_effect::{CallbackKinds, Reentrancy, WorldStateDomain};

/// Why a body may not be run again from its start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReExecutionDecline {
    /// A statement has no invocation the analysis retained.
    NoInvocation,
    /// An invocation is not proved to reach one registry implementation.
    UnprovedTarget,
    /// The registry holds no facts for an invocation.
    NoFacts,
    /// An invocation changes the frame, runs a body, or declares a state
    /// transition (a link, an alias, a declaration).
    StateTransition,
    /// An invocation touches state outside its frame's own variables.
    WorldAccess(WorldStateDomain),
    /// An invocation touches variables outside the current namespace context.
    ForeignVariables,
    /// An invocation can call back into the interpreter.
    Callback,
    /// A variable the body reads may run a trace.
    TracedRead,
    /// An expression calls a math function, whose result the registry does
    /// not declare a function of its arguments.
    MathFunction,
    /// An argument has a role other than a value or a plain variable read or
    /// write: a variable cell, a link, a name list, a script or a command.
    UnclosedRole(tcl_registry::ArgRole),
}

/// Whether `body`, a procedure body of `module` as a fresh interpreter of the
/// release reads it, may be run again from its start after anything fails
/// part way: every invocation it reaches, nested substitutions included, is
/// proved to reach one registry implementation whose declared effects touch
/// only variables of the current frame, the interpreter result and completion
/// state, and which neither calls back nor re-enters; no variable it reads may
/// run a trace; and no expression calls a math function. Which of the frame's
/// variables a body touches is the inliner's own frame proof.
///
/// # Errors
/// The first condition a reached invocation fails.
pub fn re_executable_body(
    module: &Module,
    body: &Script,
    registry: &CommandRegistry,
) -> Result<(), ReExecutionDecline> {
    use ReExecutionDecline as Decline;
    let mut pending = vec![body];
    while let Some(script) = pending.pop() {
        for site in script.command_binding_sites.iter() {
            let tokens = site.source_tokens.as_deref().ok_or(Decline::NoInvocation)?;
            invocation_re_executable(module, tokens, registry)?;
        }
        for statement in &script.statements {
            match statement.tokens() {
                Some(tokens) => invocation_re_executable(module, tokens, registry)?,
                None if statement.is_executable_invocation()
                    && !script
                        .command_binding_sites
                        .iter()
                        .any(|site| site.span == statement.span()) =>
                {
                    return Err(Decline::NoInvocation);
                }
                None => {}
            }
            if math_function_called(statement) {
                return Err(Decline::MathFunction);
            }
            pending.extend(statement.child_scripts());
        }
    }
    Ok(())
}

fn invocation_re_executable(
    module: &Module,
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Result<(), ReExecutionDecline> {
    let binding = tokens
        .source_binding
        .as_ref()
        .ok_or(ReExecutionDecline::UnprovedTarget)?;
    binding_re_executable(module, tokens, binding, registry)?;
    for (_, nested) in tokens.nested_bindings.iter() {
        let Some((_, mut nested_tokens)) = nested.original_recorded_command() else {
            return Err(ReExecutionDecline::NoInvocation);
        };
        nested_tokens.source_binding = Some(nested.clone());
        invocation_re_executable(module, &nested_tokens, registry)?;
    }
    Ok(())
}

fn binding_re_executable(
    module: &Module,
    tokens: &CommandTokens,
    binding: &SourceInvocationBinding,
    registry: &CommandRegistry,
) -> Result<(), ReExecutionDecline> {
    use ReExecutionDecline as Decline;
    let target = binding
        .proved_execution_target()
        .ok_or(Decline::UnprovedTarget)?;
    if binding.unknown || binding.may_be_absent || !target.registry_backed {
        return Err(Decline::UnprovedTarget);
    }
    let metadata = binding.original_argument_metadata_for_module(tokens, module, registry);
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry, metadata, tokens,
    )
    .ok_or(Decline::NoFacts)?;
    let effects = &invocation.facts.effects;
    let facts = &invocation.facts;
    if facts.frame_effect.is_some()
        || facts.body_execution.is_some()
        || facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| !transitions.facts().is_empty())
    {
        return Err(Decline::StateTransition);
    }
    let callback = effects.callback();
    let context = &binding.variable_context;
    let traced = context.dynamic_traces
        || !context.traced.is_empty()
        || !context.trace_registrations.is_empty()
        || !context.untracked_traces.is_empty()
        || !context.possible_trace_registrations.is_empty();
    // A trace is the only callback a variable access can make, and none is
    // set on a variable this frame holds (as the native body template rules).
    let trace_only = callback.kinds == CallbackKinds::TRACE && !traced;
    if !trace_only
        && (callback.kinds != CallbackKinds::NONE || callback.reentrancy != Reentrancy::Never)
    {
        return Err(Decline::Callback);
    }
    for access in effects.accesses() {
        match access.domain {
            WorldStateDomain::VariableStore => {
                if access.namespace != tcl_registry::world_effect::NamespaceScope::Current
                    || access.interpreter != tcl_registry::world_effect::InterpreterScope::Current
                {
                    return Err(Decline::ForeignVariables);
                }
            }
            WorldStateDomain::InterpreterResult | WorldStateDomain::CompletionState => {}
            other => return Err(Decline::WorldAccess(other)),
        }
    }
    if traced {
        return Err(Decline::TracedRead);
    }
    if !target.prepended.is_empty() {
        return Err(Decline::ForeignVariables);
    }
    variables_local(module, tokens, binding, &invocation.facts)
}

/// Whether each variable the invocation names, as an operand or a
/// substitution, is a plain local of the frame, and no argument names
/// anything else a re-run could not repeat.
fn variables_local(
    module: &Module,
    tokens: &CommandTokens,
    binding: &SourceInvocationBinding,
    facts: &tcl_registry::InvocationFacts,
) -> Result<(), ReExecutionDecline> {
    use ReExecutionDecline as Decline;
    let context = &binding.variable_context;
    if !facts.arg_roles_complete {
        return Err(Decline::ForeignVariables);
    }
    // Each variable the invocation names, as an operand or a substitution,
    // is a plain local of the frame.
    let local = |name: &str| {
        let root = tcl_syntax::naming::split_element_ref(name).map_or(name, |(root, _)| root);
        !root.is_empty()
            && !tcl_syntax::naming::is_qualified(root.as_bytes())
            && !context.globals.contains(root)
            && !context.ns_vars.contains(root)
            && !context.upvar_aliases.contains_key(root)
            && !context.instance_vars.contains(root)
    };
    for &(index, role) in &facts.arg_roles {
        use tcl_registry::ArgRole;
        match role {
            ArgRole::VarRead | ArgRole::VarWrite => {}
            ArgRole::Value
            | ArgRole::Pattern
            | ArgRole::Option
            | ArgRole::Subcommand
            | ArgRole::OptionTerminator
            | ArgRole::FormatString
            | ArgRole::ScanFormat
            | ArgRole::Index
            | ArgRole::Keyword
            | ArgRole::Result => continue,
            // A cell, a link or a name list, a script, a command name.
            _ => return Err(Decline::UnclosedRole(role)),
        }
        let word = tokens
            .words()
            .get(1 + facts.argument_offset + usize::from(index));
        match word {
            None => {}
            Some(
                crate::ir::WordExpr::Literal { text, .. }
                | crate::ir::WordExpr::BracedLiteral { text, .. },
            ) if local(text) => {}
            Some(_) => return Err(Decline::ForeignVariables),
        }
    }
    let config = module.native_lexer_config();
    for access in &tokens.variable_accesses {
        let root = tcl_syntax::naming::variable_reference_root_bytes(
            access.original_spelling.as_bytes(),
            config,
        );
        match root {
            Ok(Some(root)) if std::str::from_utf8(root).is_ok_and(local) => {}
            _ => return Err(Decline::ForeignVariables),
        }
    }
    Ok(())
}

fn math_function_called(statement: &Statement) -> bool {
    let mut called = false;
    let mut visit = |node: &crate::expr_ast::ExprNode| {
        called |= matches!(node, crate::expr_ast::ExprNode::Call { .. });
    };
    match statement {
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            walk_expression(expr, &mut visit);
        }
        Statement::Return {
            expr: Some(expr), ..
        } => walk_expression(expr, &mut visit),
        Statement::If { clauses, .. } => {
            for clause in clauses {
                walk_expression(&clause.condition, &mut visit);
            }
        }
        Statement::While { condition, .. } | Statement::For { condition, .. } => {
            walk_expression(condition, &mut visit);
        }
        _ => {}
    }
    called
}

fn walk_expression(
    node: &crate::expr_ast::ExprNode,
    visit: &mut impl FnMut(&crate::expr_ast::ExprNode),
) {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        visit(node);
        match node {
            ExprNode::Binary { left, right, .. } => pending.extend([&**left, &**right]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([&**condition, &**true_branch, &**false_branch]),
            ExprNode::Call { args, .. } => pending.extend(args.iter()),
            _ => {}
        }
    }
}
