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

use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevel};
use tcl_registry::{CommandRegistry, InvocationArgument, InvocationArguments, InvocationWord};

use crate::command_binding::{
    ModuleCommandBindings, ResolvedBindingInvocation, ResolvedFrameBody, ResolvedFrameBodySelection,
};
use crate::ir::{CommandTokens, ExecutionNamespace, Script, Statement};

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
    /// How far out of this proc's own frame its frame effects reach — see
    /// [`FrameReach`].
    ///
    /// Not part of [`Self::is_empty`]'s summary contract on its own: every
    /// site that sets it past the caller also sets
    /// [`Self::caller_frame_opaque_writes`], which `is_empty` already covers.
    pub frame_reach: FrameReach,
    /// Binding-resolved terminal user procedures this proc invokes as
    /// **ordinary** calls (not through `uplevel`), deduplicated.
    ///
    /// Only [`super::detect_upvar_procs`]'s one-hop composition reads it, to
    /// find the callees whose [`FrameReach::PastTheCaller`] effects land in
    /// this proc's caller. Like [`Self::unnameable_local_aliases`] it is
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

/// How far out of a procedure's own frame its `upvar` / `uplevel` effects
/// reach, as far as the summary can tell.
///
/// A **level-1** effect (`upvar 1`, `uplevel 1`) writes the direct caller's
/// frame and stops there: it is emphatically not transitive through an
/// ordinary call, because the callee's caller *is* the wrapper
/// (`detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper`
/// pins the tclsh transcript). A level that lands **past** the direct caller
/// travels one hop further along an ordinary call, and
/// [`super::detect_upvar_procs`] composes that hop; without it a three-frame
/// chain reads as a plain call chain and the outermost frame draws a false
/// `W210` on a variable that really is assigned.
///
/// On tclsh 9.0.4 and 8.6.16, identically: with
/// `proc setUp2 {var} {uplevel 2 [list set $var 99]}`,
/// `proc middle {} {setUp2 answer}` and
/// `proc outer {} {middle; return $answer}`, `outer` returns `99` — `answer`
/// is genuinely set in `outer`'s frame by a proc `outer` never calls
/// directly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum FrameReach {
    /// Nothing lands further out than the direct caller, so the summary's
    /// other fields describe the whole effect.
    #[default]
    NoFurtherThanTheCaller,
    /// At least one effect lands past the direct caller — an `upvar 2` /
    /// `uplevel 2` / `uplevel #3`, or a level word nothing static can place
    /// (`uplevel [expr {[info level] - $n}] …`).
    PastTheCaller,
}

impl UpvarInfo {
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

fn original_scalar_argument<'a>(
    tokens: &'a CommandTokens,
    args: &[String],
    index: usize,
) -> Option<&'a str> {
    let config = tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let word = tokens.words().get(index.checked_add(1)?)?;
    if word.legacy_text() != *args.get(index)? {
        return None;
    }
    let (spelling, source) = word.sole_variable_substitution()?;
    if source.provenance != crate::ir::Provenance::Source {
        return None;
    }
    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
}

/// True if *name* is a plain literal name with no substitutions or
/// special characters.
fn is_literal_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('$')
        && !name.contains('[')
        && !name.contains('{')
        && !name.contains('"')
        && !name.contains(' ')
}

/// Resolve a frame-crossing command's level word and its remaining
/// arguments through the registry's [`FrameEffectSpec`].
fn resolve_frame_args<'a>(
    spec: FrameEffectSpec,
    args: &'a [String],
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    tokens: Option<&CommandTokens>,
) -> (FrameLevel, &'a [String]) {
    let Some(tokens) = tokens.filter(|tokens| {
        tokens.original_words_match_argv_text() && tokens.argv_texts.get(1..) == Some(args)
    }) else {
        return (FrameLevel::Dynamic, &[]);
    };
    let words = tokens
        .words()
        .iter()
        .skip(1)
        .map(crate::registry_invocation::invocation_word)
        .collect::<Vec<_>>();
    let mut arguments = InvocationArguments::Structured(&words).with_profile(registry.profile());
    if let Some(dialect) = dialect {
        arguments = arguments.with_dialect(dialect);
    }
    match spec.successful_layout(arguments).layout {
        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => (level, &args[level_word_len..]),
        tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
            (FrameLevel::Relative(0), &[])
        }
        tcl_registry::frame_effect::FrameArgumentResolution::Unknown => (FrameLevel::Dynamic, &[]),
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

fn collect_upvar_targets_inner(
    body: &Script,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
) -> UpvarInfo {
    let mut info = UpvarInfo::default();
    walk_script(body, params, registry, bindings, namespace, &mut info);
    info
}

fn walk_script(
    script: &Script,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    for stmt in &script.statements {
        walk_stmt(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            params,
            registry,
            bindings,
            namespace,
            info,
        );
        if let Some(bindings) = bindings
            && bindings.input.has_logical_source_name_context()
            && let Some(tokens) = script.retained_source_tokens_for_statement(stmt)
        {
            record_original_child_frame_effects(
                tokens, params, registry, bindings, namespace, info,
            );
        }
    }
}

fn record_original_child_frame_effects(
    tokens: &CommandTokens,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    let mut pending = vec![(tokens.clone(), 0_u32)];
    let mut seen = BTreeSet::new();
    while let Some((parent, depth)) = pending.pop() {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        }
        let Some(site) = parent
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
        else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        };
        if !seen.insert(site.offset) {
            continue;
        }
        let selected = crate::registry_invocation::original_logical_frame_invocation_for_module(
            bindings.module,
            registry,
            &parent,
        );
        let Some(selected) = selected else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        };
        if depth != 0 {
            record_logical_frame_invocation(
                &selected,
                LogicalFrameScan {
                    tokens: &parent,
                    current: FrameContext::Own,
                    params,
                    registry,
                    bindings,
                    namespace,
                    depth,
                },
                info,
            );
        }
        if selected.layout().is_none() {
            if let Some(children) = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
                bindings.module, registry, &parent,
            ) {
                pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            } else {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        if let Some(children) =
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                &parent,
                bindings.input.lexer_config(),
                registry,
                bindings.metadata,
            )
        {
            for child in children {
                if let Some(child) = child.tokens {
                    pending.push((child, depth + 1));
                } else {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
            }
        } else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
    }
}

#[allow(clippy::too_many_lines)] // One exhaustive IR walk keeps frame-context transitions coupled.
fn walk_stmt(
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    if let Some(bindings) = bindings
        && bindings.input.has_logical_source_name_context()
        && matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. })
    {
        let selected = tokens.and_then(|tokens| {
            let metadata = tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        });
        if let (Some(tokens), Some(selected)) = (tokens, selected) {
            record_logical_frame_invocation(
                &selected,
                LogicalFrameScan {
                    tokens,
                    current: FrameContext::Own,
                    params,
                    registry,
                    bindings,
                    namespace,
                    depth: 0,
                },
                info,
            );
        } else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
        return;
    }
    if let (Some(bindings), Some(tokens)) = (bindings, tokens)
        && bindings.input.has_logical_source_name_context()
        && matches!(
            stmt,
            Statement::UpFrame {
                absolute: false,
                frame_shift: 1,
                ..
            }
        )
    {
        let candidates = tokens.source_binding.as_ref().and_then(|binding| {
            let metadata = binding.original_invocation_metadata_for_module(
                tokens,
                bindings.module,
                registry,
            )?;
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )?
            .unknown_caller_variable_candidates(bindings.module, registry, tokens)
        });
        if let Some(candidates) = candidates {
            info.uplevel_literal_writes.extend(candidates.bindings);
        }
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            if let Some(bindings) = bindings {
                // Ordinary-call composition needs the effective retained user
                // procedure, not the source spelling. An alias, namespace
                // fallback, or user-defined unknown handler can all select a
                // different terminal procedure while adding argv of their own.
                bindings.for_each_resolved_invocation(stmt, namespace, |target, _| {
                    if !target.registry_backed {
                        record_plain_call(&target.command, info);
                    }
                });
                // The source spelling may be an alias of `upvar`/`uplevel`.
                // Resolve its terminal registry descriptors and their
                // alias-prepended argv before interpreting the frame grammar;
                // consulting the source head would drop a leading frame level
                // or miss the effect entirely.
                for invocation in bindings.resolve_statement(stmt, registry, namespace) {
                    let Some(spec) = invocation.facts.frame_effect else {
                        continue;
                    };
                    match spec.layout {
                        FrameArgLayout::AliasPairs => {
                            record_resolved_alias_pairs(
                                &invocation,
                                spec,
                                params,
                                registry,
                                bindings.invocation_dialect(),
                                info,
                            );
                        }
                        FrameArgLayout::OpaqueCallerVars => {}
                        FrameArgLayout::ScriptInCurrentFrame
                        | FrameArgLayout::ScriptInSelectedFrame => {
                            record_resolved_frame_body(
                                &invocation,
                                FrameContext::Own,
                                params,
                                registry,
                                bindings,
                                namespace,
                                &ExecutionNamespace::SourceContext(namespace.clone()),
                                info,
                                0,
                            );
                        }
                    }
                }
                // A closed may-binding still has an unknown alternative when
                // it cannot enumerate the invoked implementation. That
                // implementation may select any registry-declared frame
                // effect (or arbitrary caller-frame Tcl), so preserve the
                // known alternatives above and widen for the unknown one.
                if bindings.target_resolution_may_be_unknown(command, namespace) {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
                return;
            }
            if let Some(spec) = registry.frame_effect(command) {
                record_frame_effect(
                    spec,
                    args,
                    params,
                    registry,
                    FrameCommandContext {
                        bindings: None,
                        tokens,
                        dialect: stmt
                            .tokens()
                            .and_then(|tokens| tokens.source_binding.as_ref())
                            .and_then(|binding| binding.variable_context.invocation_dialect),
                    },
                    namespace,
                    info,
                );
            } else {
                record_plain_call(command, info);
            }
        }
        // `uplevel <level> {literal body}` — the lowering already proved the
        // body readable and inlined it here. Its writes land in the frame
        // `frame_shift`/`absolute` select, so only the caller-frame form
        // contributes directly; `#0` / `0` miss the caller entirely, and a
        // deeper level lands past it (recorded so the one-hop composition in
        // `detect_upvar_procs` can carry it out to the frame it really
        // reaches).
        Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            tokens,
            ..
        } => match (*absolute, *frame_shift) {
            (false, 1) => {
                record_upframe_body(
                    body,
                    upframe_body_is_braced(tokens.as_ref()),
                    registry,
                    bindings,
                    namespace,
                    info,
                );
            }
            // `uplevel 0` preserves the callee frame. Its direct writes stay
            // local, but a nested `uplevel 1` still reaches this procedure's
            // caller and must participate in the summary.
            (false, 0) => {
                walk_script(body, params, registry, bindings, namespace, info);
            }
            // `uplevel #0` selects the global frame, which is owned by the
            // global-write summary rather than this caller-frame projection.
            (true, 0) => {}
            _ => widen_beyond_caller(info),
        },
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(&c.body, params, registry, bindings, namespace, info);
            }
            if let Some(e) = else_body {
                walk_script(e, params, registry, bindings, namespace, info);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(init, params, registry, bindings, namespace, info);
            walk_script(next, params, registry, bindings, namespace, info);
            walk_script(body, params, registry, bindings, namespace, info);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => {
            walk_script(body, params, registry, bindings, namespace, info);
        }
        Statement::Block {
            body,
            namespace: body_namespace,
            ..
        } => {
            if let Some(context) = body.namespace_context.as_deref() {
                walk_script(body, params, registry, bindings, context, info);
            } else if bindings.is_none() {
                walk_script(
                    body,
                    params,
                    registry,
                    bindings,
                    &crate::command_binding::SourceNamespaceKey::authored(body_namespace),
                    info,
                );
            } else {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(b) = &arm.body {
                    walk_script(b, params, registry, bindings, namespace, info);
                }
            }
            if let Some(b) = default_body {
                walk_script(b, params, registry, bindings, namespace, info);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(body, params, registry, bindings, namespace, info);
            for h in handlers {
                walk_script(&h.body, params, registry, bindings, namespace, info);
            }
            if let Some(f) = finally_body {
                walk_script(f, params, registry, bindings, namespace, info);
            }
        }
        _ => {}
    }
}

/// Variable-frame position of an evaluated body relative to the procedure
/// whose [`UpvarInfo`] is being built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameContext {
    Own,
    DirectCaller,
    Global,
    PastTheCaller,
}

fn selected_frame_context(
    current: FrameContext,
    selection: ResolvedFrameBodySelection,
) -> FrameContext {
    match selection {
        ResolvedFrameBodySelection::Current
        | ResolvedFrameBodySelection::Selected(FrameLevel::Relative(0)) => current,
        ResolvedFrameBodySelection::Selected(FrameLevel::Absolute(0)) => FrameContext::Global,
        ResolvedFrameBodySelection::Selected(FrameLevel::Relative(1))
            if current == FrameContext::Own =>
        {
            FrameContext::DirectCaller
        }
        ResolvedFrameBodySelection::Selected(
            FrameLevel::Relative(_) | FrameLevel::Absolute(_) | FrameLevel::Dynamic,
        ) => FrameContext::PastTheCaller,
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

fn record_logical_frame_invocation(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    use tcl_registry::frame_effect::FrameArgumentResolution;
    let Some(layout) = invocation.layout() else {
        return;
    };
    let selection = match invocation.arguments() {
        Some(FrameArgumentResolution::Valid {
            level,
            level_word_len,
        }) => (level, invocation.offset() + level_word_len),
        Some(FrameArgumentResolution::Invalid) => return,
        Some(FrameArgumentResolution::Unknown) | None => {
            if layout != FrameArgLayout::ScriptInCurrentFrame {
                widen_beyond_caller(info);
                info.caller_frame_opaque_reads = true;
            }
            return;
        }
    };
    match layout {
        FrameArgLayout::AliasPairs => {
            record_logical_alias_pairs(invocation, selection, context, info);
        }
        FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame => {
            record_logical_frame_script(invocation, layout, selection, context, info);
        }
        FrameArgLayout::OpaqueCallerVars => {}
    }
}

fn record_logical_alias_pairs(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    selection: (FrameLevel, usize),
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    let (level, start) = selection;
    if level != FrameLevel::Relative(1) {
        if !level.is_global_frame() && !level.is_current_frame() {
            widen_beyond_caller(info);
        }
        return;
    }
    for source in (start..invocation.count()).step_by(2) {
        let Some(destination) = source
            .checked_add(1)
            .filter(|index| *index < invocation.count())
        else {
            info.has_unresolvable_caller_target = true;
            return;
        };
        let local = invocation.literal(destination);
        if let Some(target) = invocation.literal(source) {
            if let Some(local) = local {
                info.literal_targets.insert(local, target);
            } else {
                info.unnameable_local_aliases.insert(target.clone());
                info.uplevel_literal_writes.insert(target);
            }
        } else if let Some(parameter) = invocation.parameter(
            source,
            context.params,
            context.bindings.input.lexer_config(),
        ) {
            if parameter == "args" {
                if let Some(local) = local {
                    info.args_tail_upvar.push(local);
                } else {
                    info.has_unresolvable_caller_target = true;
                }
            } else if let Some(local) = local {
                info.param_targets.insert(local, parameter);
            } else {
                info.uplevel_param_writes.insert(parameter);
            }
        } else {
            info.has_unresolvable_caller_target = true;
        }
    }
}

fn record_logical_frame_script(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    layout: FrameArgLayout,
    selection: (FrameLevel, usize),
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    let selected = if layout == FrameArgLayout::ScriptInCurrentFrame {
        context.current
    } else {
        selected_frame_context(
            context.current,
            ResolvedFrameBodySelection::Selected(selection.0),
        )
    };
    if selected == FrameContext::Global {
        return;
    }
    if selected == FrameContext::PastTheCaller {
        widen_beyond_caller(info);
        info.caller_frame_opaque_reads = true;
        return;
    }
    let start = if layout == FrameArgLayout::ScriptInCurrentFrame {
        invocation.offset()
    } else {
        selection.1
    };
    let body = (start.checked_add(1) == Some(invocation.count()))
        .then(|| {
            invocation.source_body(
                context.bindings.module,
                context.registry,
                context.tokens,
                start,
            )
        })
        .flatten();
    let Some(body) = body else {
        if selected == FrameContext::DirectCaller {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
        return;
    };
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(context.depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    if selected == FrameContext::Own {
        walk_script(
            &body,
            context.params,
            context.registry,
            Some(context.bindings),
            context.namespace,
            info,
        );
    } else {
        record_upframe_body_at_depth(
            &body,
            true,
            context.registry,
            Some(context.bindings),
            context.namespace,
            info,
            context.depth + 1,
        );
    }
}

/// Consume the shared binding/registry projection of an evaluated body.  The
/// command-binding layer alone owns alias-prefix insertion and body-word
/// positioning; this module only composes the selected variable frame into a
/// caller-effect summary.
#[allow(clippy::too_many_arguments)]
fn record_resolved_frame_body(
    invocation: &ResolvedBindingInvocation,
    current: FrameContext,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    execution_namespace: &ExecutionNamespace,
    info: &mut UpvarInfo,
    depth: u32,
) {
    let body = invocation.resolved_frame_body_with_metadata_context(
        registry,
        bindings.source,
        execution_namespace,
        Some(bindings.metadata),
    );
    match body {
        ResolvedFrameBody::Readable { source, selection } => {
            let selected = selected_frame_context(current, selection);
            record_readable_frame_body(
                &source, selected, params, registry, bindings, namespace, info, depth,
            );
        }
        ResolvedFrameBody::Opaque { selection } => {
            match selected_frame_context(current, selection) {
                // An arbitrary script evaluated directly in the caller can
                // read, write, or unset any caller cell.
                FrameContext::DirectCaller => {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
                // The selected frame is further out.  Keep the established
                // summary convention: expose an opaque one-hop barrier and
                // retain the structural reach for interprocedural closure.
                FrameContext::PastTheCaller => {
                    widen_beyond_caller(info);
                    info.caller_frame_opaque_reads = true;
                }
                // Current-frame opacity is owned by dynamic_names; #0 is
                // owned by global_write_info.
                FrameContext::Own | FrameContext::Global => {}
            }
        }
        ResolvedFrameBody::NotApplicable | ResolvedFrameBody::KnownError => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn record_readable_frame_body(
    source: &str,
    selected: FrameContext,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    match selected {
        FrameContext::Global => {}
        FrameContext::PastTheCaller => {
            widen_beyond_caller(info);
            info.caller_frame_opaque_reads = true;
        }
        FrameContext::Own | FrameContext::DirectCaller => {
            let config = bindings.input.lexer_config();
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
                .with_dialect(Some(bindings.input.unit_profile()))
                .with_context_registry(bindings.input.context_registry());
            let module = lowerer.lower(source).clone();
            if selected == FrameContext::Own {
                walk_script(
                    &module.top_level,
                    params,
                    registry,
                    Some(bindings),
                    namespace,
                    info,
                );
            } else {
                record_upframe_body_at_depth(
                    &module.top_level,
                    false,
                    registry,
                    Some(bindings),
                    namespace,
                    info,
                    depth + 1,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct FrameCommandContext<'a> {
    bindings: Option<UpvarBindings<'a>>,
    tokens: Option<&'a CommandTokens>,
    dialect: Option<tcl_registry::InvocationDialect>,
}

/// Dispatch a registry-declared frame grammar after command binding has
/// supplied its effective argv. This stays command-neutral: aliases, renamed
/// commands, and direct spellings all consume the same descriptor.
fn record_frame_effect(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    match spec.layout {
        FrameArgLayout::AliasPairs => {
            record_upvar_call(spec, args, params, info, registry, context);
        }
        FrameArgLayout::ScriptInSelectedFrame => {
            record_uplevel_call(spec, args, registry, context, namespace, info);
        }
        // A script that runs in the callee's own frame, and an opaque
        // injection into whoever called the command, are effects on the
        // frame the command is written in — not on this proc's caller.
        // `crate::dynamic_names` owns them.
        FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::OpaqueCallerVars => {}
    }
}

/// An ordinary call shares values rather than frames. It remains composition
/// input only because a user procedure may itself reach past its caller.
fn record_plain_call(command: &str, info: &mut UpvarInfo) {
    info.plain_calls.insert(command.to_owned());
}

/// Record a frame effect that lands **past** the direct caller — an
/// `upvar 2` / `uplevel 2` / `uplevel #3`, or a level word nothing static can
/// place.
///
/// Two facts, not one. The caller-frame widening is what today's call-site
/// resolution consumes (a write it cannot enumerate must not be trusted as
/// absent), and the beyond-caller flag is what lets
/// [`super::detect_upvar_procs`] carry the effect one hop further out along
/// an ordinary call — the frame it actually reaches.
fn widen_beyond_caller(info: &mut UpvarInfo) {
    info.caller_frame_opaque_writes = true;
    info.frame_reach = FrameReach::PastTheCaller;
}

fn record_resolved_alias_pairs(
    invocation: &ResolvedBindingInvocation,
    spec: FrameEffectSpec,
    parameters: &[String],
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    use tcl_registry::frame_effect::FrameArgumentResolution;
    let (level, start) = match invocation.frame_alias_layout(spec, registry, dialect) {
        FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => (level, level_word_len),
        FrameArgumentResolution::Invalid => return,
        FrameArgumentResolution::Unknown => {
            widen_beyond_caller(info);
            return;
        }
    };
    if !level.is_caller_frame() {
        if !level.is_global_frame() && !level.is_current_frame() {
            widen_beyond_caller(info);
        }
        return;
    }
    for source in (start..invocation.arguments.len()).step_by(2) {
        let Some(destination) = source
            .checked_add(1)
            .filter(|index| *index < invocation.arguments.len())
        else {
            info.has_unresolvable_caller_target = true;
            return;
        };
        let local = invocation
            .literal_argument(destination)
            .filter(|name| is_literal_name(name));
        let target = invocation
            .literal_argument(source)
            .filter(|name| is_literal_name(name));
        if local.is_none() {
            info.unnameable_local_aliases
                .insert(invocation.arguments[source].clone());
        }
        if let Some(target) = target {
            if let Some(local) = local {
                info.literal_targets
                    .insert(local.to_owned(), target.to_owned());
            } else {
                info.uplevel_literal_writes.insert(target.to_owned());
            }
        } else if let Some(parameter) = invocation.scalar_parameter_argument(source, parameters) {
            if parameter == "args" {
                if let Some(local) = local {
                    info.args_tail_upvar.push(local.to_owned());
                } else {
                    info.has_unresolvable_caller_target = true;
                }
            } else if let Some(local) = local {
                info.param_targets
                    .insert(local.to_owned(), parameter.to_owned());
            } else {
                info.uplevel_param_writes.insert(parameter.to_owned());
            }
        } else {
            info.has_unresolvable_caller_target = true;
        }
    }
}

/// `upvar ?level? otherVar myVar …` — classify each pair against the frame
/// the level word selects.
fn record_upvar_call(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    info: &mut UpvarInfo,
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
) {
    let (level, rest) = resolve_frame_args(spec, args, registry, context.dialect, context.tokens);
    if !level.is_caller_frame() {
        // A level this summary cannot place at the direct caller. `#0` and
        // `0` genuinely miss the caller's frame (the global frame is
        // `global_write_info`'s business, the callee's own frame nobody
        // else's), so they contribute nothing. Everything else — `upvar 2`,
        // `upvar #3`, `upvar $lvl` — writes *some* frame further out, and
        // dropping it would leave that write invisible where it lands, so
        // widen. See the module doc's frame table.
        if !level.is_global_frame() && !level.is_current_frame() {
            widen_beyond_caller(info);
        }
        return;
    }
    let start = args.len().saturating_sub(rest.len());
    let mut i = 0;
    while i + 1 < rest.len() {
        let parameter = context
            .tokens
            .and_then(|tokens| original_scalar_argument(tokens, args, start + i))
            .filter(|name| params.iter().any(|parameter| parameter == name));
        let (src, dst) = (rest[i].as_str(), rest[i + 1].as_str());
        i += 2;
        let dst_is_literal = is_literal_name(dst);
        if !dst_is_literal {
            // Dynamic destination (`upvar 1 x $dst`) — there is no local
            // key to file the alias under, so the per-local maps cannot
            // carry it.  The alias is still real: record the structural
            // fact for caller-frame reachability, and classify the CALLER
            // side below so call sites still widen the right caller name
            // — otherwise such a proc reads as summary-empty and `p x` leaves
            // the caller's `x` foldable to a stale constant.
            // tclsh 9.0.4 / 8.6.16: `proc p {dst} {upvar 1 x $dst; set $dst
            // 2}; proc c {} {set x 1; p x; puts $x}` prints 2).
            info.unnameable_local_aliases.insert(src.to_string());
        }
        if is_literal_name(src) {
            if dst_is_literal {
                // `upvar 1 caller_x x` — literal source, literal local.
                info.literal_targets
                    .insert(dst.to_string(), src.to_string());
            } else {
                // `upvar 1 caller_x $dst` — the caller-side name is still
                // exactly `caller_x`; only the local key is unknowable, so
                // it lands in the keyless bucket.
                info.uplevel_literal_writes.insert(src.to_string());
            }
        } else if let Some(param) = parameter {
            // `upvar 1 $param x` — substitution-driven, gated on
            // the param existing on the proc.
            if param == "args" {
                if dst_is_literal {
                    info.args_tail_upvar.push(dst.to_string());
                } else {
                    // `upvar 1 $args $dst` — no positional local to pair
                    // the tail against; widen.
                    info.has_unresolvable_caller_target = true;
                }
            } else if params.iter().any(|p| p == param) {
                if dst_is_literal {
                    info.param_targets
                        .insert(dst.to_string(), param.to_string());
                } else {
                    // `upvar 1 $param $dst` — the caller-side name is the
                    // parameter's value, resolvable at the call site; only
                    // the local key is missing.
                    info.uplevel_param_writes.insert(param.to_string());
                }
            } else {
                // `$foo` where `foo` isn't a param — the caller-side name
                // depends on runtime state; the callee can clobber any
                // caller variable, so the summary must widen (a silent
                // skip would let SCCP propagate stale caller values past
                // the call).
                info.has_unresolvable_caller_target = true;
            }
        } else {
            // Fully dynamic source (`upvar 1 [pick] x`, `upvar 1 a($i) x`,
            // …) — same widening as above, whatever the local side.
            info.has_unresolvable_caller_target = true;
        }
    }
}

/// `uplevel ?level? arg …` — the concatenated `arg`s run as a script in the
/// frame the level selects.
///
/// Three body shapes are distinguishable, and only the third is opaque:
///
/// * a **brace literal** never reaches here — the lowering turned it into
///   [`Statement::UpFrame`], handled by [`record_upframe_body`];
/// * a **fully literal constructed list with an absolute head** (`[list ::set
///   v 1]`) has namespace-invariant result words, so
///   [`record_constructed_body`] can read it;
/// * anything else (`$body`, `[build]`, `"$a $b"`) is arbitrary code —
///   widen both directions.
fn record_uplevel_call(
    spec: FrameEffectSpec,
    args: &[String],
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    let (level, rest) = resolve_frame_args(spec, args, registry, context.dialect, context.tokens);
    if !level.is_caller_frame() {
        // Same reasoning as `record_upvar_call`: `uplevel 0` stays in the
        // callee's own frame and `uplevel #0` runs in the global one, so
        // neither touches the caller; any other level does, somewhere this
        // summary cannot name.
        if !level.is_global_frame() && !level.is_current_frame() {
            widen_beyond_caller(info);
            info.caller_frame_opaque_reads = true;
        }
        return;
    }
    // `uplevel` concat-joins every remaining word into one script (its
    // `SCRIPT_CONCATENATES_ARGS` trait). A single constructed-list word is
    // the readable case; a multi-word join is not worth reassembling.
    if let [single] = rest
        && record_constructed_body(single, registry, context.bindings, namespace, info)
    {
        return;
    }
    info.caller_frame_opaque_writes = true;
    info.caller_frame_opaque_reads = true;
}

/// The statements of an inlined `uplevel 1 {literal}` body: every literal
/// name it assigns is a caller-frame write.
///
/// A body statement whose target is *not* a plain literal name means the
/// body writes somewhere this summary cannot name, so widen — the same
/// direction the constructed-list path takes.
fn record_upframe_body(
    body: &Script,
    source_body_is_braced: bool,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    record_upframe_body_at_depth(
        body,
        source_body_is_braced,
        registry,
        bindings,
        namespace,
        info,
        0,
    );
}

/// Whether the original `uplevel` body was an unsubstituted brace word.
///
/// A body recovered from `$tmp` after lowering a `[list ...]` constructor is
/// readable, but its relative command heads still resolve in the selected
/// caller namespace. The original token snapshot is the source-of-truth for
/// that distinction; the lowered body alone cannot recover it after `set`
/// becomes `AssignConst`.
fn upframe_body_is_braced(tokens: Option<&CommandTokens>) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    let Some(argument_index) = tokens.words().len().checked_sub(2) else {
        return false;
    };
    tokens.arg_is_braced_literal(argument_index)
}

/// Record a body that is already known to execute in the direct caller frame.
/// Nested `uplevel 0` preserves that frame; nested relative levels beyond zero
/// reach past it and must not disappear from the summary.
fn record_upframe_body_at_depth(
    body: &Script,
    source_body_is_braced: bool,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    let projection = bindings.map_or_else(
        || tcl_registry::VariableWriteProjection {
            literal_names: crate::ir_helpers::defs_from_ir_script(body),
            read_before_write_names: Vec::new(),
            opaque_variable_frame: super::global_write_info::script_has_dynamic_write_target(body),
        },
        |bindings| {
            super::global_write_info::script_value_write_projection_with_metadata_context(
                body,
                registry,
                bindings.source,
                namespace,
                Some(bindings.metadata),
                bindings.input.lexer_config(),
            )
        },
    );
    let requires_runtime_resolution =
        crate::ir_helpers::requires_runtime_command_namespace(body, registry);
    let projected_names: BTreeSet<String> = if source_body_is_braced || !requires_runtime_resolution
    {
        projection.literal_names.into_iter().collect()
    } else {
        BTreeSet::new()
    };
    if let Some(bindings) = bindings {
        compose_selected_frame_alias_writes(
            body,
            &projected_names,
            projection.opaque_variable_frame,
            registry,
            bindings,
            info,
            depth + 1,
        );
    }
    info.uplevel_literal_writes.extend(projected_names);
    info.caller_frame_opaque_writes |= projection.opaque_variable_frame;

    // A relative `uplevel` evaluates in the *target frame's* namespace. The
    // procedure's defining namespace is therefore not a sound substitute for
    // resolving an unqualified head: a caller-local alias can turn it into an
    // arbitrary variable write. Keep literal names as useful hints, but widen
    // unless every directly evaluated command spelling is absolute.
    if requires_runtime_resolution {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
    }
    record_nested_upframe_effects(body, registry, bindings, namespace, info, depth + 1);
}

/// Compose variable-cell aliases established inside a body which already runs
/// in the direct caller frame.  The write projector deliberately reports the
/// local spelling (`upvar 0 x local; set local 1` reports `local`); registry
/// transitions supply the cell identity needed to add `x`.  We retain the raw
/// local as a conservative may-write because the summary is path-insensitive
/// and the write may precede the alias on another path.
#[allow(clippy::too_many_arguments)]
fn compose_selected_frame_alias_writes(
    script: &Script,
    projected_names: &BTreeSet<String>,
    opaque_write: bool,
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        return;
    }
    let any_write = opaque_write || !projected_names.is_empty();
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        compose_selected_statement_aliases(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            projected_names,
            any_write,
            registry,
            bindings,
            info,
        );

        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, registry);
        for words in embedded.commands {
            let Some(head) = words
                .first()
                .and_then(crate::ir_helpers::CommandWord::literal)
            else {
                continue;
            };
            let execution_namespace = ExecutionNamespace::RuntimeSelected;
            let Some(command_namespace) = execution_namespace.for_head_context(head) else {
                continue;
            };
            for facts in
                bindings.resolve_command_words(&words, registry, command_namespace.as_ref())
            {
                compose_selected_alias_facts(
                    &facts,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }

        // A nested frame selector installs aliases in a different frame. All
        // other structural bodies preserve the caller frame and participate
        // in this conservative, path-insensitive union.
        if matches!(stmt, Statement::UpFrame { .. }) {
            continue;
        }
        for nested in crate::ir_helpers::nested_bodies(stmt) {
            compose_selected_frame_alias_writes(
                nested,
                projected_names,
                opaque_write,
                registry,
                bindings,
                info,
                depth + 1,
            );
        }
    }
}

fn compose_selected_statement_aliases(
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    info: &mut UpvarInfo,
) {
    if bindings.input.has_logical_source_name_context() {
        if let Some((_, aliases)) = tokens.and_then(|tokens| {
            let metadata = tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
                registry, metadata, tokens,
            )
        }) {
            for alias in &aliases {
                compose_selected_alias(
                    alias,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }
        return;
    }
    if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
        let execution_namespace = ExecutionNamespace::RuntimeSelected;
        if let Some(command_namespace) =
            execution_namespace.for_invocation_context(command, stmt.tokens())
        {
            let normal = stmt.tokens().and_then(|tokens| {
                crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                    registry,
                    Some(bindings.metadata),
                    tokens,
                )
            });
            if let Some(normal) = normal {
                // A compiler-hook choice is independent of a normal alias
                // declaration. Consume only the active alias projection.
                for alias in normal.variable_alias_transitions() {
                    compose_selected_alias(
                        alias,
                        projected_names,
                        any_write,
                        registry,
                        bindings.invocation_dialect(),
                        info,
                    );
                }
            }
            if let Some(possible) = stmt.tokens().and_then(|tokens| {
                crate::registry_invocation::possible_variable_alias_transitions_with_metadata_context(
                    registry,
                    Some(bindings.metadata),
                    tokens,
                )
            }) {
                // This summary records may-writes across possible callers.
                // Unknown physical frame identity cannot erase a selected
                // native alias candidate's outward exposure footprint.
                for alias in possible.aliases() {
                    compose_selected_alias(
                        alias,
                        projected_names,
                        any_write,
                        registry,
                        bindings.invocation_dialect(),
                        info,
                    );
                }
            }
            for invocation in bindings.resolve_statement(stmt, registry, command_namespace.as_ref())
            {
                compose_selected_alias_facts(
                    &invocation.facts,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }
    }
}

enum SelectedAliasTarget {
    SameExact(String),
    SameOpaque,
    Past,
    Unplaced,
    Other,
}

fn selected_alias_target(
    target: &tcl_registry::VariableAliasTarget,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> SelectedAliasTarget {
    let tcl_registry::VariableAliasTarget::CallerSelectedFrame { frame, variable } = target else {
        return SelectedAliasTarget::Other;
    };
    let tcl_registry::CallerFrameSelection::Explicit(level) = frame else {
        return SelectedAliasTarget::Past;
    };
    let Some(level) = level.literal() else {
        return SelectedAliasTarget::Unplaced;
    };
    let Some(level) = dialect.map_or_else(
        || FrameLevel::parse_in(level, registry),
        |dialect| FrameLevel::parse_for_dialect(level, dialect),
    ) else {
        // Tcl rejects the invocation before installing this pair.
        return SelectedAliasTarget::Other;
    };
    match level {
        FrameLevel::Relative(0) => {
            variable
                .literal()
                .map_or(SelectedAliasTarget::SameOpaque, |name| {
                    SelectedAliasTarget::SameExact(
                        crate::naming::split_element_ref(name)
                            .map_or(name, |(root, _)| root)
                            .to_owned(),
                    )
                })
        }
        FrameLevel::Absolute(0) => SelectedAliasTarget::Other,
        FrameLevel::Relative(_) => SelectedAliasTarget::Past,
        FrameLevel::Absolute(_) | FrameLevel::Dynamic => SelectedAliasTarget::Unplaced,
    }
}

fn compose_selected_alias_facts(
    facts: &tcl_registry::InvocationFacts,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    for fact in transitions.facts() {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        compose_selected_alias(alias, projected_names, any_write, registry, dialect, info);
    }
}

fn compose_selected_alias(
    alias: &tcl_registry::state_transition::VariableCellAliasTransition,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    let local_written = alias.writes_value
        || alias.local.literal().map_or(any_write, |local| {
            projected_names
                .contains(crate::naming::split_element_ref(local).map_or(local, |(root, _)| root))
        });
    if !local_written {
        return;
    }
    match selected_alias_target(&alias.target, registry, dialect) {
        SelectedAliasTarget::SameExact(target) => {
            info.uplevel_literal_writes.insert(target);
        }
        SelectedAliasTarget::SameOpaque => info.caller_frame_opaque_writes = true,
        SelectedAliasTarget::Past => widen_beyond_caller(info),
        SelectedAliasTarget::Unplaced => {
            info.caller_frame_opaque_writes = true;
            widen_beyond_caller(info);
        }
        SelectedAliasTarget::Other => {}
    }
}

/// Compose nested literal `uplevel` frame selections from a body already
/// running in the direct caller frame. The ordinary value-write projection
/// intentionally skips [`Statement::UpFrame`] children because their writes
/// are not in the enclosing script's frame; this companion supplies the
/// missing frame transform.
fn record_nested_upframe_effects(
    script: &Script,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
            if let Some(bindings) = bindings
                && bindings.input.has_logical_source_name_context()
            {
                let selected = script.retained_source_tokens_for_statement(stmt)
                    .and_then(|tokens| {
                        let metadata = tokens.source_binding.as_ref()?
                            .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
                        let invocation = crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                            registry, metadata, tokens,
                        )?;
                        Some((tokens, invocation))
                    });
                if let Some((tokens, invocation)) = selected {
                    record_logical_frame_invocation(
                        &invocation,
                        LogicalFrameScan {
                            tokens,
                            current: FrameContext::DirectCaller,
                            params: &[],
                            registry,
                            bindings,
                            namespace,
                            depth: depth + 1,
                        },
                        info,
                    );
                } else {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
                continue;
            }
            let execution_namespace = ExecutionNamespace::RuntimeSelected;
            if let Some(bindings) = bindings
                && let Some(command_namespace) =
                    execution_namespace.for_invocation_context(command, stmt.tokens())
            {
                // A call which lowering saw before its alias was installed may
                // terminally be `eval` or `uplevel`.  Resolve the effective
                // body through the shared helper, including every baked alias
                // prefix, then compose its selection from the caller frame in
                // which this enclosing body already executes.
                for invocation in
                    bindings.resolve_statement(stmt, registry, command_namespace.as_ref())
                {
                    record_resolved_frame_body(
                        &invocation,
                        FrameContext::DirectCaller,
                        &[],
                        registry,
                        bindings,
                        namespace,
                        &execution_namespace,
                        info,
                        depth + 1,
                    );
                }
            }
        }
        match stmt {
            Statement::UpFrame {
                absolute: false,
                frame_shift: 0,
                body,
                tokens,
                ..
            } => record_upframe_body_at_depth(
                body,
                upframe_body_is_braced(tokens.as_ref()),
                registry,
                bindings,
                namespace,
                info,
                depth + 1,
            ),
            Statement::UpFrame {
                absolute: true,
                frame_shift: 0,
                ..
            } => {
                // `#0` is global; global_write_info owns it, including its
                // nested level-zero bodies.
            }
            Statement::UpFrame { .. } => widen_beyond_caller(info),
            _ => {
                for body in crate::ir_helpers::nested_bodies(stmt) {
                    record_nested_upframe_effects(
                        body,
                        registry,
                        bindings,
                        namespace,
                        info,
                        depth + 1,
                    );
                }
            }
        }
    }
}

/// Read a `[list CMD ARG…]` body word, recording the caller-frame names the
/// constructed command writes.  Returns `false` when the word is not a
/// readable constructed list, so the caller can widen.
///
/// `[list …]` is recognised through the registry's
/// [`ReturnElements::ListOfArgs`](tcl_registry::ReturnElements) — the same
/// fact the concat rules read — not by matching the name `list`.
fn record_constructed_body(
    word: &str,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) -> bool {
    let Some(bindings) = bindings else {
        return false;
    };
    let Some(constructed) = crate::command_binding::constructed_script_words_with_metadata_context(
        word,
        registry,
        bindings.source,
        namespace,
        Some(bindings.metadata),
    ) else {
        return false;
    };
    let Some((command, cargs)) = constructed.split_first() else {
        return false;
    };
    // The constructed script runs in the selected caller frame, so an
    // unqualified command word resolves there rather than in this procedure's
    // namespace. The constructor is statically known, but its result's head
    // is not namespace-invariant: a caller-local `set`/`worker` alias can
    // write or read arbitrary variables. Preserve exact handling only for an
    // explicitly absolute command spelling.
    if !command.starts_with("::") {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return true;
    }
    let arg_refs: Vec<&str> = cargs.iter().map(String::as_str).collect();
    if let Some(invocation) = tcl_registry::model::assembly::resolve_invocation_in_context(
        registry,
        Some(bindings.metadata.context()),
        command,
        &arg_refs,
    ) {
        let projection =
            CommandRegistry::variable_write_projection_for_selected_source(&invocation);
        info.uplevel_literal_writes.extend(projection.literal_names);
        info.caller_frame_opaque_writes |= projection.opaque_variable_frame;
        return true;
    }
    // Not a registry command — a candidate user proc. Its own `upvar 1`
    // reaches *this* proc's caller, one frame further out than a plain call
    // would, but only the module-wide pass can resolve it.
    info.uplevel_forwarded_calls
        .push((command.clone(), cargs.to_vec()));
    true
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
mod tests {
    use super::*;
    use crate::ir::Script;
    use tcl_registry::CommandRegistry;

    fn lower(src: &str) -> Script {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("test native core");
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        crate::lowering::lower_proc_body_isolated(
            src,
            "::",
            &registry,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            Some(profile),
        )
    }

    fn module_proc_info(src: &str, qname: &str) -> UpvarInfo {
        let context =
            crate::environment_ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, input.lexer_config())
                .with_resolved_analysis_input(input),
            src,
        );
        let bindings = ModuleCommandBindings::analyse(&module, &registry);
        let procedure = module
            .procedures
            .get(&crate::interprocedural::test_body_identity::fixture_id(
                &module.procedures,
                qname,
            ))
            .expect("procedure");
        let namespace = procedure
            .body
            .namespace_context
            .as_deref()
            .expect("actual original body namespace");
        collect_upvar_targets_with_bindings(
            &procedure.body,
            &procedure.params,
            &registry,
            &bindings,
            namespace,
            &module,
        )
    }

    #[test]
    fn method_frame_summary_keeps_the_issued_header_and_actual_registry() {
        let environment = crate::environment_ingress::resolve_environment("tcl9.0");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            native_entry: Some(std::sync::Arc::new(native)),
            ..Default::default()
        };
        let build = || {
            crate::compilation_unit::CompilationUnit::build_with_context_registry(
                "oo::class create C {method empty {} {}}",
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                std::sync::Arc::clone(&context),
            )
            .ir_module
        };
        let module = build();
        assert_eq!(module.methods.len(), 1);
        let (&id, method) = module.methods.iter().next().unwrap();
        assert!(module.procedure_ids.owns(id));
        assert!(
            module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .matches_original_method_summary(id, method)
        );
        assert!(!method_reaches_caller_frame(&module, id, registry));
        let mut changed = module.clone();
        changed
            .methods
            .get_mut(&id)
            .unwrap()
            .params
            .push("detached".into());
        assert!(method_reaches_caller_frame(&changed, id, registry));
        let mut changed_body = module.clone();
        changed_body.methods.get_mut(&id).unwrap().body = lower("upvar 1 caller local");
        assert!(method_reaches_caller_frame(&changed_body, id, registry));
        let foreign = build();
        let foreign_id = *foreign.methods.keys().next().unwrap();
        assert_eq!(foreign_id.ordinal(), id.ordinal());
        assert_ne!(foreign_id, id);
        assert!(method_reaches_caller_frame(&module, foreign_id, registry));
        let foreign_registry =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(method_reaches_caller_frame(
            &module,
            id,
            foreign_registry.commands(),
        ));
    }

    #[test]
    fn native_frame_aliases_distinguish_written_scalar_reads_from_literal_values() {
        let environment = crate::environment_ingress::resolve_environment("tcl9.0");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            native_entry: Some(std::sync::Arc::new(native)),
            ..Default::default()
        };
        for (source, parameter) in [
            ("proc helper {source} {upvar 1 $source local}", true),
            ("proc helper {source} {upvar 1 \"$source\" local}", true),
            ("proc helper {source} {upvar 1 {$source} local}", false),
            (
                "interp alias {} captured {} upvar 1 {$source}; proc helper {source} {captured local}",
                false,
            ),
        ] {
            let module = crate::compilation_unit::CompilationUnit::build_with_context_registry(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                std::sync::Arc::clone(&context),
            )
            .ir_module;
            let id = crate::interprocedural::test_body_identity::fixture_id(
                &module.procedures,
                "::helper",
            );
            assert!(module.procedure_ids.owns(id));
            let summaries = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
            let summary = &summaries[&id];
            assert_eq!(
                summary.param_targets.get("local").map(String::as_str),
                parameter.then_some("source"),
                "{source}"
            );
            if !parameter {
                assert!(summary.param_targets.is_empty(), "{source}");
                assert!(
                    summary.has_unresolvable_caller_target || summary.caller_frame_opaque_writes,
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn logical_frame_aliases_keep_real_formals_and_literal_dollar_names() {
        // naming.compiler.original-analysis-metadata-context

        let formal = module_proc_info("proc helper {source} {upvar 1 $source local}", "::helper");
        assert_eq!(
            formal.param_targets.get("local"),
            Some(&"source".to_owned())
        );
        assert!(!formal.has_unresolvable_caller_target);
        assert!(!formal.caller_frame_opaque_writes);

        let literal = module_proc_info(
            "proc helper {} {upvar 1 {$scalar(open} {$alias}; set {$alias} VALUE}",
            "::helper",
        );
        assert_eq!(
            literal.literal_targets.get("$alias"),
            Some(&"$scalar(open".to_owned())
        );
        assert!(!literal.has_unresolvable_caller_target);
    }

    #[test]
    fn empty_body_has_no_upvars() {
        let body = lower("");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.is_empty());
    }

    #[test]
    fn standalone_alias_pairs_keep_original_reference_grammar_and_literal_data() {
        for (source, parameter) in [
            ("upvar 1 ${odd name} local", "odd name"),
            ("upvar 1 \"$source\" local", "source"),
        ] {
            let body = lower(source);
            let info = collect_upvar_targets(&body, &[parameter.to_owned()]);
            assert_eq!(
                info.param_targets.get("local").map(String::as_str),
                Some(parameter)
            );
        }
        let body = lower("upvar 1 {$source} local");
        let info = collect_upvar_targets(&body, &["source".to_owned()]);
        assert!(info.param_targets.is_empty());
        assert!(info.has_unresolvable_caller_target);
        let mut changed = lower("upvar 1 $source local");
        let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) =
            &mut changed.statements[0]
        else {
            panic!("actual standalone upvar call");
        };
        args[1] = "$different".to_owned();
        let info = collect_upvar_targets(&changed, &["different".to_owned()]);
        assert!(info.param_targets.is_empty());
        assert!(info.caller_frame_opaque_writes);
    }

    #[test]
    fn literal_target_recorded() {
        let body = lower("upvar 1 caller_x x");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
        assert_eq!(info.param_targets.len(), 0);
        assert_eq!(info.args_tail_upvar.len(), 0);
    }

    #[test]
    fn multiple_literal_targets_recorded() {
        let body = lower("upvar 1 a la b lb c lc");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("la"), Some(&"a".to_string()));
        assert_eq!(info.literal_targets.get("lb"), Some(&"b".to_string()));
        assert_eq!(info.literal_targets.get("lc"), Some(&"c".to_string()));
    }

    #[test]
    fn param_target_recorded_when_param_exists() {
        let body = lower("upvar 1 $name x");
        let info = collect_upvar_targets(&body, &["name".to_string()]);
        assert_eq!(info.param_targets.get("x"), Some(&"name".to_string()));
        assert_eq!(info.literal_targets.len(), 0);
    }

    #[test]
    fn dollar_var_skipped_when_not_a_param() {
        let body = lower("upvar 1 $foo x");
        let info = collect_upvar_targets(&body, &["bar".to_string()]);
        // $foo is not a param (only `bar` is); not classifiable per-name —
        // and the summary must say so, not silently under-approximate: the
        // callee can write ANY caller variable through the alias.
        assert_eq!(info.param_targets.len(), 0);
        assert_eq!(info.literal_targets.len(), 0);
        assert!(info.has_unresolvable_caller_target);
    }

    #[test]
    fn fully_dynamic_source_widens_summary() {
        // `upvar 1 [pick] x` / an array-element source — the caller-side
        // name is runtime state; the flag must be set so the call site
        // widens to an opaque caller-frame clobber.
        for src in ["upvar 1 [pick] x", "upvar 1 a($i) x"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &[]);
            assert!(
                info.has_unresolvable_caller_target,
                "{src} must widen the summary"
            );
        }
        // Resolvable shapes must NOT set the flag.
        let body = lower("upvar 1 caller_x x\nupvar 1 $p y");
        let info = collect_upvar_targets(&body, &["p".to_string()]);
        assert!(!info.has_unresolvable_caller_target);
    }

    #[test]
    fn args_tail_upvar_recorded() {
        let body = lower("upvar 1 $args y");
        let info = collect_upvar_targets(&body, &["args".to_string()]);
        // `args` triggers the trailing-vararg path regardless of
        // its position in the param list.
        assert_eq!(info.args_tail_upvar, vec!["y".to_string()]);
        assert_eq!(info.param_targets.len(), 0);
    }

    #[test]
    fn level_omitted_defaults_to_one() {
        // No leading level word: `upvar src dst`.
        let body = lower("upvar caller_x x");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
    }

    #[test]
    fn level_hash_zero_is_not_a_caller_frame_binding() {
        // TN — `upvar #0 g local_g` binds the **global** `g`, not the
        // caller's, however deep the call stack is (tclsh 9.0.4 / 8.6.14:
        // `proc deep {} {upvar #0 gvar g; set g DEEP}` called two frames
        // down sets `::gvar`).  Reporting it as a caller-frame def would
        // mark an unrelated *local* named `g` defined in every caller.
        // `global_write_info` owns the real, global effect.
        let body = lower("upvar #0 g local_g");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.len(), 0, "got {info:?}");
        assert!(!info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn level_zero_is_the_callees_own_frame_and_contributes_nothing() {
        // TN — `upvar 0 x y` aliases the *callee's own* local `x` (tclsh
        // 9.0.4 / 8.6.14: `upvar 0 target alias` inside a proc whose caller
        // has `target` reads `can't read "alias"`), so no caller effect.
        let body = lower("upvar 0 x y");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.is_empty(), "got {info:?}");
    }

    #[test]
    fn level_beyond_the_caller_widens_rather_than_binding() {
        // `upvar 2 far f` writes the caller's *caller*. Naming `far` as a
        // caller-frame def would be wrong; dropping it would hide the write
        // from the frame that does receive it. Widen — the abstain-toward-
        // silence direction (tclsh 9.0.4 / 8.6.14: `proc gp {} {upvar 2 far
        // f; set f FAR}` through one intermediate frame sets the
        // grandparent's `far`).
        for src in ["upvar 2 far f", "upvar #3 far f", "upvar $lvl a b"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &["lvl".to_string()]);
            assert!(
                info.caller_frame_opaque_writes,
                "{src} must widen; got {info:?}"
            );
            assert_eq!(info.literal_targets.len(), 0, "{src}: got {info:?}");
        }
    }

    #[test]
    fn level_word_presence_follows_argument_count_parity() {
        // `upvar $lvl a b` — three words, so `$lvl` is the level and
        // `(a, b)` is the pair. A text-sniffing level test saw no level here
        // and paired `($lvl, a)`, losing the binding (tclsh 9.0.4 / 8.6.14:
        // `proc t4 {lvl} {upvar $lvl a b; set b 4}` really does set the
        // caller's `a`).  The level is dynamic, so the summary widens.
        let body = lower("upvar $lvl a b");
        let info = collect_upvar_targets(&body, &["lvl".to_string()]);
        assert!(info.caller_frame_opaque_writes, "got {info:?}");

        // `upvar 1 b` — two words, so there is NO level word and `1` is the
        // caller-side *variable name* (tclsh 9.0.4 / 8.6.14: with `set 1
        // ONE` in the caller, the callee reads `ONE` through the alias).
        let body = lower("upvar 1 b");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("b"), Some(&"1".to_string()));

        // `upvar 1 a b c` — four words, so no level: two pairs, `(1, a)` and
        // `(b, c)` (tclsh 9.0.4 / 8.6.14 accept it without error).
        let body = lower("upvar 1 a b c");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("a"), Some(&"1".to_string()));
        assert_eq!(info.literal_targets.get("c"), Some(&"b".to_string()));
    }

    #[test]
    fn original_protected_substitution_keeps_caller_alias_pairs() {
        for command in [
            "catch {upvar 1 target local}",
            "catch {catch {upvar 1 target local}}",
        ] {
            let info = module_proc_info(
                &format!("proc p {{}} {{set result [{command}]; set local VALUE}}"),
                "::p",
            );
            assert_eq!(
                info.literal_targets.get("local"),
                Some(&"target".to_owned()),
                "{command}"
            );
        }
        let dynamic = module_proc_info("proc p {script} {set result [catch $script]}", "::p");
        assert!(dynamic.caller_frame_opaque_writes);
        assert!(dynamic.caller_frame_opaque_reads);
    }

    #[test]
    fn original_caller_argument_values_preserve_literal_scalar_names() {
        let mut info = module_proc_info("proc helper {source} {upvar 1 $source local}", "::helper");
        let params = vec!["source".to_owned()];
        for name in ["", "$name", "a b", "{literal}"] {
            let words = [InvocationWord::Literal(name)];
            let explicit =
                info.caller_side_effects(InvocationArguments::Structured(&words), &params);
            assert_eq!(explicit.defs, vec![name.to_owned()]);
            assert!(!explicit.opaque);
            info.param_defaults = name_param_defaults(
                &info,
                &format!("{{source {{{name}}}}}"),
                tcl_syntax::word_rules::WordValueRules::from_config(
                    &tcl_lexer::LexerConfig::default(),
                ),
            );
            let omitted = info.caller_side_effects(InvocationArguments::Structured(&[]), &params);
            assert_eq!(omitted.defs, vec![name.to_owned()]);
            assert!(!omitted.opaque);
        }
        let dynamic = [InvocationWord::Dynamic];
        let effects = info.caller_side_effects(InvocationArguments::Structured(&dynamic), &params);
        assert!(effects.opaque);
    }

    #[test]
    fn uplevel_literal_body_records_caller_frame_writes() {
        // `uplevel 1 {set litVar hello}` — the lowering inlines the literal
        // body as `Statement::UpFrame`; its writes land in the caller. The
        // caller can shadow unqualified `set`, so the summary also widens.
        let info = module_proc_info(
            "proc ::setter {} {uplevel 1 {set litVar hello}}; ::setter",
            "::setter",
        );
        assert!(
            info.uplevel_literal_writes.contains("litVar"),
            "got {info:?}"
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn uplevel_literal_body_resolves_late_alias_prefix_write() {
        // `setter` is lowered before `put` is installed, and the alias
        // prepends `x`: Tcl 9.0.4 evaluates the literal body as `set x 99`
        // in the caller's frame.  The module-wide binding projection must
        // recover that caller-frame write even though lower-time defs cannot.
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { put 99 } }\n\
             interp alias {} put {} set x",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn nested_relative_zero_preserves_the_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { uplevel 0 { set x 1 } } }; ::setter",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn late_alias_to_eval_preserves_an_enclosing_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} eval",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    }

    #[test]
    fn late_alias_to_relative_zero_uplevel_preserves_an_enclosing_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} uplevel 0",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    }

    #[test]
    fn late_alias_to_relative_one_uplevel_reaches_past_an_enclosing_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} uplevel 1",
            "::setter",
        );
        assert_eq!(info.frame_reach, FrameReach::PastTheCaller, "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn current_frame_alias_pairs_compose_inside_the_selected_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::upvar 0 x local; ::set local 1 } }",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    }

    #[test]
    fn alias_prefixed_alias_pairs_keep_their_effective_operands() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::bind local; ::set local 1 } }\n\
             interp alias {} ::bind {} upvar 0 x",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    }

    #[test]
    fn dynamic_current_frame_alias_target_widens_the_selected_caller_frame() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::upvar 0 $name local; ::set local 1 } }",
            "::setter",
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn caller_alias_inside_the_selected_caller_frame_reaches_past_it() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { ::upvar 1 x local; ::set local 1 } }",
            "::setter",
        );
        assert_eq!(info.frame_reach, FrameReach::PastTheCaller, "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn current_frame_body_retains_nested_caller_frame_effects() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 0 { uplevel 1 { set x 1 } } }",
            "::setter",
        );
        assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn nested_relative_one_reaches_past_the_caller() {
        let info = module_proc_info(
            "proc ::setter {} { uplevel 1 { uplevel 1 { set x 1 } } }; \
             proc ::caller {} {::setter}; ::caller",
            "::setter",
        );
        assert_eq!(info.frame_reach, FrameReach::PastTheCaller, "got {info:?}");
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn relative_upframe_does_not_borrow_the_callee_namespace_alias() {
        let info = module_proc_info(
            "interp alias {} ::callee::put {} puts\n\
             interp alias {} ::caller::put {} set x\n\
             proc ::callee::setter {} { uplevel 1 { put 99 } }",
            "::callee::setter",
        );
        // The literal `put` resolves in the frame selected by `uplevel 1`,
        // not in ::callee. A caller-local alias may write arbitrary names.
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
    }

    #[test]
    fn uplevel_constructed_list_resolves_the_written_name() {
        // A substituted constructor operand is not a literal result element.
        // Even when it names a formal parameter, retaining its source spelling
        // would mistake `$varName` for the value supplied at runtime.
        let info = module_proc_info(
            "proc ::dynamic {varName} { uplevel 1 [list set $varName 1] }",
            "::dynamic",
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
        assert_eq!(info.uplevel_param_writes.len(), 0, "got {info:?}");

        // Even literal result words retain an unqualified command head, which
        // resolves in the caller's namespace rather than ::literal's.
        let info = module_proc_info(
            "proc ::literal {} { uplevel 1 [list set fixed 1] }",
            "::literal",
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
        assert_eq!(info.uplevel_literal_writes.len(), 0, "got {info:?}");

        // A target that is neither literal nor a parameter (a `foreach`
        // variable, say) is unknowable — widen.
        let info = module_proc_info(
            "proc ::loop {vs} { foreach v $vs { uplevel 1 [list set $v 1] } }",
            "::loop",
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn absolute_constructed_set_is_exact_while_relative_set_is_opaque() {
        let exact = module_proc_info(
            "proc ::exact {} { uplevel 1 [list ::set fixed 1] }",
            "::exact",
        );
        assert_eq!(
            exact.uplevel_literal_writes,
            BTreeSet::from(["fixed".to_owned()]),
            "got {exact:?}"
        );
        assert!(!exact.caller_frame_opaque_writes, "got {exact:?}");
        assert!(!exact.caller_frame_opaque_reads, "got {exact:?}");

        let relative = module_proc_info(
            "proc ::relative {} { uplevel 1 [list set fixed 1] }",
            "::relative",
        );
        assert_eq!(relative.uplevel_literal_writes.len(), 0, "got {relative:?}");
        assert!(relative.caller_frame_opaque_writes, "got {relative:?}");
        assert!(relative.caller_frame_opaque_reads, "got {relative:?}");
    }

    #[test]
    fn relative_constructed_bodies_do_not_borrow_callee_namespace() {
        let setter = module_proc_info(
            "interp alias {} ::caller::set {} set shadow\n\
             proc ::callee::setter {} { uplevel 1 [list set fixed 1] }",
            "::callee::setter",
        );
        assert!(setter.caller_frame_opaque_writes, "got {setter:?}");
        assert!(setter.caller_frame_opaque_reads, "got {setter:?}");
        assert_eq!(setter.uplevel_literal_writes.len(), 0, "got {setter:?}");

        let forwarded = module_proc_info(
            "proc ::worker {name} { upvar 1 $name local; set local 1 }\n\
             interp alias {} ::caller::worker {} set shadow\n\
             proc ::callee::forward {} { uplevel 1 [list worker target] }",
            "::callee::forward",
        );
        assert!(forwarded.caller_frame_opaque_writes, "got {forwarded:?}");
        assert!(forwarded.caller_frame_opaque_reads, "got {forwarded:?}");
        assert_eq!(
            forwarded.uplevel_forwarded_calls.len(),
            0,
            "got {forwarded:?}"
        );
    }

    #[test]
    fn rebound_list_constructor_widens_caller_frame_effects() {
        let info = module_proc_info(
            "proc list args { return {set hidden 1} }\n\
             proc ::p {} { uplevel 1 [list set claimed 1] }",
            "::p",
        );
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
        assert_eq!(info.uplevel_literal_writes.len(), 0, "got {info:?}");
    }

    #[test]
    fn uplevel_dynamic_body_is_an_opaque_caller_frame_barrier() {
        let body = lower("uplevel 1 $script");
        let info = collect_upvar_targets(&body, &["script".to_string()]);
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
        let barrier = info.caller_frame_barrier();
        assert!(barrier.writes && barrier.reads && barrier.destroys);
    }

    #[test]
    fn uplevel_at_the_current_or_global_frame_is_not_a_caller_effect() {
        // TN — `uplevel 0 $s` re-enters the callee's own frame and
        // `uplevel #0 $s` the global one; neither touches the caller.
        // `crate::dynamic_names` raises the *callee's* own barrier for both.
        for src in ["uplevel 0 $s", "uplevel #0 $s"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &["s".to_string()]);
            assert!(info.is_empty(), "{src}: got {info:?}");
        }
    }

    #[test]
    fn uplevel_of_a_side_effect_free_command_has_no_caller_effect() {
        // TN — an absolute `::puts` head is namespace-invariant and writes no
        // variable anywhere, so
        // the summary must stay clear rather than widening on the mere
        // presence of an `uplevel`.
        let info = module_proc_info("proc ::shout {} { uplevel 1 [list ::puts hi] }", "::shout");
        assert!(info.is_empty(), "got {info:?}");
    }

    #[test]
    fn dynamic_destination_with_literal_source_still_names_the_caller_var() {
        // `upvar 1 x $dst` aliases exactly the caller's `x`;
        // only the local key is dynamic.  The summary must not read as
        // empty (tclsh 9.0.4 / 8.6.16: `proc p {dst} {upvar 1 x $dst; set
        // $dst 2}; proc c {} {set x 1; p x; puts $x}` prints 2, so a call
        // site that kept `x = 1` foldable would miscompile).
        let body = lower("upvar 1 caller_x $local");
        let info = collect_upvar_targets(&body, &[]);
        assert!(!info.is_empty(), "got {info:?}");
        assert!(
            info.uplevel_literal_writes.contains("caller_x"),
            "got {info:?}"
        );
        assert!(!info.has_unresolvable_caller_target, "got {info:?}");
        // Structural caller-frame reach survives an unnameable local.
        assert!(info.unnameable_local_aliases.contains("caller_x"));
        // And the call-site resolver surfaces the caller-side name.
        assert_eq!(info.caller_side_defs(&[], &[]), vec!["caller_x"]);
    }

    #[test]
    fn dynamic_destination_with_param_source_resolves_at_the_call_site() {
        // `upvar 1 $name $dst`: the caller-side name is the
        // value passed for `name`, exactly like `param_targets`.
        let body = lower("upvar 1 $name $local");
        let info = collect_upvar_targets(&body, &["name".to_string()]);
        assert!(info.uplevel_param_writes.contains("name"), "got {info:?}");
        assert!(!info.has_unresolvable_caller_target, "got {info:?}");
        let defs = info.caller_side_defs(&["target".to_string()], &["name".to_string()]);
        assert_eq!(defs, vec!["target"]);
    }

    #[test]
    fn dynamic_destination_with_dynamic_source_widens() {
        // Both sides dynamic: the callee can clobber any
        // caller variable, so the summary must widen, not stay empty.
        for src in [
            "upvar 1 [pick] $local",
            "upvar 1 $notparam $local",
            "upvar 1 $args $local",
        ] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &["args".to_string()]);
            assert!(
                info.has_unresolvable_caller_target,
                "{src} must widen; got {info:?}"
            );
        }
    }

    #[test]
    fn upvar_inside_if_body_collected() {
        let body = lower("if {1} { upvar 1 caller_x x }");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
    }

    #[test]
    fn upvar_inside_while_body_collected() {
        let body = lower("while {1} { upvar 1 caller_y y; break }");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("y"), Some(&"caller_y".to_string()),);
    }

    #[test]
    fn resolve_literal_or_param_literal() {
        let mut info = UpvarInfo::default();
        info.literal_targets
            .insert("x".to_string(), "caller_x".to_string());
        let args = BTreeMap::new();
        assert_eq!(
            info.resolve_literal_or_param("x", &args),
            Some("caller_x".to_string()),
        );
    }

    #[test]
    fn resolve_literal_or_param_via_param() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".to_string(), "name".to_string());
        let mut args = BTreeMap::new();
        args.insert("name".to_string(), Some("caller_real".to_string()));
        assert_eq!(
            info.resolve_literal_or_param("x", &args),
            Some("caller_real".to_string()),
        );
    }

    #[test]
    fn resolve_literal_or_param_dynamic_param_returns_none() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".to_string(), "name".to_string());
        let mut args = BTreeMap::new();
        args.insert("name".to_string(), None); // dynamic call-site arg
        assert!(info.resolve_literal_or_param("x", &args).is_none());
    }

    #[test]
    fn resolve_literal_or_param_unknown_local_returns_none() {
        let info = UpvarInfo::default();
        let args = BTreeMap::new();
        assert!(info.resolve_literal_or_param("missing", &args).is_none());
    }

    #[test]
    fn is_empty_helper() {
        let mut info = UpvarInfo::default();
        assert!(info.is_empty());
        info.literal_targets.insert("x".into(), "y".into());
        assert!(!info.is_empty());
    }

    #[test]
    fn caller_side_defs_literal_only() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("x".into(), "caller_x".into());
        info.literal_targets.insert("y".into(), "caller_y".into());
        let defs = info.caller_side_defs(&[], &[]);
        let mut sorted = defs.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["caller_x".to_string(), "caller_y".to_string()]);
    }

    #[test]
    fn caller_side_defs_param_resolution() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "name".into());
        // proc foo {name body} { upvar 1 $name x ... }
        // Call: foo my_var { ... }
        let params = vec!["name".to_string(), "body".to_string()];
        let call_args = vec!["my_var".to_string(), "{set x 1}".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert_eq!(defs, vec!["my_var".to_string()]);
    }

    #[test]
    fn caller_side_defs_param_dollar_normalisation() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "name".into());
        let params = vec!["name".to_string()];
        // call site passes `$user_var` — should normalise to `user_var`.
        let call_args = vec!["$user_var".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert_eq!(defs, vec!["user_var".to_string()]);
    }

    #[test]
    fn caller_side_defs_literal_plus_param() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("a".into(), "caller_a".into());
        info.param_targets.insert("b".into(), "n".into());
        let params = vec!["n".to_string()];
        let call_args = vec!["caller_b".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        let mut sorted = defs.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["caller_a".to_string(), "caller_b".to_string()]);
    }

    #[test]
    fn caller_side_defs_dedup() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("a".into(), "same".into());
        info.literal_targets.insert("b".into(), "same".into());
        let defs = info.caller_side_defs(&[], &[]);
        assert_eq!(defs, vec!["same".to_string()]);
    }

    #[test]
    fn caller_side_defs_args_tail_upvar_positional() {
        // `args_tail_upvar` records the locals (in declaration order)
        // for every `upvar 1 $args local` form in the proc body.  The
        // caller-side resolver pairs each entry positionally with a
        // call-site tail argument starting at `params.len() - 1`.
        let mut info = UpvarInfo::default();
        info.args_tail_upvar.push("x".into()); // first tail arg → x
        info.args_tail_upvar.push("y".into()); // second tail arg → y
        let params = vec!["a".to_string(), "args".to_string()];
        // Call: foo first_pos tail_one tail_two tail_three
        // tail_start = params.len() - 1 = 1.
        let call_args = vec![
            "first_pos".to_string(),
            "tail_one".to_string(),
            "tail_two".to_string(),
            "tail_three".to_string(),
        ];
        let defs = info.caller_side_defs(&call_args, &params);
        // Two declarations pick the first two tail args.
        assert_eq!(defs, vec!["tail_one".to_string(), "tail_two".to_string()]);
    }

    #[test]
    fn caller_side_defs_missing_param_skipped() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".into(), "missing_param".into());
        let params = vec!["other".to_string()];
        let call_args = vec!["v".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        // `missing_param` isn't in params — skip silently.
        assert_eq!(defs.len(), 0);
    }

    #[test]
    fn caller_side_defs_dynamic_arg_normalises_to_empty_skipped() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "n".into());
        let params = vec!["n".to_string()];
        // `$` alone normalises to empty — should be skipped.
        let call_args = vec!["$".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert_eq!(defs.len(), 0);
    }
    #[test]
    fn caller_frame_metadata_keeps_actual_availability_and_missing_owner_hazards() {
        // naming.compiler.original-analysis-metadata-context

        let current =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = current.commands();
        // A source MAY footprint does not require or create a Native entered frame.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&current),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut module = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "proc helper {} {uplevel 1 {::lassign {VALUE} target}}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
        .ir_module;
        let body_id =
            crate::interprocedural::test_body_identity::fixture_id(&module.procedures, "::helper");
        let selected = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
        assert!(selected[&body_id].uplevel_literal_writes.contains("target"));
        let original = module.source_metadata_input.as_ref().unwrap().clone();
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry));
        for context in [
            std::sync::Arc::new(older),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        ] {
            module.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                original.analyser_profile(),
                original.unit_profile(),
                context,
                original.lexer_config(),
            ));
            let unavailable =
                crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
            assert!(unavailable[&body_id].caller_frame_opaque_writes);
        }
        module.source_metadata_input = None;
        let missing = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
        assert!(missing[&body_id].caller_frame_opaque_writes);
        assert!(missing[&body_id].caller_frame_opaque_reads);
        assert!(!missing[&body_id].is_empty());
    }
}
