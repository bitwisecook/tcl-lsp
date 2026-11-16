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

//! Find-references for procedures and variables, with the non-symbol and out-of-range cases.

use super::*;

// references — procs

#[test]
fn references_proc_from_definition_includes_decl_and_both_calls() {
    // tclsh: `proc greet {} {return hi}; puts [greet]; puts [greet]` -> hi/hi.
    // The two `greet` words (lines 1 and 2) are genuine invocations of the
    // proc declared on line 0 — so references from the declaration must be
    // exactly {decl line 0, call line 1, call line 2}.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "decl + both call sites expected; got {refs:?}",
    );
    // First entry is the declaration on line 0 (include_declaration = true).
    assert_eq!(refs[0].start_line, 0, "decl should lead: {refs:?}");
}

#[test]
fn references_proc_from_call_site_finds_the_same_set() {
    // Triggering Find-All-References from a call site resolves to the same
    // proc and therefore the same reference set as from the declaration.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` CALL (line 1, inside `[greet]`).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "call-site Find-References should match the proc's whole set; got {refs:?}",
    );
}

#[test]
fn references_proc_named_in_info_body_include_the_introspection_site() {
    // `info body PROC` names the proc as data (introspected, not called); it
    // is a command reference, so Find-All-References from the declaration must
    // include the `greet` word inside `info body greet`.
    let src = "proc greet {} { return hi }\ninfo body greet\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1],
        "decl + the `info body` introspection site expected; got {refs:?}",
    );
}

#[test]
fn references_proc_called_inside_oo_objdefine_method_body() {
    // A per-object method body (`oo::objdefine $o { method … { helper } }`) is
    // now analysed like any method body, so a proc it calls is found by
    // Find-All-References from the proc's declaration.
    let src = "proc helper {} {}\n\
               oo::class create Foo {}\n\
               set o [Foo new]\n\
               oo::objdefine $o {\n    \
                   method greet {} { helper }\n\
               }\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert!(
        ref_lines(&refs).contains(&4),
        "the call in the per-object method body (line 4) should be a reference: {refs:?}",
    );
}

#[test]
fn namespace_which_command_probe_navigates_and_renames_945() {
    // `namespace which -command greet` is an existence PROBE (it returns ""
    // for an unknown command) — but reference identity and existence
    // assertion are orthogonal: the typed
    // `CommandNameProbe` role records a first-class, exactly-writable
    // reference (Find-All-References and rename reach the probe site)
    // whose record never feeds W123.
    let src = "proc greet {} {}\nnamespace which -command greet\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1],
        "the probe site is a navigable reference: got {refs:?}",
    );
    // Rename rewrites the exact probe token alongside the declaration.
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
        vec![0, 1],
        "rename rewrites the probe's exact name token: {edits:?}",
    );
    // A glob-pattern probe (`info commands gr*`) names no single command —
    // no reference, no rename edit (the dynamic/pattern abstention).
    let src2 = "proc greet {} {}\ninfo commands gr*\ninfo commands greet\n";
    let analysis2 = analyse(src2);
    let refs2 = references(
        src2,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis2,
        true,
    );
    assert_eq!(
        ref_lines(&refs2),
        vec![0, 2],
        "exact `info commands` probes navigate; patterns abstain: {refs2:?}",
    );
}

#[test]
fn references_proc_named_in_trace_add_execution_include_the_trace_site() {
    // `trace add execution PROC …` names the traced command; it is a
    // reference, so Find-All-References from the declaration includes the
    // `greet` word in the trace (the trailing `handler` is a separate
    // callback prefix, not part of greet's set).
    let src = "proc greet {} {}\nproc handler {args} {}\ntrace add execution greet enter handler\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 2],
        "decl + the trace site expected; got {refs:?}",
    );
}

#[test]
fn references_proc_exclude_declaration_drops_the_decl_line() {
    // include_declaration = false omits the defining span; only call sites
    // remain (lines 1 and 2).
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    let with_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let without_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        false,
    );
    assert_eq!(
        ref_lines(&without_decl),
        vec![1, 2],
        "only call sites when decl excluded; got {without_decl:?}",
    );
    assert_eq!(
        with_decl.len(),
        without_decl.len() + 1,
        "excluding the declaration drops exactly one entry",
    );
}

#[test]
fn references_proc_only_at_real_call_sites_not_substrings() {
    // tclsh: `greeter` is a *different* command from `greet`; only the two
    // bare `greet` words are real invocations of the proc. The provider must
    // not match the `greet` substring inside the proc name `greeter`.
    // Snippet is runnable: both procs return, top level calls each once.
    let src = "proc greet {} { return a }\nproc greeter {} { return b }\ngreet\ngreeter\n";
    let analysis = analyse(src);
    // References for `greet` (cursor on its decl, line 0).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the `greet` call on line 2 missing: {refs:?}"
    );
    assert!(
        !lines.contains(&1),
        "must NOT reference the `greeter` declaration on line 1: {refs:?}",
    );
    assert!(
        !lines.contains(&3),
        "must NOT match the distinct `greeter` call on line 3: {refs:?}",
    );
}

// references — variables

#[test]
fn references_var_includes_definition_and_every_read() {
    // tclsh: `set x 1; puts $x; puts $x` -> 1/1. The two `$x` reads (lines 1
    // and 2) are genuine uses of the variable defined on line 0.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside the first `$x` read (line 1).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "definition + both read sites expected; got {refs:?}",
    );
}

#[test]
fn references_var_exclude_declaration_keeps_only_reads() {
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![1, 2],
        "only the two read sites when declaration excluded; got {refs:?}",
    );
}

#[test]
fn references_var_scoped_to_its_proc_not_a_same_named_local_elsewhere() {
    // tclsh: `proc a {} {set v 10; return $v}` and
    //        `proc b {} {set v 20; return $v}` -> a=10, b=20.
    // The two `v`s are independent locals (one per call frame). References to
    // proc a's `v` must stay inside proc a (lines 1-2) and never reach proc
    // b's `v` (lines 5-6).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on `$v` in proc a's body (line 2).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        12,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "proc a's `v` references must stay within proc a (lines 1-2); got {refs:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "must NOT reference proc b's same-named local `v`; got {refs:?}",
    );
}

// references — known limitation: a command name held in a variable

#[test]
fn references_variable_held_command_name_is_not_resolved_documented_limitation() {
    // A command name stored in a variable and invoked indirectly is, in the
    // general case, statically undecidable.  The flow-sensitive value model
    // narrows the limitation further than a plain constant map: a pure
    // single-`$var` copy chain (`set src helper; set cmd $src; $cmd`)
    // resolves — see
    // `const_cmd_head_resolves_through_a_pure_copy_chain_m7` — so the
    // remaining honest abstention is a *computed* value.  This pins that
    // boundary: no crash, no false attribution, no reference.
    let src =
        "proc helper {} { return hi }\nset cmd [dict get $tbl k]\n$cmd\nset cmd2 x$suffix\n$cmd2\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0],
        "only the declaration; a computed `$cmd` is not statically resolved: {refs:?}"
    );
}

// references — non-symbols / robustness

#[test]
fn references_builtin_command_yields_nothing() {
    // `puts` is a built-in, not a user proc — no references are surfaced.
    let src = "puts hello\nputs world\n";
    let analysis = analyse(src);
    assert!(
        references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            1,
            &analysis,
            true
        )
        .is_empty(),
        "built-in `puts` should have no provider references",
    );
}

#[test]
fn references_unrelated_word_yields_nothing() {
    // A bare literal argument (`hello`) is neither a proc, class, nor var.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true
        )
        .as_slice(),
        []
    );
}

#[test]
fn references_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Way past EOF — must return empty, not panic.
    assert_eq!(
        references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            99,
            0,
            &analysis,
            true
        )
        .as_slice(),
        []
    );
    assert_eq!(
        references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            999,
            &analysis,
            true
        )
        .as_slice(),
        []
    );
    // Empty document.
    let empty = analyse("");
    assert_eq!(
        references(
            "",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            0,
            &empty,
            true
        )
        .as_slice(),
        []
    );
}
