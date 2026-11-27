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

/// [`core`] with exactly the selected scripted distribution libraries.
pub(crate) fn core_with_scripted_libraries(
    profile: &'static tcl_dialect::DialectProfile,
    libraries: &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary],
) -> Vm {
    let mut vm = core(profile);
    for &library in libraries {
        vm.install_scripted_library(library);
    }
    vm
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

/// Load every scripted distribution library, as the provider's own shell does.
pub(crate) fn install_distribution(vm: &mut Vm) {
    for &library in tcl_registry::native_scripted_distribution::NativeScriptedLibrary::ALL {
        vm.install_scripted_library(library);
    }
}
