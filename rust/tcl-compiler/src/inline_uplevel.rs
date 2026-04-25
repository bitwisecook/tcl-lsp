//! Whole-callee uplevel-passthrough inlining (C34).
//!
//! Detects procs whose entire body is a single
//! [`Statement::UpFrame`] with `frame_shift == 1` (the canonical
//! "shift to caller's frame, evaluate body, restore frame" idiom)
//! and rewrites every callsite to splice the body inline.
//!
//! Mirrors `core/compiler/inline_uplevel.py` from main commit
//! `25a4340e`.
//!
//! This file lands in strips:
//!
//! * **C34b** — [`detect_static_passthrough`]: zero-param,
//!   single-`UpFrame` body, no nested frame-reaching commands.
//! * **C34c** — [`detect_param_body_passthrough`]: single-param
//!   `proc dispatcher {body} { uplevel 1 $body }` shape.
//! * **C34d** — per-callsite rewriter (separate strip — needs the
//!   `Statement::Block` IR variant).
//! * **C34e** — corpus + optimiser-pipeline integration.

use std::collections::HashMap;

use crate::ir::{Module, Procedure, Script, Statement};
use crate::lowering::Lowerer;
use tcl_registry::CommandRegistry;

/// Recognised passthrough proc shape used by the rewriter.
#[derive(Debug, Clone)]
pub enum PassthroughShape {
    /// Zero-param proc whose body is exactly one
    /// `Statement::UpFrame { frame_shift: 1, body, .. }` — splice
    /// `body` directly at every callsite.
    Static {
        /// Pre-lowered body to inline at every callsite.
        body: Script,
    },
    /// Single-param proc whose body is a runtime ``uplevel 1 $param``
    /// call. The rewriter handles this per callsite by parsing the
    /// callsite's brace-literal argument; this enum carries no body
    /// because the body lives at the callsite.
    ParamBody {
        /// Name of the single proc parameter — sanity-checked at
        /// rewrite time (the callsite's literal is what gets inlined,
        /// not the param name).
        param_name: String,
    },
}

/// Walk every procedure in *module* and classify it as a static
/// passthrough candidate, a param-body passthrough candidate, or
/// not a passthrough at all.
///
/// Returns `{qualified_name -> shape}` for every passthrough proc.
#[must_use]
pub fn detect_passthrough_candidates(module: &Module) -> HashMap<String, PassthroughShape> {
    let mut out = HashMap::new();
    for (qname, proc) in &module.procedures {
        if let Some(shape) = classify_passthrough(proc) {
            out.insert(qname.clone(), shape);
        }
    }
    out
}

/// Convenience: only the static (zero-param) shape, used by tests
/// and any caller that hasn't ported C34c yet.
#[must_use]
pub fn detect_static_passthrough(module: &Module) -> HashMap<String, Script> {
    detect_passthrough_candidates(module)
        .into_iter()
        .filter_map(|(k, v)| match v {
            PassthroughShape::Static { body } => Some((k, body)),
            PassthroughShape::ParamBody { .. } => None,
        })
        .collect()
}

/// Classify a single procedure as a passthrough candidate or
/// return `None`.
fn classify_passthrough(proc: &Procedure) -> Option<PassthroughShape> {
    if let Some(body) = static_passthrough_body(proc) {
        return Some(PassthroughShape::Static { body });
    }
    if let Some(param) = param_body_passthrough_param(proc) {
        return Some(PassthroughShape::ParamBody { param_name: param });
    }
    None
}

/// Return the inner body if *proc* is a zero-param `uplevel 1`
/// passthrough — body is exactly one [`Statement::UpFrame`] with
/// `frame_shift == 1` and contains no nested frame-reaching
/// commands.
///
/// Mirrors `core/compiler/inline_uplevel.py::_static_passthrough_body`.
fn static_passthrough_body(proc: &Procedure) -> Option<Script> {
    if !proc.params.is_empty() {
        return None;
    }
    if proc.body.statements.len() != 1 {
        return None;
    }
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift, body, ..
        } if *frame_shift == 1 => {
            if body_has_frame_reach(body) {
                None
            } else {
                Some(body.clone())
            }
        }
        _ => None,
    }
}

/// Return the param name if *proc* matches
/// `proc NAME {P} { uplevel ?1? $P }` — the single-body-param
/// passthrough shape (C34c). Recognises both the bare `uplevel
/// $body` (implicit level 1) form lowered as a `Statement::Call` /
/// `Statement::Barrier` for the outer dispatcher (since the body
/// token is `$var`, the lowering can't relax it to
/// [`Statement::UpFrame`]).
///
/// The actual frame-reach check still runs on the *callsite's*
/// inlined body inside C34d's rewriter — at detector time we only
/// confirm the dispatcher's surface shape.
fn param_body_passthrough_param(proc: &Procedure) -> Option<String> {
    if proc.params.len() != 1 {
        return None;
    }
    let param = &proc.params[0];
    if proc.body.statements.len() != 1 {
        return None;
    }
    let stmt = &proc.body.statements[0];

    // Two surface shapes both lower to a runtime call: ``uplevel
    // $body`` becomes ``Statement::Call`` (or ``Barrier`` when the
    // default lowering treats it as opaque); the explicit
    // ``uplevel 1 $body`` form goes through the same dispatch.
    let (cmd, args) = match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            (command.as_str(), args.as_slice())
        }
        _ => return None,
    };
    if cmd != "uplevel" {
        return None;
    }

    let body_arg = match args.len() {
        // Implicit level 1: ``uplevel $body``.
        1 => &args[0],
        // Explicit ``uplevel 1 $body`` — only level == 1 is
        // recognised; deeper shifts can't be inlined the same way.
        2 if args[0] == "1" => &args[1],
        _ => return None,
    };

    // Body word must be a pure ``$param`` reference to the sole
    // proc parameter.
    let referenced = body_arg.strip_prefix('$')?;
    let referenced = referenced
        .strip_prefix('{')
        .map_or(referenced, |s| s.strip_suffix('}').unwrap_or(referenced));
    if referenced != *param {
        return None;
    }
    Some(param.clone())
}

/// True if *script* contains an `uplevel` / `upvar` / frame-
/// inspecting command that would reach outside the inlined scope.
///
/// Mirrors `core/compiler/inline_uplevel.py::_body_has_frame_reach`.
/// After inlining the body runs in the caller's frame; if the body
/// itself does `uplevel 1 {...}`, that now references the caller's
/// *caller* — a frame the original author may not have anticipated.
/// Reject conservatively.
#[must_use]
pub fn body_has_frame_reach(script: &Script) -> bool {
    script.statements.iter().any(statement_has_frame_reach)
}

fn statement_has_frame_reach(stmt: &Statement) -> bool {
    match stmt {
        Statement::UpFrame { .. } => true,
        Statement::Barrier { command, .. } | Statement::Call { command, .. }
            if matches!(command.as_str(), "uplevel" | "upvar") =>
        {
            true
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|c| body_has_frame_reach(&c.body))
                || else_body.as_ref().is_some_and(body_has_frame_reach)
        }
        Statement::For {
            init, next, body, ..
        } => body_has_frame_reach(init) || body_has_frame_reach(next) || body_has_frame_reach(body),
        Statement::While { body, .. } | Statement::Foreach { body, .. } => {
            body_has_frame_reach(body)
        }
        Statement::Catch { body, .. } | Statement::Block { body, .. } => body_has_frame_reach(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            body_has_frame_reach(body)
                || handlers.iter().any(|h| body_has_frame_reach(&h.body))
                || finally_body.as_ref().is_some_and(body_has_frame_reach)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter()
                .any(|a| a.body.as_ref().is_some_and(body_has_frame_reach))
                || default_body.as_ref().is_some_and(body_has_frame_reach)
        }
        _ => false,
    }
}

/// Rewrite every passthrough callsite in *module* to splice the
/// callee's body inline. Mutates the module in place.
///
/// Detects passthrough candidates via
/// [`detect_passthrough_candidates`] and walks every script
/// (top-level + each procedure body) replacing matching
/// [`Statement::Call`] / [`Statement::Barrier`] nodes with
/// [`Statement::Block`] (static shape) or with the inlined
/// callsite-body literal (param-body shape).
///
/// Safe to call multiple times — already-inlined callsites no
/// longer match the pattern.
///
/// Mirrors `core/compiler/inline_uplevel.py::inline_uplevel_passthrough`.
pub fn inline_uplevel_passthrough(module: &mut Module, registry: &CommandRegistry) {
    let candidates = detect_passthrough_candidates(module);
    if candidates.is_empty() {
        return;
    }
    let mut top = std::mem::take(&mut module.top_level);
    rewrite_script_in_place(&mut top, &candidates, "::", registry);
    module.top_level = top;
    let proc_qnames: Vec<String> = module.procedures.keys().cloned().collect();
    for qname in proc_qnames {
        let caller_ns = namespace_of(&qname);
        if let Some(proc) = module.procedures.get_mut(&qname) {
            let mut body = std::mem::take(&mut proc.body);
            rewrite_script_in_place(&mut body, &candidates, &caller_ns, registry);
            proc.body = body;
        }
    }
}

fn namespace_of(qname: &str) -> String {
    if let Some(idx) = qname.rfind("::") {
        if idx == 0 {
            return "::".to_string();
        }
        let prefix = &qname[..idx];
        return if prefix.starts_with("::") {
            prefix.to_string()
        } else {
            format!("::{prefix}")
        };
    }
    "::".to_string()
}

fn resolve_call_target(command: &str, caller_ns: &str) -> String {
    if command.starts_with("::") {
        return command.to_string();
    }
    if caller_ns == "::" {
        return format!("::{command}");
    }
    format!("{caller_ns}::{command}")
}

fn rewrite_script_in_place(
    script: &mut Script,
    candidates: &HashMap<String, PassthroughShape>,
    namespace: &str,
    registry: &CommandRegistry,
) {
    for stmt in &mut script.statements {
        rewrite_statement_in_place(stmt, candidates, namespace, registry);
    }
}

fn rewrite_statement_in_place(
    stmt: &mut Statement,
    candidates: &HashMap<String, PassthroughShape>,
    namespace: &str,
    registry: &CommandRegistry,
) {
    // First recurse into nested scripts so the inner-most rewrites
    // happen before the outer one — mirrors Python.
    walk_nested_scripts(
        stmt,
        |body, ns| {
            rewrite_script_in_place(body, candidates, ns, registry);
        },
        namespace,
    );

    // Then attempt to rewrite this statement itself if it's a
    // matching callsite.
    let replacement = try_inline_callsite(stmt, candidates, namespace, registry);
    if let Some(new_stmt) = replacement {
        *stmt = new_stmt;
    }
}

fn try_inline_callsite(
    stmt: &Statement,
    candidates: &HashMap<String, PassthroughShape>,
    namespace: &str,
    registry: &CommandRegistry,
) -> Option<Statement> {
    let (command, args, span, tokens) = match stmt {
        Statement::Call {
            command,
            args,
            span,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            span,
            tokens,
            ..
        } if !command.is_empty() => (command.as_str(), args.as_slice(), *span, tokens),
        _ => return None,
    };

    let target = resolve_call_target(command, namespace);
    let shape = candidates
        .get(&target)
        .or_else(|| candidates.get(&format!("::{command}")))?;

    match shape {
        PassthroughShape::Static { body } => {
            if !args.is_empty() {
                return None;
            }
            Some(Statement::Block {
                span,
                body: body.clone(),
                namespace: namespace.to_string(),
                tokens: tokens.clone(),
            })
        }
        PassthroughShape::ParamBody { .. } => {
            // ParamBody: the dispatcher proc is `proc D {body}
            // { uplevel ?1? $body }`. Rewrite a callsite when:
            //   * exactly one argument,
            //   * that argument is a single brace-string token
            //     (`TokenType::Str`) — i.e. the source wrote
            //     ``D {literal-body}``,
            //   * no `{*}`-expansion on any word,
            //   * the materialised body lowers cleanly and
            //     contains no nested frame-reaching commands.
            // Mirrors Python's `_rewrite_stmt`'s ParamBody arm.
            if args.len() != 1 {
                return None;
            }
            let tk = tokens.as_ref()?;
            // argv = [command, body_arg]. Index 1 is the body.
            if tk.argv.len() < 2 || tk.argv_kinds.len() < 2 {
                return None;
            }
            if tk.argv_kinds[1] != tcl_lexer::TokenType::Str {
                return None;
            }
            // ``{*}`` expansion on any word disables the rewrite.
            if tk.expand_word.iter().flatten().any(|&e| e) {
                return None;
            }
            let literal = &args[0];
            let inlined = lower_literal_script(literal, namespace, registry);
            if body_has_frame_reach(&inlined) {
                return None;
            }
            Some(Statement::Block {
                span,
                body: inlined,
                namespace: namespace.to_string(),
                tokens: tokens.clone(),
            })
        }
    }
}

/// Visit every nested [`Script`] field of *stmt* with *visitor*
/// (mutable). Used by the rewriter to recurse into structured
/// statements without enumerating every variant's fields at the
/// call site.
fn walk_nested_scripts<F>(stmt: &mut Statement, mut visitor: F, namespace: &str)
where
    F: FnMut(&mut Script, &str),
{
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses.iter_mut() {
                visitor(&mut c.body, namespace);
            }
            if let Some(b) = else_body.as_mut() {
                visitor(b, namespace);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            visitor(init, namespace);
            visitor(next, namespace);
            visitor(body, namespace);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::UpFrame { body, .. } => {
            visitor(body, namespace);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            visitor(body, namespace);
            for h in handlers.iter_mut() {
                visitor(&mut h.body, namespace);
            }
            if let Some(f) = finally_body.as_mut() {
                visitor(f, namespace);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms.iter_mut() {
                if let Some(b) = a.body.as_mut() {
                    visitor(b, namespace);
                }
            }
            if let Some(d) = default_body.as_mut() {
                visitor(d, namespace);
            }
        }
        Statement::Block {
            body,
            namespace: ns,
            ..
        } => {
            // Block carries its own namespace; recurse with that.
            let inner_ns = ns.clone();
            visitor(body, &inner_ns);
        }
        _ => {}
    }
}

/// Lower a brace-literal script into a [`Script`] for the inline
/// rewriter. Used by the `ParamBody` rewrite path to materialise
/// the callsite's brace-literal body before splicing.
fn lower_literal_script(literal: &str, namespace: &str, registry: &CommandRegistry) -> Script {
    let mut lowerer = Lowerer::new(registry);
    lowerer.lower_into_script(literal, namespace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn zero_param_static_passthrough_detected() {
        let m = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }", &reg());
        let candidates = detect_static_passthrough(&m);
        assert_eq!(candidates.len(), 1);
        assert!(candidates.contains_key("::reset"));
    }

    #[test]
    fn zero_param_with_extra_statement_is_not_passthrough() {
        let m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0}\n puts done }",
            &reg(),
        );
        assert!(detect_static_passthrough(&m).is_empty());
    }

    #[test]
    fn proc_with_params_is_not_static_passthrough() {
        let m = lower_to_ir("proc reset {x} { uplevel 1 {set counter 0} }", &reg());
        assert!(detect_static_passthrough(&m).is_empty());
    }

    #[test]
    fn nested_uplevel_blocks_static_passthrough() {
        let m = lower_to_ir(
            "proc reset {} { uplevel 1 {uplevel 1 {set counter 0}} }",
            &reg(),
        );
        assert!(detect_static_passthrough(&m).is_empty());
    }

    #[test]
    fn nested_upvar_call_blocks_static_passthrough() {
        let m = lower_to_ir("proc bind {} { uplevel 1 {upvar foo bar} }", &reg());
        assert!(detect_static_passthrough(&m).is_empty());
    }

    #[test]
    fn frame_shift_zero_is_not_static_passthrough() {
        // ``uplevel #0`` shifts to absolute global frame — can't be
        // expressed as a same-frame inline.
        let m = lower_to_ir("proc reset {} { uplevel #0 {set counter 0} }", &reg());
        assert!(detect_static_passthrough(&m).is_empty());
    }

    #[test]
    fn param_body_passthrough_detected() {
        let m = lower_to_ir("proc dispatcher {body} { uplevel 1 $body }", &reg());
        let candidates = detect_passthrough_candidates(&m);
        let shape = candidates.get("::dispatcher").expect("expected candidate");
        match shape {
            PassthroughShape::ParamBody { param_name } => assert_eq!(param_name, "body"),
            PassthroughShape::Static { .. } => panic!("expected ParamBody, got Static"),
        }
    }

    #[test]
    fn param_body_passthrough_implicit_level_one() {
        // ``uplevel $body`` (no explicit level) defaults to 1 so
        // it matches the same shape.
        let m = lower_to_ir("proc dispatcher {body} { uplevel $body }", &reg());
        let candidates = detect_passthrough_candidates(&m);
        assert!(matches!(
            candidates.get("::dispatcher"),
            Some(PassthroughShape::ParamBody { .. })
        ));
    }

    #[test]
    fn param_body_passthrough_two_params_rejected() {
        let m = lower_to_ir("proc dispatcher {body extra} { uplevel 1 $body }", &reg());
        assert!(detect_passthrough_candidates(&m).is_empty());
    }

    #[test]
    fn param_body_passthrough_wrong_param_rejected() {
        // ``$other`` isn't the proc's parameter — mismatch.
        let m = lower_to_ir("proc dispatcher {body} { uplevel 1 $other }", &reg());
        assert!(detect_passthrough_candidates(&m).is_empty());
    }

    #[test]
    fn body_with_only_assignment_has_no_frame_reach() {
        let m = lower_to_ir("set x 1", &reg());
        assert!(!body_has_frame_reach(&m.top_level));
    }

    fn count_blocks(script: &Script) -> usize {
        let mut n = 0;
        for stmt in &script.statements {
            if let Statement::Block { .. } = stmt {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn rewriter_inlines_zero_param_passthrough_callsite() {
        let mut m = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        inline_uplevel_passthrough(&mut m, &reg());
        // The callsite ``reset`` should now be a Statement::Block
        // splicing in the body.
        assert_eq!(count_blocks(&m.top_level), 1);
    }

    #[test]
    fn rewriter_skips_callsites_with_args() {
        // Static-shape candidates are zero-param; a call passing
        // an argument should not be rewritten (would change
        // semantics).
        let mut m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0} }\nreset extra",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_unknown_callees() {
        // Calling a non-passthrough proc shouldn't be touched.
        let mut m = lower_to_ir("proc helper {} { puts hi }\nhelper", &reg());
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_recurses_into_if_body() {
        let mut m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0} }\nif {1} { reset }",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        // ``proc`` itself emits a ``Call`` at top level (registers
        // the proc but also keeps a ``proc`` invocation statement);
        // the ``if`` follows. Find the ``If`` statement and confirm
        // the inner block contains the inlined ``Block``.
        let if_stmt = m
            .top_level
            .statements
            .iter()
            .find(|s| matches!(s, Statement::If { .. }))
            .expect("expected an If statement");
        match if_stmt {
            Statement::If { clauses, .. } => {
                assert_eq!(count_blocks(&clauses[0].body), 1);
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn rewriter_is_idempotent() {
        // Running the pass twice yields the same module — once
        // inlined, the callsite no longer matches the pattern.
        let mut m1 = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        inline_uplevel_passthrough(&mut m1, &reg());
        let after_first = m1.clone();
        inline_uplevel_passthrough(&mut m1, &reg());
        assert_eq!(
            after_first.top_level.statements.len(),
            m1.top_level.statements.len()
        );
    }

    #[test]
    fn rewriter_with_no_candidates_is_noop() {
        let mut m = lower_to_ir("set x 1\nputs $x", &reg());
        let before = m.clone();
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(
            before.top_level.statements.len(),
            m.top_level.statements.len()
        );
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_inlines_param_body_brace_literal() {
        // C34d-paramBody: ``proc dispatcher {body} { uplevel 1 $body }``
        // followed by ``dispatcher {set counter 0}`` rewrites the
        // callsite to a Block containing the parsed literal body.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {set counter 0}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        // Find the rewritten block — top-level has [proc-call,
        // dispatcher-call→Block, …].
        let block = m
            .top_level
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Block { .. }))
            .expect("expected a Block from the ParamBody rewrite");
        if let Statement::Block { body, .. } = block {
            assert!(!body.statements.is_empty(), "block body should be lowered");
        }
    }

    #[test]
    fn rewriter_skips_param_body_with_dynamic_arg() {
        // ``dispatcher $dyn`` — the arg is a $var, not a brace
        // literal. Stay on the dispatch path.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher $dyn",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_with_command_subst_arg() {
        // ``dispatcher [build_body]`` — Cmd token, not a brace
        // literal. Stay on the dispatch path.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher [build_body]",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_when_inner_has_frame_reach() {
        // The inlined body contains an ``uplevel 1`` — semantics
        // would change after inlining, refuse.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {uplevel 1 {set x 1}}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_with_expand_word() {
        // ``dispatcher {*}$args`` — expansion defeats the
        // single-arg analysis.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {*}$args",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }
}
