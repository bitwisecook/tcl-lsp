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

//! Cell updates in value position: words read as Tcl substitutes them, renamed heads and qualified names that write globals.

use super::*;

/// A value-position cell update whose read the host statement does not
/// hold declines rather than answering a pending bottom a join launders:
/// `tcl opt` had rewritten `puts $x` to `puts 5`, where `f 1` prints 2.
#[test]
fn a_value_position_increment_never_launders_a_join() {
    let source =
        "proc f {cond} {set n 1; if {$cond} {set x [incr n]} else {set x 5}; puts $x}\nf 1\nf 0\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts $x"),
            "{dialect}: `puts $x` must keep its variable\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "2\n5\n");
}

/// A cell update reads its words as Tcl substitutes them: a braced `$x` is
/// the text `$x`, not a read of `x`, and a quoted `\t` is a tab. Read by
/// their spelling, `puts $t` was rewritten to `puts a1` and `lappend l
/// "a\tb"` folded to `set l {{a\tb}}`.
#[test]
fn a_cell_update_reads_its_words_as_tcl_substitutes_them() {
    let cases: [(&str, &str, LatticeValue, &str); 4] = [
        (
            "proc p {} {set x 1; set s a; append s {$x}; set t $s; puts $t}\np\n",
            "s",
            text("a$x"),
            "a$x\n",
        ),
        (
            "proc p {} {set x 1; set l {}; lappend l {$x}; puts $l}\np\n",
            "l",
            text("{$x}"),
            "{$x}\n",
        ),
        (
            "proc p {} {set s x; append s \"a\\tb\"; set t $s; puts $t}\np\n",
            "s",
            text("xa\tb"),
            "xa\tb\n",
        ),
        (
            "proc p {} {set l {}; lappend l \"a\\tb\"; puts $l}\np\n",
            "l",
            text("{a\tb}"),
            "{a\tb}\n",
        ),
    ];
    for (source, var, value, output) in cases {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                last_value(source, dialect, "::p", var),
                value,
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// `[string range "a\tb" 0 1]` in value position reads the quoted word as
/// Tcl substitutes it (`a` and a tab); a braced word keeps its backslash.
#[test]
fn value_position_words_are_read_as_tcl_substitutes_them() {
    let quoted = "proc p {} {set r [string range \"a\\tb\" 0 1]; puts $r}\np\n";
    let braced = "proc p {} {set r [string range {a\\tb} 0 1]; puts $r}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert_eq!(last_value(quoted, dialect, "::p", "r"), text("a\t"));
        assert_eq!(last_value(braced, dialect, "::p", "r"), text("a\\"));
    }
    prints_under_every_release(quoted, "a\t\n");
    prints_under_every_release(braced, "a\\\n");
}

/// A `{*}` word in value position expands before a route reads it:
/// `[list {*}{a b}]` is the two elements `a` and `b`. Read as the one
/// braced word `a b`, `tcl opt` rewrote `puts [llength $l]` to `puts 1`
/// where tclsh prints 2.
#[test]
fn an_expanded_word_in_value_position_is_several_operands() {
    let source = "proc p {} {set l [list {*}{a b}]; puts [llength $l]; puts [lindex $l 1]}\np\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let value = last_value(source, dialect, "::p", "l");
        assert!(
            value == text("a b") || !matches!(value, LatticeValue::Const(_)),
            "{dialect}: {value:?}"
        );
    }
    prints_under_releases_from(source, "2\nb\n", "8.5");
}

/// The shared lattice folds under the module's observed bindings, the
/// stance a rewrite's re-run proves under too: with `proc incr` defined at
/// the top level, `incr n` calls that procedure, so `n#2` is not the
/// builtin's 2, and the statement says why (#2164).
#[test]
fn the_shared_lattice_declines_a_renamed_head() {
    let body = "proc p {} {set n 1; incr n; return $n}\n";
    let shadowed = format!("proc incr {{v args}} {{return 99}}\n{body}");
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(&shadowed, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["declined: rebinding-suspected"],
            "{dialect}"
        );
        let unit = unit_of(body, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 2),
            Some(LatticeValue::Const(ConstValue::int(2))),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["evaluated"],
            "{dialect}"
        );
    }
}

/// A `::`-qualified global a nested cell update writes is in its
/// procedure's global-write summary with no `global` declaration, so a
/// caller no longer forwards the value the global held before the call
/// or deletes the store it reads (#2214). An unqualified name with no
/// declaration stays local.
#[test]
fn a_qualified_nested_cell_update_is_a_global_write() {
    use tcl_compiler::cfg_builder::global_write_info::detect_global_write_procs;
    use tcl_compiler::lowering::lower_to_ir;
    let registry = static_context_for("tcl8.6").commands();
    for body in ["set y [incr ::hits]", "set y [expr {[incr ::hits] + 1}]"] {
        let module = lower_to_ir(&format!("proc bump {{}} {{{body}; return $y}}\n"), registry);
        let writes = detect_global_write_procs(&module);
        assert!(
            writes[&fixture_id(&module.procedures, "::bump")]
                .names
                .contains("::hits"),
            "{body}: {writes:?}"
        );
        assert!(
            writes[&fixture_id(&module.procedures, "::bump")]
                .names
                .contains("hits"),
            "{body}: {writes:?}"
        );
    }
    let module = lower_to_ir("proc bump {} {set y [incr hits]; return $y}\n", registry);
    let writes = detect_global_write_procs(&module);
    assert!(
        writes[&fixture_id(&module.procedures, "::bump")].is_empty(),
        "{writes:?}"
    );

    let source = "set hits 0\nproc bump {} {set y [incr ::hits]; return $y}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("puts $hits"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "1\n");
}

/// The same holds for a qualified global a procedure reaches through
/// `upvar #0`: the caller at the global frame reads `::hits` as `hits`.
#[test]
fn a_qualified_alias_target_is_a_global_write_at_the_caller() {
    let source = "set hits 0\nproc bump {} {upvar #0 ::hits h; incr h}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("puts $hits"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "1\n");
}

/// A callee's global-write summary records writes, not reads, and a
/// callee that writes a global may read it first: `set hits 10` ahead of
/// a `bump` running `global hits; incr hits 5` is observed, not dead
/// (tclsh 8.4 to 9.1 print 15, where deleting it printed 5).
#[test]
fn a_store_a_global_writing_callee_reads_is_kept() {
    let source = "set hits 10\nproc bump {} {global hits; incr hits 5}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 10"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "15\n");
}

/// A callee that only reads a global keeps the store it reads: O109 does not
/// delete `set hits 0` ahead of `show` — the rewrite keeps both stores, and
/// tclsh 8.4 to 9.1 print 0 then 1 for both programs.
#[test]
fn a_store_a_global_reading_callee_observes_is_kept() {
    let source = "set hits 0\nproc show {} {global hits; puts $hits}\nshow\nset hits 1\nshow\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("set hits 1"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "0\n1\n");
}

/// `[set x]` in value position reads the variable through the cell-write
/// route (the Tier-1 list); `[set x 10]` writes it, so the statement's call
/// holds the write and its host the result: `x` is 10 after the statement and
/// so is `w`, which tclsh 8.4 to 9.1 print, before and after the optimiser.
#[test]
fn a_value_position_set_reads_the_variable() {
    let source = "proc p {} {set x hello; set r [set x]; set w [set x 10]; return $r$w}\n";
    let ten = Some(LatticeValue::Const(ConstValue::int(10)));
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(text("hello")),
            "{dialect}"
        );
        assert_eq!(value_at(&unit, "::p", "w", 1), ten, "{dialect}");
        assert_eq!(value_at(&unit, "::p", "x", 2), ten, "{dialect}");
    }
    prints_under_every_release(&format!("{source}puts [p]\n"), "hello10\n");
}

/// Program (3) of the interface contract: `incr n` and `incr n 2` over
/// `set n 1` give `n#3` the 4 every release prints, in the shared lattice
/// under every dialect — each statement's route answering — and the
/// optimiser forwards it into `puts $n` (O100).
#[test]
fn program_three_folds_in_every_consumer() {
    let source = "proc p {} {set n 1; incr n; incr n 2; puts $n}\np\n";
    for dialect in TCL_CORE_DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 3),
            Some(LatticeValue::Const(ConstValue::int(4))),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["evaluated", "evaluated"],
            "{dialect}"
        );
        let rewrites = rewrites_of(source, dialect);
        assert!(
            rewrites.iter().any(|r| r.code == DiagCode::O100),
            "{dialect}: {rewrites:#?}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(rewritten.contains("puts 4"), "{dialect}:\n{rewritten}");
    }
    prints_under_every_release(source, "4\n");
}
