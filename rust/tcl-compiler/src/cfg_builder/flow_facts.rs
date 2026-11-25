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

//! Definite-assignment and completion facts over un-lowered IR scripts, shared with SSA for opaque `switch`.

use super::{CfgCommandClasses, MAX_LOWER_DEPTH};
use crate::ir::{Script, Statement};
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;

// Definite-assignment ("flow facts") over un-lowered IR scripts
//
// Shared by the CFG builder (to promote an opaque `switch` whose every arm
// exits the *procedure* — FP-RBS-15 — and to wire its loop-jump edges) and
// by SSA (to recover the def set an opaque switch *definitely* establishes —
// FP-RBS-14).  Both consumers agree on one model.
//
// Each statement/script is classified by how it leaves: it can complete NORMALly
// (fall through to the next statement), jump within an enclosing loop (LoopJump
// — `break`/`continue`), or exit the procedure (ProcExit —
// `return`/`error`/`throw`/`exit`/`tailcall`).  The distinction matters for an
// *opaque* switch (whose arm bodies are not lowered into the CFG, so their
// loop-jump edges are invisible): a ProcExit arm reaches no later code at all,
// but a LoopJump arm still reaches the code after the enclosing loop *without*
// the other arms' definitions — so it must NOT be treated as vacuous when
// recovering the switch's defs, and an all-LoopJump switch is NOT a procedure
// terminator.

/// How a statement or script leaves, as the completion codes it may produce:
/// `TCL_OK` falls through, `break`/`continue` jump to an enclosing loop
/// target, and any other code (or none, for a process exit) leaves the
/// procedure.
pub(crate) type Completion = crate::completion::CompletionCodeLattice;

fn normal() -> Completion {
    Completion::exact(tcl_registry::CompletionCode::Ok)
}

/// Whether `stmt` certainly ends the interpreter, running no enclosing
/// `finally`: a process-terminating command (`Traits::TERMINATES_PROCESS`,
/// e.g. `exit`) that nothing can stop from running.
///
/// [`Completion`] classes `exit` with `return` and `error` as leaving the procedure,
/// which is right for what follows the statement and wrong for an enclosing
/// `finally`: a `return` runs it, an `exit` does not (tclsh 8.6.18 and 9.0.4:
/// `try {exit 7} finally {puts FINALLY}` prints nothing).
///
/// "Nothing can stop it" is the whole difficulty, and review found it three
/// ways: an earlier statement may `return`; the command's own words may throw
/// before it runs (`exit [error boom]` runs the clause); and so may a literal
/// word it rejects (`exit abc` raises "expected integer"; `exit 09` does in
/// 8.x). The same holds for an `if` condition or a `switch` subject in front
/// of an all-`exit` body — `switch -glob $nosuch {…}` throws on the unset
/// variable. So this accepts only the command itself, with every word
/// literal and a status the registry says it accepts, and an enclosing
/// construct is never looked through. Missing a real exit
/// costs an O107; accepting a false one rewrote a live program.
/// The completion `stmt` certainly produces, when the registry can say:
/// every word literal, the call resolved by the command-binding owner to one
/// registry-backed target, and that target's alias prefix joined to the
/// written words before the registry classifies the composed invocation.
///
/// Every word must be literal because a substituted one runs first and may
/// throw. The words must be *all* the words: `interp alias {} bye {} exit abc`
/// — or the same alias named `::foo::exit` and called as `exit` inside
/// `::foo` — raises "expected integer" (found in review). Whether a literal
/// is a status the command accepts is the registry's release-aware answer:
/// `exit 09` is an invalid octal in 8.x but status 9 in 9.0.
pub(super) fn exact_statement_completion(
    stmt: &Statement,
    registry: &CommandRegistry,
    resolve: &dyn Fn(&str) -> Option<crate::ir_helpers::ResolvedEmbeddedHead>,
    query: Option<tcl_dialect::model::SurfaceQuery<'_>>,
) -> Option<tcl_registry::registry::ExactInvocationCompletion> {
    let (Statement::Call {
        command, tokens, ..
    }
    | Statement::Barrier {
        command, tokens, ..
    }) = stmt
    else {
        return None;
    };
    let tokens = tokens.as_ref()?;
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    let config = dialect.map_or_else(tcl_lexer::LexerConfig::default, |dialect| {
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar)
    });
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let literal =
        |word: &crate::ir::WordExpr| match crate::registry_invocation::effective_invocation_word(
            word,
            config.escapes,
            rules,
        ) {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => Some(value),
            _ => None,
        };
    // Words execute before dispatch. An exact command completion cannot hide
    // an error from a substituted head or argument.
    tokens
        .words()
        .iter()
        .map(literal)
        .collect::<Option<Vec<_>>>()?;
    let (target, values) = if tokens.source_binding.is_some() {
        let target = tokens
            .source_binding
            .as_ref()?
            .proved_execution_target()?
            .clone();
        if !target.registry_backed {
            return None;
        }
        let effective = crate::registry_invocation::effective_command_words(tokens)?;
        let values = effective
            .words
            .iter()
            .skip(1)
            .map(literal)
            .collect::<Option<Vec<_>>>()?;
        (target.command, values)
    } else {
        let target = resolve(command)?;
        let values = target
            .prepended
            .into_iter()
            .chain(
                tokens
                    .words()
                    .iter()
                    .skip(1)
                    .map(literal)
                    .collect::<Option<Vec<_>>>()?,
            )
            .collect();
        (target.command, values)
    };
    let words: Vec<_> = values.iter().map(String::as_str).collect();
    let mut arguments = tcl_registry::InvocationArguments::literals(&words);
    if let Some(dialect) = dialect {
        arguments = arguments.with_dialect(dialect);
    }
    registry.exact_invocation_completion_words(&target, arguments, query)
}

pub(super) fn structural_write_roots(stmt: &Statement) -> Option<Vec<String>> {
    let projection = crate::ir_helpers::structural_variable_write_projection(stmt);
    if projection.opaque_variable_frame {
        return None;
    }
    Some(
        projection
            .literal_names
            .into_iter()
            .map(|name| {
                tcl_syntax::naming::split_element_ref(&name)
                    .map_or(name.as_str(), |(root, _)| root)
                    .to_owned()
            })
            .collect(),
    )
}

/// `(must-defines, completion)` for a single statement.
///
/// * assignments (`set`/`incr`/`expr`-assign) contribute their target and
///   complete normally; a plain `Call` contributes its synthetic `defs` and
///   completes normally — *except* `error`/`throw`/`exit` and `tailcall`, which
///   `ProcExit`, and `break`/`continue`, which `LoopJump`; `Return` and a
///   `return -code` / expansion `Barrier` `ProcExit`;
/// * `Block` / `UpFrame` bodies always run, so recurse;
/// * an `If` *with* an else, or a `Switch` *with* a default, combine their
///   branches via [`intersect_completing`]; an else-less `If` / default-less
///   `Switch` has a fall-through path, so it completes normally assigning
///   nothing for certain;
/// * loops (`For`/`While`/`Foreach`), `Catch`/`Try` and everything else may not
///   execute / always complete (`catch` even swallows `break`), so they
///   contribute no must-define and complete normally.
pub(super) fn flow_facts_stmt_with_classes(
    stmt: &Statement,
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    match stmt {
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => (
            structural_write_roots(stmt)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            normal(),
        ),
        Statement::Return { .. } => (
            BTreeSet::new(),
            Completion::exact(tcl_registry::CompletionCode::Return),
        ),
        Statement::Barrier { .. } => (BTreeSet::new(), command_classes.flow_completion(stmt)),
        Statement::Call { defs, .. } => (
            defs.iter().cloned().collect(),
            command_classes.flow_completion(stmt),
        ),
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
            flow_facts_script_with_classes(body, command_classes)
        }
        Statement::If {
            clauses,
            else_body: Some(eb),
            ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            bodies.push(eb);
            intersect_completing(&bodies, command_classes)
        }
        Statement::Switch {
            arms,
            default_body: Some(default),
            ..
        } => {
            let mut bodies: Vec<&Script> = vec![default];
            bodies.extend(arms.iter().filter_map(|arm| arm.body.as_ref()));
            intersect_completing(&bodies, command_classes)
        }
        // An else-less `If` / default-less `Switch` has a fall-through path that
        // assigns nothing for certain; everything else may not run / always
        // completes.
        _ => (BTreeSet::new(), normal()),
    }
}

/// Profile-less compatibility entry point. Module CFG construction uses the
/// release-aware `*_with_classes` path owned by its [`CfgBuilder`].
#[cfg(test)]
pub(crate) fn flow_facts_stmt(stmt: &Statement) -> (BTreeSet<String>, Completion) {
    flow_facts_stmt_with_classes(stmt, &CfgCommandClasses::default())
}

/// `(vars definitely assigned, completion)` for an un-lowered IR script.
///
/// Walk statements in order accumulating must-defines; the first statement that
/// does not complete normally makes the rest dead and gives the script that
/// statement's completion (leaving the procedure or jumping to a loop), with the must-defines
/// being those accumulated up to (and including) it.  Only names guaranteed to
/// be assigned are returned, so it never over-claims (which would hide a real
/// read-before-set).
fn flow_facts_script_with_classes(
    script: &Script,
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    let mut assigned = BTreeSet::new();
    for stmt in &script.statements {
        let (defs, completion) = flow_facts_stmt_with_classes(stmt, command_classes);
        assigned.extend(defs);
        if !completion.may_complete_normally() {
            return (assigned, completion);
        }
    }
    (assigned, normal())
}

/// Combine branch bodies into `(must-defines, completion)`.
///
/// A branch that leaves the procedure reaches no code after the construct, so
/// it is vacuous (⊤) and excluded from the must-define intersection.  A branch
/// that falls through or jumps to a loop *does* reach later code (the fall-through successor, or —
/// for a loop jump — the code after the enclosing loop), so its must-defines
/// (those established before it leaves) ARE intersected: this keeps the result
/// sound when a `break`/`continue` arm escapes an opaque switch without the
/// other arms' defs.  The combined completion is the join of the branches'.
fn intersect_completing(
    bodies: &[&Script],
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    let mut common: Option<BTreeSet<String>> = None;
    let mut combined = Completion::bottom();
    for body in bodies {
        let (assigned, completion) = flow_facts_script_with_classes(body, command_classes);
        combined = combined.join(&completion);
        if completion.leaves_procedure() {
            continue;
        }
        common = Some(match common {
            None => assigned,
            Some(acc) => acc.intersection(&assigned).cloned().collect(),
        });
    }
    (common.unwrap_or_default(), combined)
}

/// Variables an opaque `switch` assigns on *every* non-proc-exit path.
///
/// Empty unless the switch has a `default` arm (otherwise an unmatched subject
/// falls through assigning nothing).  Intersection over the default and every
/// arm-with-a-body, excluding only arms that exit the procedure (which reach no
/// later code).  A `break`/`continue` arm is *kept* (with the defs it makes
/// before jumping), because it still reaches the code after the enclosing loop —
/// so an arm that breaks without assigning `y` correctly drops `y` rather than
/// letting it be claimed defined.
pub(crate) fn switch_must_defines(
    stmt: &Statement,
    registry: Option<&CommandRegistry>,
) -> BTreeSet<String> {
    let command_classes = registry.map_or_else(CfgCommandClasses::default, |registry| {
        CfgCommandClasses::from_registry(registry)
    });
    flow_facts_stmt_with_classes(stmt, &command_classes).0
}

/// Must-defined source names under the supplied complete availability owner.
/// Missing input keeps completion metadata unavailable; it does not select the
/// standalone catalogue. Original structural writes retain their IR ownership.
pub(crate) fn switch_must_defines_with_source_input(
    stmt: &Statement,
    registry: &CommandRegistry,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> BTreeSet<String> {
    if input.is_none() {
        return BTreeSet::new();
    }
    let command_classes = CfgCommandClasses::from_source_input(registry, input);
    if command_classes.metadata_context().is_none() {
        return BTreeSet::new();
    }
    flow_facts_stmt_with_classes(stmt, &command_classes).0
}

/// `(can_break, can_continue)` for `break`/`continue` that escape *script* to an
/// enclosing loop.
///
/// Recurses into `if`/`switch`/`Block`/`UpFrame` bodies (which don't capture a
/// loop jump) and into `try` — a `try` does NOT capture `break`/`continue`:
/// without a matching `on`/`trap` handler a jump in the body, a handler body, or
/// the `finally` body propagates to the enclosing loop (confirmed in tclsh
/// 9.0.3, contrast `catch {break}` which yields code 3 and is absorbed).  Does
/// NOT recurse into nested loops (`for`/`while`/`foreach`) or `catch`, which
/// capture their own jumps.  Scanning stops at the first statement that cannot
/// complete normally (`return`/`error`/`exit`/`tailcall`, or `break`/`continue`
/// itself): a jump after it is dead code that never executes, so it must not
/// create a spurious loop-exit edge.
/// `depth` is the nesting level of `script` — reuses [`MAX_LOWER_DEPTH`]
/// (this walk is mutually recursive with [`switch_escaping_jumps`], and
/// both are transitively bounded today via the same cap `lower_script`
/// already builds every `Script` under, so this is defence-in-depth /
/// consistency with that cap rather than a currently-reachable path).
fn escaping_loop_jumps_with_classes(
    script: &Script,
    depth: u32,
    command_classes: &CfgCommandClasses,
) -> (bool, bool) {
    if MAX_LOWER_DEPTH.exceeded(depth) {
        return (false, false);
    }
    let mut can_break = false;
    let mut can_continue = false;
    for stmt in &script.statements {
        // An opaque arm is conditional source, so it need not have an actual
        // handler visit. Its retained source schema can still establish the
        // completion when that arm runs, without granting execution authority.
        let completion = command_classes
            .completion_route(stmt)
            .or_else(|| command_classes.conditional_source_completion_route(stmt.tokens()?));
        match stmt {
            Statement::Call { .. } => {
                if let Some(route) = &completion {
                    for route in route.alternatives() {
                        match route {
                            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                                tcl_registry::CompletionCode::Break,
                            ) => can_break = true,
                            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                                tcl_registry::CompletionCode::Continue,
                            ) => can_continue = true,
                            _ => {}
                        }
                    }
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    let (b, c) =
                        escaping_loop_jumps_with_classes(&clause.body, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(eb) = else_body {
                    let (b, c) = escaping_loop_jumps_with_classes(eb, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            Statement::Switch { .. } => {
                let (b, c) = switch_escaping_jumps_with_classes(stmt, depth, command_classes);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
                can_break |= b;
                can_continue |= c;
                for h in handlers {
                    let (b, c) =
                        escaping_loop_jumps_with_classes(&h.body, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(fb) = finally_body {
                    let (b, c) = escaping_loop_jumps_with_classes(fb, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            _ => {}
        }
        // A statement that cannot complete normally makes everything after it
        // dead code (a later `break`/`continue` never runs), so stop scanning —
        // otherwise a dead `break` after `error`/`return` would forge a
        // loop-exit edge and fire W210 on unreachable post-loop code.
        if completion.is_some_and(|route| !route.normal_possible())
            || !flow_facts_stmt_with_classes(stmt, command_classes)
                .1
                .may_complete_normally()
        {
            break;
        }
    }
    (can_break, can_continue)
}

/// Profile-less compatibility entry point. Module CFG construction uses the
/// release-aware `*_with_classes` path owned by its [`CfgBuilder`].
#[cfg(test)]
pub(crate) fn escaping_loop_jumps(script: &Script, depth: u32) -> (bool, bool) {
    escaping_loop_jumps_with_classes(script, depth, &CfgCommandClasses::default())
}

/// `(can_break, can_continue)` over all bodies of an opaque `switch`.
/// `depth` is `stmt`'s own nesting level — see
/// [`escaping_loop_jumps_with_classes`].
pub(super) fn switch_escaping_jumps_with_classes(
    stmt: &Statement,
    depth: u32,
    command_classes: &CfgCommandClasses,
) -> (bool, bool) {
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return (false, false);
    };
    let mut can_break = false;
    let mut can_continue = false;
    if let Some(default) = default_body {
        let (b, c) = escaping_loop_jumps_with_classes(default, depth + 1, command_classes);
        can_break |= b;
        can_continue |= c;
    }
    for arm in arms {
        if let Some(body) = &arm.body {
            let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
            can_break |= b;
            can_continue |= c;
        }
    }
    (can_break, can_continue)
}
