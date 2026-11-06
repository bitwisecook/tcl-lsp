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

//! The seam between the loader and the hook host: pack-declared hook bodies
//! become live function pointers on the specs a workspace installs.
//!
//! The loader ([`crate::loader`]) reads a `const_fold { … }` body as text and
//! installs its family's **abstention** on the spec — the pre-host behaviour,
//! and the answer a crashed or quarantined hook still gives. This module is
//! what turns that into behaviour, and it exists here rather than in either
//! neighbour because it is the only place that owns both sides: the loader
//! depends on no host type, the host ([`tcl_spec_hooks`]) depends on no loader
//! type, and `tcl-spectcl` already depends on `tcl-registry`, which hosts the
//! [`pack_hooks`] slot table both of them speak through.
//!
//! ## Two lifetimes, deliberately different
//!
//! A hook has a *process*-wide half and a *thread*-local half, and getting
//! them the wrong way round is the one bug this module is shaped to prevent:
//!
//! - **The slot is process-wide.** [`pack_hooks::allocate`] hands out a
//!   [`HookSlot`] whose thunk is baked into a leaked `&'static CommandSpec`
//!   that every thread reads. So slots are allocated **once per pack-set
//!   content key** ([`plan_for`]) and memoised — reinstalling the same packs,
//!   or installing them into a second dialect profile, reuses the same slots
//!   rather than burning a fresh one out of the 64 a family has.
//! - **The host is per thread.** A [`HookHost`](tcl_spec_hooks::HookHost) owns
//!   VM instances, which are not `Send`, so [`ensure_thread_host`] builds one
//!   per thread on first use and binds it to the slots the plan already
//!   assigned ([`HookProgram::with_slot`]). A thread that never calls it keeps
//!   abstaining, which is exactly a build with no packs loaded.
//!
//! ## What crosses
//!
//! Only bodies. A `-native ID` names engine code the pack cannot supply and a
//! derivation keyword (`from-manufacturers`, `clause_grammar`) is the loader's
//! own business; both keep the loader's installed behaviour untouched.
//!
//! ## What a workspace must be trusted for
//!
//! A body runs only for a pack whose provenance lets it
//! ([`hook_bodies_run`]): in a workspace the editor has not marked trusted,
//! [`plan_for`] allocates the pack's bodies no slot, so [`specialise`] leaves
//! the loader's abstaining placeholder — the abstention a declared-but-unbound
//! hook gives — while every declarative fact the pack states is installed as
//! before. The same list, [`dormant_hooks`], is what the load reports on the
//! pack file (`docs/design/compiler/registry-consumer-contracts.md` § *Ruling
//! — trust gates execution, not authority*).

mod owned_application;
mod owned_evaluator;
mod service;
pub use owned_application::{
    FoldAuthority, OwnedApplicationExecution, OwnedApplicationReceipt,
    execute_owned_application_source,
};
pub use owned_evaluator::{
    OwnedApplicationFold, OwnedApplicationFolds, OwnedEvaluatorTelemetry, OwnedFoldLineage,
    fold_owned_application_source,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use tcl_dialect::model::Provenance;
use tcl_registry::hover::{OptionArity, OptionSpec, OptionValue};
use tcl_registry::pack_hooks::{self, HookFamily, HookSlot};
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::value_transfer::{DeclaredSemantics, SemanticsDeclaration};
use tcl_spec_hooks::{HookOwner, HookProgram, PackPrograms};

use crate::loader::{HookDecl, HookOwner as DeclOwner, HookSource, PackCommand};
use crate::pack::PackSet;

// The adapter: HookDecl → HookProgram

/// Every hook **body** `commands` declare, beside the command it hangs off:
/// what the host would run. A `-native ID` names engine code the pack cannot
/// supply and a derivation keyword is the loader's own business, so neither
/// is one. [`programs_of`] and [`dormant_hooks`] both read this, so the hooks
/// a trusted workspace runs are exactly the ones an untrusted one reports.
fn bodies(commands: &[PackCommand]) -> impl Iterator<Item = (&PackCommand, &HookDecl)> {
    commands.iter().flat_map(|command| {
        command
            .hooks
            .iter()
            .filter(|hook| matches!(hook.source, HookSource::Body { .. }))
            .map(move |hook| (command, hook))
    })
}

/// One pack's declared hook **bodies**, as the host takes them.
///
/// The whole bridge between the two crates, and deliberately small: a
/// `HookDecl` carries where it was parsed from, a `HookProgram` carries only
/// what the calling convention needs.
#[must_use]
pub fn programs_of(
    pack: &str,
    dsl_version: &str,
    content_hash: &str,
    commands: &[PackCommand],
) -> PackPrograms {
    programs_with_sources(pack, dsl_version, content_hash, commands).0
}

/// Actual declaration allocation held separately from source display names.
#[derive(Debug, Clone, Copy)]
struct HookDeclarationAllocation(&'static CommandSpec);
impl PartialEq for HookDeclarationAllocation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for HookDeclarationAllocation {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalHookPlanDeclaration {
    declaration: HookDeclarationAllocation,
    file: PathBuf,
    pack: String,
    dsl_version: String,
    provenance: Provenance,
    hooks: Vec<HookDecl>,
}

struct OriginalHookProgramSource {
    declaration: HookDeclarationAllocation,
    file: PathBuf,
}

fn programs_with_sources(
    pack: &str,
    dsl_version: &str,
    content_hash: &str,
    commands: &[PackCommand],
) -> (PackPrograms, Vec<OriginalHookProgramSource>) {
    let mut sources = Vec::new();
    let mut programs = PackPrograms::new(pack);
    dsl_version.clone_into(&mut programs.dsl_version);
    content_hash.clone_into(&mut programs.content_hash);
    for (command, hook) in bodies(commands) {
        let HookSource::Body {
            params,
            body,
            inputs,
        } = &hook.source
        else {
            continue;
        };
        sources.push(OriginalHookProgramSource {
            declaration: HookDeclarationAllocation(command.spec),
            file: command.file.clone(),
        });
        programs.programs.push(HookProgram {
            command: command.spec.name.to_owned(),
            owner: owner_of(&hook.owner),
            family: hook.family,
            parameters: params.clone(),
            body: body.clone(),
            inputs: inputs.clone(),
            slot: None,
            // A declared implementation answers for the release the call is
            // analysed under, so its body runs on an engine pinned to that
            // release.
            release_pinned: hook.family == HookFamily::Evaluate,
        });
    }
    (programs, sources)
}

// Execution gated on trust

/// Whether a pack hook body of `provenance` runs — the execution half of the
/// trust ruling (`docs/design/compiler/registry-consumer-contracts.md`
/// § *Ruling — trust gates execution, not authority*).
///
/// Only a workspace the editor has not marked trusted holds its bodies
/// dormant: a body runs per query on words the analysed document supplies,
/// so it is the one pack surface whose inputs an untrusted workspace would
/// choose. A Spec Studio override is untrusted for registration (it may not
/// `-override` a compiled name) but runs — it is the author's own live edit,
/// and its bodies answering is what the studio shows. Authority reads none
/// of this: a dormant pack's declarative facts reach the registry unchanged.
#[must_use]
pub fn hook_bodies_run(provenance: Provenance) -> bool {
    provenance != Provenance::WorkspaceUntrusted
}

/// A declared hook body that does not run, because its pack's provenance
/// holds it dormant ([`hook_bodies_run`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DormantHook {
    /// The pack that declares it.
    pub pack: String,
    /// The command it hangs off.
    pub command: String,
    /// The field it would fill (`const_fold`, `options.arity_hook`,
    /// `state_transitions.resolver`, `evaluate`, …).
    pub field: &'static str,
    /// The file that declares it — the merge's record on the command, empty
    /// for a pack evaluated from a source with no path.
    pub file: PathBuf,
    /// The line of the row that declares it ([`HookDecl::line`]).
    pub line: u32,
}

/// The hook bodies `commands` declare that stay dormant for a pack named
/// `pack` of `provenance`: every one when its bodies do not run, none
/// otherwise. What the load reports ([`crate::pack::PackNotice::dormant`])
/// and what [`HookPlan::dormant`] lists.
#[must_use]
pub fn dormant_hooks(
    pack: &str,
    commands: &[PackCommand],
    provenance: Provenance,
) -> Vec<DormantHook> {
    if hook_bodies_run(provenance) {
        return Vec::new();
    }
    bodies(commands)
        .map(|(command, hook)| DormantHook {
            pack: pack.to_owned(),
            command: command.spec.name.to_owned(),
            field: hook.field,
            file: command.file.clone(),
            line: hook.line,
        })
        .collect()
}

fn owner_of(owner: &DeclOwner) -> HookOwner {
    match owner {
        DeclOwner::Command => HookOwner::Command,
        DeclOwner::Subcommand(name) => HookOwner::Subcommand(name.clone()),
        DeclOwner::Option { subcommand, option } => HookOwner::Option {
            subcommand: subcommand.clone(),
            option: option.clone(),
        },
    }
}

// The plan: one slot assignment per pack-set content

/// One pack set's hook bodies with the process-wide slot each is bound to.
///
/// Memoised for the actual source content and immutable declaration allocation
/// roster. Fresh same-content declarations reissue their own source bindings;
/// the typed stable source tuple keeps slots across edits and no-op reloads.
#[derive(Debug, Clone, Default)]
pub struct HookPlan {
    packs: Vec<PackPrograms>,
    dormant: Vec<DormantHook>,
    bindings: Vec<OriginalHookBinding>,
}

#[derive(Debug, Clone)]
struct OriginalHookBinding {
    declaration: HookDeclarationAllocation,
    file: PathBuf,
    pack: String,
    owner: HookOwner,
    family: HookFamily,
    slot: HookSlot,
}

impl HookPlan {
    /// One [`PackPrograms`] per pack whose bodies run, each program carrying
    /// its slot — what a thread's host loads. A pack whose bodies are dormant
    /// has none here: its hooks are in [`Self::dormant`].
    #[must_use]
    pub fn packs(&self) -> &[PackPrograms] {
        &self.packs
    }

    /// Every body the plan allocated no slot because its pack's workspace is
    /// untrusted ([`dormant_hooks`]) — what `spectcl_check` reports.
    #[must_use]
    pub fn dormant(&self) -> &[DormantHook] {
        &self.dormant
    }

    /// `true` when no pack has a hook body to run, which is the common case
    /// and the one that costs nothing. Dormant bodies do not count: a host
    /// would have nothing to serve for them.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.iter().all(|pack| pack.programs.is_empty())
    }

    /// Slots issued for this actual selected original declaration allocation.
    fn bindings_for<'a>(
        &'a self,
        command: &'a PackCommand,
    ) -> impl Iterator<Item = Binding<'a>> + 'a {
        self.bindings.iter().filter_map(move |binding| {
            (binding.declaration == HookDeclarationAllocation(command.spec)
                && binding.file == command.file)
                .then_some((
                    binding.pack.as_str(),
                    &binding.owner,
                    binding.family,
                    binding.slot,
                ))
        })
    }
}

type Binding<'a> = (&'a str, &'a HookOwner, HookFamily, HookSlot);

/// The slot assignment for `packs`, allocated once per content key.
#[must_use]
pub fn plan_for(packs: &PackSet) -> Arc<HookPlan> {
    type PlanEntry = (Vec<OriginalHookPlanDeclaration>, Arc<HookPlan>);
    static PLANS: LazyLock<Mutex<HashMap<u64, PlanEntry>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    static EMPTY: LazyLock<Arc<HookPlan>> = LazyLock::new(|| Arc::new(HookPlan::default()));

    if packs.key == 0 || packs.is_empty() {
        return Arc::clone(&EMPTY);
    }
    let mut plans = match PLANS.lock() {
        Ok(plans) => plans,
        Err(poisoned) => poisoned.into_inner(),
    };
    let source_roster: Vec<_> = packs
        .packs
        .iter()
        .flat_map(|pack| {
            pack.commands
                .iter()
                .map(move |command| OriginalHookPlanDeclaration {
                    declaration: HookDeclarationAllocation(command.spec),
                    file: command.file.clone(),
                    pack: pack.name.clone(),
                    dsl_version: pack.dsl_version.clone(),
                    provenance: pack.provenance(),
                    hooks: command.hooks.clone(),
                })
        })
        .collect();
    if let Some((original, plan)) = plans.get(&packs.key)
        && original == &source_roster
    {
        return Arc::clone(plan);
    }
    let hash = format!("{:016x}", packs.key);
    let mut built = HookPlan::default();
    for pack in &packs.packs {
        // An untrusted workspace's bodies get no slot, so `specialise` leaves
        // the loader's abstaining placeholder on each field and no host ever
        // sees the text. The pack's declarative facts install regardless.
        let provenance = pack.provenance();
        if !hook_bodies_run(provenance) {
            built
                .dormant
                .extend(dormant_hooks(&pack.name, &pack.commands, provenance));
            continue;
        }
        let (mut programs, sources) =
            programs_with_sources(&pack.name, &pack.dsl_version, &hash, &pack.commands);
        programs.programs = programs
            .programs
            .into_iter()
            .zip(sources)
            .filter_map(|(mut program, source)| {
                let owner = match &program.owner {
                    HookOwner::Command => pack_hooks::HookSourceOwner::Command,
                    HookOwner::Subcommand(name) => {
                        pack_hooks::HookSourceOwner::Subcommand(name.clone())
                    }
                    HookOwner::Option { subcommand, option } => {
                        pack_hooks::HookSourceOwner::Option {
                            subcommand: subcommand.clone(),
                            option: option.clone(),
                        }
                    }
                };
                let identity = pack_hooks::HookSourceIdentity {
                    pack: pack.name.clone(),
                    provenance,
                    file: source.file.clone(),
                    command: program.command.clone(),
                    owner,
                };
                let slot = pack_hooks::allocate_stable(program.family, &identity, &program.inputs)?;
                program.slot = Some(slot);
                built.bindings.push(OriginalHookBinding {
                    declaration: source.declaration,
                    file: source.file,
                    pack: pack.name.clone(),
                    owner: program.owner.clone(),
                    family: program.family,
                    slot,
                });
                Some(program)
            })
            .collect();
        built.packs.push(programs);
    }
    let plan = Arc::new(built);
    // Only the current memo entry for this existing content bucket is held.
    // Replacing it cannot invalidate independently retained live Arc plans.
    plans.insert(packs.key, (source_roster, Arc::clone(&plan)));
    plan
}

// Specialising a loaded spec with its slots' thunks

/// `command`'s spec with each declared hook body's thunk in place of the
/// loader's abstaining placeholder.
///
/// The thunk is an ordinary function pointer of the family's shipped type, so
/// every consumer downstream — the optimiser's const-subst engine, the
/// analyser's role walk, `hover.rs`'s option scanner — keeps reading the same
/// field it always read.
#[must_use]
pub fn specialise(command: &PackCommand, plan: &HookPlan) -> &'static CommandSpec {
    let spec = command.spec;
    let bindings: Vec<Binding<'_>> = plan.bindings_for(command).collect();
    if bindings.is_empty() {
        return spec;
    }
    let mut out: CommandSpec = spec.clone();
    for &(pack, owner, family, slot) in &bindings {
        if matches!(owner, HookOwner::Command) {
            bind_command(&mut out, pack, family, slot);
        }
    }
    if bindings.iter().any(|(_, owner, ..)| {
        matches!(
            owner,
            HookOwner::Option {
                subcommand: None,
                ..
            }
        )
    }) {
        let mut options: Vec<OptionSpec> = out.options.to_vec();
        for &(_, owner, _, slot) in &bindings {
            if let HookOwner::Option {
                subcommand: None,
                option,
            } = owner
            {
                bind_option(&mut options, option, slot);
            }
        }
        out.options = Box::leak(options.into_boxed_slice());
    }
    if bindings
        .iter()
        .any(|(_, owner, ..)| owner.subcommand().is_some())
    {
        let mut subs: Vec<SubCommand> = out.subcommands.to_vec();
        for sub in &mut subs {
            bind_subcommand(sub, &bindings);
        }
        out.subcommands = Box::leak(subs.into_boxed_slice());
    }
    Box::leak(Box::new(out))
}

/// `declaration` with its declared implementation's body bound to `slot`
/// and its identity naming `pack`; any other declaration unchanged.
fn bind_semantics(
    declaration: SemanticsDeclaration,
    pack: &str,
    slot: HookSlot,
) -> SemanticsDeclaration {
    let SemanticsDeclaration::Declared(semantics) = declaration else {
        return declaration;
    };
    let Some(declared) = semantics.as_declared() else {
        return declaration;
    };
    let bound: &'static DeclaredSemantics = Box::leak(Box::new(
        declared.bound(crate::loader::leak_str(pack), slot),
    ));
    SemanticsDeclaration::Declared(bound)
}

fn bind_command(spec: &mut CommandSpec, pack: &str, family: HookFamily, slot: HookSlot) {
    match family {
        HookFamily::ArgRoleResolver => {
            spec.arg_role_resolver = pack_hooks::arg_role_resolver_fn(slot);
        }
        HookFamily::CommandPrefixResolver => {
            spec.command_prefix_resolver = pack_hooks::command_prefix_resolver_fn(slot);
        }
        HookFamily::ScriptTimingResolver => {
            spec.script_timing_resolver = pack_hooks::script_timing_resolver_fn(slot);
        }
        HookFamily::ConstFold => spec.const_fold = pack_hooks::const_fold_fn(slot),
        HookFamily::ConstFoldVersioned => {
            spec.const_fold_versioned = pack_hooks::const_fold_versioned_fn(slot);
        }
        HookFamily::TaintSinkGate => spec.taint_sink_gate = pack_hooks::taint_sink_gate_fn(slot),
        HookFamily::ContextGate => spec.context_gate = pack_hooks::context_gate_fn(slot),
        HookFamily::LiteralArgumentValidator => {
            spec.literal_argument_validator = pack_hooks::literal_argument_validator_fn(slot);
        }
        HookFamily::ClauseShapeCheck => {
            spec.clause_shape_check = pack_hooks::clause_shape_check_fn(slot);
        }
        HookFamily::Constraints => spec.constraints = pack_hooks::constraints_fn(slot),
        HookFamily::Evaluate => spec.semantics = bind_semantics(spec.semantics, pack, slot),
        HookFamily::StateTransitionResolver => {
            bind_transition_resolver(spec.state_transitions.as_mut(), slot);
        }
        // An option's `-arity-hook` never hangs off the command itself.
        HookFamily::OptionArity => {}
    }
}

fn bind_subcommand(sub: &mut SubCommand, bindings: &[Binding<'_>]) {
    let mut options: Option<Vec<OptionSpec>> = None;
    for &(pack, owner, family, slot) in bindings {
        if owner.subcommand() != Some(sub.name) {
            continue;
        }
        match owner {
            HookOwner::Subcommand(_) => match family {
                HookFamily::ArgRoleResolver => {
                    sub.arg_role_resolver = pack_hooks::arg_role_resolver_fn(slot);
                }
                HookFamily::CommandPrefixResolver => {
                    sub.command_prefix_resolver = pack_hooks::command_prefix_resolver_fn(slot);
                }
                HookFamily::ScriptTimingResolver => {
                    sub.script_timing_resolver = pack_hooks::script_timing_resolver_fn(slot);
                }
                HookFamily::ConstFold => sub.const_fold = pack_hooks::const_fold_fn(slot),
                HookFamily::ConstFoldVersioned => {
                    sub.const_fold_versioned = pack_hooks::const_fold_versioned_fn(slot);
                }
                HookFamily::LiteralArgumentValidator => {
                    sub.literal_argument_validator =
                        pack_hooks::literal_argument_validator_fn(slot);
                }
                HookFamily::Constraints => sub.constraints = pack_hooks::constraints_fn(slot),
                HookFamily::Evaluate => {
                    sub.semantics = bind_semantics(sub.semantics, pack, slot);
                }
                HookFamily::StateTransitionResolver => {
                    bind_transition_resolver(sub.state_transitions.as_mut(), slot);
                }
                // The loader declares no other family on a subcommand row.
                HookFamily::TaintSinkGate
                | HookFamily::ContextGate
                | HookFamily::ClauseShapeCheck
                | HookFamily::OptionArity => {}
            },
            HookOwner::Option { option, .. } => {
                bind_option(
                    options.get_or_insert_with(|| sub.options.to_vec()),
                    option,
                    slot,
                );
            }
            HookOwner::Command => {}
        }
    }
    if let Some(options) = options {
        sub.options = Box::leak(options.into_boxed_slice());
    }
}

/// A `state_transitions` block's resolver body bound to `slot`: the
/// descriptor keeps its plain rows and takes the slot's thunk in place of
/// the loader's abstaining placeholder.
fn bind_transition_resolver(
    descriptor: Option<&mut tcl_registry::state_transition::StateTransitionDescriptor>,
    slot: HookSlot,
) {
    if let Some(descriptor) = descriptor {
        descriptor.resolver = pack_hooks::state_transition_resolver_fn(slot);
    }
}

fn bind_option(options: &mut [OptionSpec], name: &str, slot: HookSlot) {
    let Some(hook) = pack_hooks::option_arity_fn(slot) else {
        return;
    };
    for option in options.iter_mut() {
        if option.name != name {
            continue;
        }
        if let OptionValue::Takes(arg) = &mut option.value
            && matches!(arg.arity, OptionArity::Hook(_))
        {
            arg.arity = OptionArity::Hook(hook);
        }
    }
}

// The per-thread host

/// One coherent publication: generation in the upper bits, empty-plan flag
/// in bit zero. Zero means no pack hooks have ever been published.
static PUBLICATION: AtomicU64 = AtomicU64::new(0);
static PUBLISHED: Mutex<Option<Arc<HookPlan>>> = Mutex::new(None);

thread_local! {
    /// The generation this thread's installed host was built from.
    static INSTALLED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Publish `packs` as the hook plan every worker thread should serve.
///
/// Called where the packs are *loaded* — one workspace has one pack set — and
/// deliberately not where they are queried: publishing only records what to
/// build, since the host itself cannot cross a thread boundary.
///
/// Publishing an empty plan (no pack declares a body, or the packs went away)
/// retires the previous one: threads clear their host on their next
/// [`ensure_thread_host`] and every pack hook abstains again.
pub fn publish(packs: &PackSet) {
    publish_with_before_commit(packs, || {});
}

fn publish_with_before_commit(packs: &PackSet, before_commit: impl FnOnce()) {
    let plan = plan_for(packs);
    let mut published = match PUBLISHED.lock() {
        Ok(published) => published,
        Err(poisoned) => poisoned.into_inner(),
    };
    if plan.is_empty() && published.is_none() {
        // Nothing to serve and nothing to retire: leave the generation at
        // zero so an unaccelerated process never pays for the check.
        return;
    }
    let generation = (PUBLICATION.load(Ordering::Relaxed) >> 1).wrapping_add(1);
    let publication = (generation << 1) | u64::from(plan.is_empty());
    // Bodies whose answers ship precompiled are tabulated now; the one host
    // that runs the rest is built on the hook service, not by whichever
    // worker first needs it.
    let tables = pack_hooks::begin_tabulation_plan();
    let precompiled = apply_precompiled_tables(&plan, &tables);
    service::install(Arc::clone(&plan), &precompiled, tables.clone());
    before_commit();
    *published = Some(plan);
    PUBLICATION.store(publication, Ordering::Release);
    // A new plan is new evaluators for every thread that serves it: a memo
    // shared between threads re-keys on the process's evaluator epoch.
    pack_hooks::advance_evaluator_epoch();
    // Teach the registry how to build a thread's host, so any thread that
    // dispatches a hook gets one whether or not its worker closure remembered
    // to call `ensure_thread_host`. Registered here because this is the moment
    // a host becomes worth building at all; idempotent, so repeated reloads
    // cost one `OnceLock` read.
    pack_hooks::set_installer(ensure_thread_host);
    pack_hooks::set_pending_host(pending_thread_host);
    let _activated = pack_hooks::activate_tabulation_plan(&tables);
}

/// Tabulate every argument-role resolver of `plan` whose answers were
/// precompiled into the binary ([`crate::bundled_manifest::BUNDLED_ROLE_TABLES`]),
/// returning their slots.
fn apply_precompiled_tables(
    plan: &HookPlan,
    tables: &pack_hooks::TabulationPlan,
) -> std::collections::BTreeSet<HookSlot> {
    let mut applied = std::collections::BTreeSet::new();
    for programs in plan.packs() {
        for program in &programs.programs {
            let Some(slot) = program.slot else {
                continue;
            };
            if program.family != HookFamily::ArgRoleResolver || !service::is_tabulable(program) {
                continue;
            }
            let hash = service::body_hash(program);
            let Some(row) = crate::bundled_manifest::BUNDLED_ROLE_TABLES
                .iter()
                .find(|row| row.body_hash == hash)
            else {
                continue;
            };
            let tabulated = pack_hooks::tabulate(
                tables,
                slot,
                row.answers
                    .iter()
                    .map(|answer| {
                        answer.map_or(pack_hooks::HookAnswer::Abstain, |roles| {
                            pack_hooks::HookAnswer::Roles(roles.to_vec())
                        })
                    })
                    .collect(),
            );
            if tabulated {
                applied.insert(slot);
            }
        }
    }
    applied
}

/// How many hook hosts — a VM bootstrap per pack each — this process has
/// built. A plan whose bodies all ship precompiled builds none.
#[must_use]
pub fn hosts_built() -> u64 {
    service::HOSTS_BUILT.load(Ordering::Relaxed)
}

/// One precompiled argument-role resolver table: the body's
/// [`service::body_hash`] and its answer at each word count, `None` where it
/// abstains.
pub type RoleTable = (u64, Vec<Option<Vec<(u8, tcl_registry::arg_role::ArgRole)>>>);

/// Run every count-only argument-role resolver `packs` declares and return
/// its answers by word count — what `cargo xtask gen-bundled-manifests`
/// precompiles for the bundled packs, so no process has to start a VM for
/// them.
#[must_use]
pub fn precompute_role_tables(packs: &PackSet) -> Vec<RoleTable> {
    let plan = plan_for(packs);
    if plan.is_empty() {
        return Vec::new();
    }
    let host = service::build_host(&plan);
    let mut tables = Vec::new();
    for programs in plan.packs() {
        for program in &programs.programs {
            let Some(slot) = program.slot else {
                continue;
            };
            if program.family != HookFamily::ArgRoleResolver || !service::is_tabulable(program) {
                continue;
            }
            let Some(answers) = service::table_for(host.as_ref(), slot) else {
                continue;
            };
            let answers: Option<Vec<_>> = answers
                .into_iter()
                .map(|answer| match answer {
                    pack_hooks::HookAnswer::Abstain => Some(None),
                    pack_hooks::HookAnswer::Roles(roles) => Some(Some(roles)),
                    _ => None,
                })
                .collect();
            if let Some(answers) = answers {
                tables.push((service::body_hash(program), answers));
            }
        }
    }
    tables.sort_by_key(|(hash, _)| *hash);
    tables.dedup_by_key(|(hash, _)| *hash);
    tables
}

/// What [`ensure_thread_host`] would do on this thread, without building
/// anything: the generation question compilation asks on every unit does not
/// need a VM, only the hook calls do.
fn pending_thread_host() -> pack_hooks::PendingHost {
    pending_thread_host_at(PUBLICATION.load(Ordering::Acquire))
}

fn pending_thread_host_at(publication: u64) -> pack_hooks::PendingHost {
    let generation = publication >> 1;
    if generation == 0 || INSTALLED.with(std::cell::Cell::get) == generation {
        return pack_hooks::PendingHost::Current;
    }
    if publication & 1 != 0 {
        pack_hooks::PendingHost::Cleared
    } else {
        pack_hooks::PendingHost::Plan(generation)
    }
}

/// Give this thread a hook host for the published plan, if it does not have
/// one already.
///
/// Cheap at any point: one relaxed atomic load in a process with no pack
/// hooks, one thread-local read once it has them, and a forwarding proxy to
/// install after a publish — the sandboxed VMs live on the hook service
/// ([`service`]), built once per published plan.
pub fn ensure_thread_host() {
    ensure_thread_host_with(|| {});
}

fn ensure_thread_host_with(before_snapshot: impl FnOnce()) {
    let generation = PUBLICATION.load(Ordering::Acquire) >> 1;
    if generation == 0 || INSTALLED.with(std::cell::Cell::get) == generation {
        return;
    }
    before_snapshot();
    let (plan, generation) = {
        let published = match PUBLISHED.lock() {
            Ok(published) => published,
            Err(poisoned) => poisoned.into_inner(),
        };
        (published.clone(), PUBLICATION.load(Ordering::Acquire) >> 1)
    };
    let Some(plan) = plan else {
        return;
    };
    if plan.is_empty() {
        pack_hooks::clear_host();
    } else {
        // A worker forwards to the hook service, which owns the one host:
        // nothing here bootstraps a VM. Every worker serving this plan shares
        // one evaluator generation, so their memoised answers are shared too.
        pack_hooks::install_plan_host(Rc::new(service::Proxy), generation);
    }
    INSTALLED.with(|installed| installed.set(generation));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{Origin, PackFile, Tier};
    use std::path::PathBuf;

    const SOURCE: &str = r"
speclib hooked 1 {
    command hooked::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            fold [string length [lindex $words 0]]
        }
    }
}
";

    /// Loading a pack writes a compiled-cache entry, and `cache`'s own tests
    /// count the entries under a redirected directory — so every test in this
    /// crate that loads a pack holds the same lock they do.
    fn cache_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::cache::REDIRECT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn pack_set(name: &str, source: &str) -> PackSet {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-hooks-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path: PathBuf = dir.join("pack.tclspec");
        std::fs::write(&path, source).expect("write pack");
        crate::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
            dependency_tier: None,
        }])
    }

    #[test]
    fn a_staged_actor_plan_cannot_answer_under_the_previous_publication() {
        use tcl_registry::pack_hooks::{HookAnswer, HookCall, HookWord, PackHookHost};
        let _cache = cache_guard();
        let old = pack_set("staged-publication", SOURCE);
        publish(&old);
        let plan = plan_for(&old);
        let old_command = old.packs[0].command("hooked::strlen").unwrap();
        let fold = specialise(old_command, &plan).const_fold.unwrap();
        let slot = plan.packs()[0].programs[0].slot.unwrap();
        let changed = pack_set(
            "staged-publication",
            &SOURCE.replace("fold [string length [lindex $words 0]]", "fold 99"),
        );
        assert_eq!(plan_for(&changed).packs()[0].programs[0].slot, Some(slot));
        let (ready, waiting) = std::sync::mpsc::sync_channel(0);
        let (test_stage, stage_requested) = std::sync::mpsc::sync_channel(0);
        let (observed, stage_observed) = std::sync::mpsc::sync_channel(0);
        let (committed, after_commit) = std::sync::mpsc::sync_channel(0);
        let worker = std::thread::spawn(move || {
            assert_eq!(fold(&["x"]), Some("1".to_owned()));
            assert_eq!(fold(&["x"]), Some("1".to_owned()));
            ready.send(()).unwrap();
            stage_requested.recv().unwrap();
            assert_eq!(
                fold(&["x"]),
                None,
                "a warm old answer cannot cross a staged publication"
            );
            assert_eq!(
                fold(&["not-warm"]),
                None,
                "an old-generation miss cannot reach the staged actor"
            );
            observed.send(()).unwrap();
            after_commit.recv().unwrap();
            assert_eq!(fold(&["x"]), Some("99".to_owned()));
        });
        waiting.recv().unwrap();
        publish_with_before_commit(&changed, || {
            // The actual new actor has installed and run its body, while the
            // public generation still names the old plan.
            let words = [HookWord {
                value: "x",
                kind: tcl_registry::InvocationWordKind::Literal,
            }];
            let call = HookCall {
                words: &words,
                version: None,
                in_event_body: false,
                option: None,
                constraints: None,
                dialect: None,
                targets: &[],
                budget: tcl_registry::value_transfer::ImplementationBudget::default(),
                depends: &[],
            };
            assert_eq!(
                service::Proxy.invoke(slot, &call),
                HookAnswer::Fold("99".to_owned())
            );
            test_stage.send(()).unwrap();
            stage_observed.recv().unwrap();
        });
        committed.send(()).unwrap();
        worker.join().unwrap();
        publish(&PackSet::default());
    }
    /// A reload that changes nothing must not spend a slot: 64 per family is
    /// a compile-time bound, and an editor reloads a pack on every save.
    #[test]
    fn the_same_pack_content_keeps_the_same_slots() {
        let _cache = cache_guard();
        let a = pack_set("same", SOURCE);
        let b = pack_set("same", SOURCE);
        assert_eq!(a.key, b.key, "same path, same bytes, same key");
        let (x, y) = (plan_for(&a), plan_for(&b));
        assert_eq!(x.packs()[0].programs[0].slot, y.packs()[0].programs[0].slot);
        assert!(x.packs()[0].programs[0].slot.is_some());
    }

    #[test]
    fn a_declared_body_replaces_the_abstaining_placeholder() {
        let _cache = cache_guard();
        let packs = pack_set("specialise", SOURCE);
        let plan = plan_for(&packs);
        let command = packs.packs[0]
            .command("hooked::strlen")
            .expect("the pack declares it");
        assert!(
            command.spec.const_fold.is_some(),
            "the loader installs an abstaining placeholder"
        );
        let specialised = specialise(command, &plan);
        assert!(specialised.const_fold.is_some());
        assert!(
            !std::ptr::eq(specialised, command.spec),
            "a specialised spec is a distinct leak, so the loaded one is untouched"
        );
    }

    #[test]
    fn a_pack_with_no_bodies_is_not_respecified() {
        let _cache = cache_guard();
        let packs = pack_set(
            "bodiless",
            "speclib plain 1 {\n  command plain::thing { arity 1 }\n}\n",
        );
        let plan = plan_for(&packs);
        assert!(plan.is_empty());
        let command = packs.packs[0].command("plain::thing").expect("declared");
        assert!(std::ptr::eq(specialise(command, &plan), command.spec));
    }

    #[test]
    fn actual_worker_cached_answer_rejoins_reload_and_retirement_before_dispatch() {
        let _cache = cache_guard();
        let first = pack_set("current-plan-cache", SOURCE);
        let first_plan = plan_for(&first);
        let slot = first_plan.packs()[0].programs[0].slot.unwrap();
        let fold = pack_hooks::const_fold_fn(slot).unwrap();
        publish(&first);
        let (ready_send, ready_receive) = std::sync::mpsc::channel();
        let (advance_send, advance_receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            pack_hooks::clear_host();
            // No caller-side generation query or manual host synchronization.
            assert!(pack_hooks::slot_available(slot));
            assert_eq!(fold(&["abc"]), Some("3".to_owned()));
            assert_eq!(fold(&["abc"]), Some("3".to_owned()));
            ready_send.send(()).unwrap();
            advance_receive
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
            assert_eq!(fold(&["abc"]), Some("99".to_owned()));
            assert_eq!(fold(&["abc"]), Some("99".to_owned()));
            ready_send.send(()).unwrap();
            advance_receive
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
            assert_eq!(fold(&["abc"]), None);
            assert!(!pack_hooks::slot_available(slot));
            pack_hooks::clear_host();
        });
        ready_receive
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        let changed = SOURCE.replace("fold [string length [lindex $words 0]]", "fold 99");
        let second = pack_set("current-plan-cache", &changed);
        let second_plan = plan_for(&second);
        assert_eq!(second_plan.packs()[0].programs[0].slot, Some(slot));
        publish(&second);
        advance_send.send(()).unwrap();
        ready_receive
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        publish(&PackSet::default());
        advance_send.send(()).unwrap();
        worker.join().expect("actual worker finished");
    }

    #[test]
    fn publication_reads_keep_plan_emptiness_and_host_generation_coherent() {
        let _cache = cache_guard();
        let first = pack_set("coherent-publication", SOURCE);
        publish(&first);
        let old_publication = PUBLICATION.load(Ordering::Acquire);
        let (ready_send, ready_receive) = std::sync::mpsc::channel();
        let (advance_send, advance_receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            assert_eq!(
                pending_thread_host_at(old_publication),
                pack_hooks::PendingHost::Plan(old_publication >> 1)
            );
            ensure_thread_host_with(|| {
                ready_send.send(()).unwrap();
                advance_receive
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .unwrap();
            });
            assert_eq!(
                INSTALLED.with(std::cell::Cell::get),
                PUBLICATION.load(Ordering::Acquire) >> 1,
                "a plan acquired after publication must carry that publication's generation"
            );
            assert_eq!(pending_thread_host(), pack_hooks::PendingHost::Current);
            let plan = {
                let published = PUBLISHED.lock().unwrap();
                Arc::clone(published.as_ref().unwrap())
            };
            let slot = plan.packs()[0].programs[0].slot.unwrap();
            assert_eq!(
                pack_hooks::const_fold_fn(slot).unwrap()(&["abc"]),
                Some("99".to_owned())
            );
            pack_hooks::clear_host();
        });
        ready_receive
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        let changed = SOURCE.replace("fold [string length [lindex $words 0]]", "fold 99");
        let second = pack_set("coherent-publication", &changed);
        publish(&second);
        advance_send.send(()).unwrap();
        worker.join().expect("publication worker finished");

        publish(&PackSet::default());
        std::thread::spawn(move || {
            assert_eq!(
                pending_thread_host_at(old_publication),
                pack_hooks::PendingHost::Plan(old_publication >> 1),
                "a captured generation retains its own emptiness after publication"
            );
            assert_eq!(pending_thread_host(), pack_hooks::PendingHost::Cleared);
            ensure_thread_host();
            assert_eq!(
                pack_hooks::evaluator_generation(),
                tcl_registry::value_transfer::EvaluatorGeneration::NO_HOST
            );
        })
        .join()
        .expect("empty publication worker finished");
    }

    #[test]
    fn actual_distinct_option_fields_have_distinct_stable_slots() {
        let _cache = cache_guard();
        let packs = pack_set(
            "option-owner-fields",
            "speclib options 1 {command shared {option -first -arity-hook {words ctx} {consume 1}; option -second -arity-hook {words ctx} {consume 2}}}",
        );
        let plan = plan_for(&packs);
        let programs = &plan.packs()[0].programs;
        assert_eq!(programs.len(), 2);
        assert!(
            matches!(&programs[0].owner, HookOwner::Option { option, .. } if option == "-first")
        );
        assert!(
            matches!(&programs[1].owner, HookOwner::Option { option, .. } if option == "-second")
        );
        assert_ne!(
            programs[0].slot, programs[1].slot,
            "actual option components cannot share a host slot"
        );
    }

    #[test]
    fn same_display_command_from_distinct_actual_declarations_keeps_its_own_binding() {
        let _cache = cache_guard();
        let first = pack_set(
            "first-source-owner",
            "speclib first 1 {command shared {const_fold -inputs {words} {words ctx} {fold 1}}}",
        );
        let second = pack_set(
            "second-source-owner",
            "speclib second 1 {command shared {const_fold -inputs {words} {words ctx} {fold 2}}}",
        );
        let files = first
            .files()
            .into_iter()
            .chain(second.files())
            .map(|path| PackFile {
                tier: Tier::Workspace,
                path,
                origin: Origin::DotDir,
                dependency_tier: None,
            })
            .collect::<Vec<_>>();
        let packs = crate::pack::load(&files);
        let plan = plan_for(&packs);
        assert_eq!(plan.packs().len(), 2);
        let one = packs
            .packs
            .iter()
            .find(|pack| pack.name == "first")
            .unwrap()
            .command("shared")
            .unwrap();
        let two = packs
            .packs
            .iter()
            .find(|pack| pack.name == "second")
            .unwrap()
            .command("shared")
            .unwrap();
        let first_fn = specialise(one, &plan).const_fold.unwrap();
        let second_fn = specialise(two, &plan).const_fold.unwrap();
        assert!(!std::ptr::fn_addr_eq(first_fn, second_fn));
        let first_slot = plan
            .packs()
            .iter()
            .find(|pack| pack.pack == "first")
            .unwrap()
            .programs[0]
            .slot
            .unwrap();
        let second_slot = plan
            .packs()
            .iter()
            .find(|pack| pack.pack == "second")
            .unwrap()
            .programs[0]
            .slot
            .unwrap();
        assert!(std::ptr::fn_addr_eq(
            first_fn,
            pack_hooks::const_fold_fn(first_slot).unwrap()
        ));
        assert!(std::ptr::fn_addr_eq(
            second_fn,
            pack_hooks::const_fold_fn(second_slot).unwrap()
        ));
    }

    #[test]
    fn freshly_reissued_same_content_declaration_retains_live_binding_and_stable_slot() {
        let _cache = cache_guard();
        let first = pack_set("reissued-owner", SOURCE);
        let first_plan = plan_for(&first);
        crate::cache::forget_snapshots();
        let second = pack_set("reissued-owner", SOURCE);
        assert_eq!(first.key, second.key);
        let one = first.packs[0].command("hooked::strlen").unwrap();
        let two = second.packs[0].command("hooked::strlen").unwrap();
        assert!(
            !std::ptr::eq(one.spec, two.spec),
            "actual fresh loader allocation"
        );
        let second_plan = plan_for(&second);
        assert_eq!(
            first_plan.packs()[0].programs[0].slot,
            second_plan.packs()[0].programs[0].slot
        );
        assert!(
            !std::ptr::eq(specialise(two, &second_plan), two.spec),
            "same source content must not lose its newly selected declaration binding"
        );
    }

    #[test]
    fn same_key_fresh_declarations_do_not_retain_historical_unheld_plans() {
        let _cache = cache_guard();
        let first = pack_set("bounded-owner-reissue", SOURCE);
        let live = plan_for(&first);
        let mut previous: Option<std::sync::Weak<HookPlan>> = None;
        for _ in 0..4 {
            crate::cache::forget_snapshots();
            let reissued = pack_set("bounded-owner-reissue", SOURCE);
            assert_eq!(reissued.key, first.key);
            let current = plan_for(&reissued);
            if let Some(previous) = previous.take() {
                assert!(
                    previous.upgrade().is_none(),
                    "replaced unheld same-key plan must not remain retained by memo history"
                );
            }
            let declaration = reissued.packs[0].command("hooked::strlen").unwrap();
            assert!(!std::ptr::eq(
                specialise(declaration, &current),
                declaration.spec
            ));
            previous = Some(Arc::downgrade(&current));
        }
        assert!(
            previous.unwrap().upgrade().is_some(),
            "only current memo plan remains held"
        );
        let original = first.packs[0].command("hooked::strlen").unwrap();
        assert!(
            !std::ptr::eq(specialise(original, &live), original.spec),
            "external live Arc keeps its own genuine original declaration binding"
        );
    }
}
