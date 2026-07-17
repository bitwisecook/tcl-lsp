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

//! End-to-end coverage for the issue #945 resolution-model follow-up:
//! flow-sensitive constant dispatch with writable provenance (faults 1–2),
//! multi-seed source views (fault 3), TclOO export visibility + dispatch
//! entry + per-object binding identity (faults 4–6), the interpreter
//! domain model (faults 7–8), and probe references (fault 9).
//!
//! Rename outputs are **executed** under a real tclsh where one is
//! available (`TCL_LSP_TCLSH`, else `PATH` probes) — the issue's
//! validation bar is behaviour, not edit ranges.

use serde_json::Value;

use crate::common::helpers::{locations, rename_edits, start_lines};
use crate::common::{Lsp, unique_uri};

/// Find a tclsh to execute transformed rename output under; `None` skips
/// the execution leg (mirrors `command_resolution_conformance`).
fn find_tclsh() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("TCL_LSP_TCLSH") {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
    for name in ["tclsh9.0", "tclsh8.6", "tclsh"] {
        if let Ok(out) = std::process::Command::new(name).arg("--version").output()
            && (out.status.success() || !out.stderr.is_empty() || !out.stdout.is_empty())
        {
            return Some(std::path::PathBuf::from(name));
        }
    }
    None
}

/// Apply an LSP edit array (single-line edits) to `src`.
fn apply_lsp_edits(src: &str, edits: &[Value]) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_owned).collect();
    let mut sorted: Vec<&Value> = edits.iter().collect();
    let pos = |e: &Value, key: &str| -> (usize, usize) {
        let p = &e["range"][key];
        (
            usize::try_from(p["line"].as_u64().unwrap()).unwrap(),
            usize::try_from(p["character"].as_u64().unwrap()).unwrap(),
        )
    };
    sorted.sort_by_key(|e| std::cmp::Reverse(pos(e, "start")));
    for e in sorted {
        let (sl, sc) = pos(e, "start");
        let (el, ec) = pos(e, "end");
        assert_eq!(sl, el, "single-line edits expected in these fixtures");
        lines[sl].replace_range(sc..ec, e["newText"].as_str().unwrap());
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Run `script` under the discovered tclsh, returning trimmed stdout;
/// panics on a non-zero exit.  `None` when no tclsh is installed.
fn run_tclsh(script: &str) -> Option<String> {
    use std::io::Write;
    let tclsh = find_tclsh()?;
    let mut child = std::process::Command::new(&tclsh)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn tclsh");
    child
        .stdin
        .as_mut()
        .expect("tclsh stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let out = child.wait_with_output().expect("tclsh run");
    assert!(
        out.status.success(),
        "script must execute cleanly under {}:\n{script}\nstderr: {}",
        tclsh.display(),
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// -- faults 1–2: flow-sensitive constant dispatch with provenance --------

#[test]
fn const_dispatch_rename_rewrites_the_defining_literal_and_executes_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\ntarget\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "renamed");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("set cmd renamed"),
        "the defining literal follows the rename:\n{renamed_src}"
    );
    assert!(
        renamed_src.contains("$cmd"),
        "the `$cmd` head is never rewritten:\n{renamed_src}"
    );
    // The issue's validation bar: the transformed output must EXECUTE —
    // the old edit set left `set cmd target` stale and died with
    // `invalid command name "target"` under tclsh 9.0.4.
    let script = format!("{renamed_src}puts [$cmd]\n");
    if let Some(out) = run_tclsh(&script) {
        assert_eq!(out, "hi", "the dispatch still reaches the renamed proc");
    } else {
        eprintln!("skipping tclsh execution leg: no tclsh (set TCL_LSP_TCLSH)");
    }
}

#[test]
fn branch_joined_dispatch_definition_offers_both_targets_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc foo {} {return one}\nproc bar {} {return two}\nset cmd foo\nif {$runtime} {\n    set cmd bar\n}\n$cmd\n";
    lsp.open_ready(&uri, src);
    // References from each proc's declaration include the `$cmd` join site
    // (line 6) and that proc's own contributing literal.
    let foo_refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(
        foo_refs.contains(&2) && foo_refs.contains(&6),
        "foo: literal (2) + join dispatch (6) expected: {foo_refs:?}"
    );
    let bar_refs = start_lines(&lsp.references(&uri, 1, 6, true));
    assert!(
        bar_refs.contains(&4) && bar_refs.contains(&6),
        "bar: literal (4) + join dispatch (6) expected: {bar_refs:?}"
    );
    // Renaming `bar` rewrites only bar's contributing literal; foo's stays,
    // and the renamed script still dispatches correctly on BOTH paths.
    let result = lsp.rename(&uri, 1, 6, "baz");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("set cmd foo") && renamed_src.contains("set cmd baz"),
        "only bar's literal follows its rename:\n{renamed_src}"
    );
    for (runtime, want) in [("0", "one"), ("1", "two")] {
        let script = format!("set runtime {runtime}\n{renamed_src}puts [$cmd]\n");
        if let Some(out) = run_tclsh(&script) {
            assert_eq!(out, want, "runtime={runtime} dispatch after rename");
        }
    }
}

// -- fault 3: multi-seed source views ------------------------------------

#[test]
fn multi_seeded_source_declaration_unions_every_view_945() {
    let mut lsp = Lsp::tcl();
    let b = unique_uri("tcl");
    let b_name = b.rsplit('/').next().unwrap().to_owned();
    lsp.open_ready(&b, "proc helper {} {namespace current}\n");
    let a = unique_uri("tcl");
    let a_src = format!(
        "namespace eval ::x {{ source {b_name} }}\nnamespace eval ::y {{ source {b_name} }}\n::x::helper\n::y::helper\n"
    );
    lsp.open_ready(&a, &a_src);
    // Declaration-side references from b.tcl reach BOTH runtime identities'
    // callers (tclsh 9.0.4: both ::x::helper and ::y::helper exist).
    let refs = locations(&lsp.references(&b, 0, 6, false));
    let a_lines: std::collections::BTreeSet<i64> = refs
        .iter()
        .filter(|l| l.uri == a)
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect();
    assert!(
        a_lines.contains(&2) && a_lines.contains(&3),
        "both seeded views' callers must surface: {a_lines:?}"
    );
}

// -- faults 4–6: TclOO visibility, dispatch entry, binding identity ------

#[test]
fn unexported_method_is_not_externally_resolvable_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Vault {\n    method _secret {} { return hidden }\n    method probe {} { return [my _secret] }\n}\nset v [Vault new]\n$v _secret\n";
    lsp.open_ready(&uri, src);
    // tclsh 9.0.4: `$v _secret` → `unknown method "_secret"` — external
    // navigation resolves nothing.
    let external = lsp.definition(&uri, 5, 5);
    assert!(
        start_lines(&external).is_empty(),
        "an externally unexported method is not callable: {external:?}"
    );
    // `my _secret` (internal) resolves the declaration.
    let internal: Vec<i64> = start_lines(&lsp.definition(&uri, 2, 34))
        .into_iter()
        .collect();
    assert_eq!(
        internal,
        vec![1],
        "`my _secret` reaches the unexported declaration: {internal:?}"
    );
}

#[test]
fn method_definition_selects_the_dispatch_entry_945() {
    let mut lsp = Lsp::tcl();
    let animal = unique_uri("tcl");
    lsp.open_ready(
        &animal,
        "oo::class create Animal {\n    method speak {} { return animal }\n}\n",
    );
    let dog = unique_uri("tcl");
    lsp.open_ready(
        &dog,
        "oo::class create Dog {\n    superclass Animal\n    method speak {} { return dog }\n}\n",
    );
    let main = unique_uri("tcl");
    lsp.open_ready(&main, "set d [Dog new]\n$d speak\n");
    // tclsh 9.0.4: `info object call` = Dog then Animal — definition
    // returns the runtime entry (Dog::speak) only, never the family.
    let defs = locations(&lsp.definition(&main, 1, 4));
    assert_eq!(defs.len(), 1, "one dispatch entry only: {defs:?}");
    assert_eq!(defs[0].uri, dog, "{defs:?}");
}

#[test]
fn per_object_methods_resolve_by_binding_identity_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {}\nproc a {} {\n    set o [C new]\n    oo::objdefine $o {method m {} {return a}}\n    $o m\n}\nproc b {} {\n    set o [C new]\n    oo::objdefine $o {method m {} {return b}}\n    $o m\n}\n";
    lsp.open_ready(&uri, src);
    // tclsh 9.0.4: a=a b=b — b's dispatch resolves b's own override
    // (line 8), never a's (line 3).
    let b_def: Vec<i64> = start_lines(&lsp.definition(&uri, 9, 7))
        .into_iter()
        .collect();
    assert_eq!(b_def, vec![8], "b's own override: {b_def:?}");
    let a_def: Vec<i64> = start_lines(&lsp.definition(&uri, 4, 7))
        .into_iter()
        .collect();
    assert_eq!(a_def, vec![3], "a's own override: {a_def:?}");
}

// -- faults 7–8: the interpreter domain ----------------------------------

#[test]
fn safe_interp_hides_unsafe_commands_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // tclsh 9.0.4: `interp create -safe s` hides source/exec/…; calling
    // them raises `invalid command name` (W129 here), and no source edge
    // may be built from the call.  A never-created interp target warns
    // (W140).
    let src =
        "interp create -safe s\ninterp eval s { source b.tcl }\ninterp eval ghost { puts hi }\n";
    lsp.open_ready(&uri, src);
    let diags = lsp.await_diagnostics(&uri);
    let codes: Vec<String> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert!(
        codes.iter().any(|c| c == "W129"),
        "hidden `source` in the safe interp warns: {codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "W140"),
        "the never-created `ghost` target warns: {codes:?}"
    );
}

// -- fault 9: probe references -------------------------------------------

#[test]
fn command_probe_navigates_without_asserting_existence_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {} {}\nnamespace which -command greet\nnamespace which -command no_such_command_xyz\n";
    lsp.open_ready(&uri, src);
    // The existing probe navigates (definition + references + rename)…
    let defs: Vec<i64> = start_lines(&lsp.definition(&uri, 1, 27))
        .into_iter()
        .collect();
    assert_eq!(defs, vec![0], "the probe site navigates: {defs:?}");
    let refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(refs.contains(&1), "the probe site is a reference: {refs:?}");
    // …while the missing-target probe draws no W123.
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        !diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("W123")),
        "a probe of an absent command asserts nothing: {diags:?}"
    );
}
