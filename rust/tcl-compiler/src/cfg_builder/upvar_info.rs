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

//! Per-proc **frame-effect summary** — which names a procedure injects into
//! the frame of whoever calls it.
//!
//! Tcl procedures routinely write their caller's variables: `upvar` aliases
//! a caller-frame name into a local, and `uplevel` runs a whole script up
//! there.  Neither shows up in the caller's own text, so single-function
//! dataflow sees a read of a variable nothing ever assigned.  This module
//! computes, once per procedure, *what a call to it does to the caller's
//! frame*; [`super::CfgBuilder::apply_upvar_invalidation`] then consults it
//! at every call site — a hash lookup plus work proportional to the bound
//! arguments, with no fixpoint.
//!
//! The type is still named [`UpvarInfo`] because the salsa interning layer
//! (`tcl-lsp-db`) stores it by that path; it covers `uplevel` as well.
//!
//! ## What the summary records
//!
//! **Named caller-frame bindings** — `upvar`, resolvable to a name:
//!
//! * **`literal_targets`** — `upvar 1 caller_name local_name`: the source
//!   name is a literal (no `$` substitution), so we know exactly which
//!   caller-frame variable the local aliases.
//! * **`param_targets`** — `upvar 1 $param local_name`: the source is a
//!   `$<param>` reference where the proc has a parameter named
//!   `<param>`.  Resolved at call-site by substituting the actual
//!   argument value passed for that parameter.
//! * **`args_tail_upvar`** — `upvar 1 args local_name`: the source is the
//!   proc's `args` parameter (the trailing-vararg slot).  Each
//!   call-site arg in the tail position is substituted into the alias.
//!
//! **Named caller-frame writes without an alias** — `uplevel`, whose script
//! runs in the caller's frame rather than binding a local:
//!
//! * **`uplevel_literal_writes`** — `uplevel 1 {set n …}` (a brace-literal
//!   body the lowering already turned into [`Statement::UpFrame`]) and
//!   `uplevel 1 [list ::set n …]`. An unqualified constructed command head
//!   resolves in the selected caller namespace and therefore widens.
//! * **`uplevel_param_writes`** — a caller-side name supplied through a
//!   parameter even when the local alias name is dynamic (`upvar 1 $param
//!   $local`), resolved at the call site exactly like `param_targets`.
//! * **`uplevel_forwarded_calls`** — `uplevel 1 [list ::worker target]`: the
//!   constructed script *calls* another proc, so that proc's own `upvar 1`
//!   reaches one frame further out than a plain call would — into **this**
//!   proc's caller.  Resolved one hop, in
//!   [`super::detect_upvar_procs`], where the module-wide map exists.
//!
//! **Opaque caller-frame effects** — the effect is real but the names are
//! not knowable:
//!
//! * **`has_unresolvable_caller_target`** — `upvar 1 $computed x`.
//! * **`caller_frame_opaque_writes` / `caller_frame_opaque_reads`** —
//!   `uplevel 1 $body`: arbitrary code in the caller's frame.
//!
//! ## Frame targeting is not decoration
//!
//! Only a **`Relative(1)`** level (or an omitted one) touches the direct
//! caller.  Everything else is a different frame and must not be reported
//! as a caller-frame binding — pinned on tclsh 9.0.4 and 8.6.14, identical:
//!
//! | written in the callee | frame written | in this summary |
//! |---|---|---|
//! | `upvar 1 x y` / `upvar x y` | the caller | named binding |
//! | `upvar #0 g l` | the global frame | nothing — [`super::global_write_info`] owns it |
//! | `upvar 0 x y` | the callee's **own** frame | nothing |
//! | `upvar 2 far f` | the caller's caller | widen (see below) |
//! | `upvar $lvl a b` | unknown | widen |
//! | `uplevel 1 $body` | the caller | opaque write **and** read |
//! | `eval $body` | the callee's own frame | nothing — [`crate::dynamic_names`] owns it |
//!
//! A level the summary cannot place at the direct caller is widened to an
//! opaque caller-frame write rather than dropped.  Dropping it would leave
//! a real `upvar 2` write invisible to the grandparent frame that receives
//! it, producing a false `W210`; widening only ever silences one. That is
//! the [documented abstention direction][soundness] for every consumer of
//! this summary.
//!
//! [soundness]: crate::dynamic_names

use std::collections::{BTreeMap, BTreeSet};

use tcl_registry::{CommandRegistry, InvocationArgument, InvocationArguments, InvocationWord};

use crate::command_binding::{ModuleCommandBindings, ResolvedBindingInvocation};
use crate::ir::{CommandTokens, ExecutionNamespace, Script, Statement};

mod body_walk;
mod frame_commands;
mod logical_frame_invocations;
mod selected_frame_bodies;

use body_walk::collect_upvar_targets_inner;

/// Per-proc summary of every `upvar` declaration in the body.
///
/// `literal_targets` /
/// `param_targets` / `args_tail_upvar` are mutually exclusive — each
/// `(source, local)` pair lands in exactly one bucket.  Maps key on
/// the *local* name (the alias target) so caller-side resolution can
/// look up "which caller-frame name does my local `x` alias?" in a
/// single hash hit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct UpvarInfo {
    /// `local_name -> caller_literal_name` for declarations whose
    /// source word is a plain string (no `$` / `[`).
    pub literal_targets: BTreeMap<String, String>,
    /// `local_name -> param_name` for declarations whose source word
    /// is `$<param>` and `<param>` is a proc parameter.  The
    /// caller-side resolver substitutes the actual argument passed
    /// for `<param>` to produce the caller-frame name.
    pub param_targets: BTreeMap<String, String>,
    /// `local_name`s whose source is `$args` (the trailing-vararg
    /// param).  Each call-site arg in the tail position is aliased
    /// to one of these locals; the per-local pairing is positional,
    /// the order here matches the order of declarations in the
    /// proc body.
    pub args_tail_upvar: Vec<String>,
    /// The proc has an `upvar` whose CALLER-side name cannot be resolved
    /// statically (`upvar 1 $computed x`, or `$foo` where `foo` is not a
    /// parameter): the callee can write *any* caller variable, so
    /// [`Self::caller_side_defs`] under-approximates and the call site must
    /// widen to an opaque caller-frame clobber instead of trusting it.
    pub has_unresolvable_caller_target: bool,
    /// Literal caller-frame names the proc may write with **no nameable
    /// local alias** to key on: an `uplevel` script that runs in the
    /// caller's frame (`uplevel 1 {set n 1}`, `uplevel 1 [list ::set n 1]`),
    /// and an `upvar` pair whose *local* side is dynamic (`upvar 1 n $dst`
    /// — the alias still targets exactly the caller's `n`, whatever local
    /// name it lands under).
    pub uplevel_literal_writes: BTreeSet<String>,
    /// Parameter names whose **value** names a caller-frame variable the
    /// proc may write without a nameable local alias — `upvar 1 $varName
    /// $dst` with a dynamic local side. Resolved at the call
    /// site against the actual argument passed for that parameter, exactly
    /// like [`Self::param_targets`].
    pub uplevel_param_writes: BTreeSet<String>,
    /// `uplevel <caller frame> [list CMD ARG…]` sites whose source-proven
    /// literal `CMD` is not a registry command — a candidate user proc whose
    /// own caller-frame
    /// effects land in *this* proc's caller.  Stored unresolved because a
    /// per-proc walk cannot see the module; [`super::detect_upvar_procs`]
    /// resolves each entry one hop against the completed map.
    ///
    /// Each entry is `(command, constructed argument words)`.
    pub uplevel_forwarded_calls: Vec<(String, Vec<String>)>,
    /// The proc runs a script it cannot read in its caller's frame
    /// (`uplevel 1 $body`), or aliases a caller-frame name it cannot place
    /// (`upvar 2 …`, `upvar $lvl …`).  **Any** caller-frame name may be
    /// created or overwritten, so a call site must widen instead of
    /// enumerating.
    pub caller_frame_opaque_writes: bool,
    /// The same script may **read** any caller-frame name, so no store in
    /// the caller can be proved dead across the call.
    pub caller_frame_opaque_reads: bool,
    /// Caller-side source words of alias pairs whose **local** side is
    /// dynamic (`upvar 1 x $dst` records `x`), as written — so a
    /// wholly-dynamic pair records its source word verbatim.
    ///
    /// Deliberately *not* part of [`Self::is_empty`]'s summary contract, and
    /// no caller-side resolver reads it: it exists for the strictly
    /// structural caller-frame reachability question — "can this body
    /// reach its caller's frame at all?" — where an alias nobody can name
    /// counts every bit as much as a nameable one.  The *caller-side effect*
    /// of such a pair is carried separately: a literal source
    /// goes into [`Self::uplevel_literal_writes`], a `$param` source into
    /// [`Self::uplevel_param_writes`], and anything else widens through
    /// [`Self::has_unresolvable_caller_target`] — all of which `is_empty`
    /// covers, so `detect_upvar_procs` registers the proc and its call
    /// sites widen.
    pub unnameable_local_aliases: BTreeSet<String>,
    /// The frames past the direct caller this proc may read or write
    /// ([`FrameTarget`]). Each one is opaque at its frame: any name there
    /// may be read, created or overwritten. An [`FrameTarget::Unknown`]
    /// target also widens the direct caller, which it may be.
    pub beyond_caller: BTreeSet<FrameTarget>,
    /// Binding-resolved terminal user procedures this proc invokes as
    /// **ordinary** calls (not through `uplevel`), deduplicated.
    ///
    /// Only [`super::detect_upvar_procs`]'s one-hop composition reads it, to
    /// find the callees whose [`Self::beyond_caller`] effects land in this
    /// proc's caller or further out. Like [`Self::unnameable_local_aliases`] it is
    /// structural bookkeeping, not a caller-side effect, so it stays out of
    /// [`Self::is_empty`].
    pub plain_calls: BTreeSet<String>,
    /// Each parameter whose value names a caller-frame variable
    /// ([`Self::param_targets`], [`Self::uplevel_param_writes`]) that has a
    /// default: the name a call omitting it binds it to, as Tcl decodes the
    /// default, or `None` where the parameter list cannot be read. A
    /// parameter with no default has no entry: a call omitting it raises.
    pub param_defaults: BTreeMap<String, Option<String>>,
}

/// What a call omitting a parameter whose value names a caller-frame
/// variable binds the parameter to ([`UpvarInfo::default_name`]).
pub(super) enum OmittedName<'a> {
    /// Nothing: the parameter has no default, and the call raises.
    Raises,
    /// The caller-frame variable its default names.
    Names(&'a str),
    /// An array-element default or an unreadable parameter list retains
    /// the caller-frame widening obligation.
    Unplaceable,
}

#[derive(Debug, Default)]
pub(super) struct CallerSideEffects {
    pub defs: Vec<String>,
    pub opaque: bool,
}

fn push_caller_side_def(effects: &mut CallerSideEffects, name: &str) {
    let name = crate::naming::split_element_ref(name).map_or(name, |(root, _)| root);
    if !effects.defs.iter().any(|found| found == name) {
        effects.defs.push(name.to_owned());
    }
}

use super::frame_effects::FrameContext;
pub use super::frame_effects::{FrameTarget, MAX_FRAME_DISTANCE};

/// Point `local` at `target`, returning a different target it displaced.
fn relink(links: &mut BTreeMap<String, String>, local: String, target: String) -> Option<String> {
    match links.entry(local) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(target);
            None
        }
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            (*entry.get() != target).then(|| entry.insert(target))
        }
    }
}

impl UpvarInfo {
    /// Link `local` to the caller's literal `target`. A local re-linked to a
    /// different name may already have written the first through the earlier
    /// link, so that name stays a keyless caller-side write (tclsh 8.4 to
    /// 9.1: `proc q {} {upvar 1 x v; set v 1; upvar 1 y v}` sets the caller's
    /// `x`).
    pub(super) fn link_literal(&mut self, local: String, target: String) {
        if let Some(previous) = relink(&mut self.literal_targets, local, target) {
            self.uplevel_literal_writes.insert(previous);
        }
    }

    /// Link `local` to the caller variable named by `parameter`'s value; a
    /// displaced parameter keeps its write, as for [`Self::link_literal`].
    pub(super) fn link_parameter(&mut self, local: String, parameter: String) {
        if let Some(previous) = relink(&mut self.param_targets, local, parameter) {
            self.uplevel_param_writes.insert(previous);
        }
    }

    /// True if every bucket is empty — no `upvar` declarations in
    /// the proc body.  Callers with no upvar info can skip the
    /// per-call resolution path.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.literal_targets.is_empty()
            && self.param_targets.is_empty()
            && self.args_tail_upvar.is_empty()
            && self.uplevel_literal_writes.is_empty()
            && self.uplevel_param_writes.is_empty()
            && self.uplevel_forwarded_calls.is_empty()
            // A flag-only summary still matters: an unresolvable caller
            // target means call sites must widen, so the info must not be
            // dropped by `detect_upvar_procs`'s emptiness filter.
            && !self.has_unresolvable_caller_target
            && !self.caller_frame_opaque_writes
            && !self.caller_frame_opaque_reads
            && self.beyond_caller.is_empty()
    }

    /// Whether any effect lands past the direct caller.
    #[must_use]
    pub fn reaches_past_the_caller(&self) -> bool {
        !self.beyond_caller.is_empty()
    }

    /// The caller-frame barrier a call to this proc imposes on the calling
    /// function: `writes` when any name in the caller's frame may be
    /// created or overwritten under a name this analysis cannot enumerate,
    /// `reads` when any store in the caller may be observed.
    ///
    /// `destroys` rides with `reads`: only an arbitrary script
    /// (`uplevel 1 $body`) can `unset` a caller-frame name, and that is
    /// exactly the case that sets `reads` too.  An unresolvable `upvar`
    /// target writes through an alias, which cannot destroy the binding.
    #[must_use]
    pub fn caller_frame_barrier(&self) -> crate::dynamic_names::DynamicNameBarrier {
        crate::dynamic_names::DynamicNameBarrier {
            writes: self.caller_frame_opaque_writes || self.has_unresolvable_caller_target,
            destroys: self.caller_frame_opaque_reads,
            reads: self.caller_frame_opaque_reads,
        }
    }

    /// What a call omitting `param` binds it to ([`OmittedName`]).
    pub(super) fn default_name(&self, param: &str) -> OmittedName<'_> {
        match self.param_defaults.get(param) {
            None => OmittedName::Raises,
            Some(Some(name)) if crate::naming::split_element_ref(name).is_none() => {
                OmittedName::Names(name)
            }
            Some(_) => OmittedName::Unplaceable,
        }
    }

    /// Resolve the caller-frame name aliased by *local* in this
    /// proc.  Returns the literal name for `literal_targets`, the
    /// substituted argument value for `param_targets` (looked up in
    /// the supplied `args` map), and `None` when *local* isn't an
    /// upvar target or when the param substitution fails.
    ///
    /// `args` maps proc parameter names to the literal value passed
    /// at the call site (or `None` when the value is dynamic).
    /// `args_tail_upvar` resolution is positional and lives outside
    /// this helper — caller-side logic walks the per-local list and
    /// pairs each entry with the corresponding tail-position
    /// argument.
    #[must_use]
    pub fn resolve_literal_or_param(
        &self,
        local: &str,
        args: &BTreeMap<String, Option<String>>,
    ) -> Option<String> {
        if let Some(lit) = self.literal_targets.get(local) {
            return Some(lit.clone());
        }
        if let Some(param) = self.param_targets.get(local) {
            return args.get(param).and_then(Clone::clone);
        }
        None
    }

    /// Resolve every upvar declaration in this proc against a call
    /// site, returning the unique set of caller-frame variable names
    /// the proc may modify via its upvars.
    ///
    /// `call_args` is the positional argument list at the call site;
    /// `params` is the callee proc's parameter list (used to locate
    /// param-based upvar sources by positional index).  Resolution:
    ///
    /// * Every value in [`literal_targets`](Self::literal_targets) is
    ///   added directly — those are caller-frame literal names.
    /// * For each value in [`param_targets`](Self::param_targets), we
    ///   find the param's positional index in `params`, look up the
    ///   actual argument value at that index in `call_args` — the name
    ///   the parameter's default spells where the call omits it
    ///   ([`Self::param_defaults`]) — and normalise it (stripping `$` /
    ///   `${…}` / `(…)` suffixes).
    /// * For each entry in [`args_tail_upvar`](Self::args_tail_upvar),
    ///   we pair it positionally with a tail argument at the call
    ///   site (tail start = `params.len() - 1`, since `args` consumes
    ///   the trailing slot in the param list).
    #[cfg(test)]
    #[must_use]
    pub fn caller_side_defs(&self, call_args: &[String], params: &[String]) -> Vec<String> {
        let mut defs: Vec<String> = Vec::new();
        let mut push = |name: String| {
            if !name.is_empty() && !defs.contains(&name) {
                defs.push(name);
            }
        };
        for caller_lit in self
            .literal_targets
            .values()
            .chain(self.uplevel_literal_writes.iter())
        {
            push(caller_lit.clone());
        }
        for param_name in self
            .param_targets
            .values()
            .chain(self.uplevel_param_writes.iter())
        {
            let Some(idx) = params.iter().position(|p| p == param_name) else {
                continue;
            };
            let arg = match call_args.get(idx) {
                Some(arg) => arg.as_str(),
                None => match self.default_name(param_name) {
                    OmittedName::Names(name) => name,
                    OmittedName::Raises | OmittedName::Unplaceable => continue,
                },
            };
            push(crate::naming::normalise_var_name(arg).to_owned());
        }
        if !self.args_tail_upvar.is_empty() {
            // `args` occupies the trailing param slot, so tail args
            // at the call site start at `params.len() - 1`.
            let tail_start = params.len().saturating_sub(1);
            for (i, _local) in self.args_tail_upvar.iter().enumerate() {
                if let Some(arg) = call_args.get(tail_start + i) {
                    push(crate::naming::normalise_var_name(arg).to_owned());
                }
            }
        }
        defs
    }

    /// Resolve caller-frame targets without treating substituted source text
    /// as the value Tcl passes. Expansion makes its position and the suffix
    /// after it opaque, while preserving any exactly-mapped argv prefix.
    #[must_use]
    pub(super) fn caller_side_effects(
        &self,
        call_args: InvocationArguments<'_>,
        params: &[String],
    ) -> CallerSideEffects {
        let mut effects = CallerSideEffects::default();
        for name in self
            .literal_targets
            .values()
            .chain(self.uplevel_literal_writes.iter())
        {
            push_caller_side_def(&mut effects, name);
        }
        for param_name in self
            .param_targets
            .values()
            .chain(self.uplevel_param_writes.iter())
        {
            if let Some(index) = params.iter().position(|param| param == param_name) {
                match call_args.argv_at(index) {
                    InvocationArgument::Word(InvocationWord::Literal(name)) => {
                        push_caller_side_def(&mut effects, name);
                    }
                    InvocationArgument::Word(_) | InvocationArgument::Indeterminate => {
                        effects.opaque = true;
                    }
                    InvocationArgument::Missing => match self.default_name(param_name) {
                        OmittedName::Names(name) => push_caller_side_def(&mut effects, name),
                        OmittedName::Unplaceable => effects.opaque = true,
                        OmittedName::Raises => {}
                    },
                }
            }
        }
        if !self.args_tail_upvar.is_empty() {
            let tail_start = params.len().saturating_sub(1);
            for (index, _) in self.args_tail_upvar.iter().enumerate() {
                match call_args.argv_at(tail_start + index) {
                    InvocationArgument::Word(InvocationWord::Literal(name)) => {
                        push_caller_side_def(&mut effects, name);
                    }
                    InvocationArgument::Word(_) | InvocationArgument::Indeterminate => {
                        effects.opaque = true;
                    }
                    InvocationArgument::Missing => {}
                }
            }
        }
        effects
    }
}

/// Caller-frame reach under the exact issued method source and Registry.
/// Missing source namespace or owner correspondence remains opaque.
pub(crate) fn method_reaches_caller_frame(
    module: &crate::ir::Module,
    body_id: crate::ir::ProcedureId,
    registry: &CommandRegistry,
) -> bool {
    let Some(method) = module
        .methods
        .get(&body_id)
        .filter(|_| module.procedure_ids.owns(body_id))
    else {
        return true;
    };
    let Some(owner) = module.retained_source_bindings.as_deref().filter(|owner| {
        owner.matches_module(module, registry)
            && owner.matches_original_method_summary(body_id, method)
    }) else {
        return true;
    };
    if owner.positioned_for_module(module, registry).is_none() {
        return true;
    }
    if method.body.statements.is_empty() {
        return false;
    }
    let info = if module.source_metadata_input.is_none()
        && module.source_entry.metadata_context.is_standalone()
        && module.source_entry.native_entry.is_none()
        && !module.source_entry.unknown_entry
    {
        collect_upvar_targets_with_registry(&method.body, &method.params, registry)
    } else {
        let namespace = method
            .body
            .namespace_context
            .as_deref()
            .cloned()
            .or_else(|| match &method.execution_namespace {
                ExecutionNamespace::Exact(namespace) => Some(
                    crate::command_binding::SourceNamespaceKey::authored(namespace),
                ),
                ExecutionNamespace::SourceContext(context) => Some(context.clone()),
                ExecutionNamespace::RuntimeSelected => None,
            });
        let Some(namespace) = namespace else {
            return true;
        };
        let bindings = ModuleCommandBindings::analyse(module, registry);
        collect_upvar_targets_with_bindings(
            &method.body,
            &method.params,
            registry,
            &bindings,
            &namespace,
            module,
        )
    };
    !info.is_empty() || !info.unnameable_local_aliases.is_empty()
}

/// Collect the per-proc frame-effect summary from a proc body.
///
/// `params` is the proc's parameter list (used to gate
/// `param_targets`).  `body` is the lowered IR body.  Walks every
/// statement for a registry-declared frame-crossing command
/// ([`FrameEffectSpec`]) and accumulates the result in [`UpvarInfo`].
#[cfg(test)]
fn collect_upvar_targets(body: &Script, params: &[String]) -> UpvarInfo {
    collect_upvar_targets_with_registry(body, params, tcl_registry::default_registry())
}

/// Explicit standalone structural summary under the supplied Registry.
/// Supplied whole-source analysis uses the retained Module counterpart.
#[must_use]
pub(crate) fn collect_upvar_targets_with_registry(
    body: &Script,
    params: &[String],
    registry: &CommandRegistry,
) -> UpvarInfo {
    collect_upvar_targets_inner(
        body,
        params,
        registry,
        None,
        &crate::command_binding::SourceNamespaceKey::authored("::"),
    )
}

/// Whole-module counterpart which can prove the live binding of a
/// source-level list constructor before trusting its result as script words.
pub(super) fn collect_upvar_targets_with_bindings(
    body: &Script,
    params: &[String],
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::command_binding::SourceNamespaceKey,
    module: &crate::ir::Module,
) -> UpvarInfo {
    let Some((input, metadata)) =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
            .and_then(|metadata| {
                metadata
                    .source_analysis_input()
                    .map(|input| (input, metadata))
            })
    else {
        return UpvarInfo {
            caller_frame_opaque_writes: true,
            caller_frame_opaque_reads: true,
            has_unresolvable_caller_target: true,
            ..UpvarInfo::default()
        };
    };
    let bindings = UpvarBindings {
        source: bindings,
        input,
        metadata,
        module,
    };
    collect_upvar_targets_inner(body, params, registry, Some(bindings), namespace)
}

#[derive(Clone, Copy)]
struct UpvarBindings<'a> {
    module: &'a crate::ir::Module,
    source: &'a ModuleCommandBindings,
    input: &'a crate::analyser::ResolvedAnalysisInput,
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
}
impl UpvarBindings<'_> {
    fn invocation_dialect(self) -> Option<tcl_registry::InvocationDialect> {
        self.source.invocation_dialect()
    }
    fn resolve_statement(
        self,
        statement: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        self.source.resolve_statement_with_metadata_context(
            statement,
            registry,
            Some(self.metadata),
            namespace,
        )
    }
    fn resolve_command_words(
        self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        self.source.resolve_command_words_with_metadata_context(
            words,
            registry,
            Some(self.metadata),
            namespace,
        )
    }
    fn for_each_resolved_invocation<F>(
        self,
        statement: &Statement,
        namespace: &crate::command_binding::SourceNamespaceKey,
        visit: F,
    ) where
        F: for<'w> FnMut(
            &'w crate::command_binding::ResolvedCommandTarget,
            tcl_registry::InvocationWords<'w>,
        ),
    {
        self.source
            .for_each_resolved_invocation(statement, namespace, visit);
    }
    fn target_resolution_may_be_unknown(
        self,
        command: &str,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) -> bool {
        self.source
            .target_resolution_may_be_unknown(command, namespace)
    }
}

#[derive(Clone, Copy)]
struct LogicalFrameScan<'a> {
    tokens: &'a CommandTokens,
    current: FrameContext,
    params: &'a [String],
    registry: &'a CommandRegistry,
    bindings: UpvarBindings<'a>,
    namespace: &'a crate::command_binding::SourceNamespaceKey,
    depth: u32,
}

#[derive(Clone, Copy)]
struct FrameCommandContext<'a> {
    bindings: Option<UpvarBindings<'a>>,
    tokens: Option<&'a CommandTokens>,
    dialect: Option<tcl_registry::InvocationDialect>,
}

/// The defaults of the parameters whose value names a caller-frame variable
/// in `info`, read from the procedure's parameter list `params_raw` under
/// the document's word rules ([`UpvarInfo::param_defaults`]).
pub(super) fn name_param_defaults(
    info: &UpvarInfo,
    params_raw: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> BTreeMap<String, Option<String>> {
    let named: BTreeSet<&str> = info
        .param_targets
        .values()
        .chain(info.uplevel_param_writes.iter())
        .map(String::as_str)
        .collect();
    if named.is_empty() {
        return BTreeMap::new();
    }
    match crate::signature_scan::params::parse_param_list_strict(params_raw, rules) {
        Ok(formals) => formals
            .into_iter()
            .filter(|formal| named.contains(formal.name.as_str()))
            .filter_map(|formal| Some((formal.name, Some(formal.default?))))
            .collect(),
        Err(_) => named
            .into_iter()
            .map(|name| (name.to_owned(), None))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
