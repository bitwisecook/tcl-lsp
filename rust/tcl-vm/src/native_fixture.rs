// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual per-profile native core entry for original-object conformance tests.

use std::rc::Rc;

use crate::{Vm, host_native::NativeHost};

/// Create the selected engine before root variables or command registrations.
/// This entry installs no source compiler, preserving that independent purpose.
pub(crate) fn core(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    core_with_host(profile, Rc::new(NativeHost::new()))
}

/// Retain an independently supplied native host capability at constructor entry.
pub(crate) fn core_with_host(
    profile: &'static tcl_dialect::DialectProfile,
    host: Rc<dyn tcl_platform::Host>,
) -> Vm {
    Vm::with_native_core(
        Box::new(std::io::sink()),
        host,
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("the original native fixture requires an actual core bootstrap issuer")
}

/// Create an actual native core and its separately selected source compiler.
pub(crate) fn interpreter(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    let mut vm = core(profile);
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm
}
