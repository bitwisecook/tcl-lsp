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

use crate::interp::{Code, Command, Interp};
use crate::{counters, list, obj};

thread_local! {
    static WHICH_NAMESPACE: std::cell::Cell<*mut obj::TclObj> = const {
        std::cell::Cell::new(std::ptr::null_mut())
    };
    static WHICH_NAME_OBSERVATIONS: std::cell::RefCell<Vec<bool>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

mod code_and_inscope;
mod command_resolution;
mod deferred_deletion;
mod ensemble_dispatch;
mod ensemble_subcommand;
mod import_export;
mod teardown_order;
mod tree_navigation;
mod which_and_origin;

extern "C" fn update_which_probe_name(value: *mut obj::TclObj) {
    let resident = WHICH_NAMESPACE.with(|namespace| obj::has_string_rep(namespace.get()));
    WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().push(resident));
    let bytes: &[u8] = match obj::internal_rep(value) {
        0 => b"::absent",
        1 => b"absent",
        2 => b"::return",
        3 => b"return",
        _ => unreachable!("fixed original name control"),
    };
    // SAFETY: the updater owns the live original object's String slot.
    unsafe { obj::set_string_rep(value, bytes) };
}

static WHICH_PROBE_NAME: obj::TclObjType = obj::TclObjType {
    name: c"which-name-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_which_probe_name),
    set_from_any_proc: None,
};

fn leak_free(body: impl FnOnce(&mut Interp)) {
    leak_free_with(Interp::new, body);
}

fn leak_free_with(create: impl FnOnce() -> Interp, body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = create();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

fn native_namespace_interp(version: tcl_dialect::TclVersion) -> Interp {
    Interp::with_native_core(
        std::rc::Rc::new(tcl_host_native::NativeHost::new()),
        tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("selected native namespace core")
}

/// Run `sheet` and pin its result byte for byte. Every expectation below
/// was measured on tclsh 9.0.4 and is identical on 8.6.16.
fn pins(sheet: &[u8], want: &[u8]) {
    leak_free(|i| {
        let code = i.eval_str(sheet);
        let got = String::from_utf8_lossy(&i.result_bytes()).into_owned();
        assert_eq!(code, Code::Ok, "sheet failed: {got}");
        assert_eq!(
            got.as_bytes(),
            want,
            "sheet:\n{}",
            String::from_utf8_lossy(sheet)
        );
    });
}
