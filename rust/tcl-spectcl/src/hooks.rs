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

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use tcl_registry::hover::{OptionArity, OptionSpec, OptionValue};
use tcl_registry::pack_hooks::{self, HookFamily, HookSlot};
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_spec_hooks::{HookOwner, HookProgram, PackPrograms, tclvm_host};

use crate::loader::{HookOwner as DeclOwner, HookSource, PackCommand};
use crate::pack::PackSet;

// ---------------------------------------------------------------------------
// The adapter: HookDecl → HookProgram
// ---------------------------------------------------------------------------

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
    let mut programs = PackPrograms::new(pack);
    dsl_version.clone_into(&mut programs.dsl_version);
    content_hash.clone_into(&mut programs.content_hash);
    for command in commands {
        for hook in &command.hooks {
            // Only a Tcl body crosses: `-native ID` names engine code and a
            // derivation keyword is the loader's own business.
            let HookSource::Body {
                params,
                body,
                inputs,
            } = &hook.source
            else {
                continue;
            };
            programs.programs.push(HookProgram {
                command: command.spec.name.to_owned(),
                owner: owner_of(&hook.owner),
                family: hook.family,
                parameters: params.clone(),
                body: body.clone(),
                inputs: inputs.clone(),
                slot: None,
            });
        }
    }
    programs
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

// ---------------------------------------------------------------------------
// The plan: one slot assignment per pack-set content
// ---------------------------------------------------------------------------

/// One pack set's hook bodies with the process-wide slot each is bound to.
///
/// Built once per [`PackSet::key`] and shared: the same content installed into
/// three dialect profiles, or reinstalled after a no-op reload, gets the same
/// slots, so the 64-per-family budget is spent on distinct pack *content*
/// rather than on how often it is asked for.
#[derive(Debug, Clone, Default)]
pub struct HookPlan {
    packs: Vec<PackPrograms>,
}

impl HookPlan {
    /// One [`PackPrograms`] per pack, each program carrying its slot — what a
    /// thread's host loads.
    #[must_use]
    pub fn packs(&self) -> &[PackPrograms] {
        &self.packs
    }

    /// `true` when no pack declared a hook body, which is the common case and
    /// the one that costs nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.iter().all(|pack| pack.programs.is_empty())
    }

    /// Every slot bound for `command`, as `(owner, family, slot)`.
    fn bindings_for<'a>(&'a self, command: &'a str) -> impl Iterator<Item = Binding<'a>> + 'a {
        self.packs.iter().flat_map(move |pack| {
            pack.programs.iter().filter_map(move |program| {
                (program.command == command)
                    .then(|| program.slot.map(|slot| (&program.owner, program.family, slot)))
                    .flatten()
            })
        })
    }
}

type Binding<'a> = (&'a HookOwner, HookFamily, HookSlot);

/// The slot assignment for `packs`, allocated once per content key.
#[must_use]
pub fn plan_for(packs: &PackSet) -> Arc<HookPlan> {
    static PLANS: LazyLock<Mutex<HashMap<u64, Arc<HookPlan>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    static EMPTY: LazyLock<Arc<HookPlan>> = LazyLock::new(|| Arc::new(HookPlan::default()));

    if packs.key == 0 || packs.is_empty() {
        return Arc::clone(&EMPTY);
    }
    let mut plans = match PLANS.lock() {
        Ok(plans) => plans,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(plan) = plans.get(&packs.key) {
        return Arc::clone(plan);
    }
    let hash = format!("{:016x}", packs.key);
    let mut built = HookPlan::default();
    for pack in &packs.packs {
        let mut programs = programs_of(&pack.name, &pack.dsl_version, &hash, &pack.commands);
        // A family with no slots left installs no hook at all: the command
        // keeps its declarative facts, which is the documented degradation and
        // is cheaper than an abstaining call per query.
        programs
            .programs
            .retain_mut(|program| match pack_hooks::allocate(program.family, &program.inputs) {
                Some(slot) => {
                    program.slot = Some(slot);
                    true
                }
                None => false,
            });
        built.packs.push(programs);
    }
    let plan = Arc::new(built);
    plans.insert(packs.key, Arc::clone(&plan));
    plan
}

// ---------------------------------------------------------------------------
// Specialising a loaded spec with its slots' thunks
// ---------------------------------------------------------------------------

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
    let bindings: Vec<Binding<'_>> = plan.bindings_for(spec.name).collect();
    if bindings.is_empty() {
        return spec;
    }
    let mut out: CommandSpec = spec.clone();
    for &(owner, family, slot) in &bindings {
        if matches!(owner, HookOwner::Command) {
            bind_command(&mut out, family, slot);
        }
    }
    if bindings
        .iter()
        .any(|(owner, ..)| matches!(owner, HookOwner::Option { subcommand: None, .. }))
    {
        let mut options: Vec<OptionSpec> = out.options.to_vec();
        for &(owner, _, slot) in &bindings {
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
    if bindings.iter().any(|(owner, ..)| owner.subcommand().is_some()) {
        let mut subs: Vec<SubCommand> = out.subcommands.to_vec();
        for sub in &mut subs {
            bind_subcommand(sub, &bindings);
        }
        out.subcommands = Box::leak(subs.into_boxed_slice());
    }
    Box::leak(Box::new(out))
}

fn bind_command(spec: &mut CommandSpec, family: HookFamily, slot: HookSlot) {
    match family {
        HookFamily::ArgRoleResolver => {
            spec.arg_role_resolver = pack_hooks::arg_role_resolver_fn(slot);
        }
        HookFamily::CommandPrefixResolver => {
            spec.command_prefix_resolver = pack_hooks::command_prefix_resolver_fn(slot);
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
        // An option's `-arity-hook` never hangs off the command itself.
        HookFamily::OptionArity => {}
    }
}

fn bind_subcommand(sub: &mut SubCommand, bindings: &[Binding<'_>]) {
    let mut options: Option<Vec<OptionSpec>> = None;
    for &(owner, family, slot) in bindings {
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
                HookFamily::ConstFold => sub.const_fold = pack_hooks::const_fold_fn(slot),
                HookFamily::ConstFoldVersioned => {
                    sub.const_fold_versioned = pack_hooks::const_fold_versioned_fn(slot);
                }
                HookFamily::LiteralArgumentValidator => {
                    sub.literal_argument_validator =
                        pack_hooks::literal_argument_validator_fn(slot);
                }
                // The loader declares no other family on a subcommand row.
                HookFamily::TaintSinkGate
                | HookFamily::ContextGate
                | HookFamily::ClauseShapeCheck
                | HookFamily::OptionArity => {}
            },
            HookOwner::Option { option, .. } => {
                bind_option(options.get_or_insert_with(|| sub.options.to_vec()), option, slot);
            }
            HookOwner::Command => {}
        }
    }
    if let Some(options) = options {
        sub.options = Box::leak(options.into_boxed_slice());
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

// ---------------------------------------------------------------------------
// The per-thread host
// ---------------------------------------------------------------------------

/// The plan a thread should be serving, and a generation to notice a change
/// by. Generation `0` means "no pack hooks have ever been published", which is
/// the one state [`ensure_thread_host`] answers with a single relaxed load.
static GENERATION: AtomicU64 = AtomicU64::new(0);
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
    *published = Some(plan);
    GENERATION.fetch_add(1, Ordering::Release);
}

/// Give this thread a hook host for the published plan, if it does not have
/// one already.
///
/// Cheap enough for the top of a worker closure: one relaxed atomic load in a
/// process with no pack hooks, one thread-local read once it has them. The
/// build itself — one sandboxed VM per pack — happens on a thread's first call
/// after a publish, and again after a pack edit changes the plan.
pub fn ensure_thread_host() {
    let generation = GENERATION.load(Ordering::Acquire);
    if generation == 0 || INSTALLED.with(std::cell::Cell::get) == generation {
        return;
    }
    let plan = {
        let published = match PUBLISHED.lock() {
            Ok(published) => published,
            Err(poisoned) => poisoned.into_inner(),
        };
        published.clone()
    };
    let Some(plan) = plan else {
        return;
    };
    if plan.is_empty() {
        pack_hooks::clear_host();
    } else {
        let host = Rc::new(tclvm_host());
        for programs in plan.packs() {
            // Crash containment is the host's default: budgets on, quarantine
            // on first crash, abstention on anything unexpected.
            let _installed = host.load_pack(programs.clone());
        }
        pack_hooks::install_host(host);
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
        }])
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
}
