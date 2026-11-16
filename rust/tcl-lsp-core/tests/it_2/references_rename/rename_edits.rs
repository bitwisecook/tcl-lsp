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

//! Renames of procedures and variables, the new-name safety gate, `prepare_rename` and `is_safe_symbol_name`.

use super::*;

// rename — procs

#[test]
fn rename_proc_updates_definition_and_all_call_sites() {
    // tclsh: renaming `greet`->`salute` everywhere keeps the script runnable
    // (`proc salute {} {return hi}; puts [salute]; puts [salute]` -> hi/hi).
    // The rename set must be exactly the decl + both calls.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 0).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "salute",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both call sites rewritten; got {edits:?}",
    );
    assert!(
        edits.iter().all(|e| e.new_text == "salute"),
        "every top-level edit is the bare new name; got {edits:?}",
    );
    // The declaration edit leads and replaces just `greet` -> `salute`
    // (col 5 = start of the proc name in `proc greet`).
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 5);
}

#[test]
fn rename_parent_proc_does_not_edit_a_child_interp_body() {
    // `interp eval child { proc foo }` runs in a child interpreter, so its
    // `proc foo` is isolated from the parent's `::foo` (runnable Tcl: the
    // child is created first).  Renaming the parent `foo` rewrites the
    // parent decl (line 1) and its call (line 3) but must never touch the
    // child body on line 2.
    let src = "interp create child\nproc foo {} {}\ninterp eval child { proc foo {} {} }\nfoo\n";
    let analysis = analyse(src);
    // Cursor on the parent `foo` declaration (line 1, col 6).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        "bar",
        &analysis,
        None,
    );
    assert!(
        edits.iter().all(|e| e.range.start_line != 2),
        "the isolated child interp body must be untouched: {edits:?}",
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1) && lines.contains(&3),
        "the parent decl and call are still rewritten: {edits:?}",
    );
}

#[test]
fn rename_proc_from_call_site_rewrites_declaration_too() {
    // Renaming from a call site rewrites the declaration as well — same set.
    let src = "proc greet {} { return hi }\ngreet\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the first call (line 1).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        2,
        "salute",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![0, 1, 2], "{edits:?}");
    assert!(
        edits.iter().any(|e| e.range.start_line == 0),
        "decl rewritten: {edits:?}"
    );
}

#[test]
fn rename_proc_rejected_when_new_name_collides_with_existing_proc() {
    // tclsh: `hello` already names a distinct proc. Renaming `greet`->`hello`
    // would shadow it (two procs would claim `::hello`), so the provider
    // refuses with an empty edit set rather than merge them.
    let src = "proc greet {} { return a }\nproc hello {} { return b }\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "hello",
        &analysis,
        None,
    );
    assert!(
        edits.is_empty(),
        "collision with existing proc must be refused: {edits:?}"
    );
}

// rename — variables (scoping is the load-bearing correctness fact)

#[test]
fn rename_var_updates_definition_and_reads_with_dollar_preserved() {
    // tclsh: `set y 1; puts $y; puts $y` -> 1/1 — the renamed script runs.
    // The declaration becomes the bare name; each `$x` read keeps its `$`.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor in the first `$x` read.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        "y",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both reads: {edits:?}"
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"y"),
        "declaration rewrite `y` missing: {texts:?}"
    );
    assert_eq!(
        texts.iter().filter(|t| **t == "$y").count(),
        2,
        "both `$x` reads should become `$y`: {texts:?}",
    );
}

#[test]
fn rename_var_from_definition_site_resolves_without_dollar() {
    // Cursor on the bare `x` in `set x 1` (no `$`). The definition-site
    // resolver must still find the variable and rewrite decl + reads.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `x` in `set x` (line 0, col 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        4,
        "y",
        &analysis,
        None,
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "decl edit missing: {edits:?}");
    assert!(texts.contains(&"$y"), "ref edit missing: {edits:?}");
}

#[test]
fn rename_local_var_is_scoped_and_leaves_same_named_var_in_other_proc_intact() {
    // tclsh: `proc a {} {set v 10; return $v}` / `proc b {} {set v 20;
    // return $v}` -> a=10, b=20. Renaming proc a's `v`->`w` must rewrite ONLY
    // proc a's def+read (lines 1-2) and never proc b's `v` (lines 5-6) — the
    // two locals are independent, so clobbering b's `v` would be a real bug.
    //
    // C-Tcl proof of the *renamed* form: `proc a {} {set w 10; return $w}`
    // alongside the untouched `proc b {} {set v 20; return $v}` -> a=10, b=20
    // (verified: identical output to the original).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on the def site `v` in proc a (line 1, col 8).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        8,
        "w",
        &analysis,
        None,
    );
    assert!(
        !edits.is_empty(),
        "expected a scoped rename of proc a's `v`"
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "rename must stay inside proc a (lines 1-2); got {edits:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "rename must NOT clobber proc b's same-named local `v`; got {edits:?}",
    );
    // The read site keeps its `$`.
    assert!(
        edits.iter().any(|e| e.new_text == "$w"),
        "the `$v` read should become `$w`: {edits:?}",
    );
}

#[test]
fn rename_var_rejected_on_same_scope_collision() {
    // Renaming `x`->`y` when a sibling `y` already lives in the same proc
    // scope would merge two distinct variables — refuse.
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    let analysis = analyse(src);
    // Rename `x` (its read site on line 3) to the already-present `y`.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        10,
        "y",
        &analysis,
        None,
    );
    assert!(
        edits.is_empty(),
        "same-scope collision must be refused: {edits:?}"
    );
}

// rename — safety gating (shape + builtin shadow)

#[test]
fn rename_rejects_syntactically_unsafe_new_names() {
    // A new name that isn't a valid Tcl identifier is refused (empty edits)
    // so the editor never applies a partially-broken rename.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    for bad in [
        "bad name",
        "1lead",
        "with-dash",
        "has::colon",
        "with$dollar",
        "",
    ] {
        assert!(
            rename(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                0,
                6,
                bad,
                &analysis,
                None
            )
            .is_empty(),
            "unsafe new name {bad:?} must yield no edits",
        );
    }
}

#[test]
fn rename_var_also_rejects_unsafe_new_name() {
    // The shape gate applies to variable renames too.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            "bad name",
            &analysis,
            None
        )
        .as_slice(),
        []
    );
}

#[test]
fn rename_proc_to_builtin_command_name_is_blocked_with_registry() {
    // tclsh: `puts` is a built-in command. Renaming a proc to `puts` would
    // make the editor's calls dispatch to the built-in instead of the proc,
    // so with a registry supplied the rename is refused.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        edits.is_empty(),
        "rename to built-in `puts` must be blocked: {edits:?}"
    );
}

#[test]
fn rename_proc_to_non_builtin_succeeds_with_registry() {
    // The registry gate only blocks built-in names; a free name still renames.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "salut",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "non-built-in rename should succeed with a registry"
    );
    assert!(edits.iter().all(|e| e.new_text == "salut"), "{edits:?}");
}

#[test]
fn rename_var_to_builtin_name_is_allowed() {
    // The built-in-shadow gate is proc-only — variable names occupy a
    // separate Tcl namespace, so renaming a var to `puts` is fine.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "variable rename to `puts` should succeed"
    );
}

// rename — non-symbols / robustness

#[test]
fn rename_unknown_word_yields_no_edits() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // `hello` is a bare literal — nothing to rename.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "x",
            &analysis,
            None
        )
        .as_slice(),
        []
    );
}

#[test]
fn rename_builtin_without_registry_still_yields_no_edits() {
    // `puts` has no user proc def, so even without a registry there is no
    // symbol whose definition/calls could be rewritten.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            1,
            "show",
            &analysis,
            None
        )
        .as_slice(),
        []
    );
}

#[test]
fn rename_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            99,
            0,
            "z",
            &analysis,
            None
        )
        .as_slice(),
        []
    );
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            999,
            "z",
            &analysis,
            None
        )
        .as_slice(),
        []
    );
    let empty = analyse("");
    assert_eq!(
        rename(
            "",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            0,
            "z",
            &empty,
            None
        )
        .as_slice(),
        []
    );
}

// prepare_rename — gates the rename UI

#[test]
fn prepare_rename_offers_proc_name_and_placeholder() {
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 6, &analysis).expect("proc name is renameable");
    assert_eq!(p.placeholder, "greet");
    // Anchored at the proc name span (`proc greet` -> name starts col 5).
    assert_eq!(p.range.start_line, 0);
    assert_eq!(p.range.start_character, 5);
}

#[test]
fn prepare_rename_offers_variable_name_and_placeholder() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 6, &analysis).expect("variable is renameable");
    assert_eq!(p.placeholder, "x");
}

#[test]
fn prepare_rename_rejects_builtin_command() {
    // `puts` is a built-in, not a renameable user symbol.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(
        prepare_rename(src, 0, 1, &analysis).is_none(),
        "built-in `puts` is not renameable",
    );
}

#[test]
fn prepare_rename_rejects_whitespace_and_bare_literal() {
    // The space between `set` and `x`, and the bare literal `hello`, are not
    // renameable symbols.
    let src = "set x 1\nputs hello\n";
    let analysis = analyse(src);
    // Column 3 on line 0 is the space in `set x`.
    assert!(
        prepare_rename(src, 0, 3, &analysis).is_none(),
        "whitespace is not renameable"
    );
    // The literal `hello` argument on line 1.
    assert!(
        prepare_rename(src, 1, 6, &analysis).is_none(),
        "bare literal is not renameable"
    );
}

#[test]
fn prepare_rename_out_of_range_and_empty_file_return_none() {
    let src = "set x 1\n";
    let analysis = analyse(src);
    assert!(prepare_rename(src, 99, 0, &analysis).is_none());
    assert!(prepare_rename(src, 0, 999, &analysis).is_none());
    let empty = analyse("");
    assert!(prepare_rename("", 0, 0, &empty).is_none());
}

// is_safe_symbol_name — the shape predicate behind the gate

#[test]
fn is_safe_symbol_name_accepts_identifiers_and_rejects_the_rest() {
    for ok in ["foo", "Foo", "_under", "a1", "snake_case_42"] {
        assert!(is_safe_symbol_name(ok), "{ok:?} should be accepted");
    }
    for bad in [
        "",
        "1lead",
        "has space",
        "has-dash",
        "has::colon",
        "with$dollar",
        "dotted.name",
    ] {
        assert!(!is_safe_symbol_name(bad), "{bad:?} should be rejected");
    }
}
