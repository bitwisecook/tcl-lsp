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

use std::cell::Cell;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion};
use tcl_vm::{NativeCommand, Value, Vm};

#[test]
fn original_trace_procedure_heads_remain_live_during_their_actual_activation() {
    // Source contract: naming.error.original-invocation-context-capture
    // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
    struct Inspect(Rc<Cell<usize>>);
    impl NativeCommand for Inspect {
        fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
            let caller = vm.try_eval_source("info level 0").unwrap();
            assert_eq!(caller.result.to_str().as_ref(), "observer d {} read");
            self.0.set(self.0.get() + 1);
            Completion::new(Code::Ok, Value::empty(), Value::empty())
        }
    }
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let count = Rc::new(Cell::new(0));
        vm.register_native_command("inspect", Rc::new(Inspect(Rc::clone(&count))));
        let completion = vm
            .try_eval_source(
                "proc observer {n1 n2 op} {inspect; error READ_FAIL}; proc work {} {set d {k 3}; trace add variable d read observer; catch {dict lappend d k Y Z} m; trace remove variable d read observer; return $m}; work",
            )
            .unwrap();
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            // tclsh 9.0.4 prints the same.
            "k {Y Z}",
            "{engine}"
        );
        assert_eq!(count.get(), 1, "{engine}");
    }
}
