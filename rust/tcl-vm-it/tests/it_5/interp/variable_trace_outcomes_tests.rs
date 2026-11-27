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

//! A script write trace's error through `set`, and the `::errorInfo` it leaves.

use crate::prelude::*;

#[test]
fn original_script_trace_guest_keeps_its_existing_error_episode_through_presentation() {
    // tclsh 8.6.18 and 9.0.4 report `can't set "x": STOP` with an
    // `::errorInfo` that starts at the trace's `SCRIPT_MARKER`.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(profile));
        let completion = crate::fixture::eval(
            &mut vm,
            "proc original_trace_stop {args} {error STOP SCRIPT_MARKER}; trace add variable x write original_trace_stop; list [catch {set x changed} m] $m $::errorInfo",
        );
        assert_eq!(completion.code, Code::Ok, "{profile}");
        let fields = completion.result.as_list().unwrap();
        assert_eq!(fields[0].to_str().as_ref(), "1", "{profile}");
        assert_eq!(fields[1].to_str().as_ref(), "can't set \"x\": STOP");
        let info = fields[2].to_str();
        assert!(info.starts_with("SCRIPT_MARKER"), "{profile}: {info}");
        assert!(info.contains("(write trace on \"x\")"), "{profile}");
    }
}
