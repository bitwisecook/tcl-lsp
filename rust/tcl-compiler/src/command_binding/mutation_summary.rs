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

//! The flow-insensitive summary of command rebindings the optimiser's trust gate consumes.

use super::{
    ModuleCommandBindings, ModuleCommandMutations, ProcBindingTrustProjection, default_binding,
};
use crate::ir::Statement;
use crate::naming::normalise_qualified_name as nqn;
use crate::var_escape::helpers::invocation_facts;
use std::hash::Hash;
use tcl_names::command::BindingKind;
use tcl_registry::{CommandRegistry, StateTransition, StateTransitionDomain};

/// Whether every command-*binding* subject this scan saw could be named.
///
/// A two-state fact rather than a fourth `bool` beside [`ModuleCommandMutations::dynamic`]:
/// the states carry the meaning at the use site, and both this summary and
/// its memo key stay inside `clippy::pedantic`'s bool budget without an
/// `#[allow]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RebindingSubjects {
    /// Every `rename` / `interp alias` / delete subject was one this scan
    /// could spell, so the names it did not record still denote what they did.
    #[default]
    AllNameable,
    /// At least one was not — `rename $a {}`, not `rename foo bar`. The
    /// subject may have been *this* name, so no name is claimable (#2168).
    SomeUnnameable,
}

impl RebindingSubjects {
    /// The axis a `bool`-valued observation stands for.
    #[must_use]
    pub(super) fn from_unnameable(unnameable: bool) -> Self {
        if unnameable {
            Self::SomeUnnameable
        } else {
            Self::AllNameable
        }
    }
}

impl ProcBindingTrustProjection {
    /// Whether `proc_name` still denotes its declared procedure at an
    /// arbitrary later call site.
    #[must_use]
    pub(crate) fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        !self.dynamic && !self.rebound.contains(&nqn(proc_name))
    }

    /// Whether a dynamic command-binding transition made every procedure
    /// binding untrustworthy.
    #[must_use]
    pub(crate) const fn has_dynamic_binding_transition(&self) -> bool {
        self.dynamic
    }
}

/// Hashes the complete projection [`ModuleCommandMutations::snapshot`] keeps
/// — every field, the sets sorted — so two summaries equal as sets hash
/// alike and the value can join an interned memo key beside the snapshot
/// it was rebuilt from.
impl std::hash::Hash for ModuleCommandMutations {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.snapshot().hash(state);
    }
}

impl ModuleCommandMutations {
    /// Whether the module may set the target's precision variable, so no
    /// computed double has a spelling known at compile time
    /// ([`tcl_dialect::NumberSpelling::with_precision_written`]).
    #[must_use]
    pub const fn may_set_precision(&self) -> bool {
        matches!(self.precision_written, PrecisionWrites::MayBeWritten)
    }

    /// Whether any command-table mutation has a dynamic source or target.
    ///
    /// Consumers retaining typed proof declines need to distinguish this
    /// lattice top from a statically known rebinding of one command name.
    #[must_use]
    pub const fn has_dynamic_mutation(&self) -> bool {
        self.dynamic
    }

    /// The constructed command-key label contributes only MAY trust invalidation;
    /// its written empty tail remains part of the namespace geometry.
    fn import_shadowed(&self, name: &str) -> bool {
        if self.opaque_namespaces.is_empty() {
            return false;
        }
        let parts = tcl_syntax::naming::written_command_parts(name.as_bytes());
        let path = tcl_core_types::ByteNamespacePath::from_segments(parts.qualifiers);
        let Some(namespace) = tcl_syntax::naming::checked_namespace_path_key(&path) else {
            return true;
        };
        self.opaque_namespaces.contains(&namespace)
    }

    /// Whether any body changes namespace command *resolution* — see
    /// [`Self::resolution_changed`]. A consumer folding a command it resolved
    /// itself must decline when this is true.
    #[must_use]
    pub const fn changes_command_resolution(&self) -> bool {
        self.resolution_changed
    }

    /// Whether any command whose canonical name lies under `prefix` is
    /// rebound — a `rename` source or target, or an `interp alias` name.
    ///
    /// The math-function gate asks this about `::tcl::mathfunc::`: `expr`
    /// resolves `abs(…)` through the command table, so
    /// `rename ::tcl::mathfunc::abs {}` must stop the compiler evaluating it
    /// natively (C Tcl then raises `invalid command name`).
    #[must_use]
    pub fn rebinds_under(&self, prefix: &str) -> bool {
        let bare = tcl_syntax::naming::unroot_rooted_key(prefix).unwrap_or(prefix);
        self.rebound.iter().any(|name| {
            let canonical = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
            canonical.starts_with(bare)
        })
    }

    /// True when `command_name` is not clobbered by any body mutation —
    /// i.e. the optimiser may still fold it with its original builtin
    /// semantics.
    #[must_use]
    pub fn trusts(&self, command_name: &str) -> bool {
        !self.dynamic && self.observed_binding_is_the_builtin(command_name)
    }

    /// Descriptor-label eligibility under the module's MAY mutation and
    /// observer inventories. This supplies no actual command or trace owner.
    #[must_use]
    pub(crate) fn trusts_unobserved_descriptor(
        &self,
        descriptor: tcl_registry::CanonicalCommandDescriptorKey<'_>,
        dynamic_trace: bool,
        traced: &std::collections::BTreeSet<String>,
    ) -> bool {
        let name = descriptor.label();
        let key = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
        self.trusts(key) && !dynamic_trace && !traced.contains(key) && !traced.contains(name)
    }

    /// The **named-subject** half of [`Self::trusts`]: whether the module's
    /// own observed bindings leave `command_name` denoting its registry
    /// builtin — no shadowing `proc`, no `rename` or alias onto it, no
    /// import into a namespace this scan could not enumerate.
    ///
    /// Deliberately omits [`Self::dynamic`], the unbounded top, which names
    /// no subject at all and is raised by something as ordinary as one
    /// command head the registry cannot resolve. Every fold that *becomes a
    /// source rewrite* still asks the full [`Self::trusts`]; this narrower
    /// question is what the shared per-unit value lattice asks
    /// ([`crate::sccp::FoldTrust::ObservedBindings`]), so a call the module
    /// itself resolves to a user `proc` can never be evaluated with builtin
    /// semantics there (#2164) — without withdrawing constant folding from
    /// every file that mentions a command the registry does not know.
    #[must_use]
    pub fn observed_binding_is_the_builtin(&self, command_name: &str) -> bool {
        // A rebinding whose subject this scan could not name may have moved
        // *this* name, so nothing is claimable (#2168). Deliberately not the
        // whole of `dynamic`, which also rises for effects that rebind nothing
        // and whose folds #2164 measured as worth keeping.
        self.rebinding_subjects == RebindingSubjects::AllNameable
            && !self.import_shadowed(command_name)
            && !self.names.contains(&nqn(command_name))
    }

    /// Whether this summary reports no command-table mutation that could
    /// change what a *builtin* name denotes — [`Self::trusts`] then answers
    /// `true` for every name, exactly as [`Self::default`] does.
    ///
    /// Consumers that omit the complete mutation snapshot may use this query
    /// to establish agreement with an untouched-module baseline. The function
    /// lattice memo retains the full snapshot and therefore keys separately
    /// for namespace-local shadows and unknown mutation obligations.
    #[must_use]
    pub fn agrees_with_untouched_bindings(&self) -> bool {
        !self.dynamic && self.names.is_empty() && self.opaque_namespaces.is_empty()
    }

    /// Whether any body runs in a namespace chosen at run time while naming a
    /// command relatively.
    ///
    /// A receiver-local command can shadow such a head, so nothing derived
    /// inside one of those frames is trustworthy — not even a fact that reads
    /// no variable, such as the `TclOO` frame constant. Consumers that fold
    /// *inside* a method body ask this in addition to [`Self::trusts`]; the
    /// top level and procedures do not, because a receiver namespace cannot
    /// change what a name resolves to in a namespace this scan can spell.
    #[must_use]
    pub const fn has_runtime_selected_frames(&self) -> bool {
        self.runtime_selected_frames
    }

    /// The everything-is-untrusted lattice top: `trusts` /
    /// `trusts_proc_binding` answer `false` for every name. The sound
    /// stand-in when a consumer has **no whole-module view at all** (the
    /// analyser's isolated per-item body pass) — folding with builtin
    /// semantics is then never permitted.
    #[must_use]
    pub fn distrust_all() -> Self {
        Self {
            names: std::collections::HashSet::new(),
            rebound: std::collections::HashSet::new(),
            dynamic: true,
            rebinding_subjects: RebindingSubjects::SomeUnnameable,
            runtime_selected_frames: true,
            resolution_changed: true,
            opaque_namespaces: std::collections::HashSet::new(),
            precision_written: PrecisionWrites::MayBeWritten,
        }
    }

    /// A canonical, hashable snapshot of this summary — see
    /// [`CommandTrustSnapshot`].
    #[must_use]
    pub fn snapshot(&self) -> CommandTrustSnapshot {
        let mut untrusted_builtins: Vec<String> = self.names.iter().cloned().collect();
        untrusted_builtins.sort_unstable();
        let mut rebound: Vec<String> = self.rebound.iter().cloned().collect();
        rebound.sort_unstable();
        CommandTrustSnapshot {
            untrusted_builtins,
            rebound,
            dynamic: self.dynamic,
            runtime_selected_frames: self.runtime_selected_frames,
            rebinding_subjects: self.rebinding_subjects,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: {
                let mut v: Vec<String> = self.opaque_namespaces.iter().cloned().collect();
                v.sort_unstable();
                v
            },
            precision_written: self.precision_written,
        }
    }

    /// True when `proc_name` can still be trusted to denote the module
    /// procedure it was declared as at an arbitrary later call site — i.e.
    /// its bare name was never the subject of a later `rename` (as the old
    /// name being moved away *or* the new name a different command moved
    /// onto) or `interp alias` (as the alias name) anywhere in the module.
    ///
    /// Flow-insensitive and whole-module, like [`Self::trusts`]: a
    /// rebinding buried in a proc body only takes effect when that proc
    /// runs, and the cross-proc call order isn't statically known, so any
    /// observed rebinding of the name is treated as live everywhere. This
    /// is what makes it sound to gate the optimiser's proc-call constant
    /// fold (O103) on this query — folding a call to the *original* proc's
    /// constant return would miscompile a script that later does
    /// `rename otherProc thisName` or `interp alias {} thisName {} other`.
    #[must_use]
    pub fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        // A rebinding whose subject the scan could not name may be this one.
        !self.dynamic
            && self.rebinding_subjects == RebindingSubjects::AllNameable
            && self.observed_proc_binding(proc_name)
    }

    /// The canonical names of the core builtins some body may rebind, when
    /// this scan could name every rebinding subject; `None` when one may have
    /// moved any command.
    #[must_use]
    pub fn nameable_rebound_builtins(&self) -> Option<&std::collections::HashSet<String>> {
        (self.rebinding_subjects == RebindingSubjects::AllNameable).then_some(&self.names)
    }

    /// The **named-subject** half of [`Self::trusts_proc_binding`], as
    /// [`Self::observed_binding_is_the_builtin`] is of [`Self::trusts`]: no
    /// `rename` or alias in the module names `proc_name`, and it sits in no
    /// namespace whose resolution this scan could not enumerate. Omits the
    /// unbounded `dynamic` top, which one unresolved command head raises.
    #[must_use]
    pub fn observed_proc_binding(&self, proc_name: &str) -> bool {
        !self.import_shadowed(proc_name) && !self.rebound.contains(&nqn(proc_name))
    }
}

/// A canonical (sorted), hashable form of [`ModuleCommandMutations`], so
/// the whole-module trust fact can ride inside a memoisation key — the
/// analyser's per-item body pass carries it on each deferred body whose
/// text could fold a command substitution, keeping the
/// isolated fragment memo sound when a `rename` elsewhere in the file
/// appears or disappears.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTrustSnapshot {
    untrusted_builtins: Vec<String>,
    pub(super) rebound: Vec<String>,
    pub(super) dynamic: bool,
    /// Part of the key: a memo taken while every rebinding subject was
    /// nameable must not be reused once a `rename $x` appears, or the fold it
    /// carries would be replayed under a binding it never saw (#2168).
    pub(super) rebinding_subjects: RebindingSubjects,
    /// Part of the key: a memo taken with every frame statically nameable must
    /// not be reused once a receiver-selected frame appears.
    pub(super) runtime_selected_frames: bool,
    /// Part of the key: a memo taken with namespace command resolution
    /// unperturbed must not be reused once a `namespace import` appears.
    pub(super) resolution_changed: bool,
    pub(super) opaque_namespaces: Vec<String>,
    /// Part of the key: a double folded while the default spelling held
    /// must not be replayed once a `set tcl_precision` appears.
    pub(super) precision_written: PrecisionWrites,
}

impl CommandTrustSnapshot {
    /// The registry-neutral evidence this snapshot carries into the value
    /// transfer's analysis context: the rebound builtins, the redefined
    /// procedures, the dynamic flag, and the opaque namespaces.
    #[must_use]
    pub fn binding_evidence(&self) -> tcl_registry::value_transfer::BindingEvidence {
        tcl_registry::value_transfer::BindingEvidence {
            untrusted_builtins: self.untrusted_builtins.clone(),
            rebound: self.rebound.clone(),
            dynamic: self.dynamic,
            opaque_namespaces: self.opaque_namespaces.clone(),
        }
    }

    /// Rebuild the queryable summary this snapshot was taken from.
    #[must_use]
    pub fn to_mutations(&self) -> ModuleCommandMutations {
        ModuleCommandMutations {
            names: self.untrusted_builtins.iter().cloned().collect(),
            rebound: self.rebound.iter().cloned().collect(),
            dynamic: self.dynamic,
            rebinding_subjects: self.rebinding_subjects,
            runtime_selected_frames: self.runtime_selected_frames,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: self.opaque_namespaces.iter().cloned().collect(),
            precision_written: self.precision_written,
        }
    }
}

/// The builtin a *qualified* definition shadows for bodies that resolve in its
/// namespace, if any.
///
/// `proc ::n::expr` does not rebind the builtin `expr` — its own name is
/// `::n::expr`, which has no builtin default, so the loop above drops it. But
/// an unqualified `expr` inside `::n` resolves to it first, so folding one
/// there with builtin semantics is wrong: `::n::expr` may return its argument
/// verbatim, and O110 rewriting `$r ** 2` into `$r * $r` then changes what the
/// program prints (#2159).
///
/// The answer is the bare tail, which distrusts the builtin for the whole
/// module rather than only inside `::n`. That is deliberately conservative and
/// matches what the `namespace eval ::n { proc expr … }` spelling already
/// does — the two spellings disagreeing is the defect. Narrowing it to the
/// defining namespace needs a resolution namespace at every call site, which
/// `trusts` does not take.
///
/// A namespace-local math function is the same shape one level down: from
/// 8.5 `expr` resolves `tcl::mathfunc::NAME` relative to the namespace it
/// runs in before the global one, so `proc ::ns::tcl::mathfunc::abs` is what
/// `abs(…)` calls inside `::ns` (tclsh 8.5 to 9.1 run it; 8.4 has no wrapper
/// commands). The answer is the global wrapper it shadows, distrusted for
/// the whole module as the `::n::expr` case is.
pub(super) fn builtin_shadowed_by_qualified_definition(
    name: &str,
    registry: &CommandRegistry,
) -> Option<String> {
    let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(name);
    if holder.is_empty() || tail.is_empty() {
        return None;
    }
    if tcl_registry::mathfunc::is_in_mathfunc_namespace(name) {
        let wrapper = tcl_registry::mathfunc::qualified_name(tail);
        return registry.get(&wrapper).is_some().then(|| nqn(&wrapper));
    }
    (default_binding(tail, registry).kind == BindingKind::Builtin).then(|| nqn(tail))
}

/// Whether `script` declares anything that could change what a command name
/// resolves to, at any nesting depth.
///
/// This is the question a body in an unnameable frame poses to the *rest* of
/// the module. A mutation there names its subject relatively, so it lands in a
/// namespace this scan cannot spell and no recorded name survives it. A body
/// that mutates nothing moves no name, whatever it calls, and leaves the names
/// the top level and the procedures resolve exactly as they were.
///
/// Registry-driven, like the walk it guards: a statement counts because its
/// declared state transitions touch command bindings or namespace resolution,
/// never because of the command's name. `Define` counts alongside the rest —
/// a `proc` in such a frame lands in a namespace this scan cannot spell, so
/// the widened stance is the right answer for it too.
///
/// Descends through [`crate::ir_helpers::nested_bodies`], the same inventory
/// [`crate::ir_helpers::requires_runtime_command_namespace`] walks, so the two
/// cannot disagree about which bodies belong to the frame. That inventory
/// covers the statically lowered `eval` and `uplevel` forms (`Statement::Block`
/// and `Statement::UpFrame`) as well as the structured ones, so a rebinding
/// hidden in `eval {rename ::string ::saved}` still widens.
///
/// Hitting the depth cap answers `true`: an unwalkable body is treated as one
/// that mutates.
fn declares_command_binding_effect(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    depth: u32,
) -> bool {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return true;
    }
    script.statements.iter().any(|stmt| {
        statement_declares_command_binding_effect(stmt, registry)
            || crate::ir_helpers::nested_bodies(stmt)
                .into_iter()
                .any(|body| declares_command_binding_effect(body, registry, depth + 1))
    })
}

/// The single-statement half of [`declares_command_binding_effect`].
fn statement_declares_command_binding_effect(stmt: &Statement, registry: &CommandRegistry) -> bool {
    let Some(facts) = invocation_facts(stmt, registry) else {
        return false;
    };
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    transitions
        .facts()
        .iter()
        .any(|fact| match &fact.transition {
            StateTransition::CommandBinding(_) | StateTransition::Namespace(_) => true,
            StateTransition::Widen(widening) => widening
                .domains
                .contains(&StateTransitionDomain::CommandBindings),
            StateTransition::Interpreter(_)
            | StateTransition::Package(_)
            | StateTransition::VariableCellAlias(_)
            | StateTransition::Trace(_)
            | StateTransition::ObjectDispatch(_) => false,
        })
}

/// Summarise command-table mutations across the whole module — a
/// CFG-free recursive IR walk over the top-level script *and* every proc
/// / method body, so it can run before per-function CFGs are built.
///
/// Tampered-with core builtins and rebound names generally are reported
/// (see [`collect_tampered_builtins`]).  The result feeds both the
/// optimiser's builtin-fold trust gate ([`ModuleCommandMutations::trusts`])
/// and its proc-call fold trust gate
/// ([`ModuleCommandMutations::trusts_proc_binding`]).
#[must_use]
pub fn scan_module_command_mutations(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> ModuleCommandMutations {
    let command_bindings = ModuleCommandBindings::analyse(ir_module, registry);
    scan_module_command_mutations_with_bindings(ir_module, registry, &command_bindings)
}

/// Join registry-declared namespace-resolution effects with an already-built
/// closed command-binding lattice.
///
/// Compilation-unit construction uses this seam so CFG construction,
/// optimiser trust, and runtime provenance consume one binding analysis. The
/// public convenience wrapper above remains available to callers that do not
/// already retain the prepared lattice.
#[must_use]
#[doc(hidden)]
pub fn scan_module_command_mutations_with_bindings(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
    command_bindings: &ModuleCommandBindings,
) -> ModuleCommandMutations {
    let mut projected = command_bindings.mutation_projection(registry);
    for (script, namespace) in ir_module.executable_script_roots() {
        if namespace == crate::ir::ExecutionNamespace::RuntimeSelected
            && crate::ir_helpers::requires_runtime_command_namespace(script, registry)
        {
            projected.runtime_selected_frames = true;
            projected.dynamic |= declares_command_binding_effect(script, registry, 0);
        }
    }
    projected.precision_written = module_may_set_precision(ir_module, registry);
    projected
}

/// Whether `ir_module` may set the variable that changes how `registry`'s
/// release spells a computed double
/// ([`tcl_dialect::NumberSyntax::precision_variable`]): its source names the
/// variable anywhere — `set`, `global`, `upvar`, a script string — or a
/// command writes a variable whose name, or a script that it runs, the
/// program computes ([`crate::dynamic_names::statement_barrier`]). Code the
/// module loads (`source`, `package require`) is not seen, as no fold sees
/// it. The original source bytes are searched as bytes.
pub(super) fn module_may_set_precision(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> PrecisionWrites {
    let Some(variable) = registry.numbers().precision_variable() else {
        return PrecisionWrites::Unwritten;
    };
    if ir_module
        .source
        .bytes()
        .windows(variable.len())
        .any(|window| window == variable.as_bytes())
    {
        return PrecisionWrites::MayBeWritten;
    }
    let config = crate::dynamic_names::lexer_config_for(registry);
    let declared = &ir_module.declared_frame_effects;
    let mut written = false;
    for (script, _) in ir_module.executable_script_roots() {
        crate::ir::for_each_statement(script, &mut |stmt| {
            written = written
                || crate::dynamic_names::statement_barrier(stmt, registry, declared, config).writes;
        });
    }
    if written {
        PrecisionWrites::MayBeWritten
    } else {
        PrecisionWrites::Unwritten
    }
}

/// Whether a module may set its release's precision variable — see
/// [`ModuleCommandMutations::may_set_precision`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(super) enum PrecisionWrites {
    /// Nothing in the module can name it.
    #[default]
    Unwritten,
    /// The module names it, or writes a variable whose name it computes.
    MayBeWritten,
}
