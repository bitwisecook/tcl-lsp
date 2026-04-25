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

use crate::ir::{IfClause, Module, Procedure, Script, Statement};

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
            clauses
                .iter()
                .any(|c: &IfClause| body_has_frame_reach(&c.body))
                || else_body.as_ref().is_some_and(body_has_frame_reach)
        }
        Statement::For {
            init, next, body, ..
        } => body_has_frame_reach(init) || body_has_frame_reach(next) || body_has_frame_reach(body),
        Statement::While { body, .. } | Statement::Foreach { body, .. } => {
            body_has_frame_reach(body)
        }
        Statement::Catch { body, .. } => body_has_frame_reach(body),
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
}
