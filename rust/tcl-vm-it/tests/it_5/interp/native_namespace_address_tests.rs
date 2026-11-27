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

//! `namespace eval` and `namespace exists` on the empty and root names, from
//! the global namespace and a child, against tclsh.

use crate::prelude::*;

fn compare_public_rows(engine: &str, observation: &str) {
    let rows = tcl_syntax::list::split_list(observation).unwrap();
    assert_eq!(rows.len(), 18 * 5);
    let mut compared = 0;
    let mut excluded = Vec::new();
    for row in rows.as_chunks::<5>().0 {
        let id = tcl_syntax::list::split_list(&row[0]).unwrap();
        let identity = tcl_syntax::list::split_list(&id[0]).unwrap();
        let context = identity[0].as_ref();
        let name = match identity[1].as_ref() {
            "nul-suffix" => {
                excluded.push(row[0].to_string());
                continue;
            }
            "empty" => "{}",
            "absolute-root" => "::",
            other => panic!("unrecognised oracle variant {other}"),
        };
        let command = match id[1].as_ref() {
            "eval" => format!("namespace eval {name} {{namespace current}}"),
            "exists-before" | "exists-after" => format!("namespace exists {name}"),
            other => panic!("unrecognised oracle operation {other}"),
        };
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let completion = crate::fixture::eval(
            &mut vm,
            &format!(
                "namespace eval {context} {{}}\n\
                 list [namespace eval {context} {{catch {{{command}}} ::value}}] $::value [if {{[info exists ::errorCode]}} {{set ::errorCode}}]"
            ),
        );
        assert_eq!(completion.code, Code::Ok, "{engine}: {}", row[0]);
        let columns = completion.result.as_list().unwrap();
        assert_eq!(
            columns[0].to_str().as_ref(),
            row[1].as_ref(),
            "{engine}: {}",
            row[0]
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &columns[1]),
            row[2].as_bytes(),
            "{engine}: {}",
            row[0]
        );
        if row[1] != "0" {
            assert_eq!(
                crate::fixture::bytes(&vm, &columns[2]),
                row[3].as_bytes(),
                "{engine}: {}",
                row[0]
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 12);
    assert_eq!(excluded.len(), 6);
}

macro_rules! public_rows {
    ($name:ident, $engine:literal, $release:literal) => {
        #[test]
        fn $name() {
            compare_public_rows(
                $engine,
                include_str!(concat!(
                    "../../../../../experiments/name-handling/namespace-address-purpose/",
                    $release,
                    "/stdout.bin"
                )),
            );
        }
    };
}

public_rows!(original_namespace_address_c84, "tcl8.4", "8.4.20");
public_rows!(original_namespace_address_c85, "tcl8.5", "8.5.19");
public_rows!(original_namespace_address_c86, "tcl8.6", "8.6.18");
public_rows!(original_namespace_address_c90, "tcl9.0", "9.0.4");
public_rows!(original_namespace_address_c91, "tcl9.1", "9.1.0");
