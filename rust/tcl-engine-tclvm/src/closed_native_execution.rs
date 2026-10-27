// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A private Native world whose only returned observation is detached bytes.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::optimiser::closed_consumer::{
    ClosedSourcePlan, MAX_CLOSED_OUTPUT_BYTES, MIN_CLOSED_STACK_BYTES, optimise_closed_source,
    prepare_closed_source,
};
use tcl_platform::{
    Capabilities, Clock, Env, Host, HostError, NativeCIntegerAbi, NumericEnvironment,
    NumericEnvironmentUnavailable, NumericErrorState, SignedNumericConversion, StdIo,
    UnsignedNumericConversion,
};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};
use tcl_runtime_api::closed_native_consumer::{
    ClosedNativeBackend, ClosedNativeConsumerOwner, ClosedNativeState, ClosedSourceOutput,
    ClosedSourceRefusal, ClosedSourceTranscript,
};
use tcl_vm::Vm;

mod application_hook_engine;
pub use application_hook_engine::{PrivateApplicationHookHost, private_application_hook_engine};

/// Optimise and execute inside an owned Native worker. Neither a caller's VM,
/// values, channels, callbacks nor compiler enter this worker. All Native
/// resources and thread-local pools retire before the joined bytes return.
///
/// # Errors
/// Refuses unsupported source, unavailable actual Native prerequisites, changed
/// currency, exhausted source/operation/output bounds, and execution failure.
/// Unexpected Rust panics may still invoke the process-global panic hook; this
/// is not an OS sandbox or isolation from allocator/panic instrumentation.
/// Fatal process aborts cannot promise worker completion or joining.
pub fn execute_optimised_closed_source(
    source: String,
    version: tcl_dialect::TclVersion,
) -> Result<ClosedSourceOutput, ClosedSourceRefusal> {
    let stack = usize::try_from(MIN_CLOSED_STACK_BYTES).map_err(|_| ClosedSourceRefusal::Budget)?;
    #[cfg(debug_assertions)]
    let marker = closed_observation_marker();
    #[cfg(debug_assertions)]
    let worker_marker = marker.clone();
    let joined = std::thread::Builder::new()
        .stack_size(stack)
        .name("closed-native-consumer".into())
        .spawn(move || {
            #[cfg(debug_assertions)]
            tcl_runtime_api::diagnostic_output::attach_closed_native_observation(worker_marker);
            execute_worker(&source, version)
        })
        .map_err(|_| ClosedSourceRefusal::Execution)?
        .join();
    #[cfg(debug_assertions)]
    report_closed_observation(marker.as_ref(), !joined.as_ref().is_ok_and(Result::is_ok));
    joined.map_err(|_| ClosedSourceRefusal::Execution)?
}

#[cfg(debug_assertions)]
fn closed_observation_marker() -> Option<std::sync::Arc<std::sync::atomic::AtomicU64>> {
    (std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1"))
        .then(|| std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)))
}

#[cfg(debug_assertions)]
fn report_closed_observation(
    marker: Option<&std::sync::Arc<std::sync::atomic::AtomicU64>>,
    failed: bool,
) {
    if failed && let Some(marker) = marker {
        let value = marker.load(std::sync::atomic::Ordering::Relaxed);
        // Parent only: the worker, its VM and all TLS owners have joined.
        std::eprintln!(
            "CLOSED_NATIVE_REFUSAL stage={} detail={} ordinal={}",
            value & 0xff,
            (value >> 8) & 0xffff,
            (value >> 24) & 0xffff
        );
    }
}

fn execute_worker(
    source: &str,
    version: tcl_dialect::TclVersion,
) -> Result<ClosedSourceOutput, ClosedSourceRefusal> {
    tcl_runtime_api::diagnostic_output::silence_private_worker();
    if !tcl_registry::pack_hooks::deny_external_evaluators_on_private_thread() {
        #[cfg(debug_assertions)]
        tcl_runtime_api::diagnostic_output::note_closed_native_refusal(4, 0, 0);
        return Err(ClosedSourceRefusal::NativeContext);
    }
    let profile =
        tcl_registry::model::ingress::resolve_known_environment(version.dialect_profile_name())
            .and_then(|environment| environment.catalogue_profile())
            .ok_or(ClosedSourceRefusal::NativeContext)
            .inspect_err(|_| {
                #[cfg(debug_assertions)]
                tcl_runtime_api::diagnostic_output::note_closed_native_refusal(5, 0, 0);
            })?;
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let plan = prepare_closed_source(source, &registry, profile)?;
    let output = {
        let backend = PrivateBackend::new(&plan, &registry, profile)?;
        let owner = ClosedNativeConsumerOwner::new(backend)?;
        let scope = owner.scope()?;
        let rewrite = optimise_closed_source(&plan, &registry, &scope)?;
        owner.execute(rewrite)?
    };
    // The scope and kernel's retained entry metadata have also retired here.
    drop(plan);
    drop(registry);
    Ok(output)
}

struct PrivateBackend<'plan> {
    vm: Vm,
    original: tcl_vm::PreparedClosedNativeSource,
    observation: tcl_runtime_api::NativeCompilationEntry,
    plan: &'plan ClosedSourcePlan,
    registry: &'plan CommandRegistry,
    host: Rc<MemoryHost>,
}

impl<'plan> PrivateBackend<'plan> {
    fn new(
        plan: &'plan ClosedSourcePlan,
        registry: &'plan CommandRegistry,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self, ClosedSourceRefusal> {
        if !tcl_registry::pack_hooks::private_thread_evaluators_are_closed() {
            #[cfg(debug_assertions)]
            tcl_runtime_api::diagnostic_output::note_closed_native_refusal(6, 0, 0);
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let host = Rc::new(MemoryHost::new());
        let mut vm = Vm::with_native_core(
            Box::new(MemoryWriter(Rc::clone(&host.sinks))),
            Rc::clone(&host) as Rc<dyn Host>,
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .ok_or(ClosedSourceRefusal::NativeContext)
        .inspect_err(|_| {
            #[cfg(debug_assertions)]
            tcl_runtime_api::diagnostic_output::note_closed_native_refusal(1, 0, 0);
        })?;
        let compiler_registry = registry.project_for_profile(profile);
        if registry.snapshot().semantic_key() != compiler_registry.snapshot().semantic_key() {
            return Err(ClosedSourceRefusal::Currency);
        }
        vm.set_compiler(Box::new(BytecodeCompileService::new(compiler_registry)));
        vm.set_command_limit(Some(256));
        vm.set_wall_clock_budget(Some(Duration::from_millis(250)));
        vm.set_value_size_limit(Some(4096));
        let observation = vm
            .restrict_closed_native_commands(plan.command_names(), plan.scalar_names())
            .inspect_err(|_| {
                #[cfg(debug_assertions)]
                tcl_runtime_api::diagnostic_output::note_closed_native_refusal(2, 0, 0);
            })?;
        let original = vm
            .prepare_closed_native_source(
                plan.source(),
                plan.command_names(),
                plan.scalar_names(),
                &observation,
            )
            .inspect_err(|_| {
                #[cfg(debug_assertions)]
                tcl_runtime_api::diagnostic_output::note_closed_native_refusal(3, 0, 0);
            })?;
        Ok(Self {
            vm,
            original,
            observation,
            plan,
            registry,
            host,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
struct PrivateCurrency {
    registry: RegistrySemanticKey,
    numeric: NumericErrorState,
    abi: NativeCIntegerAbi,
}

impl ClosedNativeBackend for PrivateBackend<'_> {
    type Currency = PrivateCurrency;

    fn original_source(&self) -> &str {
        self.plan.source()
    }

    fn capture_closed_state(
        &self,
    ) -> Result<ClosedNativeState<PrivateCurrency>, ClosedSourceRefusal> {
        let quiet = {
            let sinks = self.host.sinks.borrow();
            !sinks.refused && sinks.stdout.is_empty() && sinks.stderr.is_empty()
        };
        if !tcl_registry::pack_hooks::private_thread_evaluators_are_closed()
            || !self.plan.matches_registry(self.registry)
            || !quiet
            || !self.vm.closed_native_preparation_is_current(
                &self.original,
                self.plan.command_names(),
                self.plan.scalar_names(),
            )
        {
            return Err(ClosedSourceRefusal::Currency);
        }
        let entry = self.vm.observe_closed_native_state(
            self.plan.command_names(),
            self.plan.scalar_names(),
            &self.observation,
        )?;
        if &entry.profile != self.plan.profile() {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let numeric = self
            .host
            .state()
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        if numeric.range_error || numeric.domain_error {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let abi = self
            .host
            .c_integer_abi()
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        Ok(ClosedNativeState {
            entry,
            currency: PrivateCurrency {
                registry: self.registry.snapshot().semantic_key(),
                numeric,
                abi,
            },
        })
    }

    fn validate_rewrite(&self, candidate: &str) -> Result<(), ClosedSourceRefusal> {
        if self.plan.accepts_candidate(candidate, self.registry) {
            Ok(())
        } else {
            Err(ClosedSourceRefusal::Source)
        }
    }

    fn execute_owned(
        mut self,
        candidate: String,
    ) -> Result<ClosedSourceTranscript, ClosedSourceRefusal> {
        self.validate_rewrite(&candidate)?;
        self.capture_closed_state()?;
        let prepared = self.vm.prepare_closed_native_source(
            &candidate,
            self.plan.command_names(),
            self.plan.scalar_names(),
            &self.observation,
        )?;
        self.capture_closed_state()?;
        self.vm.execute_closed_native_preparation(
            prepared,
            self.plan.command_names(),
            self.plan.scalar_names(),
        )?;
        drop(self.original);
        drop(self.observation);
        // No result or compiler object survives this explicit VM retirement.
        drop(self.vm);
        let mut sinks = self.host.sinks.borrow_mut();
        if sinks.refused || sinks.stdout != self.plan.expected_stdout() || !sinks.stderr.is_empty()
        {
            return Err(ClosedSourceRefusal::Execution);
        }
        Ok(ClosedSourceTranscript {
            stdout: std::mem::take(&mut sinks.stdout),
            stderr: std::mem::take(&mut sinks.stderr),
        })
    }
}

#[derive(Default)]
struct Sinks {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    refused: bool,
}

impl Sinks {
    fn write(&mut self, bytes: &[u8], stderr: bool) -> io::Result<()> {
        if self.refused
            || self.stdout.len() + self.stderr.len() + bytes.len() > MAX_CLOSED_OUTPUT_BYTES
        {
            self.refused = true;
            return Err(io::Error::other("private output bound exceeded"));
        }
        if stderr {
            self.stderr.extend_from_slice(bytes);
        } else {
            self.stdout.extend_from_slice(bytes);
        }
        Ok(())
    }
}

struct MemoryWriter(Rc<RefCell<Sinks>>);

impl Write for MemoryWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(bytes, false)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct MemoryHost {
    sinks: Rc<RefCell<Sinks>>,
    environment: RefCell<BTreeMap<String, String>>,
    clock: Instant,
    numeric: Rc<dyn Host>,
}

impl MemoryHost {
    fn new() -> Self {
        Self {
            sinks: Rc::default(),
            environment: RefCell::default(),
            clock: Instant::now(),
            numeric: tcl_vm::standard_host(),
        }
    }

    fn actual_numeric(&self) -> Result<&dyn NumericEnvironment, NumericEnvironmentUnavailable> {
        self.numeric
            .numeric_environment()
            .ok_or(NumericEnvironmentUnavailable::Target)
    }
}

impl Host for MemoryHost {
    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }
    fn clock(&self) -> &dyn Clock {
        self
    }
    fn stdio(&self) -> &dyn StdIo {
        self
    }
    fn env(&self) -> &dyn Env {
        self
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        Some(self)
    }
}

impl Clock for MemoryHost {
    fn now_secs(&self) -> i64 {
        i64::try_from(self.clock.elapsed().as_secs()).unwrap_or(i64::MAX)
    }
    fn now_millis(&self) -> i128 {
        i128::try_from(self.clock.elapsed().as_millis()).unwrap_or(i128::MAX)
    }
}

impl StdIo for MemoryHost {
    fn write_stdout(&self, bytes: &[u8]) {
        let _ = self.sinks.borrow_mut().write(bytes, false);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        let _ = self.sinks.borrow_mut().write(bytes, true);
    }
}

impl Env for MemoryHost {
    fn get(&self, key: &str) -> Option<String> {
        self.environment.borrow().get(key).cloned()
    }
    fn set(&self, key: &str, value: &str) {
        self.environment
            .borrow_mut()
            .insert(key.into(), value.into());
    }
    fn unset(&self, key: &str) {
        self.environment.borrow_mut().remove(key);
    }
    fn vars(&self) -> Vec<(String, String)> {
        self.environment
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    fn cwd(&self) -> Result<String, HostError> {
        Err(HostError::Unsupported)
    }
    fn chdir(&self, _path: &str) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}

fn bounded_decimal(
    input: &[u8],
    offset: usize,
    base: u32,
) -> Result<(), NumericEnvironmentUnavailable> {
    let digits = input
        .strip_prefix(b"-")
        .or_else(|| input.strip_prefix(b"+"))
        .unwrap_or(input);
    let value = digits.iter().try_fold(0_u32, |value, digit| {
        digit.is_ascii_digit().then_some(())?;
        value.checked_mul(10)?.checked_add(u32::from(*digit - b'0'))
    });
    if digits.is_empty()
        || offset > input.len()
        || !matches!(base, 0 | 10)
        || value.is_none_or(|value| value > 1_000_000)
    {
        return Err(NumericEnvironmentUnavailable::Conversion);
    }
    Ok(())
}

impl NumericEnvironment for MemoryHost {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.actual_numeric()?.c_integer_abi()
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        self.actual_numeric()?.state()
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        bounded_decimal(input, offset, base)?;
        self.actual_numeric()?.unsigned(input, offset, base)
    }
    fn unsigned_long(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        bounded_decimal(input, offset, base)?;
        self.actual_numeric()?.unsigned_long(input, offset, base)
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        bounded_decimal(input, 0, base)?;
        self.actual_numeric()?.signed_long(input, base)
    }
    fn double(
        &self,
        _input: &[u8],
        _reset_errno: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned_test_worker(test: impl FnOnce() + Send + 'static) {
        #[cfg(debug_assertions)]
        let marker = closed_observation_marker();
        #[cfg(debug_assertions)]
        let worker_marker = marker.clone();
        let joined = std::thread::Builder::new()
            .stack_size(usize::try_from(MIN_CLOSED_STACK_BYTES).unwrap())
            .spawn(move || {
                #[cfg(debug_assertions)]
                tcl_runtime_api::diagnostic_output::attach_closed_native_observation(worker_marker);
                tcl_runtime_api::diagnostic_output::silence_private_worker();
                assert!(tcl_registry::pack_hooks::deny_external_evaluators_on_private_thread());
                test();
            })
            .unwrap()
            .join();
        #[cfg(debug_assertions)]
        report_closed_observation(marker.as_ref(), joined.is_err());
        joined.unwrap();
    }

    fn test_plan() -> (ClosedSourcePlan, CommandRegistry) {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let plan = prepare_closed_source(
            "set x 1\nputs [expr {$x + 1}]\nputs $x\n",
            &registry,
            profile,
        )
        .unwrap();
        (plan, registry)
    }

    #[test]
    fn actual_owners_reject_foreign_superseded_and_consumed_selections() {
        owned_test_worker(|| {
            let (plan, registry) = test_plan();
            let profile = plan.profile().profile();
            let first = ClosedNativeConsumerOwner::new(
                PrivateBackend::new(&plan, &registry, profile).unwrap(),
            )
            .unwrap();
            let second = ClosedNativeConsumerOwner::new(
                PrivateBackend::new(&plan, &registry, profile).unwrap(),
            )
            .unwrap();
            let scope = first.scope().unwrap();
            let foreign = scope.select(plan.rewritten_source().into()).unwrap();
            assert_eq!(
                second.execute(foreign).unwrap_err(),
                ClosedSourceRefusal::Selection
            );
            let old = scope.select(plan.source().into()).unwrap();
            let current = scope.select(plan.rewritten_source().into()).unwrap();
            assert_eq!(
                first.execute(old).unwrap_err(),
                ClosedSourceRefusal::Selection
            );
            assert_eq!(first.execute(current).unwrap().transcript.stdout, b"2\n1\n");
            assert!(first.scope().is_err());
        });
    }

    struct UnreachableCommand;

    #[test]
    fn actual_recapture_and_foreign_literal_owner_refuse_retained_preparation() {
        owned_test_worker(|| {
            let (plan, registry) = test_plan();
            let profile = plan.profile().profile();
            let mut recaptured = PrivateBackend::new(&plan, &registry, profile).unwrap();
            recaptured
                .vm
                .restrict_closed_native_commands(plan.command_names(), plan.scalar_names())
                .unwrap();
            assert!(ClosedNativeConsumerOwner::new(recaptured).is_err());
            let mut first = PrivateBackend::new(&plan, &registry, profile).unwrap();
            let second = PrivateBackend::new(&plan, &registry, profile).unwrap();
            first.observation = second.observation.clone();
            assert!(ClosedNativeConsumerOwner::new(first).is_err());
        });
    }

    impl tcl_vm::NativeCommand for UnreachableCommand {
        fn invoke(
            &self,
            _vm: &mut Vm,
            _args: &[tcl_vm::Value],
        ) -> tcl_vm::Completion<tcl_vm::Value> {
            panic!("replacement must never be invoked")
        }
    }

    #[test]
    fn actual_rebinding_and_owned_sink_refusal_prevent_a_grant_or_consume() {
        owned_test_worker(|| {
            let (plan, registry) = test_plan();
            let profile = plan.profile().profile();
            let mut rebound = PrivateBackend::new(&plan, &registry, profile).unwrap();
            rebound
                .vm
                .try_register_native_command("puts", Rc::new(UnreachableCommand))
                .unwrap();
            assert!(!rebound.vm.closed_native_preparation_is_current(
                &rebound.original,
                plan.command_names(),
                plan.scalar_names()
            ));
            assert!(ClosedNativeConsumerOwner::new(rebound).is_err());
            let backend = PrivateBackend::new(&plan, &registry, profile).unwrap();
            let sinks = Rc::clone(&backend.host.sinks);
            let owner = ClosedNativeConsumerOwner::new(backend).unwrap();
            let selected = owner
                .scope()
                .unwrap()
                .select(plan.rewritten_source().into())
                .unwrap();
            assert!(
                sinks
                    .borrow_mut()
                    .write(&vec![b'x'; MAX_CLOSED_OUTPUT_BYTES + 1], false)
                    .is_err()
            );
            assert_eq!(
                owner.execute(selected).unwrap_err(),
                ClosedSourceRefusal::Currency
            );
            assert_eq!(sinks.borrow().stdout.as_slice(), b"");
        });
    }

    #[test]
    fn private_consumer_preserves_the_complete_transcript() {
        let output = execute_optimised_closed_source(
            "set x 1\nputs [expr {$x + 1}]\nputs $x\n".into(),
            tcl_dialect::TclVersion::V8_6,
        )
        .unwrap();
        assert_eq!(output.transcript.stdout, b"2\n1\n");
        assert_eq!(output.transcript.stderr.as_slice(), b"");
        assert!(output.rewritten_source.contains("puts 2"));
        assert!(output.rewritten_source.contains("puts 1"));
        owned_test_worker(|| {
            let (_, registry) = test_plan();
            let profile =
                tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
            for (source, stdout) in [
                (
                    "set x 7; puts [expr {$x - 3}]; puts $x",
                    b"4\n7\n".as_slice(),
                ),
                (
                    "set x 7; puts [expr {$x * 3}]; puts $x",
                    b"21\n7\n".as_slice(),
                ),
                (
                    "set x 7; puts [expr {($x - 3) * ($x + 2)}]; puts $x",
                    b"36\n7\n".as_slice(),
                ),
            ] {
                let plan = prepare_closed_source(source, &registry, profile).unwrap();
                let owner = ClosedNativeConsumerOwner::new(
                    PrivateBackend::new(&plan, &registry, profile).unwrap(),
                )
                .unwrap();
                let selection = owner.scope().unwrap().select(source.into()).unwrap();
                let output = owner.execute(selection).unwrap();
                assert_eq!(output.transcript.stdout, stdout);
                assert!(output.transcript.stderr.is_empty());
            }
        });
    }

    #[test]
    fn observer_channels_dynamic_heads_and_external_cells_are_not_admitted() {
        for source in [
            "trace add variable x read puts; set x 1; puts $x",
            "chan push stdout transform; puts 1",
            "set c puts; $c 1",
            "puts $::env(PATH)",
            "set tcl_precision 1; puts 1",
            "proc f {} {puts 1}; f",
            "puts [expr {sin(1)}]",
            "puts [expr {1 / 0}]",
        ] {
            assert!(
                execute_optimised_closed_source(source.into(), tcl_dialect::TclVersion::V8_6)
                    .is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn arithmetic_and_output_bounds_refuse_before_publication() {
        for source in [
            "puts [expr {1000000 * 1000000}]",
            "puts [expr {1.0 + 1}]",
            "set x 1000000; puts [expr {($x + 1) - 1}]",
        ] {
            assert!(
                execute_optimised_closed_source(source.into(), tcl_dialect::TclVersion::V8_6)
                    .is_err()
            );
        }
        let mut sinks = Sinks::default();
        assert!(
            sinks
                .write(&vec![b'x'; MAX_CLOSED_OUTPUT_BYTES], false)
                .is_ok()
        );
        assert!(sinks.write(b"y", true).is_err());
        assert!(sinks.refused);
        assert_eq!(sinks.stderr.as_slice(), b"");
    }
}
