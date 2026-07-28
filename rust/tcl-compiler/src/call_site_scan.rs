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

//! Whole-module call-site literal evidence — the input to the
//! interprocedural `param_constants` SCCP seed
//! ([`crate::compilation_unit::params_constants_from_call_sites`]).
//!
//! The seed binds a procedure parameter to a compile-time literal when
//! *every* caller passes the same value there, so the evidence is only
//! usable if the scan can prove it enumerated **every** caller.  A call
//! site this scan fails to attribute doesn't merely go uncounted — it
//! vanishes from the "every caller agrees" evidence and can flip an
//! absence of contradicting evidence into a false positive (issue #969).
//!
//! Three kinds of caller exist, and all three are enumerated here:
//!
//! - a **literal command word** (`helper prod`), resolved through
//!   [`crate::interprocedural::resolve_internal_call`] in the calling
//!   function's own namespace, with a `namespace import` fallback;
//! - a **script nested in an argument** (`catch { helper dev }`, a literal
//!   `uplevel { … }`, a non-exact `switch` arm), reached by recursing every
//!   [`ArgRole::Body`] argument the registry declares; and
//! - an **indirection** — a dynamically-dispatched command word
//!   (`set cmd helper; $cmd dev`), a registry-declared command-prefix
//!   callback (`lsort -command cb`), a registry-declared user-proc invoker
//!   (iRules `call PROC …`), or a script the command receives as a *value*
//!   rather than as text (`eval $script`, `apply $fn`).
//!
//! The third kind is what issue #976 reports: before this module a dynamic
//! command word was silently skipped, so `$cmd dev` could reach a proc the
//! scan had already (otherwise soundly) seeded from its literal call sites.
//! A dispatch is resolved *by value*: the literal strings its command word
//! can hold are enumerated from the scope's own constant assignments and,
//! for a parameter, from the call-site evidence itself — a monotone
//! fixpoint (see [`collect_call_site_constants`]), never the SCCP result
//! this evidence produces, so there is no circular dependency.  A dispatch
//! whose word resolves to a known set of names is recorded as an ordinary
//! call site for each of them, so agreeing evidence stays agreeing
//! evidence.  When a word's value set — or a script's text — cannot be read
//! at all, the scan records [`CallSiteEvidence::opaque_callee`] and every
//! seed in the module is withdrawn: the alternative is trusting evidence
//! that is provably incomplete.
//!
//! No command name appears anywhere in this module.  Which argument is a
//! script, which is a callback prefix, which command invokes a user proc
//! named by its first word, and which words are variable writes are all
//! registry facts ([`ArgRole`], [`Traits`]).

use std::collections::{BTreeSet, HashMap, HashSet};

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::is_dynamic_word;
use crate::value_shapes::{
    is_pure_var_ref, parse_command_substitution, whole_word_scalar_var_name,
};

/// Recursion cap for [`record_call_site_evidence`]'s descent into nested
/// [`ArgRole::Body`] arguments (`catch { catch { catch { … } } }` and
/// similar) — defensive against a pathological or generated nesting depth;
/// real code never approaches it.
const MAX_CALL_SITE_BODY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(16);

/// Iteration cap for [`collect_call_site_constants`]'s evidence fixpoint.
///
/// Each round re-derives the whole evidence set from the previous round's,
/// which only ever grows, so the chain is increasing and terminates on its
/// own; the cap is the defensive backstop for a pathological module.  A
/// module that has not converged by then is treated as having an
/// unenumerable callee (`opaque_callee`) rather than being trusted at a
/// non-fixpoint.
const MAX_CALL_SITE_SCAN_ROUNDS: u32 = 6;

/// Per-arg-position call-site literal evidence for one callee.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct ArgConsts {
    /// At least one call passed a non-literal (`$`/`[`) value here, omitted
    /// the argument entirely (so its default — an unknown value — is used),
    /// or reached the callee through an indirection whose arguments this
    /// scan cannot see.
    pub(crate) unknown: bool,
    /// Distinct literal values seen at this position.
    pub(crate) values: HashSet<String>,
}

/// Whole-module call-site evidence: what each callee's parameters were
/// passed, plus whether the scan can prove it saw every caller at all.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct CallSiteEvidence {
    by_callee: HashMap<String, HashMap<usize, ArgConsts>>,
    /// An indirection's *callee* could not be enumerated — some call in
    /// this module may reach any procedure with any arguments, so no
    /// callee's evidence can be claimed complete.
    opaque_callee: bool,
    /// A value set was consulted while building this evidence, so another
    /// fixpoint round may still discover more callers.  Bookkeeping for
    /// [`collect_call_site_constants`]; deliberately part of the
    /// equality used as the convergence test.
    consulted_value_sets: bool,
}

impl CallSiteEvidence {
    /// Per-position evidence for `qname`, or `None` when no caller of it
    /// was seen at all.
    pub(crate) fn callee(&self, qname: &str) -> Option<&HashMap<usize, ArgConsts>> {
        self.by_callee.get(qname)
    }

    /// True when every caller in the module was attributed to a specific
    /// callee — the precondition for trusting any "every caller agrees"
    /// claim built from this evidence.
    pub(crate) const fn enumerates_every_caller(&self) -> bool {
        !self.opaque_callee
    }

    /// Record one call passing `args` to `target`.
    fn record_call(&mut self, target: &str, args: &[String], nparams: usize) {
        let by_idx = self.by_callee.entry(target.to_owned()).or_default();
        for (i, arg) in args.iter().enumerate() {
            let slot = by_idx.entry(i).or_default();
            if is_dynamic_word(arg) {
                slot.unknown = true;
            } else {
                slot.values.insert(arg.clone());
            }
        }
        // A call that omits a (defaulted) parameter uses its default, an
        // unknown value at that slot — poison every param position this
        // call doesn't provide so a single literal at another call site
        // can't bind it.
        for i in args.len()..nparams {
            by_idx.entry(i).or_default().unknown = true;
        }
    }

    /// Record one call to `target` whose argument list this scan cannot
    /// see (a command-prefix callback the runtime appends arguments to).
    fn record_unknown_call(&mut self, target: &str, nparams: usize) {
        let by_idx = self.by_callee.entry(target.to_owned()).or_default();
        for i in 0..nparams {
            by_idx.entry(i).or_default().unknown = true;
        }
    }
}

/// Invariant context [`record_call_site_evidence`] threads unchanged
/// through its recursion — grouped into one struct (rather than passed as
/// separate parameters) purely to keep the recursive function's own
/// argument count down to the things that actually change per call.
pub(crate) struct CallSiteScanCtx<'a> {
    procedures: &'a HashMap<String, crate::ir::Procedure>,
    known: &'a HashSet<String>,
    registry: &'a CommandRegistry,
    dialect: &'a str,
    /// `namespace import` directives (`(importing_namespace,
    /// absolute_pattern)` pairs), from
    /// [`crate::ir::Module::namespace_imports`] — see
    /// [`resolve_via_namespace_import`].
    namespace_imports: &'a [(String, String)],
    /// Per-scope literal-value facts for local variables, from
    /// [`collect_module_scope_var_facts`].
    var_facts: &'a ModuleVarFacts,
    /// The previous fixpoint round's evidence, consulted (never written)
    /// when resolving a parameter's value set.
    previous: &'a CallSiteEvidence,
}

/// One body the scan walks as a *caller*.
///
/// `resolve_as` and `scope` differ for a `TclOO` method: its bare command
/// words resolve against the global namespace (tclsh-confirmed), while its
/// local variables live in the method's own frame.
struct CallerFrame<'a> {
    /// Qualified-name context bare command words resolve against.
    resolve_as: &'a str,
    /// Variable-scope identity — the key into `var_facts`, and the callee
    /// key whose recorded arguments a `$param` dispatch word may take.
    scope: &'a str,
    /// The body's declared parameters, in order.
    params: &'a [String],
    /// Whether this evidence map attributes calls *to* this body. True for
    /// the top level and ordinary procedures; false for a `TclOO` method or
    /// an `apply` lambda, whose invocations are not call sites this scan
    /// resolves — so their parameters can hold values it never saw.
    callers_tracked: bool,
}

/// A body the whole-module scan walks as a caller, beyond the top level
/// and the ordinary procedures.
pub(crate) struct ExtraCaller {
    /// Qualified-name context bare command words in this body resolve
    /// against — see [`CallerFrame::resolve_as`].
    pub(crate) resolve_as: String,
    /// Variable-scope identity — see [`CallerFrame::scope`].
    pub(crate) scope: String,
    /// The body's declared parameters — see [`CallerFrame::params`].
    pub(crate) params: Vec<String>,
    /// The body's control-flow graph.
    pub(crate) cfg: CfgFunction,
}

/// Resolve `command` to a qualified proc name via a `namespace import`
/// directive active in `caller_qname`'s own namespace, when
/// [`crate::interprocedural::resolve_internal_call`] found no direct match.
///
/// `namespace import ::lib::helper` binds the bare name `helper` in the
/// importing namespace to `::lib::helper` — a real command-resolution path
/// distinct from (and checked *after*) plain namespace-relative lookup, so a
/// call site reached only this way was invisible to the collector before
/// this: `::lib::helper`'s only *visible* caller was some unrelated external
/// call, while every importing-namespace caller's (potentially differing)
/// argument silently vanished from the evidence, exactly like the
/// namespace-blind recursion and `ArgRole::Body` gaps above.
///
/// `::foo::*` (wildcard) imports resolve `command` under `::foo`; an exact
/// pattern (`::foo::bar`) binds only its own leaf name — `namespace_imports`
/// already only ever records absolute patterns (relative ones need runtime
/// namespace-path walking this compile-time pass does not model, per
/// [`crate::ir::Module::namespace_imports`]'s own doc).
fn resolve_via_namespace_import<S: std::hash::BuildHasher>(
    command: &str,
    caller_qname: &str,
    namespace_imports: &[(String, String)],
    known: &HashSet<String, S>,
) -> Option<String> {
    if namespace_imports.is_empty() {
        return None;
    }
    let ns_parts = crate::interprocedural::namespace_parts_from_proc(caller_qname);
    let caller_ns = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    for (import_ns, pattern) in namespace_imports {
        if *import_ns != caller_ns {
            continue;
        }
        if let Some(ns_prefix) = pattern.strip_suffix("::*") {
            let candidate = format!("{ns_prefix}::{command}");
            if known.contains(&candidate) {
                return Some(candidate);
            }
        } else if pattern.rsplit("::").next().unwrap_or(pattern.as_str()) == command
            && known.contains(pattern)
        {
            return Some(pattern.clone());
        }
    }
    None
}

/// The user procedure a (literal) command word names when invoked from
/// `caller_qname`'s namespace, or `None` when it names no procedure in this
/// module.
fn resolve_target(ctx: &CallSiteScanCtx<'_>, caller_qname: &str, command: &str) -> Option<String> {
    crate::interprocedural::resolve_internal_call(command, caller_qname, ctx.known).or_else(|| {
        resolve_via_namespace_import(command, caller_qname, ctx.namespace_imports, ctx.known)
    })
}

/// The set of literal strings a word may evaluate to.
enum WordValues {
    /// Fully enumerated — the word can only ever be one of these.
    Literals(BTreeSet<String>),
    /// Not enumerable by this scan.
    Unknown,
}

/// Literal-value facts for one variable in one scope.
#[derive(Default, Clone)]
struct VarLiterals {
    /// Distinct literal values assigned to it.
    values: BTreeSet<String>,
    /// A write whose value this scan cannot pin to a literal.
    unknown: bool,
}

/// Literal-value facts for every local variable of one scope.
#[derive(Default)]
struct ScopeVars {
    vars: HashMap<String, VarLiterals>,
    /// A write through a *dynamic variable name* (`set $n $v`, `upvar 1 x
    /// $alias`) happened somewhere in this scope, so any local may hold a
    /// value this scan never saw.
    dynamic_name_write: bool,
    /// A script this scan cannot read runs in *some* variable frame it does
    /// not own — a frame-shifting `uplevel` body, or a body reached only
    /// through a substitution (`eval $script`). Such a script may assign a
    /// local of any scope, including this one, so it is a module-wide fact
    /// rather than a per-scope one; it is discovered per scope and unioned
    /// by [`collect_module_scope_var_facts`].
    cross_frame_write: bool,
}

/// Per-scope literal-value facts for the whole module, plus the
/// module-wide "a script wrote a frame this scan does not own" fact.
#[derive(Default)]
struct ModuleVarFacts {
    scopes: HashMap<String, ScopeVars>,
    cross_frame_write: bool,
}

impl ScopeVars {
    /// Record an assignment to the variable `name` names — `Some(value)`
    /// when the assigned value is a compile-time literal, `None` when it
    /// is not.  Poisons the whole scope when the *name* is computed.
    fn note_assignment(&mut self, name: &str, value: Option<&str>) {
        if is_dynamic_word(name) {
            self.dynamic_name_write = true;
            return;
        }
        let facts = self.vars.entry(name.to_owned()).or_default();
        match value {
            Some(literal) => {
                facts.values.insert(literal.to_owned());
            }
            None => facts.unknown = true,
        }
    }

    fn note_unknown(&mut self, name: &str) {
        self.vars.entry(name.to_owned()).or_default().unknown = true;
    }

    /// Record a write to the variable a *word* names — poisoning the whole
    /// scope when the name itself is computed.
    fn note_write_word(&mut self, word: &str) {
        if is_dynamic_word(word) {
            self.dynamic_name_write = true;
        } else {
            self.note_unknown(word);
        }
    }
}

/// Collect literal-value facts for one scope's local variables.
///
/// Constant assignments (`set cmd helper`, or the plain-bareword
/// `AssignValue` shape lowering leaves alone) contribute a literal; every
/// other write contributes "unknown" for that name.  Which *words* of a
/// generic call name a variable is registry data ([`ArgRole::VarWrite`],
/// plus [`Traits::CREATES_SCOPE_ALIAS`] for the vararg alias forms `global
/// x y z` / `variable a b` / `upvar 1 a b 1 c d`, whose per-argument name
/// list the role query deliberately does not expand) — no command name is
/// matched here.
///
/// Structured bodies (`if`, `while`, `catch`, a *literal* `eval`/`uplevel
/// #0` body) are already flattened into the caller's own CFG blocks by
/// lowering, so their writes are ordinary block statements here.  The two
/// that are not — a frame-shifting [`Statement::UpFrame`] body and a body
/// reached only through a substitution on a [`Traits::EVALUATES_CODE`]
/// command — set [`ScopeVars::cross_frame_write`] instead.
fn collect_scope_var_facts(cfg: &CfgFunction, registry: &CommandRegistry) -> ScopeVars {
    let mut out = ScopeVars::default();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, value, .. } => {
                    out.note_assignment(name, Some(value));
                }
                // Lowering only produces `AssignConst` for a *recognised*
                // constant shape; a plain bareword value (`set cmd helper`)
                // arrives as `AssignValue` and is just as literal — provided
                // it neither substitutes nor needs backslash resolution.
                Statement::AssignValue {
                    name,
                    value,
                    value_needs_backsubst,
                    ..
                } => {
                    let literal =
                        (!*value_needs_backsubst && !is_dynamic_word(value)).then_some(value);
                    out.note_assignment(name, literal.map(String::as_str));
                }
                Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
                    out.note_write_word(name);
                }
                Statement::Call { args, defs, .. } => {
                    for d in defs {
                        out.note_write_word(d);
                    }
                    note_registry_var_writes(&mut out, registry, stmt, args);
                }
                Statement::Barrier { args, .. } => {
                    note_registry_var_writes(&mut out, registry, stmt, args);
                }
                // `uplevel N {…}`: the body's writes land in another
                // frame, which this per-scope walk does not own.
                Statement::UpFrame { .. } => out.cross_frame_write = true,
                _ => {}
            }
        }
    }
    out
}

/// Record the variable writes the registry declares for one call.
fn note_registry_var_writes(
    out: &mut ScopeVars,
    registry: &CommandRegistry,
    stmt: &Statement,
    args: &[String],
) {
    let command = stmt.canonical_command_or_source();
    let bare = command.strip_prefix("::").unwrap_or(command);
    if is_dynamic_word(bare) {
        // A computed head (`$cmd 5`) has no registry spec to read roles
        // from, so there is nothing to record. It is deliberately not
        // treated as perturbing the scope either: a *user proc* reached
        // this way can only write the caller's locals through `upvar`,
        // which this scan does not model for a literal call site either.
        // The residual — a computed head that resolves to a
        // variable-writing *builtin* (`set cmd set; $cmd x 5`) — would
        // require a builtin's own name to be among the literals a local
        // holds, and is documented rather than paid for by disqualifying
        // every dispatch-table body's own variables.
        return;
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in registry.arg_indices_for_role(bare, &arg_strs, ArgRole::VarWrite) {
        if let Some(word) = args.get(idx) {
            out.note_write_word(word);
        }
    }
    let Some(spec) = registry.get(bare) else {
        return;
    };
    // `global` / `variable` / `upvar` bind *every* name in a vararg list to
    // an outer-scope variable another body may write — the registry marks
    // exactly these with `CREATES_SCOPE_ALIAS`, because the per-argument
    // list is not expressible as fixed role indices.
    if spec.traits.contains(Traits::CREATES_SCOPE_ALIAS) {
        for word in args {
            out.note_write_word(word);
        }
    }
}

/// The literal values `word` may take when evaluated in `caller`'s scope.
///
/// A word with no substitution is its own only value.  A whole-word scalar
/// reference (`$cmd` / `${cmd}`) resolves against the scope's literal-value
/// facts, unioned — when the name is one of the body's own parameters —
/// with the literals its callers pass at that position
/// ([`parameter_values`]).  Anything else is [`WordValues::Unknown`].
fn word_values(ctx: &CallSiteScanCtx<'_>, caller: &CallerFrame<'_>, word: &str) -> WordValues {
    if !is_dynamic_word(word) {
        return WordValues::Literals(std::iter::once(word.to_owned()).collect());
    }
    let Some(var) = whole_word_scalar_var_name(word) else {
        return WordValues::Unknown;
    };
    // A namespace-qualified reference names a variable any body in the
    // module (or another file) may write; this scan models local frames
    // only.
    if var.contains("::") {
        return WordValues::Unknown;
    }
    // A script running in a frame this scan cannot read may have assigned
    // any local of any scope.
    if ctx.var_facts.cross_frame_write {
        return WordValues::Unknown;
    }
    let Some(scope_vars) = ctx.var_facts.scopes.get(caller.scope) else {
        return WordValues::Unknown;
    };
    if scope_vars.dynamic_name_write {
        return WordValues::Unknown;
    }
    let mut values = BTreeSet::new();
    if let Some(facts) = scope_vars.vars.get(var) {
        if facts.unknown {
            return WordValues::Unknown;
        }
        values.extend(facts.values.iter().cloned());
    }
    match parameter_values(ctx, caller, var) {
        Some(WordValues::Unknown) => return WordValues::Unknown,
        Some(WordValues::Literals(from_callers)) => values.extend(from_callers),
        None => {}
    }
    WordValues::Literals(values)
}

/// The literals `var`'s callers pass when `var` is one of `caller`'s
/// declared parameters, or `None` when it is not a parameter at all.
///
/// A parameter with no recorded caller contributes nothing: within a
/// compilation unit already gated on having no `package provide`, an
/// uncalled procedure's body never runs. A parameter of a body whose
/// callers this scan does not attribute at all (a method, an `apply`
/// lambda) can hold anything.
fn parameter_values(
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    var: &str,
) -> Option<WordValues> {
    let index = caller.params.iter().position(|p| p == var)?;
    // `args` slurps every remaining argument into a *list*, so its value is
    // not any one caller's word.
    if var == "args" || !caller.callers_tracked {
        return Some(WordValues::Unknown);
    }
    match ctx
        .previous
        .callee(caller.scope)
        .and_then(|by_idx| by_idx.get(&index))
    {
        Some(slot) if slot.unknown => Some(WordValues::Unknown),
        Some(slot) => Some(WordValues::Literals(slot.values.iter().cloned().collect())),
        None => Some(WordValues::Literals(BTreeSet::new())),
    }
}

/// What a resolved callee receives at an invocation site: the call's own
/// argument words (an ordinary or dispatched call, and the tail for a
/// registry-declared user-proc invoker), or nothing this scan can see (a
/// callback prefix, whose arguments the runtime appends).
#[derive(Clone, Copy)]
enum IndirectArgs<'a> {
    /// The callee receives exactly these argument words.
    Words(&'a [String]),
    /// The runtime appends arguments this scan cannot see.
    Unknowable,
}

/// Attribute one invocation of `word` (a command name, possibly dispatched
/// through a variable) to every procedure it may reach.
fn record_invocation(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    word: &str,
    args: IndirectArgs<'_>,
) {
    if is_dynamic_word(word) {
        out.consulted_value_sets = true;
    }
    let names = match word_values(ctx, caller, word) {
        WordValues::Unknown => {
            out.opaque_callee = true;
            return;
        }
        WordValues::Literals(names) => names,
    };
    for name in names {
        let Some(target) = resolve_target(ctx, caller.resolve_as, &name) else {
            continue;
        };
        let nparams = ctx.procedures.get(&target).map_or(0, |p| p.params.len());
        match args {
            IndirectArgs::Words(words) => out.record_call(&target, words, nparams),
            IndirectArgs::Unknowable => out.record_unknown_call(&target, nparams),
        }
    }
}

/// True when the whole word is a single substitution — `$v`, `${v}`, or
/// `[cmd …]` — so its *value*, not its text, is the script / lambda /
/// command prefix the command receives.
///
/// The distinction matters for every script-bearing argument: `catch {puts
/// $x}` carries the literal script text `puts $x` (which merely *contains*
/// a substitution), whereas `eval $script` carries no script text at all —
/// only a reference to one.  Testing `is_dynamic_word` alone conflates the
/// two and would treat every ordinary braced body that mentions a variable
/// as unresolvable.
fn word_is_whole_substitution(word: &str) -> bool {
    is_pure_var_ref(word) || parse_command_substitution(word).is_some()
}

/// The head (first word) of a command-prefix argument.
///
/// A prefix is normally a literal list whose first element is the command
/// (`lsort -command {compare -nocase}`).  When it was *built* by a
/// registry-declared prefix builder (`-command [list cb $x]`,
/// [`Traits::BUILDS_COMMAND_PREFIX`]) the head is that builder's own first
/// argument instead — recognising the shape generically, with no command
/// name in this consumer.
fn command_prefix_head(registry: &CommandRegistry, prefix: &str) -> Option<String> {
    if let Some((builder, built)) = parse_command_substitution(prefix) {
        let bare = builder.strip_prefix("::").unwrap_or(builder.as_str());
        if registry
            .get(bare)
            .is_some_and(|spec| spec.traits.contains(Traits::BUILDS_COMMAND_PREFIX))
        {
            return built.first().cloned();
        }
        // Some other command substitution computed the prefix.
        return None;
    }
    prefix.split_whitespace().next().map(ToOwned::to_owned)
}

/// Record one call site's literal-argument evidence into `out`, then
/// recurse into any [`ArgRole::Body`] argument of `command` (regardless of
/// whether `command` itself is a user proc) — a nested script embedded in a
/// nested script embedded in a nested script, and so on, up to
/// [`MAX_CALL_SITE_BODY_DEPTH`].
///
/// The body recursion is the fix for the residual gap issue #969's own root
/// cause left open: `catch { isEven 4 }`, a non-exact `switch` arm, a
/// literal `uplevel {…}` body, and friends all carry their nested script as
/// one opaque *argument string* to a builtin (`catch`, `switch`, `uplevel`)
/// that is never itself a user proc — so a flat, one-level walk resolves
/// `catch` (finds no matching proc, moves on) and never notices `isEven 4`
/// sitting inside its body argument at all.
///
/// The registry already knows which argument position of which command is a
/// script body (`ArgRole::Body`, driving the identical recursive call-graph
/// walk in [`crate::interprocedural::scan_source_for_calls`]) — so this
/// reuses that one fact via
/// [`tcl_registry::CommandRegistry::arg_indices_for_role`] and the shared
/// [`crate::segmenter`] rather than hand-rolling a second "which commands
/// embed scripts" list here.
fn record_call_site_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    command: &str,
    args: &[String],
    depth: u32,
) {
    record_invocation(out, ctx, caller, command, IndirectArgs::Words(args));
    if is_dynamic_word(command) {
        // Which of a computed head's arguments carry scripts, callbacks, or
        // a callee name is unknowable; `record_invocation` has already
        // accounted for the dispatch itself.
        return;
    }
    let bare = command.strip_prefix("::").unwrap_or(command);
    record_registry_indirections(out, ctx, caller, bare, args);
    if MAX_CALL_SITE_BODY_DEPTH.exceeded(depth + 1) {
        return;
    }
    recurse_script_arguments(out, ctx, caller, bare, args, depth);
}

/// Record the callers hidden behind a command's registry-declared
/// indirections: a user-proc invoker (`Traits::INVOKES_USER_PROC` — the
/// callee is named by the first argument and receives the rest) and every
/// command-prefix callback argument (`ArgRole::CommandPrefix` — the callee
/// is the prefix head and the runtime appends arguments this scan cannot
/// see).
fn record_registry_indirections(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    bare: &str,
    args: &[String],
) {
    if ctx
        .registry
        .get(bare)
        .is_some_and(|spec| spec.traits.contains(Traits::INVOKES_USER_PROC))
        && let Some((name, rest)) = args.split_first()
    {
        record_invocation(out, ctx, caller, name, IndirectArgs::Words(rest));
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in ctx
        .registry
        .arg_indices_for_role(bare, &arg_strs, ArgRole::CommandPrefix)
    {
        let Some(prefix) = args.get(idx) else {
            continue;
        };
        match command_prefix_head(ctx.registry, prefix) {
            Some(head) => record_invocation(out, ctx, caller, &head, IndirectArgs::Unknowable),
            None => out.opaque_callee = true,
        }
    }
}

/// Walk every script-bearing argument of one call site.
///
/// A script written literally — even one that *mentions* a variable, as
/// nearly every real body does — is re-segmented and scanned in place.  One
/// the command receives only as a *value* (`eval $script`, `catch $body`,
/// `apply $fn`) is a script this scan cannot read: it may call any
/// procedure with any arguments, and may assign any variable, so it makes
/// the module's callee set unenumerable.
///
/// A literal [`ArgRole::LambdaLiteral`] needs no walking here — lowering
/// gives it its own body unit, which the whole-module scan already visits
/// as a caller.
fn recurse_script_arguments(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    bare: &str,
    args: &[String],
    depth: u32,
) {
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for role in [ArgRole::Body, ArgRole::LambdaLiteral] {
        for idx in ctx.registry.arg_indices_for_role(bare, &arg_strs, role) {
            let Some(text) = args.get(idx) else {
                continue;
            };
            if word_is_whole_substitution(text) {
                out.opaque_callee = true;
            } else if role == ArgRole::Body {
                scan_nested_script(out, ctx, caller, text, depth);
            }
        }
    }
}

/// Segment one nested script and record every call site inside it.
fn scan_nested_script(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_>,
    caller: &CallerFrame<'_>,
    script: &str,
    depth: u32,
) {
    let nested = crate::segmenter::segment_commands_with_offset_and_config(
        script,
        0,
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    );
    for cmd in &nested {
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        record_call_site_evidence(out, ctx, caller, name, cmd.args(), depth + 1);
    }
}

/// Everything [`collect_call_site_constants`] needs, grouped so the entry
/// point takes one borrow rather than six positional parameters.
pub(crate) struct CallSiteScanInputs<'a> {
    /// Whole-module control-flow graphs (top level + every procedure).
    pub(crate) cfg_module: &'a CfgModule,
    /// `TclOO` method and synthetic body-unit callers — see
    /// [`ExtraCaller`].
    pub(crate) extra_callers: &'a [ExtraCaller],
    /// Every procedure defined in the module, by qualified name.
    pub(crate) procedures: &'a HashMap<String, crate::ir::Procedure>,
    /// `namespace import` directives from
    /// [`crate::ir::Module::namespace_imports`].
    pub(crate) namespace_imports: &'a [(String, String)],
    /// The command registry every per-command fact is read from.
    pub(crate) registry: &'a CommandRegistry,
    /// Analysis dialect, for re-segmenting nested scripts.
    pub(crate) dialect: &'a str,
}

/// Collect literal arg values per user-proc call site across the whole
/// module's CFGs (top-level + every proc/method/body-unit, statements
/// already flattened), including calls nested inside [`ArgRole::Body`]
/// arguments and calls reached through an indirection.
///
/// Each literal call site is resolved to its callee via
/// [`crate::interprocedural::resolve_internal_call`] — Tcl's real,
/// existence-checked, namespace-relative resolution order, evaluated in the
/// *calling* function's own namespace, not the global one. This is the same
/// resolver the analyser and optimiser use for identical same-file call
/// resolution; a bespoke or partial resolver here could disagree with them
/// on which callee a bare name reaches.
///
/// The namespace context matters because a call site this scan fails to
/// resolve doesn't just go uncounted — it *vanishes* from the "every caller
/// passes the same literal" evidence, which can flip an absence of
/// contradicting evidence into a false positive (issue #969: a proc
/// declared inside a `namespace eval` block recursed into itself by its
/// bare name, so only the one external caller's literal remained and a
/// genuinely alternating parity check folded to a fixed boolean).
///
/// Indirect call sites (`$cmd dev`, `eval $script`, a callback prefix) are
/// resolved by *value*, which needs the evidence this function produces:
/// a dispatch word that is a procedure's own parameter takes the literals
/// its callers pass.  That circularity is resolved by an optimistic
/// fixpoint — each round re-derives the whole evidence set from the
/// previous round's, starting from "no callers seen".  Every round's result
/// is monotone in its input (values only union, unknown flags only set), so
/// the chain increases to a fixpoint at which the value sets and the
/// evidence agree.  Rounds run only when a value set was actually consulted,
/// so the overwhelmingly common indirection-free module costs exactly one
/// walk.
pub(crate) fn collect_call_site_constants(inputs: &CallSiteScanInputs<'_>) -> CallSiteEvidence {
    let known: HashSet<String> = inputs.procedures.keys().cloned().collect();
    let var_facts = collect_module_scope_var_facts(inputs);
    let mut evidence = CallSiteEvidence::default();
    for round in 0..MAX_CALL_SITE_SCAN_ROUNDS {
        let ctx = CallSiteScanCtx {
            procedures: inputs.procedures,
            known: &known,
            registry: inputs.registry,
            dialect: inputs.dialect,
            namespace_imports: inputs.namespace_imports,
            var_facts: &var_facts,
            previous: &evidence,
        };
        let next = scan_round(inputs, &ctx);
        // Nothing consulted a value set, so no later round can differ.
        if !next.consulted_value_sets {
            return next;
        }
        if next == evidence {
            return next;
        }
        if round + 1 == MAX_CALL_SITE_SCAN_ROUNDS {
            // Not converged within the backstop: trust nothing rather than
            // a non-fixpoint.
            let mut bail = next;
            bail.opaque_callee = true;
            return bail;
        }
        evidence = next;
    }
    evidence
}

/// One fixpoint round: walk every caller body and record all the evidence
/// it implies, reading value sets from `ctx.previous`.
fn scan_round(inputs: &CallSiteScanInputs<'_>, ctx: &CallSiteScanCtx<'_>) -> CallSiteEvidence {
    let mut out = CallSiteEvidence::default();
    for (caller, func) in module_callers(inputs) {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. }) = stmt
                else {
                    continue;
                };
                record_call_site_evidence(&mut out, ctx, &caller, command, args, 0);
            }
        }
    }
    out
}

/// Per-scope local-variable literal facts for every body the scan walks.
fn collect_module_scope_var_facts(inputs: &CallSiteScanInputs<'_>) -> ModuleVarFacts {
    let mut out = ModuleVarFacts::default();
    for (caller, func) in module_callers(inputs) {
        let facts = collect_scope_var_facts(func, inputs.registry);
        out.cross_frame_write |= facts.cross_frame_write;
        out.scopes.insert(caller.scope.to_owned(), facts);
    }
    out
}

/// Every body the whole-module scan treats as a caller, paired with the
/// frame its command words and variables resolve in.
///
/// The top level has no qualified name of its own; `"::top"` (the same
/// pseudo-qname `FunctionUnit::build_full` uses for it) resolves to the
/// global namespace via `resolve_internal_call`'s "drop the last segment"
/// rule, matching a bare top-level call's real resolution scope.
fn module_callers<'a>(
    inputs: &'a CallSiteScanInputs<'a>,
) -> impl Iterator<Item = (CallerFrame<'a>, &'a CfgFunction)> {
    std::iter::once((
        CallerFrame {
            resolve_as: "::top",
            scope: "::top",
            params: &[],
            callers_tracked: true,
        },
        &inputs.cfg_module.top_level,
    ))
    .chain(inputs.cfg_module.procedures.iter().map(|(q, f)| {
        (
            CallerFrame {
                resolve_as: q.as_str(),
                scope: q.as_str(),
                params: inputs
                    .procedures
                    .get(q)
                    .map_or(&[][..], |p| p.params.as_slice()),
                callers_tracked: true,
            },
            f,
        )
    }))
    .chain(inputs.extra_callers.iter().map(|extra| {
        (
            CallerFrame {
                resolve_as: extra.resolve_as.as_str(),
                scope: extra.scope.as_str(),
                params: extra.params.as_slice(),
                callers_tracked: false,
            },
            &extra.cfg,
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg;
    use crate::lowering::lower_to_ir;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Build the evidence for `src` under a named dialect's registry.
    fn evidence_for_dialect(src: &str, dialect: &str) -> CallSiteEvidence {
        let reg = tcl_registry::registry_for_dialect(dialect);
        let ir = lower_to_ir(src, reg);
        let cfg_module = build_cfg(&ir, false);
        collect_call_site_constants(&CallSiteScanInputs {
            cfg_module: &cfg_module,
            extra_callers: &[],
            procedures: &ir.procedures,
            namespace_imports: &ir.namespace_imports,
            registry: reg,
            dialect,
        })
    }

    /// Build the evidence for `src` exactly as `CompilationUnit` does,
    /// minus the method / body-unit callers (which need a CFG context the
    /// unit tests here don't exercise).
    fn evidence(src: &str) -> CallSiteEvidence {
        let reg = registry();
        let ir = lower_to_ir(src, &reg);
        let cfg_module = build_cfg(&ir, false);
        collect_call_site_constants(&CallSiteScanInputs {
            cfg_module: &cfg_module,
            extra_callers: &[],
            procedures: &ir.procedures,
            namespace_imports: &ir.namespace_imports,
            registry: &reg,
            dialect: "tcl",
        })
    }

    /// The literal values recorded at `index` for `qname`, sorted, plus
    /// whether that position is poisoned.
    fn slot(ev: &CallSiteEvidence, qname: &str, index: usize) -> (Vec<String>, bool) {
        let Some(consts) = ev.callee(qname).and_then(|by_idx| by_idx.get(&index)) else {
            return (Vec::new(), false);
        };
        let mut values: Vec<String> = consts.values.iter().cloned().collect();
        values.sort();
        (values, consts.unknown)
    }

    #[test]
    fn literal_call_sites_are_recorded_and_the_module_stays_enumerable() {
        let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nhelper b\n");
        assert_eq!(
            slot(&ev, "::helper", 0),
            (vec!["a".into(), "b".into()], false)
        );
        assert!(ev.enumerates_every_caller());
    }

    #[test]
    fn a_dispatch_through_a_literal_variable_is_recorded_as_a_call_site() {
        let ev = evidence("proc helper {mode} { return $mode }\nset cmd helper\n$cmd dev\n");
        assert_eq!(slot(&ev, "::helper", 0), (vec!["dev".into()], false));
        assert!(ev.enumerates_every_caller());
    }

    /// A registry-declared user-proc invoker (`Traits::INVOKES_USER_PROC`,
    /// the iRules `call PROC ?args?` form) names its callee in the first
    /// argument and passes the rest — a call site a walk that only ever
    /// looked at the command word would miss entirely.
    #[test]
    fn a_registry_declared_user_proc_invoker_is_a_call_site() {
        let ev = evidence_for_dialect(
            "proc helper {mode} { return $mode }\nwhen RULE_INIT { call helper dev }\n",
            "irules",
        );
        assert_eq!(slot(&ev, "::helper", 0), (vec!["dev".into()], false));
        assert!(ev.enumerates_every_caller());
    }

    #[test]
    fn an_unenumerable_dispatch_marks_the_module_non_enumerable() {
        let ev = evidence("proc helper {mode} { return $mode }\nset cmd [gets stdin]\n$cmd dev\n");
        assert!(!ev.enumerates_every_caller());
    }

    #[test]
    fn a_callback_prefix_poisons_every_parameter_of_its_target() {
        let ev = evidence("proc cmp {a b} { return 0 }\nlsort -command cmp {x y}\n");
        assert_eq!(slot(&ev, "::cmp", 0), (Vec::new(), true));
        assert_eq!(slot(&ev, "::cmp", 1), (Vec::new(), true));
        assert!(
            ev.enumerates_every_caller(),
            "the callback's target is known by name — only its arguments are not",
        );
    }

    #[test]
    fn an_omitted_defaulted_argument_poisons_its_slot() {
        let ev = evidence("proc helper {a b} { return $a }\nhelper one\n");
        assert_eq!(slot(&ev, "::helper", 0), (vec!["one".into()], false));
        assert_eq!(slot(&ev, "::helper", 1), (Vec::new(), true));
    }

    #[test]
    fn a_script_the_scan_cannot_read_makes_the_module_non_enumerable() {
        for src in [
            "proc helper {mode} { return $mode }\nset s {helper dev}\neval $s\n",
            "proc helper {mode} { return $mode }\ncatch $body\n",
            "proc helper {mode} { return $mode }\napply $fn 1\n",
        ] {
            assert!(
                !evidence(src).enumerates_every_caller(),
                "a script received as a value may call anything: {src}",
            );
        }
    }

    #[test]
    fn a_literal_body_mentioning_a_variable_is_still_walked() {
        let ev = evidence("proc helper {mode} { return $mode }\ncatch {helper $x}\n");
        assert_eq!(slot(&ev, "::helper", 0), (Vec::new(), true));
        assert!(
            ev.enumerates_every_caller(),
            "`catch {{helper $x}}` carries readable script text",
        );
    }

    #[test]
    fn whole_substitution_recognises_only_a_single_reference() {
        assert!(word_is_whole_substitution("$script"));
        assert!(word_is_whole_substitution("${script}"));
        assert!(word_is_whole_substitution("[build]"));
        assert!(
            !word_is_whole_substitution("puts $v"),
            "a braced body that merely mentions a variable is literal script text",
        );
        assert!(!word_is_whole_substitution("helper dev"));
    }

    #[test]
    fn command_prefix_head_reads_a_built_prefix_through_the_registry() {
        let reg = registry();
        assert_eq!(
            command_prefix_head(&reg, "cmp -nocase").as_deref(),
            Some("cmp"),
        );
        assert_eq!(
            command_prefix_head(&reg, "[list cmp $x]").as_deref(),
            Some("cmp"),
            "`list` carries BUILDS_COMMAND_PREFIX, so its first argument is the head",
        );
        assert_eq!(
            command_prefix_head(&reg, "[pickCallback]"),
            None,
            "some other substitution computed the prefix — the head is unknown",
        );
    }

    #[test]
    fn scope_var_facts_separate_literal_from_unknown_writes() {
        let reg = registry();
        let ir = lower_to_ir(
            "proc p {} {\n set a one\n set b [gets stdin]\n set c two\n set c three\n}\n",
            &reg,
        );
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(cfg, &reg);
        assert!(!facts.dynamic_name_write);
        assert_eq!(
            facts.vars["a"].values.iter().cloned().collect::<Vec<_>>(),
            vec!["one".to_owned()],
        );
        assert!(facts.vars["b"].unknown);
        assert_eq!(
            facts.vars["c"].values.iter().cloned().collect::<Vec<_>>(),
            vec!["three".to_owned(), "two".to_owned()],
        );
    }

    #[test]
    fn a_write_through_a_computed_variable_name_poisons_the_whole_scope() {
        let reg = registry();
        let ir = lower_to_ir("proc p {n} {\n set a one\n set $n two\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        assert!(collect_scope_var_facts(cfg, &reg).dynamic_name_write);
    }

    #[test]
    fn a_scope_alias_declaration_makes_the_aliased_name_unknown() {
        let reg = registry();
        let ir = lower_to_ir("proc p {} {\n global cmd\n set other one\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(cfg, &reg);
        assert!(
            facts.vars["cmd"].unknown,
            "`global cmd` binds an outer variable any other body may write",
        );
    }
}
