// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Worker-local evaluator ownership for selected application folds.
//!
//! An owned evaluator lives on its own worker thread, on a private registry
//! built by the real installer, and answers only requests its owner armed
//! from the compiler's selection. Every other dispatch on that thread is
//! refused without evaluation. The answers are detached strings: nothing is
//! executed or rewritten here, no Native scope or completion is granted, and
//! the counters describe this evaluator alone.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use tcl_compiler::const_subst::SelectedConstFold;
use tcl_compiler::optimiser::closed_consumer::{MIN_CLOSED_STACK_BYTES, select_application_folds};
use tcl_registry::pack_hooks::{self, HookAnswer, HookCall, HookFamily, HookSlot, PackHookHost};
use tcl_registry::value_transfer::{EvaluatorGeneration, ImplementationBudget};
use tcl_registry::{CommandRegistry, CommandSpec, InvocationDialect, RegistrySemanticKey};
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;
use tcl_spec_hooks::private_application::PrivateApplicationHost;
use tcl_spec_hooks::{HookOwner, HostConfig};

use super::HookPlan;
use crate::install::OwnedHookRegistry;
use crate::pack::PackSet;

/// The release an owned evaluator runs: its private engines are native cores.
pub(super) const OWNED_PROFILE: &str = "tcl9.1";

/// One selected command substitution the owned evaluator answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedApplicationFold {
    /// Byte range of the whole `[…]` in the source.
    pub span: std::ops::Range<usize>,
    /// The hook's answer, detached from every engine.
    pub value: String,
}

/// Observations of one owned evaluator only. They are not optimiser call
/// attempts and never add to another thread's engine metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OwnedEvaluatorTelemetry {
    /// Armed requests that reached the private host.
    pub invocations: u64,
    /// Of those, answers that folded.
    pub fold_answers: u64,
    /// Of those, answers that abstained.
    pub abstentions: u64,
    /// Dispatches on the worker that no request armed; none was evaluated.
    pub unrequested: u64,
    /// Hook errors the private host logged.
    pub errors: usize,
    /// Crashes the private host recorded.
    pub crashes: usize,
    /// Installed hooks the private host quarantined.
    pub quarantined: usize,
}

/// Everything one owned evaluation returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedApplicationFolds {
    /// Answered requests, in source order.
    pub folds: Vec<OwnedApplicationFold>,
    /// The owned evaluator's own counts.
    pub telemetry: OwnedEvaluatorTelemetry,
}

/// The installed program that answered one selected fold, as detached text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedFoldLineage {
    /// The pack that declared the hook.
    pub pack: String,
    /// The pack file that declared it.
    pub file: String,
    /// The canonical command the fold replaced.
    pub command: String,
    /// The registry slot the plan bound it to.
    pub slot: String,
}

/// Select every top-level fold request in `source` against the packs'
/// private `tcl9.1` registry, and answer each request that names one of the
/// packs' installed hooks through an owned evaluator on its own worker
/// thread. Requests for any other implementation are not evaluated.
///
/// # Errors
/// Refuses a worker that cannot be started or joined, an empty or
/// unavailable hook plan, a profile or private bootstrap that is unavailable,
/// unsupported source, and any drift of the evaluator's receiver, registry,
/// generation, host health or request state.
pub fn fold_owned_application_source(
    packs: &PackSet,
    source: String,
) -> Result<OwnedApplicationFolds, ClosedSourceRefusal> {
    let stack = usize::try_from(MIN_CLOSED_STACK_BYTES).map_err(|_| ClosedSourceRefusal::Budget)?;
    let packs = packs.clone();
    std::thread::Builder::new()
        .stack_size(stack)
        .name("owned-application-evaluator".into())
        .spawn(move || owned_worker(&packs, &source))
        .map_err(|_| ClosedSourceRefusal::Execution)?
        .join()
        .map_err(|_| ClosedSourceRefusal::Execution)?
}

fn owned_worker(
    packs: &PackSet,
    source: &str,
) -> Result<OwnedApplicationFolds, ClosedSourceRefusal> {
    let profile = tcl_registry::model::ingress::resolve_known_environment(OWNED_PROFILE)
        .and_then(|environment| environment.catalogue_profile())
        .ok_or(ClosedSourceRefusal::NativeContext)?;
    let installed = crate::install::bounded_owned_hook_registry(profile, packs)
        .ok_or(ClosedSourceRefusal::Budget)?;
    let owner = OwnedApplicationEvaluator::new(installed, HostConfig::default())?;
    let selected = select_application_folds(source, owner.registry(), profile)?;
    let mut folds = Vec::new();
    for fold in &selected {
        if let Some((value, _)) = owner.answer(fold.request())?
            && fold.admits_answer(&value)
        {
            folds.push(OwnedApplicationFold {
                span: fold.span(),
                value,
            });
        }
    }
    owner.check_current()?;
    let telemetry = owner.telemetry();
    drop(owner);
    Ok(OwnedApplicationFolds { folds, telemetry })
}

#[derive(Clone, Copy, Default)]
struct InvocationCounts {
    invocations: u64,
    fold_answers: u64,
    abstentions: u64,
    unrequested: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestPhase {
    Idle,
    Preparing,
    Armed,
    Running,
    Answered,
    Failed,
}

struct FoldRequest {
    slot: HookSlot,
    arguments: Vec<String>,
    profile: &'static str,
    version: Option<tcl_dialect::TclVersion>,
}

impl FoldRequest {
    fn matches(&self, slot: HookSlot, call: &HookCall<'_>) -> bool {
        slot == self.slot
            && call.dialect == Some(self.profile)
            && pack_hooks::current_dialect() == Some(self.profile)
            && call.version == self.version
            && !call.in_event_body
            && call.option.is_none()
            && call.constraints.is_none()
            && call.targets.is_empty()
            && call.budget == ImplementationBudget::default()
            && call.depends.is_empty()
            && call.words.len() == self.arguments.len()
            && call
                .words
                .iter()
                .zip(&self.arguments)
                .all(|(word, expected)| {
                    word.kind == tcl_registry::invocation_words::InvocationWordKind::Literal
                        && word.value == expected
                })
    }
}

/// The one-shot request state. Outside an armed request every dispatch is
/// refused without evaluation; a mismatched, replayed, reentrant or
/// abandoned request retires the evaluator for good.
struct RequestGate {
    phase: Cell<RequestPhase>,
    request: RefCell<Option<FoldRequest>>,
    receiver: RefCell<Option<Weak<dyn PackHookHost>>>,
    live: Cell<bool>,
}

impl RequestGate {
    fn new() -> Self {
        Self {
            phase: Cell::new(RequestPhase::Idle),
            request: RefCell::new(None),
            receiver: RefCell::new(None),
            live: Cell::new(true),
        }
    }

    fn refuse(&self) {
        self.live.set(false);
        self.phase.set(RequestPhase::Failed);
    }

    fn begin(&self) -> Result<ActiveCall<'_>, ClosedSourceRefusal> {
        if !self.live.get() || self.phase.get() != RequestPhase::Idle {
            self.refuse();
            return Err(ClosedSourceRefusal::Currency);
        }
        self.phase.set(RequestPhase::Preparing);
        Ok(ActiveCall {
            gate: self,
            completed: false,
        })
    }

    fn arm(&self, request: FoldRequest) -> Result<(), ClosedSourceRefusal> {
        if !self.live.get() || self.phase.get() != RequestPhase::Preparing {
            self.refuse();
            return Err(ClosedSourceRefusal::Currency);
        }
        let Ok(mut held) = self.request.try_borrow_mut() else {
            self.refuse();
            return Err(ClosedSourceRefusal::Currency);
        };
        if held.is_some() {
            self.refuse();
            return Err(ClosedSourceRefusal::Currency);
        }
        *held = Some(request);
        self.phase.set(RequestPhase::Armed);
        Ok(())
    }

    /// `Ok(true)` consumes the armed request for this exact call. `Ok(false)`
    /// is a dispatch no request armed. `Err` retires the evaluator.
    fn consume(&self, slot: HookSlot, call: &HookCall<'_>) -> Result<bool, ()> {
        if !self.live.get() {
            return Err(());
        }
        match self.phase.get() {
            RequestPhase::Idle => return Ok(false),
            RequestPhase::Armed => {}
            RequestPhase::Preparing
            | RequestPhase::Running
            | RequestPhase::Answered
            | RequestPhase::Failed => {
                self.refuse();
                return Err(());
            }
        }
        let current = self
            .receiver
            .try_borrow()
            .ok()
            .and_then(|held| held.as_ref().and_then(Weak::upgrade))
            .is_some_and(|receiver| pack_hooks::current_host_is(&receiver));
        let request = self
            .request
            .try_borrow_mut()
            .ok()
            .and_then(|mut held| held.take());
        if !current || !request.is_some_and(|request| request.matches(slot, call)) {
            self.refuse();
            return Err(());
        }
        self.phase.set(RequestPhase::Running);
        Ok(true)
    }

    fn answered(&self) -> bool {
        if !self.live.get() || self.phase.get() != RequestPhase::Running {
            self.refuse();
            return false;
        }
        self.phase.set(RequestPhase::Answered);
        true
    }
}

struct ActiveCall<'a> {
    gate: &'a RequestGate,
    completed: bool,
}

impl ActiveCall<'_> {
    fn complete(mut self) -> Result<(), ClosedSourceRefusal> {
        if !self.gate.live.get() || self.gate.phase.get() != RequestPhase::Answered {
            return Err(ClosedSourceRefusal::Currency);
        }
        self.gate.phase.set(RequestPhase::Idle);
        self.completed = true;
        Ok(())
    }
}

impl Drop for ActiveCall<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.gate.refuse();
        }
        if let Ok(mut request) = self.gate.request.try_borrow_mut() {
            request.take();
        }
    }
}

/// The thread's installed receiver. It never caches, and it reaches the
/// private host only for the armed request.
struct CountedEvaluator {
    actual: Rc<dyn PackHookHost>,
    counts: Cell<InvocationCounts>,
    gate: Rc<RequestGate>,
}

impl CountedEvaluator {
    fn count(&self, update: impl FnOnce(&mut InvocationCounts)) {
        let mut counts = self.counts.get();
        update(&mut counts);
        self.counts.set(counts);
    }
}

impl PackHookHost for CountedEvaluator {
    fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        match self.gate.consume(slot, call) {
            Ok(true) => {}
            Ok(false) => {
                self.count(|counts| counts.unrequested = counts.unrequested.saturating_add(1));
                return HookAnswer::Abstain;
            }
            Err(()) => return HookAnswer::Abstain,
        }
        self.count(|counts| counts.invocations = counts.invocations.saturating_add(1));
        // No RefCell or counter borrow spans the actual evaluation, and the
        // host converts its own failures to abstention.
        let answer = self.actual.invoke(slot, call);
        if !self.gate.answered() {
            return HookAnswer::Abstain;
        }
        match &answer {
            HookAnswer::Fold(_) => {
                self.count(|counts| counts.fold_answers = counts.fold_answers.saturating_add(1));
            }
            HookAnswer::Abstain => {
                self.count(|counts| counts.abstentions = counts.abstentions.saturating_add(1));
            }
            _ => {}
        }
        answer
    }

    fn requires_uncached_dispatch(&self) -> bool {
        true
    }

    fn is_available(&self, slot: HookSlot) -> bool {
        self.gate.live.get()
            && self.gate.phase.get() == RequestPhase::Armed
            && self.actual.is_available(slot)
    }
}

/// Owns the private registry, plan, host and receiver of one worker thread.
/// Construct it only inside that worker; no caller engine, host, callback
/// or evaluator is accepted.
pub(super) struct OwnedApplicationEvaluator {
    plan: Arc<HookPlan>,
    registry: Arc<CommandRegistry>,
    registry_key: RegistrySemanticKey,
    installed: Arc<OwnedHookRegistry>,
    host: PrivateApplicationHost,
    evaluator: Rc<CountedEvaluator>,
    receiver: Rc<dyn PackHookHost>,
    gate: Rc<RequestGate>,
    generation: EvaluatorGeneration,
}

impl OwnedApplicationEvaluator {
    pub(super) fn new(
        installed: Arc<OwnedHookRegistry>,
        config: HostConfig,
    ) -> Result<Self, ClosedSourceRefusal> {
        let plan = Arc::clone(installed.plan());
        let registry = Arc::clone(installed.registry());
        let profile = installed.profile();
        if profile.name != OWNED_PROFILE
            || InvocationDialect::of_profile(profile).tcl_version
                != Some(tcl_dialect::TclVersion::V9_1)
        {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        // Refuse existing state rather than clear it to manufacture a world.
        if pack_hooks::has_host() || plan.is_empty() {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let host = PrivateApplicationHost::new(config);
        let gate = Rc::new(RequestGate::new());
        let evaluator = Rc::new(CountedEvaluator {
            actual: host.evaluator(),
            counts: Cell::new(InvocationCounts::default()),
            gate: Rc::clone(&gate),
        });
        // The empty receiver is installed before any engine is built, so no
        // lazily installed thread host can answer while hooks compile.
        let receiver: Rc<dyn PackHookHost> = evaluator.clone();
        *gate.receiver.borrow_mut() = Some(Rc::downgrade(&receiver));
        pack_hooks::install_host(Rc::clone(&receiver));
        if !pack_hooks::current_host_is(&receiver) {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let generation = pack_hooks::evaluator_generation();
        if generation == EvaluatorGeneration::NO_HOST {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        // From here `Drop` retires the receiver on every early return.
        let owned = Self {
            plan,
            registry_key: registry.snapshot().semantic_key(),
            registry,
            installed,
            host,
            evaluator,
            receiver,
            gate,
            generation,
        };
        if !owned.host.prepare_initial_engines(owned.plan.packs().len()) {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        for programs in owned.plan.packs() {
            let installed = owned.host.install_pack_hooks(programs.clone());
            if installed.len() != programs.programs.len()
                || installed
                    .iter()
                    .zip(&programs.programs)
                    .any(|(got, wanted)| {
                        got.slot.is_none()
                            || got.declined.is_some()
                            || got.slot != wanted.slot
                            || got.command != wanted.command
                            || got.owner != wanted.owner
                            || got.family != wanted.family
                    })
                || pack_hooks::evaluator_generation() != owned.generation
            {
                return Err(ClosedSourceRefusal::NativeContext);
            }
        }
        if !owned.host.initial_engines_spent() {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        owned.check_current()?;
        Ok(owned)
    }

    pub(super) fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Answer one compiler-selected request. `Ok(None)` when the request
    /// names no hook these packs installed (it is not evaluated), or when the
    /// hook abstained.
    pub(super) fn answer(
        &self,
        request: &SelectedConstFold,
    ) -> Result<Option<(String, OwnedFoldLineage)>, ClosedSourceRefusal> {
        let profile = self.installed.profile();
        if request.dialect() != InvocationDialect::of_profile(profile) {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let query = request
            .dialect()
            .authoring_query()
            .ok_or(ClosedSourceRefusal::NativeContext)?;
        // The selection the value flow uses; the last registered spec of a
        // name is deliberately not enough.
        let spec = self
            .registry
            .get_for_surface(&request.facts().canonical_command, Some(query))
            .ok_or(ClosedSourceRefusal::Source)?;
        if !self.installed.installs(spec) {
            return Ok(None);
        }
        let member = if spec.subcommands.is_empty() {
            None
        } else {
            Some(
                request
                    .facts()
                    .subcommand
                    .canonical_name()
                    .ok_or(ClosedSourceRefusal::Source)?,
            )
        };
        let (ordinary, versioned) = if let Some(name) = member {
            let selected = spec.subcommand(name).ok_or(ClosedSourceRefusal::Source)?;
            (selected.const_fold, selected.const_fold_versioned)
        } else {
            (spec.const_fold, spec.const_fold_versioned)
        };
        if ordinary.is_none() && versioned.is_none() {
            return Ok(None);
        }
        let family = if versioned.is_some() {
            HookFamily::ConstFoldVersioned
        } else {
            HookFamily::ConstFold
        };
        let arguments = request.arguments().ok_or(ClosedSourceRefusal::Source)?;
        // Reentry is refused before any host availability is consulted.
        let call = self.gate.begin()?;
        self.check_current()?;
        let binding = self.selected_binding(spec, member, family, ordinary, versioned)?;
        let slot = binding.slot;
        let _dialect = pack_hooks::DialectScope::enter(Some(profile.name));
        self.gate.arm(FoldRequest {
            slot,
            arguments: arguments.iter().map(|value| (*value).to_owned()).collect(),
            profile: profile.name,
            version: (family == HookFamily::ConstFoldVersioned)
                .then_some(request.dialect().tcl_version)
                .flatten(),
        })?;
        let answer = if let Some(name) = member {
            spec.subcommand(name)
                .ok_or(ClosedSourceRefusal::Source)?
                .run_const_fold_in(&arguments, request.dialect())
        } else {
            spec.run_const_fold_in(&arguments, request.dialect())
        };
        self.check_current()?;
        if pack_hooks::current_dialect() != Some(profile.name) {
            return Err(ClosedSourceRefusal::Currency);
        }
        call.complete()?;
        Ok(answer.map(|value| {
            (
                value,
                OwnedFoldLineage {
                    pack: binding.pack.clone(),
                    file: binding.file.display().to_string(),
                    command: request.facts().canonical_command.clone(),
                    slot: format!("{slot:?}"),
                },
            )
        }))
    }

    /// The single plan binding whose installed allocation, declaration, file,
    /// pack, owner, family and thunk all join to the selected spec.
    fn selected_binding(
        &self,
        spec: &CommandSpec,
        member: Option<&str>,
        family: HookFamily,
        ordinary: Option<tcl_registry::hooks::ConstFoldFn>,
        versioned: Option<tcl_registry::hooks::VersionedConstFoldFn>,
    ) -> Result<&super::OriginalHookBinding, ClosedSourceRefusal> {
        let mut matches = self.plan.bindings.iter().filter(|binding| {
            let owner_matches = match (&binding.owner, member) {
                (HookOwner::Command, None) => true,
                (HookOwner::Subcommand(expected), Some(actual)) => expected == actual,
                _ => false,
            };
            let thunk_matches = match family {
                HookFamily::ConstFold => ordinary
                    .zip(pack_hooks::const_fold_fn(binding.slot))
                    .is_some_and(|(actual, expected)| std::ptr::fn_addr_eq(actual, expected)),
                HookFamily::ConstFoldVersioned => versioned
                    .zip(pack_hooks::const_fold_versioned_fn(binding.slot))
                    .is_some_and(|(actual, expected)| std::ptr::fn_addr_eq(actual, expected)),
                _ => false,
            };
            owner_matches
                && binding.family == family
                && thunk_matches
                && self.installed.owns_binding(
                    spec,
                    binding.declaration.0,
                    &binding.file,
                    &binding.pack,
                )
                && self.evaluator.actual.is_available(binding.slot)
        });
        let selected = matches.next().ok_or(ClosedSourceRefusal::Currency)?;
        if matches.next().is_some() {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(selected)
    }

    fn telemetry(&self) -> OwnedEvaluatorTelemetry {
        let counts = self.evaluator.counts.get();
        let mut slots = self
            .plan
            .bindings
            .iter()
            .map(|binding| binding.slot)
            .collect::<Vec<_>>();
        slots.sort_unstable();
        slots.dedup();
        OwnedEvaluatorTelemetry {
            invocations: counts.invocations,
            fold_answers: counts.fold_answers,
            abstentions: counts.abstentions,
            unrequested: counts.unrequested,
            errors: self.host.error_log().len(),
            crashes: self.host.crash_records().len(),
            quarantined: slots
                .into_iter()
                .filter(|slot| self.host.is_quarantined(*slot))
                .count(),
        }
    }

    /// The receiver, registry, generation, bindings and private hosts are all
    /// still exactly those this owner captured. Any drift retires it.
    pub(super) fn check_current(&self) -> Result<(), ClosedSourceRefusal> {
        let current = self.gate.live.get()
            && pack_hooks::current_host_is(&self.receiver)
            && self.registry_key == self.registry.snapshot().semantic_key()
            && pack_hooks::evaluator_generation() == self.generation
            && self
                .plan
                .bindings
                .iter()
                .all(|binding| self.evaluator.actual.is_available(binding.slot))
            && self.host.is_quiet();
        if !current {
            eprintln!(
                self.gate.live.get(),
                pack_hooks::current_host_is(&self.receiver),
                self.registry_key == self.registry.snapshot().semantic_key(),
                pack_hooks::evaluator_generation() == self.generation,
                self.plan
                    .bindings
                    .iter()
                    .all(|binding| self.evaluator.actual.is_available(binding.slot)),
                self.host.is_quiet()
            );
            self.gate.refuse();
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(())
    }
}

impl Drop for OwnedApplicationEvaluator {
    fn drop(&mut self) {
        self.gate.refuse();
        // Retire only this receiver; equal generations never justify
        // clearing a foreign replacement.
        let _ = pack_hooks::clear_current_host_if(&self.receiver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{Origin, PackFile, Tier};

    struct RecordingHost {
        calls: Cell<usize>,
        reenter: bool,
    }

    impl PackHookHost for RecordingHost {
        fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
            self.calls.set(self.calls.get() + 1);
            if self.reenter {
                assert!(!pack_hooks::slot_available(slot));
                assert!(matches!(
                    pack_hooks::dispatch(slot, call),
                    HookAnswer::Abstain
                ));
            }
            HookAnswer::Fold(call.words[0].value.to_owned())
        }
    }

    fn recording(reenter: bool) -> Rc<RecordingHost> {
        Rc::new(RecordingHost {
            calls: Cell::new(0),
            reenter,
        })
    }

    fn installed(
        actual: Rc<dyn PackHookHost>,
    ) -> (Rc<CountedEvaluator>, Rc<RequestGate>, Rc<dyn PackHookHost>) {
        let gate = Rc::new(RequestGate::new());
        let counted = Rc::new(CountedEvaluator {
            actual,
            counts: Cell::new(InvocationCounts::default()),
            gate: Rc::clone(&gate),
        });
        let receiver: Rc<dyn PackHookHost> = counted.clone();
        *gate.receiver.borrow_mut() = Some(Rc::downgrade(&receiver));
        pack_hooks::install_host(Rc::clone(&receiver));
        (counted, gate, receiver)
    }

    fn slot() -> HookSlot {
        pack_hooks::allocate(
            HookFamily::ConstFoldVersioned,
            &pack_hooks::HookInputs::parse(&["words"]),
        )
        .unwrap()
    }

    fn request(slot: HookSlot, value: &str) -> FoldRequest {
        FoldRequest {
            slot,
            arguments: vec![value.to_owned()],
            profile: OWNED_PROFILE,
            version: Some(tcl_dialect::TclVersion::V9_1),
        }
    }

    #[test]
    fn a_thunk_consumes_one_armed_request_and_unrequested_dispatch_is_not_evaluated() {
        std::thread::spawn(|| {
            let actual = recording(false);
            let (counted, gate, receiver) = installed(actual.clone());
            let slot = slot();
            let fold = pack_hooks::const_fold_versioned_fn(slot).unwrap();
            let _foreign = pack_hooks::DialectScope::enter(Some("tcl8.6"));
            for value in ["first", "second"] {
                let active = gate.begin().unwrap();
                {
                    let _profile = pack_hooks::DialectScope::enter(Some(OWNED_PROFILE));
                    gate.arm(request(slot, value)).unwrap();
                    assert_eq!(
                        fold(&[value], Some(tcl_dialect::TclVersion::V9_1)),
                        Some(value.into())
                    );
                    active.complete().unwrap();
                }
                assert_eq!(pack_hooks::current_dialect(), Some("tcl8.6"));
            }
            assert_eq!(actual.calls.get(), 2);
            assert_eq!(counted.counts.get().invocations, 2);
            assert_eq!(pack_hooks::cache_stats().entries, 0);
            assert!(!pack_hooks::slot_available(slot));
            assert_eq!(fold(&["second"], Some(tcl_dialect::TclVersion::V9_1)), None);
            assert_eq!(actual.calls.get(), 2);
            assert_eq!(counted.counts.get().unrequested, 1);
            assert!(gate.live.get());
            assert!(pack_hooks::clear_current_host_if(&receiver));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn dispatch_refuses_foreign_request_axes_before_calling_the_evaluator() {
        std::thread::spawn(|| {
            for wrong in 0..5 {
                let actual = recording(false);
                let (_, gate, receiver) = installed(actual.clone());
                let selected = slot();
                let invoked = if wrong == 0 { slot() } else { selected };
                let active = gate.begin().unwrap();
                gate.arm(request(selected, "owned")).unwrap();
                let _profile = pack_hooks::DialectScope::enter(match wrong {
                    1 => None,
                    4 => Some("tcl8.6"),
                    _ => Some(OWNED_PROFILE),
                });
                let fold = pack_hooks::const_fold_versioned_fn(invoked).unwrap();
                assert_eq!(
                    fold(
                        &[if wrong == 2 { "foreign" } else { "owned" }],
                        Some(if wrong == 3 {
                            tcl_dialect::TclVersion::V9_0
                        } else {
                            tcl_dialect::TclVersion::V9_1
                        }),
                    ),
                    None
                );
                assert!(active.complete().is_err());
                assert_eq!(actual.calls.get(), 0);
                assert!(gate.begin().is_err());
                assert!(pack_hooks::clear_current_host_if(&receiver));
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn nested_dispatch_inside_a_running_request_retires_the_evaluator() {
        std::thread::spawn(|| {
            let actual = recording(true);
            let (_, gate, receiver) = installed(actual.clone());
            let slot = slot();
            let active = gate.begin().unwrap();
            let _profile = pack_hooks::DialectScope::enter(Some(OWNED_PROFILE));
            gate.arm(request(slot, "owned")).unwrap();
            assert_eq!(
                pack_hooks::const_fold_versioned_fn(slot).unwrap()(
                    &["owned"],
                    Some(tcl_dialect::TclVersion::V9_1)
                ),
                None
            );
            assert!(active.complete().is_err());
            assert_eq!(actual.calls.get(), 1);
            assert!(!gate.live.get());
            assert!(pack_hooks::clear_current_host_if(&receiver));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn a_superseded_receiver_or_abandoned_request_cannot_revive() {
        std::thread::spawn(|| {
            let actual = recording(false);
            let (counted, gate, receiver) = installed(actual.clone());
            let slot = slot();
            let active = gate.begin().unwrap();
            gate.arm(request(slot, "owned")).unwrap();
            pack_hooks::install_host(actual.clone());
            let words = [pack_hooks::HookWord {
                value: "owned",
                kind: tcl_registry::invocation_words::InvocationWordKind::Literal,
            }];
            let call = HookCall {
                words: &words,
                version: Some(tcl_dialect::TclVersion::V9_1),
                in_event_body: false,
                option: None,
                constraints: None,
                dialect: Some(OWNED_PROFILE),
                targets: &[],
                budget: ImplementationBudget::default(),
                depends: &[],
            };
            assert!(matches!(counted.invoke(slot, &call), HookAnswer::Abstain));
            assert!(active.complete().is_err());
            assert_eq!(actual.calls.get(), 0);
            assert!(!pack_hooks::clear_current_host_if(&receiver));
            pack_hooks::clear_host();
            let (_, gate, receiver) = installed(actual);
            let active = gate.begin().unwrap();
            gate.arm(request(slot, "owned")).unwrap();
            drop(active);
            assert!(gate.begin().is_err());
            assert!(pack_hooks::clear_current_host_if(&receiver));
        })
        .join()
        .unwrap();
    }

    fn pack_set(name: &str, source: &str) -> PackSet {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-owned-evaluator-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("owner.tclspec");
        std::fs::write(&path, source).unwrap();
        let packs = crate::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
            dependency_tier: None,
        }]);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(packs.notices.is_empty(), "{:?}", packs.notices);
        packs
    }

    fn hook_pack(name: &str, body: &str) -> PackSet {
        pack_set(
            name,
            &format!(
                "speclib owner 2.0 {{\n    command owner::f {{\n        arity 1\n        \
                 arg 0 -role Value\n        const_fold -inputs {{words}} {{words ctx}} {{\n{body}\n        }}\n    }}\n}}\n"
            ),
        )
    }

    #[test]
    fn an_owned_evaluator_answers_only_the_selected_pack_hook_request() {
        let packs = hook_pack("length", "fold [string length [lindex $words 0]]");
        let source = "puts [owner::f abcde]\nputs [string length abc]\n";
        let answered =
            fold_owned_application_source(&packs, source.to_owned()).expect("owned evaluation");
        let [fold] = answered.folds.as_slice() else {
            panic!("one owned fold: {answered:?}");
        };
        assert_eq!(&source[fold.span.clone()], "[owner::f abcde]");
        assert_eq!(fold.value, "5");
        assert_eq!(answered.telemetry.invocations, 1);
        assert_eq!(answered.telemetry.fold_answers, 1);
        assert_eq!(answered.telemetry.errors, 0);
        assert_eq!(answered.telemetry.crashes, 0);
        assert_eq!(answered.telemetry.quarantined, 0);
        assert!(!pack_hooks::has_host(), "the caller's thread is untouched");
    }

    /// N17: `$arr($a:b)` reads element `x:b` of `arr` in C Tcl, so the
    /// hook's own substitution answers `good`.
    #[test]
    fn an_owned_hook_body_substitutes_a_colon_suffixed_element_as_c_tcl_does() {
        let packs = hook_pack(
            "colon",
            "set a x\nset a:b BAD\nset arr(x:b) good\nset arr(BAD) bad\nfold $arr($a:b)",
        );
        let answered = fold_owned_application_source(&packs, "puts [owner::f abcde]\n".to_owned())
            .expect("owned evaluation");
        assert_eq!(
            answered
                .folds
                .iter()
                .map(|fold| fold.value.as_str())
                .collect::<Vec<_>>(),
            ["good"]
        );
    }

    #[test]
    fn an_owned_evaluator_refuses_a_thread_that_already_has_a_host() {
        let packs = hook_pack("occupied", "fold 1");
        std::thread::spawn(move || {
            let foreign = recording(false);
            pack_hooks::install_host(foreign.clone());
            assert_eq!(
                owned_worker(&packs, "puts [owner::f abcde]\n").unwrap_err(),
                ClosedSourceRefusal::NativeContext
            );
            assert_eq!(foreign.calls.get(), 0);
            pack_hooks::clear_host();
        })
        .join()
        .unwrap();
    }

    #[test]
    fn procedure_source_is_refused_before_any_hook_runs() {
        let packs = hook_pack("procedure", "fold 1");
        assert_eq!(
            fold_owned_application_source(
                &packs,
                "proc p {} {owner::f abcde}\nputs [p]\n".to_owned()
            )
            .unwrap_err(),
            ClosedSourceRefusal::Source
        );
    }
}
