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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Value roles through the shared original registration worker.
use super::*;
use std::cell::Cell;
#[test]
fn original_literal_registration_checks_episode_inside_same_value_string_worker() {

    // Software node-aware callbacks and Value roles, no external Native chronology.
    for mutate in [false, true] {
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let key = GlobalKey {
            protocol,
            namespace: None,
            original: b"original registered".to_vec(),
        };
        let (index, first) =
            NativeLiteralWorld::register(&world, key.clone(), protocol, None).unwrap();
        assert_eq!(first.native_object_reference_count(), 2);
        let calls = Cell::new(0);
        let check = || Ok(());
        let refuse = |_| NativeLiteralUnavailable::unavailable("original registration callback");
        let string =
            |value: &Value,
             selected: NativeStringProtocol,
             current: &(dyn Fn() -> Result<(), NativeLiteralUnavailable> + '_)| {
                value
                    .native_string_bytes_checked_for_value(selected, None, &mut |original| {
                        if calls.get() == 0 {
                            assert!(world.try_borrow_mut().is_ok());
                            assert_eq!(original.native_object_reference_count(), 2);
                            if mutate {
                                let nested = GlobalKey {
                                    protocol,
                                    namespace: None,
                                    original: b"nested".to_vec(),
                                };
                                let (nested_index, nested_value) =
                                    NativeLiteralWorld::register(&world, nested, protocol, None)?;
                                let retired = world.borrow_mut().release(nested_index);
                                drop(retired);
                                drop(nested_value);
                            }
                        }
                        calls.set(calls.get() + 1);
                        current()?;
                        original
                            .check_native_header()
                            .map_err(NativeLiteralUnavailable::from)
                    })
                    .map(|bytes| bytes.to_vec())
            };
        let callbacks = OriginalLiteralCallbacks {
            check: &check,
            string: &string,
            refuse: &refuse,
        };
        let reached = NativeLiteralWorld::register(&world, key, protocol, Some(&callbacks));
        assert!(calls.get() > 0);
        if mutate {
            assert!(reached.is_err());
            assert_eq!(first.native_object_reference_count(), 2);
            assert_eq!(
                world.borrow().shared.borrow().registration_owners(index),
                Some(1)
            );
        } else {
            let (second_index, second) = reached.unwrap();
            assert_eq!(second_index, index);
            assert!(second.is_same_object(&first));
            assert_eq!(first.native_object_reference_count(), 3);
            let retired = world.borrow_mut().release(second_index);
            drop(retired);
            drop(second);
        }
        let lease = first.native_lifetime_lease();
        let retired = world.borrow_mut().release(index);
        drop(retired);
        drop(first);
        assert!(!lease.value().native_object_is_live());
    }
}
