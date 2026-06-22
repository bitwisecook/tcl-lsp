//! General proc inliner (v0 + verbatim + v3-simple).
//!
//! Ported from `compiler/inlining/` — the catalogue (`decision.py`) plus
//! the IR-level splice transform (`inline_pass.py`) and the α-rename
//! machinery (`_rename.py`). The inliner rewrites statement-position calls
//! to *inlinable* procedures so the call boundary disappears before codegen
//! consumes the IR.
//!
//! # Scope
//!
//! * **v0 — empty body.** A call to a zero-statement proc vanishes.
//! * **v1 / v2 — verbatim wrapper.** A zero-parameter proc whose every
//!   body statement is *splice-eligible* (a frame-independent, def-free,
//!   substitution-free builtin call) has its body spliced verbatim into
//!   the call site.
//! * **v3 — parameterised (simple-arity).** A proc *with* parameters over
//!   the same splice-safe body shape: each positional arg binds to a freshly
//!   α-renamed parameter local ([`rewrite_value_string`] rewrites the body's
//!   `$param` references), then the bindings + renamed body splice in. Scoped
//!   to simple arity — no variadic `args`, no parameter defaults, and a call
//!   site whose args carry no `[cmd]` substitution. The fuller v3 surface —
//!   non-trailing `return` for/break wrapping, variadic packing, parameter
//!   defaults, array-write renaming, nested control-flow bodies, and
//!   dead-proc elimination — is the documented residual (FE-OPT). The inliner
//!   is exposed but not yet wired into a default pipeline; its correctness is
//!   held by the `tcl-vm` execution-differential gate
//!   (`tcl-vm/tests/inliner_differential.rs`).
//!
//! # Eligibility
//!
//! Two gates combine in [`classify_inline_spec`] / `build_inlinable_map`: the
//! precise interprocedural [`var_escape::pure_leaf`](crate::var_escape)
//! proof (`safe_to_inline` — FE-VARESCAPE's S4.1 soundness predicate) and
//! the structural shape. The per-statement [`stmt_is_splice_eligible`] check
//! then guards the body itself.
//!
//! # Soundness
//!
//! Every shape leans on [`stmt_is_splice_eligible`]: a body statement may be
//! lifted out of the proc frame only when it is an [`Statement::Call`] with
//! no `defs` (it cannot mutate the caller's scope), whose command is in the
//! frame-independent allow-list (no `info` / `uplevel` / `upvar` / `return`
//! / `break` / `continue`), and whose arguments carry no `[cmd]`
//! substitution. That makes the splice observationally equivalent to the call
//! regardless of which frame it runs in. v3 additionally α-renames each
//! parameter to a unique mangled local (per-call-site suffix), so a bound
//! parameter never captures or is captured by a caller local, and binds args
//! in caller-frame order exactly as a real call would. Empty bodies are
//! trivially safe. Redefined procs are never inlined (their body is
//! ambiguous). Recursion cannot occur: a call to a user proc is not in the
//! splice-safe allow-list, so a self-calling body is never eligible.

use std::collections::{HashMap, HashSet};

use crate::ir::{Module, Procedure, Script, Statement};
use crate::var_escape::ProcEscapeSummary;

/// Per-proc inlining eligibility — the catalogue tag. Mirrors Python's
/// `InlineDecision`, restricted to the cases the v0 / verbatim mechanism
/// acts on. `IfSingleCall` is intentionally inert here (its profitability
/// depends on post-inline proc pruning, not yet ported).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDecision {
    /// Always inline a call to this proc.
    Always,
    /// Inline only when the proc has exactly one static caller (not yet
    /// acted on — recorded for parity / future pruning).
    IfSingleCall,
    /// Never inline.
    Never,
}

/// A pure-leaf proc whose body has at most this many flat statements is
/// unconditionally inlinable. Mirrors `decision.SMALL_BODY_THRESHOLD`.
pub const SMALL_BODY_THRESHOLD: usize = 5;

/// What an inlinable proc splices in at a call site.
#[derive(Debug, Clone)]
enum InlineSpec {
    /// v0 — empty body; the call vanishes.
    Empty,
    /// v1 / v2 — splice these body statements verbatim.
    Verbatim(Vec<Statement>),
    /// v3 — parameterised: bind each call arg to an α-renamed parameter,
    /// rewrite parameter references through the body, then splice. Scoped to
    /// the same splice-safe body shape as [`InlineSpec::Verbatim`] (every body
    /// statement a frame-independent, def-free builtin [`Statement::Call`]),
    /// so the only rewrite needed is α-renaming the parameter `$refs` inside
    /// each `Call`'s argument strings. The fuller v3 surface — non-trailing
    /// `return` for/break wrapping, variadic `args` packing, parameter
    /// defaults, array-write renaming, nested control-flow bodies — is the
    /// documented residual (FE-OPT).
    Parameterised {
        /// Parameter names, in order. The call's positional args bind to these.
        params: Vec<String>,
        /// Splice-safe body statements (all `Call`s).
        body: Vec<Statement>,
    },
}

/// Total number of leaf statements reachable in `script`, walking nested
/// control-flow bodies transitively (the inlined code-size cost). Mirrors
/// `decision.count_statements`.
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

/// Commands whose semantics are independent of the calling frame — a
/// wrapped call to one of these can be spliced into any caller's frame.
/// Frame-observing (`info` / `uplevel` / `upvar`) and frame-affecting
/// control flow (`return` / `break` / `continue`) are deliberately
/// excluded. Mirrors `_SPLICE_SAFE_COMMANDS`.
const SPLICE_SAFE_COMMANDS: &[&str] = &[
    // List primitives — pure value computation.
    "list", "lindex", "lrange", "linsert", "llength", "lsort", "lsearch", "lreverse", "lreplace",
    "lrepeat", "concat", // String primitives.
    "split", "join", "string", // Arithmetic.
    "expr",   // I/O — observable but frame-independent.
    "puts",
];

/// Whether `command` (the head word as written) is a frame-independent,
/// splice-safe builtin. Mirrors `_command_is_splice_safe`.
fn command_is_splice_safe(command: &str) -> bool {
    // Accept a `::`-qualified spelling of a builtin too (`::puts`).
    let bare = command.strip_prefix("::").unwrap_or(command);
    SPLICE_SAFE_COMMANDS.contains(&bare)
}

/// Whether a statement can be lifted out of a wrapper proc and spliced
/// into a caller without changing observable behaviour. Mirrors
/// `_stmt_is_splice_eligible`.
fn stmt_is_splice_eligible(stmt: &Statement) -> bool {
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
    if !command_is_splice_safe(command) {
        return false;
    }
    // No `[cmd]` substitution in any argument — its result would depend on
    // evaluation in the original frame's command-resolution context.
    if args.iter().any(|a| a.contains('[')) {
        return false;
    }
    true
}

/// Classify a procedure's *structural shape* for inlining. A proc is
/// [`InlineDecision::Always`] when it is not redefined and is either
/// empty-bodied (v0) or a small zero-parameter wrapper whose every body
/// statement is splice-eligible (v1 / v2). Everything else is
/// [`InlineDecision::Never`] in this port (v3 parameterised inlining is the
/// residual). The caller (`build_inlinable_map`) additionally gates on the
/// interprocedural `var_escape::safe_to_inline` (`pure_leaf`) soundness
/// proof — this function only judges the shape.
#[must_use]
pub fn classify_proc<S: std::hash::BuildHasher>(
    proc: &Procedure,
    redefined: &HashSet<String, S>,
) -> InlineDecision {
    if redefined.contains(&proc.qualified_name) {
        return InlineDecision::Never;
    }
    if proc.body.statements.is_empty() {
        return InlineDecision::Always;
    }
    if proc.params.is_empty()
        && count_statements(&proc.body) <= SMALL_BODY_THRESHOLD
        && proc.body.statements.iter().all(stmt_is_splice_eligible)
    {
        return InlineDecision::Always;
    }
    InlineDecision::Never
}

/// Build the qname → [`InlineSpec`] map of inlinable procedures.
///
/// Two gates combine: the precise `var_escape::pure_leaf` predicate
/// (`safe_to_inline`, the Python S4.1 soundness proof — FE-VARESCAPE) and
/// the structural shape classification ([`classify_proc`]). A proc must
/// pass both. The escape summaries are computed from the IR module, the
/// path that populates `pure_leaf`.
fn build_inlinable_map(module: &Module) -> HashMap<String, InlineSpec> {
    let summaries = crate::var_escape::analyse_var_escape(module, true);
    let mut map = HashMap::new();
    for (qname, proc) in &module.procedures {
        // Soundness gate: the interprocedural pure-leaf proof. An absent
        // summary (shouldn't happen for a module proc) is treated as opaque.
        if !summaries
            .get(qname)
            .is_some_and(ProcEscapeSummary::safe_to_inline)
        {
            continue;
        }
        if let Some(spec) = classify_inline_spec(proc, &module.redefined_procedures) {
            map.insert(qname.clone(), spec);
        }
    }
    map
}

/// Whether `proc` takes a variadic trailing `args` parameter — declined by v3
/// (it would need call-site list packing, the documented residual).
fn has_variadic_args(proc: &Procedure) -> bool {
    proc.params.iter().any(|p| p == "args")
}

/// Whether `proc` declares any parameter default. A default surfaces as a
/// braced `{name value}` group in `params_raw`; a plain `{a b c}` list has no
/// braces. Declined by v3 (defaults need call-site arity reasoning).
fn has_parameter_defaults(proc: &Procedure) -> bool {
    proc.params_raw.contains('{')
}

/// Choose the inline shape for `proc`, or `None` when it is not inlinable.
/// Combines the existing v0/verbatim shapes with the v3 parameterised shape.
/// The caller has already applied the `safe_to_inline` soundness gate.
fn classify_inline_spec<S: std::hash::BuildHasher>(
    proc: &Procedure,
    redefined: &HashSet<String, S>,
) -> Option<InlineSpec> {
    if redefined.contains(&proc.qualified_name) {
        return None;
    }
    if proc.body.statements.is_empty() {
        return Some(InlineSpec::Empty);
    }
    let body_is_splice_safe = count_statements(&proc.body) <= SMALL_BODY_THRESHOLD
        && proc.body.statements.iter().all(stmt_is_splice_eligible);
    if !body_is_splice_safe {
        return None;
    }
    if proc.params.is_empty() {
        // v1 / v2 — zero-parameter wrapper.
        return Some(InlineSpec::Verbatim(proc.body.statements.clone()));
    }
    // v3 — parameterised, restricted to the simple-arity shape (no variadic
    // `args`, no parameter defaults) over a splice-safe body.
    if has_variadic_args(proc) || has_parameter_defaults(proc) {
        return None;
    }
    Some(InlineSpec::Parameterised {
        params: proc.params.clone(),
        body: proc.body.statements.clone(),
    })
}

/// Resolve a call's head word to a procedure qname, using the same naive
/// rule the optimiser folds use: an absolute name as-is, else rooted at
/// `::`. (Namespace-chain resolution is the shared O103 / inliner residual.)
fn resolve_qname(command: &str) -> String {
    if command.starts_with("::") {
        command.to_owned()
    } else {
        format!("::{command}")
    }
}

/// Inline eligible statement-position calls throughout `module`, returning
/// the rewritten module. Idempotent: a module with no remaining inlinable
/// sites is returned structurally unchanged. Mirrors `inline_module`'s v0 /
/// verbatim behaviour (v3 and dead-proc elimination are the residual).
#[must_use]
pub fn inline_module(mut module: Module) -> Module {
    let inlinable = build_inlinable_map(&module);
    if inlinable.is_empty() {
        return module;
    }
    // A monotonic counter gives every parameterised-inline site a unique
    // α-rename suffix, so spliced parameter locals never collide with each
    // other or (realistically) with the caller's locals.
    let mut counter: u32 = 0;
    rewrite_script(&mut module.top_level, &inlinable, &mut counter);
    let mut procedures = std::mem::take(&mut module.procedures);
    for proc in procedures.values_mut() {
        rewrite_script(&mut proc.body, &inlinable, &mut counter);
    }
    module.procedures = procedures;
    module
}

/// Rewrite a script in place: replace each inlinable statement-position
/// call with the proc's spliced statements (or nothing, for empty bodies),
/// then recurse into control-flow bodies.
fn rewrite_script(script: &mut Script, inlinable: &HashMap<String, InlineSpec>, counter: &mut u32) {
    let mut out: Vec<Statement> = Vec::with_capacity(script.statements.len());
    for mut stmt in std::mem::take(&mut script.statements) {
        // Decide the splice for this statement (if any) without holding a
        // borrow of `stmt` across the mutable recursion below.
        let spliced: Option<Vec<Statement>> = match &stmt {
            Statement::Call {
                command,
                args,
                span,
                ..
            } => inlinable
                .get(&resolve_qname(command))
                .and_then(|spec| match spec {
                    InlineSpec::Empty => Some(Vec::new()),
                    InlineSpec::Verbatim(body) => Some(body.clone()),
                    InlineSpec::Parameterised { params, body } => {
                        instantiate_parameterised(params, body, args, *span, counter)
                    }
                }),
            _ => None,
        };
        if let Some(body) = spliced {
            out.extend(body);
            continue;
        }
        recurse_into_bodies(&mut stmt, inlinable, counter);
        out.push(stmt);
    }
    script.statements = out;
}

/// Instantiate a parameterised inline at a call site: bind each positional arg
/// to a freshly α-renamed parameter local, then splice the body with every
/// parameter `$ref` rewritten to its mangled name. Returns `None` (leaving the
/// call intact) when the call is unsafe to inline — an arity mismatch, or an
/// arg carrying `[cmd]` substitution (whose result depends on the caller's
/// command-resolution context, and which the synthetic binding can't model).
fn instantiate_parameterised(
    params: &[String],
    body: &[Statement],
    args: &[String],
    span: tcl_lexer::Span,
    counter: &mut u32,
) -> Option<Vec<Statement>> {
    if args.len() != params.len() {
        return None;
    }
    if args.iter().any(|a| a.contains('[')) {
        return None;
    }
    let suffix = *counter;
    *counter += 1;
    let rename: HashMap<String, String> = params
        .iter()
        .map(|p| (p.clone(), format!("__inl_{p}_{suffix}")))
        .collect();

    let mut out: Vec<Statement> = Vec::with_capacity(params.len() + body.len());
    // Parameter bindings: `set <mangled> <arg>`, evaluated in the caller frame
    // exactly as Tcl evaluates a call's actual arguments.
    for (param, arg) in params.iter().zip(args) {
        out.push(Statement::AssignValue {
            span,
            name: rename[param].clone(),
            value: arg.clone(),
            value_needs_backsubst: arg.contains('\\'),
            tokens: None,
        });
    }
    // Spliced body with parameter references α-renamed.
    for stmt in body {
        out.push(rename_splice_stmt(stmt, &rename));
    }
    Some(out)
}

/// Clone a splice-safe body statement with the parameter rename map applied.
/// Body statements are all builtin [`Statement::Call`]s (the splice-safe
/// gate), so the rewrite is: α-rename each argument string and any explicit
/// `reads`, and drop the now-stale `tokens` so codegen re-derives from the
/// rewritten `args`.
fn rename_splice_stmt(stmt: &Statement, rename: &HashMap<String, String>) -> Statement {
    match stmt {
        Statement::Call {
            span,
            command,
            canonical_command,
            args,
            defs,
            reads,
            reads_own_defs,
            safe_on_uninit,
            foreach_groups,
            ..
        } => Statement::Call {
            span: *span,
            command: command.clone(),
            canonical_command: canonical_command.clone(),
            args: args
                .iter()
                .map(|a| rewrite_value_string(a, rename))
                .collect(),
            defs: defs.clone(),
            reads: reads.iter().map(|r| rename_var_name(r, rename)).collect(),
            reads_own_defs: *reads_own_defs,
            safe_on_uninit: *safe_on_uninit,
            tokens: None,
            foreach_groups: foreach_groups.clone(),
        },
        // Splice-safe bodies contain only `Call`s; anything else is cloned
        // verbatim (defensive — `classify_inline_spec` never admits it).
        other => other.clone(),
    }
}

/// Apply `rename` to a variable-name field, preserving an `arr(idx)` element
/// suffix (the array base carries the binding identity). Mirrors Python's
/// `_rename_var_name`.
fn rename_var_name(name: &str, rename: &HashMap<String, String>) -> String {
    if let Some(paren) = name.find('(') {
        let base = &name[..paren];
        let tail = &name[paren..];
        if let Some(renamed) = rename.get(base) {
            return format!("{renamed}{tail}");
        }
        return name.to_owned();
    }
    rename.get(name).cloned().unwrap_or_else(|| name.to_owned())
}

/// Rewrite `$name` / `${name}` substitutions in `text` through `rename`,
/// leaving names absent from the map untouched. A faithful port of Python's
/// `_rewrite_value_string`, including its backslash pair-counting: `\$x` is a
/// literal `$` (not a substitution), but `\\$x` *is* a substitution because the
/// first backslash escapes the second. Array references (`$arr(idx)`) rename
/// the array base only — the index text is preserved verbatim.
fn rewrite_value_string(text: &str, rename: &HashMap<String, String>) -> String {
    if text.is_empty() || rename.is_empty() {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        let ch = bytes[i];
        // A backslash escapes the next byte; both pass through verbatim. This
        // makes `\$` a literal `$` and `\\` a literal backslash, so the byte
        // after an unescaped `\` is never treated as a substitution start.
        if ch == b'\\' && i + 1 < n {
            out.push('\\');
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if ch != b'$' {
            out.push(ch as char);
            i += 1;
            continue;
        }
        // An unescaped `$`: try to recognise a substitution.
        if i + 1 < n && bytes[i + 1] == b'{' {
            // `${name}` form — find the closing `}` (no nesting in Tcl).
            if let Some(rel) = text[i + 2..].find('}') {
                let j = i + 2 + rel;
                let name = &text[i + 2..j];
                let (base, tail) = match name.find('(') {
                    Some(p) => (&name[..p], &name[p..]),
                    None => (name, ""),
                };
                if let Some(renamed) = rename.get(base) {
                    out.push_str("${");
                    out.push_str(renamed);
                    out.push_str(tail);
                    out.push('}');
                } else {
                    out.push_str(&text[i..=j]);
                }
                i = j + 1;
                continue;
            }
            // Malformed `${` — emit the `$` verbatim and carry on.
            out.push('$');
            i += 1;
            continue;
        }
        // Bare `$name` form — scan the variable-name run (alnum / `_` /
        // namespace `::`), then an optional `(idx)` array suffix.
        let name_start = i + 1;
        let mut j = name_start;
        while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':') {
            j += 1;
        }
        if j == name_start {
            // `$` not followed by a name char — literal `$`.
            out.push('$');
            i += 1;
            continue;
        }
        let base = &text[name_start..j];
        // Optional `(idx)` array element suffix.
        let mut tail_end = j;
        if j < n && bytes[j] == b'(' && let Some(rel) = text[j..].find(')') {
            tail_end = j + rel + 1;
        }
        let tail = &text[j..tail_end];
        if let Some(renamed) = rename.get(base) {
            out.push('$');
            out.push_str(renamed);
            out.push_str(tail);
        } else {
            out.push_str(&text[i..tail_end]);
        }
        i = tail_end;
    }
    out
}

/// Recurse the inliner into a compound statement's nested bodies.
fn recurse_into_bodies(
    stmt: &mut Statement,
    inlinable: &HashMap<String, InlineSpec>,
    counter: &mut u32,
) {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                rewrite_script(&mut c.body, inlinable, counter);
            }
            if let Some(b) = else_body {
                rewrite_script(b, inlinable, counter);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            rewrite_script(init, inlinable, counter);
            rewrite_script(next, inlinable, counter);
            rewrite_script(body, inlinable, counter);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => rewrite_script(body, inlinable, counter),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            rewrite_script(body, inlinable, counter);
            for h in handlers {
                rewrite_script(&mut h.body, inlinable, counter);
            }
            if let Some(fb) = finally_body {
                rewrite_script(fb, inlinable, counter);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &mut a.body {
                    rewrite_script(b, inlinable, counter);
                }
            }
            if let Some(b) = default_body {
                rewrite_script(b, inlinable, counter);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn module_for(source: &str) -> Module {
        CompilationUnit::build_for(source, &CommandRegistry::build_default(), false).ir_module
    }

    /// Count statement-position calls to `command` across the top level.
    fn top_calls_to(module: &Module, command: &str) -> usize {
        module
            .top_level
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
            .count()
    }

    #[test]
    fn empty_body_call_vanishes() {
        let module = module_for("proc ::noop {} {}\nnoop\nputs done");
        assert_eq!(top_calls_to(&module, "noop"), 1);
        let inlined = inline_module(module);
        assert_eq!(
            top_calls_to(&inlined, "noop"),
            0,
            "empty-body call should vanish"
        );
        // The unrelated `puts done` survives.
        assert_eq!(top_calls_to(&inlined, "puts"), 1);
    }

    #[test]
    fn verbatim_wrapper_body_is_spliced() {
        let module = module_for("proc ::greet {} { puts hello }\ngreet");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "greet"), 0, "wrapper call replaced");
        assert_eq!(
            top_calls_to(&inlined, "puts"),
            1,
            "wrapper body spliced verbatim",
        );
    }

    #[test]
    fn multi_statement_verbatim_wrapper() {
        let module = module_for("proc ::two {} { puts a\n puts b }\ntwo");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "two"), 0);
        assert_eq!(top_calls_to(&inlined, "puts"), 2, "both body stmts spliced");
    }

    #[test]
    fn wrapper_with_defs_is_not_inlined() {
        // `set x 1` mutates the proc frame — not splice-eligible.
        let module = module_for("proc ::w {} { set x 1 }\nw");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "w"), 1, "frame-mutating body kept");
    }

    #[test]
    fn parameterised_proc_is_inlined() {
        // v3 — a simple-arity proc over a splice-safe body is inlined: the
        // call is replaced by a parameter binding + the α-renamed body.
        let module = module_for("proc ::id {x} { puts $x }\nid 1");
        let inlined = inline_module(module);
        assert_eq!(
            top_calls_to(&inlined, "id"),
            0,
            "parameterised proc inlined"
        );
        // A binding `set __inl_x_0 1` precedes a `puts $__inl_x_0`.
        let has_binding = inlined.top_level.statements.iter().any(|s| {
            matches!(s, Statement::AssignValue { name, value, .. } if name == "__inl_x_0" && value == "1")
        });
        assert!(has_binding, "parameter binding spliced");
        let renamed_puts = inlined.top_level.statements.iter().any(|s| {
            matches!(s, Statement::Call { command, args, .. }
                if command == "puts" && args.iter().any(|a| a.contains("__inl_x_0")))
        });
        assert!(renamed_puts, "body `$x` α-renamed to the mangled parameter");
    }

    #[test]
    fn parameterised_arity_mismatch_keeps_call() {
        // Two args to a one-param proc: not a valid inline (and an error in
        // Tcl) — leave the call for the normal path to handle.
        let module = module_for("proc ::id {x} { puts $x }\nid 1 2");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "id"), 1, "arity mismatch keeps call");
    }

    #[test]
    fn parameterised_arg_command_subst_keeps_call() {
        // An arg with `[cmd]` substitution can't be modelled by the binding.
        let module = module_for("proc ::id {x} { puts $x }\nid [clock seconds]");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "id"), 1, "cmd-subst arg keeps call");
    }

    #[test]
    fn parameterised_with_default_is_not_inlined() {
        // Parameter defaults are the v3 residual — declined.
        let module = module_for("proc ::id {{x 5}} { puts $x }\nid 1");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "id"), 1, "defaulted proc kept");
    }

    #[test]
    fn parameterised_variadic_is_not_inlined() {
        // Variadic `args` needs call-site list packing — the v3 residual.
        let module = module_for("proc ::p {args} { puts $args }\np 1 2 3");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "p"), 1, "variadic proc kept");
    }

    #[test]
    fn rewrite_value_string_cases() {
        let mut rename = HashMap::new();
        rename.insert("a".to_owned(), "__a".to_owned());
        let rw = |s: &str| rewrite_value_string(s, &rename);
        // Bare and brace forms rename.
        assert_eq!(rw("$a"), "$__a");
        assert_eq!(rw("${a}"), "${__a}");
        assert_eq!(rw("x $a y"), "x $__a y");
        // Escaped `$` is a literal — not a substitution.
        assert_eq!(rw("\\$a"), "\\$a");
        // An escaped backslash leaves the `$a` live.
        assert_eq!(rw("\\\\$a"), "\\\\$__a");
        // Array element: base renamed, index preserved.
        assert_eq!(rw("$a(3)"), "$__a(3)");
        assert_eq!(rw("${a}(3)"), "${__a}(3)");
        // Name-boundary: `$ab` is the variable `ab`, not `a`.
        assert_eq!(rw("$ab"), "$ab");
        // Unmapped and namespace names pass through.
        assert_eq!(rw("$b"), "$b");
        assert_eq!(rw("$::g"), "$::g");
        // A lone `$` is literal.
        assert_eq!(rw("price is $"), "price is $");
    }

    #[test]
    fn rename_var_name_preserves_array_tail() {
        let mut rename = HashMap::new();
        rename.insert("arr".to_owned(), "__arr".to_owned());
        assert_eq!(rename_var_name("arr(7)", &rename), "__arr(7)");
        assert_eq!(rename_var_name("arr", &rename), "__arr");
        assert_eq!(rename_var_name("other", &rename), "other");
    }

    #[test]
    fn redefined_proc_is_not_inlined() {
        let module = module_for("proc ::r {} { puts a }\nproc ::r {} { puts b }\nr");
        assert!(module.redefined_procedures.contains("::r"));
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "r"), 1, "redefined proc kept");
    }

    #[test]
    fn arg_command_subst_blocks_verbatim() {
        // `puts [clock seconds]` depends on frame command resolution.
        let module = module_for("proc ::w {} { puts [clock seconds] }\nw");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "w"), 1);
    }

    #[test]
    fn inline_inside_control_flow_body() {
        let module = module_for("proc ::greet {} { puts hi }\nif {1} { greet }");
        let inlined = inline_module(module);
        // The `greet` call inside the `if` body is inlined to `puts hi`.
        let if_has_puts = inlined.top_level.statements.iter().any(|s| {
            matches!(s, Statement::If { clauses, .. }
                if clauses.iter().any(|c| c.body.statements.iter().any(|b|
                    matches!(b, Statement::Call { command, .. } if command == "puts"))))
        });
        assert!(if_has_puts, "call inside if-body should be inlined");
    }

    #[test]
    fn count_statements_walks_nested_bodies() {
        let module = module_for("proc ::f {} { puts a\n if {1} { puts b\n puts c } }");
        let proc = &module.procedures["::f"];
        // `puts a` + the `if` clause (1) + its two `puts` = 4.
        assert_eq!(count_statements(&proc.body), 4);
    }
}
