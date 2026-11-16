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
use crate::loader::{FileBom, statements};

#[test]
fn static_speclib_body_folds_brace_continuations_like_interpreter() {
    let source = "speclib demo 2.0 {command demo {hover -summary {before\\\n  after}; arity 1}}";
    assert!(drive_file_statically(source, None, true).is_some());
    let static_pack = evaluate_pack_in(source, &EvalOptions::default(), None);
    let interpreter_pack = evaluate_pack_in(
        source,
        &EvalOptions {
            static_fast_path: false,
            ..EvalOptions::default()
        },
        None,
    );
    assert_eq!(
        crate::golden::render(&static_pack),
        crate::golden::render(&interpreter_pack)
    );
    assert_eq!(
        static_pack.commands[0].line,
        interpreter_pack.commands[0].line
    );
    assert!(
        drive_file_statically(
            "speclib demo 2.0 {set name demo; command $name {arity 1}}",
            None,
            true,
        )
        .is_none()
    );
}

#[test]
fn original_static_rows_keep_rooted_and_tcl_namespace_heads_on_the_interpreter_path() {
    for (head, general) in [
        ("tcl", false),
        ("tcl:", false),
        ("tcl::", true),
        ("tcl:::tail", true),
        ("tcl::nested::tail", true),
        ("TCL::tail", false),
        ("tcl_data::tail", false),
        ("tcl\0::tail", false),
        ("\0tcl::tail", false),
        ("tcl::\0tail", true),
        ("::\0tail", true),
        (":::", true),
    ] {
        assert_eq!(is_general_tcl(head), general, "{head:?}");
    }
    for source in [
        "::command p {}",
        ":::command p {}",
        ":: p {}",
        "tcl::command p {}",
        "tcl::::command p {}",
        "tcl::nested::command p {}",
        "set name VALUE",
    ] {
        let rows = statements(source, 1, FileBom::Content);
        assert_eq!(rows.len(), 1, "{source}");
        assert!(!static_stmt(&rows[0]), "{source}");
    }
    for source in ["command p {}", "tcl:command p {}", "atcl::command p {}"] {
        let rows = statements(source, 1, FileBom::Content);
        assert_eq!(rows.len(), 1, "{source}");
        assert!(static_stmt(&rows[0]), "{source}");
    }
}
