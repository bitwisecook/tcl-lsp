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

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn real_tcl_oracle_distinguishes_default_and_level_zero_return_codes() {
    let script = r"
            proc probe {spec} {
                set log {}
                try { return {*}$spec payload } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_code {code} {
                set log {}
                try { return -code $code payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_options {options} {
                set log {}
                try { return -options $options payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_source_order {} {
                set log {}
                try { return -options {-code error -level 0 -errorcode {X Y}} payload } \
                    trap {X} {message options} { lappend log broad-trap } \
                    trap {X Y} {message options} { lappend log overlapping-trap } \
                    on error {message options} { lappend log first-error } \
                    on error {message options} { lappend log duplicate-error } \
                    finally { lappend log finally }
                return $log
            }
            proc probe_custom_code {code} {
                set log {}
                try { return -options [list -code $code -level 0] payload } \
                    on 42 {message options} { lappend log first-42 } \
                    on 42 {message options} { lappend log duplicate-42 } \
                    on 43 {message options} { lappend log code-43 } \
                    on -1 {message options} { lappend log minus-one } \
                    on error {message options} { lappend log symbolic-error } \
                    finally { lappend log finally }
                return $log
            }
            puts [probe {-code error}]
            puts [probe {-level 0 -code error}]
            puts [probe {-foo bar}]
            puts [probe {-c error}]
            puts [probe {-level 0x0 -code error}]
            puts [probe_dynamic_code error]
            puts [probe_dynamic_code bogus]
            foreach options {
                {-code ok -level 0}
                {-code error -level 0}
                {-code return -level 0}
                {-code break -level 0}
                {-code continue -level 0}
            } { puts [probe_dynamic_options $options] }
            puts [probe_source_order]
            puts [probe_custom_code 42]
            puts [probe_custom_code 43]
            puts [probe_custom_code 4294967295]
            puts [probe_custom_code error]
        ";
    let oracle = tcl_host_native::executable::which("tclsh").expect("tclsh oracle available");
    let mut child = Command::new(oracle)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tclsh oracle available");
    let stdin = child.stdin.as_mut().expect("oracle stdin");
    stdin
        .write_all(script.as_bytes())
        .expect("write oracle script");
    let output = child.wait_with_output().expect("wait for tclsh oracle");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("oracle stdout"),
        concat!(
            "return finally after\nerror finally after\nreturn finally after\n",
            "return finally after\nerror finally after\n",
            "return finally after\nerror finally after\n",
            "ok finally after\nerror finally after\nreturn finally after\n",
            "break finally after\ncontinue finally after\n",
            "broad-trap finally\n",
            "first-42 finally\ncode-43 finally\nminus-one finally\nsymbolic-error finally\n",
        )
    );
}
