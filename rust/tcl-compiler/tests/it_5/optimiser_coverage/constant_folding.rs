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

//! O100, O101, O104 and O105: constant propagation and folding, with the redefinition, trace and global-write guards.

use super::*;

#[test]
fn o100_constant_propagation() {
    // A propagated constant either folds the condition (O101) or eliminates the
    // `if` outright (O112 via SCCP).
    assert!(opt_any_of(
        "set a 10\nif {$a > 5} { puts yes }",
        TCL,
        &["O101", "O112"]
    ));
    assert!(opt_any_of(
        "set a true\nif {$a} { puts yes }",
        TCL,
        &["O100", "O101", "O112"]
    ));
    assert!(opt_any_of(
        "set a false\nif {$a} { puts yes }",
        TCL,
        &["O100", "O101", "O112"]
    ));

    // tclsh: `set a 1; set a 2; set b [expr {$a + 1}]` ⇒ b == 3. The reassign
    // (`set a 2`) needs the fixpoint helper to reach the fold (one pass removes
    // the dead store and forwards the literal; the multipass folds `b`), as in
    // optimiser.rs.
    let registry = static_context_for(TCL).commands();
    let (reassign, _) = optimise_source_multipass(
        "set a 1\nset a 2\nset b [expr {$a + 1}]",
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(reassign.contains("set b 3"));
    // tclsh: a=3, b=4 ⇒ a+b == 7.
    assert!(optimised("set a 3\nset b 4\nset c [expr {$a + $b}]", TCL).contains("set c 7"));
    // tclsh: a=-5 ⇒ -5 + 10 == 5.
    assert!(optimised("set a -5\nset b [expr {$a + 10}]", TCL).contains("set b 5"));
    // tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0 print `0` for the source and for
    // `puts 0`. With a consumer the propagation completes to the literal, and
    // the store of `a`, read only in the dead `set b`, goes with it as in
    // `o100_o105_const_ref_propagation`.
    assert_eq!(
        optimised("set a 0\nset b [expr {$a + 0}]\nputs $b", TCL),
        "puts 0"
    );
}

// O101 — fold pure constant expressions

#[test]
fn o101_constant_folding_arithmetic() {
    // Each value tclsh9.0-verified.
    assert!(optimised("set v [expr {2 + 3}]", TCL).contains("set v 5")); // tclsh: 5
    assert!(optimised("set v [expr {10 - 7}]", TCL).contains("set v 3")); // tclsh: 3
    assert!(optimised("set v [expr {6 * 7}]", TCL).contains("set v 42")); // tclsh: 42
    assert!(optimised("set v [expr {10 / 3}]", TCL).contains("set v 3")); // tclsh: 3 (int trunc)
    assert!(optimised("set v [expr {10 % 3}]", TCL).contains("set v 1")); // tclsh: 1
    assert!(optimised("set v [expr {1 << 4}]", TCL).contains("set v 16")); // tclsh: 16
    assert!(optimised("set v [expr {16 >> 2}]", TCL).contains("set v 4")); // tclsh: 4
    assert!(optimised("set v [expr {2 ** 8}]", TCL).contains("set v 256")); // tclsh: 256
    assert!(optimised("set v [expr {(1 + 2) * (3 + 4)}]", TCL).contains("set v 21")); // tclsh: 21
}

#[test]
fn o101_constant_folding_bitwise_and_compare() {
    // Bitwise — tclsh: 0xFF&0x0F==15, 0xF0|0x0F==255, 0xFF^0xFF==0.
    assert!(optimised("set v [expr {0xFF & 0x0F}]", TCL).contains("15"));
    assert!(optimised("set v [expr {0xF0 | 0x0F}]", TCL).contains("255"));
    assert!(optimised("set v [expr {0xFF ^ 0xFF}]", TCL).contains('0'));
    // Comparisons — tclsh: 3>2==1, 1>2==0, 5==5 is 1, 5!=5 is 0.
    assert!(optimised("set v [expr {3 > 2}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {1 > 2}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {5 == 5}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {5 != 5}]", TCL).contains("set v 0"));
}

#[test]
fn o101_constant_folding_ternary_unary_logical() {
    // tclsh: 1?42:99==42, 0?42:99==99.
    assert!(optimised("set v [expr {1 ? 42 : 99}]", TCL).contains("set v 42"));
    assert!(optimised("set v [expr {0 ? 42 : 99}]", TCL).contains("set v 99"));
    // tclsh: !0==1, -(-5)==5.
    assert!(optimised("set v [expr {!0}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {-(-5)}]", TCL).contains("set v 5"));
    // tclsh: 1&&1==1, 0||1==1.
    assert!(optimised("set v [expr {1 && 1}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {0 || 1}]", TCL).contains("set v 1"));

    // A constant branch condition triggers structure elimination (O112). tclsh:
    // 2+2==4 ⇒ 1 ⇒ the `if` body always runs.
    assert!(opt_fires("if {2 + 2 == 4} { puts yes }", TCL, "O112"));
    // No fold when the expr reads a variable.
    assert!(opt_absent("set v [expr {$x + 1}]", TCL, "O101"));
}

/// An F5 unit folds a constant expression as Tcl 8.4 computes it: F5 Tcl is
/// read as its 8.4 base until a BIG-IP measurement says otherwise (checks 10
/// and 13 to 15 in `docs/work/name-handling/bigip-name-checks.md`). tclsh 8.4.20
/// prints 3 and 9 for the first two. `1 << 70` shifts a C integer past its
/// width, which is undefined (tclsh 8.4.20 here prints 0, a masked build 64),
/// and the last raises `floating-point value too large to represent`; both
/// stay as written.
#[test]
fn an_f5_unit_folds_a_constant_expression_as_tcl_8_4_does() {
    for dialect in ["f5-irules", "f5-iapps", "f5-tmsh"] {
        for (source, folded) in [
            ("set v [expr {1 + 2}]", "set v 3"),
            ("set v [expr {010 + 1}]", "set v 9"),
            ("set v [expr {1 << 70}]", "set v [expr {1 << 70}]"),
            ("set v [expr {1e308 * 10}]", "set v [expr {1e308 * 10}]"),
        ] {
            assert_eq!(optimised(source, dialect), folded, "{dialect}: {source}");
        }
    }
}

#[test]
fn o101_dialect_octal_arithmetic() {
    // TP: `010` is octal 8 under tcl8.x/f5-irules (this suite's default
    // dialect, TCL = "tcl8.6") — tclsh8.6: `010 + 1` = 9, `010 * 2` = 16,
    // `-010` = -8. Before the dialect-aware fix these folded to the
    // decimal reading (11, 20, -10) — a miscompile.
    assert!(optimised("set v [expr {010 + 1}]", TCL).contains("set v 9"));
    assert!(optimised("set v [expr {010 * 2}]", TCL).contains("set v 16"));
    assert!(optimised("set v [expr {-010}]", TCL).contains("set v -8"));
    assert!(optimised("set v [expr {010 + 1}]", IR).contains("set v 9"));

    // TP (control): the same source under plain tcl9.0 reads `010` as
    // decimal (TIP 472) — tclsh9.0: `010 + 1` = 11.
    assert!(optimised("set v [expr {010 + 1}]", "tcl9.0").contains("set v 11"));

    // FN guard: `08` is invalid octal under tcl8.x (Tcl raises an error —
    // tclsh8.6: `expr {08 + 1}` -> "invalid bareword \"08\"") so the
    // const-folder must decline rather than guess a decimal reading.
    assert!(opt_absent("set v [expr {08 + 1}]", TCL, "O101"));
    // Unaffected under tcl9.0, where `08` is decimal 8.
    assert!(optimised("set v [expr {08 + 1}]", "tcl9.0").contains("set v 9"));
}

#[test]
fn o101_expr_redefinition_guard() {
    // FP guard: once `expr` is renamed, `expr {1 + 2}` no longer calls the
    // builtin evaluator — O101 must not fold it.
    assert!(opt_absent(
        "rename expr real_expr\nexpr {1 + 2}",
        TCL,
        "O101"
    ));
    // TN/control: an unrelated rename elsewhere in the module doesn't
    // block the fold (the gate is keyed on `expr` specifically).
    assert!(opt_fires(
        "rename puts real_puts\nexpr {1 + 2}",
        TCL,
        "O101"
    ));
}

#[test]
fn o101_mathfunc_redefinition_guard() {
    // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin math
    // function everywhere in the module — real Tcl compiles `abs(x)` to a
    // `tcl::mathfunc::abs` command invocation, so once that's
    // user-defined, `expr {abs(-5)}` no longer means "call the C abs".
    assert!(opt_absent(
        "proc ::tcl::mathfunc::abs {x} { return 999 }\nexpr {abs(-5)}",
        TCL,
        "O101"
    ));
    // TN/control: no override present, folds as usual. tclsh: abs(-5)=5.
    assert!(optimised("set v [expr {abs(-5)}]", TCL).contains("set v 5"));
}

#[test]
fn o101_global_write_across_call_guard() {
    // FP guard (repro A): `mutate` writes the global `x` via `global x; set x
    // 99`, called between the top-level `set x 5` and the `expr {$x + 1}`
    // read. tclsh: `mutate` runs before the `puts`, so `x` is 99 there —
    // `expr {$x + 1}` is 100, NOT 6. Before the CFG builder widened the
    // call's `defs` with the callee's global-write summary, SCCP treated `x`
    // as still holding the SSA value from the top-level `set x 5` across the
    // opaque `mutate` call, and O101 wrongly folded to `puts 6`.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99 }\nset x 5\nmutate\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TN/control: an unrelated global write in a *different* proc must not
    // widen `x` — the fix is name-precise, not a coarse "any call may
    // mutate any global" widening. tclsh: `mutate_y` never touches `x`, so
    // `expr {$x + 1}` is still 6 and O101 correctly folds it.
    assert!(
        optimised(
            "proc mutate_y {} { global y; incr y }\nset x 5\nmutate_y\nputs [expr {$x + 1}]",
            TCL,
        )
        .contains("puts 6")
    );

    // TP (transitive): `outer` doesn't write `x` itself but calls `mutate`,
    // which does — the call-graph closure must still invalidate `x` at the
    // `outer` call site. tclsh: same 100 result as the direct-call repro.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99 }\nproc outer {} { mutate }\nset x 5\nouter\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (canonical alias call): lowering retains `call_mutate` for source
    // fidelity and `mutate` as its resolved target. The global-write call
    // graph and the call-site invalidation must both follow that target.
    // Tcl 9.0.4 yields 100, never the stale fold 6.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99 }\ninterp alias {} call_mutate {} mutate\nset x 5\ncall_mutate\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (namespace variable): `variable` writes are outer-scope too.
    // tclsh: same reasoning as the `global` repro — 100, not 6.
    assert!(opt_absent(
        "namespace eval ::ns { variable x 5 }\nproc ::ns::mutate {} { variable x; set x 99 }\nset ::ns::x 5\n::ns::mutate\nputs [expr {$::ns::x + 1}]",
        TCL,
        "O101"
    ));

    // TP (embedded command substitution): the same soundness gap reached via
    // `set y [mutate]` rather than a bare `mutate` statement — the
    // embedded-substitution scan must catch it too.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99; return 1 }\nset x 5\nset y [mutate]\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (conditional-branch write): sound over-approximation — a global
    // write on a branch that may not execute still invalidates the name at
    // the call site (the analysis is flow-insensitive by design).
    assert!(opt_absent(
        "proc mutate {} { global x\nif {$::cond} { set x 99 } }\nset x 5\nmutate\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));
}

#[test]
fn o101_module_wide_variable_trace_guard() {
    // FP guard (repro B): a trace on `x` is installed by `install_trace`, a
    // *different* proc than the code performing the final `set x 6` / read
    // — the flow-sensitive TRACED flag (installed and read in the same
    // function body) can't see this; it needs the module-wide fact. tclsh:
    // the write trace rewrites `x` to 99 on every write, so `set x 6` really
    // leaves `x` == 99 and `expr {$x + 1}` is 100, NOT 7.
    assert!(opt_absent(
        "proc install_trace {} { trace add variable ::x write {apply {a {set ::x 99}}} }\nset x 5\ninstall_trace\nset x 6\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TN/control: an unrelated trace (on a different variable) must not
    // widen `x` — the fix is name-precise. tclsh: `y` is never written by
    // this trace, `x` stays a plain local, so `expr {$x + 1}` still folds.
    assert!(
        optimised(
            "proc cb {args} {}\ntrace add variable y write cb\nset x 6\nputs [expr {$x + 1}]",
            TCL,
        )
        .contains("puts 7")
    );

    // A callback the module cannot see may write any global, `x` included,
    // whichever variable the trace is on.
    assert!(opt_absent(
        "trace add variable y write cb\nset x 6\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (trace removed later still declines to fold): the fact is
    // flow-insensitive by design — a later `trace remove` does not
    // "untrace" a name for this analysis, so this only ever prevents an
    // otherwise-unsound fold, it never wrongly blocks one tclsh agrees with.
    assert!(opt_absent(
        "trace add variable x write cb\ntrace remove variable x write cb\nset x 6\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));
}

#[test]
fn o100_branch_condition_propagation_blocks_on_variable_trace() {
    // The same silent-miscompile class as O102, but through
    // `branch_folding::propagate_into_branches`'s *own* SCCP-constants
    // projection (a duplicate of `propagation::sccp_constants_for` this fix
    // also deduplicated) rather than `propagation::run_function`'s. A read
    // trace installed by a called proc must still block substituting `x`'s
    // literal into the `if` condition text. tclsh: prints "trace fired"
    // then "yes" — `$x` must survive as a real runtime read.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x > 0} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O100"));
    assert!(optimised(src, TCL).contains("if {$x > 0}"));
}

// O104 — fold string build chains

#[test]
fn o104_string_build_chain() {
    // tclsh: {Hello}+{ }+World ⇒ "Hello World".
    assert_eq!(
        optimised("set msg {Hello}\nappend msg { }\nappend msg World", TCL),
        "set msg {Hello World}"
    );

    // A multi-append chain folds (O104).
    assert!(opt_fires(
        "set msg {a}\nappend msg {b}\nappend msg {c}\nappend msg {d}",
        TCL,
        "O104"
    ));
    // set + single append is a valid 2-write chain.
    assert!(opt_fires(
        "set msg {Hello}\nappend msg { World}",
        TCL,
        "O104"
    ));
    // Empty initial value still chains.
    assert!(opt_fires(
        "set msg {}\nappend msg hello\nappend msg { world}",
        TCL,
        "O104"
    ));

    // Dynamic word blocks the fold.
    assert!(opt_absent("set msg {Hello}\nappend msg $name", TCL, "O104"));
    // A read between writes blocks the chain.
    assert!(opt_absent(
        "set msg {Hello}\nputs $msg\nappend msg { World}",
        TCL,
        "O104"
    ));
    // An eval/uplevel barrier breaks the chain.
    assert!(opt_absent(
        "set msg {Hello}\neval {puts hi}\nappend msg { World}",
        TCL,
        "O104"
    ));

    // Gap (sound): the optimiser does not fold a chain that straddles an
    // intervening statement (`puts ok`) into `set msg {Hello World}`; it leaves
    // `append` intact. tclsh: both forms bind msg to "Hello World", so leaving it
    // is sound.
    assert!(
        optimised("set msg {Hello}\nputs ok\nappend msg { World}", TCL)
            .contains("append msg { World}")
    );
    assert!(opt_absent(
        "set msg {Hello}\nputs ok\nappend msg { World}",
        TCL,
        "O104"
    ));
}

// O100 / O105 — propagate constants into command refs and strings

#[test]
fn o100_o105_const_ref_propagation() {
    // tclsh: x=42 ⇒ `puts 42`.
    assert_eq!(optimised("set x 42\nputs $x", TCL), "puts 42");
    // Two reads both fold. tclsh: x=5 ⇒ two `puts 5`.
    assert_eq!(
        optimised("set x 5\nputs $x\nputs $x", TCL),
        "puts 5\nputs 5"
    );
    // Through a chain. tclsh: a=1 ⇒ b==2 ⇒ `puts 2`.
    assert_eq!(
        optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL),
        "puts 2"
    );

    // String interpolation: $x inside "...". tclsh: x=5 ⇒ "val=5".
    // The string-interp fold is labelled inline as O100 (+ O102/O109 DSE) rather
    // than O105. Same observable output.
    let s = "set x 5\nputs \"val=$x\"";
    assert!(optimised(s, TCL).contains("puts \"val=5\""));
    assert!(opt_fires(s, TCL, "O100"));

    // Multiple vars in a string. tclsh: a=10,b=20 ⇒ "10 20".
    assert!(optimised("set a 10\nset b 20\nputs \"$a $b\"", TCL).contains("puts \"10 20\""));

    // ${x} standalone word — no trailing brace. tclsh: x=7 ⇒ `puts 7`.
    let braced = optimised("set x 7\nputs ${x}", TCL);
    assert!(!braced.contains("puts 7}"));
    assert!(braced.contains("puts 7"));

    // ${x} inside a string. tclsh: `puts "7"`.
    let bstr = optimised("set x 7\nputs \"${x}\"", TCL);
    assert!(!bstr.contains("\"7}\""));
    assert!(bstr.contains('7'));

    // A *safe* string constant folds into a string. tclsh: x=hello ⇒ "val=hello".
    assert!(optimised("set x hello\nputs \"val=$x\"", TCL).contains("puts \"val=hello\""));
    // A value with a space folds into the string. tclsh: "a b" ⇒ "val=a b".
    assert!(optimised("set x \"a b\"\nputs \"val=$x\"", TCL).contains("puts \"val=a b\""));

    // An *unsafe* value (contains `$` / `[`) must NOT fold into a string — it
    // would inject a fresh substitution. (Structural: no O100/O105 fires.)
    assert!(opt_absent("set x {$y}\nputs \"val=$x\"", TCL, "O100"));
    assert!(opt_absent("set x {$y}\nputs \"val=$x\"", TCL, "O105"));
    assert!(opt_absent("set x {[exit]}\nputs \"val=$x\"", TCL, "O100"));

    // Combined string-interp + expr fold + DSE. tclsh: x=5 ⇒ "x=5" and y==6.
    assert_eq!(
        optimised(
            "set x 5\nset y [expr {$x + 1}]\nputs \"x=$x\"\nputs $y",
            TCL
        ),
        "puts \"x=5\"\nputs 6"
    );

    // Dynamic value is not propagated.
    assert!(opt_absent("set x [clock seconds]\nputs $x", TCL, "O100"));
    // Array reference is not propagated (shape guard).
    let arr = "set x $arr(key)\nputs $x";
    assert_eq!(optimised(arr, TCL), arr);
    assert!(opt_absent(arr, TCL, "O100"));
}

/// #2129: the script encoding decides a non-ASCII literal's characters. A
/// file holding the UTF-8 bytes of `É` gives, for `string length` and
/// `string range "Éabc" 1 2`, 2 and `\x89a` on tclsh 8.6.18 under
/// `LC_ALL=C` (`encoding system` is `iso8859-1`), 1 and `ab` under
/// `LC_ALL=C.UTF-8`, and 1 and `ab` on tclsh 9.0.4 under either locale. The
/// front end decodes UTF-8, so a fold over such a literal is admissible only
/// from 9.0; ASCII operands fold under every release.
#[test]
fn non_ascii_literal_folds_only_where_the_release_reads_source_as_utf8() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let utf8_source = dialect.starts_with("tcl9");
        let range = optimised("puts [string range \"Éabc\" 1 2]\n", dialect);
        assert_eq!(
            range == "puts ab\n",
            utf8_source,
            "{dialect}: string range over a non-ASCII literal gave {range:?}",
        );
        for src in [
            "puts [string length \"É\"]\n",
            "puts [string toupper \"é\"]\n",
            "proc p {} { set s \"Éa\"; return [string index $s 1] }\n",
        ] {
            if !utf8_source {
                assert_eq!(optimised(src, dialect), src, "{dialect}: {src:?} folded");
            }
        }
        assert_eq!(
            optimised("puts [string range \"Eabc\" 1 2]\n", dialect),
            "puts ab\n",
            "{dialect}: the ASCII control must fold",
        );
    }
}

/// A variable proved to hold a source literal folds into an expression under
/// the unit's own number grammar. Whatever reads the shared literal object
/// first, its numeric reading is a function of its string: for `set a 1`
/// followed by `format %f $a`, `format %e $a`, `lsort -real [list $a 0]`,
/// `binary format d $a`, `expr {double($a)}`, `string is double $a` or
/// `incr a 0`, then `expr {$a + 1}`, at top level and in a procedure, tclsh
/// 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print `2`. Read the same way,
/// `010` gives 9 on 8.4 to 8.6 and 11 on 9.0 and 9.1, and `1.0` gives 2.0.
/// The fold is taken where the release has the integer protocol it reads
/// with (8.5 on). 8.4 keeps the expression because that protocol's gate is
/// about 8.4's arithmetic, which other proofs share, not because 8.4 reads
/// the literal differently; F5 units keep it until a device answers
/// (`bigip-name-checks.md` item 17), and so does the double literal.
#[test]
fn expr_operand_literal_reading() {
    for (dialect, source, expected) in [
        (
            "tcl8.6",
            "set a 1\nset b [expr {$a + 1}]\nputs $b",
            "puts 2",
        ),
        (
            "tcl9.0",
            "set a 1\nset b [expr {$a + 1}]\nputs $b",
            "puts 2",
        ),
        (
            "tcl8.6",
            "set a 010\nputs [expr {$a + 1}]",
            "set a 010\nputs 9",
        ),
        (
            "tcl9.0",
            "set a 010\nputs [expr {$a + 1}]",
            "set a 010\nputs 11",
        ),
        (
            "tcl8.4",
            "set a 010\nputs [expr {$a + 1}]",
            "set a 010\nputs [expr {$a + 1}]",
        ),
        (
            "tcl8.6",
            "set a 1.0\nputs [expr {$a + 1}]",
            "set a 1.0\nputs [expr {$a + 1}]",
        ),
    ] {
        assert_eq!(
            optimised(source, dialect),
            expected,
            "{dialect}: {source:?}"
        );
    }
}
