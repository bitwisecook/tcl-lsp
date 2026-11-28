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

//! Branch refinements: merges, traced variables, equality and range tests, and the types a `string is` arm proves.

use super::*;

/// The blocks of `proc`'s `puts` statements, in source order.
fn puts_blocks(unit: &CompilationUnit, proc: &str) -> Vec<tcl_compiler::cfg::BlockId> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let mut found: Vec<(u32, tcl_compiler::cfg::BlockId)> = function
        .cfg
        .blocks
        .iter()
        .flat_map(|(&id, block)| {
            block
                .statements
                .iter()
                .filter(|statement| {
                    matches!(statement, Statement::Call { command, .. } if command == "puts")
                })
                .map(move |statement| (statement.span().start(), id))
        })
        .collect();
    found.sort_unstable();
    found.into_iter().map(|(_, id)| id).collect()
}

/// The value `var`'s version `version` holds at `block` of `proc`: the
/// block-qualified lookup, which a refinement in force there narrows.
fn value_in(
    unit: &CompilationUnit,
    proc: &str,
    block: tcl_compiler::cfg::BlockId,
    var: &str,
    version: u32,
) -> Option<LatticeValue> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.value_at(block, (symbol, version)).cloned()
}

/// A refinement never makes a version, and a merge keeps only what every
/// path into it carries: the arms of `if {$x eq "a"} … elseif {$x eq "b"}`
/// hold `x` at `a` and at `b`, and where they meet — the `else` arm returns,
/// so only the two refined arms reach the `puts` past them — `x` reads as
/// its own value, never as either refinement or a set of the two. tclsh 8.4
/// to 9.1 print `a 1`, `a`, `b 2` and `b` for `p a; p b; p c`, before and
/// after `tcl opt`.
#[test]
fn a_merge_drops_the_refinement() {
    crate::install_surfaces();
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        set r 1\n    } elseif {$x eq \"b\"} {\n        set r 2\n    } else {\n        return\n    }\n    puts \"$x $r\"\n    puts $x\n}\np a\np b\np c\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let refined: Vec<String> = function
            .sccp
            .refinements
            .iter()
            .filter_map(|refinement| match &refinement.fact {
                tcl_registry::value_transfer::FactView::Exact(value, _) => {
                    Some(String::from_utf8_lossy(&value.bytes).into_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            refined,
            ["a", "b"],
            "{dialect}: each arm's edge refines `x`"
        );
        let blocks: Vec<_> = function
            .cfg
            .blocks
            .iter()
            .filter_map(|(&id, block)| {
                block
                    .statements
                    .iter()
                    .find_map(|statement| match statement {
                        Statement::AssignConst { name, value, .. } if name == "r" => {
                            Some((value.clone(), id))
                        }
                        _ => None,
                    })
            })
            .collect();
        for (value, block) in blocks {
            let expected = if value == "1" { "a" } else { "b" };
            assert_eq!(
                value_in(&unit, "::p", block, "x", 0),
                Some(text(expected)),
                "{dialect}: the arm that sets `r` to {value} holds `x` at {expected}"
            );
        }
        for block in puts_blocks(&unit, "::p") {
            assert_eq!(
                value_in(&unit, "::p", block, "x", 0),
                value_at(&unit, "::p", "x", 0),
                "{dialect}: past the merge `x` is its own value"
            );
            assert_eq!(function.sccp.refinements_in(block).count(), 0, "{dialect}");
        }
    }
    prints_under_every_release(source, "a 1\na\nb 2\nb\n");
}

/// A traced variable is never refined: a read trace may write the variable
/// as the read runs, so the `$x` the arm prints need not be the `a` the
/// condition read. `tr` leaves the first read alone and rewrites `x` on the
/// second, and tclsh 8.4 to 9.1 print `changed`, before and after `tcl opt`.
/// A trace another procedure installs on a name makes it externally mutable
/// in every function, so `q`'s `y` is never refined either.
#[test]
fn a_traced_variable_is_never_refined() {
    crate::install_surfaces();
    let source = "set n 0\nproc tr {name1 name2 op} {upvar 1 $name1 v; incr ::n; if {$::n > 1} {set v changed}}\nproc p {x} {\n    trace add variable x read tr\n    if {$x eq \"a\"} {puts $x}\n}\nproc t {} {trace add variable y write tr}\nproc q {y} {\n    if {$y eq \"a\"} {puts $y}\n}\np a\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        for proc in ["::p", "::q"] {
            let function = unit
                .procedures
                .get(&fixture_id(&unit.procedures, proc))
                .expect("the procedure");
            assert!(
                function.sccp.refinements.is_empty(),
                "{dialect} {proc}: {:?}",
                function.sccp.refinements
            );
            assert!(function.sccp.value_entries.is_empty(), "{dialect} {proc}");
        }
    }
    prints_under_every_release(source, "changed\n");
}

/// The nested equality decides: inside `if {$x eq "a"}`'s arm `x` is `a`,
/// so `if {$x eq "b"}` there is false — a constant branch the solver applies,
/// its true block outside the executable blocks — under every dialect, and
/// the optimiser drops `puts never`. The program prints `inner` for `a` and
/// nothing for `b` under tclsh 8.4 to 9.1, before and after `tcl opt`.
#[test]
fn the_nested_equality_decides() {
    crate::install_surfaces();
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        if {$x eq \"b\"} {puts never} else {puts inner}\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let decided = function
            .sccp
            .constant_branches
            .iter()
            .find(|branch| branch.condition == "$x eq \"b\"")
            .unwrap_or_else(|| panic!("{dialect}: {:?}", function.sccp.constant_branches));
        assert!(!decided.value, "{dialect}");
        let dead = function
            .cfg
            .block_id(&decided.not_taken_target)
            .expect("the arm");
        assert!(
            !function.sccp.executable_blocks.contains(&dead),
            "{dialect}: the inner true block never runs"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts never"),
            "{dialect}: the optimiser drops the dead arm\n{rewritten}"
        );
    }
    prints_under_every_release(source, "inner\n");
}

/// The page's twelve programs (`docs/design/compiler/value-transfers.md`
/// § *Predicate refinement*), each through a procedure whose argument no
/// call site pins — a global the analysis holds no value for — so the
/// branch itself is the only evidence: a numeric `==` keeps the string
/// (`1.0`, ` 1`, `01`, and `08` from 9.0), `eq` compares spellings, a
/// `string is` test proves a type and never a value (` 12 `, `{}` with and
/// without `-strict`, `0x10`), truth is no value (`yes`), and from 8.5 an
/// opaque `-nocase` `switch` and `in` keep what they were given. Each prints
/// the same under every release before and after `tcl opt`.
///
/// A read of the refined variable itself is never rewritten, so the `==`
/// arms, an `in` arm and a flattened `switch`'s arms also read the arm's
/// value through a definition (`set y $x; puts $y`), which O100 inlines
/// where the arm proves an exact value: an exact value where the table
/// says "never the string" prints `1` for `1.0`, and one member of an `in`
/// set prints that member for another.
#[test]
fn the_twelve_refinement_witnesses() {
    crate::install_surfaces();
    let every = "proc p {x} {if {$x == 1} {set y $x; puts $y; return [string length $x]}; return none}\n\
                 proc q {x} {if {$x == 1} {set y $x; puts $y; return $x}; return none}\n\
                 proc r {x} {if {$x eq \"1\"} {return yes}; return no}\n\
                 proc s {x} {if {$x eq \"a\"} {return $x}; return none}\n\
                 proc t {x} {if {[string is integer -strict $x]} {return [string length $x]}; return none}\n\
                 proc u {x} {if {[string is integer $x]} {return \"yes [string length $x]\"}; return no}\n\
                 proc u2 {x} {if {[string is integer -strict $x]} {return yes}; return no}\n\
                 proc v {x} {if {[string is integer -strict $x]} {return $x}; return none}\n\
                 proc w {x} {if {$x} {return $x}; return none}\n\
                 proc k {x} {switch -- $x {a {set y $x; puts $y; return A} b {set y $x; puts $y; return B}}; return none}\n\
                 set ::v 1.0; puts [p $::v]\n\
                 set ::v \" 1\"; puts [p $::v]\n\
                 set ::v 01; puts [q $::v]\n\
                 set ::v 1.0; puts [r $::v]\n\
                 set ::v a; puts [s $::v]\n\
                 set ::v \" 12 \"; puts [t $::v]\n\
                 set ::v {}; puts [u $::v]; puts [u2 $::v]\n\
                 set ::v 0x10; puts [v $::v]\n\
                 set ::v yes; puts [w $::v]\n\
                 set ::v b; puts [k $::v]\n";
    prints_under_every_release(
        every,
        "1.0\n3\n 1\n2\n01\n01\nno\na\n4\nyes 0\nno\n0x10\nyes\nb\nB\n",
    );
    let leading_zero = "proc y {x} {if {$x == 8} {set y $x; puts $y; return \"yes [string length $x]\"}; return no}\n\
                        set ::v 08; puts [y $::v]\n";
    for (series, tclsh) in releases_on_path() {
        let expected = if series < "9.0" {
            "no\n"
        } else {
            "08\nyes 2\n"
        };
        let (rewritten, _) = optimised(leading_zero, &dialect_of(series));
        for program in [leading_zero, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
    let from_85 = "proc z {x} {switch -nocase -- $x {a {return $x}}; return none}\n\
                   proc m {x} {if {$x in {a b c}} {set y $x; puts $y; return $x}; return none}\n\
                   set ::v A; puts [z $::v]\n\
                   set ::v b; puts [m $::v]\n";
    prints_under_releases_from(from_85, "A\nb\nb\n", "8.5");
}

/// The lines, from 1, of the S100 shimmer warnings in `source` under
/// `dialect`, in order.
pub(super) fn shimmer_lines(source: &str, dialect: &str) -> Vec<usize> {
    let registry = static_context_for(dialect).commands();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    let mut lines: Vec<usize> =
        tcl_compiler::shimmer::find_shimmer_warnings_for_cu(&unit, registry)
            .iter()
            .filter(|warning| warning.code == DiagCode::S100)
            .map(|warning| {
                let start = usize::try_from(warning.span.start())
                    .map_or(source.len(), |start| start.min(source.len()));
                source[..start].matches('\n').count() + 1
            })
            .collect();
    lines.sort_unstable();
    lines
}

/// A `string is` test types its arm: once `string is integer -strict $i`
/// holds, `i` has an integer representation (`tcl::unsupported::representation`
/// reads `int`, tclsh 8.6 to 9.1), so reading it as an index in the arm
/// converts nothing, while past the arm, where the test may have failed,
/// the read still converts a string — S100 on line 6 and not on line 4. The
/// program prints `b` twice under every release, before and after `tcl opt`.
#[test]
fn a_string_is_test_types_its_arm() {
    crate::install_surfaces();
    let source = "proc p {s} {\n    set i [string trim $s]\n    if {[string is integer -strict $i]} {\n        puts [lindex {a b c} $i]\n    }\n    puts [lindex {a b c} $i]\n}\nset ::v 1\np $::v\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(shimmer_lines(source, dialect), vec![6], "{dialect}");
    }
    prints_under_every_release(source, "b\nb\n");
}

/// A definition in a refined arm takes the arm's value: `set y $x` inside
/// `if {$x eq "a"}` makes `y` the constant `a`, so O100 inlines it where it
/// is read (`puts $y` becomes `puts a`), while a read of `x` itself keeps the
/// version's own value and stays as written. The program prints `a` twice
/// for `a` and nothing for `b` under tclsh 8.4 to 9.1, before and after
/// `tcl opt`.
#[test]
fn a_definition_in_a_refined_arm_takes_the_arms_value() {
    crate::install_surfaces();
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        set y $x\n        puts $y\n        puts $x\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let y = function.ssa.var_symbol("y").expect("y");
        assert_eq!(
            function.sccp.values.get(&(y, 1)),
            Some(&text("a")),
            "{dialect}"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(rewritten.contains("puts a\n"), "{dialect}:\n{rewritten}");
        assert!(rewritten.contains("puts $x"), "{dialect}:\n{rewritten}");
        assert!(
            rewrites
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O100),
            "{dialect}: {rewrites:?}"
        );
    }
    prints_under_every_release(source, "a\na\n");
}

/// A range from a comparison holds of an integer operand, and the bounds
/// checks take one only for an index proved an integer: `end` passes `$i > 5`
/// as a string and indexes `c` (#2368's program, its `gets stdin` a
/// parameter here), and `7.0` fails `$i != 7` and is no index, `lindex`
/// raising `bad index "7.0"` (the review's program), so W230 reports
/// neither, under every dialect. Proved an integer by `string is integer
/// -strict`, `9` past `$i > 5` is past the end: W230 reports it, and tclsh
/// prints `<>`.
#[test]
fn a_range_from_a_comparison_narrows_only_a_proved_integer() {
    crate::install_surfaces();
    let end = "proc q {s} {\n    set i [string trim $s]\n    if {$i > 5} {\n        set x [lindex {a b c} $i]\n        puts $x\n    }\n}\nq end\n";
    let point = "proc g {s} {\n    set i [string trim $s]\n    if {$i != 7} { puts no } else { set x [lindex {a b c} $i]; puts $x }\n}\ng 7.0\n";
    let proved = "proc h {s} {\n    set i [string trim $s]\n    if {[string is integer -strict $i] && $i > 5} {\n        set x [lindex {a b c} $i]\n        puts <$x>\n    }\n}\nh 9\n";
    for dialect in TCL_CORE_DIALECTS {
        assert!(!reports(end, dialect, DiagCode::W230), "{dialect}: end");
        assert!(!reports(point, dialect, DiagCode::W230), "{dialect}: 7.0");
        assert!(reports(proved, dialect, DiagCode::W230), "{dialect}: 9");
    }
    prints_under_every_release(end, "c\n");
    prints_under_every_release(proved, "<>\n");
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(point, &dialect_of(series));
        for program in [point, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, String::new())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}
