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

use super::*;
use std::{cell::RefCell, rc::Rc};

struct ReturnGeneratedObject {
    selected: usize,
    fail: bool,
    observed: Rc<RefCell<Vec<crate::NativeObjectLifetimeLease>>>,
}
impl crate::command::NativeCommand for ReturnGeneratedObject {
    fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        let words = vm
            .native_invocation
            .arguments
            .as_ref()
            .unwrap()
            .elements()
            .unwrap();
        *self.observed.borrow_mut() = words.iter().map(Value::native_lifetime_lease).collect();
        let mut result = ok(words[self.selected].native_lifetime_lease().into_value());
        if self.fail {
            result.code = Code::Error;
        }
        result
    }
}

#[test]
fn import_preload_retires_generated_arguments_before_resetting_the_unshared_result() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for selected in [0, 1] {
            for fail in [false, true] {
                let mut vm =
                    crate::native_fixture::core(crate::environment::profile_for_dialect(release));
                let observed = Rc::new(RefCell::new(Vec::new()));
                vm.register_native_command(
                    "auto_import",
                    Rc::new(ReturnGeneratedObject {
                        selected,
                        fail,
                        observed: Rc::clone(&observed),
                    }),
                );
                let scope =
                    native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
                let returned = vm
                    .original_c_import_preload(b"::missing::member", &scope)
                    .unwrap();
                let observed = observed.borrow();
                assert_eq!(observed.len(), 2);
                assert!(observed[selected].value().native_object_is_live());
                assert!(!observed[1 - selected].value().native_object_is_live());
                assert_eq!(
                    observed[selected].value().native_object_reference_count(),
                    1
                );
                assert!(
                    vm.with_native_interp_result(
                        |actual| actual.is_same_object(observed[selected].value())
                    )
                    .unwrap()
                );
                let expected = if fail {
                    if selected == 0 {
                        b"auto_import".as_slice()
                    } else {
                        b"::missing::member".as_slice()
                    }
                } else {
                    b"".as_slice()
                };
                assert_eq!(
                    observed[selected]
                        .value()
                        .resident_string_bytes()
                        .unwrap()
                        .as_ref(),
                    expected
                );
                if fail {
                    let returned = returned.unwrap();
                    assert_eq!(returned.code, Code::Error);
                    assert!(returned.result.is_same_object(observed[selected].value()));
                } else {
                    assert!(returned.is_none());
                    assert_eq!(observed[selected].value().native_object_type_name(), "none");
                }
                scope.finish(&mut vm).unwrap();
            }
        }
    }
}
