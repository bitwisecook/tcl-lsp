//! Interprocedural analysis — per-procedure summaries and
//! call-target resolution.
//!
//! Ported from `core/compiler/interprocedural.py` (C28). This
//! strip lands the summary types (`ProcSummary`, `MethodSummary`,
//! `InterproceduralAnalysis`) plus the call-target resolver. The
//! full summary-building pipeline (effect tracking, constant-
//! return inference, parameter-trait analysis) is a follow-up
//! that plugs into the C23 side-effect classifier and the C25
//! SCCP evaluator.

#![allow(clippy::struct_excessive_bools, clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use crate::naming::normalise_qualified_name;
use crate::side_effects::EffectRegion;

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

/// A Tcl procedure's arity as declared in `proc name {args} …`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Arity {
    /// Minimum number of arguments.
    pub min: u32,
    /// Maximum number of arguments (`u32::MAX` for variadic).
    pub max: u32,
}

impl Arity {
    /// Arity accepting any number of arguments.
    #[must_use]
    pub const fn any() -> Self {
        Self { min: 0, max: u32::MAX }
    }

    /// Exact-arity constraint — proc takes `n` arguments.
    #[must_use]
    pub const fn exact(n: u32) -> Self {
        Self { min: n, max: n }
    }
}

/// Interprocedural argument trait. Documents how a parameter is
/// used inside the callee — consumed by the optimiser for
/// parameter-specific reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcArgTrait {
    /// Parameter text is substituted into the return value
    /// unchanged.
    Passthrough,
    /// Parameter participates in a comparison that gates control
    /// flow.
    UsedInCondition,
    /// Parameter is forwarded to another procedure.
    ForwardedToCallee,
    /// Parameter is never read.
    Unused,
}

/// A proven-constant return value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantReturn {
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// Boolean (rendered as `"true"` / `"false"`).
    Bool(bool),
    /// String.
    Str(String),
}

/// Per-procedure summary of interprocedural facts.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcSummary {
    /// Fully-qualified procedure name.
    pub qualified_name: String,
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// Declared arity.
    pub arity: Arity,
    /// Names of procedures this one calls (transitive closure).
    pub calls: Vec<String>,
    /// True if the body contains a barrier command.
    pub has_barrier: bool,
    /// True if the body calls a command not in the registry and
    /// not resolvable to another internal proc.
    pub has_unknown_calls: bool,
    /// True if the body writes any global / namespace variable.
    pub writes_global: bool,
    /// True if the body is side-effect-free.
    pub pure: bool,
    /// Effect regions this proc (or its callees) may read.
    pub effect_reads: EffectRegion,
    /// Effect regions this proc (or its callees) may write.
    pub effect_writes: EffectRegion,
    /// True if every return in the body yields the same constant.
    pub returns_constant: bool,
    /// The constant return value when `returns_constant` is true.
    pub constant_return: Option<ConstantReturn>,
    /// Names of parameters whose value influences the return.
    pub return_depends_on_params: Vec<String>,
    /// When set, the return value is exactly the parameter named.
    pub return_passthrough_param: Option<String>,
    /// Whether this proc is eligible for static constant folding.
    pub can_fold_static_calls: bool,
    /// Per-parameter traits.
    pub param_traits: HashMap<String, HashSet<ProcArgTrait>>,
}

impl ProcSummary {
    /// Build a default summary with conservative values — useful
    /// for stubbing callees whose bodies haven't been analysed.
    #[must_use]
    pub fn unknown(qualified_name: impl Into<String>) -> Self {
        Self {
            qualified_name: qualified_name.into(),
            params: Vec::new(),
            arity: Arity::any(),
            calls: Vec::new(),
            has_barrier: false,
            has_unknown_calls: true,
            writes_global: true,
            pure: false,
            effect_reads: EffectRegion::UNKNOWN_STATE,
            effect_writes: EffectRegion::UNKNOWN_STATE,
            returns_constant: false,
            constant_return: None,
            return_depends_on_params: Vec::new(),
            return_passthrough_param: None,
            can_fold_static_calls: false,
            param_traits: HashMap::new(),
        }
    }
}

/// Extended summary for OO methods with class context.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSummary {
    /// Base procedure summary fields.
    pub base: ProcSummary,
    /// Name of the containing class.
    pub class_name: String,
    /// Method kind: `"method"` / `"classmethod"` / `"constructor"` /
    /// `"destructor"`.
    pub method_kind: String,
    /// Instance variables the method reads.
    pub reads_instance_vars: HashSet<String>,
    /// Instance variables the method writes.
    pub writes_instance_vars: HashSet<String>,
    /// Names of methods called via `my method`.
    pub calls_my: Vec<String>,
    /// True if the method calls `next` (MRO chain dispatch).
    pub calls_next: bool,
}

/// Result of running interprocedural analysis on a module.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InterproceduralAnalysis {
    /// Per-procedure summaries keyed by qualified name.
    pub procedures: HashMap<String, ProcSummary>,
    /// Per-method summaries keyed by qualified name.
    pub methods: HashMap<String, MethodSummary>,
}

// ---------------------------------------------------------------------------
// Call-target resolution
// ---------------------------------------------------------------------------

/// Resolve a command name to a qualified procedure name if it
/// refers to one defined in `known`.
///
/// Rules mirror Tcl's name resolution: absolute names (starting
/// with `::`) are looked up directly; names containing `::` but
/// not starting with it are treated as global-relative; bare
/// names are resolved by walking up the caller's namespace path.
#[must_use]
pub fn resolve_internal_call(
    command: &str,
    caller_qname: &str,
    known: &HashSet<String>,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }

    if command.starts_with("::") {
        let qname = normalise_qualified_name(command);
        return known.contains(qname.as_str()).then_some(qname);
    }

    if command.contains("::") {
        let qname = normalise_qualified_name(&format!("::{command}"));
        return known.contains(qname.as_str()).then_some(qname);
    }

    let ns_parts = namespace_parts_from_proc(caller_qname);
    for depth in (0..=ns_parts.len()).rev() {
        let mut candidate = String::from("::");
        for (i, part) in ns_parts[..depth].iter().enumerate() {
            if i > 0 {
                candidate.push_str("::");
            }
            candidate.push_str(part);
        }
        if depth > 0 {
            candidate.push_str("::");
        }
        candidate.push_str(command);
        let qname = normalise_qualified_name(&candidate);
        if known.contains(qname.as_str()) {
            return Some(qname);
        }
    }
    None
}

/// Top-level call-target resolver. Convenience wrapper that
/// handles the common case where the caller has no special
/// aliasing information.
#[must_use]
pub fn resolve_call_target(
    command: &str,
    _args: &[String],
    caller_qname: &str,
    known: &HashSet<String>,
) -> Option<String> {
    resolve_internal_call(command, caller_qname, known)
}

/// Return the namespace segments of a qualified proc name —
/// everything except the trailing simple name.
#[must_use]
pub fn namespace_parts_from_proc(qname: &str) -> Vec<String> {
    let normalised = normalise_qualified_name(qname);
    let parts: Vec<&str> = normalised.split("::").filter(|p| !p.is_empty()).collect();
    if parts.len() <= 1 {
        return Vec::new();
    }
    parts[..parts.len() - 1]
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn arity_helpers() {
        assert_eq!(Arity::any(), Arity { min: 0, max: u32::MAX });
        assert_eq!(Arity::exact(3), Arity { min: 3, max: 3 });
    }

    #[test]
    fn proc_summary_unknown_is_conservative() {
        let s = ProcSummary::unknown("::mystery");
        assert!(s.has_unknown_calls);
        assert!(s.writes_global);
        assert!(!s.pure);
        assert_eq!(s.effect_reads, EffectRegion::UNKNOWN_STATE);
        assert_eq!(s.effect_writes, EffectRegion::UNKNOWN_STATE);
    }

    #[test]
    fn resolve_absolute_names() {
        let known = known_set(&["::foo::bar"]);
        assert_eq!(
            resolve_internal_call("::foo::bar", "::top", &known),
            Some("::foo::bar".into())
        );
        // Absolute name not in the known set returns None.
        assert_eq!(
            resolve_internal_call("::foo::missing", "::top", &known),
            None
        );
    }

    #[test]
    fn resolve_relative_with_segments() {
        // `foo::bar` from any caller → `::foo::bar`.
        let known = known_set(&["::foo::bar"]);
        assert_eq!(
            resolve_internal_call("foo::bar", "::top", &known),
            Some("::foo::bar".into())
        );
    }

    #[test]
    fn resolve_bare_walks_caller_namespace() {
        // caller `::ns::a::caller` + bare `helper` → try
        // `::ns::a::helper`, `::ns::helper`, `::helper` in order.
        let known = known_set(&["::ns::helper"]);
        assert_eq!(
            resolve_internal_call("helper", "::ns::a::caller", &known),
            Some("::ns::helper".into())
        );
    }

    #[test]
    fn resolve_bare_falls_through_to_global() {
        let known = known_set(&["::helper"]);
        assert_eq!(
            resolve_internal_call("helper", "::ns::caller", &known),
            Some("::helper".into())
        );
    }

    #[test]
    fn resolve_bare_returns_none_when_not_found() {
        let known = known_set(&["::other"]);
        assert_eq!(
            resolve_internal_call("helper", "::ns::caller", &known),
            None
        );
    }

    #[test]
    fn resolve_empty_command_is_none() {
        let known = known_set(&["::helper"]);
        assert_eq!(resolve_internal_call("", "::top", &known), None);
    }

    #[test]
    fn namespace_parts_from_proc_extracts_segments() {
        assert_eq!(namespace_parts_from_proc("::foo::bar::baz"), vec!["foo", "bar"]);
        assert_eq!(namespace_parts_from_proc("::simple"), Vec::<String>::new());
        assert_eq!(namespace_parts_from_proc("::"), Vec::<String>::new());
    }

    #[test]
    fn resolve_call_target_delegates() {
        let known = known_set(&["::helper"]);
        assert_eq!(
            resolve_call_target("helper", &[], "::top", &known),
            Some("::helper".into())
        );
    }
}
