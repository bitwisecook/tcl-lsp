// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual private-host engine construction, before bootstrap or hook parsing.
//!
//! This is a private constructor slice, not a closed-consumer admission. Its
//! eventual application owner must retain the host and check its state around
//! each fold; compiler, literal, nested-code and retirement checks remain.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_platform::{NativeCIntegerAbi, NumericEnvironment, NumericErrorState};
use tcl_registry::{CommandRegistry, InvocationDialect};
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;
use tcl_vm::Vm;

use super::{MemoryHost, MemoryWriter};
use crate::TclVmEngine;

/// Opaque retained actual memory-host owner. It exposes no VM, values, Host,
/// compiler or closed-consumer scope. The application retains it through the
/// evaluator's final retirement and checks it around every selected fold.
pub struct PrivateApplicationHookHost {
    host: Rc<MemoryHost>,
    numeric: NumericErrorState,
    abi: NativeCIntegerAbi,
}

/// Restrictive construction plumbing for the private application evaluator.
/// No caller Host, Write, VM, CompileService or callback enters construction.
/// This creates no closed-consumer permission; the joined application backend
/// independently owns and validates the entire source/evaluator footprint.
///
/// # Errors
/// Refuses unsupported native profiles, unavailable bootstrap, nonquiet private
/// I/O or unavailable/currently erroneous numeric environment.
pub fn private_application_hook_engine(
    version: tcl_dialect::TclVersion,
    pinned: bool,
) -> Result<(TclVmEngine, PrivateApplicationHookHost), ClosedSourceRefusal> {
    let _grammar = crate::GrammarGuard::keep();
    if !matches!(
        version,
        tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1
    ) {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    let profile =
        tcl_registry::model::ingress::resolve_known_environment(version.dialect_profile_name())
            .and_then(|environment| environment.catalogue_profile())
            .filter(|profile| InvocationDialect::of_profile(profile).tcl_version == Some(version))
            .ok_or(ClosedSourceRefusal::NativeContext)?;
    let host = Rc::new(MemoryHost::new());
    let mut vm = Vm::with_native_core(
        Box::new(MemoryWriter(Rc::clone(&host.sinks))),
        Rc::clone(&host) as Rc<dyn tcl_platform::Host>,
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .ok_or(ClosedSourceRefusal::NativeContext)?;
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::new(registry)));
    let engine = TclVmEngine {
        vm,
        budget: tcl_engine_api::Budget::default(),
        release: pinned.then_some(profile.name),
        units: 0,
        host_commands: Rc::new(RefCell::new(Vec::new())),
        unit_commands: Vec::new(),
    };
    let numeric = host
        .state()
        .map_err(|_| ClosedSourceRefusal::NativeContext)?;
    if numeric.range_error || numeric.domain_error {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    let abi = host
        .c_integer_abi()
        .map_err(|_| ClosedSourceRefusal::NativeContext)?;
    let retained = PrivateApplicationHookHost { host, numeric, abi };
    retained.check_quiet_host()?;
    Ok((engine, retained))
}

impl PrivateApplicationHookHost {
    /// Check the retained actual memory I/O and numerical context without
    /// clearing errors, replacing the Host or manufacturing a fresh snapshot.
    ///
    /// # Errors
    /// Refuses busy/nonquiet sinks or changed/unavailable numerical state/ABI.
    pub fn check_quiet_host(&self) -> Result<(), ClosedSourceRefusal> {
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
        if !quiet
            || !empty_environment
            || self
                .host
                .state()
                .map_err(|_| ClosedSourceRefusal::NativeContext)?
                != self.numeric
            || self
                .host
                .c_integer_abi()
                .map_err(|_| ClosedSourceRefusal::NativeContext)?
                != self.abi
        {
            return Err(ClosedSourceRefusal::Execution);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_hook_bootstrap_retains_actual_native_release_and_memory_output() {
        std::thread::spawn(|| {
            let (engine, retained) =
                private_application_hook_engine(tcl_dialect::TclVersion::V9_1, true)
                    .expect("private C9.1 bootstrap");
            assert_eq!(engine.vm.runtime_version(), tcl_dialect::TclVersion::V9_1);
            assert_eq!(engine.release(), Some("tcl9.1"));
            assert!(retained.host.environment.borrow().is_empty());
            assert!(retained.check_quiet_host().is_ok());
            tcl_platform::StdIo::write_stdout(retained.host.as_ref(), b"private-only");
            assert_eq!(
                retained.host.sinks.borrow().stdout.as_slice(),
                b"private-only"
            );
            assert_eq!(
                retained.check_quiet_host(),
                Err(ClosedSourceRefusal::Execution)
            );
            drop(engine);
            drop(retained);
        })
        .join()
        .expect("private hook owner joined");
    }
}
