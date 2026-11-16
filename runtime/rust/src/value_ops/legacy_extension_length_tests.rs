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
use std::cell::Cell;
use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; }
extern "C" fn update(original: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    unsafe { obj::set_native_updater_string_rep(original, b"one two", false) };
}
static LEGACY: obj::TclObjType = obj::TclObjType {
    name: c"registeredLegacyLength".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
static UNREGISTERED: obj::TclObjType = obj::TclObjType {
    name: c"unregisteredLength".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
#[test]
fn registered_legacy_length_converts_original_without_admitting_unknown_descriptors() {
    obj::register_legacy_extension_type(&LEGACY);
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        READS.with(|reads| reads.set(0));
        let mut actual = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let legacy = obj::Owned::fresh(obj::alloc_typed(&LEGACY, 0));
        let unknown = obj::Owned::fresh(obj::alloc_typed(&UNREGISTERED, 0));
        assert_eq!(
            obj::stock_list_input_class(legacy.as_ptr()),
            Class::LegacyExtension
        );
        assert_eq!(
            obj::stock_list_input_class(unknown.as_ptr()),
            Class::Unknown
        );
        assert_eq!(actual.list_len(&legacy.as_ptr()).unwrap(), 2, "{profile}");
        assert_eq!(READS.with(Cell::get), 1);
        assert!(actual.list_len(&unknown.as_ptr()).is_err());
        assert_eq!(READS.with(Cell::get), 1);
        assert!(!obj::has_string_rep(unknown.as_ptr()));
    }
}
