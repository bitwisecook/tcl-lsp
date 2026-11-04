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

//! **One warm hook VM per process.**
//!
//! A hook body runs on a `tcl-vm` engine, which is not `Send`, so the host
//! that owns the engines lives on one dedicated thread on native targets.
//! Browser and WASI hosts dispatch synchronously on their one execution thread.
//! It is built once per published plan — every pack's sandbox bootstrapped and every body bound
//! before the first call — and serves every worker thread. A worker installs
//! a [`Proxy`], which costs nothing to build, so no analysis thread ever pays
//! a VM bootstrap, however often the pool turns its threads over. A WASM
//! host builds its VM at the first invocation outside a precompiled table.
//!
//! Per-pack isolation is unchanged: each pack still runs in its own sandboxed
//! engine inside that host, with its own budgets and quarantine.
//!
//! At install the service also **tabulates** each resolver whose body reads
//! its call only through `[llength $words]`: the answer for every word count
//! up to [`TABULATED_WORDS`] is computed once and published to the registry
//! (`pack_hooks::tabulate`), which then answers those calls without a host.

use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

use tcl_registry::InvocationWordKind;
use tcl_registry::pack_hooks::{
    HookAnswer, HookCall, HookFamily, HookSlot, HookWord, PackHookHost,
};
use tcl_registry::value_transfer::ImplementationBudget;

use super::HookPlan;

/// The largest word count a tabulated resolver is answered for.
pub(crate) const TABULATED_WORDS: usize = 32;

/// Slots whose hook the service quarantined, process-wide.
struct Quarantine {
    generation: u64,
    slots: BTreeSet<HookSlot>,
}

static QUARANTINED: Mutex<Quarantine> = Mutex::new(Quarantine {
    generation: 0,
    slots: BTreeSet::new(),
});

fn next_generation() -> u64 {
    let mut quarantine = QUARANTINED.lock().unwrap_or_else(PoisonError::into_inner);
    quarantine.generation = quarantine.generation.wrapping_add(1);
    quarantine.slots.clear();
    quarantine.generation
}

#[cfg(not(target_arch = "wasm32"))]
mod transport {
    use super::{
        Arc, HookAnswer, HookCall, HookPlan, HookSlot, HookWord, HostState, ImplementationBudget,
        InvocationWordKind, Mutex, PoisonError, next_generation,
    };
    use std::sync::mpsc;
    use tcl_registry::pack_hooks::{ConstraintCallCtx, OptionCallCtx};
    use tcl_registry::value_transfer::ContextDependency;
    /// A constraint context, owned: its options, its positionals, and whether
    /// the invocation was read to its end.
    type OwnedConstraints = (
        Vec<(&'static str, Option<String>)>,
        Vec<Option<String>>,
        bool,
    );

    /// A hook call with every borrowed field owned, to cross to the service.
    struct OwnedCall {
        words: Vec<(String, InvocationWordKind)>,
        version: Option<tcl_dialect::TclVersion>,
        in_event_body: bool,
        option: Option<(String, usize, usize)>,
        constraints: Option<OwnedConstraints>,
        dialect: Option<&'static str>,
        targets: Vec<usize>,
        budget: ImplementationBudget,
        depends: Vec<ContextDependency>,
    }

    impl OwnedCall {
        fn of(call: &HookCall<'_>) -> Self {
            Self {
                words: call
                    .words
                    .iter()
                    .map(|word| (word.value.to_owned(), word.kind))
                    .collect(),
                version: call.version,
                in_event_body: call.in_event_body,
                option: call
                    .option
                    .map(|option| (option.option.to_owned(), option.index, option.value_start)),
                constraints: call.constraints.map(|constraints| {
                    (
                        constraints
                            .options
                            .iter()
                            .map(|&(name, value)| (name, value.map(str::to_owned)))
                            .collect(),
                        constraints
                            .positionals
                            .iter()
                            .map(|value| value.map(str::to_owned))
                            .collect(),
                        constraints.complete,
                    )
                }),
                dialect: call.dialect,
                targets: call.targets.to_vec(),
                budget: call.budget,
                depends: call.depends.to_vec(),
            }
        }

        fn with<R>(&self, run: impl FnOnce(&HookCall<'_>) -> R) -> R {
            let words: Vec<HookWord<'_>> = self
                .words
                .iter()
                .map(|(value, kind)| HookWord { value, kind: *kind })
                .collect();
            let options: Vec<(&'static str, Option<&str>)> = self
                .constraints
                .iter()
                .flat_map(|(options, _, _)| {
                    options
                        .iter()
                        .map(|(name, value)| (*name, value.as_deref()))
                })
                .collect();
            let positionals: Vec<Option<&str>> = self
                .constraints
                .iter()
                .flat_map(|(_, positionals, _)| positionals.iter().map(Option::as_deref))
                .collect();
            let call =
                HookCall {
                    words: &words,
                    version: self.version,
                    in_event_body: self.in_event_body,
                    option: self.option.as_ref().map(|(option, index, value_start)| {
                        OptionCallCtx {
                            option,
                            index: *index,
                            value_start: *value_start,
                        }
                    }),
                    constraints: self.constraints.as_ref().map(|(_, _, complete)| {
                        ConstraintCallCtx {
                            options: &options,
                            positionals: &positionals,
                            complete: *complete,
                        }
                    }),
                    dialect: self.dialect,
                    targets: &self.targets,
                    budget: self.budget,
                    depends: &self.depends,
                };
            run(&call)
        }
    }

    enum Message {
        /// Serve a plan. `eager` builds its host now; a plan whose every body is
        /// already tabulated builds it only if a call falls outside a table.
        Install {
            plan: Arc<HookPlan>,
            eager: bool,
            generation: u64,
        },
        Invoke {
            slot: HookSlot,
            call: Box<OwnedCall>,
            reply: mpsc::SyncSender<HookAnswer>,
        },
    }

    /// The service's inbox, and the plan it serves — kept so a service that had
    /// to be restarted comes back serving the same plan.
    struct Service {
        sender: Option<mpsc::Sender<Message>>,
        plan: Option<(Arc<HookPlan>, bool, u64)>,
    }

    static SERVICE: Mutex<Service> = Mutex::new(Service {
        sender: None,
        plan: None,
    });

    fn spawn() -> mpsc::Sender<Message> {
        let (sender, inbox) = mpsc::channel();
        std::thread::Builder::new()
            .name("spectcl-hooks".to_owned())
            // A hook body is bounded by its budgets, not by this stack; the
            // headroom is for the VM's own recursion during a deep body.
            .stack_size(32 * 1024 * 1024)
            .spawn(move || serve(&inbox))
            .expect("spawn the SpecTcl hook service");
        sender
    }

    /// Send `message`, starting (or restarting) the service as needed.
    fn send(message: Message) -> bool {
        let mut service = SERVICE.lock().unwrap_or_else(PoisonError::into_inner);
        let sender = if let Some(sender) = &service.sender {
            sender.clone()
        } else {
            let sender = spawn();
            if let Some((plan, eager, generation)) = &service.plan {
                let _ = sender.send(Message::Install {
                    plan: Arc::clone(plan),
                    eager: *eager,
                    generation: *generation,
                });
            }
            service.sender = Some(sender.clone());
            sender
        };
        if let Err(mpsc::SendError(message)) = sender.send(message) {
            // The service died; the next send starts a fresh one on the same plan.
            service.sender = None;
            drop(service);
            if let Message::Invoke { .. } = message {
                return false;
            }
        }
        true
    }

    pub(super) fn install(plan: Arc<HookPlan>, eager: bool) {
        let generation = next_generation();
        SERVICE.lock().unwrap_or_else(PoisonError::into_inner).plan =
            Some((Arc::clone(&plan), eager, generation));
        let _ = send(Message::Install {
            plan,
            eager,
            generation,
        });
    }

    pub(super) fn invoke(slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        let (reply, answer) = mpsc::sync_channel(1);
        let message = Message::Invoke {
            slot,
            call: Box::new(OwnedCall::of(call)),
            reply,
        };
        if !send(message) {
            return HookAnswer::Abstain;
        }
        answer.recv().unwrap_or(HookAnswer::Abstain)
    }

    fn serve(inbox: &mpsc::Receiver<Message>) {
        let mut state = HostState::default();
        while let Ok(message) = inbox.recv() {
            match message {
                Message::Install {
                    plan,
                    eager,
                    generation,
                } => {
                    state = HostState::new(plan, eager, generation);
                    if eager {
                        state.prepare();
                    }
                }
                Message::Invoke { slot, call, reply } => {
                    let answer = call.with(|call| state.invoke(slot, call));
                    let _ = reply.send(answer);
                }
            }
        }
    }
}

/// Serve `plan` from now on: build its host on the native service thread, bind
/// every body, and tabulate what can be. `precompiled` are the slots already
/// answered from a shipped table; when they are every body the plan has, the
/// host is built only if a call falls outside a table. WASM defers all host
/// construction until invocation, after publication releases its plan lock.
pub(crate) fn install(plan: Arc<HookPlan>, precompiled: &BTreeSet<HookSlot>) {
    let eager = plan.packs().iter().any(|programs| {
        programs
            .programs
            .iter()
            .any(|program| program.slot.is_none_or(|slot| !precompiled.contains(&slot)))
    });
    transport::install(plan, eager);
}

/// A worker thread's host: every call goes to the service.
pub(crate) struct Proxy;

impl PackHookHost for Proxy {
    fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        transport::invoke(slot, call)
    }

    #[cfg(target_arch = "wasm32")]
    fn requires_uncached_dispatch(&self) -> bool {
        transport::is_active()
    }

    fn is_available(&self, slot: HookSlot) -> bool {
        !QUARANTINED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .slots
            .contains(&slot)
    }
}

/// How many hosts this process has built for the service or for
/// precomputation — each one a VM bootstrap per pack.
pub(crate) static HOSTS_BUILT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The host for `plan`: one sandboxed engine per pack, every body bound.
pub(crate) fn build_host(plan: &HookPlan) -> Rc<dyn PackHookHost> {
    HOSTS_BUILT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let built = tcl_spec_hooks::tclvm_host();
    for programs in plan.packs() {
        let _installed = built.install_pack_hooks(programs.clone());
    }
    Rc::new(built)
}

/// The same host lifecycle for the native actor and synchronous WASM host.
#[derive(Default)]
struct HostState {
    generation: u64,
    host: Option<Rc<dyn PackHookHost>>,
    deferred: Option<(Arc<HookPlan>, bool)>,
}

impl HostState {
    fn new(plan: Arc<HookPlan>, eager: bool, generation: u64) -> Self {
        Self {
            generation,
            host: None,
            deferred: (!plan.is_empty()).then_some((plan, eager)),
        }
    }

    fn prepare(&mut self) {
        if let Some((plan, tabulate_now)) = self.deferred.take() {
            let host = build_host(&plan);
            if tabulate_now {
                tabulate(host.as_ref(), &plan);
            }
            self.host = Some(host);
        }
    }

    fn invoke(&mut self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        self.prepare();
        let Some(host) = &self.host else {
            return HookAnswer::Abstain;
        };
        let answer = host.invoke(slot, call);
        if !host.is_available(slot) {
            let mut quarantine = QUARANTINED.lock().unwrap_or_else(PoisonError::into_inner);
            if quarantine.generation == self.generation {
                quarantine.slots.insert(slot);
            }
        }
        answer
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
mod inline {
    use super::{Arc, HookAnswer, HookCall, HookPlan, HookSlot, HostState, next_generation};
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub(super) struct Service {
        state: RefCell<Option<HostState>>,
        active: Cell<bool>,
    }

    struct Lease<'a> {
        service: &'a Service,
        state: Option<HostState>,
    }

    impl Drop for Lease<'_> {
        fn drop(&mut self) {
            // A reentrant publication, if any, wins over the returning plan.
            let mut stored = self.service.state.borrow_mut();
            if stored.is_none() {
                *stored = self.state.take();
            }
            self.service.active.set(false);
        }
    }

    impl Service {
        pub(super) fn install(&self, plan: Arc<HookPlan>, eager: bool) {
            // Defer bootstrap until invocation: publication still holds its
            // plan mutex, and VM compilation may itself query the registry.
            *self.state.borrow_mut() = Some(HostState::new(plan, eager, next_generation()));
        }

        pub(super) fn is_active(&self) -> bool {
            self.active.get()
        }

        pub(super) fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
            if self.active.replace(true) {
                return HookAnswer::Abstain;
            }
            let mut lease = Lease {
                service: self,
                state: Some(self.state.take().unwrap_or_default()),
            };
            // No TLS borrow crosses VM construction or execution. A nested
            // dispatch cannot reborrow the live VM or cache its temporary silence.
            lease
                .state
                .as_mut()
                .expect("leased state")
                .invoke(slot, call)
        }
    }
    #[cfg(test)]
    mod tests {
        use super::super::{HookPlan, HostState, build_host};
        use super::Service;
        use std::rc::Rc;
        use std::sync::Arc;
        use tcl_registry::InvocationWordKind;
        use tcl_registry::pack_hooks::{
            self, ConstraintCallCtx, HookAnswer, HookCall, HookFamily, HookInputs, HookSlot,
            HookWord, OptionCallCtx, PackHookHost,
        };
        use tcl_registry::value_transfer::ImplementationBudget;
        use tcl_spec_hooks::{HookProgram, PackPrograms};

        fn plan(mut programs: Vec<HookProgram>) -> Arc<HookPlan> {
            for program in &mut programs {
                program.slot =
                    Some(pack_hooks::allocate(program.family, &program.inputs).expect("test slot"));
            }
            let mut pack = PackPrograms::new("inline-control");
            pack.programs = programs;
            Arc::new(HookPlan {
                packs: vec![pack],
                ..HookPlan::default()
            })
        }

        fn call<'a>(words: &'a [HookWord<'a>]) -> HookCall<'a> {
            HookCall {
                words,
                version: Some(tcl_dialect::TclVersion::V9_0),
                in_event_body: true,
                option: None,
                constraints: None,
                dialect: Some("tcl9.0"),
                targets: &[],
                budget: ImplementationBudget::default(),
                depends: &[],
            }
        }

        fn family_programs() -> Vec<HookProgram> {
            let fixtures = [
                (
                    HookFamily::ArgRoleResolver,
                    "if {[lindex $words 0] eq {x}} {role 0 Body}",
                ),
                (HookFamily::CommandPrefixResolver, "prefix 0 {Exactly 2}"),
                (HookFamily::ScriptTimingResolver, "timing 0 Deferred"),
                (HookFamily::ConstFold, "fold [lindex $words 0]"),
                (
                    HookFamily::ConstFoldVersioned,
                    "fold [dict get $ctx tcl-version]",
                ),
                (HookFamily::TaintSinkGate, "sink-suppressed"),
                (HookFamily::ContextGate, "reject [dict get $ctx dialect]"),
                (
                    HookFamily::LiteralArgumentValidator,
                    "invalid -index 0 -subject value -reason invalid-members -members x -allowed y",
                ),
                (HookFamily::ClauseShapeCheck, "missing-body 0"),
                (
                    HookFamily::OptionArity,
                    "consume [dict get $ctx option-value-start]",
                ),
                (
                    HookFamily::Constraints,
                    "if {[option-value -min] > [option-value -max]} {invalid -min bounds}",
                ),
                (HookFamily::Evaluate, "write 1 $s; preserve 2; fold $s"),
                (HookFamily::StateTransitionResolver, "namespace-variable 0"),
            ];
            assert_eq!(
                fixtures.map(|(family, _)| family),
                pack_hooks::HOOK_FAMILIES
            );
            fixtures
                .iter()
                .map(|(family, body)| {
                    let mut program = HookProgram::new("inline::control", *family, *body);
                    if *family == HookFamily::Constraints {
                        program.inputs = HookInputs::parse(&["invocation"]);
                    } else if *family == HookFamily::Evaluate {
                        program.parameters = vec!["s".to_owned()];
                        program.inputs = HookInputs::parse(&["words"]);
                    }
                    program
                })
                .collect()
        }

        #[test]
        fn inline_service_runs_every_hook_family_on_the_actual_vm() {
            let _guard = crate::cache::REDIRECT_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let plan = plan(family_programs());
            let direct = build_host(&plan);
            let service = Service::default();
            service.install(Arc::clone(&plan), false);
            assert!(service.state.borrow().as_ref().unwrap().host.is_none());
            let words = [HookWord {
                value: "x",
                kind: InvocationWordKind::Literal,
            }];
            let options = [("-min", Some("9")), ("-max", Some("2"))];
            for program in &plan.packs()[0].programs {
                let slot = program.slot.unwrap();
                let mut invocation = call(&words);
                if program.family == HookFamily::OptionArity {
                    invocation.option = Some(OptionCallCtx {
                        option: "-value",
                        index: 0,
                        value_start: 1,
                    });
                } else if program.family == HookFamily::Constraints {
                    invocation.constraints = Some(ConstraintCallCtx {
                        options: &options,
                        positionals: &[Some("body")],
                        complete: true,
                    });
                } else if program.family == HookFamily::Evaluate {
                    invocation.targets = &[1, 2];
                }
                let expected = direct.invoke(slot, &invocation);
                assert_ne!(expected, HookAnswer::Abstain, "{:?}", program.family);
                assert_eq!(
                    service.invoke(slot, &invocation),
                    expected,
                    "{:?}",
                    program.family
                );
                assert_eq!(
                    service.invoke(slot, &invocation),
                    expected,
                    "warm {:?}",
                    program.family
                );
            }
            let first_host = Rc::clone(
                service
                    .state
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .host
                    .as_ref()
                    .unwrap(),
            );
            let fold = plan.packs()[0]
                .programs
                .iter()
                .find(|p| p.family == HookFamily::ConstFold)
                .unwrap()
                .slot
                .unwrap();
            let changed = [HookWord {
                value: "different",
                kind: InvocationWordKind::Literal,
            }];
            assert_eq!(
                service.invoke(fold, &call(&changed)),
                HookAnswer::Fold("different".to_owned())
            );
            assert!(Rc::ptr_eq(
                &first_host,
                service
                    .state
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .host
                    .as_ref()
                    .unwrap()
            ));
            service.install(Arc::new(HookPlan::default()), false);
            assert_eq!(service.invoke(fold, &call(&words)), HookAnswer::Abstain);
            service.install(Arc::clone(&plan), false);
            assert_eq!(
                service.invoke(fold, &call(&words)),
                HookAnswer::Fold("x".to_owned())
            );
        }

        struct Reentrant {
            service: Rc<Service>,
        }

        impl PackHookHost for Reentrant {
            fn is_available(&self, _slot: HookSlot) -> bool {
                false
            }

            fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
                assert!(self.service.is_active());
                assert_eq!(self.service.invoke(slot, call), HookAnswer::Abstain);
                // Publication while the old receiver is leased is retained.
                self.service.install(Arc::new(HookPlan::default()), false);
                assert_eq!(self.service.invoke(slot, call), HookAnswer::Abstain);
                HookAnswer::Fold("outer".to_owned())
            }
        }

        #[test]
        fn inline_service_does_not_reborrow_an_active_host_or_restore_a_retired_plan() {
            let _guard = crate::cache::REDIRECT_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let service = Rc::new(Service::default());
            let generation = super::super::next_generation();
            let slot =
                pack_hooks::allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
            *service.state.borrow_mut() = Some(HostState {
                host: Some(Rc::new(Reentrant {
                    service: Rc::clone(&service),
                })),
                deferred: None,
                generation,
            });
            assert_eq!(
                service.invoke(slot, &call(&[])),
                HookAnswer::Fold("outer".to_owned())
            );
            assert!(!service.is_active());
            assert!(
                !super::super::QUARANTINED
                    .lock()
                    .unwrap()
                    .slots
                    .contains(&slot)
            );
            assert_eq!(service.invoke(slot, &call(&[])), HookAnswer::Abstain);
        }

        #[test]
        fn inline_service_contains_a_runaway_and_retains_a_healthy_hook() {
            let _guard = crate::cache::REDIRECT_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let plan = plan(vec![
                HookProgram::new("inline::spin", HookFamily::ArgRoleResolver, "while 1 {}"),
                HookProgram::new(
                    "inline::healthy",
                    HookFamily::ConstFold,
                    "fold [lindex $words 0]",
                ),
            ]);
            let spin = plan.packs()[0].programs[0].slot.unwrap();
            let healthy = plan.packs()[0].programs[1].slot.unwrap();
            let service = Service::default();
            service.install(plan, false);
            let words = [HookWord {
                value: "still-live",
                kind: InvocationWordKind::Literal,
            }];
            assert_eq!(service.invoke(spin, &call(&words)), HookAnswer::Abstain);
            assert!(
                !service
                    .state
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .host
                    .as_ref()
                    .unwrap()
                    .is_available(spin)
            );
            assert!(
                super::super::QUARANTINED
                    .lock()
                    .unwrap()
                    .slots
                    .contains(&spin)
            );
            assert_eq!(service.invoke(spin, &call(&words)), HookAnswer::Abstain);
            assert_eq!(
                service.invoke(healthy, &call(&words)),
                HookAnswer::Fold("still-live".to_owned())
            );
            let dynamic = [HookWord {
                value: "still-live",
                kind: InvocationWordKind::Dynamic,
            }];
            assert_eq!(
                service.invoke(healthy, &call(&dynamic)),
                HookAnswer::Abstain
            );
            assert_eq!(
                service.invoke(healthy, &call(&words)),
                HookAnswer::Fold("still-live".to_owned())
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod transport {
    use super::{Arc, HookAnswer, HookCall, HookPlan, HookSlot, inline::Service};
    thread_local! { static SERVICE: Service = Service::default(); }

    pub(super) fn install(plan: Arc<HookPlan>, eager: bool) {
        SERVICE.with(|service| service.install(plan, eager));
    }

    pub(super) fn invoke(slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        SERVICE.with(|service| service.invoke(slot, call))
    }

    pub(super) fn is_active() -> bool {
        SERVICE.with(Service::is_active)
    }
}

/// The families whose answer is a function of the call's words alone, with
/// no option or constraint context.
const WORD_FAMILIES: &[HookFamily] = &[
    HookFamily::ArgRoleResolver,
    HookFamily::CommandPrefixResolver,
    HookFamily::ScriptTimingResolver,
];

/// Whether `body`, with `parameters`, reads its call only through the count
/// of its words: every mention of the words parameter is `[llength $words]`,
/// and the context parameter is never mentioned.
pub(crate) fn reads_only_word_count(parameters: &[String], body: &str) -> bool {
    let Some(words) = parameters.first() else {
        return true;
    };
    let mut rest = body.to_owned();
    for idiom in [
        format!("[llength ${words}]"),
        format!("[llength ${{{words}}}]"),
    ] {
        rest = rest.replace(&idiom, "");
    }
    let mentions = |name: &str| {
        rest.match_indices(name).any(|(at, _)| {
            let before = rest[..at].chars().next_back();
            let after = rest[at + name.len()..].chars().next();
            !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':')
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    };
    !mentions(words) && parameters.get(1).is_none_or(|ctx| !mentions(ctx))
}

/// Tabulate every count-only resolver of `plan` on `host` ([`table_for`]).
fn tabulate(host: &dyn PackHookHost, plan: &HookPlan) {
    for programs in plan.packs() {
        for program in &programs.programs {
            if let Some(slot) = program.slot
                && is_tabulable(program)
                && let Some(answers) = table_for(host, slot)
            {
                tcl_registry::pack_hooks::tabulate(slot, answers);
            }
        }
    }
}

/// The identity a precompiled table is keyed by: `xxh3` of the parameter
/// list and the body. A body is a pure function in a deterministic sandbox,
/// so its answers follow from its text alone.
#[must_use]
pub fn body_hash(program: &tcl_spec_hooks::HookProgram) -> u64 {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    for parameter in &program.parameters {
        hasher.update(parameter.as_bytes());
        hasher.update(&[0]);
    }
    hasher.update(&[1]);
    hasher.update(program.body.as_bytes());
    hasher.digest()
}

/// Whether `program` is a family answered from its words alone and its body
/// reads them only through their count.
pub(crate) fn is_tabulable(program: &tcl_spec_hooks::HookProgram) -> bool {
    WORD_FAMILIES.contains(&program.family)
        && reads_only_word_count(&program.parameters, &program.body)
}

/// `slot`'s answer at every word count up to [`TABULATED_WORDS`], when three
/// probes — empty literal words with no dialect, distinct literal words under
/// Tcl 9.0, and computed words under iRules — agree at every count.
pub(crate) fn table_for(host: &dyn PackHookHost, slot: HookSlot) -> Option<Vec<HookAnswer>> {
    let mut answers = Vec::with_capacity(TABULATED_WORDS + 1);
    for count in 0..=TABULATED_WORDS {
        let probe = |value: &dyn Fn(usize) -> String,
                     kind: InvocationWordKind,
                     dialect: Option<&'static str>| {
            let values: Vec<String> = (0..count).map(value).collect();
            let words: Vec<HookWord<'_>> = values
                .iter()
                .map(|value| HookWord { value, kind })
                .collect();
            host.invoke(
                slot,
                &HookCall {
                    words: &words,
                    version: None,
                    in_event_body: false,
                    option: None,
                    constraints: None,
                    dialect,
                    targets: &[],
                    budget: ImplementationBudget::default(),
                    depends: &[],
                },
            )
        };
        let first = probe(&|_| String::new(), InvocationWordKind::Literal, None);
        let second = probe(
            &|index| format!("-w{index}"),
            InvocationWordKind::Literal,
            Some("tcl9.0"),
        );
        let third = probe(
            &|_| String::new(),
            InvocationWordKind::Dynamic,
            Some("f5-irules"),
        );
        if first != second || second != third {
            return None;
        }
        answers.push(first);
    }
    host.is_available(slot).then_some(answers)
}

#[cfg(test)]
mod tests {
    use super::reads_only_word_count;

    fn params() -> Vec<String> {
        vec!["words".to_owned(), "ctx".to_owned()]
    }

    #[test]
    fn a_body_reading_only_the_word_count_is_recognised() {
        assert!(reads_only_word_count(
            &params(),
            "set n [llength $words]\nif {$n >= 2} { role [expr {$n - 1}] Body }"
        ));
        assert!(reads_only_word_count(
            &params(),
            "role [llength ${words}] Body"
        ));
    }

    #[test]
    fn any_other_read_of_the_words_or_the_context_is_not() {
        for body in [
            "role [lindex $words 0] Body",
            "set n [llength $words]; foreach w $words {}",
            "if {[dict get $ctx dialect] eq {x}} {role 0 Body}",
            "upvar 0 words w; role [llength $w] Body",
            "set v ctx; role 0 Body",
        ] {
            assert!(!reads_only_word_count(&params(), body), "{body}");
        }
        assert!(reads_only_word_count(
            &params(),
            "set wordsmith 1; role 0 Body"
        ));
    }
}
