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

//! Native numeric sequence arguments and result generation over the shared core.

use crate::command::{completion_from_cmd_error, native_wrong_args};
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_cmd_core::lseq::{self, LseqError};
use tcl_runtime_api::Completion;

/// Register the native numeric sequence command.
pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("lseq", cmd_lseq);
}

fn cmd_lseq(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let bytes = args.iter().map(Value::string_bytes).collect::<Vec<_>>();
    let refs = bytes
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<_>>();
    let plan = match lseq::decode(&refs) {
        Ok(plan) => plan,
        Err(LseqError::WrongArguments) => {
            return native_wrong_args(vm, "lseq n ??op? n ??by? n??");
        }
        Err(LseqError::Command(error)) => return completion_from_cmd_error(vm, error),
    };
    match lseq::generate(vm, &plan) {
        Ok(value) => ok(value),
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_backend_capacity_bypasses_guest_capture_and_finally() {
        for script in [
            "set prior BEFORE; catch {lseq 100000001} captured options; set after YES",
            "set prior BEFORE; try {lseq 100000001} finally {set final YES}; set after YES",
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            );
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::default(),
            ));
            let error = vm.try_eval_source(script).unwrap_err();
            let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(refusal) = error else {
                panic!("expected reached materialization refusal: {error:?}");
            };
            assert_eq!(
                refusal.reason,
                "host cannot materialize 100000001 elements; backend limit is 100000000"
            );
            assert_eq!(
                vm.get_var("prior").unwrap().string_bytes().as_ref(),
                b"BEFORE"
            );
            for name in ["captured", "options", "final", "after"] {
                assert!(vm.get_var(name).is_none());
            }
        }
    }
}
