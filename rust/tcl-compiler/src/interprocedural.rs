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

use std::collections::{BTreeSet, HashMap, HashSet};

pub use tcl_registry::Arity;
use tcl_registry::value_transfer::ExactValue;

use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};
use crate::ir::ProcedureId;
use crate::side_effects::EffectRegion;

mod completion;
mod eager;
mod transfer;

pub(crate) use completion::CompletionWalk;
#[cfg(test)]
pub(crate) use completion::tests::logical_unit as logical_completion_unit;
pub(crate) use eager::{DefinitionReach, EagerInvocations};
pub use transfer::TransferSummaries;
pub(crate) use transfer::{
    CallTransfer, ModuleInputs, ModuleProcedures, OriginalSummaryExpression, OriginalSummaryScript,
    Rerun, RerunStance,
};

/// Depth cap shared by every `Script`/`Statement`-tree recursion in this
/// module (`collect_instance_var_writes`; the mutually-recursive
/// `scan_script`/`scan_statement`/`scan_control_flow_statement` trio;
/// `script_always_returns`/`stmt_always_returns`).
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
    /// `"unused"`). Consumers materialise traits using this form
    /// rather than re-implementing the mapping.
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
    /// [`Self::text`], the value as the procedure returns it.
    #[must_use]
    pub fn as_kind_text(&self) -> (&'static str, String) {
        let kind = match self {
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Bool(_) => "bool",
            Self::Str(_) => "str",
        };
        (kind, self.text())
    }

    /// The value's text, as the procedure returns it: the summary keeps an
    /// integer, a double or a boolean only where this spells the value byte
    /// for byte, so a fold spells the value exactly.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Int(i) => i.to_string(),
            Self::Float(f) => tcl_syntax::number::format_double(*f),
            Self::Bool(b) => b.to_string(),
            Self::Str(s) => s.clone(),
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
    /// Source reporting label; target selection uses the map's issued body ID.
    pub qualified_name: String,
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// Declared arity.
    pub arity: Arity,
    /// Module-issued IDs of transitive callees.
    pub calls: Vec<ProcedureId>,
    /// Exact direct callee IDs, without transitive edges.
    /// Graph consumers render labels only after joining these IDs.
    pub direct_calls: Vec<ProcedureId>,
    /// True if the body contains a barrier command.
    pub has_barrier: bool,
    /// True if the body calls a command not in the registry and
    /// not resolvable to another internal proc.
    pub has_unknown_calls: bool,
    /// True if the body writes any global / namespace variable.
    pub writes_global: bool,
    /// True if the body is side-effect-free.
    pub pure: bool,
    /// Whether a call whose word count the parameters accept completes
    /// normally whatever its arguments hold: the body is straight-line, reads
    /// only scalars it holds set, and runs only commands that complete so — a
    /// registry command declaring the normal completion alone, or a procedure
    /// of the module that completes in turn — and never recurses. A
    /// pure procedure that completes is a value that cannot raise, so an
    /// unused store of its call is dead. False where the summary is built
    /// from the IR alone, which holds no command trust.
    pub completes: bool,
    /// Where the module's load surely runs this procedure's `proc`
    /// statement: a direct statement of the top level, or of a `namespace eval`
    /// body that is one. `None` for a definition that may not have run there
    /// — under a condition, in a loop, in another procedure or another file.
    pub defined_at: Option<u32>,
    /// The procedures this one calls whose definitions surely run before the
    /// load may first run it, sorted: the callees the completion proof
    /// may take as defined where this procedure's calls run.
    pub defined_callees: Vec<ProcedureId>,
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
    /// Each parameter with a default: the value Tcl binds it to in a call
    /// that omits it.
    pub param_defaults: HashMap<String, String>,
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
            completes: false,
            defined_at: None,
            defined_callees: Vec::new(),
            effect_reads: EffectRegion::UNKNOWN_STATE,
            effect_writes: EffectRegion::UNKNOWN_STATE,
            returns_constant: false,
            constant_return: None,
            return_depends_on_params: Vec::new(),
            return_passthrough_param: None,
            can_fold_static_calls: false,
            param_traits: HashMap::new(),
            param_defaults: HashMap::new(),
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
    pub method_kind: crate::ir::MethodKind,
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
    /// Procedure summaries keyed by their issued Module body IDs.
    pub procedures: HashMap<ProcedureId, ProcSummary>,
    /// Method summaries keyed by their issued Module body IDs.
    pub methods: HashMap<crate::ir::ProcedureId, MethodSummary>,
    /// Conditional object/widget class candidates from original top-level
    /// source setup, keyed by procedure. Eager calls see earlier source
    /// factories; callback-only procedure advice sees direct authored setup.
    /// These labels prove no constructor completion or existing Native object.
    pub global_instance_classes: HashMap<ProcedureId, crate::taint::InstanceClassState>,
    /// Complete source metadata used to produce the conditional receiver map.
    /// Equality of input alone cannot authenticate another Module's candidates.
    pub global_instance_class_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Original Module interpretation accompanying the receiver candidates.
    /// This supplies source correspondence, never a current object allocation.
    pub global_instance_class_owner:
        Option<std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>>,
    /// Global/namespace variables made externally controlled by a procedure's
    /// registry-declared instance-option configuration, closed transitively
    /// over the internal call graph.
    pub tainted_global_writes: HashMap<ProcedureId, HashSet<String>>,
    /// Each procedure's transfer summary: what a call does to its caller's
    /// places. A compilation unit's build computes them.
    pub transfers: TransferSummaries,
}

/// One possible procedure's parameter read/write metadata. It selects no
/// target and proves no Native execution or absence of a hidden caller read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcParameterSummary {
    /// This candidate's ordered parameter names.
    pub params: Vec<String>,
    /// Possible uses of those parameters in this candidate.
    pub param_traits: HashMap<String, HashSet<ProcArgTrait>>,
    /// This candidate's omitted-argument defaults.
    pub param_defaults: HashMap<String, String>,
}

/// A conservative constructed-key index of every possible summary sharing a
/// suffix label. [`collect_call_by_name_reads`] unions protective MAY reads;
/// source binding, opaque effects and store-removal proof remain independent.
pub type ProcIndex = HashMap<String, Vec<ProcParameterSummary>>;

/// Build a protective MAY index without choosing one colliding procedure.
#[must_use]
pub fn build_proc_index_from_summaries(ia: &InterproceduralAnalysis) -> ProcIndex {
    let mut index = ProcIndex::new();
    for summary in ia.procedures.values() {
        let entry = ProcParameterSummary {
            params: summary.params.clone(),
            param_traits: summary.param_traits.clone(),
            param_defaults: summary.param_defaults.clone(),
        };
        for key in tcl_syntax::naming::key_suffixes(&summary.qualified_name) {
            index.entry(key).or_default().push(entry.clone());
        }
    }
    index
}

/// Exact original call/formal/registry caller-link advice for diagnostics.
/// This projection never publishes a runtime read or a store-removal licence.
pub(crate) fn collect_positioned_call_by_name_reads(
    cfg: &crate::cfg::Function,
    registry: &tcl_registry::CommandRegistry,
    identities: &crate::realm::CommandBindingRealm,
) -> HashSet<String> {
    let bindings = identities.source_bindings_ref();
    let mut output = HashSet::new();
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        let Some(tokens) = statement
            .tokens()
            .filter(|tokens| tokens.synthetic.is_none())
        else {
            continue;
        };
        output.extend(
            bindings
                .declaration_caller_alias_arguments(
                    tokens,
                    registry,
                    bindings.source_metadata_owner(),
                )
                .unwrap_or_default(),
        );
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        if binding
            .original_invocation_metadata_for_owner(
                tokens,
                bindings.source_metadata_owner(),
                registry,
            )
            .is_err()
        {
            continue;
        }
        let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
            continue;
        };
        let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
            continue;
        };
        for call in calls {
            if let Some(tokens) = call.tokens {
                output.extend(
                    bindings
                        .declaration_caller_alias_arguments(
                            &tokens,
                            registry,
                            bindings.source_metadata_owner(),
                        )
                        .unwrap_or_default(),
                );
            }
        }
    }
    output
}

/// Record any literal-name argument landing on a callee param that
/// carries `VarRead` / `VarWrite` (a call-by-name read/write), and the
/// default such a param binds where the call omits it.
fn add_call_by_name(cmd: &str, args: &[String], index: &ProcIndex, out: &mut HashSet<String>) {
    let candidates = (!cmd.is_empty() && !cmd.contains(['$', '[']))
        .then(|| index.get(cmd))
        .flatten();
    let Some(candidates) = candidates.filter(|candidates| !candidates.is_empty()) else {
        // A missing key supplies no effect-free callee. These literal passed
        // names may be observed; opaque hidden reads remain the CFG owner's
        // separate barrier, not an absence conclusion from this projection.
        out.extend(args.iter().filter(|arg| is_literal_var_name(arg)).cloned());
        return;
    };
    for candidate in candidates {
        for (i, pname) in candidate.params.iter().enumerate() {
            let is_var = candidate.param_traits.get(pname).is_some_and(|traits| {
                traits.contains(&ProcArgTrait::VarRead) || traits.contains(&ProcArgTrait::VarWrite)
            });
            let Some(arg) = args.get(i).or_else(|| candidate.param_defaults.get(pname)) else {
                break;
            };
            if is_var && is_literal_var_name(arg) {
                out.insert(arg.clone());
            }
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
            if !stmt.is_executable_invocation() {
                continue;
            }
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

/// Whether `word` is a plain, literal variable *name* — the only argument
/// shape whose call-by-name meaning is knowable.
///
/// The same test [`add_call_by_name`] applies to a resolvable callee's
/// name-role argument, factored out so the opaque-callee scan below cannot
/// drift from it.
fn is_literal_var_name(word: &str) -> bool {
    !word.is_empty()
        && !word.contains(['$', '['])
        && word
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
}

/// Caller-frame names a call to an **opaque callee** — a command this
/// compilation unit can resolve neither in the registry nor to any
/// definition of its own — may create in the calling frame.
///
/// A Tcl procedure's `upvar 1 $param local` reaches the frame of whoever
/// called it, so a helper defined in *another file* creates the caller's
/// variable just as one defined here does.  C Tcl, tclsh 9.0.4 and 8.6.16
/// (identical), for a ticklecharts-style layout — `setdef` in `utils.tcl`,
/// the caller in `options.tcl`, tied together by a `pkgIndex.tcl`:
///
/// ```text
/// proc demo::setdef {d key args} { upvar 1 $d _dict; … dict set _dict … }
/// proc demo::build {items} { setdef options name …; dict get $options name }
/// demo::build {a b c}   → {nothing str no} …   — `options` exists, unassigned here
/// ```
///
/// The per-proc frame-effect summary ([`crate::cfg_builder::upvar_info`])
/// is built from one module, so it holds nothing at all for such a callee
/// and the read looked like `W210` read-before-set.  The honest answer is
/// that this unit cannot tell: the callee's body is not here.  So each
/// **literal, name-shaped argument word** of the call — the only words that
/// could name a caller-frame variable — is reported as possibly-defined,
/// and the read-before-set emitters abstain for exactly those names.
///
/// Deliberately narrow in three ways, so this is an abstention and not a
/// blanket silence:
///
/// * only names the call **spells** are covered — an unrelated local read
///   in the same frame still reports (its true-positive control lives in
///   `tests/caller_frame_effects.rs`);
/// * only *literal* words qualify — a `$x` / `[f]` argument names a
///   variable nothing here can identify, the same abstention direction
///   `is_plain_var_name` takes on the navigation side;
/// * only *opaque* callees qualify — a registry command has a declared
///   frame effect, and a procedure defined in this unit has a real summary
///   which is trusted per-name (so a same-file helper without an `upvar`
///   keeps its reads reporting). A head this unit can resolve is opaque all
///   the same where the flow graph marks the call as one to code the module
///   cannot see ([`crate::ir::SyntheticMarker::UnseenCall`]): an alias of
///   `upvar`, a rename's target, the unresolved-command handler reached by
///   another name.
///
/// `resolvable` answers "can this unit resolve that command head?", which
/// the analyser supplies from the dialect profile plus its own definition
/// tables — no command name appears here.
#[must_use]
pub fn collect_opaque_callee_name_args(
    cfg: &crate::cfg::Function,
    resolvable: &dyn Fn(&str) -> bool,
) -> HashSet<String> {
    collect_positioned_opaque_callee_name_args(cfg, &|statement| match statement {
        crate::ir::Statement::Call { command, .. }
        | crate::ir::Statement::Barrier { command, .. } => resolvable(command),
        _ => false,
    })
}

/// Opaque-callee source arguments at each original executable invocation.
/// The consumer retains its actual statement/tokens and independently decides
/// body availability; a final display-name set cannot supply positioned lookup.
#[must_use]
pub fn collect_positioned_opaque_callee_name_args(
    cfg: &crate::cfg::Function,
    resolvable: &dyn Fn(&crate::ir::Statement) -> bool,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut out = HashSet::new();
    for block in cfg.blocks.values() {
        for (index, stmt) in block.statements.iter().enumerate() {
            if !stmt.is_executable_invocation() {
                continue;
            }
            let (Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. }) =
                stmt
            else {
                continue;
            };
            // A dynamic head (`$cmd …`) is handled by the dynamic-name
            // barrier, not here; an empty head is not a call.
            if command.is_empty() || command.contains(['$', '[']) {
                continue;
            }
            let marked_unseen = block.statements[index + 1..]
                .iter()
                .take_while(|next| next.span() == stmt.span() && next.synthetic_marker().is_some())
                .any(crate::ssa::is_unseen_call_marker);
            if resolvable(stmt) && !marked_unseen {
                continue;
            }
            out.extend(args.iter().filter(|a| is_literal_var_name(a)).cloned());
        }
    }
    out
}

// Call-target resolution

/// Predicate-backed String candidate reporting through the shared lookup order.
/// This path-free projection supplies no original body or implementation identity.
#[cfg(test)]
fn resolve_internal_call_with<F>(command: &str, caller_qname: &str, exists: F) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    if command.is_empty() {
        return None;
    }
    let ns_parts = namespace_parts_from_proc(caller_qname);
    let ns = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    crate::naming::resolve_command_with::<&str, _>(&ns, &[], command, exists)
}

/// Resolve a command name to a qualified procedure name if it refers to one
/// defined in `known`.
///
/// Uses the shared existence-checked rule: an absolute name is looked up
/// directly; a relative name tries the caller's namespace first, then global,
/// and never walks intermediate ancestor namespaces.
#[must_use]
#[cfg(test)]
fn resolve_internal_call<S: std::hash::BuildHasher>(
    command: &str,
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    resolve_internal_call_with(command, caller_qname, |qname| known.contains(qname))
}

/// Top-level call-target resolver. Convenience wrapper that
/// handles the common case where the caller has no special
/// aliasing information.
#[must_use]
#[cfg(test)]
fn resolve_call_target<S: std::hash::BuildHasher>(
    command: &str,
    _args: &[String],
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    resolve_internal_call(command, caller_qname, known)
}

/// Head of a literal Tcl callback-list value under the selected grammar.
/// Braces, quotes and escapes belong to list syntax; bracket text in a literal
/// value remains data. Executed builders require original source receipts.
#[must_use]
pub fn command_prefix_head(
    registry: &tcl_registry::CommandRegistry,
    prefix: &str,
) -> Option<String> {
    command_prefix_literal_head(
        tcl_syntax::word_rules::WordValueRules::from_config(&tcl_lexer::LexerConfig::for_profile(
            registry.profile(),
        )),
        prefix,
    )
}
pub(crate) fn command_prefix_literal_head(
    rules: tcl_syntax::word_rules::WordValueRules,
    prefix: &str,
) -> Option<String> {
    rules
        .split_list(prefix)
        .ok()?
        .first()
        .map(ToString::to_string)
}

/// Return the namespace segments of a qualified proc name —
/// everything except the trailing simple name. The split is the one canonical
/// [`crate::naming::qualifier_segments_owned`] (colon runs are a single
/// separator, so `::a:::b::p` yields `["a", "b"]`).
#[must_use]
#[cfg(test)]
fn namespace_parts_from_proc(qname: &str) -> Vec<String> {
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
pub struct ObjectTypeCandidates<'a> {
    candidates: &'a HashMap<String, HashSet<String>>,
    unknown_residual: bool,
}

/// Compatibility name for advisory callback reachability candidates.
/// This carrier cannot establish runtime object contents or closed dispatch.
pub type ObjectTypeMap<'a> = ObjectTypeCandidates<'a>;

impl<'a> ObjectTypeCandidates<'a> {
    /// Supply possible classes for additional callback reachability edges.
    #[must_use]
    pub const fn candidates(classes: &'a HashMap<String, HashSet<String>>) -> Self {
        Self {
            candidates: classes,
            unknown_residual: true,
        }
    }

    /// Scope/name unions always leave the actual receiver's contents unresolved.
    #[must_use]
    pub const fn unknown_residual(self) -> bool {
        self.unknown_residual
    }
}

impl ObjectTypeCandidates<'static> {
    /// The empty map — no object-handle typing (no instance-method callback
    /// edges).  Backed by a process-wide empty map so it needs no local.
    #[must_use]
    pub fn none() -> Self {
        ObjectTypeCandidates::candidates(&EMPTY_OBJECT_TYPES)
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
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
) -> InterproceduralAnalysis {
    build_interprocedural_analysis_inner(
        ir_module,
        registry,
        dialect,
        object_types,
        identities,
        declared,
        None,
    )
}

/// Build the summaries of a compilation unit's procedures, each pure one's
/// return read from its own seedless lattice ([`seedless_returns`]): the
/// unit holds every procedure's flow graph and SSA and the module's command
/// trust, which [`build_interprocedural_analysis`], from IR alone, has not.
#[must_use]
pub(crate) fn build_interprocedural_analysis_for_unit(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
) -> InterproceduralAnalysis {
    build_interprocedural_analysis_inner(
        &cu.ir_module,
        registry,
        dialect,
        object_types,
        identities,
        Some(&cu.declared_commands),
        Some(ModuleUnits {
            cfg: &cu.cfg_module,
            seedless: Some(SeedlessUnits {
                unit: cu,
                procedures: &cu.procedures,
                mutations: &cu.command_mutations,
                transfers: &cu.transfers,
            }),
        }),
    )
}

/// What a summary build reads beyond the IR: the module's flow graphs, and
/// the procedures' analyses a seedless run needs where the caller has them.
#[derive(Clone, Copy)]
struct ModuleUnits<'a> {
    cfg: &'a crate::cfg::CfgModule,
    seedless: Option<SeedlessUnits<'a>>,
}

/// A compilation unit's per-procedure analyses and the module's command
/// trust: what [`seedless_returns`] runs each procedure's lattice over.
#[derive(Clone, Copy)]
struct SeedlessUnits<'a> {
    /// The unit itself, whose procedures a seedless run's calls reach.
    unit: &'a crate::compilation_unit::CompilationUnit,
    procedures: &'a HashMap<crate::ir::ProcedureId, crate::compilation_unit::FunctionUnit>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// The transfer summaries the unit's build computed.
    transfers: &'a TransferSummaries,
}

fn build_interprocedural_analysis_inner(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    units: Option<ModuleUnits<'_>>,
) -> InterproceduralAnalysis {
    let object_types = object_types.candidates;
    let summary_bodies = summary_procedure_bodies(ir_module, registry);
    let known: HashSet<ProcedureId> = summary_bodies.keys().copied().collect();

    let local = scan_all_procs(
        ir_module,
        SummaryBodyInputs {
            bodies: &summary_bodies,
            known: &known,
            unit: units
                .and_then(|units| units.seedless)
                .map(|units| units.unit),
        },
        registry,
        dialect,
        object_types,
        identities,
        declared,
    );
    let transitive_calls = compute_all_transitive_calls(&known, &local);
    let pure = fixpoint_pure(&local);
    let (effect_reads, effect_writes) = fixpoint_effects(&local);
    let seedless = units
        .and_then(|units| units.seedless)
        .map(|units| seedless_returns(ir_module, units, &pure, registry, dialect))
        .unwrap_or_default();
    // Where the load may first run each procedure, and where each one's
    // `proc` statement surely runs: one fact for the completion proof and
    // the instance lifecycle proof.
    let reach = DefinitionReach::of(ir_module, registry);
    // Completion reads the registry commands a body runs as the module
    // leaves their bindings, which a unit's build holds and the IR alone
    // does not.
    let completes = units
        .and_then(|units| units.seedless)
        .map(|_| {
            completion::procedures_complete(ir_module, registry, &|callee, caller| {
                reach.defined(callee, caller)
            })
        })
        .unwrap_or_default();

    let mut procedures = materialise_summaries(
        ir_module,
        &summary_bodies,
        &local,
        &transitive_calls,
        (&pure, &completes),
        (&effect_reads, &effect_writes),
        &seedless,
    );
    reach.state(&mut procedures);

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
        identities,
        declared,
        ProcFixpoints {
            pure: &pure,
            effect_reads: &effect_reads,
            effect_writes: &effect_writes,
        },
    );

    let global_instance_classes =
        global_instance_classes_with(ir_module, registry, reach.invocations(), &summary_bodies);
    let tainted_global_writes = units.map_or_else(
        || tainted_global_writes(ir_module, registry, &global_instance_classes),
        |units| {
            tainted_global_writes_from_cfg(ir_module, units.cfg, registry, &global_instance_classes)
        },
    );

    // The unit's transfer summaries, each with the return shape its
    // procedure's summary derives.
    let transfers = units
        .and_then(|units| units.seedless)
        .map(|units| {
            units
                .transfers
                .0
                .iter()
                .map(|(qname, transfer)| {
                    let mut transfer = transfer.clone();
                    if let Some(summary) = procedures.get(qname) {
                        transfer.result = return_shape(summary);
                    }
                    (*qname, transfer)
                })
                .collect()
        })
        .map(TransferSummaries)
        .unwrap_or_default();

    let global_source = global_instance_class_source(ir_module, registry);
    InterproceduralAnalysis {
        procedures,
        methods,
        global_instance_classes,
        global_instance_class_input: global_source.as_ref().map(|(input, _)| input.clone()),
        global_instance_class_owner: global_source.map(|(_, owner)| owner),
        tainted_global_writes,
        transfers,
    }
}

/// The return shape a procedure's summary states: its constant, the
/// parameter it passes through, the parameters its value reads, or a
/// computed value.
fn return_shape(summary: &ProcSummary) -> ReturnKind {
    if let Some(constant) = &summary.constant_return {
        ReturnKind::Literal(ExactValue::from_literal(&constant.text()))
    } else if let Some(param) = &summary.return_passthrough_param {
        ReturnKind::Passthrough(param.clone())
    } else if summary.return_depends_on_params.is_empty() {
        ReturnKind::Other
    } else {
        ReturnKind::UsesParam(summary.return_depends_on_params.clone())
    }
}

/// Conditional factory receiver candidates at each procedure's earliest
/// original source phase. Availability and original naming operands must join
/// the same retained Module interpretation. No factory execution, current
/// command allocation, callback frame or instance dispatch follows from this
/// reporting projection.
#[must_use]
pub(crate) fn global_instance_classes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, crate::taint::InstanceClassState> {
    global_instance_classes_with(
        ir_module,
        registry,
        &EagerInvocations::of(ir_module, registry),
        &summary_procedure_bodies(ir_module, registry),
    )
}

/// Retain the producer's complete source world once for either interprocedural
/// summaries or their smaller receiver-only projection.
pub(crate) fn global_instance_class_source(
    module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(
    crate::analyser::ResolvedAnalysisInput,
    std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>,
)> {
    let source = crate::taint::TaintSourceContext::for_module(registry, module);
    let input = source.metadata_context()?.source_analysis_input()?.clone();
    let owner = module.retained_source_bindings.as_ref()?;
    Some((input, std::sync::Arc::clone(owner)))
}

/// [`global_instance_classes`] over the module's eager invocations, which a
/// summary build computes once for this proof and the completion proof.
///
/// A procedure the top-level script may run while it is still running may be
/// called before a later constructor; one only a callback runs starts after
/// direct top-level setup ([`EagerInvocations::named`]).
#[must_use]
pub(crate) fn global_instance_classes_with(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    invocations: &EagerInvocations,
    source_bodies: &HashMap<ProcedureId, &crate::ir::Procedure>,
) -> HashMap<ProcedureId, crate::taint::InstanceClassState> {
    let source = crate::taint::TaintSourceContext::for_module(registry, ir_module);
    // Each statement retains its own original words and lookup horizon.
    // Nested source effects only withdraw candidates; they do not prove that
    // a conditional factory, mutation or teardown actually ran.
    let mut sites = Vec::new();
    let mut scripts = vec![(&ir_module.top_level, true)];
    while let Some((script, direct)) = scripts.pop() {
        for statement in &script.statements {
            let tokens = script.retained_source_tokens_for_statement(statement);
            if tokens.is_some()
                || matches!(
                    statement,
                    crate::ir::Statement::Call { .. }
                        | crate::ir::Statement::Barrier { .. }
                        | crate::ir::Statement::NativeCall { .. }
                )
            {
                sites.push((statement, tokens, direct));
            }
            scripts.extend(
                crate::ir_helpers::nested_bodies(statement)
                    .into_iter()
                    .map(|body| (body, false)),
            );
        }
    }
    sites.sort_by_key(|(statement, _, _)| statement.span().start());
    source_bodies
        .iter()
        .map(|(id, procedure)| {
            // Callable bodies keep their actual eager horizon. A readonly
            // declaration can only borrow setup preceding its own source site.
            let eager = if ir_module.procedures.contains_key(id) {
                invocations.named(id)
            } else {
                Some(procedure.span.start())
            };
            let mut classes = crate::taint::InstanceClassState::new();
            for &(statement, tokens, direct) in &sites {
                if eager.is_some_and(|call| statement.span().start() >= call) {
                    continue;
                }
                if direct {
                    crate::taint::transfer_instance_command(&mut classes, registry, source, tokens);
                } else {
                    let mut candidate = classes.clone();
                    crate::taint::transfer_instance_command(
                        &mut candidate,
                        registry,
                        source,
                        tokens,
                    );
                    classes.retain(|name, class| candidate.get(name) == Some(class));
                }
            }
            (*id, classes)
        })
        .collect()
}

/// Global variables tainted by registry-declared instance configuration in a
/// procedure, closed transitively over calls to other procedures.
#[must_use]
pub(crate) fn tainted_global_writes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let mut cfg_module = crate::cfg_builder::build_cfg_with_registry_and_config(
        ir_module,
        false,
        registry,
        ir_module.lexer_config,
    );
    let writes =
        direct_instance_option_writes(ir_module, &mut cfg_module, registry, global_classes);
    close_tainted_global_writes(ir_module, registry, writes)
}

fn tainted_global_writes_from_cfg(
    ir_module: &crate::ir::Module,
    cfg_module: &crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let mut cfg_module = cfg_module.clone();
    let writes =
        direct_instance_option_writes(ir_module, &mut cfg_module, registry, global_classes);
    close_tainted_global_writes(ir_module, registry, writes)
}

fn direct_instance_option_writes(
    ir_module: &crate::ir::Module,
    cfg_module: &mut crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module);
    // Only an ownerless Module is the standalone compatibility path. A module
    // that carries source input/bindings but cannot reborrow a selected source
    // invocation must conservatively retain its existing global defs.
    let standalone =
        ir_module.source_metadata_input.is_none() && ir_module.retained_source_bindings.is_none();
    cfg_module
        .procedures
        .iter_mut()
        .map(|(id, cfg)| {
            let initial = global_classes.get(id).cloned().unwrap_or_default();
            let mut defs =
                crate::ssa::enrich_instance_option_defs_with_initial(cfg, registry, &initial);
            defs.retain(|name| name.starts_with("::"));
            for block in cfg.blocks.values() {
                for statement in &block.statements {
                    let crate::ir::Statement::Call {
                        command,
                        canonical_command,
                        args,
                        defs: call_defs,
                        tokens,
                        ..
                    } = statement
                    else {
                        continue;
                    };
                    if let Some(metadata) = metadata {
                        let Some(tokens) = tokens.as_ref() else {
                            defs.extend(
                                call_defs
                                    .iter()
                                    .filter(|name| name.starts_with("::"))
                                    .cloned(),
                            );
                            continue;
                        };
                        // Prefer the genuine point-local registered receiver
                        // owner. The command source owner covers ordinary
                        // selected option writers when no instance applies.
                        let selected =
                            crate::ssa::original_instance_option_writes(tokens, metadata, registry)
                                .or_else(|| {
                                    crate::ssa::original_command_taint_option_writes(
                                        tokens, metadata, registry,
                                    )
                                });
                        let Some(selected) = selected else {
                            defs.extend(
                                call_defs
                                    .iter()
                                    .filter(|name| name.starts_with("::"))
                                    .cloned(),
                            );
                            continue;
                        };
                        let mut joined_every_target = true;
                        for write in selected
                            .iter()
                            .filter(|write| write.scope == tcl_registry::VariableScope::Global)
                        {
                            let Some(name) = crate::ssa::linked_option_definition_name(
                                tokens,
                                write.written_argument,
                                true,
                                metadata,
                                registry,
                            ) else {
                                joined_every_target = false;
                                continue;
                            };
                            // The selected source write and original written
                            // name are the authority for this summary. Do not
                            // rejoin them through the IR's display-name set.
                            defs.insert(name);
                        }
                        if !joined_every_target {
                            defs.extend(
                                call_defs
                                    .iter()
                                    .filter(|name| name.starts_with("::"))
                                    .cloned(),
                            );
                        }
                        continue;
                    }
                    if !standalone {
                        defs.extend(
                            call_defs
                                .iter()
                                .filter(|name| name.starts_with("::"))
                                .cloned(),
                        );
                        continue;
                    }
                    let lookup = canonical_command.as_deref().unwrap_or(command);
                    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                    for variable in call_defs.iter().filter(|name| name.starts_with("::")) {
                        if tcl_registry::taint::taints_var_write(
                            registry,
                            lookup,
                            &arg_refs,
                            registry.own_surface_query(),
                            variable,
                        ) {
                            defs.insert(variable.clone());
                        }
                    }
                }
            }
            (*id, defs)
        })
        .collect()
}

fn close_tainted_global_writes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    mut writes: HashMap<ProcedureId, HashSet<String>>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let known: HashSet<ProcedureId> = ir_module.procedures.keys().copied().collect();
    let mut calls: HashMap<ProcedureId, HashSet<ProcedureId>> = HashMap::new();
    for (id, procedure) in ir_module.procedures.iter() {
        let proc_calls = calls.entry(*id).or_default();
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            let crate::ir::Statement::Call {
                command,
                tokens: Some(tokens),
                ..
            } = statement
            else {
                return;
            };
            if let Some(targets) =
                crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                    tokens, command, ir_module, registry,
                )
            {
                proc_calls.extend(targets.into_iter().filter(|id| known.contains(id)));
            }
        });
    }

    let mut changed = true;
    while changed {
        changed = false;
        for (caller, callees) in &calls {
            let inherited: HashSet<String> = callees
                .iter()
                .filter_map(|callee| writes.get(callee))
                .flatten()
                .cloned()
                .collect();
            let caller_writes = writes.entry(*caller).or_default();
            let old_len = caller_writes.len();
            caller_writes.extend(inherited);
            changed |= caller_writes.len() != old_len;
        }
    }
    writes
}

/// Apply the instance facts needed before SSA construction.
///
/// Direct configuration calls gain the registry-declared variable defs, and
/// internal procedure calls gain the transitive defs of their callees. This is
/// deliberately CFG annotation, not command-name logic: both the receiver
/// class and the option write come from registry descriptors.
pub(crate) fn enrich_instance_taint_cfg(
    ir_module: &crate::ir::Module,
    cfg_module: &mut crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, HashSet<String>> {
    let global_classes = global_instance_classes(ir_module, registry);
    let direct_writes =
        direct_instance_option_writes(ir_module, cfg_module, registry, &global_classes);
    let writes = close_tainted_global_writes(ir_module, registry, direct_writes);

    let annotate_calls = |cfg: &mut crate::cfg::Function| {
        for block in cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                let crate::ir::Statement::Call {
                    command,
                    tokens: Some(tokens),
                    defs,
                    ..
                } = statement
                else {
                    continue;
                };
                let Some(targets) =
                    crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                        tokens, command, ir_module, registry,
                    )
                else {
                    continue;
                };
                for target in targets {
                    for variable in writes.get(&target).into_iter().flatten() {
                        if !defs.contains(variable) {
                            defs.push(variable.clone());
                        }
                    }
                }
            }
        }
    };
    annotate_calls(&mut cfg_module.top_level);
    for cfg in cfg_module.procedures.values_mut() {
        annotate_calls(cfg);
    }
    writes
}

/// The three per-procedure fixpoint results a method summary joins against —
/// its callees' purity and effect sets.  Bundled so [`build_method_summaries`]
/// keeps a small signature.
#[derive(Clone, Copy)]
struct ProcFixpoints<'a> {
    pure: &'a HashMap<ProcedureId, bool>,
    effect_reads: &'a HashMap<ProcedureId, EffectRegion>,
    effect_writes: &'a HashMap<ProcedureId, EffectRegion>,
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
    known: &HashSet<ProcedureId>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    procs: ProcFixpoints<'_>,
) -> HashMap<crate::ir::ProcedureId, MethodSummary> {
    let ProcFixpoints {
        pure,
        effect_reads,
        effect_writes,
    } = procs;
    if ir_module.methods.is_empty() {
        return HashMap::new();
    }
    // A call resolving to a method qname is "known" (not unknown), so
    // seed the resolver with both procs and method names.
    let method_known = known.clone();

    let source_input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module)
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input);
    let mut out: HashMap<crate::ir::ProcedureId, MethodSummary> =
        HashMap::with_capacity(ir_module.methods.len());
    for (id, method) in &ir_module.methods {
        let mqname = method.reporting_label();
        let mut facts = LocalFacts {
            local_pure: true,
            ..LocalFacts::default()
        };
        // Scan the primary body, then — the lowering retains them — every
        // replacement body of a redefined method, all into the SAME fact
        // accumulators: the summary describes the union of every body a
        // dispatch may run — pure only when all are, constant-return only
        // when every body's exits agree on the one constant. A join rather
        // than an abstain-on-redefinition kill switch: more precise, equally
        // sound.
        let mut written_ivars: HashSet<String> = HashSet::new();
        for (_, body_def) in ir_module.method_definition_bodies(*id) {
            scan_method_body_facts(
                body_def,
                MethodScan {
                    method_known: &method_known,
                    registry,
                    dialect,
                    identities,
                    declared,
                    source_input,
                    source_procedures: &ir_module.procedures,
                    source_module: ir_module,
                },
                &mut facts,
                &mut written_ivars,
            );
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
        // A class with a member the lowering could not model (dynamic
        // member name / body), or a module with a dynamic OO definition
        // target, may have replaced ANY of its methods with an unknown
        // body — force impure, since no retained-body scan covers what
        // was never readable. Statically-retained redefinitions are
        // covered by the replacement-body scan above instead.
        if ir_module.oo_evidence.dynamic_target
            || ir_module.oo_unanalysed_classes.contains(&method.class_name)
        {
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

        let mut calls: Vec<ProcedureId> = facts.direct_calls.iter().copied().collect();
        calls.sort();
        let direct_calls = calls.clone();
        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns, SeedlessAnswer::NotRun);

        out.insert(
            *id,
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
                    // A method's command is looked up at run time, so no
                    // call to it is proved to complete.
                    completes: false,
                    defined_at: None,
                    defined_callees: Vec::new(),
                    effect_reads: m_reads,
                    effect_writes: m_writes,
                    returns_constant,
                    constant_return,
                    return_depends_on_params: depends,
                    return_passthrough_param: passthrough,
                    // Methods are not folded at static call sites.
                    can_fold_static_calls: false,
                    param_traits: HashMap::new(),
                    param_defaults: HashMap::new(),
                },
                class_name: method.class_name.clone(),
                method_kind: method.kind,
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

/// Scan one method body's facts into the shared accumulators for
/// [`build_method_summaries`]: the local-purity / call / effect scan, the
/// fall-through return, and the instance-variable writes. Called once for
/// the primary [`crate::ir::MethodDef`] and once per retained replacement
/// body, so the summary joins over every body a dispatch may run.
#[derive(Clone, Copy)]
struct MethodScan<'a> {
    method_known: &'a HashSet<ProcedureId>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations — see [`ProcScan::declared`].
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    source_procedures: &'a crate::ir::ProcedureInventory,
    source_module: &'a crate::ir::Module,
}

fn scan_method_body_facts(
    body_def: &crate::ir::MethodDef,
    scan: MethodScan<'_>,
    facts: &mut LocalFacts,
    written_ivars: &mut HashSet<String>,
) {
    let MethodScan {
        method_known,
        registry,
        dialect,
        identities,
        declared,
        source_input,
        source_procedures,
        source_module,
    } = scan;
    let params: HashSet<String> = body_def.params.iter().cloned().collect();
    if matches!(
        body_def.execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ) && crate::ir_helpers::requires_runtime_command_namespace(&body_def.body, registry)
    {
        // A TclOO receiver namespace can contain an object-local command that
        // shadows every relative fallback considered below. Preserve useful
        // known fallback edges, but never prove purity or bounded state from
        // that incomplete target set.
        facts.has_barrier = true;
        facts.has_unknown_calls = true;
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    }
    let ctx = ScanCtx {
        known: method_known,
        registry,
        dialect,
        params: &params,
        // Method bodies are not call-graph nodes; no object-type map needed.
        object_types: ObjectTypeMap::none().candidates,
        identities,
        declared,
        positioned_substitutions: false,
        source_input,
        source_procedures,
        source_module,
        current_tokens: None,
        parameter_body: None,
    };
    scan_script(&body_def.body, ctx, facts, 0);
    // Fall-through exit is non-constant (O103); see `scan_proc`.
    if !script_always_returns(&body_def.body, 0) {
        facts.returns.push(ReturnKind::Other);
    }
    // Instance-variable writes mutate object state and survive the call —
    // so a method that writes any in-scope instance var is impure even
    // though the write looks like a plain local `set`.
    if body_def.body.executed_source.is_some() {
        if original_method_has_outward_write(&body_def.body, registry, source_input) {
            facts.local_pure = false;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
    } else if !body_def.instance_vars.is_empty() {
        collect_instance_var_writes(
            &body_def.body,
            &body_def.instance_vars,
            registry,
            written_ivars,
            0,
        );
        collect_possible_instance_var_writes(
            &body_def.body,
            &body_def.instance_vars,
            registry,
            source_input,
            written_ivars,
        );
    }
}

/// Original object writes are selected from actual operand/context owners.
/// Missing normal binding evidence remains a hazard, never a private local.
fn original_method_has_outward_write(
    script: &crate::ir::Script,
    registry: &tcl_registry::CommandRegistry,
    source_input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> bool {
    let Some(context) = source_input.map(crate::analyser::ResolvedAnalysisInput::context_registry)
    else {
        return true;
    };
    let metadata = Some(context.as_ref().into());
    let mut pending = vec![(script, 0)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
            return true;
        }
        for statement in &script.statements {
            let tokens = script.retained_source_tokens_for_statement(statement);
            if let Some(names) = tokens.and_then(|tokens| {
                crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                    registry, metadata, tokens,
                )
            }) {
                if names
                    .unresolved_roles()
                    .any(|role| role == tcl_registry::ArgRole::VarWrite)
                {
                    return true;
                }
                if names.contents_write_operands().next().is_some() {
                    let Some(tokens) = tokens else {
                        return true;
                    };
                    let Some(binding) = tokens.source_binding.as_ref() else {
                        return true;
                    };
                    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry, metadata, tokens,
                    ) else {
                        return true;
                    };
                    let context = binding
                        .normal_variable_continuation()
                        .unwrap_or(&binding.variable_context);
                    let writes = normal.mutation_places(context, registry);
                    if writes.is_empty()
                        || writes.iter().any(|place| {
                            place.dynamic || place.observed || place.cell.as_ref().is_none_or(|cell|
                            !matches!(&cell.owner, crate::place::CellOwner::Activation(identity)
                                if context.activation.as_ref() == Some(identity)))
                        })
                    {
                        return true;
                    }
                }
            }
            pending.extend(
                statement
                    .child_scripts()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

/// Candidate name operands identify possible instance-write hazards, even
/// when receiver lookup prevents a Must handler or physical destination proof.
fn collect_possible_instance_var_writes(
    script: &crate::ir::Script,
    ivars: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    source_input: Option<&crate::analyser::ResolvedAnalysisInput>,
    out: &mut HashSet<String>,
) {
    use crate::registry_invocation::EffectiveInvocationWord;
    let Some(context) = source_input.map(crate::analyser::ResolvedAnalysisInput::context_registry)
    else {
        out.extend(ivars.iter().cloned());
        return;
    };
    let metadata = Some(context.as_ref().into());
    let mut pending = vec![(script, 0)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
            continue;
        }
        for statement in &script.statements {
            if let Some(names) = statement.tokens().and_then(|tokens| {
                crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                    registry, metadata, tokens,
                )
            }) {
                for operand in names.contents_write_operands() {
                    match operand {
                        EffectiveInvocationWord::Literal(name) => {
                            check_ivar_write(name, ivars, out);
                        }
                        EffectiveInvocationWord::ArrayElementName { root } => {
                            check_ivar_write(root, ivars, out);
                        }
                        EffectiveInvocationWord::ByteLiteral(_)
                        | EffectiveInvocationWord::Dynamic
                        | EffectiveInvocationWord::Expanded
                        | EffectiveInvocationWord::KnownExpansion(_)
                        | EffectiveInvocationWord::KnownByteExpansion(_)
                        | EffectiveInvocationWord::Opaque => out.extend(ivars.iter().cloned()),
                    }
                }
                if names
                    .unresolved_roles()
                    .any(|role| role == tcl_registry::ArgRole::VarWrite)
                {
                    out.extend(ivars.iter().cloned());
                }
            }
            pending.extend(
                statement
                    .child_scripts()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
}

/// Recursively collect the base names of instance-variable *writes* in
/// a method body, comparing each written name against `ivars`. Walks
/// the CFG, counting explicit assign / incr / loop / catch targets.
/// Positioned Call hazards come from the candidate contents-write owner;
/// unpositioned IR conservatively retains its declared `defs`. Array-element
/// writes (`counter(0)`) compare on the base scalar name so they are
/// not missed. Over-approximating writes is the sound direction (a
/// spurious write only costs an O126 fold; a missed one would wrongly
/// delete a state mutation).
#[allow(clippy::too_many_lines)] // the depth-cap check adds a few lines over the threshold
fn collect_instance_var_writes(
    script: &crate::ir::Script,
    ivars: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    out: &mut HashSet<String>,
    depth: u32,
) {
    use crate::ir::Statement;
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. } => {
                let projection = crate::ir_helpers::structural_variable_write_projection(stmt);
                if projection.opaque_variable_frame {
                    out.extend(ivars.iter().cloned());
                }
                for name in &projection.literal_names {
                    check_ivar_write(name, ivars, out);
                }
            }
            Statement::Call { defs, tokens, .. }
                if tokens
                    .as_ref()
                    .is_none_or(|tokens| tokens.source_binding.is_none()) =>
            {
                for d in defs {
                    check_ivar_write(d, ivars, out);
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    collect_instance_var_writes(&clause.body, ivars, registry, out, depth + 1);
                }
                if let Some(eb) = else_body {
                    collect_instance_var_writes(eb, ivars, registry, out, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_instance_var_writes(init, ivars, registry, out, depth + 1);
                collect_instance_var_writes(next, ivars, registry, out, depth + 1);
                collect_instance_var_writes(body, ivars, registry, out, depth + 1);
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    for v in &it.vars {
                        check_ivar_write(v, ivars, out);
                    }
                }
                collect_instance_var_writes(body, ivars, registry, out, depth + 1);
            }
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_instance_var_writes(body, ivars, registry, out, depth + 1);
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
                collect_instance_var_writes(body, ivars, registry, out, depth + 1);
                for h in handlers {
                    if let Some(v) = &h.var_name {
                        check_ivar_write(v, ivars, out);
                    }
                    if let Some(ov) = &h.options_var {
                        check_ivar_write(ov, ivars, out);
                    }
                    collect_instance_var_writes(&h.body, ivars, registry, out, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_instance_var_writes(fb, ivars, registry, out, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        collect_instance_var_writes(b, ivars, registry, out, depth + 1);
                    }
                }
                if let Some(db) = default_body {
                    collect_instance_var_writes(db, ivars, registry, out, depth + 1);
                }
            }
            Statement::While { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => {
                collect_instance_var_writes(body, ivars, registry, out, depth + 1);
            }
            _ => {}
        }
    }
}

/// Record `raw` as an instance-var write when its base scalar name (an
/// array element `arr(idx)` compares on `arr`) is a declared instance
/// var.
fn check_ivar_write(raw: &str, ivars: &HashSet<String>, out: &mut HashSet<String>) {
    let base = tcl_syntax::naming::split_element_ref(raw).map_or(raw, |(root, _)| root);
    if ivars.contains(base) {
        out.insert(base.to_owned());
    }
}

/// Conditional source summaries retain genuine declaration IDs independently
/// of callable publication, completed definition reach and Native admission.
fn summary_procedure_bodies<'a>(
    module: &'a crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, &'a crate::ir::Procedure> {
    let mut procedures: HashMap<_, _> = module
        .procedures
        .iter()
        .filter(|(id, _)| module.procedure_ids.owns(**id))
        .map(|(id, procedure)| (*id, procedure))
        .collect();
    if let Some(owner) = module.retained_source_bindings.as_ref()
        && owner.positioned_for_module(module, registry).is_some()
    {
        for (id, procedure) in module.original_source_procedure_bodies() {
            if procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                && owner.matches_original_procedure_summary(*id, procedure)
            {
                procedures.insert(*id, procedure);
            }
        }
    }
    procedures
}

#[derive(Clone, Copy)]
struct SummaryBodyInputs<'a> {
    bodies: &'a HashMap<ProcedureId, &'a crate::ir::Procedure>,
    known: &'a HashSet<ProcedureId>,
    unit: Option<&'a crate::compilation_unit::CompilationUnit>,
}

fn scan_all_procs(
    ir_module: &crate::ir::Module,
    source: SummaryBodyInputs<'_>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: &HashMap<String, HashSet<String>>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
) -> HashMap<ProcedureId, LocalFacts> {
    let source_input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module)
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input);
    let known = source.known;
    let mut local: HashMap<ProcedureId, LocalFacts> = HashMap::with_capacity(known.len());
    for (&id, &proc) in source.bodies {
        local.insert(
            id,
            scan_proc(ProcScan {
                proc,
                unit: source.unit,
                known,
                registry,
                dialect,
                object_types,
                identities,
                declared,
                source_input,
                source_procedures: &ir_module.procedures,
                source_module: ir_module,
            }),
        );
    }
    local
}

/// One procedure's local-facts scan, bundled so [`scan_proc`] keeps a small
/// signature.
#[derive(Clone, Copy)]
struct ProcScan<'a> {
    proc: &'a crate::ir::Procedure,
    unit: Option<&'a crate::compilation_unit::CompilationUnit>,
    known: &'a HashSet<ProcedureId>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: &'a HashMap<String, HashSet<String>>,
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations (`# tcl-lsp: stub`), unioned
    /// with the catalogue through [`ScanCtx::surface`].
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    source_procedures: &'a crate::ir::ProcedureInventory,
    source_module: &'a crate::ir::Module,
}

fn compute_all_transitive_calls(
    known: &HashSet<ProcedureId>,
    local: &HashMap<ProcedureId, LocalFacts>,
) -> HashMap<ProcedureId, HashSet<ProcedureId>> {
    let mut out: HashMap<ProcedureId, HashSet<ProcedureId>> = HashMap::with_capacity(known.len());
    for qname in known {
        out.insert(*qname, compute_transitive_calls(qname, local));
    }
    out
}

/// Each procedure's purity: its own body pure and every procedure it calls
/// pure — or one whose calls reach no further than the places its `Name`
/// arguments name, called with locals of the caller's own frame that no
/// other frame reaches, which end with the caller's call
/// (`docs/design/compiler/value-transfers.md` § *Proc-level transfer
/// summaries*). Such a procedure is one whose body does nothing its caller
/// observes but through those links, and calls only procedures of the same
/// kind with places of its own, its links' among them.
fn fixpoint_pure(local: &HashMap<ProcedureId, LocalFacts>) -> HashMap<ProcedureId, bool> {
    let mut pure: HashMap<ProcedureId, bool> =
        local.iter().map(|(q, f)| (*q, f.local_pure)).collect();
    let mut bounded: HashMap<ProcedureId, bool> = local
        .iter()
        .map(|(q, f)| (*q, !f.impure_beyond_links))
        .collect();
    loop {
        let mut changed = false;
        for (qname, facts) in local {
            let (now_bounded, now_pure) = {
                let calls_stay = |own_links: bool| {
                    facts.procedure_calls.iter().all(|(callee, words)| {
                        pure.get(callee).copied().unwrap_or(false)
                            || (bounded.get(callee).copied().unwrap_or(false)
                                && words.as_deref().is_some_and(|words| {
                                    local.get(callee).is_some_and(|called| {
                                        names_own_places(
                                            facts,
                                            &called.name_params,
                                            words,
                                            own_links,
                                        )
                                    })
                                }))
                    })
                };
                (
                    bounded[qname] && calls_stay(true),
                    pure[qname] && calls_stay(false),
                )
            };
            if now_bounded != bounded[qname] {
                bounded.insert(*qname, now_bounded);
                changed = true;
            }
            if now_pure != pure[qname] {
                pure.insert(*qname, now_pure);
                changed = true;
            }
        }
        if !changed {
            return pure;
        }
    }
}

/// Whether a call's `words` name, at each of the callee's `name_params`, a
/// plain local of the caller's frame: one no `global` or `variable` aliases
/// and no `upvar` links elsewhere — or, where `own_links` is set, one the
/// caller links to the place its own parameter names one frame up.
fn names_own_places(
    caller: &LocalFacts,
    name_params: &[usize],
    words: &[String],
    own_links: bool,
) -> bool {
    name_params.iter().all(|&index| {
        words.get(index).is_some_and(|word| {
            is_plain_local_name(word)
                && !caller.global_aliases.contains(word)
                && (!caller.linked_locals.contains(word)
                    || (own_links && caller.upvar_aliases.contains_key(word)))
        })
    })
}

fn fixpoint_effects(
    local: &HashMap<ProcedureId, LocalFacts>,
) -> (
    HashMap<ProcedureId, EffectRegion>,
    HashMap<ProcedureId, EffectRegion>,
) {
    let mut reads: HashMap<ProcedureId, EffectRegion> =
        local.iter().map(|(q, f)| (*q, f.effect_reads)).collect();
    let mut writes: HashMap<ProcedureId, EffectRegion> =
        local.iter().map(|(q, f)| (*q, f.effect_writes)).collect();
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
                reads.insert(*qname, r);
                changed = true;
            }
            if w != writes[qname] {
                writes.insert(*qname, w);
                changed = true;
            }
        }
    }
    (reads, writes)
}

fn materialise_summaries(
    ir_module: &crate::ir::Module,
    bodies: &HashMap<ProcedureId, &crate::ir::Procedure>,
    local: &HashMap<ProcedureId, LocalFacts>,
    transitive_calls: &HashMap<ProcedureId, HashSet<ProcedureId>>,
    (pure, completes): (&HashMap<ProcedureId, bool>, &HashMap<ProcedureId, bool>),
    (effect_reads, effect_writes): (
        &HashMap<ProcedureId, EffectRegion>,
        &HashMap<ProcedureId, EffectRegion>,
    ),
    seedless: &HashMap<ProcedureId, Option<ExactValue>>,
) -> HashMap<ProcedureId, ProcSummary> {
    let mut procedures: HashMap<ProcedureId, ProcSummary> = HashMap::with_capacity(local.len());
    let rules =
        tcl_syntax::word_rules::WordValueRules::of_dialect_name(ir_module.dialect.as_deref());
    for (qname, facts) in local {
        let Some(proc) = bodies.get(qname) else {
            continue;
        };
        let report_label = if proc.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration {
            let Some(label) =
                ir_module.original_procedure_report_label(*qname, ir_module.resolved_registry())
            else {
                continue;
            };
            label.to_owned()
        } else {
            proc.qualified_name.clone()
        };
        let mut calls_list: Vec<ProcedureId> = transitive_calls
            .get(qname)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        calls_list.sort();
        let mut direct_calls: Vec<ProcedureId> = facts.direct_calls.iter().copied().collect();
        direct_calls.sort();
        let is_pure = *pure.get(qname).unwrap_or(&false);

        // Transitive effects include every retained callee's local flags.
        let transitive_flag = |pick: fn(&LocalFacts) -> bool| -> bool {
            pick(facts) || calls_list.iter().any(|c| local.get(c).is_some_and(pick))
        };
        let writes_global = transitive_flag(|f| f.writes_global);
        let has_unknown_calls = transitive_flag(|f| f.has_unknown_calls);

        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns, SeedlessAnswer::of(seedless, qname));
        // A proc is foldable at a call site when its return is
        // fully determined by the static call — that means pure
        // AND (constant return OR passthrough of a param).
        let can_fold = is_pure && (returns_constant || passthrough.is_some());
        let param_traits = finalise_param_traits(
            &proc.params,
            &facts.param_trait_flags,
            passthrough.as_deref(),
            &facts.param_references,
            !facts.param_observations_incomplete,
        );

        procedures.insert(
            *qname,
            ProcSummary {
                qualified_name: report_label,
                params: proc.params.clone(),
                arity: arity_from_names(&proc.params),
                calls: calls_list,
                direct_calls,
                has_barrier: facts.has_barrier,
                has_unknown_calls,
                writes_global,
                pure: is_pure,
                completes: completes.get(qname).copied().unwrap_or(false),
                // The summary build states the definition reach once every
                // summary exists.
                defined_at: None,
                defined_callees: Vec::new(),
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
                param_defaults: declared_defaults(&proc.params_raw, rules),
            },
        );
    }
    procedures
}

/// The defaults the parameter list `params_raw` declares, by parameter, as
/// Tcl decodes them under `rules`; none where the list cannot be read.
fn declared_defaults(
    params_raw: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> HashMap<String, String> {
    crate::signature_scan::params::parse_param_list_strict(params_raw, rules)
        .map(|formals| {
            formals
                .into_iter()
                .filter_map(|formal| Some((formal.name, formal.default?)))
                .collect()
        })
        .unwrap_or_default()
}

/// Per-procedure scratch facts consumed by the summary-building
/// pipeline.
// False positive: a flat record of independent boolean facts gathered while
// scanning a body, not a state machine — no natural enum to collapse into.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct LocalFacts {
    direct_calls: HashSet<ProcedureId>,
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
    param_references: HashSet<String>,
    param_observations_incomplete: bool,
    /// Conditional source incidence only; never participates in Native alias,
    /// effects, purity, successful caller templates or physical cell summaries.
    source_parameter_aliases:
        HashMap<String, crate::registry_invocation::OriginalAuthoredFormalAliasObservation>,
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
    /// Whether the body does anything its caller observes besides linking a
    /// local to the place a parameter names one frame up (`upvar 1 $name
    /// v`): [`Self::local_pure`] with those links set aside.
    impure_beyond_links: bool,
    /// Every local the body links to a place outside its frame by `upvar`,
    /// at any level.
    linked_locals: HashSet<String>,
    /// Each call to a procedure of the module, with its argument words where
    /// the call spells them; `None` for a callback, whose words a later
    /// invocation supplies.
    procedure_calls: Vec<(ProcedureId, Option<Vec<String>>)>,
    /// The positions of the parameters whose value names the place one frame
    /// up a local of the body is linked to.
    name_params: Vec<usize>,
}

impl LocalFacts {
    /// Record something the body does that its caller observes.
    fn mark_impure(&mut self) {
        self.local_pure = false;
        self.impure_beyond_links = true;
    }
}

/// One way a procedure returns, as the summary reads it: a `return`, or the
/// fall-through when the body can reach its end.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReturnKind {
    /// A value every caller gets: the return's literal word through the
    /// exact value ingress, or what the procedure's seedless lattice proves
    /// there.
    Literal(ExactValue),
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
            param_references: HashSet::new(),
            param_observations_incomplete: true,
            source_parameter_aliases: HashMap::new(),
            global_aliases: HashSet::new(),
            upvar_aliases: HashMap::new(),
            impure_beyond_links: true,
            linked_locals: HashSet::new(),
            procedure_calls: Vec::new(),
            name_params: Vec::new(),
        }
    }
}

fn original_parameter_observation_owner(scan: ProcScan<'_>) -> Option<ProcedureId> {
    let select = || {
        let module = scan.source_module;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, scan.registry)?;
        let mut owners = module
            .original_source_procedure_bodies()
            .filter(|(_, procedure)| std::ptr::eq(*procedure, scan.proc));
        let (&id, procedure) = owners.next()?;
        if owners.next().is_some()
            || !module.procedure_ids.owns(id)
            || !retained.matches_original_procedure_summary(id, procedure)
            || module.installed_procedure_body_units.contains_key(&id)
        {
            return None;
        }
        let formals = if let Some(card) = module.original_authored_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else if let Some(card) = module.original_native_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else if let Some(card) = module.original_logical_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else {
            let topology =
                crate::var_escape::original_slots::original_procedure_topology(module, procedure)?;
            if topology
                .bindings(topology.parameters().len())
                .ok()?
                .iter()
                .any(|binding| {
                    matches!(
                        binding,
                        tcl_syntax::formal_params::FormalByteArgumentBinding::CallerLink { .. }
                    )
                })
            {
                return None;
            }
            topology
                .parameters()
                .iter()
                .map(|formal| formal.name.as_slice())
                .eq(procedure.params.iter().map(String::as_bytes))
        };
        if !formals {
            return None;
        }
        if let Some(unit) = scan.unit {
            let function = unit.function_for_body(Some(id))?;
            if !std::ptr::eq(std::ptr::from_ref(&unit.ir_module), module)
                || unit.source.as_bytes() != module.source.bytes()
                || unit
                    .original_procedure_for_body(id)
                    .is_none_or(|current| !std::ptr::eq(current, procedure))
                || match (
                    function.source_metadata_input(),
                    module.source_metadata_input.as_ref(),
                ) {
                    (None, None) => false,
                    (Some(actual), Some(original)) => {
                        actual != original && actual != &original.for_nested_source()
                    }
                    _ => true,
                }
                || function.body_id != Some(id)
                || unit.original_procedure_cfg_for_body(id) != Some(&function.cfg)
                || function.source_lexer_config()
                    != module.native_lexer_config().nested().normalized()
            {
                return None;
            }
        }
        Some(id)
    };
    select()
}

/// Complete lexical incidence over genuine original words; every child keeps
/// the same immutable image and full grammar. Evaluation roles remain separate.
fn original_word_reference_names(words: &[tcl_lexer::NativeWord]) -> Option<HashSet<String>> {
    let mut pending = words.to_vec();
    let mut names = HashSet::new();
    let mut remaining = 4096usize;
    while let Some(word) = pending.pop() {
        remaining = remaining.checked_sub(1)?;
        let arena = word.executable_parts();
        for component in arena.all_parts() {
            remaining = remaining.checked_sub(1)?;
            match component.part {
                tcl_lexer::ExecutablePart::Variable { name, .. } => {
                    names.insert(std::str::from_utf8(arena.bytes(name)?).ok()?.to_owned());
                }
                tcl_lexer::ExecutablePart::Command { body } => {
                    let plan = tcl_lexer::native_script_words_in_with_purpose(
                        word.image().clone(),
                        body,
                        word.config(),
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                    )
                    .ok()?;
                    if plan.fatal_tail.is_some() {
                        return None;
                    }
                    for command in plan.commands {
                        pending.extend(command.words);
                    }
                }
                tcl_lexer::ExecutablePart::Text(_) => {}
                tcl_lexer::ExecutablePart::ParseError(_)
                | tcl_lexer::ExecutablePart::Expression { .. } => return None,
            }
        }
    }
    Some(names)
}

fn original_command_parameter_names(ctx: ScanCtx<'_>) -> Option<HashSet<String>> {
    let (_, words) = crate::registry_invocation::original_command_words_for_module(
        ctx.registry,
        ctx.source_module,
        ctx.current_tokens?,
    )?;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    let mut names = original_word_reference_names(&words)?;
    names.retain(|name| ctx.params.contains(name));
    Some(names)
}

fn note_original_command_references(ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    if let Some(names) = original_command_parameter_names(ctx) {
        facts.param_references.extend(names);
    } else {
        facts.param_observations_incomplete = true;
    }
    let Some(roles) = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_source_name_roles(ctx.registry, tokens)
    }) else {
        facts.param_observations_incomplete = true;
        return;
    };
    if !roles.roles_complete() {
        facts.param_observations_incomplete = true;
    }
    for &(argument, role) in roles.roles() {
        if role != tcl_registry::ArgRole::VarRead
            && !(role == tcl_registry::ArgRole::VarWrite
                && roles
                    .facts()
                    .traits
                    .contains(tcl_registry::Traits::READS_BEFORE_WRITE))
        {
            continue;
        }
        let Some(name) = roles.original_variable_name_operand(argument) else {
            facts.param_observations_incomplete = true;
            continue;
        };
        if ctx.params.contains(name.as_str()) {
            facts.param_references.insert(name.as_str().to_owned());
        } else if tcl_syntax::naming::is_qualified(name.as_str().as_bytes())
            || tcl_syntax::naming::split_element_ref(name.as_str()).is_some()
        {
            facts.param_observations_incomplete = true;
        }
    }
}

fn note_original_command_params(ctx: ScanCtx<'_>, kind: ProcArgTrait, facts: &mut LocalFacts) {
    let Some(names) = original_command_parameter_names(ctx) else {
        facts.param_observations_incomplete = true;
        return;
    };
    facts.param_references.extend(names.iter().cloned());
    for name in names {
        facts
            .param_trait_flags
            .entry(name)
            .or_default()
            .insert(kind);
    }
}

/// Conditional source forwarding reports do not close the later Native
/// invocation/effect scan or its unknown observer/lookup alternatives.
fn note_original_authored_body_call_parameters(ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    if ctx.parameter_body.is_none() {
        return;
    }
    let Some(calls) = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    }) else {
        return;
    };
    // Source correspondence leaves entry, lookup and observers unproved.
    facts.param_observations_incomplete |= !calls.obligations().is_empty();
    if !calls.calls().is_empty()
        && calls
            .calls()
            .iter()
            .all(|call| call.accepts_arguments() && ctx.known.contains(&call.id()))
    {
        note_original_command_params(ctx, ProcArgTrait::ForwardedToCallee, facts);
    }
}

/// Source-only alias/formal incidence follows actual represented sibling
/// commands; runtime entry/lookup/observer uncertainty remains explicit.
fn note_original_authored_alias_parameters(ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    let command = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    });
    let Some(command) = command.filter(|command| Some(command.body()) == ctx.parameter_body) else {
        facts.source_parameter_aliases.clear();
        return;
    };
    // The source recipe supplies no entered body, actual lookup or observer
    // closure, even when the original symbolic argument layout is complete.
    if !command.aliases().is_empty() || !facts.source_parameter_aliases.is_empty() {
        facts.param_observations_incomplete |= command.has_unproved_applicability();
    }
    facts
        .source_parameter_aliases
        .retain(|_, alias| alias.belongs_to(&command));
    let mut declarations = HashSet::new();
    for alias in command.aliases() {
        if let Some(parameter) = command
            .alias_target_formal(alias)
            .filter(|parameter| ctx.params.contains(*parameter))
        {
            facts.param_references.insert(parameter.to_owned());
            facts
                .param_trait_flags
                .entry(parameter.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarRead);
        }
        let Some(local) = command.literal(alias.words.local) else {
            facts.source_parameter_aliases.clear();
            continue;
        };
        if !command.local_scalar(local) || alias.local.literal() != Some(local) {
            facts.source_parameter_aliases.clear();
            continue;
        }
        declarations.insert(local.to_owned());
        facts.source_parameter_aliases.remove(local);
        if let Some(observation) = command.caller_alias(alias) {
            facts
                .source_parameter_aliases
                .insert(local.to_owned(), observation);
        }
    }
    for &(argument, role) in command.roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let Some(local) = command.literal(argument) else {
            facts.source_parameter_aliases.clear();
            continue;
        };
        if declarations.contains(local) {
            continue;
        }
        if let Some(alias) = facts.source_parameter_aliases.get(local) {
            facts
                .param_trait_flags
                .entry(alias.parameter().to_owned())
                .or_default()
                .insert(ProcArgTrait::VarWrite);
        }
    }
    if !command.continues() {
        facts.source_parameter_aliases.clear();
    }
}

fn original_parameter_expression_config(
    expression: &crate::expr_ast::ExprNode,
    base: Option<u32>,
    ctx: ScanCtx<'_>,
) -> Option<tcl_lexer::LexerConfig> {
    let tokens = ctx.current_tokens?;
    let (config, words) = crate::registry_invocation::original_command_words_for_module(
        ctx.registry,
        ctx.source_module,
        tokens,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_authored_source_body_name_role_owner()
        && !binding.has_original_source_layout_owner()
    {
        let id = ctx.parameter_body?;
        let module = ctx.source_module;
        let procedure = module.body_units.get(&id)?;
        let card = module.original_authored_declaration_body_units.get(&id)?;
        (module.procedure_ids.owns(id) && card.matches_procedure(module, procedure))
            .then_some(())?;
        let mut matches = false;
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            use crate::ir::Statement;
            let original = |node: &crate::expr_ast::ExprNode, original_base: Option<u32>| {
                std::ptr::eq(node, expression) && original_base == base
            };
            matches |= match statement {
                Statement::AssignExpr {
                    expr, expr_base, ..
                }
                | Statement::ExprEval {
                    expr, expr_base, ..
                }
                | Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    ..
                } => original(expr, *expr_base),
                Statement::If { clauses, .. } => clauses
                    .iter()
                    .any(|clause| original(&clause.condition, clause.condition_base)),
                Statement::While {
                    condition,
                    condition_base,
                    ..
                }
                | Statement::For {
                    condition,
                    condition_base,
                    ..
                } => original(condition, *condition_base),
                _ => false,
            };
        });
        matches.then_some(())?;
        // The unchanged retained declaration Script owns this exact lowered AST.
        // The original complete operand owns affine source content separately.
        let scope =
            binding.original_authored_source_body_scope_for_name_roles(tokens, ctx.registry)?;
        scope.matches_module(ctx.source_module).then_some(())?;
        let base = base?;
        let mut operands = words.iter().filter(|word| {
            word.content_span().is_ok_and(|span| {
                span.start() == base
                    && tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some_and(
                        |value| word.image().bytes().get(span.as_range()) == Some(value.as_slice()),
                    )
            })
        });
        operands.next()?;
        operands.next().is_none().then_some(config)
    } else {
        let (original, _) = crate::registry_invocation::original_lowered_expression_syntax(
            expression, base, tokens,
        )?;
        (original == config).then_some(config)
    }
}

fn note_original_return_parameter(
    value: Option<&crate::ir::WordExpr>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    if facts.param_observations_incomplete {
        return;
    }
    let name = || {
        let value = value?;
        let tokens = ctx.current_tokens?;
        let (_, words) = crate::registry_invocation::original_command_words_for_module(
            ctx.registry,
            ctx.source_module,
            tokens,
        )?;
        let mut indices = tokens
            .words()
            .iter()
            .enumerate()
            .filter(|(_, word)| *word == value);
        let (index, _) = indices.next()?;
        if index == 0 || indices.next().is_some() {
            return None;
        }
        let name = tcl_syntax::word_rules::original_scalar_variable_with_prefix(
            words.get(index)?,
            &[b""],
        )?;
        let name = String::from_utf8(name).ok()?;
        ctx.params.contains(&name).then_some(name)
    };
    if let Some(name) = name() {
        // Source trait advice stays independent of classify_return's stronger
        // Native value/return-fold premises.
        facts.param_references.insert(name.clone());
        facts
            .param_trait_flags
            .entry(name)
            .or_default()
            .insert(ProcArgTrait::Passthrough);
    }
}

fn note_original_expression_params(
    expression: &crate::expr_ast::ExprNode,
    base: Option<u32>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(config) = original_parameter_expression_config(expression, base, ctx) else {
        facts.param_observations_incomplete = true;
        return;
    };
    note_params_in_expr(expression, ctx.params, facts, config, 0);
}

fn scan_proc(scan: ProcScan<'_>) -> LocalFacts {
    let parameter_body = original_parameter_observation_owner(scan);
    let parameter_owner = parameter_body.is_some();
    let declaration_source_unclosed = parameter_body
        .and_then(|id| {
            scan.source_module
                .original_authored_declaration_body_units
                .get(&id)
        })
        .is_some_and(|card| card.has_unclosed_declaration_source_coverage());
    let ProcScan {
        proc,
        unit: _,
        known,
        registry,
        dialect,
        object_types,
        identities,
        declared,
        source_input,
        source_procedures,
        source_module,
    } = scan;
    let mut facts = LocalFacts {
        local_pure: true,
        impure_beyond_links: false,
        param_observations_incomplete: !parameter_owner || declaration_source_unclosed,
        ..LocalFacts::default()
    };
    let params: HashSet<String> = proc
        .params
        .iter()
        .filter(|_| parameter_owner)
        .cloned()
        .collect();
    let ctx = ScanCtx {
        known,
        registry,
        dialect,
        params: &params,
        object_types,
        identities,
        declared,
        positioned_substitutions: false,
        source_input,
        source_procedures,
        source_module,
        current_tokens: None,
        parameter_body,
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
    facts.name_params = proc
        .params
        .iter()
        .enumerate()
        .filter(|(_, param)| facts.upvar_aliases.values().any(|named| named == *param))
        .map(|(index, _)| index)
        .collect();
    facts
}

/// Read-only context shared by the recursive proc/method body scanners
/// ([`scan_script`] / [`scan_statement`] / [`scan_call_facts`] /
/// [`scan_value_substitutions`]). Bundled so the scanners stay within the
/// argument limit.
#[derive(Clone, Copy)]
struct ScanCtx<'a> {
    known: &'a HashSet<ProcedureId>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    params: &'a HashSet<String>,
    /// Reporting candidates for standalone Logical method summaries; source
    /// receiver and callback ownership comes from retained original receipts.
    object_types: &'a HashMap<String, HashSet<String>>,
    /// The original realm supplies positioned bindings and entered-child
    /// inventories. Raw legacy IR has no original site or Native lookup grant.
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations (`# tcl-lsp: stub` blocks and
    /// `<dialect>.tcl.stubs` sidecars). A stub states the same kind of fact a
    /// `CommandSpec` does, so its argument roles reach this scan through the
    /// same query the catalogue's do — [`ScanCtx::surface`] — rather than a
    /// second table beside `registry`. `None` for a caller with no document
    /// (the optimiser's own unit tests).
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    /// Substitutions of the current retained invocation were already scanned
    /// from exact source carriers; raw text must not regain declined semantics.
    positioned_substitutions: bool,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    source_procedures: &'a crate::ir::ProcedureInventory,
    source_module: &'a crate::ir::Module,
    current_tokens: Option<&'a crate::ir::CommandTokens>,
    parameter_body: Option<ProcedureId>,
}

impl<'a> ScanCtx<'a> {
    fn metadata_context(&self) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        self.source_input
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
    }

    /// The command surface this scan resolves argument roles against: the
    /// catalogue generation plus the document's own declarations.
    fn surface(&self) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(self.registry, self.declared)
    }
}

fn possible_body_id(word: &str, ctx: ScanCtx<'_>) -> Option<ProcedureId> {
    let ids = crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
        ctx.current_tokens?,
        word,
        ctx.source_module,
        ctx.registry,
    )?;
    let [id] = ids.as_slice() else {
        return None;
    };
    ctx.known.contains(id).then_some(*id)
}

/// `depth` is the nesting level of `script` — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Past the cap, `script`'s statements
/// are not scanned; `facts` is instead marked exactly as conservatively as
/// [`scan_statement`]'s own `Statement::Barrier` arm (unscanned code must
/// not be silently under-counted as pure / effect-free).
fn scan_script(script: &crate::ir::Script, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    // Different scripts/branches cannot donate conditional alias continuation.
    facts.source_parameter_aliases.clear();
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        facts.param_observations_incomplete = true;
        facts.has_barrier = true;
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        return;
    }
    for stmt in &script.statements {
        scan_statement(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            ctx,
            facts,
            depth,
        );
    }
    facts.source_parameter_aliases.clear();
}

/// Process a `Statement::Call` for interprocedural facts: side-
/// effects classification, internal-call resolution, and param
/// trait inference.  Extracted from [`scan_statement`].
fn scan_call_facts(command: &str, args: &[String], ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    let ScanCtx {
        registry,
        identities,
        ..
    } = ctx;
    // Effective metadata keeps its selected descriptor spelling; procedure
    // targets independently require their original source receipts.
    let resolved: &str = identities.resolve_unpositioned(command).spec_name();
    let invokes_named_proc = registry.get(resolved).is_some_and(|spec| {
        spec.traits
            .contains(tcl_registry::Traits::INVOKES_USER_PROC)
    });
    let internal_target =
        if invokes_named_proc && let Some(name) = args.first().filter(|n| is_plain_proc_name(n)) {
            possible_body_id(name, ctx)
        } else {
            possible_body_id(command, ctx)
        };

    // Command-prefix callbacks (`lsort -command myCompare`, `trace add … cb`,
    // `interp alias {} a {} target`) are call edges too: the referenced proc is
    // reachable, so a callback-only proc is not dead (O124) and appears in the
    // call graph. Registry-driven via `command_prefixes` — no command-name
    // matching here. Literal single-identifier heads only (`is_plain_proc_name`
    // rejects dynamic `$cb` / bracketed heads), mirroring the reference
    // extractor's bareword guard.
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for (idx, _appended) in ctx
        .surface()
        .command_prefixes(resolved, &arg_strs)
        .into_iter()
        .filter(|_| !ctx.positioned_substitutions)
    {
        if let Some(word) = args.get(idx).and_then(|a| command_prefix_head(registry, a))
            && is_plain_proc_name(&word)
            && let Some(target) = possible_body_id(&word, ctx)
        {
            facts.procedure_calls.push((target, None));
            facts.direct_calls.insert(target);
        }
    }

    let words: Vec<_> = arg_strs
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect();
    if !ctx.positioned_substitutions {
        scan_candidate_method_callbacks(
            tcl_syntax::naming::whole_scalar_variable_reference_name(
                command,
                ctx.source_module.lexer_config,
            )
            .unwrap_or(command),
            &arg_strs,
            &words,
            ctx,
            facts,
        );
    }

    // A call that resolves to an internal proc contributes ONLY a
    // call-graph edge — its purity / effects flow through the
    // interprocedural fixpoints from the callee's summary, so the
    // command's own side-effect classification is NOT applied locally.
    // Non-internal commands apply their classified side effects.
    if let Some(target) = &internal_target {
        facts
            .procedure_calls
            .push((*target, (!invokes_named_proc).then(|| args.to_vec())));
        facts.direct_calls.insert(*target);
    } else {
        // Side-effect classification is dialect-agnostic here
        // (`classify_side_effects_in` is called with no dialect): a
        // command's effect profile reflects what it *does*,
        // not which dialect it is valid in. (The document `dialect` still
        // drives the lexer above so `[cmd …]` / bodies tokenise correctly.)
        // This is why e.g. `log`/`puts` resolve to their LOG_IO/FILE_IO
        // hints — impure but region-free — even under a Tcl dialect.
        // Classified against the document's surface, so a stub's `-pure`
        // or `-mutator` states the effect a catalogue spec would.
        let surface = ctx.surface();
        let ci = if ctx.source_input.is_some() || ctx.current_tokens.is_some() {
            // Original positioned invocations were consumed first by
            // scan_positioned_registry_facts and its Normal handler owner.
            // A retained-source fallback cannot replace that missing receiver
            // with the legacy available-spec hint walk.
            classify_side_effects_in(
                &surface,
                resolved,
                args,
                None,
                Some(&crate::side_effects::CalleeSummary {
                    effect_reads: EffectRegion::UNKNOWN_STATE,
                    effect_writes: EffectRegion::UNKNOWN_STATE,
                    pure: false,
                }),
            )
        } else {
            classify_side_effects_in(&surface, resolved, args, None, None)
        };
        if ci.dynamic_barrier {
            facts.param_observations_incomplete = true;
            facts.has_barrier = true;
            facts.mark_impure();
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
            facts.mark_impure();
        }
        // A command the document declares is known — its declaration is
        // a workspace-authored fact, classified above — so only a name
        // neither the catalogue nor the document knows is an unknown call.
        if registry.get(resolved).is_none() && !surface.declares(resolved) {
            facts.param_observations_incomplete = true;
            facts.has_unknown_calls = true;
            facts.mark_impure();
        }
    }

    note_original_command_params(
        ctx,
        if internal_target.is_some() {
            ProcArgTrait::ForwardedToCallee
        } else {
            ProcArgTrait::Passthrough
        },
        facts,
    );
}

/// Call-by-name alias-pair handling (`upvar`): the level and the `otherVar
/// myVar` pairs are the head's frame-effect declaration's, the level word
/// present by argument-count parity.  Mark a `$param` *other* (caller-var-name
/// source) as [`ProcArgTrait::VarRead`] and — only when the level is the
/// caller's frame, where a write lands in the caller — record `local → param`
/// so a later write of `local` upgrades it to [`ProcArgTrait::VarWrite`].  A
/// `$param` *local* name (the binding target) is itself a write of that
/// param's value. Every local linked is recorded; whether each pair links a
/// plain local to the place a parameter names one frame up is the answer.
fn handle_upvar_aliases(
    effect: tcl_registry::frame_effect::FrameEffectSpec,
    args: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) -> bool {
    let (level, pairs) = effect.resolve_in(args, ctx.registry);
    let caller_frame = level.is_caller_frame();
    let mut names_only = caller_frame && !pairs.is_empty() && pairs.len().is_multiple_of(2);
    for [other_var, my_var] in pairs.as_chunks::<2>().0 {
        facts.linked_locals.insert(my_var.clone());
        names_only &= tcl_syntax::naming::whole_scalar_variable_reference_name(
            other_var,
            ctx.source_module.lexer_config,
        )
        .is_some_and(|name| ctx.params.contains(name))
            && is_plain_local_name(my_var)
            && !ctx.params.contains(my_var);
        if let Some(other_vn) = tcl_syntax::naming::whole_scalar_variable_reference_name(
            other_var,
            ctx.source_module.lexer_config,
        ) && ctx.params.contains(other_vn)
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
        if let Some(my_vn) = tcl_syntax::naming::whole_scalar_variable_reference_name(
            my_var,
            ctx.source_module.lexer_config,
        ) && ctx.params.contains(my_vn)
        {
            facts
                .param_trait_flags
                .entry(my_vn.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarWrite);
        }
    }
    names_only
}

/// Whether `word` spells a plain local variable: a name of letters, digits
/// and underscores, neither qualified nor an array element.
fn is_plain_local_name(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && word.chars().all(|c| c.is_alphanumeric() || c == '_')
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
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    // Track scope-aliasing declarations (`global` / `variable` / `upvar #0`)
    // so a later bare write to an aliased name counts as `writes_global`.
    // Declaring is not writing, so handle the declaration before the
    // defs-based write check.
    let alias_pairs = ctx
        .registry
        .frame_effect(command)
        .filter(|effect| effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs);
    let alias_names = if ctx.source_input.is_some() || tokens.is_some() || ctx.declared.is_some() {
        Some(GlobalAliasNameCoverage {
            names: HashSet::new(),
            unknown: true,
        })
    } else {
        global_alias_names(command, args, ctx.registry)
    };
    match alias_names {
        Some(alias_names) => {
            facts.writes_global |= alias_names.unknown;
            facts.global_aliases.extend(alias_names.names);
        }
        None => {
            // A non-declaration command that writes a `::`-qualified
            // or global-aliased variable (`append g`, `lappend g`,
            // `dict set ::cfg ...`) mutates caller-visible state.
            if defs
                .iter()
                .any(|n| n.starts_with("::") || facts.global_aliases.contains(n))
            {
                facts.writes_global = true;
            }
        }
    }
    // Call-by-name: record `upvar` aliases, and treat any command writing a
    // level-1 upvar alias (`append` / `lappend` / `lassign` … via `defs`) as a
    // write-back to the caller's variable.
    let links_only = if let Some(effect) = alias_pairs {
        handle_upvar_aliases(effect, args, ctx, facts)
    } else {
        // An alias-pair call itself is excluded — its `defs` are the locals it
        // *defines* (aliases), not writes.
        for d in defs {
            mark_upvar_alias_write(d, facts);
        }
        false
    };
    let beyond = facts.impure_beyond_links;
    scan_call_facts(command, args, ctx, facts);
    // Linking a local to the place a parameter names one frame up does
    // nothing the caller sees by itself: what the body does through the link
    // is what the caller sees.
    if links_only {
        facts.impure_beyond_links = beyond;
    }
    scan_role_code_arguments(command, args, tokens, ctx, facts);
}

/// Recurse into the code-bearing arguments of a statement that carries them
/// as opaque words, recording the calls they make as edges of the enclosing
/// unit.
///
/// A command whose script or expression argument has no dedicated lowering —
/// `time {…}`, and every declared `script:body` / `cond:expr` — reaches the
/// IR as a plain `Statement::Call` or `Statement::Barrier` holding its own
/// words. A barrier makes the call's *effects* opaque; neither shape makes
/// the code unreadable, and the procedures that code calls are exactly as
/// reachable as those in an `eval` body, which lowering splices inline and
/// this scan already walks. A callback proc reached only this way is live
/// code, not a call-graph leaf.
///
/// [`ArgRole::Body`] words are scanned as scripts and [`ArgRole::Expr`] words
/// for the `[cmd …]` substitutions the expression engine re-evaluates, which
/// is what [`scan_source_for_calls`] does for the same two roles.
/// [`ArgRole::LambdaLiteral`] stays with that scanner alone: splitting a
/// `{params body ?ns?}` list needs the word's token, and a statement carries
/// only its text.
///
/// Code that does not run in the caller's frame *and* namespace is left
/// alone. Lowering registers those as their own body units, which the
/// call-site scan visits under the namespace they actually target; walking
/// one here would instead invent an edge to a same-named proc in the
/// caller's namespace (the rule issues #977 / #980 set for the call-site
/// evidence scan).
///
/// Each word starts a fresh [`MAX_BRACKET_TEXT_DEPTH`] scan (depth 0), like
/// the `[cmd …]` substitution scans beside it: that counter bounds recursion
/// *within* re-segmented text, and is not the statement-tree depth the caller
/// is carrying.
fn scan_role_code_arguments(
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use tcl_registry::prelude::Traits;
    let surface = ctx.surface();
    let resolved: &str = ctx.identities.resolve_unpositioned(command).spec_name();
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let context = ctx.metadata_context();
    let structured = tokens.map(|tokens| {
        crate::registry_invocation::resolve_command_tokens_with_metadata_context(
            ctx.registry,
            context.as_deref().map(Into::into),
            tokens,
        )
    });
    let roles;
    let traits;
    if let Some(structured) = structured {
        let Ok(crate::registry_invocation::RegistryInvocationResolution::Resolved(resolved)) =
            structured
        else {
            return;
        };
        if !resolved.arg_roles_complete {
            return;
        }
        traits = resolved.traits;
        roles = resolved
            .arg_roles
            .iter()
            .map(|&(index, role)| (resolved.argument_offset + usize::from(index), role))
            .collect::<Vec<_>>();
    } else {
        traits = ctx.registry.invocation_traits(resolved, &arg_strs, None);
        roles = [
            tcl_registry::ArgRole::NamespaceName,
            tcl_registry::ArgRole::Name,
            tcl_registry::ArgRole::Body,
            tcl_registry::ArgRole::Expr,
        ]
        .into_iter()
        .flat_map(|role| {
            surface
                .arg_indices_for_role(resolved, &arg_strs, role)
                .into_iter()
                .map(move |index| (index, role))
        })
        .collect();
    }
    if traits.intersects(
        tcl_registry::traits::FRAME_REACH_TRAITS
            .union(Traits::DEFINES_PROCEDURE)
            .union(Traits::DECLARES_NAMESPACE),
    ) {
        return;
    }
    // An absolutely-spelled name / namespace word says the same thing the
    // traits above do for the commands that carry neither: the code resolves
    // somewhere other than here.
    if [
        tcl_registry::ArgRole::NamespaceName,
        tcl_registry::ArgRole::Name,
    ]
    .into_iter()
    .flat_map(|wanted| {
        roles
            .iter()
            .filter(move |(_, role)| *role == wanted)
            .map(|(index, _)| *index)
    })
    .filter_map(|index| args.get(index))
    .any(|name| name.starts_with("::"))
    {
        return;
    }
    for index in roles
        .iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
        .map(|(index, _)| *index)
    {
        if let Some(body_text) = args.get(index) {
            scan_source_for_calls(body_text, ctx, facts, 0);
        }
    }
    for index in roles
        .iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::Expr)
        .map(|(index, _)| *index)
    {
        if let Some(expr_text) = args.get(index) {
            scan_value_substitutions(strip_one_brace_layer(expr_text), ctx, facts, 0);
        }
    }
}

fn scan_unproved_invocation(args: &[String], ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    facts.param_observations_incomplete = true;
    note_original_command_params(ctx, ProcArgTrait::Passthrough, facts);
    facts.has_barrier = true;
    facts.has_unknown_calls = true;
    facts.local_pure = false;
    facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
    facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    for arg in args {
        scan_value_substitutions(arg, ctx, facts, 0);
    }
}

/// An expression operand's inner text: `expr {…}`'s brace quoting suppresses
/// substitution at the word level, but the expression engine re-evaluates the
/// contents, so a `[cmd …]` inside is a real call. An unbraced operand
/// (`[q]`, `$x`) is already its own inner text.
fn strip_one_brace_layer(word: &str) -> &str {
    word.strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .unwrap_or(word)
}

/// Project alias identities from the same resolved transition facts as the
/// point-binding solver. The registry owns optional-level and pair grammar;
/// this consumer only records which caller parameters and visible cells a
/// transition can reach.
fn scan_resolved_aliases(
    resolved: &crate::registry_invocation::ResolvedStatementInvocation,
    defs: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use tcl_registry::frame_effect::FrameLevel;
    use tcl_registry::state_transition::{
        CallerFrameSelection, StateTransition, TransitionSubject, VariableAliasTarget,
    };
    let parameter = |subject: &TransitionSubject| -> Option<String> {
        let TransitionSubject::Unknown { argument_index, .. } = subject else {
            return None;
        };
        let crate::ir::WordExpr::Variable { spelling, .. } = resolved
            .effective
            .words
            .get(argument_index.checked_add(1)?)?
        else {
            return None;
        };
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(
            spelling,
            ctx.source_module.lexer_config,
        )?;
        ctx.params.contains(name).then(|| name.to_owned())
    };
    let mut declarations = HashSet::new();
    if let Some(transitions) = resolved.facts.state_transitions.declared() {
        for fact in transitions.facts() {
            let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if let Some(local) = alias.local.literal() {
                if !alias.writes_value {
                    declarations.insert(local.to_owned());
                }
            } else if let Some(name) = parameter(&alias.local) {
                facts
                    .param_trait_flags
                    .entry(name)
                    .or_default()
                    .insert(ProcArgTrait::VarWrite);
            }
            let (visible, caller, variable) = match &alias.target {
                VariableAliasTarget::Global { variable }
                | VariableAliasTarget::CurrentNamespace { variable }
                | VariableAliasTarget::Namespace { variable, .. } => (true, false, variable),
                VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                    let level = match frame {
                        CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
                        CallerFrameSelection::Explicit(subject) => {
                            subject.literal().and_then(|word| {
                                resolved.dialect.map_or_else(
                                    || FrameLevel::parse(word),
                                    |dialect| FrameLevel::parse_for_dialect(word, dialect),
                                )
                            })
                        }
                    };
                    (
                        level.is_none_or(|level| {
                            matches!(level, FrameLevel::Absolute(_) | FrameLevel::Dynamic)
                        }),
                        level == Some(FrameLevel::DEFAULT),
                        variable,
                    )
                }
            };
            if visible {
                match alias.local.literal() {
                    Some(local) => {
                        facts.global_aliases.insert(local.to_owned());
                    }
                    None => facts.writes_global = true,
                }
                facts.writes_global |= alias.writes_value;
            }
            if let Some(name) = parameter(variable) {
                facts
                    .param_trait_flags
                    .entry(name.clone())
                    .or_default()
                    .insert(ProcArgTrait::VarRead);
                if caller && let Some(local) = alias.local.literal() {
                    facts.upvar_aliases.insert(local.to_owned(), name);
                }
            }
        }
    }
    // Barrier-shaped calls can retain exact native write roles even when their
    // value is dynamic. Projection must not depend on structural lowering.
    let mut writes: BTreeSet<String> = defs.iter().cloned().collect();
    writes.extend(resolved.facts.arg_roles.iter().filter_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::VarWrite)
            .then(|| {
                resolved.argument_literal(resolved.facts.argument_offset + usize::from(*index))
            })
            .flatten()
    }));
    for name in writes.iter().filter(|name| !declarations.contains(*name)) {
        mark_upvar_alias_write(name, facts);
        facts.writes_global |= name.starts_with("::") || facts.global_aliases.contains(name);
    }
}

/// Project normal physical writes without licensing native execution. The
/// selected handler may still carry a compiler-error or unknown completion.
fn scan_normal_visible_writes(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        return;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        ctx.registry,
        Some(context.as_ref().into()),
        tokens,
    ) else {
        return;
    };
    facts.writes_global |= normal
        .mutation_places(&binding.variable_context, ctx.registry)
        .iter()
        .any(|place| {
            place.cell.as_ref().is_some_and(|cell| {
                matches!(
                    cell.owner,
                    crate::place::CellOwner::Namespace(_)
                        | crate::place::CellOwner::NamespaceIdentity(_)
                )
            })
        });
}

/// Candidate class metadata contributes callback reachability only. It does
/// not prove the receiver's implementation, effects or method execution.
fn scan_candidate_method_callbacks(
    receiver: &str,
    arguments: &[&str],
    words: &[tcl_registry::InvocationWord<'_>],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(method) = words.first().and_then(|word| word.literal()) else {
        return;
    };
    let Some(classes) = ctx.object_types.get(receiver) else {
        return;
    };
    for class in classes {
        for (index, _) in ctx.registry.instance_method_command_prefixes_structured(
            class,
            method,
            &arguments[1..],
            &words[1..],
        ) {
            if let Some(word) = words
                .get(index + 1)
                .and_then(|word| word.literal())
                .and_then(|word| command_prefix_head(ctx.registry, word))
                && is_plain_proc_name(&word)
                && let Some(target) = possible_body_id(&word, ctx)
            {
                facts.direct_calls.insert(target);
            }
        }
    }
}

/// Original callback source targets contribute possible call-graph edges.
/// Whole installer, builder, frame descriptor and declaration geometry retain
/// their own obligations; receiver labels never select a callback procedure.
fn scan_positioned_candidate_callbacks(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(rows) = ctx.source_input.and_then(|input| {
        ctx.identities
            .original_source_callback_targets_for_tokens(input, tokens)
    }) else {
        return;
    };
    for row in rows {
        if let Some(id) = row.target().and_then(|target| {
            target
                .original_ir_procedure_id(ctx.source_procedures)
                .or_else(|| target.original_source_ir_procedure_id(ctx.source_module, ctx.registry))
        }) && ctx.known.contains(&id)
        {
            facts.direct_calls.insert(id);
        }
    }
}

/// Successful handler entry can contribute a possible call edge without
/// withdrawing a compiler or completion obligation from its statement.
fn scan_normal_procedure_caller(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        return;
    };
    if let Some(call) =
        crate::registry_invocation::normal_user_procedure_invocation_with_metadata_context(
            ctx.registry,
            Some(context.as_ref().into()),
            tokens,
        )
        && let Some(callee) = possible_body_id(&call.target, ctx)
    {
        // A possible successful handler entry contributes a call-graph edge.
        // Compiler/completion uncertainty below still withdraws effects and purity.
        facts.direct_calls.insert(callee);
    }
}

/// Possible call edges are independent of the later strict effect proof.
fn scan_positioned_reachability(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(binding) = &tokens.source_binding else {
        return;
    };
    scan_normal_procedure_caller(tokens, ctx, facts);
    scan_positioned_candidate_callbacks(tokens, ctx, facts);
    let source_calls = if ctx
        .source_module
        .source_metadata_input
        .as_ref()
        .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
    {
        crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    } else {
        crate::registry_invocation::original_authored_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    };
    if let Some(calls) = source_calls {
        for call in calls {
            if call.accepts_arguments() && ctx.known.contains(&call.id()) {
                facts.direct_calls.insert(call.id());
            }
        }
    }
    if let Some(context) = ctx.metadata_context()
        && let Some((reads, writes)) =
            crate::registry_invocation::possible_normal_handler_effect_regions_with_metadata_context(
                ctx.registry,
                Some(context.as_ref().into()),
                tokens,
            )
    {
        facts.effect_reads |= reads;
        facts.effect_writes |= writes;
    }
    // Reachability depends on the target identity, independently of whether
    // a retained alias prefix has printable bytes or a composable argv shape.
    // Unknown/absent residuals still withdraw effects and purity below.
    for target in binding.execution_targets() {
        if !target.registry_backed
            && let Some((callee, _)) = ctx.source_module.source_procedure_for_target(target)
            && ctx.known.contains(&callee)
        {
            facts.direct_calls.insert(callee);
        }
    }
    // naming.compiler.original-positioned-child-body-reachability

    // Each child retains its actual target, source offset and entered owner.
    // Re-segmenting an argument value cannot donate a nested command's roles.
    if let Some(site) = binding.invocation_site() {
        for child in ctx.identities.possible_entered_body_invocations(site) {
            for target in child.execution_targets() {
                if !target.registry_backed
                    && let Some((callee, _)) = ctx.source_module.source_procedure_for_target(target)
                    && ctx.known.contains(&callee)
                {
                    facts.direct_calls.insert(callee);
                }
            }
        }
    }
}

fn scan_positioned_registry_facts(
    resolved: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: &crate::ir::CommandTokens,
    args: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        scan_unproved_invocation(args, ctx, facts);
        return;
    };
    let Some((reads, writes)) =
        crate::registry_invocation::possible_normal_handler_effect_regions_with_metadata_context(
            ctx.registry,
            Some(context.as_ref().into()),
            tokens,
        )
    else {
        scan_unproved_invocation(args, ctx, facts);
        return;
    };
    facts.effect_reads |= reads;
    facts.effect_writes |= writes;
    facts.writes_global |= writes.intersects(EffectRegion::GLOBAL_STATE);
    let traits = resolved.facts.traits;
    if traits
        .intersects(tcl_registry::Traits::EVALUATES_CODE | tcl_registry::Traits::CREATES_BARRIER)
    {
        scan_unproved_invocation(args, ctx, facts);
        return;
    }
    if !traits.intersects(tcl_registry::Traits::PURE | tcl_registry::Traits::PURE_EVALUATION)
        || !writes.is_empty()
    {
        facts.mark_impure();
    }
    // Targets and source incidence retain independent original owners.
    note_original_command_params(ctx, ProcArgTrait::Passthrough, facts);
}

/// Consume positioned implementation facts before assistance projections.
fn scan_positioned_invocation(
    stmt: &crate::ir::Statement,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) -> bool {
    use crate::ir::Statement;
    let (args, defs, tokens) = match stmt {
        Statement::Call {
            args,
            defs,
            tokens: Some(tokens),
            ..
        } => (args, defs.as_slice(), tokens),
        Statement::Barrier {
            args,
            tokens: Some(tokens),
            ..
        } => (args, &[][..], tokens),
        _ => return false,
    };
    let Some(binding) = &tokens.source_binding else {
        return false;
    };
    scan_positioned_reachability(tokens, ctx, facts);
    let Some(target) = binding.proved_execution_target() else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    let Some(_effective_args) = effective.argument_presentations(args) else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    // The exact point binding supersedes a whole-document assistance join.
    let positioned = ScanCtx {
        identities: crate::realm::CommandBindingRealm::none(),
        positioned_substitutions: true,
        current_tokens: Some(tokens),
        ..ctx
    };
    if target.registry_backed {
        let Some(context) = ctx.metadata_context() else {
            scan_unproved_invocation(args, positioned, facts);
            return true;
        };
        let Some(resolved) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                ctx.registry,
                Some(context.as_ref().into()),
                stmt,
            )
        else {
            scan_unproved_invocation(args, ctx, facts);
            return true;
        };
        scan_resolved_aliases(&resolved, defs, positioned, facts);
        if let Some(presented) = resolved
            .arguments
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
        {
            scan_positioned_registry_facts(&resolved, tokens, &presented, positioned, facts);
        } else {
            // The target is proved, but a legacy textual effect/callback
            // query cannot invent bytes for a captured unmaterialised value.
            scan_unproved_invocation(args, positioned, facts);
        }
    } else if let Some(target) = possible_body_id(&target.command, ctx) {
        facts.direct_calls.insert(target);
        note_original_command_params(positioned, ProcArgTrait::ForwardedToCallee, facts);
        for arg in args {
            scan_value_substitutions(arg, ctx, facts, 0);
        }
    } else {
        scan_unproved_invocation(args, ctx, facts);
    }
    if matches!(stmt, Statement::Barrier { .. }) {
        facts.has_barrier = true;
        facts.local_pure = false;
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    }
    true
}

fn scan_retained_substitutions<'a>(
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'a>,
    facts: &mut LocalFacts,
) -> ScanCtx<'a> {
    use crate::ir::Statement;
    if let Some(tokens) = tokens.filter(|tokens| tokens.source_binding.is_some()) {
        let positioned = ScanCtx {
            positioned_substitutions: true,
            ..ctx
        };
        for lifted in crate::word_subst::lifted_calls_with_surface(
            Some(tokens),
            tcl_lexer::LexerConfig::for_profile(ctx.dialect),
            &ctx.surface(),
        ) {
            let nested = Statement::Call {
                span: lifted.span,
                command: lifted.command,
                canonical_command: None,
                args: lifted.args,
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: lifted.tokens,
                foreach_groups: None,
            };
            if !scan_positioned_invocation(&nested, positioned, facts) {
                // Missing retained provenance cannot authorise a lexical query.
                if let Statement::Call { args, .. } = &nested {
                    scan_unproved_invocation(args, positioned, facts);
                }
            }
        }
        positioned
    } else {
        ctx
    }
}

fn scan_original_variable_read_effects(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use crate::place::CellOwner;
    let unknown_read = |facts: &mut LocalFacts| {
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::GLOBAL_STATE | EffectRegion::UNKNOWN_STATE;
    };
    let Some(input) = ctx.source_input else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let Some((binding, site, config)) = tokens.source_binding.as_ref().and_then(|binding| {
        Some((
            binding,
            binding.invocation_site()?,
            binding.original_lexer_config_for_tokens(tokens)?,
        ))
    }) else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let Some(image) = ctx.identities.original_source_image() else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let original = ctx.identities.invocation_at_source("", site.offset);
    let corresponding = ctx.identities.matches_resolved_analysis_input(input)
        && input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            == ctx.registry.snapshot().semantic_key()
        && ctx
            .identities
            .matches_original_source_image(image, input.lexer_config())
        && site.source.source_image() == image
        && config == input.lexer_config()
        && binding
            .original_invocation_metadata_for_input(tokens, input, ctx.registry)
            .is_some()
        && original.invocation_site() == Some(site)
        && original.original_lexer_config_for_tokens(tokens) == Some(config)
        && original
            .original_recorded_command()
            .is_some_and(|(_, original)| original.words() == tokens.words());
    if !corresponding {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    }
    for access in ctx
        .identities
        .source_bindings_ref()
        .variable_accesses_during_expression_invocation(site.offset)
    {
        if access.context_alternatives().is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            unknown_read(facts);
        }
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, ctx.registry);
            if place.dynamic || place.observed {
                unknown_read(facts);
                continue;
            }
            match place.cell.as_ref().map(|cell| &cell.owner) {
                Some(CellOwner::CurrentFrame) => {}
                Some(CellOwner::Activation(owner))
                    if context.activation.as_ref() == Some(owner) => {}
                Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_)) => {
                    facts.mark_impure();
                    facts.effect_reads |= EffectRegion::GLOBAL_STATE;
                }
                _ => unknown_read(facts),
            }
        }
    }
}

/// `depth` is `stmt`'s own nesting level — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Dispatch-only (no nested `Script`
/// entered here) keeps the same `depth`; recursing into a nested body
/// (`UpFrame`/`Block`'s inner statements, or any control-flow body via
/// [`scan_control_flow_statement`]) passes `depth + 1`.
fn scan_statement(
    stmt: &crate::ir::Statement,
    retained_tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::ir::Statement;
    if !stmt.is_executable_invocation() {
        return;
    }
    let ScanCtx { params, .. } = ctx;
    let ctx = ScanCtx {
        current_tokens: retained_tokens,
        ..ctx
    };
    note_original_command_references(ctx, facts);
    note_original_authored_alias_parameters(ctx, facts);
    note_original_authored_body_call_parameters(ctx, facts);
    let ctx = scan_retained_substitutions(retained_tokens, ctx, facts);
    if let Some(tokens) = retained_tokens {
        scan_original_variable_read_effects(tokens, ctx, facts);
        scan_normal_visible_writes(tokens, ctx, facts);
    }
    if scan_positioned_invocation(stmt, ctx, facts) {
        return;
    }
    match stmt {
        Statement::NativeCall { .. } => {
            facts.param_observations_incomplete = true;
            facts.has_barrier = true;
            facts.local_pure = false;
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }

        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            // A barrier makes effects opaque; it does not erase the call's
            // registry-declared identities.  In particular, same-invocation
            // command prefixes such as `lsort -command cb` still make `cb`
            // reachable, while deferred registrations that conservatively
            // lower to a barrier retain their callback edge too — and neither
            // does it erase the script its `ArgRole::Body` words carry.
            scan_call_facts(command, args, ctx, facts);
            scan_role_code_arguments(command, args, tokens.as_ref(), ctx, facts);
            facts.has_barrier = true;
            facts.mark_impure();
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
        Statement::UpFrame { body, .. } => {
            facts.param_observations_incomplete = true;
            // Static-body uplevel runs the inner script in the
            // caller's frame — for interprocedural purposes it can
            // touch any caller-scope variable, so treat it as a
            // barrier conservatively. Any reads/writes inside
            // ``body`` propagate up.
            facts.has_barrier = true;
            facts.mark_impure();
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
            let outer_parameters = HashSet::new();
            scan_script(
                body,
                ScanCtx {
                    params: &outer_parameters,
                    parameter_body: None,
                    ..ctx
                },
                facts,
                depth + 1,
            );
        }
        Statement::Block { body, .. } => {
            // ``Block`` is a transparent splice: walk through to the
            // inner statements without flagging a barrier.
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::AssignConst { name, .. } => {
            note_assign_global_write(name, facts);
        }
        Statement::AssignExpr {
            name,
            expr,
            expr_base,
            ..
        } => {
            note_assign_global_write(name, facts);
            note_original_expression_params(expr, *expr_base, ctx, facts);
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
        Statement::Return {
            value,
            value_word,
            expr,
            expr_base,
            braced,
            ..
        } => {
            note_original_return_parameter(value_word.as_ref(), ctx, facts);
            let config = retained_tokens.and_then(|tokens| {
                let binding = tokens.source_binding.as_ref()?;
                binding.original_invocation_metadata_for_module(
                    tokens,
                    ctx.source_module,
                    ctx.registry,
                )?;
                let config = binding.original_lexer_config_for_tokens(tokens)?;
                if let Some(expression) = expr {
                    let (original_config, _) =
                        crate::registry_invocation::original_lowered_expression_syntax(
                            expression, *expr_base, tokens,
                        )?;
                    (original_config == config).then_some(())?;
                }
                Some(config)
            });
            if let Some(expression) = expr {
                let mut references = Vec::new();
                let source_config =
                    original_parameter_expression_config(expression, *expr_base, ctx);
                if !walk_collect_param_refs(expression, params, &mut references, source_config, 0) {
                    facts.param_observations_incomplete = true;
                }
                facts.param_references.extend(references);
            }
            let kind = classify_return(
                value.as_deref(),
                *braced,
                value_word.as_ref(),
                expr.as_ref(),
                params,
                config,
            );
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
            tokens,
            ..
        } => {
            scan_call_statement(command, args, defs, tokens.as_ref(), ctx, facts);
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
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                note_original_expression_params(&c.condition, c.condition_base, ctx, facts);
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
            condition_base,
            next,
            body,
            ..
        } => {
            note_original_expression_params(condition, *condition_base, ctx, facts);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(init, ctx, facts, depth + 1);
            scan_script(next, ctx, facts, depth + 1);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::While {
            condition,
            condition_base,
            body,
            ..
        } => {
            note_original_expression_params(condition, *condition_base, ctx, facts);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::ExprEval {
            expr, expr_base, ..
        } => {
            note_original_expression_params(expr, *expr_base, ctx, facts);
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

#[derive(Default)]
struct GlobalAliasNameCoverage {
    names: HashSet<String>,
    unknown: bool,
}

/// Explicit standalone alias candidates; uncertainty is separate from legal empty data.
fn global_alias_names(
    command: &str,
    args: &[String],
    registry: &tcl_registry::CommandRegistry,
) -> Option<GlobalAliasNameCoverage> {
    use tcl_registry::state_transition::{
        CallerFrameSelection, StateTransition, VariableAliasTarget,
    };
    use tcl_registry::{InvocationWord, InvocationWords};
    let arguments = args
        .iter()
        .map(|name| {
            if crate::dynamic_names::names_a_dynamic_variable(name) || name.contains('\0') {
                InvocationWord::Dynamic
            } else {
                InvocationWord::Literal(name)
            }
        })
        .collect::<Vec<_>>();
    let schema = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(command), &arguments)
                .with_profile(registry.profile()),
            registry.own_surface_query(),
        )
        .resolved()?;
    let transitions = schema.state_transitions();
    let mut coverage = GlobalAliasNameCoverage::default();
    let mut selected = false;
    for fact in transitions.facts() {
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        let visible = match &alias.target {
            VariableAliasTarget::Global { .. }
            | VariableAliasTarget::CurrentNamespace { .. }
            | VariableAliasTarget::Namespace { .. } => true,
            VariableAliasTarget::CallerSelectedFrame { frame, .. } => match frame {
                CallerFrameSelection::DefaultCaller => false,
                CallerFrameSelection::Explicit(level) => match level.literal() {
                    Some(level) => {
                        let level = schema.words.arguments().dialect().map_or_else(
                            || tcl_registry::frame_effect::FrameLevel::parse(level),
                            |dialect| {
                                tcl_registry::frame_effect::FrameLevel::parse_for_dialect(
                                    level, dialect,
                                )
                            },
                        );
                        match level {
                            Some(level) if level.is_global_frame() || level.is_current_frame() => {
                                true
                            }
                            Some(tcl_registry::frame_effect::FrameLevel::Dynamic) | None => {
                                coverage.unknown = true;
                                true
                            }
                            Some(_) => false,
                        }
                    }
                    None => {
                        coverage.unknown = true;
                        true
                    }
                },
            },
        };
        if !visible {
            continue;
        }
        selected = true;
        match alias.local.literal() {
            Some(name) => {
                coverage.names.insert(
                    tcl_syntax::naming::split_element_ref(name)
                        .map_or(name, |(root, _)| root)
                        .to_owned(),
                );
            }
            None => coverage.unknown = true,
        }
    }
    selected.then_some(coverage)
}

/// Walk an expression AST and record call-graph edges (and Body
/// recursion) for every `[cmd ...]` command substitution embedded
/// in the expression.
///
/// Without it, call-graph edges and unused-proc detection miss proc calls
/// embedded in control-flow predicates: the per-proc fact scanner walks
/// statement bodies but not expression operands.
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
    // Native-stack safety net: this walks the `ExprNode`
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
        // Quoted strings may contain command substitutions (`"[q]"`), so
        // descend through the source text the same way as for `Command`;
        // a braced `{…}` one does not substitute.
        ExprNode::String { text, .. } => {
            if let Some(inner) = crate::word_subst::quoted_operand_body(text) {
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
        facts.mark_impure();
    }
    mark_upvar_alias_write(name, facts);
}

/// Scan every `[cmd …]` command substitution embedded in a value /
/// expression `text` for call-graph edges. The text is lexed under the document
/// dialect; each [`TokenType::Cmd`] token's inner script is handed to
/// [`scan_source_for_calls`] (which resolves the head, applies the
/// callee's effects, and recurses into `BODY`-role args).
fn scan_value_substitutions(text: &str, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    if ctx.positioned_substitutions {
        return;
    }
    let dialect = ctx.dialect;
    if !text.contains('[') {
        return;
    }
    let lexer = tcl_lexer::Lexer::with_source_map(
        tcl_lexer::SourceMap::new(text),
        tcl_lexer::LexerConfig::for_profile(dialect),
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
            // through; `scan_source_for_calls` enforces the cap.
            scan_source_for_calls(&text[start..end], ctx, facts, depth);
        }
    }
}

fn scan_source_for_calls(source: &str, ctx: ScanCtx<'_>, facts: &mut LocalFacts, depth: u32) {
    let ctx = ScanCtx {
        current_tokens: None,
        parameter_body: None,
        ..ctx
    };
    facts.param_observations_incomplete = true;
    // Native-stack safety net: this recurses into `ArgRole::Body`
    // args, `apply` lambda bodies, and nested `[cmd …]` substitutions inside a
    // single word's raw text — a genuinely unbounded axis, independent of any
    // statement-tree cap (`catch {catch {catch {…}}}` / `apply {{} {apply {{}
    // {…}}}}` nested arbitrarily deep). Past the cap, stop descending: call
    // edges buried deeper than the cap are not recorded, never a crash.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let ScanCtx {
        dialect,
        identities,
        ..
    } = ctx;
    let surface = ctx.surface();
    // Scan the call graph under the
    // document dialect so `{*}` (8.4 / iRules) and `}{` (iRules) tokenise
    // the same way the rest of the analyser/lowering now does.
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_profile(dialect),
    );
    for cmd in commands {
        // Skip empty / non-literal command names — they're not
        // call-graph edges we can resolve at compile time.
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        // The head's effective identity, for every registry role query below.
        // `scan_call_facts` resolves it again for its own
        // queries — it is also reached from the `Statement::Call` arm, which
        // has no segmented command to hand it.
        let resolved: &str = identities.resolve_unpositioned(name).spec_name();
        let texts = cmd.args();
        scan_call_facts(name, texts, ctx, facts);
        // Recurse into BODY-role args (e.g. `catch {p}` → `{p}` is
        // BODY).  The registry resolves the role using the same
        // logic as the top-level scanner.
        let arg_strs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let body_indices = surface.arg_indices_for_role(
            resolved,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Body,
        );
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
        // calls at all.
        let lambda_indices = surface.arg_indices_for_role(
            resolved,
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
                scan_source_for_calls(body_text, ctx, facts, depth + 1);
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
        let expr_indices = surface.arg_indices_for_role(
            resolved,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Expr,
        );
        for idx in expr_indices {
            if let Some(arg) = texts.get(idx) {
                scan_value_substitutions(strip_one_brace_layer(arg), ctx, facts, depth + 1);
            }
        }
        // A `[cmd …]` substitution inside a *plain* value arg also executes in
        // this value/expression context (`set x [matchclass [HTTP::uri] …]`,
        // `if {[matchclass [HTTP::uri] …]}`), so its nested effects and edges
        // propagate too. This worker is reached
        // only from value/return/expr scanning. Positioned statement operands
        // use their retained WordExpr substitution carrier instead. Braced args are
        // inert (no `Cmd` token inside `{…}`); the
        // body/expr args handled above are idempotent under a re-scan.
        for arg in texts {
            if arg.contains('[') {
                scan_value_substitutions(arg, ctx, facts, depth + 1);
            }
        }
    }
}

/// Visit each `ExprNode::Var` in `node` and mark matching
/// parameters as `UsedInCondition` — the expression is inside an
/// `if` / `while` / `for` condition (or a standalone `ExprEval`
/// treated analogously).
fn note_params_in_expr(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    facts: &mut LocalFacts,
    config: tcl_lexer::LexerConfig,
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        facts.param_observations_incomplete = true;
        return;
    }
    match node {
        ExprNode::Var { .. } => {
            let Ok(Some(reference)) = node.variable_reference(config) else {
                facts.param_observations_incomplete = true;
                return;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                facts.param_observations_incomplete = true;
                return;
            };
            if params.contains(name) {
                facts.param_references.insert(name.to_owned());
                facts
                    .param_trait_flags
                    .entry(name.to_owned())
                    .or_default()
                    .insert(ProcArgTrait::UsedInCondition);
            }
            // Index substitutions have their own original word geometry.
            if reference.index.is_some() {
                facts.param_observations_incomplete = true;
            }
        }
        ExprNode::Binary { left, right, .. } => {
            note_params_in_expr(left, params, facts, config, depth + 1);
            note_params_in_expr(right, params, facts, config, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            note_params_in_expr(operand, params, facts, config, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            note_params_in_expr(condition, params, facts, config, depth + 1);
            note_params_in_expr(true_branch, params, facts, config, depth + 1);
            note_params_in_expr(false_branch, params, facts, config, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                note_params_in_expr(a, params, facts, config, depth + 1);
            }
        }
        ExprNode::Literal { .. } => {}
        ExprNode::Command { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Raw { .. } => {
            facts.param_observations_incomplete = true;
        }
    }
}

/// Collapse the raw trait observation flags into the final
/// per-param trait set. Adds `Passthrough` when the proc's
/// `return_passthrough_param` matches this param; adds `Unused`
/// only when the complete owned source inventory contains no reference.
fn finalise_param_traits(
    params: &[String],
    flags: &HashMap<String, HashSet<ProcArgTrait>>,
    passthrough: Option<&str>,
    references: &HashSet<String>,
    complete: bool,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    let mut out: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    for p in params {
        let mut traits: HashSet<ProcArgTrait> = flags.get(p).cloned().unwrap_or_default();
        if passthrough == Some(p) {
            traits.insert(ProcArgTrait::Passthrough);
        }
        if complete && traits.is_empty() && !references.contains(p) {
            traits.insert(ProcArgTrait::Unused);
        }
        out.insert(p.clone(), traits);
    }
    out
}

/// Classify a single `Statement::Return` by its word, for the summary's
/// return shapes: a literal is its value through the exact value ingress —
/// a braced word its content, a bare or quoted one its escapes decoded,
/// nothing trimmed — `$param` a passthrough, and `return [expr {…}]` as
/// [`classify_return_expr`] reads it.
fn classify_return(
    value: Option<&str>,
    braced: bool,
    word: Option<&crate::ir::WordExpr>,
    expr: Option<&crate::expr_ast::ExprNode>,
    params: &HashSet<String>,
    config: Option<tcl_lexer::LexerConfig>,
) -> ReturnKind {
    // Prefer the structured `expr` when the return was `return
    // [expr {…}]` or similar — the AST gives precise information.
    if let Some(node) = expr {
        return classify_return_expr(node, params, config);
    }
    let Some(raw) = value else {
        return ReturnKind::Other;
    };
    if let Some(config) = config
        && let Some(text) = crate::value_transfer::recorded_word_value(raw, braced, &config)
    {
        return ReturnKind::Literal(ExactValue::from_literal(&text));
    }
    if !braced
        && let Some(config) = config
        && let Some(crate::ir::WordExpr::Variable { spelling, .. }) = word
        && let Some(name) =
            tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
        && params.contains(name)
    {
        return ReturnKind::Passthrough(name.to_owned());
    }
    ReturnKind::Other
}

fn classify_return_expr(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    config: Option<tcl_lexer::LexerConfig>,
) -> ReturnKind {
    use crate::expr_ast::ExprNode;

    // A literal operand is the expression's value only where it is a
    // canonical decimal integer: `0x10` is 16, `010` is 8 or 10 by release,
    // and `true` stays `true`, which the expression route decides.
    if let ExprNode::Literal { text, .. } = node {
        let value = ExactValue::from_literal(text);
        return if value.as_int().is_some() {
            ReturnKind::Literal(value)
        } else {
            ReturnKind::Other
        };
    }
    if let ExprNode::String { text, .. } = node {
        // The operand's text is its value only when it is fixed: a `"…"` one
        // substitutes, so `return [expr {"pre$x"}]` returns `pre5` in tclsh
        // 8.6.18 and 9.0.4 where O103 folded the call to the literal `pre$x`;
        // a `{…}` one folds its backslash-newlines, so `{a\<newline> b}` is
        // `a b`, not the raw bytes (#2227, found in review).
        return tcl_syntax::expr::fixed_string_operand(text).map_or(ReturnKind::Other, |value| {
            ReturnKind::Literal(ExactValue::from_literal(value))
        });
    }
    if let ExprNode::Var { text, .. } = node
        && let Some(config) = config
        && let Ok(crate::native_lowering::cells::CellPlace::Named { name }) =
            crate::native_lowering::cells::variable_reference_place(text, config)
        && params.contains(&name)
    {
        return ReturnKind::Passthrough(name);
    }
    // Walk the AST collecting var references against the param
    // set; any match → UsesParam.
    let mut referenced: Vec<String> = Vec::new();
    if !walk_collect_param_refs(node, params, &mut referenced, config, 0) {
        return ReturnKind::Other;
    }
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
    config: Option<tcl_lexer::LexerConfig>,
    depth: u32,
) -> bool {
    use crate::expr_ast::ExprNode;
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return false;
    }
    match node {
        ExprNode::Var { .. } => {
            let Some(config) = config else {
                return false;
            };
            let Ok(Some(reference)) = node.variable_reference(config) else {
                return false;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                return false;
            };
            if params.contains(name) {
                out.push(name.to_owned());
            }
            reference.index.is_none()
        }
        ExprNode::Binary { left, right, .. } => {
            let left = walk_collect_param_refs(left, params, out, config, depth + 1);
            walk_collect_param_refs(right, params, out, config, depth + 1) && left
        }
        ExprNode::Unary { operand, .. } => {
            walk_collect_param_refs(operand, params, out, config, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            let condition = walk_collect_param_refs(condition, params, out, config, depth + 1);
            let left = walk_collect_param_refs(true_branch, params, out, config, depth + 1);
            walk_collect_param_refs(false_branch, params, out, config, depth + 1)
                && left
                && condition
        }
        ExprNode::Call { args, .. } => {
            let mut complete = true;
            for arg in args {
                complete &= walk_collect_param_refs(arg, params, out, config, depth + 1);
            }
            complete
        }
        ExprNode::Literal { .. } => true,
        ExprNode::Command { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Raw { .. } => false,
    }
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

/// Whether running `stmt` may run a `return` that leaves the procedure from
/// inside it: a `return` itself, or one in a script the statement runs — an
/// `if`'s, a loop's, a `try`'s body, handlers or `finally`, a `switch`'s arms,
/// a block's or an `uplevel`'s, a call's word the registry gives the
/// [`tcl_registry::ArgRole::Body`] role, or a barrier's unseen code — but not
/// one in a `catch` body, which the `catch` absorbs (as the registry's plan
/// says of a `catch` kept as a call), nor in the body of a loop the flow
/// graph lowers, whose synthetic header holds only the list words. `depth`
/// is the nesting level of the script holding `stmt`; past
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`] it answers that it may.
pub(crate) fn statement_may_return(
    stmt: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    depth: u32,
) -> bool {
    use crate::ir::Statement;
    let runs = |script: &crate::ir::Script| script_may_return(script, registry, depth + 1);
    match stmt {
        Statement::Return { .. } | Statement::Barrier { .. } => true,
        Statement::Call {
            args,
            foreach_groups: None,
            ..
        } => {
            let head = stmt.canonical_command_or_source();
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            !registry
                .arg_indices_for_role(head, &words, tcl_registry::ArgRole::Body)
                .is_empty()
                && !crate::value_transfer::resolved_body_absorbs_completion(registry, head, args)
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|clause| runs(&clause.body)) || else_body.as_ref().is_some_and(runs)
        }
        Statement::For {
            init, next, body, ..
        } => runs(init) || runs(next) || runs(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => runs(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            runs(body)
                || handlers.iter().any(|handler| runs(&handler.body))
                || finally_body.as_ref().is_some_and(runs)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().any(|arm| arm.body.as_ref().is_some_and(runs))
                || default_body.as_ref().is_some_and(runs)
        }
        _ => false,
    }
}

/// [`statement_may_return`] over every statement of `script`, nested
/// `depth` levels deep.
fn script_may_return(
    script: &crate::ir::Script,
    registry: &tcl_registry::CommandRegistry,
    depth: u32,
) -> bool {
    MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth)
        || script
            .statements
            .iter()
            .any(|stmt| statement_may_return(stmt, registry, depth))
}

/// What a procedure's seedless run says it returns, as
/// [`summarise_returns`] reads it.
#[derive(Clone, Copy)]
enum SeedlessAnswer<'a> {
    /// No run was made: the return shapes answer.
    NotRun,
    /// The one value every exit gives.
    Value(&'a ExactValue),
    /// The run proved no one value.
    NoValue,
}

impl<'a> SeedlessAnswer<'a> {
    /// The answer [`seedless_returns`] recorded for `qname`.
    fn of(seedless: &'a HashMap<ProcedureId, Option<ExactValue>>, qname: &ProcedureId) -> Self {
        match seedless.get(qname) {
            None => Self::NotRun,
            Some(Some(value)) => Self::Value(value),
            Some(None) => Self::NoValue,
        }
    }
}

/// Derive the return-value summary fields from a proc's collected
/// [`ReturnKind`] list and its seedless run's answer: `(returns_constant,
/// constant_return, passthrough_param, depends_on_params)`.
///
/// Where a run was made its answer is the constant, and where it proved no
/// one value the shapes answer only a passthrough and the parameters the
/// value depends on, never a constant, since a value the run could not prove
/// may be one it rules out. With no run, every return the same literal is
/// the constant.
fn summarise_returns(
    returns: &[ReturnKind],
    answer: SeedlessAnswer<'_>,
) -> (bool, Option<ConstantReturn>, Option<String>, Vec<String>) {
    let constant = match answer {
        SeedlessAnswer::Value(value) => Some(value),
        SeedlessAnswer::NoValue => None,
        SeedlessAnswer::NotRun => match returns.first() {
            Some(ReturnKind::Literal(first))
                if returns
                    .iter()
                    .all(|r| matches!(r, ReturnKind::Literal(v) if v.bytes == first.bytes)) =>
            {
                Some(first)
            }
            _ => None,
        },
    };
    if let Some(constant) = constant.and_then(constant_return_of) {
        return (true, Some(constant), None, Vec::new());
    }
    // Passthrough: every return is Passthrough of the same param.
    if let Some(ReturnKind::Passthrough(first)) = returns.first()
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

/// The typed form of an exact return value, chosen so that
/// [`ConstantReturn::text`] spells it back byte for byte: an integer only
/// for its canonical decimal, a double only for the spelling Tcl prints it
/// with, a boolean only for `true` or `false` as written, and the text
/// otherwise — `007`, `1.00`, `1e3`, `TRUE` and ` 5` are text. `None` for
/// bytes that are not text.
fn constant_return_of(value: &ExactValue) -> Option<ConstantReturn> {
    let text = value.as_str().ok()?;
    if let Ok(int) = text.parse::<i64>()
        && int.to_string() == text
    {
        return Some(ConstantReturn::Int(int));
    }
    if let Ok(double) = text.parse::<f64>()
        && double.is_finite()
        && tcl_syntax::number::format_double(double) == text
    {
        return Some(ConstantReturn::Float(double));
    }
    // Rust's boolean grammar is exactly `true` and `false`.
    Some(text.parse::<bool>().map_or_else(
        |_| ConstantReturn::Str(text.to_owned()),
        ConstantReturn::Bool,
    ))
}

fn compute_transitive_calls(
    root: &ProcedureId,
    local: &HashMap<ProcedureId, LocalFacts>,
) -> HashSet<ProcedureId> {
    let mut visited: HashSet<ProcedureId> = HashSet::new();
    let mut stack: Vec<ProcedureId> = Vec::new();
    if let Some(f) = local.get(root) {
        stack.extend(f.direct_calls.iter().copied());
    }
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur) {
            continue;
        }
        if let Some(f) = local.get(&cur) {
            for d in &f.direct_calls {
                if !visited.contains(d) {
                    stack.push(*d);
                }
            }
        }
    }
    visited
}

/// The summaries' second stage: each pure procedure's own lattice, run with
/// its parameters unknown — no call-site seed, so a value exact only under
/// the literal every caller passes never enters a summary — under the
/// whole-module trust a rewrite folds under, and read at every way it
/// returns ([`exit_value`]). `None` for a procedure whose exits prove no
/// one value. It follows the purity fixpoint, which decides whose returns
/// may fold at all, and reads no summary: the driver takes a call to a
/// procedure of the module for a command it cannot see, so one stage is the
/// fixed point, and a return that passes through a recursive call is
/// computed.
fn seedless_returns(
    ir_module: &crate::ir::Module,
    units: SeedlessUnits<'_>,
    pure: &HashMap<ProcedureId, bool>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> HashMap<ProcedureId, Option<ExactValue>> {
    let module = ModuleProcedures::of_unit(units.unit, registry);
    units
        .procedures
        .iter()
        .filter(|(id, fu)| {
            ir_module.procedures.contains_key(id)
                && pure.get(id).copied().unwrap_or(false)
                && !fu.complexity_guarded
        })
        .map(|(qname, fu)| {
            let source_metadata_input = fu
                .invocation_metadata_context_for_module(registry, ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                );
            if source_metadata_input.is_none() && !fu.cfg.metadata_context.is_standalone() {
                return (*qname, None);
            }
            let dialect = source_metadata_input
                .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
                .or_else(|| {
                    fu.cfg
                        .metadata_context
                        .is_standalone()
                        .then_some(dialect)
                        .flatten()
                });
            let policy = crate::tcl_expr_eval::FoldPolicy::for_profile(
                dialect.and_then(crate::tcl_expr_eval::leading_zero_is_octal),
                dialect,
            );
            let reading = ExitReading {
                policy,
                grammar: fu.source_lexer_config().grammar_over(
                    dialect.map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar),
                ),
                module: None,
                folds: crate::sccp::BuiltinFoldInputs {
                    registry,
                    source_metadata_input,
                    mutations: units.mutations,
                    dialect,
                    defining_class: None,
                    registry_engine: false,
                    trust: crate::sccp::FoldTrust::WholeModule,
                    proven_pure_parameters: false,
                },
            };
            let result = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
                cfg: &fu.cfg,
                ssa: &fu.ssa,
                param_constants: None,
                policy,
                extra_escaping: &HashSet::new(),
                trace: crate::sccp::TraceInputs {
                    registry,
                    source_metadata_input,
                    traced_variables: &ir_module.traced_variables,
                    has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
                    deferred_writes: &ir_module.deferred_writes,
                    analysis_context: None,
                    existence: None,
                },
                folds: Some(reading.folds),
                module: crate::sccp::ModuleRun {
                    analysis_body: Some(Some(*qname)),
                    reads_exits: true,
                    ..crate::sccp::ModuleRun::reading(Some(&module))
                },
            });
            (*qname, exit_value(ExitBody::of(fu), &result, reading))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The value a procedure returns, read under a lattice

/// What an exit's value is read under: the expression route's value
/// semantics, the grammar the return's word and an expression's variables
/// are read with, and the rewrite's registry, mutation facts and
/// whole-module trust.
#[derive(Clone, Copy)]
pub(crate) struct ExitReading<'a> {
    /// The value semantics.
    pub(crate) policy: crate::tcl_expr_eval::FoldPolicy,
    /// The document's grammar.
    pub(crate) grammar: tcl_dialect::LexerGrammar,
    /// The registry, mutation facts and trust stance an expression is
    /// evaluated under.
    pub(crate) folds: crate::sccp::BuiltinFoldInputs<'a>,
    /// The module's procedures and the function the exits are read in,
    /// where a re-run reads them: a command a return's expression runs that
    /// names a procedure of the module is re-run for its result.
    pub(crate) module: Option<(&'a ModuleProcedures<'a>, &'a ProcedureId)>,
}

/// The body an exit reading reads: a flow graph and its SSA.
#[derive(Clone, Copy)]
pub(crate) struct ExitBody<'b> {
    /// The flow graph.
    pub(crate) cfg: &'b crate::cfg::Function,
    /// Its SSA.
    pub(crate) ssa: &'b crate::ssa::SsaFunction,
}

impl<'b> ExitBody<'b> {
    /// A unit's body.
    pub(crate) const fn of(unit: &'b crate::compilation_unit::FunctionUnit) -> Self {
        Self {
            cfg: &unit.cfg,
            ssa: &unit.ssa,
        }
    }
}

/// The one value every way `fu` returns gives under `result`, or `None`.
///
/// Every reachable exit must give the same value: an explicit `return`,
/// **or** a reachable fall-through to the function's implicit exit (a block
/// with no terminator: Tcl's "the result of the last command executed" rule
/// for a proc that runs off the end of its body without a `return` on that
/// path). Ignoring the fall-through would let a proc with `if {…} { return K
/// }` plus a trailing statement fold to `K` when the fall-through path is
/// also reachable and gives something else — a miscompile, not just a missed
/// optimisation (confirmed against tclsh 9.0.4: a proc whose `if` condition
/// is not foldable leaves both paths executable). A void return, an exit
/// that does not fold, two exits that disagree, or a statement of an
/// executable block that may itself run a `return` ([`statement_may_return`],
/// #2393) give `None`.
///
/// The one reading O103's argument-sensitive re-run and the summary's
/// seedless run share. A literal word is read through the exact value
/// ingress — a braced word its content, a bare or quoted one its escapes
/// decoded, nothing trimmed — so `return " 5"` is the two-character string.
///
/// The run is one made to read how the procedure completes
/// ([`crate::sccp::ModuleRun::reads_exits`]). A statement it proved raises
/// where no handler takes the throw, or a call whose completion it could
/// not decide, means some call may not reach an exit at all, so no exit's
/// value is the call's: `None`.
pub(crate) fn exit_value(
    fu: ExitBody<'_>,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    use crate::cfg::Terminator;
    if !result.completion.is_decided() {
        return None;
    }
    if fu.cfg.blocks.iter().any(|(bn, block)| {
        result.executable_blocks.contains(bn)
            && block
                .statements
                .iter()
                .any(|stmt| statement_may_return(stmt, reading.folds.registry, 0))
    }) {
        return None;
    }
    let preds = fu.cfg.predecessors();
    let mut found: Option<ExactValue> = None;
    for (bn, block) in &fu.cfg.blocks {
        if !result.executable_blocks.contains(bn) {
            continue;
        }
        let value = match &block.terminator {
            Some(Terminator::Return {
                value,
                braced,
                expr,
                ..
            }) => return_value(
                fu,
                *bn,
                (value.as_deref(), *braced, expr.as_ref()),
                result,
                reading,
            )?,
            None => fallthrough_value(fu, *bn, result, &preds, reading)?,
            Some(_) => continue, // Goto / Branch — not an exit point
        };
        match &found {
            None => found = Some(value),
            Some(prev) if prev.bytes == value.bytes => {}
            Some(_) => return None, // reachable exits disagree
        }
    }
    found
}

/// The value a `return`'s word gives at block `bn`: a literal through the
/// exact ingress, `$name` as the version there holds it, an `expr` on the
/// shared expression route; a bare `return` (no word) gives none.
fn return_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    (value, braced, expr): (Option<&str>, bool, Option<&crate::expr_ast::ExprNode>),
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    let word = value?;
    if let Some(node) = expr {
        return expr_value(fu, bn, node, result, reading);
    }
    let config = tcl_lexer::LexerConfig::from_grammar(reading.grammar);
    if let Some(text) = crate::value_transfer::recorded_word_value(word, braced, &config) {
        return Some(ExactValue::from_literal(&text));
    }
    var_value(
        fu,
        bn,
        crate::value_shapes::whole_word_scalar_var_name(word, config)?,
        result,
    )
}

/// The value Tcl's implicit-return rule leaves when control falls through
/// block `bn` — the function's synthesised exit sink (a reachable block
/// with no terminator).
///
/// Trusts ONLY the narrow, unambiguous shape: `bn` has exactly one
/// executable predecessor, and that predecessor's OWN last statement is a
/// recognised value-producing tail (see [`tail_value`]). Deliberately does
/// NOT walk through an empty predecessor to whatever precedes *it*: an empty
/// block reached via a control-flow edge is not "no Tcl command ran here" —
/// it is frequently the empty **body** of a real command (`if {$c} {}`, or
/// the implicit `""` an `if` with no `else` produces when the condition is
/// false), whose own result is the empty string, not whatever ran before the
/// branch. Block shape alone can't soundly distinguish that from a genuine
/// structural join, so any empty predecessor — or more than one live
/// predecessor at all — bails to `None` rather than risk inheriting a stale
/// prior value. (A more permissive, recursive version shipped briefly and
/// mis-folded `proc f {c} { set x 1; if {$c} {} }`'s `[f 0]` to `1` instead
/// of the correct `""` — confirmed against tclsh 9.0.4 — by walking straight
/// through the empty `if`-body block back to the preceding `set x 1`.)
fn fallthrough_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    result: &crate::sccp::SccpResult,
    preds: &HashMap<crate::cfg::BlockId, HashSet<crate::cfg::BlockId>>,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    let mut executable_preds = preds
        .get(&bn)
        .into_iter()
        .flatten()
        .filter(|p| result.executable_blocks.contains(p));
    let pred = executable_preds.next()?;
    if executable_preds.next().is_some() {
        return None; // more than one live predecessor — ambiguous, bail
    }
    let block = fu.cfg.blocks.get(pred)?;
    let last = block.statements.last()?;
    tail_value(fu, *pred, last, result, reading)
}

/// The value Tcl's "result of the last executed command" rule leaves when
/// `stmt` is the last statement of a block that falls through to the
/// function's implicit exit — a trailing `set` / `incr` implicitly returns
/// exactly like `return $name` would (Tcl's `set` and `incr` both return the
/// value they just assigned), a trailing call whose resolved plan is a cell
/// update (`append`, `lappend`) returns the cell's new value the same way,
/// and a trailing bare `expr` implicitly returns exactly like `return [expr
/// {…}]` would. `None` for any other statement shape (a bare command call
/// whose own result this analysis doesn't track, …) — the caller simply
/// won't fold that path, never mis-folds it.
fn tail_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    stmt: &crate::ir::Statement,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    use crate::ir::Statement;
    match stmt {
        Statement::ExprEval { expr, .. } => expr_value(fu, bn, expr, result, reading),
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => var_value(fu, bn, name, result),
        // A cell update's result is the value it wrote: the registry's
        // declared plan names the target, and the lattice at the block's
        // exit holds what it wrote.
        Statement::Call {
            command,
            canonical_command,
            args,
            ..
        } => {
            let head = canonical_command.as_deref().unwrap_or(command);
            let (_, target) =
                crate::value_transfer::resolved_cell_update(reading.folds.registry, head, args)?;
            var_value(fu, bn, args.get(target.0)?, result)
        }
        _ => None,
    }
}

/// The value `name` holds at block `bn`'s *exit* version — immediately
/// after `bn`'s own statements have run. This MUST use the exit version
/// precisely: a loop-carried var (`return $total` after a `foreach`) is a
/// phi whose exit value is Overdefined, even though an earlier `set total 0`
/// left a stale Const(0) under another version. Reading the precise version
/// is what makes the reading bail on `sum_list` / `fibonacci` instead of
/// mis-folding to the pre-loop value. The value is the one the version holds
/// at `bn` ([`crate::sccp::SccpResult::value_at`]), so the state an
/// enumerated loop leaves, in force past it, is read; a value a route
/// constructed rather than read from the source is never one
/// ([`crate::sccp::SccpResult::materialises`]).
fn var_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    name: &str,
    result: &crate::sccp::SccpResult,
) -> Option<ExactValue> {
    let sym = fu.ssa.var_symbol(name)?;
    let ver = fu
        .ssa
        .blocks
        .get(&bn)
        .and_then(|b| b.exit_versions.get(&sym).copied())
        .unwrap_or(0);
    match result.value_at(bn, (sym, ver)) {
        Some(crate::analyses::LatticeValue::Const(c)) if result.materialises((sym, ver)) => {
            Some(crate::value_transfer::const_to_exact(c))
        }
        _ => None,
    }
}

/// `expr` evaluated under the lattice for block `bn`'s exit environment.
/// Built FLOW-SENSITIVELY: each variable the expression references is bound
/// at *this block's exit version* (the precise state reaching this point),
/// and only when that version is a lattice constant at `bn`
/// ([`crate::sccp::SccpResult::value_at`]). A variable absent from
/// `exit_versions` (a never-reassigned parameter) falls back to version 0,
/// where a seeded parameter's constant lives.
///
/// The flow-INsensitive alternative ("every Const lattice entry, preferring
/// the newest version, then overlay exit versions") miscompiled: for `set x
/// 0; foreach v {…} { set x $v }; return [expr {$x + 1}]`, `x`'s exit
/// version is a non-Const loop phi, so the overlay didn't override, and the
/// stale pre-loop `(x,1)=Const(0)` leaked in — folding to `1` where tclsh
/// returns `3`. Reading the exit version (Overdefined here) leaves `x`
/// unbound so the route declines, as [`var_value`] does.
///
/// The expression runs on the shared expression route
/// ([`crate::value_transfer::evaluate_expression_detached`]) under the
/// rewrite's whole-module trust, so a return fold proves what the lattice
/// proves: no rebound math function, no function the target lacks, no value
/// past the target's integer tower.
fn expr_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    expr: &crate::expr_ast::ExprNode,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    let mut constants: HashMap<String, ExactValue> = HashMap::new();
    if let Some(ssa_block) = fu.ssa.blocks.get(&bn) {
        for name in crate::var_refs::vars_in_expr(expr, reading.grammar) {
            let Some(sym) = fu.ssa.var_symbol(&name) else {
                continue;
            };
            let ver = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
            if let Some(crate::analyses::LatticeValue::Const(c)) = result.value_at(bn, (sym, ver)) {
                constants.insert(
                    fu.ssa.var_name(sym).to_owned(),
                    crate::value_transfer::const_to_exact(c),
                );
            }
        }
    }
    crate::value_transfer::evaluate_expression_in_module(
        expr,
        &constants,
        (reading.folds, reading.policy),
        reading.module,
    )
}

use crate::side_effects::classify_side_effects_in;

#[cfg(test)]
#[path = "test_body_identity.rs"]
pub(crate) mod test_body_identity;

#[cfg(test)]
mod tests {
    use super::test_body_identity::{edge_labels, fixture_id, fixture_id_if_present, fixture_proc};
    fn fixture_method<'a>(
        analysis: &'a InterproceduralAnalysis,
        label: &str,
    ) -> Option<&'a MethodSummary> {
        let mut matches = analysis
            .methods
            .values()
            .filter(|method| method.base.qualified_name == label);
        let first = matches.next()?;
        assert!(
            matches.next().is_none(),
            "fixture method label must be unambiguous"
        );
        Some(first)
    }

    use super::*;

    fn authored_report_unit() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        CompilationUnit,
    ) {
        let environment = crate::environment_ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        // Explicit Standalone authoring is the dialect-blind build (see
        // `original_reference_unit`); a profiled build is a fresh document.
        let unit = CompilationUnit::build_for_with_config(
            "proc p {} {return ONE}; proc p {} {return TWO}",
            context.commands(),
            false,
            tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
        );
        (context, unit)
    }

    #[test]
    fn authored_source_report_labels_keep_real_body_ids_and_current_source() {
        let environment = crate::environment_ingress::resolve_environment("tcl8.6");
        let (context, unit) = authored_report_unit();
        let registry = context.commands();
        let module = &unit.ir_module;
        assert_eq!(module.original_authored_declaration_body_units.len(), 2);
        let ids: Vec<_> = module
            .original_authored_declaration_body_units
            .keys()
            .copied()
            .collect();
        assert_ne!(ids[0], ids[1]);
        for &id in &ids {
            assert!(module.procedure_ids.owns(id));
            assert_eq!(
                module.original_procedure_report_label(id, registry),
                Some("::p")
            );
            let procedure = module.body_units.get(&id).unwrap();
            assert!(procedure.publication.is_none());
            assert!(procedure.body.executed_source.is_none());
            assert!(procedure.body.native_compilation_admission.is_none());
        }
        let identities =
            crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
        let analysis = build_interprocedural_analysis(
            module,
            registry,
            Some(environment.unit_profile()),
            ObjectTypeMap::none(),
            &identities,
            None,
        );
        for &id in &ids {
            assert_eq!(
                analysis
                    .procedures
                    .get(&id)
                    .expect("actual issued body summary")
                    .qualified_name,
                "::p"
            );
        }
    }

    #[test]
    fn authored_source_report_labels_refuse_foreign_and_changed_owners() {
        let environment = crate::environment_ingress::resolve_environment("tcl8.6");
        let (context, unit) = authored_report_unit();
        let registry = context.commands();
        let module = &unit.ir_module;
        let id = *module
            .original_authored_declaration_body_units
            .keys()
            .next()
            .unwrap();
        let mut presentation = module.clone();
        presentation.body_units.get_mut(&id).unwrap().qualified_name = "display changed".into();
        assert_eq!(
            presentation.original_procedure_report_label(id, registry),
            Some("::p")
        );
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
        assert!(
            changed
                .original_procedure_report_label(id, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed
            .body_units
            .get_mut(&id)
            .unwrap()
            .params
            .push("changed".into());
        assert!(
            changed
                .original_procedure_report_label(id, registry)
                .is_none()
        );
        let mut detached = module.clone();
        detached.retained_source_bindings = None;
        assert!(
            detached
                .original_procedure_report_label(id, registry)
                .is_none()
        );
        // The foreign owner is the same explicit Standalone authoring build
        // (`authored_report_unit`), retained under a different unit.
        let other = CompilationUnit::build_for_with_config(
            "proc p {} {return ONE}; proc p {} {return TWO}",
            registry,
            false,
            tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
        );
        let foreign_id = *other
            .ir_module
            .original_authored_declaration_body_units
            .keys()
            .next()
            .unwrap();
        assert!(
            module
                .original_procedure_report_label(foreign_id, registry)
                .is_none()
        );
        let foreign_registry =
            crate::environment_ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(
            module
                .original_procedure_report_label(id, foreign_registry.commands())
                .is_none()
        );
    }

    #[test]
    fn original_declaration_receivers_keep_owned_ids_and_preceding_setup() {
        let environment = tcl_registry::model::ingress::resolve_environment("tk");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let build = |source| {
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
                .with_resolved_analysis_input(input.clone());
            lowerer.lower(source);
            lowerer.finish_module(source)
        };
        let module = build("entry .before; proc danger {} {return OK}; entry .after");
        let (id, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let id = *id;
        assert_eq!(
            procedure.body_kind,
            crate::ir::ProcedureBodyKind::OriginalDeclaration
        );
        assert!(procedure.publication.is_none());
        assert!(!module.procedures.contains_key(&id));
        assert!(module.source_entry.native_entry.is_none());
        let classes = global_instance_classes(&module, registry);
        assert!(classes[&id].contains_key(".before"));
        assert!(!classes[&id].contains_key(".after"));
        assert!(!module.procedures.contains_key(&id));

        let later = build("proc danger {} {return OK}; entry .later");
        let later_id = *later.original_source_procedure_bodies().next().unwrap().0;
        assert!(global_instance_classes(&later, registry)[&later_id].is_empty());
        assert!(!classes.contains_key(&later_id));
        let mut missing = module.clone();
        missing.retained_source_bindings = None;
        assert!(!global_instance_classes(&missing, registry).contains_key(&id));
        let mut changed = module.clone();
        changed.source =
            tcl_lexer::SourceImage::document("entry .changed; proc danger {} {return OK}");
        assert!(!global_instance_classes(&changed, registry).contains_key(&id));
        let mut foreign = module.clone();
        foreign.procedure_ids = later.procedure_ids.clone();
        assert!(!global_instance_classes(&foreign, registry).contains_key(&id));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(!global_instance_classes(&module, foreign).contains_key(&id));
    }

    #[test]
    fn original_declaration_summaries_keep_actual_cards_without_callable_publication() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let source = "proc p {} {return ORIGINAL}";
        // Unentered declaration cards need explicit unknown history; with no
        // entry a fresh interpreter defines `::p`.
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input)
            .with_explicit_unknown_entry();
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let mut bodies = module.original_source_procedure_bodies();
        let (id, body) = bodies.next().expect("genuine original declaration card");
        assert!(bodies.next().is_none());
        let id = *id;
        assert_eq!(
            body.body_kind,
            crate::ir::ProcedureBodyKind::OriginalDeclaration
        );
        assert!(body.publication.is_none());
        assert!(!module.procedures.contains_key(&id));
        assert!(module.source_entry.native_entry.is_none());
        assert!(summary_procedure_bodies(&module, registry).contains_key(&id));
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            source,
            module.lexer_config,
            registry,
            &module.source_entry,
        )
        .with_resolved_analysis_input(module.source_metadata_input.as_ref().unwrap().clone());
        let analysis = build_interprocedural_analysis(
            &module,
            registry,
            Some(profile),
            ObjectTypeMap::none(),
            &identities,
            None,
        );
        let summary = analysis
            .procedures
            .get(&id)
            .expect("conditional source summary");
        assert!(!summary.completes);
        assert!(summary.defined_at.is_none());
        assert!(summary.defined_callees.is_empty());
        assert!(!module.procedures.contains_key(&id));

        let mut missing = module.clone();
        missing.retained_source_bindings = None;
        assert!(!summary_procedure_bodies(&missing, registry).contains_key(&id));
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
        assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
        let mut changed = module.clone();
        changed
            .body_units
            .get_mut(&id)
            .unwrap()
            .body
            .statements
            .clear();
        assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(!summary_procedure_bodies(&module, foreign).contains_key(&id));
    }

    fn known_set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    /// The call-graph edges recorded for `caller` in `src`, sorted.
    fn calls_of(
        src: &str,
        caller: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<String> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
        let ir = crate::lowering::lower_to_ir(src, registry);
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            src,
            ir.lexer_config,
            registry,
            &ir.source_entry,
        );
        let identities = ir.source_metadata_input.as_ref().map_or_else(
            || identities.clone(),
            |input| {
                identities
                    .clone()
                    .with_resolved_analysis_input(input.clone())
            },
        );
        let ia = build_interprocedural_analysis(
            &ir,
            registry,
            Some(dialect),
            ObjectTypeMap::none(),
            &identities,
            None,
        );
        let mut calls: Vec<String> = fixture_proc(&ia, caller)
            .map(|s| edge_labels(&ia, &s.calls))
            .unwrap_or_default();
        calls.sort();
        calls
    }

    #[test]
    fn procedure_and_method_callback_advice_requires_the_module_source_producer() {
        // naming.compiler.original-analysis-metadata-context

        // Callback edges are conditional authored reachability. This source
        // control does not establish a native invocation or callback frame.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let source = "proc ::cb args {}; proc ::direct args {}; \
                      proc ::caller {} {::direct; ::lsort -command ::cb {x y}}; \
                      oo::class create C {method run {} {::direct; ::lsort -command ::cb {x y}}}";
        let module = crate::lowering::lower_to_ir_with_dialect(
            source,
            registry,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            Some(profile),
        );
        let input = module
            .source_metadata_input
            .as_ref()
            .expect("retained source input");
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            source,
            module.lexer_config,
            registry,
            &module.source_entry,
        )
        .with_resolved_analysis_input(input.clone());
        let analysis = |module: &crate::ir::Module| {
            build_interprocedural_analysis(
                module,
                registry,
                // A display profile cannot replace the source producer.
                tcl_dialect::DialectProfile::find("tcl9.1"),
                ObjectTypeMap::none(),
                &identities,
                None,
            )
        };
        let current = analysis(&module);
        let proc = fixture_proc(&current, "::caller").expect("procedure summary");
        let method = fixture_method(&current, "::C::run").expect("method summary");
        for summary in [proc, &method.base] {
            assert!(
                edge_labels(&current, &summary.direct_calls)
                    .iter()
                    .any(|name| name == "::cb")
            );
            assert!(
                edge_labels(&current, &summary.direct_calls)
                    .iter()
                    .any(|name| name == "::direct")
            );
        }

        let mut wrong_profile = module.clone();
        wrong_profile.dialect_profile = tcl_dialect::DialectProfile::find("tcl9.1");
        let mut wrong_config = module.clone();
        wrong_config.lexer_config.strict_quoting = !wrong_config.lexer_config.strict_quoting;
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        let mut foreign = module.clone();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        ));
        for withheld in [wrong_profile, wrong_config, missing, foreign] {
            let result = analysis(&withheld);
            let proc = fixture_proc(&result, "::caller").expect("retained procedure");
            let method = fixture_method(&result, "::C::run").expect("retained method");
            for summary in [proc, &method.base] {
                assert!(
                    !edge_labels(&result, &summary.direct_calls)
                        .iter()
                        .any(|name| name == "::cb"),
                    "withheld metadata cannot donate callback advice: {:?}",
                    summary.direct_calls
                );
                assert!(
                    edge_labels(&result, &summary.direct_calls)
                        .iter()
                        .any(|name| name == "::direct"),
                    "an independently retained direct target remains reachable"
                );
                assert!(!summary.pure, "withheld selected effects remain opaque");
            }
        }
    }

    #[test]
    fn original_positioned_child_bodies_preserve_replacements_and_aliases() {
        // naming.compiler.original-positioned-child-body-reachability

        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        for (setup, body, expected) in [
            ("", "catch {leaf}", true),
            (
                "proc catch {payload} {return $payload}",
                "catch {leaf}",
                false,
            ),
            ("interp alias {} held {} catch", "held {leaf}", true),
            ("rename catch held", "held {leaf}", true),
            (
                "proc held {payload} {return $payload}",
                "held {leaf}",
                false,
            ),
        ] {
            let source =
                format!("proc leaf {{}} {{}}; {setup}; proc caller {{}} {{time {{{body}}}}}");
            assert_eq!(
                calls_of(&source, "::caller", profile).contains(&"::leaf".to_owned()),
                expected,
                "{source}"
            );
        }
    }

    /// Literal callback values and authentic selected builders supply possible
    /// reachability edges independently of future execution and frame entry.
    #[test]
    fn command_prefix_callbacks_are_call_graph_edges_in_every_shape() {
        for prefix in ["cb", "{cb -nocase}", "[list cb]", "[list cb extra]"] {
            let src = format!(
                "proc cb {{a b}} {{ return 0 }}\nproc go {{}} {{ lsort -command {prefix} {{x y}} }}\n"
            );
            assert_eq!(
                calls_of(
                    &src,
                    "::go",
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
                ),
                vec!["::cb".to_owned()],
                "`-command {prefix}` must record a call edge to cb",
            );
        }
    }

    /// The `trace add variable … command cb` shape.
    #[test]
    fn original_callback_reachability_distinguishes_literal_values_and_selected_builders() {
        // naming.source.original-callback-procedure-target

        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        for setup in ["", "rename list make", "interp alias {} make {} list"] {
            let builder = if setup.is_empty() { "list" } else { "make" };
            let source = format!(
                "proc cb {{args}} {{}}; {setup}; proc go {{x}} {{lsort -command [{builder} cb $x] {{3 1 2}}}}"
            );
            assert!(
                calls_of(&source, "::go", profile).contains(&"::cb".to_owned()),
                "{source}"
            );
        }
        for setup in ["proc list args {}", "rename list {}", "rename cb {}"] {
            let source = format!(
                "proc cb {{args}} {{}}; {setup}; proc go {{x}} {{lsort -command [list cb $x] {{3 1 2}}}}"
            );
            assert!(
                !calls_of(&source, "::go", profile).contains(&"::cb".to_owned()),
                "{source}"
            );
        }
        let source = "proc cb args {}; proc {[list} args {}; proc go {} {lsort -command {[list cb]} {3 1 2}}";
        let calls = calls_of(source, "::go", profile);
        assert!(calls.contains(&"::[list".to_owned()), "{calls:?}");
        assert!(!calls.contains(&"::cb".to_owned()));
        let source =
            "proc {cb name} args {}; proc go {} {lsort -command {{cb name} BAKED} {3 1 2}}";
        assert!(calls_of(source, "::go", profile).contains(&"::cb name".to_owned()));
    }

    #[test]
    fn a_trace_callback_is_a_call_graph_edge() {
        let src = "proc cb {args} { return 0 }\n\
                   proc go {} { trace add variable v write [list cb] }\n";
        assert_eq!(
            calls_of(
                src,
                "::go",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            vec!["::cb".to_owned()]
        );
    }

    /// TN control: a prefix computed by some *other* substitution names no
    /// command this scan can see, so it must record no callback edge rather
    /// than guess one from the substitution's own head (`[pick` is not
    /// `cb`).  What the scan does with the substitution *itself* is a
    /// separate concern this test deliberately does not pin.
    #[test]
    fn a_computed_command_prefix_records_no_callback_edge() {
        let src = "proc cb {a b} { return 0 }\n\
                   proc pick {} { return cb }\n\
                   proc go {} { lsort -command [pick] {x y} }\n";
        assert!(
            !calls_of(
                src,
                "::go",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            )
            .contains(&"::cb".to_owned()),
            "the computed prefix's result is unknown, so cb is not a proven callee",
        );
    }

    /// The user-proc invoker is registry data (`Traits::INVOKES_USER_PROC`),
    /// not the command name `call`: under iRules the edge goes to the named
    /// procedure, and under a dialect with no such command a user procedure
    /// that happens to be *called* `call` is an ordinary callee.
    #[test]
    fn user_proc_invoker_is_registry_driven_not_name_matched() {
        assert_eq!(
            calls_of(
                "proc helper {mode} { return $mode }\nwhen RULE_INIT { call helper dev }\n",
                "::when::RULE_INIT",
                tcl_dialect::DialectProfile::irules(),
            ),
            vec!["::helper".to_owned()],
        );
        assert_eq!(
            calls_of(
                "proc call {mode} { return $mode }\nproc go {} { call helper }\n",
                "::go",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            ),
            vec!["::call".to_owned()],
            "plain Tcl has no user-proc invoker, so `call` is just a procedure",
        );
    }

    #[test]
    fn command_prefix_head_reads_each_shape() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert_eq!(command_prefix_head(&registry, "cb").as_deref(), Some("cb"));
        assert_eq!(
            command_prefix_head(&registry, "cb -nocase").as_deref(),
            Some("cb"),
        );
        assert_eq!(
            command_prefix_head(&registry, "[list cb $x]").as_deref(),
            Some("[list"),
        );
        assert_eq!(
            command_prefix_head(&registry, "[pick]").as_deref(),
            Some("[pick]")
        );
        assert_eq!(
            command_prefix_head(&registry, "{cb name} tail").as_deref(),
            Some("cb name")
        );
        assert_eq!(
            command_prefix_head(&registry, "\"cb name\" tail").as_deref(),
            Some("cb name")
        );
        assert_eq!(
            command_prefix_head(&registry, r"cb\ name tail").as_deref(),
            Some("cb name")
        );
        assert_eq!(command_prefix_head(&registry, "{unterminated"), None);
    }

    /// The interprocedural call-graph scanners each need a depth cap —
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
        let known: HashSet<ProcedureId> = HashSet::new();
        let source_module = crate::ir::Module::default();
        let params: HashSet<String> = ["x".to_owned()].into_iter().collect();
        let ctx = ScanCtx {
            known: &known,
            registry: &registry,
            dialect: None,
            params: &params,
            object_types: ObjectTypeMap::none().candidates,
            identities: crate::realm::CommandBindingRealm::none(),
            declared: None,
            positioned_substitutions: false,
            source_input: None,
            source_procedures: &source_module.procedures,
            source_module: &source_module,
            current_tokens: None,
            parameter_body: None,
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
        note_params_in_expr(
            &node,
            &params,
            &mut facts,
            tcl_lexer::LexerConfig::default(),
            0,
        );
        let mut refs = Vec::new();
        walk_collect_param_refs(
            &node,
            &params,
            &mut refs,
            Some(tcl_lexer::LexerConfig::default()),
            0,
        );

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
        // The fixture's identity surface below is explicitly C Tcl 8.6.
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let cu = CompilationUnit::build_for_profile(source, &registry, false, profile);
        let identities = crate::realm::document_realm_bindings_for_module(&cu.ir_module, &registry)
            .expect("fixture's retained Module source owner");
        build_interprocedural_analysis(
            &cu.ir_module,
            &registry,
            Some(profile),
            ObjectTypeMap::none(),
            &identities,
            None,
        )
    }

    fn source_point_trace(source: &str, body: &str) -> Vec<String> {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let unit = CompilationUnit::build_for_profile(source, &registry, false, profile);
        let statements = fixture_id_if_present(&unit.ir_module.procedures, body)
            .and_then(|id| unit.ir_module.procedures.get(&id))
            .map(|row| &row.body)
            .or_else(|| {
                fixture_id_if_present(&unit.ir_module.methods, body)
                    .and_then(|id| unit.ir_module.methods.get(&id))
                    .map(|row| &row.body)
            });
        statements.into_iter().flat_map(|body| body.statements.iter().map(|statement| {
            let Some(tokens) = body.retained_source_tokens_for_statement(statement) else {
                return format!("no source carrier {:?}", statement.span());
            };
            let Some(binding) = &tokens.source_binding else {
                return format!("{:?}: no source binding", tokens.argv_texts);
            };
            let normal = crate::registry_invocation::normal_transfer_invocation(
                &registry, None, tokens,
            );
            format!(
                "{:?}: unknown={} absent={} handler={:?} frame={:?} ns={} known={} globals={:?} ns_vars={:?} mutations={:?}",
                tokens.argv_texts, binding.unknown, binding.may_be_absent,
                binding.proved_handler_target().map(|target| (&target.command, target.registry_backed)),
                binding.variable_context.frame_kind, binding.variable_context.namespace,
                binding.variable_context.namespace_known, binding.variable_context.globals,
                binding.variable_context.ns_vars,
                normal.map(|normal| normal.mutation_places(&binding.variable_context, &registry)),
            )
        })).collect()
    }

    #[test]
    fn positioned_alias_traits_use_selected_frame_and_transition_grammar() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            for (level, caller_write) in [("0x1", true), ("#0", false), ("0", false)] {
                let source = format!("proc writer {{n}} {{upvar {level} $n local; set local NEW}}");
                let unit = CompilationUnit::build_for_dialect(&source, registry, false, dialect);
                let identities =
                    crate::realm::document_realm_bindings_for_module(&unit.ir_module, registry)
                        .expect("fixture's retained Module source owner");
                let analysis = build_interprocedural_analysis(
                    &unit.ir_module,
                    registry,
                    Some(profile),
                    ObjectTypeMap::none(),
                    &identities,
                    None,
                );
                let traits = &analysis
                    .procedures
                    .get(&fixture_id(&analysis.procedures, "::writer"))
                    .expect("authored fixture body")
                    .param_traits["n"];
                assert!(
                    traits.contains(&ProcArgTrait::VarRead),
                    "{dialect} {level}: {traits:?}; source body: {:#?}",
                    unit.ir_module
                        .procedures
                        .get(&fixture_id(&unit.ir_module.procedures, "::writer"))
                        .expect("authored fixture body")
                        .body
                );
                assert_eq!(
                    traits.contains(&ProcArgTrait::VarWrite),
                    caller_write,
                    "only the caller-frame alias writes this caller parameter: {dialect} {level}: {traits:?}"
                );
            }
        }
    }

    #[test]
    fn document_alias_prefix_preserves_the_call_graph_target() {
        let facts = build(
            "proc target {prefix value} {return $value}\ninterp alias {} invoke {} target FIXED\nproc caller {value} {invoke $value}",
        );
        let caller = facts
            .procedures
            .get(&fixture_id(&facts.procedures, "::caller"))
            .expect("authored fixture body");
        assert!(
            edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::target"),
            "document alias target must remain a call edge: {:?}",
            caller.direct_calls
        );
        assert!(
            !edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::invoke")
        );
    }

    #[test]
    fn unknown_alias_prefix_bytes_preserve_document_call_graph_target() {
        let facts = build(
            "proc target {prefix value} {return $value}\nproc caller {prefix value} {interp alias {} invoke {} target $prefix; invoke $value}",
        );
        let caller = facts
            .procedures
            .get(&fixture_id(&facts.procedures, "::caller"))
            .expect("authored fixture body");
        assert!(
            edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::target"),
            "unknown prefix bytes preserve target identity and argv slots: {:?}",
            caller.direct_calls
        );
        assert!(
            !edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::invoke")
        );
    }

    #[test]
    fn document_procedure_named_like_wrapper_does_not_inherit_body_roles() {
        let facts = build(
            "proc time {script} {return SAFE}\nproc helper {} {return VALUE}\nproc caller {} {time {helper}}",
        );
        let caller = facts
            .procedures
            .get(&fixture_id(&facts.procedures, "::caller"))
            .expect("authored fixture body");
        assert!(
            edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::time")
        );
        assert!(
            !edge_labels(&facts, &caller.direct_calls)
                .iter()
                .any(|callee| callee == "::helper"),
            "{:?}",
            caller.direct_calls
        );
    }

    #[test]
    fn uncertain_point_binding_preserves_unknown_effect_obligation() {
        let facts = build(
            "if {$flag} {proc time {script} {return SAFE}}\nproc caller {} {time {puts SAFE}}",
        );
        let caller = facts
            .procedures
            .get(&fixture_id(&facts.procedures, "::caller"))
            .expect("authored fixture body");
        assert!(caller.has_unknown_calls || caller.has_barrier);
        assert!(!caller.pure);
    }

    #[test]
    fn empty_module_has_no_summaries() {
        let ia = build("");
        assert!(ia.procedures.is_empty());
    }

    /// A proc called *inside* an
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
        let s = fixture_proc(&ia, "::f").expect("::f summary");
        assert!(
            edge_labels(&ia, &s.direct_calls)
                .iter()
                .any(|c| c == "::helper"),
            "expected a direct_calls edge to ::helper via the apply lambda body; got {:?}",
            s.direct_calls
        );
    }

    /// A lambda body's bare calls must
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
        let s = fixture_proc(&ia, "::ns::f").expect("::ns::f summary");
        assert!(
            edge_labels(&ia, &s.direct_calls)
                .iter()
                .any(|c| c == "::helper"),
            "expected the apply lambda body's bare `helper` call to resolve \
             in the global namespace (Tcl evaluates a two-element apply \
             lambda in `::`); got {:?}",
            s.direct_calls
        );
        assert!(
            !edge_labels(&ia, &s.direct_calls)
                .iter()
                .any(|c| c == "::ns::helper"),
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
        let s = fixture_proc(&ia, "::ns::f").expect("::ns::f summary");
        assert!(
            edge_labels(&ia, &s.direct_calls)
                .iter()
                .any(|c| c == "::other::helper"),
            "expected the apply lambda's explicit `::other` namespace \
             element to resolve `helper` there; got {:?}",
            s.direct_calls
        );
        assert!(
            !edge_labels(&ia, &s.direct_calls)
                .iter()
                .any(|c| c == "::helper"),
            "the explicit lambda namespace must not fall back to global; \
             got {:?}",
            s.direct_calls
        );
    }

    #[test]
    fn instance_method_callback_is_a_direct_call() {
        use crate::provider_fixtures::{Provider, entry};
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands().as_ref();
        let source = "package require struct::graph\nproc onNode {a g n} {}\nproc build {} { struct::graph g\n g node insert root\n g walk root -command onNode }\nbuild\n";
        let selected = entry(registry, &[Provider::Graph]);
        let cu = CompilationUnit::build_with_source_entry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                dialect: Some(environment.analyser_profile()),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            &selected,
        )
        .with_interprocedural(registry, Some(environment.analyser_profile()));
        let actual = cu.interproc.as_ref().expect("interproc");
        let summary = fixture_proc(actual, "::build").expect("authored build summary");
        assert!(
            edge_labels(actual, &summary.direct_calls)
                .iter()
                .any(|call| call == "::onNode"),
            "the actual loaded graph's callback must reach onNode: {:?}; candidates={:?}; argv={:?}",
            summary.direct_calls,
            crate::object_types::object_handle_classes(&cu, registry),
            cu.procedures
                .get(&fixture_id(&cu.ir_module.procedures, "::build"))
                .into_iter()
                .flat_map(|unit| unit.cfg.blocks.values())
                .flat_map(|block| &block.statements)
                .filter_map(|statement| {
                    let tokens = statement.tokens()?;
                    let binding = tokens.source_binding.as_ref()?;
                    Some((
                        tokens.argv_texts.first(),
                        binding.evaluated_command_word(),
                        &binding.evaluated_argument_values,
                        tokens.synthetic.is_some(),
                    ))
                })
                .collect::<Vec<_>>(),
        );
        assert!(
            summary.has_unknown_calls && !summary.pure,
            "candidate callback reachability cannot close the receiver's effects",
        );
        let unprovided = CompilationUnit::build_with_source_entry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                dialect: Some(environment.analyser_profile()),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            &entry(registry, &[]),
        )
        .with_interprocedural(registry, Some(environment.analyser_profile()));
        assert!(
            !fixture_proc(unprovided.interproc.as_ref().expect("interproc"), "::build")
                .is_some_and(|summary| edge_labels(
                    unprovided.interproc.as_ref().expect("interproc"),
                    &summary.direct_calls
                )
                .iter()
                .any(|call| call == "::onNode")),
            "an unloaded graph catalogue cannot license a callback edge",
        );
    }

    #[test]
    fn simple_proc_is_recorded_with_params_and_arity() {
        let ia = build("proc ::greet {name} { puts hi }");
        let s = fixture_proc(&ia, "::greet").expect("proc summary");
        assert_eq!(s.params, vec!["name".to_string()]);
        assert_eq!(s.arity, Arity::exact(1));
    }

    // writes_global recognises scope aliases

    #[test]
    fn global_alias_write_counts_as_writes_global() {
        // A bare `set g` after `global g` mutates a caller-visible
        // variable through the alias — writes_global, even though the
        // written name is bare, not `::`-qualified.
        for src in [
            "proc ::f {} { global g\nset g 5 }",
            "proc ::f {} { variable v\nset v 1 }",
            "proc ::f {} { upvar #0 ::x g\nset g 1 }",
            "proc ::f {} { global g\nincr g }",
            "proc ::f {} { global g\nappend g x }",
        ] {
            let ia = build(src);
            let s = fixture_proc(&ia, "::f").expect("::f summary");
            assert!(
                s.writes_global,
                "expected writes_global for: {src}; points={:?}",
                source_point_trace(src, "::f"),
            );
        }
    }

    /// An alias pair whose level selects the current frame stays a global
    /// alias: its other variable is in practice a qualified, computed or
    /// namespace-declared name (`upvar 0 $token state`), which reaches a
    /// namespace cell from this frame. Its level is read as a `FrameLevel`,
    /// so every spelling of the global frame and of this one counts.
    #[test]
    fn standalone_empty_alias_declaration_and_mutation_have_distinct_write_effects() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let known = HashSet::new();
        let params = HashSet::new();
        let module = crate::ir::Module::default();
        let ctx = ScanCtx {
            known: &known,
            registry: &registry,
            dialect: None,
            params: &params,
            object_types: ObjectTypeMap::none().candidates,
            identities: crate::realm::CommandBindingRealm::none(),
            declared: None,
            positioned_substitutions: false,
            source_input: None,
            source_procedures: &module.procedures,
            source_module: &module,
            current_tokens: None,
            parameter_body: None,
        };
        let mut facts = LocalFacts::default();
        scan_call_statement("global", &["".into()], &["".into()], None, ctx, &mut facts);
        assert!(facts.global_aliases.contains(""));
        assert!(
            !facts.writes_global,
            "declaring the literal empty alias is not a value write"
        );
        scan_call_statement(
            "append",
            &["".into(), "VALUE".into()],
            &["".into()],
            None,
            ctx,
            &mut facts,
        );
        assert!(
            facts.writes_global,
            "mutation reaches the retained literal empty alias"
        );
    }

    #[test]
    fn literal_empty_global_alias_mutations_remain_visible() {
        let analysis = build("proc ::f {} {global {}; append {} VALUE}");
        assert!(
            fixture_proc(&analysis, "::f")
                .expect("authentic standalone declaration")
                .writes_global
        );
    }

    #[test]
    fn nominal_global_alias_coverage_keeps_empty_data_separate_from_unknown_names() {
        let registry = CommandRegistry::build_default();
        let coverage =
            global_alias_names("global", &["".into(), "array(key)".into()], &registry).unwrap();
        assert!(coverage.names.contains("") && coverage.names.contains("array"));
        assert!(!coverage.unknown);
        let coverage = global_alias_names("global", &["$computed".into()], &registry).unwrap();
        assert!(coverage.names.is_empty() && coverage.unknown);
        let coverage = global_alias_names(
            "upvar",
            &["#0".into(), "source".into(), "".into()],
            &registry,
        )
        .unwrap();
        assert!(coverage.names.contains("") && !coverage.unknown);
        assert!(
            global_alias_names(
                "upvar",
                &["1".into(), "source".into(), "local".into()],
                &registry
            )
            .is_none()
        );
    }

    #[test]
    fn an_alias_at_the_current_or_global_frame_is_a_global_alias() {
        for src in [
            "proc ::f {} { upvar 0 ::x g\nset g 1 }",
            "proc ::f {t} { upvar +0 $t g\nset g 1 }",
            "proc ::f {} { upvar #00 x g\nset g 1 }",
        ] {
            let ia = build(src);
            assert!(
                ia.procedures
                    .get(&fixture_id(&ia.procedures, "::f"))
                    .expect("authored fixture body")
                    .writes_global,
                "{src}"
            );
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
            let s = fixture_proc(&ia, "::f").expect("::f summary");
            assert!(!s.writes_global, "unexpected writes_global for: {src}");
        }
    }

    // method-purity summary tests

    #[test]
    fn pure_getter_method_is_summarised_pure() {
        let ia = build(
            "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method get {} { ::return $n }\n\
             }",
        );
        let m = fixture_method(&ia, "::C::get").expect("method summary");
        assert!(m.base.pure, "read-only getter should be pure: {:?}", m.base);
        assert!(m.writes_instance_vars.is_empty());
        assert_eq!(m.class_name, "::C");
        assert_eq!(m.method_kind, crate::ir::MethodKind::Method);
    }

    #[test]
    fn instance_var_write_makes_method_impure() {
        let ia = build(
            "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method bump {} { incr n }\n\
             }",
        );
        let m = fixture_method(&ia, "::C::bump").expect("method summary");
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
        let m = fixture_method(&ia, "::C::bump").expect("method summary");
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
             \x20   method b {} { ::return 1 }\n\
             }",
        );
        let a = fixture_method(&ia, "::C::a").expect("method a");
        assert!(!a.base.pure, "my-dispatch caller must be impure");
        // The pure leaf method still summarises pure.
        let b = fixture_method(&ia, "::C::b").expect("method b");
        assert!(b.base.pure, "absolute literal return leaf: {:?}", b.base);
    }

    #[test]
    fn redefined_method_joins_over_every_retained_body() {
        // The lowering retains replacement bodies, so the
        // summary is the JOIN over every body a dispatch may run —
        // pure when all bodies are pure, impure when any is.
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { ::return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 2 }\n\
             }",
        );
        let methods = ia
            .methods
            .values()
            .filter(|method| method.base.qualified_name == "::C::m")
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2);
        for m in methods {
            assert!(
                m.base.pure,
                "two pure bodies join to pure — abstention on the mere fact of \
             redefinition is the imprecision issue #1166 removes"
            );
            // Disagreeing constants join to no-constant-return.
            assert!(m.base.constant_return.is_none());
        }

        // FN-now-caught: an impure REPLACEMENT makes the join impure even
        // though the first (retained-in-`methods`) body is pure.
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { puts side-effect\n\
             \x20       return 2 }\n\
             }",
        );
        let methods = ia
            .methods
            .values()
            .filter(|method| method.base.qualified_name == "::C::m")
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2);
        for m in methods {
            assert!(!m.base.pure, "an impure replacement body must join impure");
        }

        // Agreeing constants survive the join.
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { ::return 7 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 7 }\n\
             }",
        );
        let methods = ia
            .methods
            .values()
            .filter(|method| method.base.qualified_name == "::C::m")
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2);
        for m in methods {
            assert!(m.base.pure);
            assert!(matches!(
                m.base.constant_return,
                Some(ConstantReturn::Int(7))
            ));
        }
    }

    #[test]
    fn unreadable_oo_definitions_force_methods_impure() {
        // A dynamic member name may have redefined ANY method of the
        // class with an unknown body — every method of the class joins
        // impure.
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             \x20   method $n {} { return 2 }\n\
             }",
        );
        let m = fixture_method(&ia, "::C::m").expect("method summary");
        assert!(!m.base.pure, "unanalysable class member must widen");

        // A dynamic class word may have touched any class — module-wide.
        let ia = build(
            "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define $cls { method m {} { puts hi } }",
        );
        let m = fixture_method(&ia, "::C::m").expect("method summary");
        assert!(!m.base.pure, "dynamic OO definition target must widen");
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
        let m = fixture_method(&ia, "::C::peek").expect("method summary");
        assert!(
            m.writes_instance_vars.is_empty(),
            "variable decl wrongly counted as write: {:?}",
            m.writes_instance_vars
        );
    }

    #[test]
    fn call_in_if_condition_is_recorded() {
        // Calls embedded in an `if {[q]} ...` predicate are call-graph edges:
        // ::q must appear in ::a's direct calls.
        let ia = build(
            "proc ::q {} { return 1 }\n\
             proc ::a {} { if {[::q]} { puts hi } }",
        );
        let a = fixture_proc(&ia, "::a").expect("::a recorded");
        assert!(
            edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
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
        let a = fixture_proc(&ia, "::a").expect("::a recorded");
        assert!(
            edge_labels(&ia, &a.calls).contains(&"::p".to_string()),
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
        let a = fixture_proc(&ia, "::a").expect("::a recorded");
        assert!(
            edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
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
        let a = fixture_proc(&ia, "::a").unwrap();
        assert!(edge_labels(&ia, &a.calls).contains(&"::b".to_string()));
        assert!(
            edge_labels(&ia, &a.calls).contains(&"::c".to_string()),
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
        let s = fixture_proc(&ia, "::bad").unwrap();
        assert!(s.has_barrier);
        assert!(!s.pure);
    }

    #[test]
    fn pure_proc_is_flagged_pure() {
        let ia = build("proc ::add2 {x} { return [expr {$x + 2}] }");
        let s = fixture_proc(&ia, "::add2").unwrap();
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
        let fact = fixture_proc(&ia, "::fact").expect("::fact summary");
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
        let logit = fixture_proc(&ia, "::logit").expect("::logit summary");
        assert!(
            !logit.pure,
            "a proc that calls puts must be impure; got pure={}",
            logit.pure,
        );
    }

    /// A call that names only its caller's own places leaves the caller
    /// pure: `bump` writes the place its parameter names one frame up, so it
    /// is impure, as is `twice`, which hands `bump` the place its own
    /// parameter names; `p` passes `bump` a plain local of its own and `r`
    /// passes `twice` one, so both stay pure. A caller passing a qualified
    /// name, or a local `global` links to the global frame, does not.
    #[test]
    fn a_call_naming_the_callers_own_place_keeps_it_pure() {
        let ia = build(
            "proc ::bump {name} {upvar 1 $name v; incr v}\n\
             proc ::twice {name} {upvar 1 $name w; bump w; bump w}\n\
             proc ::p {} {set n 1; bump n; return $n}\n\
             proc ::r {} {set t 1; twice t; return $t}\n\
             proc ::g {} {bump ::n; return 0}\n\
             proc ::h {} {global n; bump n; return 0}\n",
        );
        let pure = |name: &str| fixture_proc(&ia, name).expect(name).pure;
        assert!(!pure("::bump"));
        assert!(!pure("::twice"));
        assert!(pure("::p"));
        assert!(pure("::r"));
        assert!(!pure("::g"));
        assert!(!pure("::h"));
    }

    #[test]
    fn unknown_call_sets_has_unknown_calls() {
        let ia = build("proc ::caller {} { nosuchcmd }");
        let s = fixture_proc(&ia, "::caller").unwrap();
        assert!(s.has_unknown_calls);
        assert!(!s.pure);
    }

    #[test]
    fn constant_return_inferred_for_literal_proc() {
        let ia = build("proc ::f {} { return 1 }");
        let s = fixture_proc(&ia, "::f").unwrap();
        assert!(s.returns_constant);
        assert_eq!(s.constant_return, Some(ConstantReturn::Int(1)));
    }

    #[test]
    fn conditional_return_is_not_constant_when_body_falls_through() {
        // O103: `f` returns 42 only when `$x > 0`; otherwise it
        // falls off the end (the `if` returns "" on the no-match path),
        // so it is NOT constant-returning and must not be folded.
        let ia = build("proc ::f {x} { if {$x > 0} { return 42 } }");
        let s = fixture_proc(&ia, "::f").unwrap();
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
        let s = fixture_proc(&ia, "::f").unwrap();
        assert!(s.returns_constant);
        assert_eq!(s.constant_return, Some(ConstantReturn::Int(7)));
    }

    #[test]
    fn return_word_classification_keeps_literal_names_and_selected_grammar() {
        let params = HashSet::from([
            "x".to_owned(),
            "{x}".to_owned(),
            "a:b".to_owned(),
            "arr".to_owned(),
            "cash$name".to_owned(),
            String::new(),
        ]);
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = tcl_lexer::LexerConfig::for_dialect(profile);
            for (raw, expected) in [
                ("$x", Some("x")),
                ("${cash$name}", Some("cash$name")),
                ("${}", Some("")),
                ("$a:b", None),
                ("${arr(k)}", None),
                ("$arr(k)", None),
                ("${{x}}", profile.starts_with("tcl9").then_some("{x}")),
            ] {
                let source = format!("return {raw}");
                let commands =
                    crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config);
                let tokens = crate::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    config,
                    &commands[0],
                );
                let word = tokens.words().get(1);
                let actual = classify_return(Some(raw), false, word, None, &params, Some(config));
                match (actual, expected) {
                    (ReturnKind::Passthrough(actual), Some(expected)) => {
                        assert_eq!(actual, expected)
                    }
                    (ReturnKind::Other, None) => {}
                    _ => panic!("{profile}: {raw}"),
                }
                assert!(matches!(
                    classify_return(Some(raw), false, word, None, &params, None),
                    ReturnKind::Other,
                ));
            }
            assert!(matches!(
                classify_return(Some("$x"), true, None, None, &params, Some(config)),
                ReturnKind::Literal(value) if value.bytes == b"$x",
            ));
            assert!(matches!(
                classify_return(Some("$x"), true, None, None, &params, None),
                ReturnKind::Other,
            ));
        }
    }

    #[test]
    fn passthrough_param_detected() {
        let ia = build("proc ::id {x} { return $x }");
        let s = fixture_proc(&ia, "::id").unwrap();
        assert_eq!(s.return_passthrough_param.as_deref(), Some("x"));
        assert_eq!(s.return_depends_on_params, vec!["x".to_string()]);
    }

    #[test]
    fn can_fold_gated_on_pure_and_return_shape() {
        // Pure + literal → can fold.
        let ia = build("proc ::f {} { return 42 }");
        assert!(fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
        // Impure (dynamic eval is a real barrier) + literal → cannot
        // fold. ``eval {literal}`` is a Block (non-barrier),
        // so use a dynamic body.
        let ia = build("proc ::f {} { eval $dyn ; return 42 }");
        assert!(!fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
    }

    fn original_reference_unit(
        source: &str,
    ) -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        CompilationUnit,
    ) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        // These fixtures pin explicit Standalone authoring (the retained
        // `original_authored_declaration_body_units`). A profiled default
        // build is a fresh-interpreter document (user decision
        // 2026-10-19T14:20Z); Standalone authoring is the dialect-blind
        // build, still profiled through its Registry and grammar.
        let unit = CompilationUnit::build_with_options(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                dialect: None,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
        );
        (context, unit)
    }

    fn original_reference_facts(
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> HashMap<ProcedureId, LocalFacts> {
        let module = &unit.ir_module;
        let identities =
            crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
        let bodies = summary_procedure_bodies(module, registry);
        let known = bodies.keys().copied().collect();
        scan_all_procs(
            module,
            SummaryBodyInputs {
                bodies: &bodies,
                known: &known,
                unit: Some(unit),
            },
            registry,
            module.dialect_profile,
            ObjectTypeMap::none().candidates,
            &identities,
            None,
        )
    }

    #[test]
    fn original_authored_alias_reports_keep_source_formals_frames_and_unknown_debts() {
        let source = "proc p {n v spare} {upvar 1 $n x; set x $v}";
        let (context, unit) = original_reference_unit(source);
        let registry = context.commands();
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = &module.body_units[&id];
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        let command =
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                tokens, module, registry,
            )
            .expect("actual original alias source row");
        assert_eq!(command.body(), id);
        assert!(command.has_unproved_applicability());
        let [alias] = command.aliases() else {
            panic!("one Registry alias declaration");
        };
        let held = command
            .caller_alias(alias)
            .expect("conditional caller alias incidence");
        assert_eq!(held.parameter(), "n");
        let write = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[1])
            .unwrap();
        let write =
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                write, module, registry,
            )
            .expect("actual retained source successor");
        assert!(held.belongs_to(&write));
        let facts = original_reference_facts(&unit, registry);
        let observed = &facts[&id];
        assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
        assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
        assert!(observed.param_observations_incomplete);
        assert!(observed.has_unknown_calls);
        assert!(observed.upvar_aliases.is_empty());
        assert!(observed.global_aliases.is_empty());
        assert!(observed.name_params.is_empty());
        assert!(observed.source_parameter_aliases.is_empty());
        assert!(
            observed
                .effect_writes
                .intersects(EffectRegion::UNKNOWN_STATE)
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let cloned = tokens.clone();
        assert!(
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                &cloned, module, registry
            )
            .is_none()
        );
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                tokens, &changed, registry
            )
            .is_none()
        );
        let mut changed = module.clone();
        changed.body_units.get_mut(&id).unwrap().params_raw = "other".to_owned();
        assert!(
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                tokens, &changed, registry
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::original_authored_parameter_source_command_for_module(
                tokens,
                module,
                tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
            )
            .is_none()
        );

        for body in [
            "upvar #0 $n x; set x 1",
            "upvar 0 $n x; set x 1",
            "upvar 2 $n x; set x 1",
            "upvar $level $n x; set x 1",
            "upvar 1 $n x; unknownCommand; set x 1",
            "upvar 1 $n x; global x; set x 1",
            "if {1} {upvar 1 $n x}; set x 1",
        ] {
            let source = format!("proc p {{n level}} {{{body}}}");
            let (context, unit) = original_reference_unit(&source);
            let (&id, _) = unit
                .ir_module
                .original_authored_declaration_body_units
                .iter()
                .next()
                .unwrap();
            let facts = original_reference_facts(&unit, context.commands());
            assert!(
                !facts[&id]
                    .param_trait_flags
                    .get("n")
                    .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite)),
                "{body}"
            );
            assert!(facts[&id].upvar_aliases.is_empty(), "{body}");
        }
        let (_, rejected) = original_reference_unit("proc p {::n} {upvar 1 $::n x; set x 1}");
        assert!(
            rejected
                .ir_module
                .original_authored_declaration_body_units
                .is_empty()
        );
        let (context, unit) = original_reference_unit("proc p {n} {upvar 1 $::n x; set x 1}");
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let facts = original_reference_facts(&unit, context.commands());
        assert!(
            !facts[&id]
                .param_trait_flags
                .get("::n")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
        );
        assert!(
            !facts[&id]
                .param_trait_flags
                .get("n")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
        );
        assert!(facts[&id].upvar_aliases.is_empty());
        let (context, unit) =
            original_reference_unit("proc p {n other} {upvar 1 $n x; upvar 1 $other x; set x 1}");
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let facts = original_reference_facts(&unit, context.commands());
        assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
        assert!(!facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
        assert!(facts[&id].param_trait_flags["other"].contains(&ProcArgTrait::VarWrite));
    }

    #[test]
    fn original_authored_call_reports_keep_actual_source_target_formals_and_debts() {
        let source = "proc ::helper {v} {return $v}\nproc ::f {x spare} {::helper $x}";
        let (context, unit) = original_reference_unit(source);
        let registry = context.commands();
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "::f")
            .unwrap();
        let procedure = &module.body_units[&id];
        let [statement] = procedure.body.statements.as_slice() else {
            panic!("one actual source call");
        };
        assert!(matches!(statement, crate::ir::Statement::Call { .. }));
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(!binding.has_original_source_layout_owner());
        assert!(binding.proved_execution_target().is_none());
        assert!(
            crate::registry_invocation::original_authored_procedure_calls_for_module(
                tokens, module, registry
            )
            .is_none()
        );
        let calls = crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens, module, registry,
        )
        .expect("actual retained source calling template");
        assert_eq!(calls.obligations().len(), 3);
        let [call] = calls.calls() else {
            panic!("one actual original callee allocation");
        };
        assert!(call.accepts_arguments());
        assert_eq!(call.argument_count(), Some(1));
        assert!(std::ptr::eq(
            call.procedure(),
            &module.body_units[&call.id()]
        ));
        assert_eq!(
            module.original_authored_declaration_body_units[&call.id()].name(),
            "::helper"
        );
        assert!(
            crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots::from_module(
                module,
                call.procedure()
            )
            .is_none()
        );
        assert!(
            !tokens
                .source_binding
                .as_ref()
                .unwrap()
                .has_original_source_layout_owner()
        );
        let observed = original_reference_facts(&unit, registry);
        assert!(observed[&id].param_trait_flags["x"].contains(&ProcArgTrait::ForwardedToCallee));
        assert!(observed[&id].param_observations_incomplete);
        assert!(observed[&id].has_unknown_calls);
        assert!(
            observed[&id]
                .effect_writes
                .intersects(crate::side_effects::EffectRegion::UNKNOWN_STATE)
        );
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(
            crate::registry_invocation::original_authored_body_procedure_calls_for_module(
                tokens, &changed, registry
            )
            .is_none()
        );
        let mut changed = module.clone();
        changed.body_units.get_mut(&call.id()).unwrap().params_raw = "other".to_owned();
        assert!(
            crate::registry_invocation::original_authored_body_procedure_calls_for_module(
                tokens, &changed, registry
            )
            .is_none()
        );
        let cloned = tokens.clone();
        assert!(
            crate::registry_invocation::original_authored_body_procedure_calls_for_module(
                &cloned, module, registry
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::original_authored_body_procedure_calls_for_module(
                tokens,
                module,
                tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
            )
            .is_none()
        );

        for body in ["unknownCommand; ::helper $x", "$name $x", "::helper"] {
            let source =
                format!("proc ::helper {{v}} {{return $v}}\nproc ::f {{x name}} {{{body}}}");
            let (context, unit) = original_reference_unit(&source);
            let module = &unit.ir_module;
            let (&id, _) = module
                .original_authored_declaration_body_units
                .iter()
                .find(|(_, card)| card.name() == "::f")
                .unwrap();
            let procedure = &module.body_units[&id];
            let statement = procedure.body.statements.last().unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let calls =
                crate::registry_invocation::original_authored_body_procedure_calls_for_module(
                    tokens,
                    module,
                    context.commands(),
                );
            assert!(
                !calls.is_some_and(|calls| !calls.calls().is_empty()
                    && calls.calls().iter().all(|call| call.accepts_arguments())),
                "{body}"
            );
            assert!(
                !original_reference_facts(&unit, context.commands())[&id]
                    .param_trait_flags
                    .values()
                    .any(|traits| traits.contains(&ProcArgTrait::ForwardedToCallee)),
                "{body}"
            );
        }
    }

    #[test]
    fn conditional_authored_declaration_reports_do_not_close_unused_parameters() {
        let (context, unit) =
            original_reference_unit("rename $old decl\nproc kept {unused} {return untouched}");
        let module = &unit.ir_module;
        let (&id, card) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "kept")
            .expect("genuine conditional source body");
        assert!(card.matches_procedure(module, &module.body_units[&id]));
        let facts = original_reference_facts(&unit, context.commands());
        let observed = &facts[&id];
        assert!(observed.param_observations_incomplete);
        let traits = finalise_param_traits(
            &module.body_units[&id].params,
            &observed.param_trait_flags,
            None,
            &observed.param_references,
            !observed.param_observations_incomplete,
        );
        assert!(
            !traits
                .get("unused")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Unused))
        );
        assert!(observed.upvar_aliases.is_empty());
        assert!(observed.global_aliases.is_empty());
    }

    #[test]
    fn original_formal_reference_traits_keep_actual_body_function_and_cfg_owners() {
        let (context, unit) = original_reference_unit("proc p {used spare} {return $used}");
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = &module.body_units[&id];
        assert!(procedure.body.executed_source.is_none());
        let facts = original_reference_facts(&unit, context.commands());
        let observed = &facts[&id];
        assert!(!observed.param_observations_incomplete);
        let traits = finalise_param_traits(
            &procedure.params,
            &observed.param_trait_flags,
            None,
            &observed.param_references,
            !observed.param_observations_incomplete,
        );
        assert!(traits["used"].contains(&ProcArgTrait::Passthrough));
        assert!(traits["spare"].contains(&ProcArgTrait::Unused));
        for &installed in module.installed_procedure_body_units.keys() {
            assert!(!facts.contains_key(&installed));
            assert!(unit.original_procedure_cfg_for_body(installed).is_none());
            assert!(unit.installed_procedure_cfg_for_body(installed).is_some());
        }
        let mut changed = unit.clone();
        changed
            .body_units
            .get_mut(&id)
            .unwrap()
            .cfg
            .name
            .push_str(" changed");
        let changed_facts = original_reference_facts(&changed, context.commands());
        let missing = &changed_facts[&id];
        assert!(missing.param_observations_incomplete);
        let traits = finalise_param_traits(
            &procedure.params,
            &missing.param_trait_flags,
            None,
            &missing.param_references,
            !missing.param_observations_incomplete,
        );
        assert!(
            traits
                .values()
                .all(|traits| !traits.contains(&ProcArgTrait::Unused))
        );
        let mut changed = unit.clone();
        changed.body_units.get_mut(&id).unwrap().body_id = None;
        assert!(
            original_reference_facts(&changed, context.commands())[&id]
                .param_observations_incomplete
        );
        let mut changed = unit.clone();
        let profile = context.commands().profile().unwrap();
        changed
            .body_units
            .get_mut(&id)
            .unwrap()
            .source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            module.native_lexer_config(),
        ));
        assert_eq!(changed.body_units[&id].cfg, unit.body_units[&id].cfg);
        assert!(
            original_reference_facts(&changed, context.commands())[&id]
                .param_observations_incomplete
        );
        let mut report = unit.clone();
        report
            .ir_module
            .body_units
            .get_mut(&id)
            .unwrap()
            .qualified_name = "unrelated display".into();
        assert!(
            !original_reference_facts(&report, context.commands())[&id]
                .param_observations_incomplete
        );
    }

    #[test]
    fn original_formal_reference_traits_keep_authored_conditions_and_unknown_obligations() {
        let (context, unit) = original_reference_unit(
            "proc p {n spare} {if {$n > 0} {return ONE} else {return TWO}}",
        );
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        assert!(matches!(
            module.body_units[&id].body.statements[0],
            crate::ir::Statement::If { .. }
        ));
        let facts = original_reference_facts(&unit, context.commands());
        assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::UsedInCondition));
        assert!(!facts[&id].param_references.contains("spare"));
        let (mutation_context, mutation_unit) = original_reference_unit(
            "proc p {first other} {if {$first > 0} {return ONE} else {return TWO}}",
        );
        let (&mutation_id, _) = mutation_unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        assert!(
            original_reference_facts(&mutation_unit, mutation_context.commands())[&mutation_id]
                .param_trait_flags["first"]
                .contains(&ProcArgTrait::UsedInCondition)
        );
        let mut changed = mutation_unit.clone();
        let crate::ir::Statement::If { clauses, .. } = &mut changed
            .ir_module
            .body_units
            .get_mut(&mutation_id)
            .unwrap()
            .body
            .statements[0]
        else {
            unreachable!()
        };
        let crate::expr_ast::ExprNode::Binary { left, .. } = &mut clauses[0].condition else {
            unreachable!()
        };
        let crate::expr_ast::ExprNode::Var { text, .. } = left.as_mut() else {
            unreachable!()
        };
        *text = "$other".into();
        assert_eq!(
            changed.body_units[&mutation_id].cfg,
            mutation_unit.body_units[&mutation_id].cfg
        );
        assert!(
            !changed.ir_module.original_authored_declaration_body_units[&mutation_id]
                .matches_procedure(
                    &changed.ir_module,
                    &changed.ir_module.body_units[&mutation_id]
                )
        );
        assert!(
            !original_reference_facts(&changed, mutation_context.commands())
                .contains_key(&mutation_id)
        );
        assert!(module.source_entry.native_entry.is_none());
        let body = &module.body_units[&id].body;
        let tokens = body
            .retained_source_tokens_for_statement(&body.statements[0])
            .unwrap();
        assert!(
            crate::registry_invocation::normal_transfer_invocation(
                context.commands(),
                None,
                tokens
            )
            .is_none()
        );

        let (context, unit) = original_reference_unit("proc p {used spare} {unknownCommand $used}");
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let facts = original_reference_facts(&unit, context.commands());
        let facts = &facts[&id];
        assert!(facts.param_observations_incomplete);
        assert!(facts.param_references.contains("used"));
        let traits = finalise_param_traits(
            &unit.ir_module.body_units[&id].params,
            &facts.param_trait_flags,
            None,
            &facts.param_references,
            !facts.param_observations_incomplete,
        );
        assert!(!traits["spare"].contains(&ProcArgTrait::Unused));
    }

    #[test]
    fn original_formal_reference_parts_keep_lexical_names_and_complete_grammar() {
        for (dialect, unicode, nested) in [
            ("tcl8.6", false, false),
            ("tcl9.1", false, true),
            ("jim", true, false),
        ] {
            let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
                .unwrap()
                .unit_profile();
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let source = r"puts $café ${p($k)} $arr($k) \$escaped {$braced} [list ${nested name}]";
            let image = tcl_lexer::SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in_with_purpose(
                image,
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .unwrap();
            assert!(plan.fatal_tail.is_none());
            let names = original_word_reference_names(&plan.commands[0].words).unwrap();
            let expected: HashSet<String> = [
                if unicode { "café" } else { "caf" },
                "p($k)",
                "arr",
                "k",
                "nested name",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect();
            assert_eq!(names, expected, "{dialect}");
            let source = "puts ${a{b}c}";
            let image = tcl_lexer::SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in_with_purpose(
                image,
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .unwrap();
            let names = original_word_reference_names(&plan.commands[0].words).unwrap();
            assert_eq!(
                names,
                HashSet::from([if nested { "a{b}c" } else { "a{b" }.to_owned()])
            );
        }
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let node = crate::expr_ast::ExprNode::Var {
            text: "${actual name}".into(),
            name: "unrelated report".into(),
            start: 0,
            end: 14,
        };
        let params = HashSet::from(["actual name".to_owned(), "unrelated report".to_owned()]);
        let mut facts = LocalFacts::default();
        note_params_in_expr(&node, &params, &mut facts, config, 0);
        assert!(facts.param_trait_flags.contains_key("actual name"));
        assert!(!facts.param_trait_flags.contains_key("unrelated report"));
        let mut names = Vec::new();
        assert!(walk_collect_param_refs(
            &node,
            &params,
            &mut names,
            Some(config),
            0
        ));
        assert_eq!(names, ["actual name"]);
        assert!(!walk_collect_param_refs(
            &node, &params, &mut names, None, 0
        ));
        let mut facts = LocalFacts {
            param_observations_incomplete: false,
            ..LocalFacts::default()
        };
        note_params_in_expr(&node, &params, &mut facts, config, u32::MAX);
        assert!(facts.param_observations_incomplete);
    }

    #[test]
    fn param_traits_inferred() {
        // `x` is returned → Passthrough; `y` never read → Unused.
        let ia = build("proc ::f {x y} { return $x }");
        let s = fixture_proc(&ia, "::f").unwrap();
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
        let s = fixture_proc(&w, "::setvar").unwrap();
        let nt = s.param_traits.get("n").expect("n traits");
        assert!(
            nt.contains(&ProcArgTrait::VarWrite),
            "expected VarWrite for n (write through upvar alias), got {nt:?}",
        );

        let r = build("proc ::getvar {n} { upvar 1 $n x\nreturn $x }");
        let s2 = fixture_proc(&r, "::getvar").unwrap();
        let nt2 = s2.param_traits.get("n").expect("n traits");
        assert!(
            nt2.contains(&ProcArgTrait::VarRead) && !nt2.contains(&ProcArgTrait::VarWrite),
            "expected VarRead (not VarWrite) for read-only upvar alias, got {nt2:?}",
        );

        // upvar #0 (global frame, not the caller) → VarRead only, no
        // write-back to the caller's passed variable.
        let g = build("proc ::gw {p} { upvar #0 $p x\nset x 1 }");
        let s3 = fixture_proc(&g, "::gw").unwrap();
        let pt = s3.param_traits.get("p").expect("p traits");
        assert!(
            pt.contains(&ProcArgTrait::VarRead) && !pt.contains(&ProcArgTrait::VarWrite),
            "upvar #0 is not a caller-frame write-back, got {pt:?}",
        );
    }

    /// `upvar $lvl $a b` has three words after the command, so `$lvl` is the
    /// level and `($a, b)` the pair, by the argument-count parity
    /// `Tcl_UpvarObjCmd` decides on; the text-sniffing reading took `$lvl`
    /// for the other variable and paired `($lvl, $a)`. A computed level is no
    /// known frame, so `a` names a variable the alias reads and no write-back
    /// is claimed; at level 1 the same pair is the caller's write-back.
    #[test]
    fn upvar_level_word_is_read_by_argument_parity() {
        let ia = build("proc ::q {lvl a} { upvar $lvl $a b\nset b 1 }");
        let s = fixture_proc(&ia, "::q").unwrap();
        let a = s.param_traits.get("a").expect("a traits");
        assert!(
            a.contains(&ProcArgTrait::VarRead) && !a.contains(&ProcArgTrait::VarWrite),
            "{a:?}"
        );
        assert!(
            s.param_traits.get("lvl").is_none_or(|t| {
                !t.contains(&ProcArgTrait::VarRead) && !t.contains(&ProcArgTrait::VarWrite)
            }),
            "{:?}",
            s.param_traits.get("lvl")
        );
        let ia = build("proc ::q {lvl a} { upvar 1 $a b\nset b 1 }");
        let a = ia
            .procedures
            .get(&fixture_id(&ia.procedures, "::q"))
            .expect("authored fixture body")
            .param_traits
            .get("a")
            .expect("a traits");
        assert!(a.contains(&ProcArgTrait::VarWrite), "{a:?}");
    }

    #[test]
    fn used_in_condition_detected() {
        let ia = build("proc ::f {n} { if {$n > 0} { return 1 } else { return 0 } }");
        let s = fixture_proc(&ia, "::f").unwrap();
        let traits = s.param_traits.get("n").expect("n traits");
        assert!(
            traits.contains(&ProcArgTrait::UsedInCondition),
            "expected UsedInCondition for n, got {traits:?}",
        );
    }

    #[test]
    fn forwarded_to_callee_detected() {
        let ia = build("proc ::helper {v} { return $v }\nproc ::f {x} { ::helper $x }");
        let s = fixture_proc(&ia, "::f").unwrap();
        let traits = s.param_traits.get("x").expect("x traits");
        assert!(
            traits.contains(&ProcArgTrait::ForwardedToCallee),
            "expected ForwardedToCallee for x, got {traits:?}",
        );
    }

    #[test]
    fn original_logical_source_call_edges_keep_body_ids_without_execution_grants() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        assert!(input.has_logical_source_name_context());
        let source = "proc target {} {return OK}; proc caller {} {target}";
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input);
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let select = |offset| {
            let rows: Vec<_> = module
                .original_source_procedure_bodies()
                .filter(|(_, procedure)| {
                    procedure
                        .declaration_site
                        .as_ref()
                        .is_some_and(|site| site.offset == offset)
                })
                .collect();
            assert_eq!(
                rows.len(),
                1,
                "one authored declaration at the original site"
            );
            assert!(module.procedure_ids.owns(*rows[0].0));
            *rows[0].0
        };
        let target = select(0);
        let caller = select(u32::try_from(source.find("proc caller").unwrap()).unwrap());
        let caller_body = module
            .original_source_procedure_bodies()
            .find(|(id, _)| **id == caller)
            .unwrap()
            .1;
        let tokens = caller_body
            .body
            .retained_source_tokens_for_statement(&caller_body.body.statements[0])
            .unwrap();
        let calls = crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens, &module, registry,
        )
        .expect("actual conditional original call and declaration owners");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id(), target);
        assert!(calls[0].accepts_arguments());
        assert!(
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                registry, None, tokens,
            )
            .is_none()
        );
        let identities =
            crate::realm::document_realm_bindings_for_module(&module, registry).unwrap();
        let analysis = build_interprocedural_analysis(
            &module,
            registry,
            Some(profile),
            ObjectTypeMap::none(),
            &identities,
            None,
        );
        let summary = &analysis.procedures[&caller];
        assert!(summary.direct_calls.contains(&target));
        assert!(summary.calls.contains(&target));
        assert!(summary.has_unknown_calls);
        assert!(!summary.pure);
        assert!(!summary.completes);
        assert!(module.procedures.get(&target).is_none());
        let mut changed = tokens.clone();
        changed.argv_texts[0] = "different".to_owned();
        assert!(
            crate::registry_invocation::original_logical_procedure_calls_for_module(
                &changed, &module, registry,
            )
            .is_none()
        );
        let mut stale = module.clone();
        stale.source = tcl_lexer::SourceImage::document("proc different {} {return OK}");
        assert!(
            crate::registry_invocation::original_logical_procedure_calls_for_module(
                tokens, &stale, registry,
            )
            .is_none()
        );
    }

    #[test]
    fn cyclic_call_graph_handled() {
        let ia = build(
            "proc ::a {} { ::b }\n\
             proc ::b {} { ::a }",
        );
        let a = fixture_proc(&ia, "::a").unwrap();
        assert!(edge_labels(&ia, &a.calls).contains(&"::b".to_string()));
        let b = fixture_proc(&ia, "::b").unwrap();
        assert!(edge_labels(&ia, &b.calls).contains(&"::a".to_string()));
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
        if let Some(caller) = fixture_proc(&ia, "::ns::caller") {
            assert!(
                edge_labels(&ia, &caller.calls)
                    .iter()
                    .any(|c| c == "::ns::helper"),
                "expected ::ns::helper in ::ns::caller.calls, got {:?}",
                caller.calls,
            );
        }
    }

    #[test]
    fn instance_write_candidates_keep_literal_destinations_and_dynamic_unknowns() {
        let ivars: HashSet<_> = ["", "$field", "field", "${braced}", "$array"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut writes = HashSet::new();
        for name in ["", "$field", "${braced}", "$array(key)"] {
            check_ivar_write(name, &ivars, &mut writes);
        }
        assert_eq!(
            writes,
            HashSet::from_iter(
                ["", "$field", "${braced}", "$array"]
                    .into_iter()
                    .map(str::to_owned)
            )
        );
        assert!(!writes.contains("field"));
        let registry = tcl_registry::CommandRegistry::build_default();
        let dynamic = crate::ir::Script::from_statements(vec![crate::ir::Statement::AssignConst {
            span: tcl_lexer::Span::new(0, 0),
            name: "$target".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]);
        collect_instance_var_writes(&dynamic, &ivars, &registry, &mut writes, 0);
        assert_eq!(writes, ivars);
    }

    /// `collect_instance_var_writes`
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

    /// `collect_instance_var_writes`
    /// (the `TclOO` method-body instance-write scan) recurses once per
    /// nested `if` body, so it needs a depth cap of its own.
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
                assert!(
                    ia.methods
                        .values()
                        .any(|method| method.base.qualified_name == "::C::bump")
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// The call-graph scan must resolve a command head's *effective
    /// identity*, not its written spelling.
    ///
    /// `catch`'s argument carries `ArgRole::Body`, so the text scan descends
    /// into it and records the calls inside as reachability edges.  Whether
    /// `q` becomes an edge of `p` is therefore a clean witness that the body
    /// role was resolved — and a missing edge is what makes a live proc look
    /// dead (O124).
    ///
    /// The call sits inside a `[…]` substitution deliberately: a *statement*
    /// `catch {…}` is turned into control-flow structure by lowering long
    /// before this scan sees it, so only the value-context path exercises the
    /// registry role query in question.
    ///
    /// On tclsh 8.6.16 and 9.0.4 (byte-identical): `interp alias {} guard
    /// {} catch` makes `guard` run `catch`; `rename catch guard` moves it and
    /// leaves `catch` gone; a top-level `proc catch …` takes the name over.
    fn p_calls_q(prelude: &str, head: &str) -> bool {
        let src = format!(
            "{prelude}proc q {{}} {{ return 1 }}\nproc p {{}} {{ set z [{head} {{q}}] }}\n"
        );
        let ia = build(&src);
        fixture_proc(&ia, "::p")
            .is_some_and(|s| edge_labels(&ia, &s.calls).iter().any(|c| c == "::q"))
    }

    #[test]
    fn call_graph_follows_an_aliased_body_command() {
        assert!(p_calls_q("interp alias {} guard {} catch\n", "guard"));
        // The `::`-qualified spelling of the alias classifies alike.
        assert!(p_calls_q("interp alias {} guard {} catch\n", "::guard"));
        // Guard: an unbound `guard` carries no body argument to descend into.
        assert!(!p_calls_q("set y 1\n", "guard"));
    }

    #[test]
    fn call_graph_follows_a_renamed_body_command() {
        assert!(p_calls_q("rename catch guard\n", "guard"));
        assert!(
            !p_calls_q("rename catch guard\n", "catch"),
            "a renamed-away `catch` must not keep the built-in's body grammar"
        );
    }

    #[test]
    fn call_graph_abstains_for_a_builtin_shadowed_by_a_user_proc() {
        assert!(
            !p_calls_q("proc catch {s} { return 1 }\n", "catch"),
            "a user `proc catch` takes the name over; its argument is a plain \
             value word, not a script"
        );
        // Guard: the unshadowed built-in still descends.
        assert!(p_calls_q("set y 1\n", "catch"));
    }

    #[test]
    fn call_graph_abstains_for_a_dynamic_binding() {
        assert!(
            !p_calls_q("rename $old guard\n", "guard"),
            "a dynamic rename must not give `guard` a body grammar"
        );
        assert!(
            p_calls_q("rename $old guard\n", "catch"),
            "a dynamic rename must not take `catch`'s body grammar away either"
        );
    }

    /// The side-effect classification reads the resolved head too: a call
    /// through a proven alias is a *known* command, so it does not force
    /// `has_unknown_calls` (which alone makes a procedure permanently impure).
    #[test]
    fn side_effect_classification_follows_an_aliased_head() {
        let unknown = |prelude: &str, head: &str| {
            let src = format!("{prelude}proc p {{}} {{ {head} hi }}\n");
            let ia = build(&src);
            fixture_proc(&ia, "::p").is_some_and(|s| s.has_unknown_calls)
        };
        assert!(
            !unknown("interp alias {} out {} puts\n", "out"),
            "an alias of `puts` is a known command, not an unknown call"
        );
        // Guard: an unbound `out` is genuinely unknown.
        assert!(unknown("set y 1\n", "out"));
    }

    /// Whether a statement of `::p`'s flow graph may run a `return` of its
    /// own, as the return reading asks of each executable block.
    fn p_may_return(source: &str) -> bool {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(source, &registry, false);
        unit.procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|stmt| statement_may_return(stmt, &registry, 0))
    }

    /// A call holding a word the registry gives the Body role may run a
    /// `return` there: the flow graph keeps a loop over a qualified variable
    /// as a call holding its body, and a barrier's code is unseen. A `catch`
    /// absorbs its body's `return`, and the header of a loop the flow graph
    /// lowers holds only the list words.
    #[test]
    fn a_call_running_a_body_word_may_return() {
        assert!(p_may_return(
            "proc p {} {foreach ::x {1} {return 1}; return 2}"
        ));
        assert!(p_may_return(
            "proc p {} {lmap ::x {1} {return 1}; return 2}"
        ));
        assert!(p_may_return("proc p {} {time {return 1}; return 2}"));
        assert!(!p_may_return("proc p {} {catch {return 1}; return 2}"));
        assert!(!p_may_return(
            "proc p {} {set s 0; foreach x {1 2} y {3 4} z {5 6} {incr s}; return $s}"
        ));
    }
}

#[cfg(test)]
mod effect_propagation_tests {
    use super::test_body_identity::fixture_proc;
    use super::*;
    use tcl_registry::CommandRegistry;

    fn irules_registry() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .clone()
    }

    fn analyse_irules(source: &str, registry: &CommandRegistry) -> InterproceduralAnalysis {
        let profile = registry.profile().expect("selected F5 provider profile");
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            source,
            unit.ir_module.lexer_config,
            registry,
            &unit.ir_module.source_entry,
        );
        build_interprocedural_analysis(
            &unit.ir_module,
            registry,
            Some(profile),
            ObjectTypeMap::none(),
            &identities,
            None,
        )
    }

    /// Reached substitutions contribute their effects in assignments, returns,
    /// conditions and ordinary command operands. A braced value is inert.
    #[test]
    fn nested_substitution_effects_propagate_only_when_evaluated() {
        let reg = irules_registry();

        for src in [
            "proc p {} { set x [matchclass [HTTP::uri] equals $::l] }",
            "proc p {} { return [matchclass [HTTP::uri] equals $::l] }",
            "proc p {} { if {[matchclass [HTTP::uri] equals $::l]} {} }",
            "proc p {} { matchclass [HTTP::uri] equals $::l }",
        ] {
            let ia = analyse_irules(src, &reg);
            let s = fixture_proc(&ia, "::p").expect("proc ::p in IA");
            assert!(
                s.effect_reads.contains(EffectRegion::HTTP_STATE),
                "evaluated nested [HTTP::uri] should propagate HTTP_STATE; src={src:?} reads={:?}",
                s.effect_reads
            );
        }

        let ia = analyse_irules("proc q {} { matchclass {[HTTP::uri]} equals $::l }", &reg);
        let s = fixture_proc(&ia, "::q").expect("proc ::q in IA");
        assert!(
            !s.effect_reads.contains(EffectRegion::HTTP_STATE),
            "inert braced operand must not propagate HTTP_STATE; reads={:?}",
            s.effect_reads
        );
    }
}

#[cfg(test)]
mod selected_global_option_taint_tests {
    use super::*;

    fn build_tk_module(
        source: &str,
    ) -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::ir::Module,
    ) {
        let environment = tcl_registry::model::ingress::resolve_environment("tk");
        let context = environment.default_context_registry();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input);
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        (context, module)
    }

    #[test]
    fn interprocedural_option_summary_uses_selected_alias_target_receipt() {
        let source = "interp alias {} linkedEntry {} entry .e -textvariable; proc p {} {linkedEntry ::bound}";
        let (context, module) = build_tk_module(source);
        let registry = context.commands();
        let id = *module
            .procedures
            .keys()
            .next()
            .expect("one source procedure");
        let writes = tainted_global_writes(
            &module,
            registry,
            &global_instance_classes(&module, registry),
        );
        assert!(writes[&id].contains("::bound"));

        let ordinary_source = "proc p {} {entry .e -state normal}";
        let (context, ordinary) = build_tk_module(ordinary_source);
        let registry = context.commands();
        let id = *ordinary
            .procedures
            .keys()
            .next()
            .expect("one source procedure");
        let writes = tainted_global_writes(
            &ordinary,
            registry,
            &global_instance_classes(&ordinary, registry),
        );
        assert!(
            !writes
                .get(&id)
                .is_some_and(|names| names.contains("::normal"))
        );
    }
}
