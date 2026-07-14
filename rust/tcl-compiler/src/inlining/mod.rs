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

//! General proc inliner (v0 + verbatim + v3 parameterised).
//!
//! Combines the decision catalogue, the IR-level splice transform, and
//! the α-rename machinery (here [`rename`]). The inliner rewrites
//! statement-position calls to *inlinable* procedures so the call
//! boundary disappears before WASM codegen consumes the IR.
//!
//! # Scope
//!
//! Four call shapes are handled:
//!
//! * **v0 — empty body.** A call to a zero-statement proc vanishes.
//! * **v1 / v2 — verbatim wrapper.** A zero-parameter proc whose every
//!   body statement is *splice-eligible* (a frame-independent, def-free,
//!   substitution-free builtin call) has its body spliced verbatim.
//! * **v3 — parameterised.** Procs with parameters and / or local writes.
//!   Every parameter and locally-written variable is α-renamed to a fresh
//!   `__inline_<id>__<name>` slot ([`rename`]), call-site args are bound
//!   to the mangled parameter slots (honouring defaults, variadic `args`,
//!   and braced-literal args), and a non-trailing `return` is lowered to
//!   `set __RESULT <v>; break` wrapped in a one-shot `while {1} { … }`
//!   loop so it doesn't short-circuit the caller.
//!
//! # Consumer
//!
//! Inlining is a pre-codegen IR transform — it dissolves call boundaries
//! so the backend emits flatter code — so its only consumer is the WASM
//! codegen. The LSP and CLI analysis paths report on the program *as
//! written* and never lower to codegen, so they deliberately do not run
//! the inliner; wiring it in is owned by the codegen consumer. It is thus
//! exposed but unwired, with the IR-shape unit tests here as its current
//! verification.
//!
//! # Soundness
//!
//! Two gates combine in [`build_inlinable_map`]: the precise
//! interprocedural [`var_escape::pure_leaf`](crate::var_escape) proof
//! (`safe_to_inline`, the soundness predicate) and the
//! structural shape. Redefined procs are never inlined (their body is
//! ambiguous). Recursion cannot loop: the splice replaces a single static
//! call site, and the rename gives every inline a fresh slot namespace.
//! v3 declines a proc whose `return` sits inside a loop / `catch` / `try`
//! / `uplevel` body (where our `break`-based early-return lowering would
//! be trapped) and any call site using `{*}` expansion (runtime arity).

mod rename;

use std::collections::{HashMap, HashSet};

use tcl_lexer::{Span, TokenType};

use crate::expr_ast::ExprNode;
use crate::ir::{
    CommandTokens, IfClause, Module, Procedure, Script, Statement, SwitchArm, TryHandler,
};
use crate::var_escape::ProcEscapeSummary;
use crate::var_escape::interprocedural::resolve_callee;
use tcl_registry::CommandRegistry;

/// Per-proc inlining policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDecision {
    /// Always inline a call to this proc (small, pure-leaf body).
    Always,
    /// Inline only when the proc has exactly one static caller. Recorded
    /// for future pruning; not acted on by the splice (the
    /// post-inline proc-pruning interaction is not yet implemented).
    IfSingleCall,
    /// Never inline.
    Never,
}

/// A pure-leaf proc whose body has at most this many flat statements is
/// unconditionally inlinable.
pub const SMALL_BODY_THRESHOLD: usize = 5;

/// What an inlinable proc splices in at a call site.
#[derive(Debug, Clone)]
enum InlineSpec {
    /// v0 — empty body; the call vanishes.
    Empty,
    /// v1 / v2 — splice these body statements verbatim.
    Verbatim(Vec<Statement>),
    /// v3 — parameterised; rewrite per call site from this proc.
    Parameterised(Procedure),
}

// statement counting

/// Total number of leaf statements reachable in `script`, walking nested
/// control-flow bodies transitively (the inlined code-size cost).
#[must_use]
pub fn count_statements(script: &Script) -> usize {
    script.statements.iter().map(count_one).sum()
}

fn count_one(stmt: &Statement) -> usize {
    match stmt {
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => count_statements(body),
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut n = 0;
            for c in clauses {
                n += 1 + count_statements(&c.body);
            }
            if let Some(b) = else_body {
                n += count_statements(b);
            }
            n
        }
        Statement::For {
            init, next, body, ..
        } => count_statements(init) + 1 + count_statements(next) + count_statements(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => 1 + count_statements(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut n = count_statements(body);
            for h in handlers {
                n += count_statements(&h.body);
            }
            if let Some(fb) = finally_body {
                n += count_statements(fb);
            }
            n
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut n = 0;
            for a in arms {
                if let Some(b) = &a.body {
                    n += count_statements(b);
                }
            }
            if let Some(b) = default_body {
                n += count_statements(b);
            }
            n
        }
        _ => 1,
    }
}

// static call counting

/// Return `{qname: static_call_count}` over the whole module. Walks the
/// top-level script and every proc body; an [`Statement::Call`] whose
/// command resolves (per the same namespace-walk rules the
/// interprocedural pass uses) to a known proc contributes `+1`.
fn count_static_calls(
    module: &Module,
    summaries: &HashMap<String, ProcEscapeSummary>,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> =
        module.procedures.keys().map(|q| (q.clone(), 0)).collect();
    tally_calls(&module.top_level, "::", "::", summaries, &mut counts);
    for (qname, proc) in &module.procedures {
        let root_ns = root_namespace_of(qname);
        tally_calls(&proc.body, &root_ns, qname, summaries, &mut counts);
    }
    counts
}

/// The namespace a proc body's unqualified calls resolve against —
/// `::ns::helper` → `::ns`, bare/global → `::`.
fn root_namespace_of(qname: &str) -> String {
    let tail = qname.strip_prefix("::").unwrap_or(qname);
    if let Some(idx) = tail.rfind("::") {
        let root = &qname[..qname.len() - (tail.len() - idx)];
        if root.is_empty() {
            "::".to_owned()
        } else {
            root.to_owned()
        }
    } else {
        "::".to_owned()
    }
}

fn tally_calls(
    script: &Script,
    ns: &str,
    caller_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counts: &mut HashMap<String, usize>,
) {
    for stmt in &script.statements {
        match stmt {
            Statement::Call { command, .. } => {
                let mut target = resolve_callee(command, ns, summaries);
                if target.is_none() && ns != caller_qname {
                    target = resolve_callee(command, caller_qname, summaries);
                }
                if let Some(t) = target
                    && let Some(c) = counts.get_mut(&t)
                {
                    *c += 1;
                }
            }
            Statement::Block {
                body, namespace, ..
            } => {
                let inner = if namespace.is_empty() {
                    "::"
                } else {
                    namespace
                };
                tally_calls(body, inner, caller_qname, summaries, counts);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    tally_calls(&c.body, ns, caller_qname, summaries, counts);
                }
                if let Some(b) = else_body {
                    tally_calls(b, ns, caller_qname, summaries, counts);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                tally_calls(init, ns, caller_qname, summaries, counts);
                tally_calls(next, ns, caller_qname, summaries, counts);
                tally_calls(body, ns, caller_qname, summaries, counts);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::UpFrame { body, .. } => {
                tally_calls(body, ns, caller_qname, summaries, counts);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                tally_calls(body, ns, caller_qname, summaries, counts);
                for h in handlers {
                    tally_calls(&h.body, ns, caller_qname, summaries, counts);
                }
                if let Some(fb) = finally_body {
                    tally_calls(fb, ns, caller_qname, summaries, counts);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        tally_calls(b, ns, caller_qname, summaries, counts);
                    }
                }
                if let Some(b) = default_body {
                    tally_calls(b, ns, caller_qname, summaries, counts);
                }
            }
            _ => {}
        }
    }
}

// policy

/// Apply the inlining policy to a single proc. A proc is inlinable only if
/// its escape summary is pure-leaf (`safe_to_inline`); among that set,
/// size and static-call-count decide between [`InlineDecision::Always`]
/// (small enough that growth is bounded), [`InlineDecision::IfSingleCall`]
/// (size-neutral), and [`InlineDecision::Never`].
#[must_use]
pub fn classify_proc(
    proc: &Procedure,
    summary: Option<&ProcEscapeSummary>,
    static_call_count: usize,
) -> InlineDecision {
    if !summary.is_some_and(ProcEscapeSummary::safe_to_inline) {
        return InlineDecision::Never;
    }
    let body_size = count_statements(&proc.body);
    if body_size <= SMALL_BODY_THRESHOLD {
        InlineDecision::Always
    } else if static_call_count == 1 {
        InlineDecision::IfSingleCall
    } else {
        InlineDecision::Never
    }
}

// splice-eligibility

/// Whether `command` is a frame-independent, splice-safe builtin — a
/// wrapped call to one can be spliced into any caller's frame.  Membership
/// is the registry's [`CommandRegistry::is_splice_safe`] classification
/// (fully-lowered `FRAMELESS_RUNTIME` commands minus the frame-observing /
/// frame-affecting / variable-name-operating traits), so frame-observing
/// (`info` / `uplevel` / `upvar`) and frame-affecting control flow
/// (`return` / `break` / `continue`) stay excluded without a name list
/// here.
fn command_is_splice_safe(command: &str, registry: &CommandRegistry) -> bool {
    registry.is_splice_safe(command)
}

/// True iff `arg` contains a `[cmd …]` command substitution at brace
/// depth zero (brace-aware, backslash-aware). False positives just
/// decline a hoist, never inline an unsafe one.
fn arg_has_command_subst(arg: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = arg.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '{' => depth += 1,
            '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            '[' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// True when `command` resolves the same way from any caller's namespace
/// as it does from the callee's: it is fully qualified, or it is
/// unqualified and doesn't resolve to any known proc (so it's a global
/// builtin).
fn command_is_namespace_invariant(
    command: &str,
    callee_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
) -> bool {
    if command.starts_with("::") {
        return true;
    }
    resolve_callee(command, callee_qname, summaries).is_none()
}

/// True iff `stmt` can be lifted out of the wrapper proc and spliced into
/// a caller without changing observable behaviour.
fn stmt_is_splice_eligible(
    stmt: &Statement,
    callee_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
    registry: &CommandRegistry,
) -> bool {
    let Statement::Call {
        command,
        args,
        defs,
        ..
    } = stmt
    else {
        return false;
    };
    if !defs.is_empty() {
        return false;
    }
    if !command_is_namespace_invariant(command, callee_qname, summaries) {
        return false;
    }
    if !command_is_splice_safe(command, registry) {
        return false;
    }
    if args.iter().any(|a| arg_has_command_subst(a)) {
        return false;
    }
    true
}

// v3 eligibility

/// Return true iff `proc` qualifies for parameterised (v3) inlining.
fn v3_eligible(
    proc: &Procedure,
    qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
    registry: &CommandRegistry,
) -> bool {
    if has_irreturn_in_unsafe_scope(&proc.body, false) {
        return false;
    }
    for stmt in &proc.body.statements {
        if matches!(stmt, Statement::Return { .. }) {
            continue; // top-level return is fine — handled by wrap or kept
        }
        if !v3_stmt_eligible(stmt, qname, summaries, registry) {
            return false;
        }
    }
    true
}

/// Walk `script` looking for any [`Statement::Return`] inside a construct
/// whose `break` semantics would trap our wrapper's break (loop bodies,
/// `catch` / `try` traps, `uplevel` frame-shifts). Transparent groupings
/// (`Block`, `if` clauses, `switch` arms) propagate the parent flag.
fn has_irreturn_in_unsafe_scope(script: &Script, inside_unsafe: bool) -> bool {
    for stmt in &script.statements {
        match stmt {
            Statement::Return { .. } => {
                if inside_unsafe {
                    return true;
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                if has_irreturn_in_unsafe_scope(init, inside_unsafe)
                    || has_irreturn_in_unsafe_scope(next, inside_unsafe)
                    || has_irreturn_in_unsafe_scope(body, true)
                {
                    return true;
                }
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::UpFrame { body, .. } => {
                if has_irreturn_in_unsafe_scope(body, true) {
                    return true;
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                if has_irreturn_in_unsafe_scope(body, true) {
                    return true;
                }
                for h in handlers {
                    if has_irreturn_in_unsafe_scope(&h.body, true) {
                        return true;
                    }
                }
                if let Some(fb) = finally_body
                    && has_irreturn_in_unsafe_scope(fb, true)
                {
                    return true;
                }
            }
            Statement::Block { body, .. } => {
                if has_irreturn_in_unsafe_scope(body, inside_unsafe) {
                    return true;
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    if has_irreturn_in_unsafe_scope(&c.body, inside_unsafe) {
                        return true;
                    }
                }
                if let Some(b) = else_body
                    && has_irreturn_in_unsafe_scope(b, inside_unsafe)
                {
                    return true;
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body
                        && has_irreturn_in_unsafe_scope(b, inside_unsafe)
                    {
                        return true;
                    }
                }
                if let Some(b) = default_body
                    && has_irreturn_in_unsafe_scope(b, inside_unsafe)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Per-statement eligibility for v3 inlining.
fn v3_stmt_eligible(
    stmt: &Statement,
    callee_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
    registry: &CommandRegistry,
) -> bool {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            // `::`-qualified names (globals) decline — the rename map only
            // covers locals. Array writes (`arr(idx)`) qualify.
            !name.contains("::")
        }
        Statement::Call { .. } => stmt_is_splice_eligible(stmt, callee_qname, summaries, registry),
        Statement::ExprEval { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .all(|c| v3_script_eligible(&c.body, callee_qname, summaries, registry))
                && else_body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, registry))
        }
        Statement::For {
            init, next, body, ..
        } => {
            v3_script_eligible(init, callee_qname, summaries, registry)
                && v3_script_eligible(next, callee_qname, summaries, registry)
                && v3_script_eligible(body, callee_qname, summaries, registry)
        }
        Statement::Block { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::UpFrame { body, .. } => {
            v3_script_eligible(body, callee_qname, summaries, registry)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            v3_script_eligible(body, callee_qname, summaries, registry)
                && handlers
                    .iter()
                    .all(|h| v3_script_eligible(&h.body, callee_qname, summaries, registry))
                && finally_body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, registry))
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().all(|a| {
                a.body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, registry))
            }) && default_body
                .as_ref()
                .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, registry))
        }
        // Return is handled by the caller; Barrier and anything else decline.
        _ => false,
    }
}

/// Recurse into a nested script and check per-statement eligibility.
/// `return` inside a nested script IS allowed (the for/break wrap routes
/// it).
fn v3_script_eligible(
    script: &Script,
    callee_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary>,
    registry: &CommandRegistry,
) -> bool {
    for stmt in &script.statements {
        if matches!(stmt, Statement::Return { .. }) {
            continue;
        }
        if !v3_stmt_eligible(stmt, callee_qname, summaries, registry) {
            return false;
        }
    }
    true
}

// catalogue → inlinable map

/// Build the qname → [`InlineSpec`] map of inlinable procedures. Only
/// [`InlineDecision::Always`] procs qualify; `IfSingleCall` is not
/// eligible, because its profitability depends on post-inline pruning
/// this pass does not perform. Redefined procs are never inlined.
fn build_inlinable_map(
    module: &Module,
    summaries: &HashMap<String, ProcEscapeSummary>,
    registry: &CommandRegistry,
) -> HashMap<String, InlineSpec> {
    let counts = count_static_calls(module, summaries);
    let mut map = HashMap::new();
    for (qname, proc) in &module.procedures {
        if module.redefined_procedures.contains(qname) {
            continue;
        }
        let count = counts.get(qname).copied().unwrap_or(0);
        if classify_proc(proc, summaries.get(qname), count) != InlineDecision::Always {
            continue;
        }

        // v0 — empty body.
        if proc.body.statements.is_empty() {
            map.insert(qname.clone(), InlineSpec::Empty);
            continue;
        }

        // v1 / v2 — zero-param wrapper, every body statement splice-safe.
        if proc.params.is_empty()
            && proc
                .body
                .statements
                .iter()
                .all(|s| stmt_is_splice_eligible(s, qname, summaries, registry))
        {
            map.insert(
                qname.clone(),
                InlineSpec::Verbatim(proc.body.statements.clone()),
            );
            continue;
        }

        // v3 — parameterised inline with α-renaming.
        if v3_eligible(proc, qname, summaries, registry) {
            map.insert(qname.clone(), InlineSpec::Parameterised(proc.clone()));
        }
    }
    map
}

// the pass

/// Inline eligible statement-position calls throughout `module`,
/// returning the rewritten module. Idempotent: a module with no remaining
/// inlinable sites is returned structurally unchanged.
///
/// Dead-proc elimination is intentionally omitted: the [`Procedure`] type
/// carries no `compiler_synthetic` flag to key it on. User procs stay in
/// the module so host eval / `info procs` / `rename` observers still see
/// them.
#[must_use]
pub fn inline_module(mut module: Module, registry: &CommandRegistry) -> Module {
    let summaries = crate::var_escape::analyse_var_escape(&module, true);
    let inlinable = build_inlinable_map(&module, &summaries, registry);
    if inlinable.is_empty() {
        return module;
    }

    let mut counter: usize = 0;
    let (new_top, _) = rewrite_script(
        &module.top_level,
        "::",
        &inlinable,
        &summaries,
        &mut counter,
        true,
    );
    module.top_level = new_top;

    let mut procedures = std::mem::take(&mut module.procedures);
    for (qname, proc) in &mut procedures {
        let (new_body, changed) = rewrite_script(
            &proc.body,
            qname,
            &inlinable,
            &summaries,
            &mut counter,
            true,
        );
        if changed {
            proc.body = new_body;
        }
    }
    module.procedures = procedures;
    module
}

/// Rewrite a script, returning `(new_script, changed)`. `parent_is_terminal`
/// is true only when this script sits in terminal position of its own
/// enclosing structure (a proc body / the top-level script); it propagates
/// to the LAST statement so a v3 inline of a proc with a trailing `return`
/// can keep the return intact.
fn rewrite_script(
    script: &Script,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
    parent_is_terminal: bool,
) -> (Script, bool) {
    let mut out: Vec<Statement> = Vec::with_capacity(script.statements.len());
    let mut changed = false;
    let n = script.statements.len();
    for (i, stmt) in script.statements.iter().enumerate() {
        let is_terminal = parent_is_terminal && i == n - 1;
        match rewrite_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            is_terminal,
        ) {
            None => out.push(stmt.clone()),
            Some(repl) => {
                changed = true;
                out.extend(repl);
            }
        }
    }
    (Script { statements: out }, changed)
}

/// Return a replacement statement list, or `None` to keep `stmt`. `[]`
/// drops the statement (v0); a non-empty list substitutes the inlined
/// body. Recursion into nested control flow uses
/// `parent_is_terminal = false` (terminality applies only at a body's
/// direct top level).
fn rewrite_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
    is_terminal: bool,
) -> Option<Vec<Statement>> {
    match stmt {
        Statement::Call { command, .. } => {
            let target = resolve_callee(command, caller_qname, summaries)?;
            let spec = inlinable.get(&target)?;
            splice_call_site(stmt, spec, counter, is_terminal)
        }
        Statement::Block { .. } => {
            rewrite_block_stmt(stmt, caller_qname, inlinable, summaries, counter)
        }
        Statement::If { .. } => rewrite_if_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            is_terminal,
        ),
        Statement::For { .. } => {
            rewrite_for_stmt(stmt, caller_qname, inlinable, summaries, counter)
        }
        Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::UpFrame { .. }
        | Statement::Catch { .. } => {
            rewrite_single_body_stmt(stmt, caller_qname, inlinable, summaries, counter)
        }
        Statement::Try { .. } => {
            rewrite_try_stmt(stmt, caller_qname, inlinable, summaries, counter)
        }
        Statement::Switch { .. } => rewrite_switch_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            is_terminal,
        ),
        _ => None,
    }
}

/// `Block`: recurse into the body under the block's (possibly namespaced)
/// caller name.
fn rewrite_block_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
) -> Option<Vec<Statement>> {
    let Statement::Block {
        span,
        body,
        namespace,
        tokens,
    } = stmt
    else {
        return None;
    };
    let inner_caller = if !namespace.is_empty() && namespace != "::" {
        format!("{namespace}::__inblock__")
    } else {
        caller_qname.to_owned()
    };
    let (new_body, changed) =
        rewrite_script(body, &inner_caller, inlinable, summaries, counter, false);
    changed.then(|| {
        vec![Statement::Block {
            span: *span,
            body: new_body,
            namespace: namespace.clone(),
            tokens: tokens.clone(),
        }]
    })
}

/// `If`: recurse into each clause body and the else body, propagating
/// `is_terminal` (a taken branch is itself a tail position).
fn rewrite_if_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
    is_terminal: bool,
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
        let (body, ch) = rewrite_script(
            &c.body,
            caller_qname,
            inlinable,
            summaries,
            counter,
            is_terminal,
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
            let (s, ch) =
                rewrite_script(b, caller_qname, inlinable, summaries, counter, is_terminal);
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

/// `For`: recurse into the init / next / body sub-scripts (none is a tail
/// position).
fn rewrite_for_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
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
    let (new_init, c1) = rewrite_script(init, caller_qname, inlinable, summaries, counter, false);
    let (new_next, c2) = rewrite_script(next, caller_qname, inlinable, summaries, counter, false);
    let (new_body, c3) = rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
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

/// `While` / `Foreach` / `Catch` / `UpFrame`: recurse into the single
/// (non-tail) body, rebuilding the statement when it changed.
fn rewrite_single_body_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
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
            let (new_body, ch) =
                rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
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
        Statement::Foreach {
            span,
            iterators,
            body,
            body_span,
            is_lmap,
            raw_args,
            is_dict_iteration,
            is_array_iteration,
            raw_tokens,
        } => {
            let (new_body, ch) =
                rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
            ch.then(|| {
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
        Statement::Catch {
            span,
            body,
            body_span,
            result_var,
            options_var,
            raw_args,
            tokens,
        } => {
            let (new_body, ch) =
                rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
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
        Statement::UpFrame {
            span,
            frame_shift,
            body,
            tokens,
        } => {
            let (new_body, ch) =
                rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
            ch.then(|| {
                vec![Statement::UpFrame {
                    span: *span,
                    frame_shift: *frame_shift,
                    body: new_body,
                    tokens: tokens.clone(),
                }]
            })
        }
        _ => None,
    }
}

/// `Try`: recurse into the body, each handler body, and the finally body
/// (none is a tail position).
fn rewrite_try_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
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
    let (new_body, mut changed) =
        rewrite_script(body, caller_qname, inlinable, summaries, counter, false);
    let mut new_handlers = Vec::with_capacity(handlers.len());
    for h in handlers {
        let (hbody, ch) =
            rewrite_script(&h.body, caller_qname, inlinable, summaries, counter, false);
        if ch {
            changed = true;
            new_handlers.push(TryHandler {
                kind: h.kind.clone(),
                match_arg: h.match_arg.clone(),
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
            let (s, ch) = rewrite_script(b, caller_qname, inlinable, summaries, counter, false);
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

/// `Switch`: recurse into each arm / default body, propagating
/// `is_terminal` (the matched arm is a tail position, like `if`).
fn rewrite_switch_stmt(
    stmt: &Statement,
    caller_qname: &str,
    inlinable: &HashMap<String, InlineSpec>,
    summaries: &HashMap<String, ProcEscapeSummary>,
    counter: &mut usize,
    is_terminal: bool,
) -> Option<Vec<Statement>> {
    let Statement::Switch {
        span,
        subject,
        subject_span,
        arms,
        default_body,
        default_span,
        mode,
        nocase,
        raw_args,
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
                let (body, ch) =
                    rewrite_script(b, caller_qname, inlinable, summaries, counter, is_terminal);
                if ch {
                    changed = true;
                    new_arms.push(SwitchArm {
                        pattern: a.pattern.clone(),
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
            let (s, ch) =
                rewrite_script(b, caller_qname, inlinable, summaries, counter, is_terminal);
            if ch {
                changed = true;
            }
            Some(s)
        }
        None => None,
    };
    changed.then(|| {
        vec![Statement::Switch {
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

// per-call-site splice

/// Produce the inlined statement list for a single call site, or `None`
/// to decline (keep the call).
fn splice_call_site(
    call: &Statement,
    spec: &InlineSpec,
    counter: &mut usize,
    is_terminal: bool,
) -> Option<Vec<Statement>> {
    let Statement::Call { args, span, .. } = call else {
        return None;
    };
    let span = *span;
    match spec {
        InlineSpec::Empty => {
            if !args.is_empty() {
                return None;
            }
            Some(Vec::new())
        }
        InlineSpec::Verbatim(body) => {
            if !args.is_empty() {
                return None;
            }
            // Re-attribute each spliced call to the call site for diagnostics.
            Some(
                body.iter()
                    .map(|inner| match inner {
                        Statement::Call { .. } => with_span(inner, span),
                        other => other.clone(),
                    })
                    .collect(),
            )
        }
        InlineSpec::Parameterised(proc) => splice_v3(call, proc, counter, is_terminal),
    }
}

/// v3 splice — build parameter bindings, α-rename the body, and handle
/// the `return`-disposition strategy.
fn splice_v3(
    call: &Statement,
    proc: &Procedure,
    counter: &mut usize,
    is_terminal: bool,
) -> Option<Vec<Statement>> {
    let span = call.span();
    let (cid, mut rename, bindings) = build_param_bindings(call, proc, counter)?;

    // Mangle every locally-written variable too.
    for name in collect_local_names(&proc.body) {
        rename
            .entry(name.clone())
            .or_insert_with(|| format!("__inline_{cid}__{name}"));
    }

    let body_stmts = &proc.body.statements;
    let body_has_trailing_return = body_stmts
        .last()
        .is_some_and(|s| matches!(s, Statement::Return { .. }));
    let body_has_non_trailing_return = body_stmts
        .iter()
        .rev()
        .skip(1)
        .any(|s| matches!(s, Statement::Return { .. }))
        || has_nested_irreturn(&proc.body);

    let renamed_body = rename::rewrite_script(&proc.body, &rename);

    let mut out = bindings;
    if body_has_non_trailing_return {
        // Capture the implicit-trailing-return value when the body falls
        // off the end without an explicit `return`.
        let mut implicit: Option<String> = None;
        if !body_has_trailing_return && let Some(trailing) = renamed_body.statements.last() {
            implicit = Some(capture_implicit_return_value(trailing)?);
        }
        let result_var = format!("__inline_{cid}__RESULT");
        let mut wrapped = wrap_with_irreturn_loop(&renamed_body, &result_var, span, implicit);
        if is_terminal {
            wrapped.push(Statement::Return {
                span,
                value: Some(format!("${result_var}")),
                expr: None,
                braced: false,
            });
        }
        out.extend(wrapped);
        return Some(out);
    }

    if body_has_trailing_return && !is_terminal {
        // Trailing return at a non-terminal site — wrap so it doesn't
        // short-circuit the rest of the caller's body.
        let result_var = format!("__inline_{cid}__RESULT");
        out.extend(wrap_with_irreturn_loop(
            &renamed_body,
            &result_var,
            span,
            None,
        ));
        return Some(out);
    }

    // No return, or a trailing return at a terminal call site — flat splice.
    out.extend(renamed_body.statements);
    Some(out)
}

/// Clone `stmt` (a [`Statement::Call`]) with its span replaced. Used to
/// re-attribute spliced statements to the call site.
fn with_span(stmt: &Statement, span: Span) -> Statement {
    match stmt {
        Statement::Call {
            command,
            canonical_command,
            args,
            defs,
            reads,
            reads_own_defs,
            safe_on_uninit,
            tokens,
            foreach_groups,
            ..
        } => Statement::Call {
            span,
            command: command.clone(),
            canonical_command: canonical_command.clone(),
            args: args.clone(),
            defs: defs.clone(),
            reads: reads.clone(),
            reads_own_defs: *reads_own_defs,
            safe_on_uninit: *safe_on_uninit,
            tokens: tokens.clone(),
            foreach_groups: foreach_groups.clone(),
        },
        other => other.clone(),
    }
}

// nested-return scanning

fn has_nested_irreturn(script: &Script) -> bool {
    script
        .statements
        .iter()
        .any(|s| !matches!(s, Statement::Return { .. }) && scan_for_irreturn(s))
}

fn scan_for_irreturn(stmt: &Statement) -> bool {
    match stmt {
        Statement::Return { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|c| has_any_irreturn(&c.body))
                || else_body.as_ref().is_some_and(has_any_irreturn)
        }
        Statement::For {
            init, next, body, ..
        } => has_any_irreturn(init) || has_any_irreturn(next) || has_any_irreturn(body),
        Statement::Block { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => has_any_irreturn(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            has_any_irreturn(body)
                || handlers.iter().any(|h| has_any_irreturn(&h.body))
                || finally_body.as_ref().is_some_and(has_any_irreturn)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter()
                .any(|a| a.body.as_ref().is_some_and(has_any_irreturn))
                || default_body.as_ref().is_some_and(has_any_irreturn)
        }
        _ => false,
    }
}

fn has_any_irreturn(script: &Script) -> bool {
    script
        .statements
        .iter()
        .any(|s| matches!(s, Statement::Return { .. }) || scan_for_irreturn(s))
}

/// Return the literal string the body's last statement contributes as
/// Tcl's implicit return value, or `None` when the shape is too complex
/// to capture safely.
fn capture_implicit_return_value(stmt: &Statement) -> Option<String> {
    match stmt {
        Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => {
            Some(value.clone())
        }
        _ => None,
    }
}

// return-as-break wrap

/// Wrap `renamed_body` in a one-shot `while {1} { … }` loop that routes
/// `return`-as-break. Uses `while`
/// rather than `for` so empty init/next clauses don't emit placeholder
/// calls the WASM codegen would trap on.
fn wrap_with_irreturn_loop(
    renamed_body: &Script,
    result_var: &str,
    span: Span,
    implicit_value: Option<String>,
) -> Vec<Statement> {
    let rewritten = substitute_irreturn(renamed_body, result_var);
    let mut body_with_break: Vec<Statement> = rewritten.statements;
    if let Some(v) = implicit_value {
        body_with_break.push(Statement::AssignValue {
            span,
            name: result_var.to_owned(),
            name_braced: false,
            value: v.clone(),
            value_needs_backsubst: v.contains('\\'),
            tokens: None,
        });
    }
    body_with_break.push(break_call(span));

    vec![
        Statement::AssignConst {
            span,
            name: result_var.to_owned(),
            name_braced: false,
            value: String::new(),
        },
        Statement::While {
            span,
            condition: ExprNode::Literal {
                text: "1".to_owned(),
                start: 0,
                end: 1,
            },
            condition_span: span,
            condition_base: None,
            body: Script {
                statements: body_with_break,
            },
            body_span: span,
            raw_args: Vec::new(),
            raw_tokens: None,
        },
    ]
}

/// A synthetic `break` call.
fn break_call(span: Span) -> Statement {
    Statement::Call {
        span,
        command: "break".to_owned(),
        canonical_command: Some("::break".to_owned()),
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

/// Replace each [`Statement::Return`] reachable from `script` with
/// `set <result_var> <value>; break`. Recurses into transparent nested
/// bodies (`Block`, `if`, `switch`); does NOT recurse into loop / catch /
/// try / upframe bodies (the eligibility gate guarantees no return is
/// nested there).
fn substitute_irreturn(script: &Script, result_var: &str) -> Script {
    let mut out = Vec::with_capacity(script.statements.len());
    for stmt in &script.statements {
        if let Statement::Return { span, value, .. } = stmt {
            let v = value.clone().unwrap_or_default();
            out.push(Statement::AssignValue {
                span: *span,
                name: result_var.to_owned(),
                name_braced: false,
                value: v.clone(),
                value_needs_backsubst: v.contains('\\'),
                tokens: None,
            });
            out.push(break_call(*span));
        } else {
            out.push(substitute_irreturn_stmt(stmt, result_var));
        }
    }
    Script { statements: out }
}

fn substitute_irreturn_stmt(stmt: &Statement, result_var: &str) -> Statement {
    match stmt {
        Statement::Block {
            span,
            body,
            namespace,
            tokens,
        } => Statement::Block {
            span: *span,
            body: substitute_irreturn(body, result_var),
            namespace: namespace.clone(),
            tokens: tokens.clone(),
        },
        Statement::If {
            span,
            clauses,
            else_body,
            else_span,
        } => Statement::If {
            span: *span,
            clauses: clauses
                .iter()
                .map(|c| IfClause {
                    condition: c.condition.clone(),
                    condition_span: c.condition_span,
                    condition_base: c.condition_base,
                    body: substitute_irreturn(&c.body, result_var),
                    body_span: c.body_span,
                })
                .collect(),
            else_body: else_body
                .as_ref()
                .map(|b| substitute_irreturn(b, result_var)),
            else_span: *else_span,
        },
        Statement::Switch {
            span,
            subject,
            subject_span,
            arms,
            default_body,
            default_span,
            mode,
            nocase,
            raw_args,
            patterns_braced,
        } => Statement::Switch {
            span: *span,
            subject: subject.clone(),
            subject_span: *subject_span,
            arms: arms
                .iter()
                .map(|a| SwitchArm {
                    pattern: a.pattern.clone(),
                    pattern_span: a.pattern_span,
                    body: a.body.as_ref().map(|b| substitute_irreturn(b, result_var)),
                    body_span: a.body_span,
                    fallthrough: a.fallthrough,
                })
                .collect(),
            default_body: default_body
                .as_ref()
                .map(|b| substitute_irreturn(b, result_var)),
            default_span: *default_span,
            mode: *mode,
            nocase: *nocase,
            raw_args: raw_args.clone(),
            patterns_braced: *patterns_braced,
        },
        other => other.clone(),
    }
}

// parameter bindings

/// Build the parameter-binding statement list for a v3 call. Returns
/// `(cid, rename, bindings)` on success, or `None` to decline.
fn build_param_bindings(
    call: &Statement,
    proc: &Procedure,
    counter: &mut usize,
) -> Option<(usize, HashMap<String, String>, Vec<Statement>)> {
    let Statement::Call {
        args, tokens, span, ..
    } = call
    else {
        return None;
    };
    let span = *span;

    // `{*}` expansion changes call arity at runtime — decline.
    if let Some(t) = tokens
        && let Some(expand) = &t.expand_word
        && expand.iter().skip(1).any(|&b| b)
    {
        return None;
    }

    *counter += 1;
    let cid = *counter;
    let mut rename: HashMap<String, String> = HashMap::new();
    for p in &proc.params {
        rename.insert(p.clone(), format!("__inline_{cid}__{p}"));
    }

    let params = &proc.params;
    let has_variadic = params.last().is_some_and(|p| p == "args");
    let tokens = tokens.as_ref();

    let mut bound: Vec<String> = Vec::new();
    let mut bound_is_literal: Vec<bool> = Vec::new();

    if has_variadic {
        let positional_count = params.len() - 1;
        if args.len() < positional_count {
            return build_with_defaults(cid, &rename, proc, args, tokens, positional_count, true);
        }
        for (i, a) in args.iter().enumerate().take(positional_count) {
            bound.push(a.clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
        }
        let extras = &args[positional_count..];
        if extras.is_empty() {
            bound.push(String::new());
            bound_is_literal.push(true);
        } else {
            for i in positional_count..args.len() {
                if arg_is_braced_literal(tokens, i) {
                    return None;
                }
            }
            for w in extras {
                // An empty extra word (e.g. the quoted `""` in `v ""`) would
                // collapse inside `[list …]` — `[list ]` is a zero-element
                // list, dropping the original one-element value. Decline so
                // the call falls back to runtime dispatch with correct arity.
                if w.is_empty() || !list_clean_for_splice(w) {
                    return None;
                }
            }
            bound.push(format!("[list {}]", extras.join(" ")));
            bound_is_literal.push(false);
        }
    } else {
        if args.len() > params.len() {
            return None;
        }
        if args.len() < params.len() {
            return build_with_defaults(cid, &rename, proc, args, tokens, params.len(), false);
        }
        for (i, a) in args.iter().enumerate() {
            bound.push(a.clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
        }
    }

    let mut bindings: Vec<Statement> = Vec::with_capacity(params.len());
    for ((p, v), is_literal) in params.iter().zip(&bound).zip(&bound_is_literal) {
        let name = rename[p].clone();
        if *is_literal {
            bindings.push(Statement::AssignConst {
                span,
                name,
                name_braced: false,
                value: v.clone(),
            });
        } else {
            bindings.push(Statement::AssignValue {
                span,
                name,
                name_braced: false,
                value: v.clone(),
                value_needs_backsubst: v.contains('\\'),
                tokens: None,
            });
        }
    }
    Some((cid, rename, bindings))
}

/// Whether the call-site word at `args[idx]` was a braced literal (`{…}`)
/// in source — so it must bind as an [`Statement::AssignConst`], preserving
/// embedded `$`/`[` rather than re-substituting them at the inlined site.
/// `argv[0]` is the command name, so `args[idx]` aligns with `argv[idx+1]`.
fn arg_is_braced_literal(tokens: Option<&CommandTokens>, idx: usize) -> bool {
    let Some(t) = tokens else { return false };
    t.argv_kinds
        .get(idx + 1)
        .is_some_and(|k| *k == TokenType::Str)
}

/// Try to fill missing positional args from declared defaults.
///
/// Matches the non-default path's literal binding: a call-site word that
/// was a braced literal binds as [`Statement::AssignConst`], and so does a
/// declared default — Tcl proc defaults are literal (`proc p {{a $x}} …; p`
/// binds `a` to the string `$x`, never the caller's `$x`). Without this the
/// default path would re-substitute both in the caller frame.
fn build_with_defaults(
    cid: usize,
    rename: &HashMap<String, String>,
    proc: &Procedure,
    args: &[String],
    tokens: Option<&CommandTokens>,
    positional_count: usize,
    has_variadic: bool,
) -> Option<(usize, HashMap<String, String>, Vec<Statement>)> {
    if proc.params_raw.is_empty() {
        return None;
    }
    let parsed = parse_params_with_defaults(&proc.params_raw);
    if parsed.len() != proc.params.len() {
        return None; // parse drift — bail safely
    }

    let mut bound: Vec<String> = Vec::new();
    let mut bound_is_literal: Vec<bool> = Vec::new();
    for i in 0..positional_count {
        if i < args.len() {
            bound.push(args[i].clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
            continue;
        }
        let default = parsed[i].1.as_ref()?;
        bound.push(default.clone());
        bound_is_literal.push(true); // defaults are literal
    }
    if has_variadic {
        bound.push(String::new());
        bound_is_literal.push(true); // empty literal
    }

    let bindings: Vec<Statement> = proc
        .params
        .iter()
        .zip(&bound)
        .zip(&bound_is_literal)
        .map(|((p, v), is_literal)| {
            let name = rename[p].clone();
            if *is_literal {
                Statement::AssignConst {
                    span: proc.span,
                    name,
                    name_braced: false,
                    value: v.clone(),
                }
            } else {
                Statement::AssignValue {
                    span: proc.span,
                    name,
                    name_braced: false,
                    value: v.clone(),
                    value_needs_backsubst: v.contains('\\'),
                    tokens: None,
                }
            }
        })
        .collect();
    Some((cid, rename.clone(), bindings))
}

/// Parse a Tcl proc `param_str` into `(name, default_or_None)` pairs.
fn parse_params_with_defaults(param_str: &str) -> Vec<(String, Option<String>)> {
    let text: Vec<char> = param_str.trim().chars().collect();
    let n = text.len();
    let mut params = Vec::new();
    let mut i = 0;
    let is_ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r');
    while i < n {
        while i < n && is_ws(text[i]) {
            i += 1;
        }
        if i >= n {
            break;
        }
        if text[i] == '{' {
            let mut level = 1;
            i += 1;
            let start = i;
            while i < n && level > 0 {
                if text[i] == '{' {
                    level += 1;
                } else if text[i] == '}' {
                    level -= 1;
                }
                i += 1;
            }
            let inner: Vec<char> = text[start..i - 1].to_vec();
            if inner.iter().all(|&c| is_ws(c)) {
                continue;
            }
            let mut split_at = 0;
            while split_at < inner.len() && !is_ws(inner[split_at]) {
                split_at += 1;
            }
            let name: String = inner[..split_at].iter().collect();
            let default: Option<String> = if split_at < inner.len() {
                let raw: String = inner[split_at..].iter().collect();
                let raw = raw.trim();
                let stripped = if raw.len() >= 2
                    && ((raw.starts_with('"') && raw.ends_with('"'))
                        || (raw.starts_with('{') && raw.ends_with('}')))
                {
                    raw[1..raw.len() - 1].to_owned()
                } else {
                    raw.to_owned()
                };
                Some(stripped)
            } else {
                None
            };
            if !name.is_empty() {
                params.push((name, default));
            }
            continue;
        }
        let start = i;
        while i < n && !is_ws(text[i]) {
            i += 1;
        }
        let word: String = text[start..i].iter().collect();
        if !word.is_empty() {
            params.push((word, None));
        }
    }
    params
}

/// Return True iff `text` can be safely spliced verbatim into a Tcl
/// `[list <e1> <e2> …]` synth as a single element.
fn list_clean_for_splice(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        let ch = chars[i];
        if ch == '\\' && i + 1 < n {
            i += 2;
            continue;
        }
        if ch == '$' && i + 1 < n && chars[i + 1] == '{' {
            let mut j = i + 2;
            while j < n && chars[j] != '}' {
                j += 1;
            }
            if j >= n {
                return false; // unbalanced ${
            }
            i = j + 1;
            continue;
        }
        if ch == '[' {
            let mut depth = 1;
            let mut j = i + 1;
            while j < n && depth > 0 {
                let c2 = chars[j];
                if c2 == '\\' && j + 1 < n {
                    j += 2;
                    continue;
                }
                if c2 == '[' {
                    depth += 1;
                } else if c2 == ']' {
                    depth -= 1;
                }
                j += 1;
            }
            if depth != 0 {
                return false; // unbalanced [
            }
            i = j;
            continue;
        }
        if ch.is_whitespace() {
            return false;
        }
        if ch == '{' || ch == '}' || ch == '"' {
            return false;
        }
        i += 1;
    }
    true
}

// local-name collection

/// Return the union of every local-variable name written inside `script`.
/// Array-element writes contribute the array base name.
fn collect_local_names(script: &Script) -> HashSet<String> {
    let mut names = HashSet::new();
    walk_local_writes(script, &mut names);
    names
}

fn record_local_base(name: &str, names: &mut HashSet<String>) {
    if name.contains("::") {
        return;
    }
    let base = match name.find('(') {
        Some(p) => &name[..p],
        None => name,
    };
    if !base.is_empty() {
        names.insert(base.to_owned());
    }
}

fn walk_local_writes(script: &Script, names: &mut HashSet<String>) {
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => record_local_base(name, names),
            Statement::Call { defs, .. } => {
                for d in defs {
                    record_local_base(d, names);
                }
            }
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                walk_local_writes(body, names);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_local_writes(&c.body, names);
                }
                if let Some(b) = else_body {
                    walk_local_writes(b, names);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_local_writes(init, names);
                walk_local_writes(next, names);
                walk_local_writes(body, names);
            }
            Statement::While { body, .. } => walk_local_writes(body, names),
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    for v in &it.vars {
                        record_local_base(v, names);
                    }
                }
                walk_local_writes(body, names);
            }
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                walk_local_writes(body, names);
                if let Some(v) = result_var {
                    names.insert(v.clone());
                }
                if let Some(v) = options_var {
                    names.insert(v.clone());
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_local_writes(body, names);
                for h in handlers {
                    walk_local_writes(&h.body, names);
                    if let Some(v) = &h.var_name {
                        names.insert(v.clone());
                    }
                    if let Some(v) = &h.options_var {
                        names.insert(v.clone());
                    }
                }
                if let Some(fb) = finally_body {
                    walk_local_writes(fb, names);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_local_writes(b, names);
                    }
                }
                if let Some(b) = default_body {
                    walk_local_writes(b, names);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
