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

//! The loader and link lifecycle (issue #1204).
//!
//! A deployment moves through an explicit state machine —
//! `plan → load → (test-run) → attach → status → detach` — with atomic
//! rollback on partial failure and strict resource ownership. Every kernel
//! effect goes through the [`KernelOps`] trait, so the whole lifecycle is
//! **modelled and unit-tested deterministically in userspace** without a kernel
//! (this container has none). A real `bpf()`-syscall implementation of
//! [`KernelOps`] is the only part that needs root, and is exercised only by the
//! `#[ignore]`d privileged integration tests.
//!
//! # Resource ownership
//!
//! Every pin this loader creates lives under a unique, ownership-labelled path:
//!
//! ```text
//! /sys/fs/bpf/bpftcl/<deployment>/<program>
//! ```
//!
//! `<deployment>` is the caller's deployment name. The loader **only** removes
//! resources whose pin path is under its own `PIN_ROOT/<deployment>/` prefix and
//! whose recorded owner label matches — it refuses to replace or delete a pin
//! owned by anything else (issue #1204: "refuse to replace unrelated
//! links/pins", "remove only resources owned by the deployment"). A `load` that
//! fails partway cleans up every resource it created before returning, so a
//! failed deployment leaves no orphan maps, programs, links, or pins.

use crate::ir::{AttachSpec, BpfModule, ProgType};

/// The pin filesystem root every deployment's resources live under.
pub const PIN_ROOT: &str = "/sys/fs/bpf/bpftcl";

/// The lifecycle state of a deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Planned only — nothing has touched the kernel.
    Planned,
    /// Programs verifier-loaded and maps created; not attached.
    Loaded,
    /// Links created and pinned; the deployment is live.
    Attached,
    /// Detached — all owned resources removed.
    Detached,
}

/// A loader error. Every fallible lifecycle step returns this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderError {
    /// A step was called from a state it is not valid in.
    BadState {
        /// The step attempted.
        step: &'static str,
        /// The current state.
        state: State,
    },
    /// The kernel (or the model) rejected a program at verifier load.
    VerifierRejected(String),
    /// `BPF_PROG_TEST_RUN` is not supported for this program type.
    TestRunUnsupported(ProgType),
    /// Attaching a link failed.
    AttachFailed(String),
    /// A pin path is owned by a different deployment — refused.
    ForeignPin {
        /// The contested pin path.
        path: String,
        /// The recorded owner.
        owner: String,
    },
    /// The deployment has no compiled programs to load.
    Empty,
}

impl core::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoaderError::BadState { step, state } => {
                write!(f, "cannot `{step}` from state {state:?}")
            }
            LoaderError::VerifierRejected(m) => write!(f, "verifier rejected program: {m}"),
            LoaderError::TestRunUnsupported(pt) => {
                write!(f, "BPF_PROG_TEST_RUN unsupported for {pt:?}")
            }
            LoaderError::AttachFailed(m) => write!(f, "attach failed: {m}"),
            LoaderError::ForeignPin { path, owner } => {
                write!(f, "refusing to touch pin {path} owned by `{owner}`")
            }
            LoaderError::Empty => write!(f, "deployment has no programs to load"),
        }
    }
}

impl std::error::Error for LoaderError {}

/// A planned program within a deployment (the unit of load/attach).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedProgram {
    /// The event/handler name.
    pub event: String,
    /// Its program type.
    pub prog_type: ProgType,
    /// Handler priority (composition order).
    pub priority: u32,
    /// The deployment target, if any.
    pub attach: Option<AttachSpec>,
    /// The unique, ownership-labelled pin path this program is pinned at.
    pub pin: String,
}

/// A whole deployment plan derived from a compiled module + a deployment name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentPlan {
    /// The deployment name — the ownership label baked into every pin path.
    pub name: String,
    /// The planned programs, in the module's composition order.
    pub programs: Vec<PlannedProgram>,
}

impl DeploymentPlan {
    /// Build a plan from a compiled module. Each program gets a unique
    /// ownership-labelled pin path under `PIN_ROOT/<name>/`.
    #[must_use]
    pub fn from_module(name: &str, module: &BpfModule) -> Self {
        let mut programs = Vec::with_capacity(module.programs.len());
        for (i, decl) in module.programs.iter().enumerate() {
            // The index disambiguates two handlers of the same event so pins
            // never collide within a deployment.
            let pin = format!("{PIN_ROOT}/{name}/{}_{i}", decl.event.to_ascii_lowercase());
            programs.push(PlannedProgram {
                event: decl.event.clone(),
                prog_type: decl.program.prog_type,
                priority: decl.priority,
                attach: decl.attach.clone(),
                pin,
            });
        }
        DeploymentPlan {
            name: name.to_owned(),
            programs,
        }
    }

    /// Whether the plan owns `pin` — it is under this deployment's prefix.
    #[must_use]
    pub fn owns_pin(&self, pin: &str) -> bool {
        pin.starts_with(&format!("{PIN_ROOT}/{}/", self.name))
    }
}

/// A handle to a loaded program in the (modelled or real) kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgHandle(pub u32);

/// A handle to an attached link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkHandle(pub u32);

/// The kernel operations the lifecycle drives. A real implementation issues
/// `bpf()` syscalls (needs root); [`ModelKernel`] simulates them deterministically
/// for userspace tests.
pub trait KernelOps {
    /// Verifier-load one program; returns its handle or a rejection reason.
    ///
    /// # Errors
    /// The verifier (or model) rejected the program.
    fn load_program(&mut self, prog: &PlannedProgram) -> Result<ProgHandle, LoaderError>;

    /// Run a loaded program through `BPF_PROG_TEST_RUN` over `packet`; returns
    /// the verdict value.
    ///
    /// # Errors
    /// Test-run is unsupported for the program type, or failed.
    fn test_run(&mut self, prog: ProgHandle, packet: &[u8]) -> Result<u64, LoaderError>;

    /// Create + pin a link for a loaded program at `pin`, recording `owner`.
    ///
    /// # Errors
    /// The pin is owned by someone else, or the attach failed.
    fn attach_link(
        &mut self,
        prog: ProgHandle,
        pin: &str,
        owner: &str,
    ) -> Result<LinkHandle, LoaderError>;

    /// Remove a link and its pin, but only if the pin's recorded owner matches
    /// `owner`.
    ///
    /// # Errors
    /// The pin is owned by someone else.
    fn detach_link(&mut self, link: LinkHandle, pin: &str, owner: &str) -> Result<(), LoaderError>;

    /// Remove a loaded (but unattached) program.
    fn unload_program(&mut self, prog: ProgHandle);

    /// The owner recorded for a pin, if it exists.
    fn pin_owner(&self, pin: &str) -> Option<String>;
}

/// A live deployment driving the state machine over a [`KernelOps`] backend.
#[derive(Debug)]
pub struct Deployment<K: KernelOps> {
    plan: DeploymentPlan,
    kernel: K,
    state: State,
    loaded: Vec<(usize, ProgHandle)>,
    links: Vec<(usize, LinkHandle, String)>,
}

impl<K: KernelOps> Deployment<K> {
    /// Begin a deployment from a plan and a kernel backend, in [`State::Planned`].
    #[must_use]
    pub fn new(plan: DeploymentPlan, kernel: K) -> Self {
        Deployment {
            plan,
            kernel,
            state: State::Planned,
            loaded: Vec::new(),
            links: Vec::new(),
        }
    }

    /// The current lifecycle state.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// The plan being deployed.
    #[must_use]
    pub fn plan(&self) -> &DeploymentPlan {
        &self.plan
    }

    /// Borrow the kernel backend (e.g. to inspect the [`ModelKernel`] in tests).
    #[must_use]
    pub fn kernel(&self) -> &K {
        &self.kernel
    }

    /// Verifier-load every program and create maps, without attaching. On any
    /// rejection, unload everything already loaded (atomic: all-or-nothing).
    ///
    /// # Errors
    /// [`LoaderError::Empty`] with no programs, or the first verifier rejection
    /// (after cleaning up).
    pub fn load(&mut self) -> Result<(), LoaderError> {
        if self.state != State::Planned {
            return Err(LoaderError::BadState {
                step: "load",
                state: self.state,
            });
        }
        if self.plan.programs.is_empty() {
            return Err(LoaderError::Empty);
        }
        for (i, prog) in self.plan.programs.iter().enumerate() {
            match self.kernel.load_program(prog) {
                Ok(handle) => self.loaded.push((i, handle)),
                Err(e) => {
                    // Roll back the partial load — remove every program we made.
                    for (_, h) in self.loaded.drain(..) {
                        self.kernel.unload_program(h);
                    }
                    return Err(e);
                }
            }
        }
        self.state = State::Loaded;
        Ok(())
    }

    /// Run one loaded program (by plan index) through `BPF_PROG_TEST_RUN`.
    ///
    /// # Errors
    /// Wrong state, unknown index, or test-run failure.
    pub fn test_run(&mut self, program_index: usize, packet: &[u8]) -> Result<u64, LoaderError> {
        if self.state != State::Loaded && self.state != State::Attached {
            return Err(LoaderError::BadState {
                step: "test-run",
                state: self.state,
            });
        }
        let handle = self
            .loaded
            .iter()
            .find(|(i, _)| *i == program_index)
            .map(|(_, h)| *h)
            .ok_or(LoaderError::Empty)?;
        self.kernel.test_run(handle, packet)
    }

    /// Attach every loaded program via a pinned link. On any failure, detach
    /// every link already created in this call (rollback) and stay in
    /// [`State::Loaded`], leaving unrelated host state untouched.
    ///
    /// # Errors
    /// Wrong state, a foreign pin, or an attach failure (after rollback).
    pub fn attach(&mut self) -> Result<(), LoaderError> {
        if self.state != State::Loaded {
            return Err(LoaderError::BadState {
                step: "attach",
                state: self.state,
            });
        }
        let owner = self.plan.name.clone();
        let planned: Vec<(usize, ProgHandle, String)> = self
            .loaded
            .iter()
            .map(|(i, h)| (*i, *h, self.plan.programs[*i].pin.clone()))
            .collect();
        for (i, handle, pin) in planned {
            match self.kernel.attach_link(handle, &pin, &owner) {
                Ok(link) => self.links.push((i, link, pin)),
                Err(e) => {
                    // Roll back links created in this attach call only.
                    for (_, link, pin) in self.links.drain(..) {
                        let _ = self.kernel.detach_link(link, &pin, &owner);
                    }
                    return Err(e);
                }
            }
        }
        self.state = State::Attached;
        Ok(())
    }

    /// Detach and remove **only** this deployment's owned resources: every
    /// pinned link, then every loaded program. Never touches a pin owned by
    /// something else.
    ///
    /// # Errors
    /// A pin turned out to be foreign (should not happen for owned pins).
    pub fn detach(&mut self) -> Result<(), LoaderError> {
        if self.state != State::Attached && self.state != State::Loaded {
            return Err(LoaderError::BadState {
                step: "detach",
                state: self.state,
            });
        }
        let owner = self.plan.name.clone();
        for (_, link, pin) in self.links.drain(..) {
            debug_assert!(self.plan.owns_pin(&pin));
            self.kernel.detach_link(link, &pin, &owner)?;
        }
        for (_, handle) in self.loaded.drain(..) {
            self.kernel.unload_program(handle);
        }
        self.state = State::Detached;
        Ok(())
    }

    /// A snapshot of the deployment's live resources for `status` output.
    #[must_use]
    pub fn status(&self) -> DeploymentStatus {
        DeploymentStatus {
            name: self.plan.name.clone(),
            state: self.state,
            loaded: self.loaded.len(),
            attached: self.links.len(),
            pins: self.links.iter().map(|(_, _, pin)| pin.clone()).collect(),
        }
    }
}

/// A `status` snapshot of a deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentStatus {
    /// Deployment name / ownership label.
    pub name: String,
    /// Current lifecycle state.
    pub state: State,
    /// Number of loaded programs.
    pub loaded: usize,
    /// Number of attached (pinned) links.
    pub attached: usize,
    /// The owned pin paths currently live.
    pub pins: Vec<String>,
}

/// A deterministic in-memory model of the kernel BPF object space, for
/// userspace tests of the lifecycle. It records loaded programs, pin ownership,
/// and links, and can be configured to inject a verifier rejection or attach
/// failure at a chosen program index (to exercise rollback).
#[derive(Debug, Default)]
pub struct ModelKernel {
    next_prog: u32,
    next_link: u32,
    /// pin path -> owner label.
    pins: std::collections::BTreeMap<String, String>,
    /// loaded program handle -> its program type.
    live_progs: std::collections::BTreeMap<u32, ProgTypeKey>,
    /// Program index at which `load_program` should fail (verifier reject).
    fail_load_at_call: Option<usize>,
    /// Attach call ordinal at which `attach_link` should fail.
    fail_attach_at_call: Option<usize>,
    load_calls: usize,
    attach_calls: usize,
    /// Program types that support test-run.
    test_runnable: std::collections::BTreeSet<ProgTypeKey>,
}

/// A hashable key for [`ProgType`] (which is `Copy` but not `Ord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ProgTypeKey(u8);

impl From<ProgType> for ProgTypeKey {
    fn from(pt: ProgType) -> Self {
        ProgTypeKey(match pt {
            ProgType::SocketFilter => 0,
            ProgType::Xdp => 1,
            ProgType::TcIngress => 2,
            ProgType::TcEgress => 3,
            ProgType::CgroupInet4Connect => 4,
            ProgType::CgroupInet4Bind => 5,
        })
    }
}

impl ProgTypeKey {
    /// The inverse of [`From<ProgType>`], for reconstructing a `ProgType` from
    /// a stored key (e.g. for an error that names the program type).
    fn to_prog_type(self) -> ProgType {
        match self.0 {
            0 => ProgType::SocketFilter,
            1 => ProgType::Xdp,
            2 => ProgType::TcIngress,
            3 => ProgType::TcEgress,
            4 => ProgType::CgroupInet4Connect,
            _ => ProgType::CgroupInet4Bind,
        }
    }
}

impl ModelKernel {
    /// A model where every program type supports test-run and nothing fails.
    #[must_use]
    pub fn new() -> Self {
        let mut test_runnable = std::collections::BTreeSet::new();
        for pt in [
            ProgType::SocketFilter,
            ProgType::Xdp,
            ProgType::TcIngress,
            ProgType::TcEgress,
            ProgType::CgroupInet4Connect,
            ProgType::CgroupInet4Bind,
        ] {
            test_runnable.insert(ProgTypeKey::from(pt));
        }
        ModelKernel {
            test_runnable,
            ..ModelKernel::default()
        }
    }

    /// Configure a verifier rejection at the given (0-based) load call.
    #[must_use]
    pub fn failing_load_at(mut self, call: usize) -> Self {
        self.fail_load_at_call = Some(call);
        self
    }

    /// Configure an attach failure at the given (0-based) attach call.
    #[must_use]
    pub fn failing_attach_at(mut self, call: usize) -> Self {
        self.fail_attach_at_call = Some(call);
        self
    }

    /// Pre-seed a pin owned by someone else (to test foreign-pin refusal).
    #[must_use]
    pub fn with_foreign_pin(mut self, pin: &str, owner: &str) -> Self {
        self.pins.insert(pin.to_owned(), owner.to_owned());
        self
    }

    /// Configure the model so that the given program type does **not** support
    /// `BPF_PROG_TEST_RUN` (e.g. cgroup hooks in a real kernel).
    #[must_use]
    pub fn without_test_run_for(mut self, pt: ProgType) -> Self {
        self.test_runnable.remove(&ProgTypeKey::from(pt));
        self
    }

    /// The number of pins currently recorded (owned by anyone).
    #[must_use]
    pub fn pin_count(&self) -> usize {
        self.pins.len()
    }

    /// The number of loaded programs still live.
    #[must_use]
    pub fn live_program_count(&self) -> usize {
        self.live_progs.len()
    }

    /// Whether a pin exists.
    #[must_use]
    pub fn has_pin(&self, pin: &str) -> bool {
        self.pins.contains_key(pin)
    }
}

impl KernelOps for ModelKernel {
    fn load_program(&mut self, prog: &PlannedProgram) -> Result<ProgHandle, LoaderError> {
        let call = self.load_calls;
        self.load_calls += 1;
        if self.fail_load_at_call == Some(call) {
            return Err(LoaderError::VerifierRejected(format!(
                "modelled rejection loading `{}`",
                prog.event
            )));
        }
        let id = self.next_prog;
        self.next_prog += 1;
        self.live_progs
            .insert(id, ProgTypeKey::from(prog.prog_type));
        Ok(ProgHandle(id))
    }

    fn test_run(&mut self, prog: ProgHandle, _packet: &[u8]) -> Result<u64, LoaderError> {
        let Some(&ty) = self.live_progs.get(&prog.0) else {
            return Err(LoaderError::Empty);
        };
        if !self.test_runnable.contains(&ty) {
            // Every type is runnable in the default model; this only fires
            // when a test configures `without_test_run_for` to disallow one.
            return Err(LoaderError::TestRunUnsupported(ty.to_prog_type()));
        }
        // The model returns a fixed sentinel; real verdicts come from the rbpf
        // harness in the bpf-tcl crate, not the lifecycle model.
        Ok(0)
    }

    fn attach_link(
        &mut self,
        prog: ProgHandle,
        pin: &str,
        owner: &str,
    ) -> Result<LinkHandle, LoaderError> {
        let call = self.attach_calls;
        self.attach_calls += 1;
        // Refuse to overwrite a pin owned by someone else.
        if let Some(existing) = self.pins.get(pin)
            && existing != owner
        {
            return Err(LoaderError::ForeignPin {
                path: pin.to_owned(),
                owner: existing.clone(),
            });
        }
        if self.fail_attach_at_call == Some(call) {
            return Err(LoaderError::AttachFailed(format!(
                "modelled attach failure for prog {}",
                prog.0
            )));
        }
        let id = self.next_link;
        self.next_link += 1;
        self.pins.insert(pin.to_owned(), owner.to_owned());
        Ok(LinkHandle(id))
    }

    fn detach_link(
        &mut self,
        _link: LinkHandle,
        pin: &str,
        owner: &str,
    ) -> Result<(), LoaderError> {
        match self.pins.get(pin) {
            Some(existing) if existing == owner => {
                self.pins.remove(pin);
                Ok(())
            }
            Some(existing) => Err(LoaderError::ForeignPin {
                path: pin.to_owned(),
                owner: existing.clone(),
            }),
            None => Ok(()),
        }
    }

    fn unload_program(&mut self, prog: ProgHandle) {
        self.live_progs.remove(&prog.0);
    }

    fn pin_owner(&self, pin: &str) -> Option<String> {
        self.pins.get(pin).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_module;

    fn plan(name: &str, src: &str) -> DeploymentPlan {
        let module = compile_module(src).expect("compiles");
        DeploymentPlan::from_module(name, &module)
    }

    const TWO_XDP: &str = "when XDP priority 10 { pass }\nwhen XDP priority 20 { pass }\n";

    #[test]
    fn pin_paths_are_unique_and_ownership_labelled() {
        let p = plan("deploy-a", TWO_XDP);
        assert_eq!(p.programs.len(), 2);
        assert!(
            p.programs[0]
                .pin
                .starts_with("/sys/fs/bpf/bpftcl/deploy-a/")
        );
        assert_ne!(p.programs[0].pin, p.programs[1].pin);
        assert!(p.owns_pin(&p.programs[0].pin));
        assert!(!p.owns_pin("/sys/fs/bpf/bpftcl/other/xdp_0"));
    }

    #[test]
    fn full_lifecycle_load_attach_detach_leaks_nothing() {
        let mut d = Deployment::new(plan("d1", TWO_XDP), ModelKernel::new());
        assert_eq!(d.state(), State::Planned);
        d.load().unwrap();
        assert_eq!(d.state(), State::Loaded);
        assert_eq!(d.kernel().live_program_count(), 2);
        d.attach().unwrap();
        assert_eq!(d.state(), State::Attached);
        assert_eq!(d.kernel().pin_count(), 2);
        let status = d.status();
        assert_eq!(status.attached, 2);
        assert_eq!(status.pins.len(), 2);
        d.detach().unwrap();
        assert_eq!(d.state(), State::Detached);
        // Every owned resource removed.
        assert_eq!(d.kernel().pin_count(), 0);
        assert_eq!(d.kernel().live_program_count(), 0);
    }

    #[test]
    fn partial_load_failure_rolls_back_every_program() {
        // Fail the second load; the first must be cleaned up (all-or-nothing).
        let mut d = Deployment::new(plan("d2", TWO_XDP), ModelKernel::new().failing_load_at(1));
        let err = d.load().unwrap_err();
        assert!(matches!(err, LoaderError::VerifierRejected(_)));
        assert_eq!(d.state(), State::Planned);
        assert_eq!(d.kernel().live_program_count(), 0, "no orphan programs");
    }

    #[test]
    fn partial_attach_failure_rolls_back_links_and_stays_loaded() {
        let mut d = Deployment::new(plan("d3", TWO_XDP), ModelKernel::new().failing_attach_at(1));
        d.load().unwrap();
        let err = d.attach().unwrap_err();
        assert!(matches!(err, LoaderError::AttachFailed(_)));
        assert_eq!(d.state(), State::Loaded, "attach failure returns to Loaded");
        assert_eq!(d.kernel().pin_count(), 0, "no orphan pins after rollback");
        // Programs remain loaded (rollback is of the attach, not the load).
        assert_eq!(d.kernel().live_program_count(), 2);
        // Recovery: detach cleans up the still-loaded programs.
        d.detach().unwrap();
        assert_eq!(d.kernel().live_program_count(), 0);
    }

    #[test]
    fn refuses_to_attach_over_a_foreign_pin() {
        let p = plan("d4", "when XDP { pass }\n");
        let foreign = p.programs[0].pin.clone();
        let mut d = Deployment::new(
            p,
            ModelKernel::new().with_foreign_pin(&foreign, "someone-else"),
        );
        d.load().unwrap();
        let err = d.attach().unwrap_err();
        assert!(matches!(err, LoaderError::ForeignPin { .. }));
        // The foreign pin is untouched.
        assert_eq!(
            d.kernel().pin_owner(&foreign).as_deref(),
            Some("someone-else")
        );
    }

    #[test]
    fn detach_never_removes_a_foreign_pin() {
        // A deployment whose plan pin collides with a foreign owner: detach must
        // refuse rather than delete someone else's pin.
        let p = plan("d5", "when XDP { pass }\n");
        let pin = p.programs[0].pin.clone();
        let mut kernel = ModelKernel::new();
        // Manually simulate: our link exists but the pin is owned by another.
        kernel = kernel.with_foreign_pin(&pin, "intruder");
        let mut d = Deployment::new(p, kernel);
        d.load().unwrap();
        // Attach refuses (foreign pin), so nothing of ours is attached.
        assert!(d.attach().is_err());
        // Detach the loaded-only deployment: it has no owned links, and must not
        // remove the intruder's pin.
        d.detach().unwrap();
        assert_eq!(d.kernel().pin_owner(&pin).as_deref(), Some("intruder"));
    }

    #[test]
    fn out_of_order_steps_are_rejected() {
        let mut d = Deployment::new(plan("d6", TWO_XDP), ModelKernel::new());
        // attach before load
        assert!(matches!(
            d.attach().unwrap_err(),
            LoaderError::BadState { .. }
        ));
        // test-run before load
        assert!(matches!(
            d.test_run(0, &[]).unwrap_err(),
            LoaderError::BadState { .. }
        ));
    }

    #[test]
    fn test_run_returns_a_value_for_supported_types() {
        let mut d = Deployment::new(plan("tr", "when XDP { pass }\n"), ModelKernel::new());
        d.load().unwrap();
        assert!(d.test_run(0, &[0u8; 64]).is_ok());
    }

    #[test]
    fn test_run_rejects_unsupported_program_types() {
        let mut d = Deployment::new(
            plan("tr2", "when XDP { pass }\n"),
            ModelKernel::new().without_test_run_for(ProgType::Xdp),
        );
        d.load().unwrap();
        assert!(matches!(
            d.test_run(0, &[]).unwrap_err(),
            LoaderError::TestRunUnsupported(ProgType::Xdp)
        ));
    }

    #[test]
    fn empty_module_cannot_load() {
        // A module with no compiled programs.
        let plan = DeploymentPlan {
            name: "empty".to_owned(),
            programs: Vec::new(),
        };
        let mut d = Deployment::new(plan, ModelKernel::new());
        assert_eq!(d.load().unwrap_err(), LoaderError::Empty);
    }
}
