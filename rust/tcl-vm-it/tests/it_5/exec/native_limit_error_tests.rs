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

use crate::prelude::*;

fn actual(profile: &str) -> Vm {
    crate::fixture::interpreter(tcl_dialect::DialectProfile::find(profile).unwrap())
}

#[test]
fn an_exceeded_limit_publishes_its_result_before_the_error_frames() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(profile);
        let completion = vm
            .try_eval_source(
                "interp create c
                c eval {proc budget_loop {} {while {1} {incr x}}}
                interp limit c commands -value 1000
                list [catch {c eval budget_loop} message options] $message [dict get $options -errorinfo]",
            )
            .unwrap();
        assert_eq!(completion.code, Code::Ok, "{profile}");
        let fields = ValueOps::list_elements(&mut vm, &completion.result).unwrap();
        assert_eq!(fields[0].string_bytes().as_ref(), b"1", "{profile}");
        assert_eq!(
            fields[1].string_bytes().as_ref(),
            b"command count limit exceeded",
            "{profile}"
        );
        // tclsh 8.6.18 and 9.0.4 report these frames, then a last
        // `invoked from within "c eval budget_loop"` the VM does not add.
        let info = fields[2].string_bytes();
        assert!(
            info.starts_with(b"command count limit exceeded\n    while executing\n\"incr x\"\n    (procedure \"budget_loop\" line 1)\n    invoked from within\n\"budget_loop\""),
            "{profile}: {}",
            String::from_utf8_lossy(&info)
        );
    }
}

#[test]
fn an_exceeded_limit_does_not_replace_an_existing_owner_refusal() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(profile);
        let _ = vm.refuse_host_command("original owner refusal".into());
        vm.set_command_limit(Some(0));
        let error = vm.try_eval_source("set forbidden_effect yes").unwrap_err();
        assert!(
            error.to_string().contains("original owner refusal"),
            "{profile}: {error}"
        );
    }
}
