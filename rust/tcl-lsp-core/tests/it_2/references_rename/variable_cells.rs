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

//! Which variable cell a name reaches: object variables, `uplevel` bodies, cells shared across procedures, role-read name words and list-built bodies.

use super::*;

// Renaming a TclOO instance variable must NOT rewrite the method body, and
// `uplevel #0` var resolution must skip proc locals.  TP/FP/TN/FN coverage.

/// The `oo::class create C { variable n; method get {} {return $n} … }`
/// fixture used across the object-variable tests.  `$n` in `get` sits at line 2.
const OBJECT_VAR_SRC: &str = "oo::class create C {\n    variable n\n    method get {} {return $n}\n    method set {x} {set n $x}\n}\n";

/// FP-guard (the corruption regression): a rename edit must never span more
/// than one line, and must never cover the whole `{return $n}` method body.
/// Before the fix the declaration edit was `2:18-2:28 → "w"`, destroying the
/// body.
#[test]
fn object_var_rename_never_rewrites_method_body() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(
        OBJECT_VAR_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        col,
        "w",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "expected a rename to be produced");
    for e in &edits {
        assert_eq!(
            e.range.start_line, e.range.end_line,
            "no rename edit may span multiple lines (body-destroying): {e:?}"
        );
        // The method body `{return $n}` starts at col 18 on line 2.  No edit
        // may start at the brace and run to end-of-body.
        let spans_body =
            e.range.start_line == 2 && e.range.start_character <= 18 && e.range.end_character >= 28;
        assert!(!spans_body, "edit covers the whole method body: {e:?}");
    }
}

/// TP: the declaration edit lands on the `variable n` name token (line 1),
/// and the `$n` read (line 2) is rewritten — the correct, non-destructive
/// rename.
#[test]
fn object_var_rename_edits_declaration_and_use() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(
        OBJECT_VAR_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        col,
        "w",
        &analysis,
        None,
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "expected the `variable n` declaration (line 1) to be renamed; got {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "expected the `$n` use (line 2) to be renamed; got {edits:?}"
    );
    // The declaration edit is exactly the `n` token, not a wide span.
    let decl = edits
        .iter()
        .find(|e| e.range.start_line == 1)
        .expect("declaration edit");
    assert_eq!(decl.new_text, "w");
    assert_eq!(
        decl.range.end_character - decl.range.start_character,
        1,
        "declaration edit must cover just the `n` token: {decl:?}"
    );
}

/// FN-that-must-hold: `references` on the object variable surfaces the
/// declaration + the use without a body-wide span (the reference set feeds
/// document-highlight; a whole-body highlight was the visible symptom).
#[test]
fn object_var_references_are_token_sized() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let refs = references(
        OBJECT_VAR_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        col,
        &analysis,
        true,
    );
    assert_ne!(refs.as_slice(), []);
    for r in &refs {
        assert_eq!(
            r.start_line, r.end_line,
            "reference span crosses lines: {r:?}"
        );
    }
}

/// TN: an ordinary proc-local variable (no `TclOO`) still renames across its
/// own decl + uses — the fix must not disturb the common path.
#[test]
fn plain_proc_local_rename_unaffected() {
    let src = "proc p {} {\n    set count 0\n    incr count\n    return $count\n}\n";
    let analysis = analyse(src);
    // cursor on `$count` (line 3)
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        3,
        col,
        "total",
        &analysis,
        None,
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "decl `set count` should rename: {edits:?}"
    );
    assert!(lines.contains(&3), "`$count` use should rename: {edits:?}");
    for e in &edits {
        assert_eq!(e.range.start_line, e.range.end_line);
    }
}

/// TP: inside `uplevel #0 { … }` the body runs in the global frame, so a
/// `$g` there resolves to the GLOBAL `g`, not a same-named proc-local.
/// References on `$g` in the uplevel body must include the global definition
/// (line 0), not the proc-local (`set g 99`, line 1).
#[test]
fn uplevel_zero_resolves_global_not_proc_local() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    uplevel #0 { puts $g }\n}\n";
    let analysis = analyse(src);
    // cursor on `$g` inside the uplevel body (line 3)
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1; // on the `g`
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&0),
        "uplevel #0 `$g` must resolve to the global `set g` on line 0; got {refs:?}"
    );
    assert!(
        !lines.contains(&2),
        "uplevel #0 `$g` must NOT resolve to the proc-local `set g 99` on line 2; got {refs:?}"
    );
}

/// TN: a non-uplevel `$g` inside the proc still resolves to the proc-local
/// (the guard must only fire inside an uplevel scope).
#[test]
fn non_uplevel_proc_local_still_resolves_locally() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    puts $g\n}\n";
    let analysis = analyse(src);
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&2),
        "plain proc-body `$g` must resolve to the proc-local on line 2; got {refs:?}"
    );
}

/// FP-guard: inside a non-`#0` `uplevel N { … }` the body runs in the caller's
/// frame — statically unknown — so a `$g` it does not itself declare abstains:
/// it must link neither the enclosing proc-local (a definite mis-attribution)
/// nor the global (the frame is not necessarily global, unlike `#0`).
#[test]
fn uplevel_nonzero_abstains_from_proc_and_global() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    uplevel 1 { puts $g }\n}\n";
    let analysis = analyse(src);
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1; // on the `g`
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        !lines.contains(&2),
        "uplevel 1 `$g` must NOT link the proc-local (line 2); got {refs:?}"
    );
    assert!(
        !lines.contains(&0),
        "uplevel 1 `$g` must NOT link the global (line 0) — the frame is unknown; got {refs:?}"
    );
}

/// A variable declared *inside* a non-`#0` `uplevel` body resolves to itself
/// (the abstention drops only the frames outside the body).
#[test]
fn uplevel_nonzero_body_local_resolves_within_body() {
    let src = "proc p {} {\n    uplevel 1 {\n        set h 5\n        puts $h\n    }\n}\n";
    let analysis = analyse(src);
    // cursor on `$h` (line 3)
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$h").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&2),
        "the body's own `set h` (line 2) must be a reference; got {refs:?}"
    );
    assert!(
        lines.contains(&3),
        "the `$h` use (line 3) must be a reference; got {refs:?}"
    );
}

#[test]
fn object_var_references_unify_across_methods() {
    // `$n` used in `get` and written in `bump`; both are the one instance
    // variable declared by `variable n`.  Find-References on `$n` in `get`
    // must reach the declaration (line 1) and the sibling-method use.
    let src = "oo::class create C {\n    variable n\n    method get {} { return $n }\n    method bump {} { incr n }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&1),
        "declaration (line 1) expected: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`$n` in get (line 2) expected: {refs:?}"
    );
    assert!(
        lines.contains(&3),
        "the sibling-method use `incr n` (line 3) must unify: {refs:?}"
    );
}

#[test]
fn object_var_rename_unifies_declaration_and_all_method_uses() {
    // Renaming the instance variable rewrites its `variable` declaration and
    // every method's use as one variable — not just the method under the cursor.
    let src = "oo::class create C {\n    variable n\n    method get {} { return $n }\n    method bump {} { incr n }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        col,
        "count",
        &analysis,
        None,
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "declaration (line 1) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "`$n` in get (line 2) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&3),
        "`incr n` in bump (line 3) must rename: {edits:?}"
    );
    // No edit spans the body (the earlier corruption guard still holds).
    assert!(
        edits.iter().all(|e| e.range.start_line == e.range.end_line),
        "{edits:?}"
    );
}

#[test]
fn namespace_variable_unifies_across_procs() {
    // `variable count` in two procs plus the namespace-level declaration are
    // one cell (`::app::count`); Find-References on `$count` must reach them all.
    let src = "namespace eval ::app {\n    variable count 0\n    proc bump {} { variable count; incr count }\n    proc get {} { variable count; return $count }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    eprintln!("namespace-variable reference lines = {lines:?}");
    assert!(
        lines.contains(&1),
        "namespace-level `variable count 0` (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`variable count`/`incr count` in bump (line 2): {refs:?}"
    );
    assert!(lines.contains(&3), "`$count` in get (line 3): {refs:?}");
}

#[test]
fn global_variable_unifies_across_procs() {
    // `global g` in a proc aliases `::g`; the top-level `set g` and another
    // proc's `global g` are the same cell.
    let src = "set g 0\nproc a {} { global g; incr g }\nproc b {} { global g; return $g }\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$g").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    eprintln!("global-variable reference lines = {lines:?}");
    assert!(
        lines.contains(&1),
        "`global g`/`incr g` in a (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`global g`/`$g` in b (line 2): {refs:?}"
    );
}

#[test]
fn namespace_variable_rename_unifies_all_aliases() {
    // Renaming the namespace variable rewrites its declaration and every
    // `variable count` alias + use across procs, as one variable.
    let src = "namespace eval ::app {\n    variable count 0\n    proc bump {} { variable count; incr count }\n    proc get {} { variable count; return $count }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        3,
        col,
        "total",
        &analysis,
        None,
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "namespace decl (line 1) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "bump's alias + use (line 2) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&3),
        "get's alias + use (line 3) must rename: {edits:?}"
    );
    assert!(
        edits.iter().all(|e| e.new_text.contains("total")),
        "{edits:?}"
    );
}

#[test]
fn namespace_variables_in_different_namespaces_do_not_unify() {
    // FP guard: `variable count` in ::a and ::b are distinct cells
    // (`::a::count` vs `::b::count`) and must NOT be unified.
    let src = "namespace eval ::a {\n    proc p {} { variable count; return $count }\n}\nnamespace eval ::b {\n    proc q {} { variable count; incr count }\n}\n";
    let analysis = analyse(src);
    // `$count` in ::a::p (line 1)
    let col = src.lines().nth(1).unwrap().find("$count").unwrap() as u32 + 1;
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        col,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "::a's own use (line 1): {refs:?}");
    assert!(
        !lines.contains(&4),
        "::b::count (line 4) is a different cell and must NOT unify: {refs:?}"
    );
}

// A registry `VarRead`-role name word is a navigable use site.
//
// A variable is read by more than `$name`.  tclsh 9.0.4 / 8.6.16, identical:
//
//   proc f {} {set m 1; puts [set m]}          ; f  -> 1
//   proc f {} {set m 1; puts [info exists m]}  ; f  -> 1
//
// Both `m` words really do read the cell, so both are reference sites — and
// a cursor on either must resolve the cell, exactly as a cursor on `$m` does.

/// `puts [set m]` / `puts [info exists m]` — the read is a bare name word.
const ROLE_READ_SRC: &str = "proc f {} {\n    \
set m 1\n    \
puts [set m]\n    \
puts [info exists m]\n\
}\n";

#[test]
fn find_references_reports_var_read_role_name_words() {
    let analysis = analyse(ROLE_READ_SRC);
    // Cursor on the `m` of `set m 1`.
    let refs = references(
        ROLE_READ_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        8,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![1, 2, 3],
        "the declaration plus both role-read sites; got {refs:?}"
    );
}

#[test]
fn a_cursor_on_a_role_read_word_resolves_the_cell() {
    let analysis = analyse(ROLE_READ_SRC);
    let decl = LspRange {
        start_line: 1,
        start_character: 8,
        end_line: 1,
        end_character: 9,
    };
    // The `m` of `[set m]` (line 2) and of `[info exists m]` (line 3).
    for (line, col) in [(2u32, 14u32), (3, 22)] {
        assert_eq!(
            tcl_lsp_core::definition::definition(ROLE_READ_SRC, line, col, &analysis),
            vec![decl],
            "go-to-definition from the role-read word at ({line},{col})"
        );
        assert_eq!(
            ref_lines(&references(
                ROLE_READ_SRC,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                line,
                col,
                &analysis,
                true
            )),
            vec![1, 2, 3],
            "find-references from the role-read word at ({line},{col})"
        );
    }
}

#[test]
fn tn_a_value_word_of_the_same_spelling_is_not_a_reference() {
    // FP guard: only the *name* position counts.  `puts m` prints the literal
    // string `m` (tclsh: `set m 1; puts m` -> m), so that word is not a read
    // of the variable and must not join its reference set.
    let src = "proc f {} {\n    set m 1\n    puts m\n}\n";
    let analysis = analyse(src);
    assert_eq!(
        ref_lines(&references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            1,
            8,
            &analysis,
            true
        )),
        vec![1],
        "a plain value word is not a variable reference"
    );
}

// A read inside a `[list …]`-built script argument.
//
// `::tk::SourceLibFile` in Tk's `library/tk.tcl`:
//
//   proc ::tk::SourceLibFile {file} {
//       namespace eval :: [list source [file join $::tk_library $file.tcl]]
//   }
//
// `$file` is the proc's own parameter.  The `[…]` is evaluated in the proc's
// frame *before* `namespace eval` enters `::`, so the read is an ordinary use
// of the parameter — yet hover, go-to-definition and find-references all
// answered nothing, because the namespace scope had claimed those bytes.

const TK_SOURCE_LIB_FILE: &str = "proc ::tk::SourceLibFile {file} {\n    \
namespace eval :: [list source [file join $::tk_library $file.tcl]]\n\
}\n";

#[test]
fn a_parameter_read_inside_a_list_built_body_navigates() {
    let analysis = analyse(TK_SOURCE_LIB_FILE);
    let decl = LspRange {
        start_line: 0,
        start_character: 26,
        end_line: 0,
        end_character: 30,
    };
    let read = LspRange {
        start_line: 1,
        start_character: 60,
        end_line: 1,
        end_character: 65,
    };
    // Cursor on the `file` of `$file`.
    assert_eq!(
        tcl_lsp_core::definition::definition(TK_SOURCE_LIB_FILE, 1, 62, &analysis),
        vec![decl],
        "go-to-definition must reach the `{{file}}` parameter"
    );
    assert_eq!(
        references(
            TK_SOURCE_LIB_FILE,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            1,
            62,
            &analysis,
            true
        ),
        vec![decl, read],
        "find-references must report the declaration and the read"
    );
    let hover =
        hover_text(TK_SOURCE_LIB_FILE, 1, 62, &analysis).expect("hover must resolve the parameter");
    assert!(
        hover.contains("**Variable** `file`"),
        "hover names the parameter, not the `file` command; got {hover:?}"
    );
}

#[test]
fn the_parameter_declaration_hovers_as_a_variable_not_the_file_command() {
    // `file` is also a registry command name, so the parameter *declaration*
    // must not hover as that command's documentation.
    let analysis = analyse(TK_SOURCE_LIB_FILE);
    let hover = hover_text(TK_SOURCE_LIB_FILE, 0, 27, &analysis)
        .expect("hover must resolve the parameter declaration");
    assert!(
        hover.contains("**Variable** `file`"),
        "a parameter-list word is a declaration, not a command; got {hover:?}"
    );
}
