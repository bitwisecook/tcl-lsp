// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The private application VM: a native C9.1 core with memory-only I/O, an
//! input-restricted compiler and bounded limits. It issues no closed scope;
//! its owning backend admits the candidate first.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use tcl_compiler::compile_service::ApplicationInputCompileService;
use tcl_platform::{Host, NativeCIntegerAbi, NumericEnvironment};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};
use tcl_runtime_api::NativeCompilationEntry;
use tcl_runtime_api::closed_native_consumer::{ClosedSourceRefusal, ClosedSourceTranscript};
use tcl_vm::{PreparedOwnedApplicationSource, Vm};

use super::{MemoryHost, MemoryWriter};

/// The release the private application VM runs.
const RELEASE: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_1;

/// One private VM for one admitted candidate. It owns its host, compiler and
/// every value; nothing it holds is reachable by a caller.
pub struct PrivateApplicationVm {
    vm: Vm,
    host: Rc<MemoryHost>,
    observation: NativeCompilationEntry,
    commands: Vec<String>,
    scalars: Vec<String>,
    abi: NativeCIntegerAbi,
}

impl PrivateApplicationVm {
    /// Build the VM for `candidate`, compiled against `registry` itself. The
    /// compiler is installed once, before
    /// the first observation: `Vm::with_native_core` installs none, and
    /// `set_compiler` advances the compiler generation exactly once, which the
    /// prepared module then pins. `commands` are restricted and `scalars`
    /// must start absent before any observation is taken.
    ///
    /// # Errors
    /// `Currency` when `registry` does not carry `expected`; `NativeContext`
    /// for an unavailable profile, bootstrap, command or scalar root.
    pub fn new(
        registry: &Arc<CommandRegistry>,
        expected: &RegistrySemanticKey,
        candidate: &str,
        commands: Vec<String>,
        scalars: Vec<String>,
    ) -> Result<Self, ClosedSourceRefusal> {
        let profile =
            tcl_registry::model::ingress::resolve_known_environment(RELEASE.dialect_profile_name())
                .and_then(|environment| environment.catalogue_profile())
                .ok_or(ClosedSourceRefusal::NativeContext)?;
        if registry.snapshot().semantic_key() != *expected {
            return Err(ClosedSourceRefusal::Currency);
        }
        let host = Rc::new(MemoryHost::new());
        let mut vm = Vm::with_native_core(
            Box::new(MemoryWriter(Rc::clone(&host.sinks))),
            Rc::clone(&host) as Rc<dyn Host>,
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .ok_or(ClosedSourceRefusal::NativeContext)?;
        vm.set_compiler(Box::new(ApplicationInputCompileService::new(
            profile,
            Arc::clone(registry),
            candidate,
        )));
        vm.set_command_limit(Some(256));
        vm.set_wall_clock_budget(Some(Duration::from_millis(250)));
        vm.set_value_size_limit(Some(4096));
        let observation = vm.restrict_closed_native_commands(&commands, &scalars)?;
        let abi = host
            .c_integer_abi()
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        let private = Self {
            vm,
            host,
            observation,
            commands,
            scalars,
            abi,
        };
        private.capture_state()?;
        Ok(private)
    }

    /// The current entry, after checking that memory I/O is silent, the
    /// environment empty, no numeric error pending and the integer ABI
    /// unchanged.
    ///
    /// # Errors
    /// Refuses any of those, or a changed command, observer or scalar state.
    pub fn capture_state(&self) -> Result<NativeCompilationEntry, ClosedSourceRefusal> {
        let quiet = {
            let sinks = self
                .host
                .sinks
                .try_borrow()
                .map_err(|_| ClosedSourceRefusal::Currency)?;
            !sinks.refused && sinks.stdout.is_empty() && sinks.stderr.is_empty()
        };
        let empty_environment = self
            .host
            .environment
            .try_borrow()
            .map_err(|_| ClosedSourceRefusal::Currency)?
            .is_empty();
        let numeric = self
            .host
            .state()
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        let abi = self
            .host
            .c_integer_abi()
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        if !quiet
            || !empty_environment
            || numeric.range_error
            || numeric.domain_error
            || abi != self.abi
        {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        self.vm
            .observe_closed_native_state(&self.commands, &self.scalars, &self.observation)
    }

    /// The integer ABI captured at construction.
    #[must_use]
    pub fn abi(&self) -> NativeCIntegerAbi {
        self.abi
    }

    /// Compile and admit `candidate` through the installed service.
    ///
    /// # Errors
    /// As [`Vm::prepare_owned_application_source`], or a state drift.
    pub fn prepare(
        &self,
        candidate: &str,
    ) -> Result<PreparedOwnedApplicationSource, ClosedSourceRefusal> {
        let before = self.capture_state()?;
        let prepared = self.vm.prepare_owned_application_source(
            candidate,
            &self.commands,
            &self.scalars,
            &self.observation,
        )?;
        if self.capture_state()? != before {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(prepared)
    }

    /// Run the prepared candidate once, retire the VM and its compiler, and
    /// return the detached bytes.
    ///
    /// # Errors
    /// Refuses drift, any non-normal completion or limit breach, refused or
    /// standard-error output.
    pub fn execute(
        mut self,
        prepared: PreparedOwnedApplicationSource,
    ) -> Result<ClosedSourceTranscript, ClosedSourceRefusal> {
        self.capture_state()?;
        self.vm
            .execute_owned_application_preparation(prepared, &self.commands, &self.scalars)?;
        drop(self.observation);
        drop(self.vm);
        let mut sinks = self
            .host
            .sinks
            .try_borrow_mut()
            .map_err(|_| ClosedSourceRefusal::Currency)?;
        if sinks.refused || !sinks.stderr.is_empty() {
            return Err(ClosedSourceRefusal::Execution);
        }
        Ok(ClosedSourceTranscript {
            stdout: std::mem::take(&mut sinks.stdout),
            stderr: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .and_then(|environment| environment.catalogue_profile())
            .unwrap()
    }

    fn registry() -> Arc<CommandRegistry> {
        Arc::new(CommandRegistry::build_default().project_for_profile(profile()))
    }

    fn private(candidate: &str, commands: &[&str], scalars: &[&str]) -> PrivateApplicationVm {
        let registry = registry();
        let key = registry.snapshot().semantic_key();
        PrivateApplicationVm::new(
            &registry,
            &key,
            candidate,
            commands.iter().map(|name| (*name).to_owned()).collect(),
            scalars.iter().map(|name| (*name).to_owned()).collect(),
        )
        .expect("private application VM")
    }

    fn on_worker<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(body)
            .unwrap()
            .join()
            .unwrap()
    }

    #[test]
    fn an_admitted_candidate_runs_once() {
        on_worker(|| {
            let candidate = "set y 3\nputs $y\nputs {a b}\n";
            let vm = private(candidate, &["puts", "set"], &["y"]);
            // Admission already proves every PUSH literal under the closure.
            let prepared = vm.prepare(candidate).expect("admitted");
            // A plain top-level module carries only its structural labels.
            for labels in [
                &prepared.module().top_level.labels,
                &prepared.module().top_level_body.labels,
            ] {
                let mut kinds = labels
                    .keys()
                    .map(|label| label.trim_end_matches(|c: char| c.is_ascii_digit()))
                    .collect::<Vec<_>>();
                kinds.sort_unstable();
                kinds.dedup();
                assert_eq!(kinds, ["cmd_end_", "entry_", "exit_"]);
            }
            let transcript = vm.execute(prepared).expect("executed");
            assert_eq!(transcript.stdout, b"3\na b\n");
        });
    }

    #[test]
    fn a_registry_without_the_plan_key_is_refused() {
        on_worker(|| {
            let stock = registry();
            let foreign = CommandRegistry::build_default().snapshot().semantic_key();
            assert_eq!(
                PrivateApplicationVm::new(&stock, &foreign, "puts 1", vec!["puts".into()], vec![])
                    .err(),
                Some(ClosedSourceRefusal::Currency)
            );
        });
    }

    #[test]
    fn a_compiler_replaced_between_prepare_and_execute_is_refused() {
        on_worker(|| {
            let mut vm = private("puts 1", &["puts"], &[]);
            let prepared = vm.prepare("puts 1").expect("admitted");
            let registry = registry();
            vm.vm
                .set_compiler(Box::new(ApplicationInputCompileService::new(
                    profile(),
                    registry,
                    "puts 1",
                )));
            assert_eq!(
                vm.execute(prepared).err(),
                Some(ClosedSourceRefusal::Currency)
            );
        });
    }

    #[test]
    fn other_source_and_unlisted_commands_are_refused() {
        on_worker(|| {
            let vm = private("puts 1", &["puts"], &[]);
            assert_eq!(
                vm.prepare("puts 2").err(),
                Some(ClosedSourceRefusal::Source)
            );
            let vm = private("set y 1", &["puts"], &[]);
            let prepared = vm
                .prepare("set y 1")
                .expect("compiles; set is not retained");
            assert_eq!(
                vm.execute(prepared).err(),
                Some(ClosedSourceRefusal::Execution)
            );
        });
    }

    #[test]
    fn a_caught_limit_breach_and_a_long_value_are_refused() {
        on_worker(|| {
            let long = format!("puts {}", "x".repeat(300));
            let vm = private(&long, &["puts"], &[]);
            let prepared = vm.prepare(&long).expect("admitted");
            assert_eq!(
                vm.execute(prepared).err(),
                Some(ClosedSourceRefusal::Execution)
            );
            let many = "puts 1\n".repeat(300);
            let vm = private(&many, &["puts"], &[]);
            let prepared = vm.prepare(&many).expect("admitted");
            assert_eq!(
                vm.execute(prepared).err(),
                Some(ClosedSourceRefusal::Execution)
            );
        });
    }
}
