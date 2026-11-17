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

//! The rewrite walk: each statement kind's recursion into its bodies, and what becomes of the value a call stands in.

use super::splice::{InlineExpansion, splice_call_site};
use super::{InlineSpec, Inlining, InliningSummaries, build_inlinable_map};
use std::collections::HashMap;
use tcl_compiler::ir::{
    CommandBindingSite, IfClause, Module, Script, Statement, SwitchArm, TryHandler,
};
use tcl_registry::CommandRegistry;

/// What becomes of the value of the command a call stands in, which decides what
/// replaces the call.
///
/// A procedure's value is its last command's, so the value of a call is
/// somebody's only where the call is the last command of its script and the
/// script's own value is somebody's in turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tail {
    /// Nothing reads it: a command that is not the last of its script, or the
    /// body of a loop.
    Dropped,
    /// It is the procedure's own value, so a `return` in its place ends the
    /// procedure as the callee's would have ended the callee.
    Returned,
    /// A command around the call reads it and a `return` would not end the
    /// procedure — a `catch` or a `try` holds it. Only a replacement that is
    /// itself the value will do.
    Captured,
}

/// Where a call stands in its script.
#[derive(Debug, Clone, Copy)]
pub(super) struct Site {
    /// What becomes of the call's value.
    pub(super) tail: Tail,
    /// Whether nothing of the rewritten script stands before the call, so that
    /// no earlier command can be mistaken for its value once it is gone.
    pub(super) first: bool,
}

pub(super) fn inline_procedures(
    mut module: Module,
    registry: &CommandRegistry,
    scope: &Inlining<'_>,
) -> Module {
    let catalogue = module.clone();
    let escape =
        tcl_compiler::var_escape::analyse_var_escape_with_registry(&catalogue, true, registry);
    let summaries = InliningSummaries {
        module: &catalogue,
        registry,
        escapes: &escape,
        reference_sites: match scope {
            Inlining::Everything => None,
            Inlining::Only(imported) | Inlining::Live(imported, _) => Some(&imported.sites),
        },
        live: match scope {
            Inlining::Live(_, bodies) => Some(bodies),
            Inlining::Everything | Inlining::Only(_) => None,
        },
    };
    // An imported definition was proved in the module it was lowered in: here
    // its text is an appendix no source binding of this module describes.
    let mut inlinable = match scope {
        Inlining::Everything => build_inlinable_map(&catalogue, &summaries, registry),
        Inlining::Only(imported) | Inlining::Live(imported, _) => imported.inlinable.clone(),
    };
    inlinable.retain(|id, _| scope.admits(id));
    if inlinable.is_empty() {
        return module;
    }

    let mut counter: usize = 0;
    let top_caller = module.top_level_namespace.clone();
    if let Some(may_be_global_frame) = scope.top_level(&module) {
        let (new_top, _) = rewrite_script(
            &module.top_level,
            &top_caller,
            &inlinable,
            &summaries,
            &mut counter,
            Tail::Returned,
            may_be_global_frame,
        );
        module.top_level = new_top;
    }

    let mut procedures = std::mem::take(&mut module.procedures);
    for (qname, proc) in &mut procedures {
        if matches!(scope, Inlining::Only(imported) | Inlining::Live(imported, _) if imported.ids.contains(qname))
        {
            continue;
        }
        let Some(caller_namespace) = proc
            .body
            .namespace_context
            .as_deref()
            .and_then(tcl_compiler::command_binding::SourceNamespaceKey::advisory_key)
        else {
            continue;
        };
        let (new_body, changed) = rewrite_script(
            &proc.body,
            &caller_namespace,
            &inlinable,
            &summaries,
            &mut counter,
            Tail::Returned,
            false,
        );
        if changed {
            proc.body = new_body;
        }
    }
    module.procedures = procedures;
    module
}

/// Rewrite a script, returning `(new_script, changed)`. `tail` is what becomes
/// of the script's own value; it is the last statement's, so a v3 inline of a
/// proc with a trailing `return` keeps the return where the value is the
/// procedure's and wraps it where nothing reads the value.
///
/// `caller_may_be_global_frame` distinguishes a module script from a procedure
/// body without encoding that fact in a synthetic procedure name. A module's
/// top-level activation can be Tcl frame zero, and every structured body
/// flattened into it keeps that possibility. The runtime cannot transparently
/// replay a command copied from another namespace there: frame zero is the
/// immutable `uplevel #0` / coroutine-flow root. Procedure bodies have a real
/// supra-global frame whose namespace can be replaced for the duration of a
/// stale-command replay.
pub(super) fn rewrite_script(
    script: &Script,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    tail: Tail,
    caller_may_be_global_frame: bool,
) -> (Script, bool) {
    if tcl_compiler::native_compilation_admission::script_requires_admission(script) {
        return (script.clone(), false);
    }
    let mut out: Vec<Statement> = Vec::with_capacity(script.statements.len());
    let mut command_binding_sites: Vec<CommandBindingSite> =
        script.command_binding_sites.iter().cloned().collect();
    let mut procedure_binding_requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity> = script
        .procedure_binding_requirements
        .iter()
        .cloned()
        .collect();
    let mut changed = false;
    let n = script.statements.len();
    for (i, stmt) in script.statements.iter().enumerate() {
        let site = Site {
            tail: if i + 1 == n { tail } else { Tail::Dropped },
            first: out.is_empty(),
        };
        match rewrite_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            site,
            caller_may_be_global_frame,
        ) {
            None => out.push(stmt.clone()),
            Some(replacement) => {
                changed = true;
                out.extend(replacement.statements);
                command_binding_sites.extend(replacement.command_binding_sites);
                procedure_binding_requirements.extend(replacement.procedure_bindings);
            }
        }
    }
    // The rewritten script is still the script it was: the same namespace,
    // the same evaluated source, and what its own expressions were prepared as.
    let mut rewritten =
        Script::from_transformed_parts(out, command_binding_sites, procedure_binding_requirements);
    rewritten
        .namespace_context
        .clone_from(&script.namespace_context);
    rewritten
        .executed_source
        .clone_from(&script.executed_source);
    rewritten
        .implicit_math_invocations
        .clone_from(&script.implicit_math_invocations);
    rewritten
        .expression_preparations
        .clone_from(&script.expression_preparations);
    (rewritten, changed)
}

/// Return a replacement expansion, or `None` to keep `stmt`. An empty
/// statement list drops the statement (v0); a non-empty list substitutes
/// the inlined body. A body nested in the statement is rewritten for the value
/// the statement gives it: a branch of an `if` or `switch`, and the body of a
/// block, are the statement's own tail; a `catch` or `try` holds the value of
/// its body; a loop reads none.
pub(super) fn rewrite_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    site: Site,
    caller_may_be_global_frame: bool,
) -> Option<InlineExpansion> {
    match stmt {
        Statement::Call {
            tokens: Some(tokens),
            ..
        } => {
            let target = summaries.selected_body(tokens)?;
            // A procedure invocation normally creates a frame in its defining
            // namespace. Inlining erases that frame. If a preceding command in
            // the copied body mutates a later specialised command, the VM
            // replays that later command in the copied source namespace. It can
            // do so transparently only above frame zero; replacing frame zero
            // would change `uplevel #0` and leak through coroutine flow swaps.
            // Keep the original procedure call whenever a module-level caller
            // may occupy frame zero and the callee belongs to another namespace.
            if caller_may_be_global_frame
                && summaries
                    .module
                    .procedures
                    .get(&target)?
                    .body
                    .namespace_context
                    .as_ref()?
                    .advisory_key()?
                    != caller_qname
            {
                return None;
            }
            let spec = inlinable.get(&target)?;
            let renamed = summaries
                .live
                .map(|bodies| move |names: &HashMap<String, String>| bodies.renamed(target, names));
            splice_call_site(
                stmt,
                spec,
                caller_qname,
                counter,
                site,
                summaries.live.is_some(),
                renamed
                    .as_ref()
                    .map(|renamed| renamed as &super::splice::RenamedBody<'_>),
            )
        }
        Statement::Block { .. } => rewrite_block_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            site.tail,
            caller_may_be_global_frame,
        )
        .map(InlineExpansion::without_bindings),
        Statement::If { .. } => rewrite_if_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            site.tail,
            caller_may_be_global_frame,
        )
        .map(InlineExpansion::without_bindings),
        Statement::For { .. } => rewrite_for_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            caller_may_be_global_frame,
        )
        .map(InlineExpansion::without_bindings),
        Statement::While { .. } | Statement::Foreach { .. } | Statement::Catch { .. } => {
            rewrite_single_body_stmt(
                stmt,
                caller_qname,
                inlinable,
                summaries,
                counter,
                caller_may_be_global_frame,
            )
            .map(InlineExpansion::without_bindings)
        }
        Statement::Try { .. } => rewrite_try_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            caller_may_be_global_frame,
        )
        .map(InlineExpansion::without_bindings),
        Statement::Switch { .. } => rewrite_switch_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            site.tail,
            caller_may_be_global_frame,
        )
        .map(InlineExpansion::without_bindings),
        // An `uplevel` body is not lowered into the function: the statement is
        // emitted from its source text, so a splice made in the copy would be
        // discarded, and what it recorded would be a requirement of code that
        // never runs. Nothing else holds a body.
        _ => None,
    }
}

/// `Block`: recurse into the body under the block's (possibly namespaced)
/// caller name. The block's value is its last command's, so the body's tail is
/// the block's own.
fn rewrite_block_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    tail: Tail,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::Block {
        span,
        body,
        namespace,
        tokens,
        error_context,
    } = stmt
    else {
        return None;
    };
    let inner_caller = if !namespace.is_empty() && namespace != "::" {
        namespace.to_owned()
    } else {
        caller_qname.to_owned()
    };
    let (new_body, changed) = rewrite_script(
        body,
        &inner_caller,
        inlinable,
        summaries,
        counter,
        tail,
        caller_may_be_global_frame,
    );
    changed.then(|| {
        vec![Statement::Block {
            span: *span,
            body: new_body,
            namespace: namespace.clone(),
            tokens: tokens.clone(),
            error_context: *error_context,
        }]
    })
}

/// Whether `script` ends in a branch — an `if` or a `switch`.
fn ends_in_a_branch(script: &Script) -> bool {
    matches!(
        script.statements.last(),
        Some(Statement::If { .. } | Statement::Switch { .. })
    )
}

/// [`rewrite_script`] for an arm of an `if` or a `switch`, except that where
/// somebody reads the arm's value, a rewrite that makes a branch of the command
/// the arm ended with is not made. The code generator loses the value of an `if`
/// or a `switch` that ends an arm — `if {$n} {if {$n < 0} {set r neg} else {set r
/// pos}} else {return zero}` answers the empty string — and a splice that gave a
/// call's value to one would answer what the call did not.
fn rewrite_arm(
    body: &Script,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    tail: Tail,
    caller_may_be_global_frame: bool,
) -> (Script, bool) {
    let (rewritten, changed) = rewrite_script(
        body,
        caller_qname,
        inlinable,
        summaries,
        counter,
        tail,
        caller_may_be_global_frame,
    );
    if changed && tail != Tail::Dropped && ends_in_a_branch(&rewritten) && !ends_in_a_branch(body) {
        (body.clone(), false)
    } else {
        (rewritten, changed)
    }
}

/// `If`: recurse into each clause body and the else body, propagating `tail`
/// (the taken branch's value is the `if`'s).
fn rewrite_if_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    tail: Tail,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::If {
        span,
        clauses,
        else_body,
        else_span,
    } = stmt
    else {
        return None;
    };
    let mut new_clauses = Vec::with_capacity(clauses.len());
    let mut changed = false;
    for c in clauses {
        let (body, ch) = rewrite_arm(
            &c.body,
            caller_qname,
            inlinable,
            summaries,
            counter,
            tail,
            caller_may_be_global_frame,
        );
        if ch {
            changed = true;
            new_clauses.push(IfClause {
                condition: c.condition.clone(),
                condition_span: c.condition_span,
                condition_base: c.condition_base,
                body,
                body_span: c.body_span,
            });
        } else {
            new_clauses.push(c.clone());
        }
    }
    let new_else = match else_body {
        Some(b) => {
            let (s, ch) = rewrite_arm(
                b,
                caller_qname,
                inlinable,
                summaries,
                counter,
                tail,
                caller_may_be_global_frame,
            );
            if ch {
                changed = true;
            }
            Some(s)
        }
        None => None,
    };
    changed.then(|| {
        vec![Statement::If {
            span: *span,
            clauses: new_clauses,
            else_body: new_else,
            else_span: *else_span,
        }]
    })
}

/// `For`: recurse into the init / next / body sub-scripts (a loop reads the
/// value of none).
fn rewrite_for_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::For {
        span,
        init,
        init_span,
        condition,
        condition_span,
        condition_base,
        next,
        next_span,
        body,
        body_span,
        raw_args,
        raw_tokens,
    } = stmt
    else {
        return None;
    };
    let (new_init, c1) = rewrite_script(
        init,
        caller_qname,
        inlinable,
        summaries,
        counter,
        Tail::Dropped,
        caller_may_be_global_frame,
    );
    let (new_next, c2) = rewrite_script(
        next,
        caller_qname,
        inlinable,
        summaries,
        counter,
        Tail::Dropped,
        caller_may_be_global_frame,
    );
    let (new_body, c3) = rewrite_script(
        body,
        caller_qname,
        inlinable,
        summaries,
        counter,
        Tail::Dropped,
        caller_may_be_global_frame,
    );
    (c1 || c2 || c3).then(|| {
        vec![Statement::For {
            span: *span,
            init: new_init,
            init_span: *init_span,
            condition: condition.clone(),
            condition_span: *condition_span,
            condition_base: *condition_base,
            next: new_next,
            next_span: *next_span,
            body: new_body,
            body_span: *body_span,
            raw_args: raw_args.clone(),
            raw_tokens: raw_tokens.clone(),
        }]
    })
}

/// `While` / `Foreach` / `Catch`: recurse into the single body, rebuilding the
/// statement when it changed. A loop reads no value of its body, except an
/// `lmap` ([`rewrite_foreach_stmt`]); a `catch` holds the value of its body for
/// its result variable.
fn rewrite_single_body_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    match stmt {
        Statement::While {
            span,
            condition,
            condition_span,
            condition_base,
            body,
            body_span,
            raw_args,
            raw_tokens,
        } => {
            let (new_body, ch) = rewrite_script(
                body,
                caller_qname,
                inlinable,
                summaries,
                counter,
                Tail::Dropped,
                caller_may_be_global_frame,
            );
            ch.then(|| {
                vec![Statement::While {
                    span: *span,
                    condition: condition.clone(),
                    condition_span: *condition_span,
                    condition_base: *condition_base,
                    body: new_body,
                    body_span: *body_span,
                    raw_args: raw_args.clone(),
                    raw_tokens: raw_tokens.clone(),
                }]
            })
        }
        Statement::Foreach { .. } => rewrite_foreach_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            caller_may_be_global_frame,
        ),
        Statement::Catch {
            span,
            body,
            body_span,
            result_var,
            options_var,
            raw_args,
            tokens,
        } => {
            let (new_body, ch) = rewrite_script(
                body,
                caller_qname,
                inlinable,
                summaries,
                counter,
                Tail::Captured,
                caller_may_be_global_frame,
            );
            ch.then(|| {
                vec![Statement::Catch {
                    span: *span,
                    body: new_body,
                    body_span: *body_span,
                    result_var: result_var.clone(),
                    options_var: options_var.clone(),
                    raw_args: raw_args.clone(),
                    tokens: tokens.clone(),
                }]
            })
        }
        _ => None,
    }
}

/// `Foreach` and `lmap`: recurse into the body. A loop reads no value of its body
/// and an `lmap` gathers each iteration's, so its body is rewritten as `Captured`;
/// the collector the code generator gives an `lmap` gathers from a body of one
/// fall-through block, so a body the splice gave a branch or a loop is not given
/// to it.
fn rewrite_foreach_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::Foreach {
        span,
        iterators,
        body,
        body_span,
        is_lmap,
        raw_args,
        is_dict_iteration,
        is_array_iteration,
        raw_tokens,
    } = stmt
    else {
        return None;
    };
    let tail = if *is_lmap {
        Tail::Captured
    } else {
        Tail::Dropped
    };
    let (new_body, changed) = rewrite_script(
        body,
        caller_qname,
        inlinable,
        summaries,
        counter,
        tail,
        caller_may_be_global_frame,
    );
    (changed && (!*is_lmap || new_body.is_straight_line())).then(|| {
        vec![Statement::Foreach {
            span: *span,
            iterators: iterators.clone(),
            body: new_body,
            body_span: *body_span,
            is_lmap: *is_lmap,
            raw_args: raw_args.clone(),
            is_dict_iteration: *is_dict_iteration,
            is_array_iteration: *is_array_iteration,
            raw_tokens: raw_tokens.clone(),
        }]
    })
}

/// `Try`: recurse into the body, each handler body, and the finally body. The
/// value of a `try` is its body's or the handler's that ran, which its handlers
/// and the caller of the `try` may read; the `finally` body's is read by none.
fn rewrite_try_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::Try {
        span,
        body,
        body_span,
        handlers,
        finally_body,
        finally_span,
        raw_args,
    } = stmt
    else {
        return None;
    };
    let (new_body, mut changed) = rewrite_script(
        body,
        caller_qname,
        inlinable,
        summaries,
        counter,
        Tail::Captured,
        caller_may_be_global_frame,
    );
    let mut new_handlers = Vec::with_capacity(handlers.len());
    for h in handlers {
        let (hbody, ch) = rewrite_script(
            &h.body,
            caller_qname,
            inlinable,
            summaries,
            counter,
            Tail::Captured,
            caller_may_be_global_frame,
        );
        if ch {
            changed = true;
            new_handlers.push(TryHandler {
                kind: h.kind,
                match_arg: h.match_arg.clone(),
                trap_pattern: h.trap_pattern.clone(),
                var_name: h.var_name.clone(),
                options_var: h.options_var.clone(),
                body: hbody,
                body_span: h.body_span,
                fallthrough: h.fallthrough,
            });
        } else {
            new_handlers.push(h.clone());
        }
    }
    let new_finally = match finally_body {
        Some(b) => {
            let (s, ch) = rewrite_script(
                b,
                caller_qname,
                inlinable,
                summaries,
                counter,
                Tail::Dropped,
                caller_may_be_global_frame,
            );
            if ch {
                changed = true;
            }
            Some(s)
        }
        None => None,
    };
    changed.then(|| {
        vec![Statement::Try {
            span: *span,
            body: new_body,
            body_span: *body_span,
            handlers: new_handlers,
            finally_body: new_finally,
            finally_span: *finally_span,
            raw_args: raw_args.clone(),
        }]
    })
}

/// `Switch`: recurse into each arm / default body, propagating `tail` (the
/// matched arm's value is the `switch`'s, like `if`).
fn rewrite_switch_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<tcl_compiler::ir::ProcedureId, InlineSpec>,
    summaries: &InliningSummaries<'_>,
    counter: &mut usize,
    tail: Tail,
    caller_may_be_global_frame: bool,
) -> Option<Vec<Statement>> {
    let Statement::Switch {
        span,
        subject,
        subject_braced,
        subject_span,
        arms,
        default_body,
        default_span,
        mode,
        nocase,
        raw_args,
        raw_arg_braced,
        raw_arg_quoted,
        command,
        patterns_braced,
    } = stmt
    else {
        return None;
    };
    let mut new_arms = Vec::with_capacity(arms.len());
    let mut changed = false;
    for a in arms {
        match &a.body {
            Some(b) => {
                let (body, ch) = rewrite_arm(
                    b,
                    caller_qname,
                    inlinable,
                    summaries,
                    counter,
                    tail,
                    caller_may_be_global_frame,
                );
                if ch {
                    changed = true;
                    new_arms.push(SwitchArm {
                        pattern: a.pattern.clone(),
                        pattern_braced: a.pattern_braced,
                        pattern_span: a.pattern_span,
                        body: Some(body),
                        body_span: a.body_span,
                        fallthrough: a.fallthrough,
                    });
                } else {
                    new_arms.push(a.clone());
                }
            }
            None => new_arms.push(a.clone()),
        }
    }
    let new_default = match default_body {
        Some(b) => {
            let (s, ch) = rewrite_arm(
                b,
                caller_qname,
                inlinable,
                summaries,
                counter,
                tail,
                caller_may_be_global_frame,
            );
            if ch {
                changed = true;
            }
            Some(s)
        }
        None => None,
    };
    changed.then(|| {
        vec![Statement::Switch {
            subject_braced: *subject_braced,
            raw_arg_braced: raw_arg_braced.clone(),
            raw_arg_quoted: raw_arg_quoted.clone(),
            command: command.clone(),
            span: *span,
            subject: subject.clone(),
            subject_span: *subject_span,
            arms: new_arms,
            default_body: new_default,
            default_span: *default_span,
            mode: *mode,
            nocase: *nocase,
            raw_args: raw_args.clone(),
            patterns_braced: *patterns_braced,
        }]
    })
}
