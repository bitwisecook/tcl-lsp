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

//! Interpreters over an explicit release's native core, with that release's
//! source compiler installed, built through `tcl-vm`'s public constructor.

use std::io::Write;
use std::rc::Rc;

use tcl_vm::{Vm, host_native::NativeHost};

/// A native core of `profile` with no compiler, writing output to `output`.
pub fn core_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    Vm::with_native_core(
        output,
        Rc::new(NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("an actual native core for the profile")
}

/// [`core`] with `profile`'s source compiler installed.
pub fn interpreter(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    interpreter_with_output(profile, Box::new(std::io::sink()))
}

/// [`core_with_output`] with `profile`'s source compiler installed.
pub fn interpreter_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    let mut vm = core_with_output(profile, output);
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm
}
