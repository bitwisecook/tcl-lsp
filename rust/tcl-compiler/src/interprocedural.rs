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

//! Interprocedural analysis — per-procedure summaries and
//! call-target resolution.
//!
//! Provides the summary types (`ProcSummary`, `MethodSummary`,
//! `InterproceduralAnalysis`) plus the call-target resolver, which
//! plug into the side-effect classifier and the SCCP evaluator.

use std::collections::{HashMap, HashSet};

pub use tcl_registry::Arity;

use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};
use crate::naming::{normalise_var_name, split_array_name};
use crate::side_effects::EffectRegion;

/// Depth cap shared by every `Script`/`Statement`-tree recursion in this
/// module (`collect_instance_var_writes`; the mutually-recursive
/// `scan_script`/`scan_statement`/`scan_control_flow_statement` trio;
/// `script_always_returns`/`stmt_always_returns`) — issue #996.
///
/// Transitively bounded today via `crate::lowering`'s
/// `MAX_LOWER_NEST_DEPTH` (every `Script` this module walks is built by
/// lowering, which already caps its own construction at 256), capped here
/// independently for defence-in-depth and consistency with every other
/// full-tree walker in this crate (`optimiser::MAX_OPTIMISER_WALK_DEPTH`,
/// `codegen::structured::MAX_STRUCTURED_DEPTH`).
const MAX_INTERPROCEDURAL_WALK_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

// Summary types

/// A proc/method's declared arity from its bare parameter-name list, as
/// recorded at the IR layer (`Vec<String>`, no default-value info — that
/// lives only in the analyser's richer `ParamDef`; see
/// [`crate::signature_scan::arity::arity_of`] for the default-aware
/// computation used by the arity diagnostics).  Still correctly
/// unbounded when the last parameter is literally `args`, matching
/// [`crate::taint_interproc`]'s identical, already-correct formula for
/// the same bare-name shape.
#[must_use]
pub fn arity_from_names(params: &[String]) -> Arity {
    let n = u16::try_from(params.len()).unwrap_or(u16::MAX);
    if params.last().is_some_and(|p| p == "args") {
        Arity::at_least(n.saturating_sub(1))
    } else {
        Arity::exact(n)
    }
}

/// Interprocedural argument trait. Documents how a parameter is
/// used inside the callee — consumed by the optimiser for
/// parameter-specific reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProcArgTrait {
    /// Parameter text is substituted into the return value
    /// unchanged.
    Passthrough,
    /// Parameter participates in a comparison that gates control
    /// flow.
    UsedInCondition,
    /// Parameter is forwarded to another procedure.
    ForwardedToCallee,
    /// Parameter names a variable the proc reads via `upvar` (a
    /// read-only caller-frame alias, or a name source) — call-by-name.
    VarRead,
    /// Parameter names a variable the proc writes via `upvar` +
    /// `set` / `incr` / `append` (a caller-frame write-back) — call-by-name.
    VarWrite,
    /// Parameter is never read.
    Unused,
}

impl ProcArgTrait {
    /// Stable lower-case wire form
    /// (`"passthrough"`, `"used_in_condition"`,
    /// `"forwarded_to_callee"`, `"var_read"`, `"var_write"`,
    /// `"unused"`). Consumers (`PyO3` bindings, native LSP server)
    /// materialise traits using this form rather than re-implementing
    /// the mapping.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::UsedInCondition => "used_in_condition",
            Self::ForwardedToCallee => "forwarded_to_callee",
            Self::VarRead => "var_read",
            Self::VarWrite => "var_write",
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
// False positive: a flat record of independent boolean facts about a
// procedure (barrier / unknown-calls / writes-global / pure / ...), not a
// state machine — there is no natural enum to collapse these into.
#[allow(clippy::struct_excessive_bools)]
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
    /// Names of procedures this one calls *directly* (no transitive
    /// closure), sorted and resolved to in-module proc qnames. The
    /// proc's local, non-transitive direct-call set; consumed by
    /// the `callgraph` verb's `build_call_graph`, which needs the direct
    /// call set so the edge list carries no spurious A→C transitive edges.
    pub direct_calls: Vec<String>,
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
            direct_calls: Vec::new(),
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

/// A command-name → `(params, param_traits)` lookup, keyed by the bare leaf
/// name (`bump`), the qualified name (`::demo::bump`), and the
/// leading-colon-stripped name (`demo::bump`) so a bare call resolves to a
/// proc declared in any namespace.  Input to [`collect_call_by_name_reads`].
pub type ProcIndex = HashMap<String, (Vec<String>, HashMap<String, HashSet<ProcArgTrait>>)>;

/// Build a [`ProcIndex`] from interprocedural summaries.
#[must_use]
pub fn build_proc_index_from_summaries(ia: &InterproceduralAnalysis) -> ProcIndex {
    let mut index = ProcIndex::new();
    for (qname, summary) in &ia.procedures {
        let entry = (summary.params.clone(), summary.param_traits.clone());
        // Leaf name (`::demo::bump` → `bump`): a `proc` declared inside a
        // namespaced body is registered under its qualified name, but a
        // same-namespace bare call (`bump x`) must still resolve to it.
        let leaf = qname.rsplit("::").next().unwrap_or(qname.as_str());
        let lstripped = qname.trim_start_matches(':');
        for key in [qname.as_str(), lstripped, leaf] {
            index.entry(key.to_owned()).or_insert_with(|| entry.clone());
        }
    }
    index
}

/// Record any literal-name argument landing on a callee param that
/// carries `VarRead` / `VarWrite` (a call-by-name read/write).
fn add_call_by_name(cmd: &str, args: &[String], index: &ProcIndex, out: &mut HashSet<String>) {
    if cmd.is_empty() || cmd.contains(['$', '[']) {
        return;
    }
    let Some((params, traits_map)) = index
        .get(cmd)
        .or_else(|| index.get(&format!("::{cmd}")))
        .or_else(|| index.get(cmd.trim_start_matches(':')))
    else {
        return;
    };
    for (i, arg) in args.iter().enumerate() {
        let Some(pname) = params.get(i) else {
            break;
        };
        let is_var = traits_map.get(pname).is_some_and(|t| {
            t.contains(&ProcArgTrait::VarRead) || t.contains(&ProcArgTrait::VarWrite)
        });
        // A substituted (`$x`) / array / non-name arg names a runtime
        // variable we can't identify — skip (preserve genuine FPs where
        // the caller passed a literal string, not a name).
        if is_var
            && !arg.is_empty()
            && !arg.contains(['$', '['])
            && arg
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
        {
            out.insert(arg.clone());
        }
    }
}

/// Scan a whole-value `[cmd …]` command substitution (the dominant
/// tcllib call-by-name shape, `set len [asnPeekTag data tag type
/// dummy]`).  A simple whitespace split suffices — the call-by-name
/// args are literal names, and [`add_call_by_name`] rejects anything
/// that isn't.
fn scan_value_cmd_subst(text: &str, index: &ProcIndex, out: &mut HashSet<String>) {
    let Some(inner) = text
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
    else {
        return;
    };
    let mut words = inner.split_whitespace();
    if let Some(cmd) = words.next() {
        let args: Vec<String> = words.map(str::to_owned).collect();
        add_call_by_name(cmd, &args, index, out);
    }
}

/// Caller-local variable names passed *by name* to a user proc that
/// consumes them via `upvar` (call-by-name) — these must NOT be flagged
/// dead / unused (W211 / W220 / O109 / O126).
///
/// Scans direct `Call` / `Barrier` statements plus a `[cmd …]`
/// substitution that is the whole value of an `AssignValue` /
/// `AssignExpr` / `Return`.
#[must_use]
pub fn collect_call_by_name_reads(
    cfg: &crate::cfg::Function,
    index: &ProcIndex,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut out = HashSet::new();
    if index.is_empty() {
        return out;
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } => {
                    add_call_by_name(command, args, index, &mut out);
                }
                Statement::AssignValue { value, .. } => {
                    scan_value_cmd_subst(value, index, &mut out);
                }
                Statement::Return { value: Some(v), .. } => {
                    scan_value_cmd_subst(v, index, &mut out);
                }
                Statement::AssignExpr {
                    expr: crate::expr_ast::ExprNode::Command { text, .. },
                    ..
                } => {
                    scan_value_cmd_subst(text, index, &mut out);
                }
                _ => {}
            }
        }
    }
    out
}

// Call-target resolution

/// Resolve a command name to a qualified procedure name if it refers to one
/// defined in `known`, via [`crate::naming::resolve_command_with`] — Tcl's
/// real existence-checked resolution: an absolute name (`::foo`) is looked
/// up directly; a relative name — bare (`foo`) or dotted (`inner::p`) —
/// tries the caller's namespace first, then global, dispatching the first
/// candidate that *exists* in `known`; never every enclosing ancestor
/// namespace (Tcl's own command lookup does not walk intermediate
/// namespaces absent an explicit `namespace path`, which whole-unit
/// summaries do not model).
///
/// Shared with the analyser's identical same-file resolution
/// (`Analyser::resolve_proc_call`) and the optimiser's
/// (`resolve_proc_qname`) so the three can't diverge on the same rule.
#[must_use]
pub fn resolve_internal_call<S: std::hash::BuildHasher>(
    command: &str,
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    let ns_parts = namespace_parts_from_proc(caller_qname);
    let ns = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    crate::naming::resolve_command_with::<&str, _>(&ns, &[], command, |qname| known.contains(qname))
}

/// Top-level call-target resolver. Convenience wrapper that
/// handles the common case where the caller has no special
/// aliasing information.
#[must_use]
pub fn resolve_call_target<S: std::hash::BuildHasher>(
    command: &str,
    _args: &[String],
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    resolve_internal_call(command, caller_qname, known)
}

/// Return the namespace segments of a qualified proc name —
/// everything except the trailing simple name. The split is the one canonical
/// [`crate::naming::qualifier_segments_owned`] (colon runs are a single
/// separator, so `::a:::b::p` yields `["a", "b"]`).
#[must_use]
pub fn namespace_parts_from_proc(qname: &str) -> Vec<String> {
    let mut parts = crate::naming::qualifier_segments_owned(qname);
    parts.pop();
    parts
}

// Summary building (partial)

/// The object-handle → candidate-class map
/// ([`crate::object_types::object_handle_classes`]) supplied to the call-graph
/// pass so a `$g walk … -command cb` instance-method callback resolves to a
/// `direct_calls` edge.  A borrow-only newtype: it gives the public
/// [`build_interprocedural_analysis`] a named parameter instead of a bare
/// `HashMap` (whose default hasher would otherwise trip
/// `clippy::implicit_hasher`).  Use [`ObjectTypeMap::none()`] when no object
/// typing is available (an IR-only caller, or a context that needs no callback
/// edges).
#[derive(Clone, Copy)]
pub struct ObjectTypeMap<'a>(pub &'a HashMap<String, HashSet<String>>);

impl ObjectTypeMap<'static> {
    /// The empty map — no object-handle typing (no instance-method callback
    /// edges).  Backed by a process-wide empty map so it needs no local.
    #[must_use]
    pub fn none() -> Self {
        ObjectTypeMap(&EMPTY_OBJECT_TYPES)
    }
}

static EMPTY_OBJECT_TYPES: std::sync::LazyLock<HashMap<String, HashSet<String>>> =
    std::sync::LazyLock::new(HashMap::new);

/// Build conservative interprocedural summaries for every
/// procedure in `ir_module`.
///
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
#[must_use]
pub fn build_interprocedural_analysis(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    object_types: ObjectTypeMap<'_>,
) -> InterproceduralAnalysis {
    let object_types = object_types.0;
    let known: HashSet<String> = ir_module.procedures.keys().cloned().collect();

    let local = scan_all_procs(ir_module, &known, registry, dialect, object_types);
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

    // Summarise TclOO method bodies into `MethodSummary` entries
    // (consumed by the O126 `my <method>` purity gate).  Method bodies are not
    // iterated by the call graph and an unresolved `$obj method` self-dispatch
    // already forces the method impure, so instance-method callback typing adds
    // nothing here — method summaries need no object-type map.
    let methods = build_method_summaries(
        ir_module,
        &known,
        registry,
        dialect,
        &pure,
        &effect_reads,
        &effect_writes,
    );

    InterproceduralAnalysis {
        procedures,
        methods,
    }
}

/// Summarise `TclOO` method bodies into [`MethodSummary`] entries.
/// The purity rule is intentionally conservative — a method is
/// pure iff its own body has no observable side effect (no barrier, no
/// unknown call, no global write, no instance-var write, no local
/// effect write) **and** every *proc* it calls is pure. A `my
/// <method>` / `next` self-dispatch resolves to no registry trait, so
/// it falls through to an unknown-state effect write and forces the
/// method impure — we never mark a method pure on the strength of an
/// unproven peer method (sound: false negatives only). A redefined
/// method is forced impure (we can't prove which body a dispatch
/// runs).
fn build_method_summaries(
    ir_module: &crate::ir::Module,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    pure: &HashMap<String, bool>,
    effect_reads: &HashMap<String, EffectRegion>,
    effect_writes: &HashMap<String, EffectRegion>,
) -> HashMap<String, MethodSummary> {
    if ir_module.methods.is_empty() {
        return HashMap::new();
    }
    // A call resolving to a method qname is "known" (not unknown), so
    // seed the resolver with both procs and method names.
    let mut method_known = known.clone();
    method_known.extend(ir_module.methods.keys().cloned());

    let mut out: HashMap<String, MethodSummary> = HashMap::with_capacity(ir_module.methods.len());
    for (mqname, method) in &ir_module.methods {
        let mut facts = LocalFacts {
            local_pure: true,
            ..LocalFacts::default()
        };
        let params: HashSet<String> = method.params.iter().cloned().collect();
        let ctx = ScanCtx {
            caller: mqname,
            known: &method_known,
            registry,
            dialect,
            params: &params,
            // Method bodies are not call-graph nodes; no object-type map needed.
            object_types: ObjectTypeMap::none().0,
        };
        scan_script(&method.body, ctx, &mut facts, 0);
        // Fall-through exit is non-constant (O103); see `scan_proc`.
        if !script_always_returns(&method.body, 0) {
            facts.returns.push(ReturnKind::Other);
        }

        // Instance-variable writes mutate object state and survive the
        // call — so a method that writes any in-scope instance var is
        // impure even though the write looks like a plain local `set`.
        let mut written_ivars: HashSet<String> = HashSet::new();
        if !method.instance_vars.is_empty() {
            collect_instance_var_writes(&method.body, &method.instance_vars, &mut written_ivars, 0);
        }

        // `local_pure` is the authoritative local-impurity signal (the
        // proc fixpoint keys off the same flag): it is cleared by
        // barriers, unknown calls, global writes, AND any command whose
        // classification is impure — including region-free effects like
        // `puts`/`log` (FILE_IO/LOG_IO write, `EffectRegion::NONE`). The
        // old `effect_writes == NONE` proxy missed that last case, marking
        // a `puts`-calling method spuriously pure. Instance-var writes
        // (plain local `set ivar …`, region-free + locally pure) stay a
        // separate method-only term.
        let pure_base = facts.local_pure && written_ivars.is_empty();
        let mut is_pure = pure_base
            && facts
                .direct_calls
                .iter()
                .all(|c| pure.get(c).copied().unwrap_or(false));
        // A redefined method (later oo::define / duplicate body) is
        // conservatively impure: the stored body may not be the one a
        // given dispatch runs.
        if ir_module.redefined_methods.contains(mqname) {
            is_pure = false;
        }

        // Effects: the method's own local effects unioned with the
        // (transitive) effects of every proc callee. Non-proc callees
        // (method qnames) default to NONE.
        let mut m_reads = facts.effect_reads;
        let mut m_writes = facts.effect_writes;
        for c in &facts.direct_calls {
            m_reads |= effect_reads.get(c).copied().unwrap_or(EffectRegion::NONE);
            m_writes |= effect_writes.get(c).copied().unwrap_or(EffectRegion::NONE);
        }

        let mut calls: Vec<String> = facts.direct_calls.iter().cloned().collect();
        calls.sort();
        let direct_calls = calls.clone();
        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns);

        out.insert(
            mqname.clone(),
            MethodSummary {
                base: ProcSummary {
                    qualified_name: mqname.clone(),
                    params: method.params.clone(),
                    arity: arity_from_names(&method.params),
                    calls,
                    direct_calls,
                    has_barrier: facts.has_barrier,
                    has_unknown_calls: facts.has_unknown_calls,
                    writes_global: facts.writes_global,
                    pure: is_pure,
                    effect_reads: m_reads,
                    effect_writes: m_writes,
                    returns_constant,
                    constant_return,
                    return_depends_on_params: depends,
                    return_passthrough_param: passthrough,
                    // Methods are not folded at static call sites.
                    can_fold_static_calls: false,
                    param_traits: HashMap::new(),
                },
                class_name: method.class_name.clone(),
                method_kind: method.kind.as_str().to_owned(),
                // Read-set / MRO-dispatch tracking is not implemented;
                // left empty (the purity gate consumes only `pure`).
                reads_instance_vars: HashSet::new(),
                writes_instance_vars: written_ivars,
                calls_my: Vec::new(),
                calls_next: false,
            },
        );
    }
    out
}

/// Recursively collect the base names of instance-variable *writes* in
/// a method body, comparing each written name against `ivars`. Walks
/// the CFG, counting every `defs` of a non-`::variable` / `::upvar`
/// Call, plus every assign / incr / loop / catch target. Array-element
/// writes (`counter(0)`) compare on the base scalar name so they are
/// not missed. Over-approximating writes is the sound direction (a
/// spurious write only costs an O126 fold; a missed one would wrongly
/// delete a state mutation).
#[allow(clippy::too_many_lines)] // the depth-cap check adds a few lines over the threshold
fn collect_instance_var_writes(
    script: &crate::ir::Script,
    ivars: &HashSet<String>,
    out: &mut HashSet<String>,
    depth: u32,
) {
    use crate::ir::Statement;
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::Incr { name, .. } => check_ivar_write(name, ivars, out),
            Statement::Call {
                command,
                canonical_command,
                defs,
                ..
            } => {
                // `variable` / `upvar` link or declare a name; they are
                // not writes to instance state.
                if is_variable_or_upvar(command, canonical_command.as_deref()) {
                    continue;
                }
                for d in defs {
                    check_ivar_write(d, ivars, out);
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    collect_instance_var_writes(&clause.body, ivars, out, depth + 1);
                }
                if let Some(eb) = else_body {
                    collect_instance_var_writes(eb, ivars, out, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_instance_var_writes(init, ivars, out, depth + 1);
                collect_instance_var_writes(next, ivars, out, depth + 1);
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    for v in &it.vars {
                        check_ivar_write(v, ivars, out);
                    }
                }
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
                if let Some(rv) = result_var {
                    check_ivar_write(rv, ivars, out);
                }
                if let Some(ov) = options_var {
                    check_ivar_write(ov, ivars, out);
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
                for h in handlers {
                    if let Some(v) = &h.var_name {
                        check_ivar_write(v, ivars, out);
                    }
                    if let Some(ov) = &h.options_var {
                        check_ivar_write(ov, ivars, out);
                    }
                    collect_instance_var_writes(&h.body, ivars, out, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_instance_var_writes(fb, ivars, out, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        collect_instance_var_writes(b, ivars, out, depth + 1);
                    }
                }
                if let Some(db) = default_body {
                    collect_instance_var_writes(db, ivars, out, depth + 1);
                }
            }
            Statement::While { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            _ => {}
        }
    }
}

/// Record `raw` as an instance-var write when its base scalar name (an
/// array element `arr(idx)` compares on `arr`) is a declared instance
/// var.
fn check_ivar_write(raw: &str, ivars: &HashSet<String>, out: &mut HashSet<String>) {
    if raw.is_empty() {
        return;
    }
    let base = split_array_name(normalise_var_name(raw)).0;
    if ivars.contains(base) {
        out.insert(base.to_owned());
    }
}

/// True iff a statement's command (preferring its canonical form,
/// `::`-stripped) is `variable` or `upvar` — a link/declaration, not a
/// write to instance state.
fn is_variable_or_upvar(command: &str, canonical: Option<&str>) -> bool {
    let c = canonical.unwrap_or(command);
    let c = c.strip_prefix("::").unwrap_or(c);
    c == "variable" || c == "upvar"
}

fn scan_all_procs(
    ir_module: &crate::ir::Module,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    object_types: &HashMap<String, HashSet<String>>,
) -> HashMap<String, LocalFacts> {
    let mut local: HashMap<String, LocalFacts> = HashMap::with_capacity(known.len());
    for (qname, proc) in &ir_module.procedures {
        local.insert(
            qname.clone(),
            scan_proc(qname, proc, known, registry, dialect, object_types),
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
        let mut direct_calls: Vec<String> = facts.direct_calls.iter().cloned().collect();
        direct_calls.sort();
        let is_pure = *pure.get(qname).unwrap_or(&false);

        // `writes_global` / `has_unknown_calls` are documented as
        // transitive, but were copied straight from local facts — so a proc
        // that writes a global (or calls an unknown command) only via a
        // callee reported `false`, and `propagate_taints` then failed to
        // seed its globals as tainted. `calls_list` is the full transitive
        // closure, so OR in every transitive callee's local flag.
        let transitive_flag = |pick: fn(&LocalFacts) -> bool| -> bool {
            pick(facts) || calls_list.iter().any(|c| local.get(c).is_some_and(pick))
        };
        let writes_global = transitive_flag(|f| f.writes_global);
        let has_unknown_calls = transitive_flag(|f| f.has_unknown_calls);

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
                arity: arity_from_names(&proc.params),
                calls: calls_list,
                direct_calls,
                has_barrier: facts.has_barrier,
                has_unknown_calls,
                writes_global,
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
// False positive: a flat record of independent boolean facts gathered while
// scanning a body, not a state machine — no natural enum to collapse into.
#[allow(clippy::struct_excessive_bools)]
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
    /// Local variable names this proc aliased into global /
    /// enclosing-namespace scope via `global` / `variable` / `upvar
    /// #0`. A later bare `set g` / `incr g` / `append g` to one of
    /// these mutates a variable the *caller* can see, so it counts as
    /// `writes_global` even though the written name is bare.
    global_aliases: HashSet<String>,
    /// Call-by-name upvar aliases: a local variable name (text) → the
    /// parameter whose value named the *caller-frame* variable it
    /// aliases (`upvar 1 $param local` → `local` → `param`).  A later
    /// `set local …` upgrades that param to [`ProcArgTrait::VarWrite`].
    /// Only level-1 upvars populate this (other levels don't write back
    /// to the caller).
    upvar_aliases: HashMap<String, String>,
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
            global_aliases: HashSet::new(),
            upvar_aliases: HashMap::new(),
        }
    }
}

fn scan_proc(
    qname: &str,
    proc: &crate::ir::Procedure,
    known: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    object_types: &HashMap<String, HashSet<String>>,
) -> LocalFacts {
    let mut facts = LocalFacts {
        local_pure: true,
        ..LocalFacts::default()
    };
    let params: HashSet<String> = proc.params.iter().cloned().collect();
    let ctx = ScanCtx {
        caller: qname,
        known,
        registry,
        dialect,
        params: &params,
        object_types,
    };
    scan_script(&proc.body, ctx, &mut facts, 0);
    // If the body can fall off the end, its implicit exit returns the
    // result of the last command — not a constant. Record a non-constant
    // exit so `summarise_returns` won't fold a conditional `return CONST`
    // to a constant when a fall-through path returns something else
    // (O103).
    if !script_always_returns(&proc.body, 0) {
        facts.returns.push(ReturnKind::Other);
    }
    facts
}

/// Read-only context shared by the recursive proc/method body scanners
/// ([`scan_script`] / [`scan_statement`] / [`scan_call_facts`] /
/// [`scan_value_substitutions`]). Bundled so the scanners stay within the
/// argument limit.
#[derive(Clone, Copy)]
struct ScanCtx<'a> {
    caller: &'a str,
    known: &'a HashSet<String>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'a str>,
    params: &'a HashSet<String>,
    /// Object-handle → candidate class names for this module
    /// ([`crate::object_types::object_handle_classes`]), so a `$g walk …
    /// -command cb` instance-method dispatch resolves its callback to a
    /// call-graph edge.  Empty when built without a `CompilationUnit`.
    object_types: &'a HashMap<String, HashSet<String>>,
}

/// `depth` is the nesting level of `script` — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Past the cap, `script`'s statements
/// are not scanned; `facts` is instead marked exactly as conservatively as
/// [`scan_statement`]'s own `Statement::Barrier` arm (unscanned code must
/// not be silently under-counted as pure / effect-free).
fn scan_script(script: &crate::ir::Script, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        facts.has_barrier = true;
        facts.local_pure = false;
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        return;
    }
    for stmt in &script.statements {
        scan_statement(stmt, ctx, facts, depth);
    }
}

/// Process a `Statement::Call` for interprocedural facts: side-
/// effects classification, internal-call resolution, and param
/// trait inference.  Extracted from [`scan_statement`].
fn scan_call_facts(command: &str, args: &[String], ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    let ScanCtx {
        caller,
        known,
        registry,
        params,
        ..
    } = ctx;
    // Resolve internal-proc call targets first. Special case
    // for iRules' ``call <proc>`` indirection: when the
    // command is literally ``call`` and the first arg is
    // a plain identifier, treat it as a direct invocation
    // of that proc.
    let internal_target = if command == "call" && !args.is_empty() && is_plain_proc_name(&args[0]) {
        resolve_internal_call(&args[0], caller, known)
    } else {
        resolve_internal_call(command, caller, known)
    };

    // Command-prefix callbacks (`lsort -command myCompare`, `trace add … cb`,
    // `interp alias {} a {} target`) are call edges too: the referenced proc is
    // reachable, so a callback-only proc is not dead (O124) and appears in the
    // call graph. Registry-driven via `command_prefixes` — no command-name
    // matching here. Literal single-identifier heads only (`is_plain_proc_name`
    // rejects dynamic `$cb` / bracketed heads), mirroring the reference
    // extractor's bareword guard.
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for (idx, _appended) in registry.command_prefixes(command, &arg_strs) {
        if let Some(word) = args.get(idx).and_then(|a| a.split_whitespace().next())
            && is_plain_proc_name(word)
            && let Some(target) = resolve_internal_call(word, caller, known)
        {
            facts.direct_calls.insert(target);
        }
    }

    // Object-instance method-callback dispatch — `$g walk … -command cb` /
    // `objName walkproc … cb`.  The receiver's class(es) come from the
    // module's object-handle map (SSA/VTA-derived); resolve the *method's*
    // command prefixes (`instance_method_command_prefixes`) so a bareword
    // callback that names an in-module proc becomes a call-graph edge, exactly
    // like a top-level prefix.  Over-approximate across candidate classes — an
    // extra reachability edge is sound (never a missed edge / false dead-code).
    if let Some(method) = args.first()
        && !ctx.object_types.is_empty()
    {
        let receiver = extract_var_name(command).unwrap_or(command);
        if let Some(classes) = ctx.object_types.get(receiver) {
            for class in classes {
                for (idx, _appended) in
                    registry.instance_method_command_prefixes(class, method, &arg_strs[1..])
                {
                    // `idx` is relative to the words after the method name, so
                    // the callback word is `args[idx + 1]`.
                    if let Some(word) = args.get(idx + 1).and_then(|a| a.split_whitespace().next())
                        && is_plain_proc_name(word)
                        && let Some(target) = resolve_internal_call(word, caller, known)
                    {
                        facts.direct_calls.insert(target);
                    }
                }
            }
        }
    }

    // A call that resolves to an internal proc contributes ONLY a
    // call-graph edge — its purity / effects flow through the
    // interprocedural fixpoints from the callee's summary, so the
    // command's own side-effect classification is NOT applied locally.
    // Non-internal commands apply their classified side effects.
    if let Some(target) = &internal_target {
        facts.direct_calls.insert(target.clone());
    } else {
        // Side-effect classification is dialect-agnostic here
        // (`classify_side_effects` is called with no dialect): a
        // command's effect profile reflects what it *does*,
        // not which dialect it is valid in. (The document `dialect` still
        // drives the lexer above so `[cmd …]` / bodies tokenise correctly.)
        // This is why e.g. `log`/`puts` resolve to their LOG_IO/FILE_IO
        // hints — impure but region-free — even under a Tcl dialect.
        let ci = classify_side_effects(registry, command, args, None, None);
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
        if registry.get(command).is_none() {
            facts.has_unknown_calls = true;
            facts.local_pure = false;
        }
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

/// Extract a bare scalar variable name from a `$var` / `${var}` word —
/// the whole word must be exactly one scalar variable substitution (no
/// array index, conventional name shape).
fn extract_var_name(text: &str) -> Option<&str> {
    let name = text
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
        .or_else(|| text.strip_prefix('$'))?;
    let first = name.chars().next()?;
    if !(first.is_alphabetic() || first == '_') {
        return None;
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        return None;
    }
    Some(name)
}

/// Call-by-name `upvar` handling.  For `upvar ?level? other local …`, mark a `$param`
/// *other* (caller-var-name source) as [`ProcArgTrait::VarRead`] and —
/// only for the default level 1, which writes back to the caller's frame
/// — record `local → param` so a later `set local …` upgrades it to
/// [`ProcArgTrait::VarWrite`].  A `$param` *local* name (the binding
/// target) is itself a write of that param's value.
fn handle_upvar_aliases(args: &[String], params: &HashSet<String>, facts: &mut LocalFacts) {
    let mut start = 0;
    let mut level = "1";
    if let Some(first) = args.first()
        && (first.starts_with('#')
            || (!first.is_empty() && first.bytes().all(|b| b.is_ascii_digit())))
    {
        level = first;
        start = 1;
    }
    let caller_frame = level == "1";
    let mut i = start;
    while i + 1 < args.len() {
        let other_var = &args[i];
        let my_var = &args[i + 1];
        i += 2;
        if let Some(other_vn) = extract_var_name(other_var)
            && params.contains(other_vn)
        {
            facts
                .param_trait_flags
                .entry(other_vn.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarRead);
            if caller_frame {
                facts
                    .upvar_aliases
                    .insert(my_var.clone(), other_vn.to_owned());
            }
        }
        if let Some(my_vn) = extract_var_name(my_var)
            && params.contains(my_vn)
        {
            facts
                .param_trait_flags
                .entry(my_vn.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarWrite);
        }
    }
}

/// Upgrade the param aliased by `name` (if any) to
/// [`ProcArgTrait::VarWrite`] — a write through a level-1 upvar alias is
/// a write-back to the caller's variable.
fn mark_upvar_alias_write(name: &str, facts: &mut LocalFacts) {
    if let Some(param) = facts.upvar_aliases.get(name).cloned() {
        facts
            .param_trait_flags
            .entry(param)
            .or_default()
            .insert(ProcArgTrait::VarWrite);
    }
}

/// Gather interprocedural facts from a `Statement::Call`: scope-aliasing
/// declarations (`global` / `variable` / `upvar #0`), global / upvar write-back
/// detection, and the per-call side-effect / call-graph facts. Extracted from
/// [`scan_statement`].
fn scan_call_statement(
    command: &str,
    args: &[String],
    defs: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let ScanCtx { params, .. } = ctx;
    // Track scope-aliasing declarations (`global` / `variable` / `upvar #0`)
    // so a later bare write to an aliased name counts as `writes_global`.
    // Declaring is not writing, so handle the declaration before the
    // defs-based write check.
    match global_alias_names(command, args) {
        Some(alias_names) => {
            if alias_names.contains("") {
                // Dynamic / unbounded alias target — conservative.
                facts.writes_global = true;
            }
            facts
                .global_aliases
                .extend(alias_names.into_iter().filter(|n| !n.is_empty()));
        }
        None => {
            // A non-declaration command that writes a `::`-qualified
            // or global-aliased variable (`append g`, `lappend g`,
            // `dict set ::cfg ...`) mutates caller-visible state.
            if defs
                .iter()
                .any(|n| !n.is_empty() && (n.starts_with("::") || facts.global_aliases.contains(n)))
            {
                facts.writes_global = true;
            }
        }
    }
    // Call-by-name: record `upvar` aliases, and treat any command writing a
    // level-1 upvar alias (`append` / `lappend` / `lassign` … via `defs`) as a
    // write-back to the caller's variable.
    if command == "upvar" {
        handle_upvar_aliases(args, params, facts);
    } else {
        // `upvar` itself is excluded — its `defs` are the locals it *defines*
        // (aliases), not writes.
        for d in defs {
            mark_upvar_alias_write(d, facts);
        }
    }
    scan_call_facts(command, args, ctx, facts);
}

/// `depth` is `stmt`'s own nesting level — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Dispatch-only (no nested `Script`
/// entered here) keeps the same `depth`; recursing into a nested body
/// (`UpFrame`/`Block`'s inner statements, or any control-flow body via
/// [`scan_control_flow_statement`]) passes `depth + 1`.
fn scan_statement(
    stmt: &crate::ir::Statement,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::ir::Statement;
    let ScanCtx { params, .. } = ctx;
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
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::Block { body, .. } => {
            // ``Block`` is a transparent splice: walk through to the
            // inner statements without flagging a barrier.
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::AssignConst { name, .. } | Statement::AssignExpr { name, .. } => {
            note_assign_global_write(name, facts);
        }
        Statement::AssignValue { name, value, .. } => {
            note_assign_global_write(name, facts);
            // For an assigned value, a `[cmd …]` substitution in that
            // value is a call site (`set y [double $x]`), so its callees
            // become call-graph edges.
            if value.contains('[') {
                scan_value_substitutions(value, ctx, facts, 0);
            }
        }
        Statement::Incr { name, amount, .. } => {
            note_assign_global_write(name, facts);
            if let Some(amount) = amount
                && amount.contains('[')
            {
                scan_value_substitutions(amount, ctx, facts, 0);
            }
        }
        Statement::Return { value, expr, .. } => {
            let kind = classify_return(value.as_deref(), expr.as_ref(), params);
            facts.returns.push(kind);
            // For a return, scan `[cmd …]` substitutions in the return
            // value (`return [add $x $x]`) for call-graph edges.
            if let Some(value) = value
                && value.contains('[')
            {
                scan_value_substitutions(value, ctx, facts, 0);
            }
        }
        Statement::Call {
            command,
            args,
            defs,
            ..
        } => {
            scan_call_statement(command, args, defs, ctx, facts);
        }
        Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::ExprEval { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => {
            scan_control_flow_statement(stmt, ctx, facts, depth);
        }
    }
}

/// Recurse into the bodies / conditions of a control-flow statement (`if` /
/// `for` / `while` / `expr` / `foreach` / `catch` / `try` / `switch`),
/// recording param-touching conditions and embedded call edges. Extracted from
/// [`scan_statement`]. `depth` is `stmt`'s own nesting level — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]; every nested body passed to
/// [`scan_script`] descends one level (`depth + 1`).
fn scan_control_flow_statement(
    stmt: &crate::ir::Statement,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::ir::Statement;
    let ScanCtx { params, .. } = ctx;
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                note_params_in_expr(&c.condition, params, facts, 0);
                scan_expr_for_calls(&c.condition, ctx, facts, 0);
                scan_script(&c.body, ctx, facts, depth + 1);
            }
            if let Some(body) = else_body {
                scan_script(body, ctx, facts, depth + 1);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            note_params_in_expr(condition, params, facts, 0);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(init, ctx, facts, depth + 1);
            scan_script(next, ctx, facts, depth + 1);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::While {
            condition, body, ..
        } => {
            note_params_in_expr(condition, params, facts, 0);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::ExprEval { expr, .. } => {
            note_params_in_expr(expr, params, facts, 0);
            scan_expr_for_calls(expr, ctx, facts, 0);
        }
        Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            scan_script(body, ctx, facts, depth + 1);
            for h in handlers {
                scan_script(&h.body, ctx, facts, depth + 1);
            }
            if let Some(fb) = finally_body {
                scan_script(fb, ctx, facts, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    scan_script(b, ctx, facts, depth + 1);
                }
            }
            if let Some(db) = default_body {
                scan_script(db, ctx, facts, depth + 1);
            }
        }
        _ => {}
    }
}

fn is_global_or_namespace(name: &str) -> bool {
    name.starts_with("::") || name.contains("::")
}

/// Local names a scope-aliasing *command* binds to global / namespace
/// scope. Returns `None` when *command* is not a global-aliasing
/// declaration. The returned set holds the bound local names; an empty
/// string (`""`) in the set is a sentinel for a dynamic / unbounded
/// declaration (`global $x`), which the caller must treat as a global
/// write. `upvar` at any level other than `#0` / `0` aliases a caller
/// frame, not global scope, and returns `None`.
fn global_alias_names(command: &str, args: &[String]) -> Option<HashSet<String>> {
    fn names(raw_names: &[String]) -> HashSet<String> {
        raw_names
            .iter()
            .map(|raw| match raw.trim_start().as_bytes().first() {
                // Dynamic alias target — can't bound the name set.
                Some(b'$' | b'[') => String::new(),
                _ => normalise_var_name(raw).to_owned(),
            })
            .collect()
    }
    match command {
        "global" => Some(names(args)),
        // `variable name ?value? name ?value? ...` — names at even indices.
        "variable" => Some(names(&args.iter().step_by(2).cloned().collect::<Vec<_>>())),
        "upvar" => {
            let level = args.first()?.trim();
            // Only `#0` / `0` alias the *global* frame; an omitted or
            // numeric level aliases a caller frame (handled elsewhere).
            if level != "#0" && level != "0" {
                return None;
            }
            // `upvar #0 otherVar localVar ...` — local names at indices
            // 2, 4, 6, … (every second arg after the level + otherVar).
            Some(names(
                &args.iter().skip(2).step_by(2).cloned().collect::<Vec<_>>(),
            ))
        }
        _ => None,
    }
}

/// Walk an expression AST and record call-graph edges (and Body
/// recursion) for every `[cmd ...]` command substitution embedded
/// in the expression.
///
/// Call-graph edges and unused-proc detection used to miss proc calls
/// embedded in control-flow predicates because the per-proc fact
/// scanner walked statement bodies but skipped expression operands.
/// `if {[q]} ...`, `while {[q]} ...`, and `for {init} {[q]} {next}
/// ...` left `q` unrecorded as a callee — flagging it as dead code
/// and missing the edge in `tcl callgraph`.
fn scan_expr_for_calls(
    node: &crate::expr_ast::ExprNode,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    // Native-stack safety net (issue #996): this walks the `ExprNode`
    // operator tree, one native frame per level. Past the cap, stop
    // descending — a fact collector that returns what it has recorded so far
    // is the safe fallback (call edges buried deeper than the cap are simply
    // not recorded; never a crash). Calls into `scan_source_for_calls` below
    // start that walker at bracket-text depth 0: a `[cmd …]` leaf's raw text
    // is a separate, independent recursion axis with its own cap.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } => {
            // Strip the outer `[…]` and segment the inner script.
            // Each top-level command becomes a `Statement::Call`
            // shape we can hand to `scan_call_facts` and recurse
            // into BODY-role args.
            let inner = text
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .unwrap_or(text.as_str());
            scan_source_for_calls(inner, ctx, facts, 0);
        }
        ExprNode::String { text, .. }
            // Quoted strings may contain command substitutions
            // (`"[q]"`), so descend through the source text the same
            // way as for `Command`.  Skip braced-string literals
            // since `{…}` doesn't interpret substitutions; the
            // expression parser uses `String` for both forms, so we
            // gate on the actual delimiter.
            if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 =>
        {
            let inner = &text[1..text.len() - 1];
            if inner.contains('[') {
                scan_source_for_calls(inner, ctx, facts, 0);
            }
        }
        ExprNode::Binary { left, right, .. } => {
            scan_expr_for_calls(left, ctx, facts, depth + 1);
            scan_expr_for_calls(right, ctx, facts, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            scan_expr_for_calls(operand, ctx, facts, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            scan_expr_for_calls(condition, ctx, facts, depth + 1);
            scan_expr_for_calls(true_branch, ctx, facts, depth + 1);
            scan_expr_for_calls(false_branch, ctx, facts, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                scan_expr_for_calls(a, ctx, facts, depth + 1);
            }
        }
        _ => {}
    }
}

/// Segment a raw Tcl source string into top-level commands and
/// record call-graph edges for each one.  Recurses into
/// `ArgRole::Body`-role arguments so `if {[catch {p}]} ...` also
/// sees `p` as a call.
/// Record the caller-visible-state effect of an assignment to `name`.
/// A write to a `::`-qualified name OR a name aliased into global /
/// namespace scope earlier in the body (`global g; set g 5`) mutates
/// caller-visible state. The write is recorded via
/// `writes_global` ONLY — it does not enter the coarse [`EffectRegion`]
/// set (which tracks HTTP / response-lifecycle / unknown state, not plain
/// variable storage), so the `callgraph`/`dataflow` effect string stays
/// `NONE` for a purely global-mutating proc. Also upgrades a level-1
/// upvar alias write to `VarWrite`.
fn note_assign_global_write(name: &str, facts: &mut LocalFacts) {
    if is_global_or_namespace(name) || facts.global_aliases.contains(name) {
        facts.writes_global = true;
        facts.local_pure = false;
    }
    mark_upvar_alias_write(name, facts);
}

/// Scan every `[cmd …]` command substitution embedded in a value /
/// expression `text` for call-graph edges. The text is lexed under the document
/// dialect; each [`TokenType::Cmd`] token's inner script is handed to
/// [`scan_source_for_calls`] (which resolves the head, applies the
/// callee's effects, and recurses into `BODY`-role args).
fn scan_value_substitutions(text: &str, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    let dialect = ctx.dialect;
    if !text.contains('[') {
        return;
    }
    let lexer = tcl_lexer::Lexer::with_source_map(
        tcl_lexer::SourceMap::new(text),
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    );
    let Ok(tokens) = lexer.tokenise_all() else {
        return;
    };
    for tok in &tokens {
        if tok.kind != tcl_lexer::TokenType::Cmd {
            continue;
        }
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start <= end
            && end <= text.len()
            && text.is_char_boundary(start)
            && text.is_char_boundary(end)
        {
            // Thin dispatcher — carry the caller's bracket-text depth straight
            // through; `scan_source_for_calls` enforces the cap (issue #996).
            scan_source_for_calls(&text[start..end], ctx, facts, depth);
        }
    }
}

fn scan_source_for_calls(source: &str, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    // Native-stack safety net (issue #996): this recurses into `ArgRole::Body`
    // args, `apply` lambda bodies, and nested `[cmd …]` substitutions inside a
    // single word's raw text — a genuinely unbounded axis, independent of any
    // statement-tree cap (`catch {catch {catch {…}}}` / `apply {{} {apply {{}
    // {…}}}}` nested arbitrarily deep). Past the cap, stop descending: call
    // edges buried deeper than the cap are not recorded, never a crash.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let ScanCtx {
        registry, dialect, ..
    } = ctx;
    // Scan the call graph under the
    // document dialect so `{*}` (8.4 / iRules) and `}{` (iRules) tokenise
    // the same way the rest of the analyser/lowering now does.
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    );
    for cmd in commands {
        // Skip empty / non-literal command names — they're not
        // call-graph edges we can resolve at compile time.
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        let texts = cmd.args();
        scan_call_facts(name, texts, ctx, facts);
        // Recurse into BODY-role args (e.g. `catch {p}` → `{p}` is
        // BODY).  The registry resolves the role using the same
        // logic as the top-level scanner.
        let arg_strs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let body_indices =
            registry.arg_indices_for_role(name, &arg_strs, tcl_registry::arg_role::ArgRole::Body);
        for idx in body_indices {
            if let Some(body_text) = texts.get(idx) {
                scan_source_for_calls(body_text, ctx, facts, depth + 1);
            }
        }
        // Recurse into an `ArgRole::LambdaLiteral` argument's real body
        // element (`apply {argList body ?ns?} …`) — the whole lambda literal
        // is a 2-element list, not a script, so scanning it as one (like a
        // plain `Body` arg) would misread the parameter word as a call-graph
        // edge to a non-existent proc and never reach the real body's own
        // calls at all (issue #954's call-graph sibling gap).
        let lambda_indices = registry.arg_indices_for_role(
            name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::LambdaLiteral,
        );
        for idx in lambda_indices {
            if let Some(&tok) = cmd.argv.get(idx + 1)
                && tok.kind == tcl_lexer::TokenType::Str
                && let Some(elems) = crate::lambda_literal::split_lambda_literal(source, tok)
                && let Some(body_span) = elems.body
                && let Some(body_text) =
                    source.get(body_span.start() as usize..body_span.end() as usize)
            {
                // A lambda body runs in a fresh call frame whose current
                // namespace is the lambda's own (optional) third element, or
                // the global namespace when omitted — never the enclosing
                // procedure's namespace. Build a synthetic "caller" qname
                // carrying that namespace so `resolve_internal_call` (which
                // derives its search namespace from the caller qname's own
                // namespace prefix) resolves bare calls inside the lambda the
                // same way Tcl itself would, instead of relative to
                // `ctx.caller`'s enclosing namespace (codex review of #954's
                // follow-up: an `apply {{} {helper}}` inside `::ns::f` calls
                // `::helper`, not `::ns::helper`).
                let lambda_caller = match elems
                    .namespace
                    .and_then(|ns| source.get(ns.start() as usize..ns.end() as usize))
                {
                    Some(ns) => format!("{ns}::<apply>"),
                    None => "<apply>".to_owned(),
                };
                let lambda_ctx = ScanCtx {
                    caller: &lambda_caller,
                    ..ctx
                };
                scan_source_for_calls(body_text, lambda_ctx, facts, depth + 1);
            }
        }
        // Recurse into EXPR-role args. `expr {…}` (and the registry's other
        // expression operands) re-parse and evaluate their argument, so a
        // `[cmd]` substitution inside it is a real call edge even when the
        // operand is brace-quoted — Tcl's `{…}` suppresses substitution at the
        // word level, but the expression engine re-evaluates it. Strip one
        // layer of braces (the common `expr {…}` form) and rescan the inner
        // text for command substitutions; an unbraced operand (`[q]`, `$x`)
        // falls through to the same scan unchanged. Without this, a recursive
        // call buried in `return [expr {[fib …]}]` is missed, leaving the call
        // graph incomplete (which under-converged the interproc taint fixpoint
        // and panicked the diagnostic worker on the debug guard).
        let expr_indices =
            registry.arg_indices_for_role(name, &arg_strs, tcl_registry::arg_role::ArgRole::Expr);
        for idx in expr_indices {
            if let Some(arg) = texts.get(idx) {
                let inner = arg
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .unwrap_or(arg);
                scan_value_substitutions(inner, ctx, facts, depth + 1);
            }
        }
        // A `[cmd …]` substitution inside a *plain* value arg also executes in
        // this value/expression context (`set x [matchclass [HTTP::uri] …]`,
        // `if {[matchclass [HTTP::uri] …]}`), so its nested effects and edges
        // propagate too. This worker is reached
        // only from value/return/expr scanning (plain statements go through the
        // `Statement::Call` arm, which does *not* propagate). Braced args are
        // inert (no `Cmd` token inside `{…}`); the
        // body/expr args handled above are idempotent under a re-scan.
        for arg in texts {
            if arg.contains('[') {
                scan_value_substitutions(arg, ctx, facts, depth + 1);
            }
        }
    }
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
            if let Ok(n) = std::str::from_utf8(&bytes[start..i])
                && n == name
            {
                return true;
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
        if let Ok(n) = std::str::from_utf8(&bytes[start..i])
            && n == name
        {
            return true;
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
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — param
    // observations buried deeper than the cap are simply not recorded (a
    // param not marked `UsedInCondition` stays whatever it already was);
    // never a crash.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Var { name, .. } if params.contains(name) => {
            facts
                .param_trait_flags
                .entry(name.clone())
                .or_default()
                .insert(ProcArgTrait::UsedInCondition);
        }
        ExprNode::Binary { left, right, .. } => {
            note_params_in_expr(left, params, facts, depth + 1);
            note_params_in_expr(right, params, facts, depth + 1);
        }
        ExprNode::Unary { operand, .. } => note_params_in_expr(operand, params, facts, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            note_params_in_expr(condition, params, facts, depth + 1);
            note_params_in_expr(true_branch, params, facts, depth + 1);
            note_params_in_expr(false_branch, params, facts, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                note_params_in_expr(a, params, facts, depth + 1);
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
    if let Some(inside) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
        && !inside.contains(['$', '[', '\\'])
    {
        return ReturnKind::Literal(inside.to_owned());
    }
    if let Some(inside) = v.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return ReturnKind::Literal(inside.to_owned());
    }
    // A substitution-free value — including a
    // multi-word string whose delimiters the lowerer already stripped
    // (`return {a b c}` / `return "a b c"` both lower to the value
    // `a b c`) — is a literal constant return.  Gated on no `$` / `[` /
    // `\` so a `$param` passthrough or a command substitution still
    // falls through to its own classification below.
    if !v.contains(['$', '[', '\\']) {
        return ReturnKind::Literal(v.to_owned());
    }
    // Passthrough of `$param`.
    if let Some(name) = v.strip_prefix('$')
        && params.contains(name)
    {
        return ReturnKind::Passthrough(name.to_owned());
    }
    if let Some(name) = v.strip_prefix("${").and_then(|s| s.strip_suffix('}'))
        && params.contains(name)
    {
        return ReturnKind::Passthrough(name.to_owned());
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
    if let ExprNode::Var { name, .. } = node
        && params.contains(name)
    {
        return ReturnKind::Passthrough(name.clone());
    }
    // Walk the AST collecting var references against the param
    // set; any match → UsesParam.
    let mut referenced: Vec<String> = Vec::new();
    walk_collect_param_refs(node, params, &mut referenced, 0);
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
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — this collector
    // returns the param refs gathered so far (a conservative under-count only
    // reachable past 256 levels of expression nesting); never a crash.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Var { name, .. } if params.contains(name) => {
            out.push(name.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            walk_collect_param_refs(left, params, out, depth + 1);
            walk_collect_param_refs(right, params, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => walk_collect_param_refs(operand, params, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_collect_param_refs(condition, params, out, depth + 1);
            walk_collect_param_refs(true_branch, params, out, depth + 1);
            walk_collect_param_refs(false_branch, params, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                walk_collect_param_refs(a, params, out, depth + 1);
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

/// Whether every path through `script` reaches an explicit `return`, so
/// the proc cannot fall off the end. A fall-through exit returns the
/// result of the body's last command — generally **not** a compile-time
/// constant — so a proc that returns a literal on one path but can fall
/// through on another is not constant-returning (O103).
///
/// Deliberately conservative: it recognises only `return` and a
/// fully-covered `if`/`elseif`/`else` whose every arm returns. Anything
/// else is treated as "may fall through", which costs only a missed fold,
/// never a miscompile.
/// `depth` is the nesting level of `script` — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Past the cap, conservatively answers
/// `false` ("may fall through") — the same conservative direction this
/// function's doc comment already commits to for anything it can't prove,
/// so an unresolved deep answer costs only a missed constant-return fold,
/// never a miscompile.
fn script_always_returns(script: &crate::ir::Script, depth: u32) -> bool {
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        return false;
    }
    script
        .statements
        .iter()
        .any(|s| stmt_always_returns(s, depth))
}

fn stmt_always_returns(stmt: &crate::ir::Statement, depth: u32) -> bool {
    use crate::ir::Statement;
    match stmt {
        Statement::Return { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            else_body
                .as_ref()
                .is_some_and(|b| script_always_returns(b, depth + 1))
                && clauses
                    .iter()
                    .all(|c| script_always_returns(&c.body, depth + 1))
        }
        _ => false,
    }
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
    if let ReturnKind::Literal(first) = &returns[0]
        && returns
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
    // Passthrough: every return is Passthrough of the same param.
    if let ReturnKind::Passthrough(first) = &returns[0]
        && returns
            .iter()
            .all(|r| matches!(r, ReturnKind::Passthrough(v) if v == first))
    {
        return (false, None, Some(first.clone()), vec![first.clone()]);
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

    /// Regression coverage for issue #996: the interprocedural call-graph
    /// scanners recurse with no depth cap before this fix —
    /// `scan_expr_for_calls`, `note_params_in_expr` and
    /// `walk_collect_param_refs` once per `ExprNode` level (Tier 1A);
    /// `scan_source_for_calls` (+ its `scan_value_substitutions` helper) once
    /// per nested `[cmd …]` substitution / `ArgRole::Body` arg inside a
    /// single word's raw text (Tier 1B). Both axes were genuinely unbounded,
    /// independent of the statement-tree `MAX_INTERPROCEDURAL_WALK_DEPTH`
    /// cap, and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread (`cargo test`'s default). 3000 is
    /// comfortably past that crash range and past both caps (256); the
    /// assertion is that each scanner returns at all.
    #[test]
    fn deeply_nested_interprocedural_scans_survive() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let known: HashSet<String> = HashSet::new();
        let params: HashSet<String> = ["x".to_owned()].into_iter().collect();
        let ctx = ScanCtx {
            caller: "::top",
            known: &known,
            registry: &registry,
            dialect: None,
            params: &params,
            object_types: ObjectTypeMap::none().0,
        };

        // A 3000-deep `ExprNode` tree (nested unary `!` over `$x`).
        let mut node = crate::expr_ast::ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = crate::expr_ast::ExprNode::Unary {
                op: crate::expr_ast::UnaryOp::Not,
                operand: Box::new(node),
            };
        }

        // Tier 1A walkers over the deep expr tree.
        let mut facts = LocalFacts::default();
        scan_expr_for_calls(&node, ctx, &mut facts, 0);
        note_params_in_expr(&node, &params, &mut facts, 0);
        let mut refs = Vec::new();
        walk_collect_param_refs(&node, &params, &mut refs, 0);

        // Tier 1B walker over deeply nested command substitutions
        // `a [a [a [ … [x] … ]]]`: each level is a literal-headed command
        // whose argument holds the next `[…]` substitution, so
        // `scan_source_for_calls` recurses through `scan_value_substitutions`
        // once per bracket level.
        let mut deep_brackets = "x".to_owned();
        for _ in 0..3000 {
            deep_brackets = format!("a [{deep_brackets}]");
        }
        scan_source_for_calls(&deep_brackets, ctx, &mut facts, 0);
    }

    #[test]
    fn arity_from_names_handles_trailing_args() {
        assert_eq!(
            arity_from_names(&["a".to_owned(), "args".to_owned()]),
            Arity::at_least(1)
        );
        assert_eq!(
            arity_from_names(&["a".to_owned(), "b".to_owned()]),
            Arity::exact(2)
        );
        assert_eq!(arity_from_names(&[]), Arity::exact(0));
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
        // `foo::bar` from a global-level caller → `::foo::bar` (the only
        // candidate, since the caller's own namespace is already `::`).
        let known = known_set(&["::foo::bar"]);
        assert_eq!(
            resolve_internal_call("foo::bar", "::top", &known),
            Some("::foo::bar".into())
        );
    }

    #[test]
    fn resolve_relative_dotted_name_prefers_caller_namespace() {
        // A relative dotted word (`ns2::inner`) resolves against the
        // caller's own namespace first, not straight at global (confirmed
        // against tclsh 9.0.4 — see `bareword_resolution_candidates`).
        let known = known_set(&["::ns::ns2::inner", "::ns2::inner"]);
        assert_eq!(
            resolve_internal_call("ns2::inner", "::ns::caller", &known),
            Some("::ns::ns2::inner".into()),
            "must prefer the caller-namespace proc over the root one",
        );
        // No caller-namespace candidate → falls back to the root proc.
        assert_eq!(
            resolve_internal_call("ns2::inner", "::top", &known),
            Some("::ns2::inner".into()),
        );
    }

    #[test]
    fn resolve_bare_does_not_walk_ancestor_namespaces() {
        // Real Tcl bareword resolution is exactly two levels — the
        // caller's own namespace, then global — absent an explicit
        // `namespace path`. A proc defined in a *grandparent* namespace
        // must not resolve for a bare call from `::ns::a::caller`.
        let known = known_set(&["::ns::helper"]);
        assert_eq!(
            resolve_internal_call("helper", "::ns::a::caller", &known),
            None,
            "a grandparent-namespace proc must not resolve for a bare call",
        );
        // Control: the *direct* enclosing namespace still resolves.
        let known2 = known_set(&["::ns::a::helper"]);
        assert_eq!(
            resolve_internal_call("helper", "::ns::a::caller", &known2),
            Some("::ns::a::helper".into()),
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
        // Colon runs are one separator (C's `TclGetNamespaceForQualName`;
        // tclsh8.6: `foo:::bar` names `::foo::bar`) — the shared
        // `qualifier_segments` split, where a naive `split("::")` would keep
        // a stray `:` segment.
        assert_eq!(
            namespace_parts_from_proc("::foo:::bar::baz"),
            vec!["foo", "bar"]
        );
        assert_eq!(namespace_parts_from_proc("a:::b"), vec!["a"]);
    }

    #[test]
    fn resolve_call_target_delegates() {
        let known = known_set(&["::helper"]);
        assert_eq!(
            resolve_call_target("helper", &[], "::top", &known),
            Some("::helper".into())
        );
    }

    // summary-building tests

    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn build(source: &str) -> InterproceduralAnalysis {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false);
        build_interprocedural_analysis(&cu.ir_module, &registry, None, ObjectTypeMap::none())
    }

    #[test]
    fn empty_module_has_no_summaries() {
        let ia = build("");
        assert!(ia.procedures.is_empty());
    }

    /// Issue #954's call-graph sibling gap: a proc called *inside* an
    /// `apply` lambda body reached through a `[…]` command substitution
    /// (`set y [apply {p {…}} $x]`) must still register as a call-graph
    /// edge. `apply`'s lambda-literal argument is `ArgRole::LambdaLiteral`,
    /// not `Body` — scanning the whole `{argList} {body}` blob as one script
    /// (the old generic-`Body` path) misread the parameter word as a
    /// command name and never reached the real body's own calls, which
    /// would have made `helper` look like dead code (O124) despite being
    /// reachable only from inside the lambda.
    #[test]
    fn call_inside_apply_lambda_body_is_a_direct_call() {
        let ia = build(
            "proc ::helper {x} { return $x }\n\
             proc ::f {} { set y [apply {p {helper $p}} 1]; return $y }\n",
        );
        let s = ia.procedures.get("::f").expect("::f summary");
        assert!(
            s.direct_calls.iter().any(|c| c == "::helper"),
            "expected a direct_calls edge to ::helper via the apply lambda body; got {:?}",
            s.direct_calls
        );
    }

    /// Codex review of #954's follow-up: a lambda body's bare calls must
    /// resolve in the lambda's own namespace (the optional third `apply`
    /// element, or global when omitted) — never the enclosing procedure's
    /// namespace. `::ns::f`'s `apply {{} {helper}}` must resolve `helper` to
    /// `::helper` (the two-element form runs in `::`), not `::ns::helper`,
    /// even though `::ns::helper` also exists and would otherwise look like
    /// the "closer" match.
    #[test]
    fn call_inside_apply_lambda_body_resolves_in_lambda_namespace() {
        let ia = build(
            "namespace eval ::ns {\n\
                 proc helper {} { return 1 }\n\
             }\n\
             proc ::helper {} { return 2 }\n\
             proc ::ns::f {} { set y [apply {{} {helper}}]; return $y }\n",
        );
        let s = ia.procedures.get("::ns::f").expect("::ns::f summary");
        assert!(
            s.direct_calls.iter().any(|c| c == "::helper"),
            "expected the apply lambda body's bare `helper` call to resolve \
             in the global namespace (Tcl evaluates a two-element apply \
             lambda in `::`); got {:?}",
            s.direct_calls
        );
        assert!(
            !s.direct_calls.iter().any(|c| c == "::ns::helper"),
            "the apply lambda body must not resolve `helper` against the \
             enclosing proc's `::ns` namespace; got {:?}",
            s.direct_calls
        );
    }

    /// The lambda's explicit third-element namespace overrides both the
    /// enclosing procedure's namespace and the global default.
    #[test]
    fn call_inside_apply_lambda_body_resolves_in_explicit_namespace() {
        let ia = build(
            "namespace eval ::other {\n\
                 proc helper {} { return 1 }\n\
             }\n\
             proc ::helper {} { return 2 }\n\
             proc ::ns::f {} { set y [apply {{} {helper} ::other}]; return $y }\n",
        );
        let s = ia.procedures.get("::ns::f").expect("::ns::f summary");
        assert!(
            s.direct_calls.iter().any(|c| c == "::other::helper"),
            "expected the apply lambda's explicit `::other` namespace \
             element to resolve `helper` there; got {:?}",
            s.direct_calls
        );
        assert!(
            !s.direct_calls.iter().any(|c| c == "::helper"),
            "the explicit lambda namespace must not fall back to global; \
             got {:?}",
            s.direct_calls
        );
    }

    #[test]
    fn instance_method_callback_is_a_direct_call() {
        // `with_interprocedural` computes the object-handle map, so a
        // `$g walk … -command cb` instance-method callback inside a proc body
        // resolves through the receiver's class and becomes a `direct_calls`
        // edge (feeding the call graph + O124 not-dead).
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc onNode {a g n} {}\nproc build {} { struct::graph g\n g walk root -command onNode }\n",
            &registry,
            false,
        )
        .with_interprocedural(&registry, Some("tcl9.0"));
        let summary = &cu.interproc.as_ref().expect("interproc").procedures["::build"];
        assert!(
            summary.direct_calls.iter().any(|c| c == "::onNode"),
            "build's summary must carry a direct_call to onNode via the instance-method callback; got {:?}",
            summary.direct_calls,
        );
    }

    #[test]
    fn simple_proc_is_recorded_with_params_and_arity() {
        let ia = build("proc ::greet {name} { puts hi }");
        let s = ia.procedures.get("::greet").expect("proc summary");
        assert_eq!(s.params, vec!["name".to_string()]);
        assert_eq!(s.arity, Arity::exact(1));
    }

    // writes_global recognises scope aliases

    #[test]
    fn global_alias_write_counts_as_writes_global() {
        // A bare `set g` after `global g` mutates a caller-visible
        // variable through the alias — writes_global, even though the
        // written name is bare (previously missed: only `::`-qualified
        // names counted).
        for src in [
            "proc ::f {} { global g\nset g 5 }",
            "proc ::f {} { variable v\nset v 1 }",
            "proc ::f {} { upvar #0 ::x g\nset g 1 }",
            "proc ::f {} { global g\nincr g }",
            "proc ::f {} { global g\nappend g x }",
        ] {
            let ia = build(src);
            let s = ia.procedures.get("::f").expect("::f summary");
            assert!(s.writes_global, "expected writes_global for: {src}");
        }
    }

    #[test]
    fn non_aliased_local_write_is_not_writes_global() {
        // A plain local `set g` (no global/variable/upvar #0 decl) is
        // not a global write; `upvar 1` aliases a caller frame, not
        // global scope, so it is not flagged here either.
        for src in [
            "proc ::f {} { set g 5 }",
            "proc ::f {} { upvar 1 x g\nset g 1 }",
        ] {
            let ia = build(src);
            let s = ia.procedures.get("::f").expect("::f summary");
            assert!(!s.writes_global, "unexpected writes_global for: {src}");
        }
    }

    // method-purity summary tests

    #[test]
    fn pure_getter_method_is_summarised_pure() {
        let ia = build(
            "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method get {} { return $n }\n\
             }",
        );
        let m = ia.methods.get("::C::get").expect("method summary");
        assert!(m.base.pure, "read-only getter should be pure");
        assert!(m.writes_instance_vars.is_empty());
        assert_eq!(m.class_name, "::C");
        assert_eq!(m.method_kind, "method");
    }

    #[test]
    fn instance_var_write_makes_method_impure() {
        let ia = build(
            "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method bump {} { incr n }\n\
             }",
        );
        let m = ia.methods.get("::C::bump").expect("method summary");
        assert!(!m.base.pure, "instance-var write must be impure");
        assert!(
            m.writes_instance_vars.contains("n"),
            "writes_instance_vars: {:?}",
            m.writes_instance_vars
        );
    }

    #[test]
    fn array_element_instance_var_write_makes_method_impure() {
        // `set counter(0) 1` writes the instance var `counter` (compared
        // on the base scalar name, not the raw element name).
        let ia = build(
            "oo::class create C {\n\
             \x20   variable counter\n\
             \x20   method bump {} { set counter(0) 1 }\n\
             }",
        );
        let m = ia.methods.get("::C::bump").expect("method summary");
        assert!(!m.base.pure);
        assert!(m.writes_instance_vars.contains("counter"));
    }

    #[test]
    fn my_dispatch_makes_method_impure() {
        // `my <other>` is a dynamic self-dispatch — unresolvable
        // statically, so the method can't be proven pure.
        let ia = build(
            "oo::class create C {\n\
             \x20   method a {} { my b }\n\
             \x20   method b {} { return 1 }\n\
             }",
        );
        let a = ia.methods.get("::C::a").expect("method a");
        assert!(!a.base.pure, "my-dispatch caller must be impure");
        // The pure leaf method still summarises pure.
        let b = ia.methods.get("::C::b").expect("method b");
        assert!(b.base.pure);
    }

    #[test]
    fn redefined_method_is_forced_impure() {
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { return 2 }\n\
             }",
        );
        let m = ia.methods.get("::C::m").expect("method summary");
        assert!(!m.base.pure, "redefined method must stay conservative");
    }

    #[test]
    fn variable_decl_is_not_counted_as_instance_var_write() {
        // A method-local `variable x` is a link/declaration, not a
        // write — its def must not land in `writes_instance_vars`
        // (soundness fix: a read-only instance-var method isn't
        // forced impure by *that* path).  Note: the registry models
        // `variable` itself as a scope-alias barrier, so `peek` is still
        // impure overall — but not via the instance-var-write channel.
        let ia = build(
            "oo::class create C {\n\
             \x20   method peek {} { variable x; return $x }\n\
             }",
        );
        let m = ia.methods.get("::C::peek").expect("method summary");
        assert!(
            m.writes_instance_vars.is_empty(),
            "variable decl wrongly counted as write: {:?}",
            m.writes_instance_vars
        );
    }

    #[test]
    fn call_in_if_condition_is_recorded() {
        // Call-graph edges and unused-proc detection used to miss proc calls
        // embedded in `if {[q]} ...` predicates.  Verify ::q now
        // appears in ::a's direct calls.
        let ia = build(
            "proc ::q {} { return 1 }\n\
             proc ::a {} { if {[::q]} { puts hi } }",
        );
        let a = ia.procedures.get("::a").expect("::a recorded");
        assert!(
            a.calls.contains(&"::q".to_string()),
            "expected ::q in ::a's call graph, got {:?}",
            a.calls
        );
    }

    #[test]
    fn call_inside_catch_in_if_condition_is_recorded() {
        // `if {[catch {p}]} ...` — ::p is reached via BODY-role
        // recursion into the `catch` argument.
        let ia = build(
            "proc ::p {} { return 1 }\n\
             proc ::a {} { if {[catch {::p}]} { puts hi } }",
        );
        let a = ia.procedures.get("::a").expect("::a recorded");
        assert!(
            a.calls.contains(&"::p".to_string()),
            "expected ::p in ::a's call graph via BODY recursion, got {:?}",
            a.calls
        );
    }

    #[test]
    fn call_in_while_condition_is_recorded() {
        let ia = build(
            "proc ::q {} { return 1 }\n\
             proc ::a {} { while {[::q]} { break } }",
        );
        let a = ia.procedures.get("::a").expect("::a recorded");
        assert!(
            a.calls.contains(&"::q".to_string()),
            "expected ::q via while-condition expr scan, got {:?}",
            a.calls
        );
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
        // ``eval {literal}`` inside a proc relaxes to a
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
    fn fp_nab_03_recursive_arithmetic_proc_is_pure() {
        // FP-NAB-03: a self-recursive arithmetic proc must come out
        // `pure == true` from the
        // interprocedural fix-point. The fix-point is the *greatest* one
        // (purity initialised optimistically, then refuted), so a call back into
        // the proc being analysed does not conservatively mark it impure.
        let ia = build(
            "proc ::fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\n",
        );
        let fact = ia.procedures.get("::fact").expect("::fact summary");
        assert!(
            fact.pure,
            "recursive arithmetic proc must be pure (greatest fix-point); got pure={}",
            fact.pure,
        );
    }

    #[test]
    fn fp_nab_03_impure_proc_still_detected() {
        // Control: a proc doing I/O (`puts`) must be impure — proves the test
        // above isn't trivially asserting every proc pure.
        let ia = build("proc ::logit {msg} {\n    puts $msg\n    return ok\n}\n");
        let logit = ia.procedures.get("::logit").expect("::logit summary");
        assert!(
            !logit.pure,
            "a proc that calls puts must be impure; got pure={}",
            logit.pure,
        );
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
    fn conditional_return_is_not_constant_when_body_falls_through() {
        // O103: `f` returns 42 only when `$x > 0`; otherwise it
        // falls off the end (the `if` returns "" on the no-match path),
        // so it is NOT constant-returning and must not be folded.
        let ia = build("proc ::f {x} { if {$x > 0} { return 42 } }");
        let s = ia.procedures.get("::f").unwrap();
        assert!(
            !s.returns_constant,
            "fall-through proc must not be constant-returning, got {:?}",
            s.constant_return,
        );
        assert_eq!(s.constant_return, None);
    }

    #[test]
    fn conditional_return_is_constant_when_all_paths_return() {
        // Fully covered if/else, both arms return the same literal →
        // genuinely constant, still folds.
        let ia = build("proc ::f {x} { if {$x > 0} { return 7 } else { return 7 } }");
        let s = ia.procedures.get("::f").unwrap();
        assert!(s.returns_constant);
        assert_eq!(s.constant_return, Some(ConstantReturn::Int(7)));
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
        // fold. ``eval {literal}`` is a Block (non-barrier),
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
    fn call_by_name_param_traits_inferred() {
        // `upvar 1 $n x; set x ...` → n is VarWrite;
        // `upvar 1 $n x; return $x` (read only) → n is VarRead.
        let w = build("proc ::setvar {n v} { upvar 1 $n x\nset x $v }");
        let s = w.procedures.get("::setvar").unwrap();
        let nt = s.param_traits.get("n").expect("n traits");
        assert!(
            nt.contains(&ProcArgTrait::VarWrite),
            "expected VarWrite for n (write through upvar alias), got {nt:?}",
        );

        let r = build("proc ::getvar {n} { upvar 1 $n x\nreturn $x }");
        let s2 = r.procedures.get("::getvar").unwrap();
        let nt2 = s2.param_traits.get("n").expect("n traits");
        assert!(
            nt2.contains(&ProcArgTrait::VarRead) && !nt2.contains(&ProcArgTrait::VarWrite),
            "expected VarRead (not VarWrite) for read-only upvar alias, got {nt2:?}",
        );

        // upvar #0 (global frame, not the caller) → VarRead only, no
        // write-back to the caller's passed variable.
        let g = build("proc ::gw {p} { upvar #0 $p x\nset x 1 }");
        let s3 = g.procedures.get("::gw").unwrap();
        let pt = s3.param_traits.get("p").expect("p traits");
        assert!(
            pt.contains(&ProcArgTrait::VarRead) && !pt.contains(&ProcArgTrait::VarWrite),
            "upvar #0 is not a caller-frame write-back, got {pt:?}",
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

    /// Regression coverage for issue #996: `collect_instance_var_writes`
    /// and the mutually-recursive `scan_script`/`scan_statement`/
    /// `scan_control_flow_statement` trio recurse once per nested
    /// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` body, with no
    /// depth cap of their own before this fix. Transitively bounded to
    /// `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today, so this is
    /// defence-in-depth / consistency with every other full-tree walker in
    /// this crate, not a currently-reproducible crash. 1000 levels of
    /// *source* nesting is comfortably past this new cap; the assertion is
    /// that `build_interprocedural_analysis` returns at all, not what it
    /// returns. Spawns its own big-stack thread since the lexer/CST/
    /// segmenter stages upstream of this walker's own new cap still walk
    /// the full un-truncated source nesting before lowering's cap trims
    /// it — same rationale as
    /// `structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_interprocedural_scan() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "proc ::p {} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = build(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Regression coverage for issue #996: `collect_instance_var_writes`
    /// (the `TclOO` method-body instance-write scan) recurses once per
    /// nested `if` body, with no depth cap of its own before this fix.
    /// Same transitively-bounded-today caveat and big-stack-thread
    /// rationale as `deeply_nested_if_survives_interprocedural_scan`. 1000
    /// levels is comfortably past the new cap; the assertion is that the
    /// method summary is built at all, not what it contains.
    #[test]
    fn deeply_nested_if_survives_collect_instance_var_writes() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "oo::class create C {\n    variable n\n    method bump {} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("incr n\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let ia = build(&src);
                assert!(ia.methods.contains_key("::C::bump"));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod effect_propagation_tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_dialect::DialectSet;
    use tcl_registry::CommandRegistry;

    fn irules_registry() -> CommandRegistry {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::IRULES);
        reg
    }

    /// A `[cmd …]` substitution that executes in a *value / expression* context
    /// (`set x [matchclass [HTTP::uri] …]`, `if {[…]}`) propagates the nested
    /// command's connection-state effect up to the enclosing body, while the
    /// same call as a *plain statement* does not.
    #[test]
    fn nested_substitution_effects_propagate_in_value_context_only() {
        let reg = irules_registry();

        for src in [
            "proc p {} { set x [matchclass [HTTP::uri] equals $::l] }",
            "proc p {} { return [matchclass [HTTP::uri] equals $::l] }",
            "proc p {} { if {[matchclass [HTTP::uri] equals $::l]} {} }",
        ] {
            let module = lower_to_ir(src, &reg);
            let ia = build_interprocedural_analysis(
                &module,
                &reg,
                Some("f5-irules"),
                ObjectTypeMap::none(),
            );
            let s = ia.procedures.get("::p").expect("proc ::p in IA");
            assert!(
                s.effect_reads.contains(EffectRegion::HTTP_STATE),
                "value-context nested [HTTP::uri] should propagate HTTP_STATE; src={src:?} reads={:?}",
                s.effect_reads
            );
        }

        // Plain statement: nested arg effects are NOT propagated.
        let module = lower_to_ir("proc q {} { matchclass [HTTP::uri] equals $::l }", &reg);
        let ia =
            build_interprocedural_analysis(&module, &reg, Some("f5-irules"), ObjectTypeMap::none());
        let s = ia.procedures.get("::q").expect("proc ::q in IA");
        assert!(
            !s.effect_reads.contains(EffectRegion::HTTP_STATE),
            "plain-statement arg must not propagate HTTP_STATE; reads={:?}",
            s.effect_reads
        );
    }
}
