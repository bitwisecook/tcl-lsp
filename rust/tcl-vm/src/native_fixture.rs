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

//! Actual per-profile native core entry for original-object conformance tests.

use std::io::Write;
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
    core_with_output_and_host(profile, Box::new(std::io::sink()), host)
}

fn core_with_output_and_host(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
    host: Rc<dyn tcl_platform::Host>,
) -> Vm {
    Vm::with_native_core(
        output,
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
    interpreter_with_output(profile, Box::new(std::io::sink()))
}

/// Retain captured output while installing the actual profile's source compiler.
pub(crate) fn interpreter_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    let mut vm = core_with_output_and_host(profile, output, Rc::new(NativeHost::new()));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm
}

/// Retain an actual native core and source compiler, then load exactly the
/// independently selected distribution libraries. An empty selection remains
/// a core-and-compiler fixture; no catalogue or process-wide default supplies
/// additional libraries.
pub(crate) fn interpreter_with_scripted_libraries(
    profile: &'static tcl_dialect::DialectProfile,
    libraries: &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary],
) -> Vm {
    let mut vm = interpreter(profile);
    for &library in libraries {
        vm.install_scripted_library(library);
    }
    vm
}

/// Select only the shared scripted dictionary library required by distribution
/// specimens. Native core observers use `core` or `interpreter` independently.
pub(crate) fn interpreter_with_dictionary_library(
    profile: &'static tcl_dialect::DialectProfile,
) -> Vm {
    interpreter_with_scripted_libraries(
        profile,
        &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::Dictionary],
    )
}
