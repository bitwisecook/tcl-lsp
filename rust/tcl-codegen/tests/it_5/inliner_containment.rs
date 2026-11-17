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

//! Inlined procedure bodies reach no shipped compile until they present errors
//! as the procedure call does.
//!
//! A spliced body runs in its caller's frame, so an error inside it logs no
//! `(procedure "NAME" line N)` frame and no `-errorstack` `CALL` entry, and its
//! renamed locals are visible to `info locals`. Until splice ranges re-invoke
//! the call on any abnormal exit, no production path may inline. Remove these
//! guards only when the live differential
//! (`tcl-spectcl` `codegen_stamps::live_differential`) passes with `errorInfo`,
//! `errorCode`, the errorstack and frame introspection compared on every
//! release.

use std::path::{Path, PathBuf};

const REMOVAL: &str = "inlining reaches a shipped compile before splice ranges present \
    errors as the call does; remove this guard only when the live differential compares \
    errorInfo, errorCode, the errorstack and frame introspection on every release";

fn production_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .expect("readable source tree")
        .flatten()
    {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if name != "tests" && name != "target" {
                production_sources(&path, found);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && name != "tests.rs"
            && !name.ends_with("_tests.rs")
        {
            found.push(path);
        }
    }
}

#[test]
fn no_production_source_calls_the_procedure_inliner() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&workspace).expect("workspace").flatten() {
        let src = entry.path().join("src");
        if src.is_dir() {
            production_sources(&src, &mut sources);
        }
    }
    let inliner = workspace.join("tcl-codegen/src/inlining");
    let callers: Vec<_> = sources
        .iter()
        .filter(|path| !path.starts_with(&inliner))
        .filter(|path| {
            std::fs::read_to_string(path).is_ok_and(|text| {
                text.lines()
                    .filter(|line| !line.trim_start().starts_with("//"))
                    .any(|line| line.contains("inline_module("))
            })
        })
        .collect();
    assert!(callers.is_empty(), "{REMOVAL}: {callers:?}");
}

#[test]
fn the_live_compile_splices_no_reference_body() {
    let live = tcl_codegen::compile_service::LIVE_REFERENCE_INLINING;
    assert!(!live, "{REMOVAL}");
}
