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

//! Caught callback results follow the surviving cell, independently of formals.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn caught_callback_result_follows_replaced_formal_alias_native_controls() {
    // Native proof: naming.variable.catch-result-alias-continuity
    // docs/design/analysis/name-resolution-proofs/catch-result-alias-continuity.md
    // The script reports through `binary scan`; jimsh's `binary` is a C
    // extension the VM installs only through a private registration, so the
    // Jim capture is not compared here.
    const SOURCE: &[u8] =
        include_bytes!("../../../../tcl-vm/tests/data/native_catch_formal_alias/cases.tcl");
    for (engine, observed) in [
        (
            "tcl8.4",
            include_str!("../../../../tcl-vm/tests/data/native_catch_formal_alias/tcl8.4/stdout"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tcl-vm/tests/data/native_catch_formal_alias/tcl8.5/stdout"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tcl-vm/tests/data/native_catch_formal_alias/tcl8.6/stdout"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-vm/tests/data/native_catch_formal_alias/tcl9.0/stdout"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-vm/tests/data/native_catch_formal_alias/tcl9.1/stdout"),
        ),
    ] {
        let output = Output(Rc::new(RefCell::new(Vec::new())));
        let mut vm = crate::fixture::interpreter_with_output(
            tcl_vm::test_support::profile(engine),
            Box::new(output.clone()),
        );
        let result = vm.try_eval_source_bytes(SOURCE).unwrap();
        assert_eq!(
            result.code,
            tcl_runtime_api::Code::Ok,
            "{engine}: {result:?}"
        );
        let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
        let measured_rows = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("VERSION|"))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(measured_rows(&actual), measured_rows(observed), "{engine}");
    }
}
