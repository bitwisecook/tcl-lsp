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

//! Jim's automatic error frames after a cold rename, against jimsh 0.84.

use crate::prelude::*;
use std::cell::RefCell;

#[test]
fn automatic_error_frames_follow_original_cold_rename_and_selected_key_lifetimes() {
    struct Output(Rc<RefCell<Vec<u8>>>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let cases: &[(&str, &[u8])] = &[
        (
            "proc P {n} {if {$n} {rename P Q; Q 0; error fail}; return ok}\nset c [catch {P 1} result options]\nputs [list $c $result [dict get $options -errorinfo]]\n",
            b"1 fail {Q stdin 1 {error fail} {} stdin 2 {P 1}}\n",
        ),
        (
            "proc P {} {rename eval {}; error fail}\nset c [catch {eval {P}} result options]\nputs [list $c $result [dict get $options -errorinfo]]\n",
            b"1 fail {P stdin 1 {error fail} {} stdin 2 P}\n",
        ),
    ];
    let profile = tcl_vm::test_support::profile("jim");
    for &(source, expected) in cases {
        let output = Rc::new(RefCell::new(Vec::new()));
        let mut vm =
            crate::fixture::interpreter_with_output(profile, Box::new(Output(Rc::clone(&output))));
        let completion = vm
            .try_eval_source_at(
                source,
                tcl_runtime_api::script_source_location::ScriptSourceLocation {
                    file: "stdin".into(),
                    line: 1,
                },
            )
            .expect("original actual Jim source entry");
        assert!(
            completion.code.is_ok(),
            "{source}: {}",
            completion.result.to_str()
        );
        assert_eq!(output.borrow().as_slice(), expected, "{source}");
    }
}
