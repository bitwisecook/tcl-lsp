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

//! Minimal JSON dumper for the differential test harness.
//!
//! Reads Tcl source (from `--source <text>`, a file path argument, or
//! stdin when neither is given) and prints `serialise_result` as JSON.
//! The differential test harness shells out to this and compares
//! the output key-by-key.
//!
//! It deliberately has no dependencies beyond `std`.

use std::io::Read;

fn main() {
    let mut dialect = String::from("tcl8.6");
    let mut source: Option<String> = None;
    let mut path: Option<String> = None;
    // When set, dump `build_view(view, serialise_result)` instead of the
    // serialised result — used by the TUI-tree test harness.
    let mut view_tree: Option<String> = None;
    // When set, read an already-serialised result as JSON from stdin and dump
    // `build_view(view, that)` — isolates the tree builder from any
    // serialise divergence (both sides build from identical data).
    let mut view_tree_stdin: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dialect" => dialect = args.next().unwrap_or(dialect),
            "--source" => source = args.next(),
            "--view-tree" => view_tree = args.next(),
            "--view-tree-stdin" => view_tree_stdin = args.next(),
            other => path = Some(other.to_owned()),
        }
    }

    if let Some(view) = view_tree_stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        let data: serde_json::Value = serde_json::from_str(&buf).expect("valid JSON on stdin");
        println!(
            "{}",
            serde_json::to_string(&tcl_explorer::build_view(&view, &data)).expect("serialise")
        );
        return;
    }

    let source = source.unwrap_or_else(|| {
        if let Some(p) = path {
            std::fs::read_to_string(&p).unwrap_or_else(|e| {
                eprintln!("error reading {p}: {e}");
                std::process::exit(2);
            })
        } else {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok();
            buf
        }
    });

    let result = tcl_explorer::run_pipeline(&source, &dialect);
    let value = tcl_explorer::serialise_result(&result);

    let out = if let Some(view) = view_tree {
        serde_json::to_string(&tcl_explorer::build_view(&view, &value))
    } else {
        serde_json::to_string(&value)
    };
    println!("{}", out.expect("serialise to JSON"));
}
