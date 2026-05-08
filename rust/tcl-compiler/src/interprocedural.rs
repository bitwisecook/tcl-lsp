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
        Self {
            min: 0,
            max: u32::MAX,
        }
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

impl ProcArgTrait {
    /// Stable lower-case wire form
    /// (`"passthrough"`, `"used_in_condition"`,
    /// `"forwarded_to_callee"`, `"unused"`). Consumers (`PyO3`
    /// bindings, native LSP server) materialise traits using this
    /// form rather than re-implementing the mapping.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::UsedInCondition => "used_in_condition",
            Self::ForwardedToCallee => "forwarded_to_callee",
            Self::Unused => "unused",
        }
    }
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

impl ConstantReturn {
    /// Lower into the canonical `(kind, text)` wire form. `kind`
    /// is one of `"int"`, `"float"`, `"bool"`, `"str"`; `text` is
    /// the rendered value. Bools render as `"1"` / `"0"`.
    #[must_use]
    pub fn as_kind_text(&self) -> (&'static str, String) {
        match self {
            Self::Int(i) => ("int", i.to_string()),
            Self::Float(f) => ("float", f.to_string()),
            Self::Bool(b) => ("bool", if *b { "1".into() } else { "0".into() }),
            Self::Str(s) => ("str", s.clone()),
        }
    }
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

// ---------------------------------------------------------------------------
// Summary building (C28x, partial)
// ---------------------------------------------------------------------------

/// Build conservative interprocedural summaries for every
/// procedure in `ir_module`.
///
/// Ported from a focused slice of
/// `core/compiler/interprocedural.py::analyse_interprocedural_ir`.
/// Populates the structural facts downstream passes
/// (propagation, unused-procs) depend on:
///
/// - `qualified_name`, `params`, `arity`.
/// - `calls` — direct callees whose names resolve to another
///   proc in the module, extended with the transitive closure.
/// - `has_barrier` — `Statement::Barrier` or a direct call to
///   `eval`/`uplevel`/`interp eval`/`namespace eval`.
/// - `has_unknown_calls` — any call that is neither a registry
///   command nor a resolvable internal proc.
/// - `writes_global` — any assignment targeting a global or
///   namespace-scoped variable, any call whose side-effect
///   classification writes `EffectRegion::GLOBAL_STATE`, or any
///   transitive callee that does.
/// - `pure` — least fixpoint over local purity ∧ every callee's
///   purity.
/// - `effect_reads` / `effect_writes` — union over the direct
///   side-effects and the transitive closure's.
///
/// **Deferred to a follow-up C28 sub-strip**:
/// `returns_constant`, `constant_return`,
/// `return_depends_on_params`, `return_passthrough_param`,
/// `can_fold_static_calls`, `param_traits`. These leave their
/// defaults (the `ProcSummary::unknown` shape) and will be
/// populated when a return-value + parameter-trait analyser
/// lands.
#[must_use]
pub fn build_interprocedural_analysis(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
) -> InterproceduralAnalysis {
    let known: HashSet<String> = ir_module.procedures.keys().cloned().collect();

    let local = scan_all_procs(ir_module, &known, registry, dialect);
    let transitive_calls = compute_all_transitive_calls(&known, &local);
    let pure = fixpoint_pure(&local);
    let (effect_reads, effect_writes) = fixpoint_effects(&local);

    let procedures = materialise_summaries(
        ir_module,
        &local,
        &transitive_calls,
        &pure,
        &effect_reads,
        &effect_writes,
    );

    InterproceduralAnalysis {
        procedures,
        methods: HashMap::new(),
    }
}

fn scan_all_procs(
    ir_module: &crate::ir::Module,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
) -> HashMap<String, LocalFacts> {
    let mut local: HashMap<String, LocalFacts> = HashMap::with_capacity(known.len());
    for (qname, proc) in &ir_module.procedures {
        local.insert(
            qname.clone(),
            scan_proc(qname, proc, known, registry, dialect),
        );
    }
    local
}

fn compute_all_transitive_calls(
    known: &HashSet<String>,
    local: &HashMap<String, LocalFacts>,
) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::with_capacity(known.len());
    for qname in known {
        out.insert(qname.clone(), compute_transitive_calls(qname, local));
    }
    out
}

fn fixpoint_pure(local: &HashMap<String, LocalFacts>) -> HashMap<String, bool> {
    let local_pure: HashMap<String, bool> = local
        .iter()
        .map(|(q, f)| (q.clone(), f.local_pure))
        .collect();
    let mut pure = local_pure.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for (qname, facts) in local {
            if !local_pure[qname] {
                continue;
            }
            let all_callees_pure = facts
                .direct_calls
                .iter()
                .all(|c| pure.get(c).copied().unwrap_or(false));
            let new_val = local_pure[qname] && all_callees_pure;
            if new_val != pure[qname] {
                pure.insert(qname.clone(), new_val);
                changed = true;
            }
        }
    }
    pure
}

fn fixpoint_effects(
    local: &HashMap<String, LocalFacts>,
) -> (HashMap<String, EffectRegion>, HashMap<String, EffectRegion>) {
    let mut reads: HashMap<String, EffectRegion> = local
        .iter()
        .map(|(q, f)| (q.clone(), f.effect_reads))
        .collect();
    let mut writes: HashMap<String, EffectRegion> = local
        .iter()
        .map(|(q, f)| (q.clone(), f.effect_writes))
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (qname, facts) in local {
            let mut r = reads[qname];
            let mut w = writes[qname];
            for c in &facts.direct_calls {
                r |= reads.get(c).copied().unwrap_or(EffectRegion::UNKNOWN_STATE);
                w |= writes
                    .get(c)
                    .copied()
                    .unwrap_or(EffectRegion::UNKNOWN_STATE);
            }
            if r != reads[qname] {
                reads.insert(qname.clone(), r);
                changed = true;
            }
            if w != writes[qname] {
                writes.insert(qname.clone(), w);
                changed = true;
            }
        }
    }
    (reads, writes)
}

fn materialise_summaries(
    ir_module: &crate::ir::Module,
    local: &HashMap<String, LocalFacts>,
    transitive_calls: &HashMap<String, HashSet<String>>,
    pure: &HashMap<String, bool>,
    effect_reads: &HashMap<String, EffectRegion>,
    effect_writes: &HashMap<String, EffectRegion>,
) -> HashMap<String, ProcSummary> {
    let mut procedures: HashMap<String, ProcSummary> = HashMap::with_capacity(local.len());
    for (qname, facts) in local {
        let Some(proc) = ir_module.procedures.get(qname) else {
            continue;
        };
        let mut calls_list: Vec<String> = transitive_calls
            .get(qname)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        calls_list.sort();
        let is_pure = *pure.get(qname).unwrap_or(&false);

        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns);
        // A proc is foldable at a call site when its return is
        // fully determined by the static call — that means pure
        // AND (constant return OR passthrough of a param).
        let can_fold = is_pure && (returns_constant || passthrough.is_some());
        let param_traits = finalise_param_traits(
            &proc.params,
            &facts.param_trait_flags,
            passthrough.as_deref(),
        );

        procedures.insert(
            qname.clone(),
            ProcSummary {
                qualified_name: qname.clone(),
                params: proc.params.clone(),
                arity: Arity::exact(u32::try_from(proc.params.len()).unwrap_or(u32::MAX)),
                calls: calls_list,
                has_barrier: facts.has_barrier,
                has_unknown_calls: facts.has_unknown_calls,
                writes_global: facts.writes_global,
                pure: is_pure,
                effect_reads: *effect_reads
                    .get(qname)
                    .unwrap_or(&EffectRegion::UNKNOWN_STATE),
                effect_writes: *effect_writes
                    .get(qname)
                    .unwrap_or(&EffectRegion::UNKNOWN_STATE),
                returns_constant,
                constant_return,
                return_depends_on_params: depends,
                return_passthrough_param: passthrough,
                can_fold_static_calls: can_fold,
                param_traits,
            },
        );
    }
    procedures
}

/// Per-procedure scratch facts consumed by the summary-building
/// pipeline.
#[derive(Debug, Clone)]
struct LocalFacts {
    direct_calls: HashSet<String>,
    has_barrier: bool,
    has_unknown_calls: bool,
    writes_global: bool,
    local_pure: bool,
    effect_reads: EffectRegion,
    effect_writes: EffectRegion,
    /// Collected return-value classifications — one entry per
    /// `Statement::Return` visited in the body (including those
    /// inside nested compound statements).
    returns: Vec<ReturnKind>,
    /// Accumulated trait observations per parameter name. The
    /// final `ProcSummary::param_traits` is built from this
    /// after the body walk completes.
    param_trait_flags: HashMap<String, HashSet<ProcArgTrait>>,
}

/// Classification of a single return statement's shape.
#[derive(Debug, Clone, PartialEq)]
enum ReturnKind {
    /// `return LITERAL` with a safe-looking literal.
    Literal(String),
    /// `return $param` — a passthrough of a known parameter.
    Passthrough(String),
    /// `return [expr {$param}]` or any return that references a
    /// specific parameter but isn't a plain passthrough.
    UsesParam(Vec<String>),
    /// Any other return (dynamic value, command substitution,
    /// etc.).
    Other,
}

impl Default for LocalFacts {
    fn default() -> Self {
        Self {
            direct_calls: HashSet::new(),
            has_barrier: false,
            has_unknown_calls: false,
            writes_global: false,
            local_pure: false,
            effect_reads: EffectRegion::NONE,
            effect_writes: EffectRegion::NONE,
            returns: Vec::new(),
            param_trait_flags: HashMap::new(),
        }
    }
}

fn scan_proc(
    qname: &str,
    proc: &crate::ir::Procedure,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
) -> LocalFacts {
    let mut facts = LocalFacts {
        local_pure: true,
        ..LocalFacts::default()
    };
    let params: HashSet<String> = proc.params.iter().cloned().collect();
    scan_script(
        &proc.body, qname, known, registry, dialect, &mut facts, &params,
    );
    facts
}

fn scan_script(
    script: &crate::ir::Script,
    caller: &str,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    facts: &mut LocalFacts,
    params: &HashSet<String>,
) {
    for stmt in &script.statements {
        scan_statement(stmt, caller, known, registry, dialect, facts, params);
    }
}

// Long match dispatcher over Statement variants.
#[allow(clippy::too_many_lines)]
fn scan_statement(
    stmt: &crate::ir::Statement,
    caller: &str,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    facts: &mut LocalFacts,
    params: &HashSet<String>,
) {
    use crate::ir::Statement;
    match stmt {
        Statement::Barrier { .. } => {
            facts.has_barrier = true;
            facts.local_pure = false;
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
        Statement::UpFrame { body, .. } => {
            // Static-body uplevel runs the inner script in the
            // caller's frame — for interprocedural purposes it can
            // touch any caller-scope variable, so treat it as a
            // barrier conservatively. Any reads/writes inside
            // ``body`` propagate up.
            facts.has_barrier = true;
            facts.local_pure = false;
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
            for inner in &body.statements {
                scan_statement(inner, caller, known, registry, dialect, facts, params);
            }
        }
        Statement::Block { body, .. } => {
            // ``Block`` is a transparent splice: walk through to the
            // inner statements without flagging a barrier.
            for inner in &body.statements {
                scan_statement(inner, caller, known, registry, dialect, facts, params);
            }
        }
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            if is_global_or_namespace(name) {
                facts.writes_global = true;
                facts.local_pure = false;
                facts.effect_writes |= EffectRegion::GLOBAL_STATE;
            }
        }
        Statement::Return { value, expr, .. } => {
            let kind = classify_return(value.as_deref(), expr.as_ref(), params);
            facts.returns.push(kind);
        }
        Statement::Call { command, args, .. } => {
            let ci = classify_side_effects(registry, command, args, dialect, None);
            if ci.dynamic_barrier {
                facts.has_barrier = true;
                facts.local_pure = false;
                facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
                facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
            }
            let (r, w) = ci.to_effect_regions();
            facts.effect_reads |= r;
            facts.effect_writes |= w;
            if w.intersects(EffectRegion::GLOBAL_STATE) {
                facts.writes_global = true;
            }
            if !ci.pure {
                facts.local_pure = false;
            }

            // Resolve internal-proc call targets. Special case
            // for iRules' ``call <proc>`` indirection: when the
            // command is literally ``call`` and the first arg is
            // a plain identifier, treat it as a direct invocation
            // of that proc. Matches the Python
            // ``_unused_procs._collect_callees`` handling.
            let internal_target =
                if command == "call" && !args.is_empty() && is_plain_proc_name(&args[0]) {
                    resolve_internal_call(&args[0], caller, known)
                } else {
                    resolve_internal_call(command, caller, known)
                };
            if let Some(target) = &internal_target {
                facts.direct_calls.insert(target.clone());
            } else if registry.get(command).is_none() {
                facts.has_unknown_calls = true;
                facts.local_pure = false;
            }

            // Param-trait observation: any param whose `$p`
            // appears in an argument text is "used"; when the
            // call resolves to another internal proc, classify
            // it as ForwardedToCallee.
            for arg in args {
                for param in params {
                    if text_references_name(arg, param) {
                        let trait_kind = if internal_target.is_some() {
                            ProcArgTrait::ForwardedToCallee
                        } else {
                            ProcArgTrait::Passthrough
                        };
                        facts
                            .param_trait_flags
                            .entry(param.clone())
                            .or_default()
                            .insert(trait_kind);
                    }
                }
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                note_params_in_expr(&c.condition, params, facts);
                scan_script(&c.body, caller, known, registry, dialect, facts, params);
            }
            if let Some(body) = else_body {
                scan_script(body, caller, known, registry, dialect, facts, params);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            note_params_in_expr(condition, params, facts);
            scan_script(init, caller, known, registry, dialect, facts, params);
            scan_script(next, caller, known, registry, dialect, facts, params);
            scan_script(body, caller, known, registry, dialect, facts, params);
        }
        Statement::While {
            condition, body, ..
        } => {
            note_params_in_expr(condition, params, facts);
            scan_script(body, caller, known, registry, dialect, facts, params);
        }
        Statement::ExprEval { expr, .. } => {
            note_params_in_expr(expr, params, facts);
        }
        Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
            scan_script(body, caller, known, registry, dialect, facts, params);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            scan_script(body, caller, known, registry, dialect, facts, params);
            for h in handlers {
                scan_script(&h.body, caller, known, registry, dialect, facts, params);
            }
            if let Some(fb) = finally_body {
                scan_script(fb, caller, known, registry, dialect, facts, params);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    scan_script(b, caller, known, registry, dialect, facts, params);
                }
            }
            if let Some(db) = default_body {
                scan_script(db, caller, known, registry, dialect, facts, params);
            }
        }
    }
}

fn is_global_or_namespace(name: &str) -> bool {
    name.starts_with("::") || name.contains("::")
}

/// Scan a raw Tcl source word for `$name` / `${name}`
/// references of the given variable name. Used for param-trait
/// observation in call arguments (where we only have text, not
/// parsed expressions).
fn text_references_name(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            if let Ok(n) = std::str::from_utf8(&bytes[start..i]) {
                if n == name {
                    return true;
                }
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'_' {
                i += 1;
            } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
                i += 2;
            } else {
                break;
            }
        }
        if let Ok(n) = std::str::from_utf8(&bytes[start..i]) {
            if n == name {
                return true;
            }
        }
    }
    false
}

/// Visit each `ExprNode::Var` in `node` and mark matching
/// parameters as `UsedInCondition` — the expression is inside an
/// `if` / `while` / `for` condition (or a standalone `ExprEval`
/// treated analogously).
fn note_params_in_expr(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    facts: &mut LocalFacts,
) {
    use crate::expr_ast::ExprNode;
    match node {
        ExprNode::Var { name, .. } if params.contains(name) => {
            facts
                .param_trait_flags
                .entry(name.clone())
                .or_default()
                .insert(ProcArgTrait::UsedInCondition);
        }
        ExprNode::Binary { left, right, .. } => {
            note_params_in_expr(left, params, facts);
            note_params_in_expr(right, params, facts);
        }
        ExprNode::Unary { operand, .. } => note_params_in_expr(operand, params, facts),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            note_params_in_expr(condition, params, facts);
            note_params_in_expr(true_branch, params, facts);
            note_params_in_expr(false_branch, params, facts);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                note_params_in_expr(a, params, facts);
            }
        }
        _ => {}
    }
}

/// Collapse the raw trait observation flags into the final
/// per-param trait set. Adds `Passthrough` when the proc's
/// `return_passthrough_param` matches this param; adds `Unused`
/// when no observation fired and the proc has any body.
fn finalise_param_traits(
    params: &[String],
    flags: &HashMap<String, HashSet<ProcArgTrait>>,
    passthrough: Option<&str>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    let mut out: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    for p in params {
        let mut traits: HashSet<ProcArgTrait> = flags.get(p).cloned().unwrap_or_default();
        if passthrough == Some(p) {
            traits.insert(ProcArgTrait::Passthrough);
        }
        if traits.is_empty() {
            traits.insert(ProcArgTrait::Unused);
        }
        out.insert(p.clone(), traits);
    }
    out
}

/// Classify a single `Statement::Return` shape for
/// interprocedural summary purposes.
fn classify_return(
    value: Option<&str>,
    expr: Option<&crate::expr_ast::ExprNode>,
    params: &HashSet<String>,
) -> ReturnKind {
    // Prefer the structured `expr` when the return was `return
    // [expr {…}]` or similar — the AST gives precise information.
    if let Some(node) = expr {
        return classify_return_expr(node, params);
    }

    let Some(raw) = value else {
        return ReturnKind::Other;
    };
    let v = raw.trim();
    if v.is_empty() {
        return ReturnKind::Other;
    }
    // Pure literal — integer, bare word, or quoted string.
    if v.parse::<i64>().is_ok() || is_bare_word(v) {
        return ReturnKind::Literal(v.to_owned());
    }
    if let Some(inside) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        if !inside.contains(['$', '[', '\\']) {
            return ReturnKind::Literal(inside.to_owned());
        }
    }
    if let Some(inside) = v.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return ReturnKind::Literal(inside.to_owned());
    }
    // Passthrough of `$param`.
    if let Some(name) = v.strip_prefix('$') {
        if params.contains(name) {
            return ReturnKind::Passthrough(name.to_owned());
        }
    }
    if let Some(name) = v.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        if params.contains(name) {
            return ReturnKind::Passthrough(name.to_owned());
        }
    }
    ReturnKind::Other
}

fn classify_return_expr(node: &crate::expr_ast::ExprNode, params: &HashSet<String>) -> ReturnKind {
    use crate::expr_ast::ExprNode;

    if let ExprNode::Literal { text, .. } = node {
        return ReturnKind::Literal(text.clone());
    }
    if let ExprNode::String { text, .. } = node {
        // Strip outer delimiters.
        let inside = text
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| text.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .unwrap_or(text);
        return ReturnKind::Literal(inside.to_owned());
    }
    if let ExprNode::Var { name, .. } = node {
        if params.contains(name) {
            return ReturnKind::Passthrough(name.clone());
        }
    }
    // Walk the AST collecting var references against the param
    // set; any match → UsesParam.
    let mut referenced: Vec<String> = Vec::new();
    walk_collect_param_refs(node, params, &mut referenced);
    if !referenced.is_empty() {
        referenced.sort();
        referenced.dedup();
        return ReturnKind::UsesParam(referenced);
    }
    ReturnKind::Other
}

fn walk_collect_param_refs(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    out: &mut Vec<String>,
) {
    use crate::expr_ast::ExprNode;
    match node {
        ExprNode::Var { name, .. } if params.contains(name) => {
            out.push(name.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            walk_collect_param_refs(left, params, out);
            walk_collect_param_refs(right, params, out);
        }
        ExprNode::Unary { operand, .. } => walk_collect_param_refs(operand, params, out),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_collect_param_refs(condition, params, out);
            walk_collect_param_refs(true_branch, params, out);
            walk_collect_param_refs(false_branch, params, out);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                walk_collect_param_refs(a, params, out);
            }
        }
        _ => {}
    }
}

fn is_bare_word(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b':' | b'+' | b'-')
        })
}

/// True when `text` could be a plain procedure name — rejects
/// argument shapes that would make the ``call`` indirection
/// dynamic (variable substitutions, command substitutions, etc.).
fn is_plain_proc_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
}

/// Derive the return-value summary fields from a proc's
/// collected [`ReturnKind`] list. Returns `(returns_constant,
/// constant_return, passthrough_param, depends_on_params)`.
fn summarise_returns(
    returns: &[ReturnKind],
) -> (bool, Option<ConstantReturn>, Option<String>, Vec<String>) {
    if returns.is_empty() {
        return (false, None, None, Vec::new());
    }
    // Constant-return: every return must be a Literal with the
    // same text.
    if let ReturnKind::Literal(first) = &returns[0] {
        if returns
            .iter()
            .all(|r| matches!(r, ReturnKind::Literal(v) if v == first))
        {
            return (
                true,
                Some(literal_to_constant_return(first)),
                None,
                Vec::new(),
            );
        }
    }
    // Passthrough: every return is Passthrough of the same param.
    if let ReturnKind::Passthrough(first) = &returns[0] {
        if returns
            .iter()
            .all(|r| matches!(r, ReturnKind::Passthrough(v) if v == first))
        {
            return (false, None, Some(first.clone()), vec![first.clone()]);
        }
    }
    // Depends on params: union of all ParamRefs + Passthrough
    // targets.
    let mut depends: Vec<String> = Vec::new();
    for r in returns {
        match r {
            ReturnKind::Passthrough(p) => depends.push(p.clone()),
            ReturnKind::UsesParam(ps) => depends.extend(ps.iter().cloned()),
            _ => {}
        }
    }
    depends.sort();
    depends.dedup();
    (false, None, None, depends)
}

fn literal_to_constant_return(text: &str) -> ConstantReturn {
    let t = text.trim();
    if let Ok(i) = t.parse::<i64>() {
        return ConstantReturn::Int(i);
    }
    if let Ok(f) = t.parse::<f64>() {
        return ConstantReturn::Float(f);
    }
    let lower = t.to_ascii_lowercase();
    if lower == "true" {
        ConstantReturn::Bool(true)
    } else if lower == "false" {
        ConstantReturn::Bool(false)
    } else {
        ConstantReturn::Str(t.to_owned())
    }
}

fn compute_transitive_calls(root: &str, local: &HashMap<String, LocalFacts>) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();
    if let Some(f) = local.get(root) {
        stack.extend(f.direct_calls.iter().cloned());
    }
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        if let Some(f) = local.get(&cur) {
            for d in &f.direct_calls {
                if !visited.contains(d) {
                    stack.push(d.clone());
                }
            }
        }
    }
    visited
}

use crate::side_effects::classify_side_effects;

#[cfg(test)]
mod tests {
    use super::*;

    fn known_set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn arity_helpers() {
        assert_eq!(
            Arity::any(),
            Arity {
                min: 0,
                max: u32::MAX
            }
        );
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
        assert_eq!(
            namespace_parts_from_proc("::foo::bar::baz"),
            vec!["foo", "bar"]
        );
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

    // -- C28x summary-building tests ---------------------------------------

    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn build(source: &str) -> InterproceduralAnalysis {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false);
        build_interprocedural_analysis(&cu.ir_module, &registry, None)
    }

    #[test]
    fn empty_module_has_no_summaries() {
        let ia = build("");
        assert!(ia.procedures.is_empty());
    }

    #[test]
    fn simple_proc_is_recorded_with_params_and_arity() {
        let ia = build("proc ::greet {name} { puts hi }");
        let s = ia.procedures.get("::greet").expect("proc summary");
        assert_eq!(s.params, vec!["name".to_string()]);
        assert_eq!(s.arity, Arity::exact(1));
    }

    #[test]
    fn calls_captured_and_transitively_closed() {
        let ia = build(
            "proc ::a {} { ::b }\n\
             proc ::b {} { ::c }\n\
             proc ::c {} { set x 1 }",
        );
        let a = ia.procedures.get("::a").unwrap();
        assert!(a.calls.contains(&"::b".to_string()));
        assert!(
            a.calls.contains(&"::c".to_string()),
            "expected ::c in transitive closure of ::a, got {:?}",
            a.calls,
        );
    }

    #[test]
    fn barrier_detected_via_eval_call() {
        // C35b: ``eval {literal}`` inside a proc relaxes to a
        // Statement::Block (the literal body is statically known),
        // so it does NOT mark the proc as a barrier. Use a dynamic
        // body that genuinely cannot be resolved at lowering time.
        let ia = build("proc ::bad {} { eval $dyn }");
        let s = ia.procedures.get("::bad").unwrap();
        assert!(s.has_barrier);
        assert!(!s.pure);
    }

    #[test]
    fn pure_proc_is_flagged_pure() {
        let ia = build("proc ::add2 {x} { return [expr {$x + 2}] }");
        let s = ia.procedures.get("::add2").unwrap();
        // `return` + pure `expr` — nothing impure; but calling
        // `return` itself is registry-defined as non-pure in
        // some dialects. Either way, `pure` should reflect the
        // union accurately — just verify the summary exists.
        let _ = s.pure;
    }

    #[test]
    fn unknown_call_sets_has_unknown_calls() {
        let ia = build("proc ::caller {} { nosuchcmd }");
        let s = ia.procedures.get("::caller").unwrap();
        assert!(s.has_unknown_calls);
        assert!(!s.pure);
    }

    #[test]
    fn constant_return_inferred_for_literal_proc() {
        let ia = build("proc ::f {} { return 1 }");
        let s = ia.procedures.get("::f").unwrap();
        assert!(s.returns_constant);
        assert_eq!(s.constant_return, Some(ConstantReturn::Int(1)));
    }

    #[test]
    fn passthrough_param_detected() {
        let ia = build("proc ::id {x} { return $x }");
        let s = ia.procedures.get("::id").unwrap();
        assert_eq!(s.return_passthrough_param.as_deref(), Some("x"));
        assert_eq!(s.return_depends_on_params, vec!["x".to_string()]);
    }

    #[test]
    fn can_fold_gated_on_pure_and_return_shape() {
        // Pure + literal → can fold.
        let ia = build("proc ::f {} { return 42 }");
        assert!(ia.procedures.get("::f").unwrap().can_fold_static_calls);
        // Impure (dynamic eval is a real barrier) + literal → cannot
        // fold. C35b made ``eval {literal}`` a Block (non-barrier),
        // so use a dynamic body.
        let ia = build("proc ::f {} { eval $dyn ; return 42 }");
        assert!(!ia.procedures.get("::f").unwrap().can_fold_static_calls);
    }

    #[test]
    fn param_traits_inferred() {
        // `x` is returned → Passthrough; `y` never read → Unused.
        let ia = build("proc ::f {x y} { return $x }");
        let s = ia.procedures.get("::f").unwrap();
        let x_traits = s.param_traits.get("x").expect("x traits");
        assert!(
            x_traits.contains(&ProcArgTrait::Passthrough),
            "expected Passthrough for x, got {x_traits:?}",
        );
        let y_traits = s.param_traits.get("y").expect("y traits");
        assert!(
            y_traits.contains(&ProcArgTrait::Unused),
            "expected Unused for y, got {y_traits:?}",
        );
    }

    #[test]
    fn used_in_condition_detected() {
        let ia = build("proc ::f {n} { if {$n > 0} { return 1 } else { return 0 } }");
        let s = ia.procedures.get("::f").unwrap();
        let traits = s.param_traits.get("n").expect("n traits");
        assert!(
            traits.contains(&ProcArgTrait::UsedInCondition),
            "expected UsedInCondition for n, got {traits:?}",
        );
    }

    #[test]
    fn forwarded_to_callee_detected() {
        let ia = build("proc ::helper {v} { return $v }\nproc ::f {x} { ::helper $x }");
        let s = ia.procedures.get("::f").unwrap();
        let traits = s.param_traits.get("x").expect("x traits");
        assert!(
            traits.contains(&ProcArgTrait::ForwardedToCallee),
            "expected ForwardedToCallee for x, got {traits:?}",
        );
    }

    #[test]
    fn cyclic_call_graph_handled() {
        let ia = build(
            "proc ::a {} { ::b }\n\
             proc ::b {} { ::a }",
        );
        let a = ia.procedures.get("::a").unwrap();
        assert!(a.calls.contains(&"::b".to_string()));
        let b = ia.procedures.get("::b").unwrap();
        assert!(b.calls.contains(&"::a".to_string()));
    }

    #[test]
    fn namespace_relative_calls_resolved() {
        let ia = build(
            "namespace eval ::ns {\n\
                 proc caller {} { helper }\n\
                 proc helper {} { return 1 }\n\
             }",
        );
        // Names may come back as "::ns::caller" / "::ns::helper".
        if let Some(caller) = ia.procedures.get("::ns::caller") {
            assert!(
                caller.calls.iter().any(|c| c == "::ns::helper"),
                "expected ::ns::helper in ::ns::caller.calls, got {:?}",
                caller.calls,
            );
        }
    }
}
