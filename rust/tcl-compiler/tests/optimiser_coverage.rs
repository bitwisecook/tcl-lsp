//! The comprehensive optimiser *coverage* suite (O100–O129, the shimmer
//! S100–S102 pipeline, and the GVN/CSE/LICM redundancy detector).
//!
//! ## Driving the optimiser
//!
//! Three entry points are exercised:
//!   * `apply_optimisations(src, &optimise_with_dialect(...))` for the rewritten
//!     text, and the `Vec<Optimisation>` for the `O1xx` codes.
//!   * the GVN/CSE/LICM finder: `find_redundancies_for_cu`,
//!     `find_partial_redundancies_for_cu`, and `find_loop_invariants_for_cu` over
//!     a `CompilationUnit` (full/partial → O105, loop → O106).
//!   * the shimmer/thunking detector: `find_shimmer_warnings_for_cu` (S100/S101)
//!     + `find_thunking_warnings_for_cu` (S102).
//!
//! De-Morgan / inversion logic lives inside the O110 `InstCombine` pass on the
//! expr AST (there is no standalone string→string helper). Those cases are
//! exercised as O110-driven `[expr {...}]` rewrites, the form `optimiser.rs`
//! already uses.
//!
//! ## C-Tcl proof
//!
//! Every assertion that encodes a Tcl-language *value* fact (a fold result, an
//! arithmetic outcome, an evaluation result) was checked against `tclsh9.0`
//! (and the De-Morgan / strength-reduction / self-comparison families were
//! exhaustively swept over their operands), with the verified value cited in a
//! `// tclsh:` comment. Optimiser-structural facts (which `O1xx`/`S1xx` code
//! fired, that a store was eliminated) are about the compiler IR, not Tcl
//! semantics, and are flagged as such.
//!
//! ## Notes on behaviour
//!
//! Where the optimiser is confirmed *sound* against tclsh (more aggressive, or a
//! different but equivalent O-code), the assertion is adapted to the actual
//! behaviour and commented at the site. Gaps (the optimiser soundly leaves a
//! construct unchanged) assert the conservative no-op. A genuine miscompile
//! (`$x - $x` → 0 on an untyped operand) has since been FIXED (O110 now gates the
//! self-subtraction/xor fold on a provably-numeric operand); the assertion
//! asserts the corrected behaviour. No `#[ignore]`; every test passes.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_compiler::shimmer::{find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu};
use tcl_registry::registry_for_dialect;

const TCL: &str = "tcl8.6";
const IR: &str = "f5-irules";

// ---------------------------------------------------------------------------
// Helpers (mirror optimiser.rs: opt_codes / opt_fires / optimised)
// ---------------------------------------------------------------------------

/// Sorted, de-duplicated `Oxxx` codes emitted by a single optimiser pass.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    let mut v: Vec<String> = optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code.as_str() == code)
}

/// True if NONE of the optimisations carry `code`.
fn opt_absent(src: &str, dialect: &str, code: &str) -> bool {
    !opt_fires(src, dialect, code)
}

/// Apply all optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// True if any emitted code is in `wanted`.
fn opt_any_of(src: &str, dialect: &str, wanted: &[&str]) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| wanted.contains(&o.code.as_str()))
}

/// GVN/CSE/LICM codes: the union of full + partial redundancies (→ O105) and
/// loop invariants (→ O106).
fn gvn_codes(src: &str) -> Vec<String> {
    let registry = registry_for_dialect(TCL);
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for r in find_redundancies_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    for r in find_partial_redundancies_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    for r in find_loop_invariants_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    v.sort();
    v
}

/// Shimmer-warning codes (S100/S101 + S102 thunking).
fn shimmer_codes(src: &str) -> Vec<String> {
    let registry = registry_for_dialect(TCL);
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&cu, registry) {
        v.push(w.code.as_str().to_owned());
    }
    for w in find_thunking_warnings_for_cu(&cu) {
        v.push(w.code.as_str().to_owned());
    }
    v.sort();
    v
}

/// Count shimmer warnings carrying `code` (S100/S101/S102).
fn shimmer_count(src: &str, code: &str) -> usize {
    shimmer_codes(src).iter().filter(|c| *c == code).count()
}

/// Wraps `body` in a loop where `$x` is SCCP-typed INT and live (the
/// precondition the D5-O110 identity/annihilator drops require). Matches the
/// helper in `optimiser.rs`.
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// ===========================================================================
// O100 — constant propagation into expressions
// ===========================================================================

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
    let registry = registry_for_dialect(TCL);
    let (reassign, _) = optimise_source_multipass(
        "set a 1\nset a 2\nset b [expr {$a + 1}]",
        registry,
        Some(TCL),
        10,
    );
    assert!(reassign.contains("set b 3"));
    // tclsh: a=3, b=4 ⇒ a+b == 7.
    assert!(optimised("set a 3\nset b 4\nset c [expr {$a + $b}]", TCL).contains("set c 7"));
    // tclsh: a=-5 ⇒ -5 + 10 == 5.
    assert!(optimised("set a -5\nset b [expr {$a + 10}]", TCL).contains("set b 5"));
    // tclsh: a=0 ⇒ 0 + 0 == 0. In a single pass O110's `+0` identity removal
    // pre-empts the constant fold, leaving `set a 0\nset b [expr {$a}]` (still
    // b==0). With a consumer the propagation completes to the literal — assert
    // the tclsh value end-to-end.
    assert_eq!(
        optimised("set a 0\nset b [expr {$a + 0}]\nputs $b", TCL),
        "set a 0\nputs 0"
    );
}

// ===========================================================================
// O101 — fold pure constant expressions
// ===========================================================================

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

// ===========================================================================
// O102 — fold [expr {...}] command substitution
// ===========================================================================

#[test]
fn o102_expr_command_substitution() {
    // tclsh: 3+4 == 7. A *pure-literal* expr subst is folded by the constant
    // folder (O101); O102 is reserved for an expr that consumed a *propagated*
    // constant. Both produce the same value.
    assert!(optimised("set v [expr {3 + 4}]", TCL).contains("set v 7"));
    assert!(opt_fires("set v [expr {3 + 4}]", TCL, "O101"));

    // Propagated constant into the expr subst ⇒ O102 (+ O100 propagation). tclsh:
    // a=5 ⇒ 5*2 == 10.
    let prop = "set a 5\nset v [expr {$a * 2}]";
    assert!(optimised(prop, TCL).contains("set v 10"));
    assert!(opt_fires(prop, TCL, "O102"));
    assert!(opt_fires(prop, TCL, "O100"));

    // Dynamic expr ([clock seconds]) does not fold via O102.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 1}]",
        TCL,
        "O102"
    ));
}

// ===========================================================================
// O103 — interprocedural folding
// ===========================================================================

#[test]
fn o103_interprocedural_folding() {
    // tclsh: pi == 3.
    assert!(optimised("proc pi {} { return 3 }\nset v [pi]\n", TCL).contains("set v 3"));
    // tclsh: double 21 == 42.
    assert!(
        optimised(
            "proc double {x} { return [expr {$x * 2}] }\nset v [double 21]\n",
            TCL
        )
        .contains("set v 42")
    );
    // tclsh: id 99 == 99.
    assert!(
        optimised("proc id {x} { return $x }\nset a 99\nset v [id $a]\n", TCL).contains("set v 99")
    );
    // tclsh: bump 5 (incr x; return x) == 6.
    assert!(
        optimised("proc bump {x} { incr x; return $x }\nset v [bump 5]\n", TCL).contains("set v 6")
    );
    // tclsh: util::add 10 20 == 30.
    assert!(optimised(
        "namespace eval util {\n    proc add {a b} { return [expr {$a + $b}] }\n}\nset v [util::add 10 20]\n",
        TCL
    )
    .contains("set v 30"));

    // Impure proc (puts) is not folded.
    assert!(opt_absent(
        "proc effect {} { puts hi; return 1 }\nset v [effect]\n",
        TCL,
        "O103"
    ));
    // Recursive proc is not folded (infinite-inline risk).
    assert!(opt_absent(
        "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nset v [fact 5]\n",
        TCL,
        "O103"
    ));
}

// ===========================================================================
// O104 — fold string build chains
// ===========================================================================

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

// ===========================================================================
// O100 / O105 — propagate constants into command refs and strings
// ===========================================================================

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

// ===========================================================================
// O107 / O108 / O109 — dead code & dead store elimination
// ===========================================================================

#[test]
fn o107_dead_code_in_false_branch() {
    // `if {0}` body is unreachable; trailing statement survives.
    let out = optimised("if {0} {\n    puts dead\n}\nputs alive\n", TCL);
    assert!(!out.contains("puts dead"));
    assert!(out.contains("puts alive"));

    // Code after `return` in a proc: must not crash (Rust may or may not emit
    // O107). Assert only that optimisation completes (proc preserved).
    let after_ret = "proc foo {} {\n    return 1\n    puts never\n}\n";
    let _ = opt_codes(after_ret, TCL); // no panic
}

#[test]
fn o108_aggressive_dce() {
    // tclsh: a=1, b reassigned to 10 ⇒ b==10. The dead chain is removed via
    // O108/O109 (either code may carry it).
    let s = "set a 1\nset b [expr {$a + 1}]\nset b 10\nputs $b";
    assert!(optimised(s, TCL).contains("puts 10"));
    assert!(opt_any_of(s, TCL, &["O108", "O109"]));

    // A consumed value is not eliminated. tclsh: a=1 ⇒ b==2 ⇒ `puts 2`.
    assert!(optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL).contains("puts 2"));

    // Multi-step transitively-dead chain. tclsh: final c==99 ⇒ `puts 99`.
    let multi = "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset c 99\nputs $c\n";
    assert!(optimised(multi, TCL).contains("puts 99"));
    assert!(opt_any_of(multi, TCL, &["O108", "O109"]));
}

#[test]
fn o109_dead_store_elimination() {
    // First store dead (O109). tclsh: a==2.
    let s = "set a 1\nset a 2\nputs $a";
    assert!(opt_fires(s, TCL, "O109"));
    assert!(optimised(s, TCL).contains("puts 2"));

    // tclsh: x final 200.
    assert!(optimised("set x 100\nset x 200\nputs $x", TCL).contains("puts 200"));

    // eval reads all vars — the store feeding `[eval $s]` survives (not DSE'd).
    let esc = "set n [eval $s]\nset n 1\nputs $n";
    assert_eq!(
        optimised(esc, TCL).lines().next().unwrap(),
        "set n [eval $s]"
    );

    // A dead store inside a proc fires O109.
    assert!(opt_fires(
        "proc foo {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
        TCL,
        "O109"
    ));

    // Call-by-name suppression: a local passed BY NAME to a proc with upvar
    // VAR_READ/VAR_WRITE traits is indirectly read — its store must survive
    // (no O109/O126). Deleting it would feed an uninitialised name to the callee.
    let byname = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set x \"hello\"\n    reader x\n}\n";
    assert!(opt_absent(byname, TCL, "O109"));
    assert!(opt_absent(byname, TCL, "O126"));

    // TP control: a dead store on a DIFFERENT (not passed-by-name) var still
    // fires O109.
    let byname_tp = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set unrelated \"discarded\"\n    set unrelated \"actual\"\n    set y \"passed\"\n    reader y\n    return $unrelated\n}\n";
    assert!(opt_fires(byname_tp, TCL, "O109"));
}

// ===========================================================================
// O110 — InstCombine: reassociation / identity / annihilator
// ===========================================================================

#[test]
fn o110_reassociation() {
    // tclsh sweep: $a + 1 + 2 == $a + 3 (all $a).
    assert!(optimised("set v [expr {$a + 1 + 2}]", TCL).contains("$a + 3"));
    // tclsh sweep: $a * 2 * 3 == $a * 6.
    assert!(optimised("set v [expr {$a * 2 * 3}]", TCL).contains("$a * 6"));
    // tclsh sweep: $a + 3 - 1 == $a + 2.
    assert!(optimised("set v [expr {$a + 3 - 1}]", TCL).contains("$a + 2"));
    // Real fold through reassoc at top level: ($a + 1) + 2 flattens to $a + 3.
    // tclsh sweep: ($a + 1) + 2 == $a + 3.
    assert!(optimised("set v [expr {($a + 1) + 2}]", TCL).contains("$a + 3"));
    assert!(opt_fires("set v [expr {($a + 1) + 2}]", TCL, "O110"));
    // NOTE (gap, sound): the same reassoc inside a `proc ... return [expr {...}]`
    // value position is NOT folded (no code fires); leaving it is sound. Given
    // `proc f {a} { return [expr {($a + 1) + 2}] }` no optimisation fires.
    assert!(opt_codes("proc f {a} { return [expr {($a + 1) + 2}] }", TCL).is_empty());
}

#[test]
fn o110_identity_and_annihilator() {
    // Identity/annihilator drops need a provably-INT operand (D5-O110) — wrap in
    // `int_x`. tclsh sweep (x≥0): x**0==1, x**1==x, x<<0==x, x>>0==x, x&0==0,
    // x|0==x, x^0==x, x%1==0, x*0==0, x*1==x, x+0==x, x-0==x, x/1==x.
    assert!(optimised(&int_x("set v [expr {$x ** 0}]"), TCL).contains("set v 1")); // tclsh: 1
    assert!(opt_fires(&int_x("set v [expr {$x ** 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x << 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x >> 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x & 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x | 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x ^ 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised(&int_x("set v [expr {$x * 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x * 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x - 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x / 1}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0"));

    // Left-side identity (commutativity). tclsh sweep: 0|x==x, 0^x==x.
    assert!(opt_fires(&int_x("set v [expr {0 | $x}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {0 ^ $x}]"), TCL, "O110"));
}

#[test]
fn o110_logical_identities() {
    // Logical annihilators/identities do not require an INT proof (boolean
    // context). tclsh sweep: x&&0==0, x||1==1, 0&&x==0, 1||x==1.
    assert!(optimised("set v [expr {$x && 0}]", TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised("set v [expr {$x || 1}]", TCL).contains("set v 1")); // tclsh: 1
    assert!(optimised("set v [expr {0 && $x}]", TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised("set v [expr {1 || $x}]", TCL).contains("set v 1")); // tclsh: 1
    // x&&1, x||0, 1&&x, 0||x simplify (O110) without becoming a constant.
    assert!(opt_fires("set v [expr {$x && 1}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {$x || 0}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {1 && $x}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {0 || $x}]", TCL, "O110"));
}

#[test]
fn o110_whitespace_noise_suppressed() {
    // Whitespace-only differences must NOT fire O110 (the user's spacing is not a
    // defect). All confirmed no-O110 in Rust.
    for src in [
        "set v [expr {$x-1}]",
        "set v [expr {$x*$y}]",
        "set v [expr {$a*$b/$c}]",
        "set v [expr {$secure ? 6697: 6667}]",
        "set v [expr {$tagnumber >>7}]",
        "proc f {x} { if {$x<0} { return -1 }; return 1 }",
        "proc f {x} { while {$x>0} { incr x -1 } }",
        // Pure commutative reorder — no win.
        "proc f {x} { return [expr {1 + $x}] }",
        "proc f {x} { return [expr {2 * $x}] }",
        // Mixed bitwise/shift parens are kept for readability (CERT EXP00-C).
        "set v [expr {($x >> 16) & 0xFF}]",
        // Gaps (sound): `$x - -1` ⇒ `$x + 1` and the same-op bitwise chain
        // `(a & b) & c` could fire O110 but NEITHER does (no rewrite) — leaving
        // both unchanged is sound.
        "proc f {x} { return [expr {$x - -1}] }",
        "set v [expr {($a & $b) & $c}]",
    ] {
        assert!(opt_absent(src, TCL, "O110"), "spurious O110: {src}");
    }

    // Genuine structural changes that DO still fire O110.
    // Additive identity `$x + 0` ⇒ `$x` (INT-proven).
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    // Boolean simplify `($action && 1) ? ...` ⇒ `$action ? ...`.
    assert!(opt_fires(
        "set v [expr {($action && 1) ? \"on\" : \"off\"}]",
        TCL,
        "O110"
    ));
}

#[test]
fn o110_self_comparison_tautologies() {
    // tclsh sweep (any defined $x, string or numeric — `==`/`<` use the string
    // fallback): x==x ⇒ 1, x!=x ⇒ 0, x<=x ⇒ 1, x>=x ⇒ 1, x<x ⇒ 0, x>x ⇒ 0.
    assert!(optimised("set v [expr {$x == $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x != $x}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {$x <= $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x >= $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x < $x}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {$x > $x}]", TCL).contains("set v 0"));
    // String self-comparison. tclsh: x eq x ⇒ 1, x ne x ⇒ 0 (any $x).
    assert!(optimised("set v [expr {$x eq $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x ne $x}]", TCL).contains("set v 0"));
    // x ^ x ⇒ 0 — needs an INT proof (xor errors on non-numeric strings); the
    // `int_x` wrapper supplies it. tclsh sweep (x≥0): x^x == 0.
    assert!(optimised(&int_x("set v [expr {$x ^ $x}]"), TCL).contains("set v 0"));

    // FIXED (was a miscompile): bare `set v [expr {$x - $x}]` on an UNTYPED $x
    // must NOT fold to `set v 0`. tclsh proves the original RAISES an error when
    // $x is a non-numeric string:
    //   set x hello; expr {$x - $x}  ⇒  "can't use non-numeric string as operand
    //   of \"-\"" (8.6) / "cannot use non-numeric string ... as left operand"
    //   (9.0)  — catch ⇒ 1
    // so folding to 0 would silently swallow that error. Subtraction is
    // arithmetic-only, unlike `==`/`<` (string fallback), so this is not folded.
    assert!(
        !optimised("set v [expr {$x - $x}]", TCL).contains("set v 0"),
        "untyped $x - $x must not fold to 0 (arithmetic op errors on non-numeric)"
    );
    // …but with a provably-numeric operand the fold IS sound. tclsh sweep (x≥0):
    // x-x == 0. The `int_x` wrapper makes $x provably numeric.
    assert!(optimised(&int_x("set v [expr {$x - $x}]"), TCL).contains("set v 0"));
}

#[test]
fn o110_unary_and_not_inversions() {
    // tclsh: !!(bool) collapses (==/!=/< are already boolean).
    assert!(optimised("set v [expr {!!($a == $b)}]", TCL).contains("$a == $b"));
    // tclsh sweep: !(a==b) ⇒ a!=b, !(a<b) ⇒ a>=b, !(a>=b) ⇒ a<b, !(a>b) ⇒ a<=b,
    // !(a<=b) ⇒ a>b, !(a!=b) ⇒ a==b.
    assert!(optimised("set v [expr {!($a == $b)}]", TCL).contains("$a != $b"));
    assert!(optimised("set v [expr {!($a < $b)}]", TCL).contains("$a >= $b"));
    assert!(optimised("set v [expr {!($a >= $b)}]", TCL).contains("$a < $b"));
    assert!(optimised("set v [expr {!($a > $b)}]", TCL).contains("$a <= $b"));
    assert!(optimised("set v [expr {!($a <= $b)}]", TCL).contains("$a > $b"));
    assert!(optimised("set v [expr {!($a != $b)}]", TCL).contains("$a == $b"));
    // String comparison inversions. tclsh: !(a eq b) ⇒ a ne b, !(a ne b) ⇒ a eq b.
    assert!(optimised("set v [expr {!($a eq $b)}]", TCL).contains("$a ne $b"));
    assert!(optimised("set v [expr {!($a ne $b)}]", TCL).contains("$a eq $b"));

    // Double-not in an `if` condition simplifies (O110). tclsh: !!x ⇒ x (bool ctx).
    assert!(opt_fires("if {!!$x} { puts yes }", TCL, "O110"));
    assert!(optimised("if {!!$x} { puts yes }", TCL).contains("if {$x}"));

    // INT-proven unary folds: ~~x ⇒ x, -(-x) ⇒ x, +x ⇒ x. tclsh sweep (x≥0).
    assert!(optimised(&int_x("set v [expr {~~$x}]"), TCL).contains("set v [expr {$x}]"));
    assert!(optimised(&int_x("set v [expr {-(-$x)}]"), TCL).contains("$x"));
    assert!(opt_fires(&int_x("set v [expr {+$x}]"), TCL, "O110"));

    // NOTE on gaps (sound to leave unchanged):
    //  - `!($a in $b)` ⇒ `$a ni $b` and `!($a ni $b)` ⇒ `$a in $b` — in/ni are
    //    not inverted.
    //  - `-(5)` ⇒ `-5` literal negation — not folded (stays `-(5)`).
}

#[test]
fn o110_de_morgan() {
    // tclsh sweep (a,b ∈ {0,1,5}): !($a && $b) == !$a || !$b; !($a || $b) ==
    // !$a && !$b; the 4-var chain matches its inversion (tclsh-proven).
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL).contains("$a != $b || $c >= $d")
    );

    // De Morgan inside an `if` condition is carried under O110 or O113 (the
    // if-condition rewrite path is owned by the strength-reduce pass). The
    // rewrite text matches tclsh either way.
    let in_if = "if {!($x && $y)} { puts yes }";
    assert!(optimised(in_if, TCL).contains("!$x || !$y"));
    assert!(opt_any_of(in_if, TCL, &["O110", "O113"]));
}

#[test]
fn o110_ternary_and_double_not() {
    // Constant-condition ternary simplifies (O110). tclsh: 1?a:b==a, 0?a:b==b.
    assert!(opt_fires("set v [expr {1 ? $a : $b}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {0 ? $a : $b}]", TCL, "O110"));
    // Double-not in an `if`. (Structural: O110 fires.)
    assert!(opt_fires("if {!!$x} { puts yes }", TCL, "O110"));
    // De Morgan inside an `if` is reported under O110/O113.
    assert!(opt_any_of(
        "if {!($x && $y)} { puts yes }",
        TCL,
        &["O110", "O113"]
    ));
    // Boolean-context ternary `($a > $b) ? 1 : 0` simplifies (O110).
    assert!(opt_fires(
        "if {($a > $b) ? 1 : 0} { puts yes }",
        TCL,
        "O110"
    ));

    // NOTE on gaps (sound): `$x ? 0 : 1` ⇒ `!$x`, `$c ? $a : $a` (identical
    // branches), and `!$c ? $a : $b` ⇒ `$c ? $b : $a` (cond flip) are not folded.
    assert!(opt_absent("set v [expr {$x ? 0 : 1}]", TCL, "O110"));
}

#[test]
fn o110_no_instcombine_with_command_subst() {
    // x+0=x is unsafe when x has side effects ([clock seconds]). No O110.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 0}]",
        TCL,
        "O110"
    ));
}

// ===========================================================================
// O112 — structure elimination
// ===========================================================================

#[test]
fn o112_if_elimination() {
    // tclsh: `if {1} {set x 1}` runs the body ⇒ unwrap.
    let it = optimised("if {1} {\n    set x 1\n}", TCL);
    assert!(!it.contains("if"));
    assert!(it.contains("set x 1"));

    // `if {0}` deletes the body; trailing statement survives.
    let iff = optimised("if {0} {\n    set x 1\n}\nputs alive", TCL);
    assert!(!iff.contains("set x 1"));
    assert!(iff.contains("puts alive"));

    // `if {0} ... else ...` keeps the else body. tclsh: else runs.
    let ie = optimised("if {0} {\n    set x 1\n} else {\n    set y 2\n}", TCL);
    assert!(!ie.contains("set x 1"));
    assert!(ie.contains("set y 2"));

    // elseif chain selects the first true clause. tclsh: `set b 2` runs.
    let el = optimised(
        "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}",
        TCL,
    );
    assert!(el.contains("set b 2"));
    assert!(!el.contains("set a 1"));
    assert!(!el.contains("set c 3"));

    // All conditions false, no else ⇒ whole thing gone, trailing survives (O112).
    let all_false = "if {0} { set a 1 } elseif {0} { set b 2 }\nputs done";
    assert!(optimised(all_false, TCL).contains("puts done"));
    assert!(opt_fires(all_false, TCL, "O112"));

    // Runtime condition `if {$x}` is untouched.
    assert!(opt_absent("if {$x} {\n    set y 1\n}", TCL, "O112"));
}

#[test]
fn o112_while_for_elimination() {
    // `while {0}` deletes the loop.
    let wf = "while {0} {\n    puts looping\n}\nputs done";
    assert!(!optimised(wf, TCL).contains("puts looping"));
    assert!(optimised(wf, TCL).contains("puts done"));
    assert!(opt_fires(wf, TCL, "O112"));

    // `while {1}` is an infinite loop — NOT eliminated by O112. (The constant `1`
    // condition folds as O101 but the loop is never deleted; O112 does not fire.)
    assert!(opt_absent("while {1} {\n    break\n}", TCL, "O112"));

    // `for {init} {0} ...` keeps only the init.
    let ff = "for {set i 0} {0} {incr i} {\n    puts looping\n}";
    assert!(!optimised(ff, TCL).contains("puts looping"));
    assert!(optimised(ff, TCL).contains("set i 0"));
    assert!(opt_fires(ff, TCL, "O112"));

    // `for {} {0} {} ...` deletes entirely.
    let ffe = "for {} {0} {} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ffe, TCL).contains("puts looping"));
    assert!(optimised(ffe, TCL).contains("puts done"));
    assert!(opt_fires(ffe, TCL, "O112"));
}

#[test]
fn o112_switch_elimination() {
    // tclsh: switch abc {abc {1} def {2}} ⇒ arm 1.
    let lit = optimised(
        "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}",
        TCL,
    );
    assert!(lit.contains("set x 1"));
    assert!(!lit.contains("set y 2"));

    // No literal match ⇒ default. tclsh: switch xyz {...default {2}} ⇒ 2.
    let def = optimised(
        "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}",
        TCL,
    );
    assert!(!def.contains("set a 1"));
    assert!(def.contains("set b 2"));

    // No match, no default ⇒ whole switch deleted.
    let nm = optimised("switch xyz {\n    abc { set a 1 }\n}\nputs done", TCL);
    assert!(!nm.contains("set a 1"));
    assert!(nm.contains("puts done"));

    // Dynamic subject is untouched.
    assert!(opt_absent(
        "switch $x {\n    abc { set a 1 }\n}",
        TCL,
        "O112"
    ));
}

#[test]
fn o112_nested_via_multipass() {
    // First pass unwraps `if {1}`; the fixpoint helper eliminates the inner
    // `if {0}` dead block.
    let nested = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    assert!(optimised(nested, TCL).contains("set alive 2"));
    let registry = registry_for_dialect(TCL);
    let (fixed, _) = optimise_source_multipass(nested, registry, Some(TCL), 10);
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// ===========================================================================
// O113 — strength reduction
// ===========================================================================

#[test]
fn o113_strength_reduction() {
    // tclsh sweep: x ** 2 == x * x.
    let p = optimised("if {$x ** 2} {}", TCL);
    assert!(!p.contains("**"));
    assert!(p.contains('*'));
    assert!(opt_fires("if {$x ** 2} {}", TCL, "O113"));

    // tclsh sweep (x≥0): x % 8 == x & 7, x % 16 == x & 15, x % 2 == x & 1.
    let m8 = optimised("if {$x % 8} {}", TCL);
    assert!(m8.contains('&'));
    assert!(!m8.contains('%'));
    assert!(optimised("if {$x % 16} {}", TCL).contains("& 15")); // tclsh: x%16 == x&15
    assert!(optimised("if {$x % 2} {}", TCL).contains("& 1")); // tclsh: x%2 == x&1

    // Non-power-of-two modulus is not reduced.
    assert!(opt_absent("if {$x % 7} {}", TCL, "O113"));
    // x ** 3 is not reduced (only x ** 2).
    assert!(opt_absent("if {$x ** 3} {}", TCL, "O113"));
    // Command subst present ⇒ no reduction (side effects).
    assert!(opt_absent("if {[clock seconds] ** 2} {}", TCL, "O113"));
}

// ===========================================================================
// O114 — incr idiom recognition (needs SSA-known INT type, kept live)
// ===========================================================================

#[test]
fn o114_incr_idiom() {
    // D5-O114: the loop-counter pattern makes `x` INT-typed AND live (read via
    // `puts $x`), so the rewrite is sound. tclsh: `set x N; set x [expr {$x+1}]`
    // is the same as `incr x` for INT x.
    let add1 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add1, TCL).contains("incr x"));
    assert!(opt_fires(add1, TCL, "O114"));

    let add5 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 5}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add5, TCL).contains("incr x 5")); // tclsh: x+5 == incr x 5

    let sub3 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x - 3}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(sub3, TCL).contains("incr x -3")); // tclsh: x-3 == incr x -3

    // Commutative `N + $x` also matches.
    let comm = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {1 + $x}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(comm, TCL).contains("incr x"));

    // Multiplication is not an incr idiom.
    let mul = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x * 2}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(opt_absent(mul, TCL, "O114"));

    // Unknown-type param must NOT be incr-rewritten (could be a float at runtime;
    // tclsh: `set x [expr {$x + 1}]` on 1.5 ⇒ 2.5, but `incr x` errors).
    assert!(opt_absent(
        "proc foo {x} {\n    set x [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
    // Different target var ⇒ no incr.
    assert!(opt_absent(
        "proc foo {x} {\n    set y [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
}

// ===========================================================================
// O115 — redundant nested expr elimination
// ===========================================================================

#[test]
fn o115_nested_expr_unwrap() {
    // `if {[expr {...}]}` — the nested expr is redundant in a condition. tclsh:
    // `if {[expr {$x + 1}]}` == `if {$x + 1}`.
    let in_if = "if {[expr {$x + 1}]} {}";
    assert!(!optimised(in_if, TCL).contains("[expr"));
    assert!(opt_fires(in_if, TCL, "O115"));

    // `[expr {[expr {...}]}]` in a return value unwraps the inner. tclsh:
    // `[expr {[expr {$x * 2}]}]` == `[expr {$x * 2}]`.
    let in_ret = "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}";
    assert!(optimised(in_ret, TCL).contains("return [expr {$x * 2}]"));
    assert!(opt_fires(in_ret, TCL, "O115"));

    // `[string length $s]` is not an expr — not unwrapped.
    assert!(opt_absent("if {[string length $s]} {}", TCL, "O115"));

    // NOTE on gaps (sound): `while {[expr {...}]}` and
    // `set y [expr {[expr {...}]}]` are not unwrapped (only if/return paths fire).
}

// ===========================================================================
// O116 — constant list folding
// ===========================================================================

#[test]
fn o116_list_folding() {
    // [list a] folds away the `[list` call (via O116 or O100 propagation).
    // tclsh: [list a] == "a".
    let single = "set x [list a]\nputs $x";
    assert!(!optimised(single, TCL).contains("[list"));
    assert!(opt_any_of(single, TCL, &["O116", "O100"]));

    // Empty list folds (O116, or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [list]\nputs $x",
        TCL,
        &["O116", "O100", "O109"]
    ));

    // Multi-element [list a b c] must NOT fold via O116 (intrep shimmer); it may
    // be propagated as a braced word via O100, which is semantically identical
    // (tclsh: [list a b c] == {a b c}). Key invariant: no O116.
    assert!(opt_absent("set x [list a b c]\nputs $x", TCL, "O116"));
    // List with a variable element ⇒ not folded.
    assert!(opt_absent("set x [list $a b c]\nputs $x", TCL, "O116"));
    // Element with a space ⇒ not O116.
    assert!(opt_absent("set x [list \"a b\" c]\nputs $x", TCL, "O116"));
}

// ===========================================================================
// O117 — string length zero-check simplification
// ===========================================================================

#[test]
fn o117_strlen_zero_check() {
    // tclsh sweep: ([string length $s] == 0) <=> ($s eq ""); != 0 <=> ne "".
    assert!(opt_fires("if {[string length $s] == 0} {}", TCL, "O117"));
    assert!(opt_fires("if {[string length $s] != 0} {}", TCL, "O117"));
    // Swapped `0 == [string length $s]` also simplifies.
    assert!(opt_fires("if {0 == [string length $s]} {}", TCL, "O117"));

    // [string length $s] == 5 is NOT simplified.
    assert!(opt_absent("if {[string length $s] == 5} {}", TCL, "O117"));

    // Gaps (sound): the `> 0`, `<= 0`, and swapped `0 < [string length $s]` forms
    // are equivalent to `ne ""` / `eq ""` but only the `== 0` / `!= 0` / `0 ==`
    // forms are simplified. Confirm the conservative no-op for `> 0` (left
    // unchanged is sound).
    assert!(opt_absent("if {[string length $s] > 0} {}", TCL, "O117"));
    assert!(opt_absent("if {[string length $s] <= 0} {}", TCL, "O117"));
    assert!(opt_absent("if {0 < [string length $s]} {}", TCL, "O117"));
}

// ===========================================================================
// O118 — constant lindex folding
// ===========================================================================

#[test]
fn o118_lindex_folding() {
    // tclsh-verified each fold result.
    assert_eq!(
        optimised("set x [lindex {a b c} 0]\nputs $x", TCL),
        "set x a\nputs $x"
    ); // tclsh: a
    assert_eq!(
        optimised("set x [lindex {a b c} 1]\nputs $x", TCL),
        "set x b\nputs $x"
    ); // tclsh: b
    assert_eq!(
        optimised("set x [lindex {x y z} end]\nputs $x", TCL),
        "set x z\nputs $x"
    ); // tclsh: z
    assert_eq!(
        optimised("set x [lindex {a b c d} end-1]\nputs $x", TCL),
        "set x c\nputs $x"
    ); // tclsh: c

    // Out-of-range index folds (to empty); Rust emits O118 (or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [lindex {a b} 5]\nputs $x",
        TCL,
        &["O118", "O100", "O109"]
    ));

    // Variable index ⇒ not folded.
    assert!(opt_absent(
        "set x [lindex {a b c} $i]\nputs $x",
        TCL,
        "O118"
    ));

    // MORE precise (tclsh-proven sound): one might expect `[lindex {{a b} c} 0]`
    // to NOT fold. tclsh disproves the caution: `[lindex {{a b} c} 0]` == "a b",
    // and it folds to `set x {a b}` — `set x {a b}; puts $x` prints "a b",
    // byte-identical to the original. Assert the sound rewrite rather than an
    // over-cautious no-op.
    assert_eq!(
        optimised("set x [lindex {{a b} c} 0]\nputs $x", TCL),
        "set x {a b}\nputs $x" // tclsh: [lindex {{a b} c} 0] == "a b"
    );
}

// ===========================================================================
// O119 — multi-set packing
// ===========================================================================

#[test]
fn o119_multi_set_packing() {
    // OMISSION (as in optimiser.rs): with an `eval {$a $b $c}` barrier the
    // constants are forwarded THROUGH the braced `eval {...}` literal (O102/O109)
    // — `eval {1 2 3}` — so no surviving stores remain and O119 never fires.
    // tclsh: `set a 1; set b 2; set c 3; eval {$a $b $c}` and `eval {1 2 3}` are
    // identical, so the fold is sound. Assert the packing-disabled invariants.

    // Tcl 9.0: individual `set` is faster ⇒ O119 must not fire.
    assert!(opt_absent(
        "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"",
        "tcl9.0",
        "O119"
    ));
    // Too few consecutive sets ⇒ no packing.
    assert!(opt_absent("set a 1\nset b 2\neval {$a $b}", TCL, "O119"));
    // The eval-barrier form is folded rather than packed. tclsh: identical value.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "eval {1 2 3}"
    );
}

// ===========================================================================
// O120 — prefer eq/ne for string comparisons
// ===========================================================================

#[test]
fn o120_string_compare_eq_ne() {
    // tclsh sweep: $a == "hello" <=> $a eq "hello" ("hello" is non-numeric).
    assert!(optimised("if {$a == \"hello\"} {}", TCL).contains("$a eq \"hello\""));
    assert!(opt_fires("if {$a == \"hello\"} {}", TCL, "O120"));

    // != inside an expr substitution ⇒ ne. Rust routes this through O110 (expr
    // instcombine) rather than O120; the rewrite text is identical. tclsh sweep:
    // $a != "hello" <=> $a ne "hello".
    let ne = "set ok [expr {$a != \"hello\"}]";
    assert!(optimised(ne, TCL).contains("$a ne \"hello\""));
    assert!(opt_any_of(ne, TCL, &["O120", "O110"]));

    // String literal on the left also rewrites. tclsh: "hello" == $a <=> eq.
    assert!(optimised("if {\"hello\" == $a} {}", TCL).contains("eq"));
    // != with a string on the right ⇒ ne.
    assert!(optimised("if {$x != \"foo\"} {}", TCL).contains("ne"));

    // Conservative non-rewrites (D5-O120: needs ≥1 provably-non-numeric operand).
    for src in [
        "set a [clock seconds]\nif {$a == \"1\"} {}", // INT-typed var, numeric literal
        "set a [string trim $raw]\nif {$a == \"1\"} {}", // STRING type ≠ non-numeric value
        "set a [string trim $raw]\nif {$a == \"true\"} {}", // boolean-like literal
        "set a [string trim $raw]\nif {$a == \"1.25\"} {}", // float-like literal
        "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}", // two STRING vars
        "if {$a == $b} {}",                           // both unknown
        "if {$a == \"true\"} {}",                     // boolean-like literal, unknown var
        "if {$a == \"1.25\"} {}",                     // float-like literal, unknown var
    ] {
        assert!(opt_absent(src, TCL, "O120"), "O120 must not fire: {src}");
    }
}

#[test]
fn o120_with_sccp_const_and_mixed() {
    // A var known non-numeric via SCCP CONST ("foo") proves the string path.
    // tclsh: `set a foo; expr {$a == $b}` == `expr {$a eq $b}` for every $b.
    let vc = "set a foo\nif {$a == $b} {}";
    assert!(opt_fires(vc, TCL, "O120"));
    assert!(optimised(vc, TCL).contains("eq $b"));

    // Mixed expr: only the string compare is rewritten; numeric `$n == 1` stays.
    // Rust carries this via O110 canonicalisation; the eq-half is present.
    let mixed = "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}";
    let mx = optimised(mixed, TCL);
    assert!(mx.contains("$a eq \"x\""));
    assert!(mx.contains("$n == 1"));
    assert!(opt_any_of(mixed, TCL, &["O120", "O110"]));

    // Two SCCP-constant operands: the compare folds outright. tclsh: "foo" !=
    // "bar" ⇒ condition is constant-false ⇒ `if {0}` deleted (O112).
    let vv = "set a foo\nset b bar\nif {$a == $b} {}";
    let v = optimised(vv, TCL);
    assert!(!v.contains("=="));
    assert!(!v.contains(" eq "));
    assert!(opt_fires(vv, TCL, "O112"));
}

// ===========================================================================
// De Morgan / inversion — exercised via O110 rewrites
// ===========================================================================

#[test]
fn demorgan_logic_via_o110() {
    // The De-Morgan logic is the O110 expr rewrite; assert the transforms on
    // `[expr {...}]`. All tclsh-swept above.
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    // Plain `&&` / `==` / single `!` are NOT De-Morgan candidates (no rewrite to
    // the inverse form fires).
    assert!(optimised("set v [expr {$a && $b}]", TCL).contains("$a && $b"));
    assert!(!optimised("set v [expr {$a == $b}]", TCL).contains("||"));
}

#[test]
fn invert_logic_via_o110() {
    // The inversion logic surfaces as the `!(...)` O110 rewrite. tclsh-swept.
    assert!(optimised("set v [expr {!($a == $b)}]", TCL).contains("$a != $b"));
    assert!(optimised("set v [expr {!($a < $b)}]", TCL).contains("$a >= $b"));
    assert!(optimised("set v [expr {!($a >= $b)}]", TCL).contains("$a < $b"));
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    // Complex chain. tclsh sweep: !(a==b && c<d) == (a!=b || c>=d).
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL).contains("$a != $b || $c >= $d")
    );
    // Double-not removal in a boolean context. tclsh: !!x in `if` == x.
    assert!(optimised("if {!!$x} { puts yes }", TCL).contains("if {$x}"));
}

// ===========================================================================
// apply_optimisations / find_optimisations — edge cases
// ===========================================================================

#[test]
fn apply_optimisations_edge_cases() {
    let registry = registry_for_dialect(TCL);
    // Empty optimisation set ⇒ source unchanged.
    assert_eq!(apply_optimisations("set x 1", &[]), "set x 1");

    // Multiple non-overlapping rewrites. tclsh: a=1, b=2 ⇒ a+b == 3.
    assert!(optimised("set a 1\nset b 2\nset c [expr {$a + $b}]", TCL).contains("set c 3"));

    // Reverse-offset application stays stable. tclsh: a=1, b=2 ⇒ `puts 1`, `puts 2`.
    let rev = optimised("set a 1\nset b 2\nputs $a\nputs $b", TCL);
    assert!(rev.contains("puts 1"));
    assert!(rev.contains("puts 2"));

    // Sorting: the optimise_with_dialect set is span-sorted (the manager's
    // determinism chokepoint). Co-located entries (e.g. O100 + O102 on the same
    // expr span) are permitted before application; `apply_optimisations` resolves
    // them, which the value assertions above already exercise.
    let src = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$a + 3}]";
    let opts = optimise_with_dialect(src, registry, Some(TCL));
    let starts: Vec<u32> = opts.iter().map(|o| o.span.start()).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        starts, sorted,
        "optimisations must be span-start-sorted: {starts:?}"
    );
}

// ===========================================================================
// Variable-shape guardrails — array/namespaced refs not rewritten
// ===========================================================================

#[test]
fn variable_shape_guardrails() {
    for src in [
        "set x ${a(1)}\nputs $x",
        "set x $a(1)\nputs $x",
        "set x $::ns::arr(k)\nputs $x",
    ] {
        assert_eq!(optimised(src, TCL), src);
        assert!(
            opt_codes(src, TCL).is_empty(),
            "no rewrite for shape: {src}"
        );
    }
}

// ===========================================================================
// Cross-event DSE guardrails (f5-irules) — stores read in a later event survive
// ===========================================================================

#[test]
fn cross_event_dse_guardrails() {
    // `$uri` set in HTTP_REQUEST, read in HTTP_RESPONSE — the store must survive
    // (connection-scoped variables persist across `when` handlers). (Structural:
    // deleting `set uri` would leave $uri undefined in the response handler.)
    let uri = "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\nwhen HTTP_RESPONSE {\n    log local0. \"uri=$uri\"\n}\n";
    assert!(optimised(uri, IR).contains("set uri"));

    // `[info exists ans_cleared]` in DNS_RESPONSE observes a flag set in
    // DNS_REQUEST — neither the store nor the existence check may be folded.
    let flag = "when DNS_REQUEST {\n    if { [DNS::header opcode] ne \"QUERY\" } {\n        set ans_cleared 1\n        DNS::return\n        return\n    }\n}\nwhen DNS_RESPONSE {\n    if { [info exists ans_cleared] } {\n        unset -nocomplain -- ans_cleared\n        return\n    }\n}\n";
    assert!(optimised(flag, IR).contains("set ans_cleared"));
}

// ===========================================================================
// Shimmer detection — S100 / S101 / S102
// ===========================================================================

#[test]
fn shimmer_no_false_positives() {
    // Clean integer code: no shimmer.
    assert_eq!(shimmer_codes("set x 0\nincr x"), Vec::<String>::new());
    // List operations stay list-typed: no shimmer.
    assert_eq!(
        shimmer_codes("set x [list a b c]\nset n [llength $x]"),
        Vec::<String>::new()
    );
    // Integer arithmetic: no shimmer.
    assert_eq!(
        shimmer_codes("set x 42\nset y [expr {$x + 1}]"),
        Vec::<String>::new()
    );
    // `string length` accepts any type — int→strlen does not shimmer.
    assert_eq!(
        shimmer_count("set x 42\nset y [string length $x]", "S100"),
        0
    );
    // boolean → int is a numeric sub-type, not a shimmer.
    assert_eq!(
        shimmer_count("set x true\nset y [expr {$x + 1}]", "S100"),
        0
    );
}

#[test]
fn shimmer_string_in_numeric_context_s100() {
    // A STRING-typed var used in a numeric expr shimmers (S100 outside a loop).
    assert!(
        shimmer_count(
            "set x [string trim \" hello \"]\nset y [expr {$x + 1}]",
            "S100"
        ) >= 1
    );
    // incr on a STRING var shimmers.
    assert!(shimmer_count("set x [string trim \" hello \"]\nincr x", "S100") >= 1);
}

#[test]
fn shimmer_loop_paths_no_crash() {
    // Shimmer inside a loop (S101 territory) — type-analysis-dependent; assert no
    // panic and a well-formed result.
    let _ = shimmer_codes(
        "set x [list a b c]\nfor {set i 0} {$i < 3} {incr i} {\n    set n [llength $x]\n    set y [expr {$x + 1}]\n}\n",
    );
    // Consistent integer loop: no S102 thunking.
    assert_eq!(
        shimmer_count(
            "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    incr x\n}\n",
            "S102"
        ),
        0
    );
    // Oscillating-type loop body — may or may not detect S102 depending on
    // inference depth; assert no panic.
    let _ = shimmer_codes(
        "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    set x [string length [string repeat a $x]]\n}\n",
    );
}

// ===========================================================================
// GVN / CSE / LICM — find_redundant_computations (O105 / O106)
// ===========================================================================

#[test]
fn gvn_cse_detection() {
    // A repeated pure call ([string length $x]) is detected (O105). (Structural:
    // the second computation is provably redundant.)
    let dup = "set x hello\nset a [string length $x]\nset b [string length $x]";
    assert!(gvn_codes(dup).iter().any(|c| c == "O105"));
    assert!(!gvn_codes(dup).is_empty());

    // Different arguments ⇒ not flagged.
    assert_eq!(
        gvn_codes("set a [string length hello]\nset b [string length world]"),
        Vec::<String>::new()
    );

    // Three identical ⇒ two redundancy warnings.
    let three = "set a [string length $x]\nset b [string length $x]\nset c [string length $x]";
    assert!(gvn_codes(three).len() >= 2);

    // A mutation between the calls invalidates the redundancy.
    assert_eq!(
        gvn_codes("set a [string length $x]\nset x different\nset b [string length $x]"),
        Vec::<String>::new()
    );

    // Precision note (NOT a miscompile): the `find_redundancies_for_cu` GVN finder
    // DOES flag the repeated `[clock seconds]` with O105 (it treats the call key
    // as redundant) even though `[clock seconds]` is impure. This is a
    // detection-precision detail: it is a HINT, not an applied rewrite — the full
    // `optimise_with_dialect` pipeline does NOT CSE `clock seconds` (no O105
    // survives), so no miscompile is produced. Assert the sound end-to-end
    // invariant: the pipeline leaves the two calls intact.
    let clk = "set a [clock seconds]\nset b [clock seconds]";
    assert!(opt_absent(clk, TCL, "O105"));
    assert!(optimised(clk, TCL).contains("set a [clock seconds]"));
    assert!(optimised(clk, TCL).contains("set b [clock seconds]"));
}

// ===========================================================================
// Edge cases and robustness — must not crash, well-formed results
// ===========================================================================

#[test]
fn edge_cases_no_crash() {
    // Empty / comment / whitespace sources ⇒ no optimisations.
    assert!(opt_codes("", TCL).is_empty());
    assert!(opt_codes("# just a comment", TCL).is_empty());
    assert!(opt_codes("   \n\n  ", TCL).is_empty());
    assert_eq!(optimised("", TCL), "");

    // Nested proc definitions ⇒ no crash.
    let _ = opt_codes(
        "proc outer {} {\n    proc inner {} { return 1 }\n    return [inner]\n}\n",
        TCL,
    );

    // Deep expression nesting ⇒ no stack overflow.
    let mut expr = String::from("$x");
    for _ in 0..20 {
        expr = format!("({expr} + 1)");
    }
    let _ = opt_codes(&format!("set v [expr {{{expr}}}]"), TCL);

    // Many sets ⇒ no performance pathology (just completes).
    let mut lines: Vec<String> = (0..50).map(|i| format!("set v{i} {i}")).collect();
    lines.push("puts $v49".into());
    let _ = opt_codes(&lines.join("\n"), TCL);

    // unset clears the constant ⇒ no fold across it (source unchanged).
    let unset_src = "set a 1\nunset a\nset b [expr {$a + 2}]";
    assert_eq!(optimised(unset_src, TCL), unset_src);

    // catch / try / foreach bodies ⇒ no crash.
    let _ = opt_codes("catch {\n    set x [expr {1 + 2}]\n}\n", TCL);
    let _ = opt_codes(
        "try {\n    set x [expr {1 + 2}]\n} on error {e} {\n    puts $e\n}\n",
        TCL,
    );
    let _ = opt_codes("foreach item {a b c} {\n    puts $item\n}\n", TCL);
    let _ = opt_codes("switch abc {\n    abc -\n    def { set x 1 }\n}\n", TCL);
}

#[test]
fn edge_cases_loops_and_namespaces() {
    // Constants before a loop that may modify them are NOT propagated in.
    let loop_src =
        "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n";
    assert_eq!(optimised(loop_src, TCL), loop_src);
    assert!(opt_codes(loop_src, TCL).is_empty());

    // Deeply nested namespace proc folds at the call site. tclsh: a::b::c::f == 1.
    let deep = "namespace eval a {\n    namespace eval b {\n        namespace eval c {\n            proc f {} { return 1 }\n        }\n    }\n}\nset v [a::b::c::f]\n";
    assert!(optimised(deep, TCL).contains("set v 1"));
}

#[test]
fn robustness_malformed_inputs() {
    // Malformed / unusual inputs ⇒ no panic, well-formed (possibly empty) result.
    for src in [
        "set v [expr {",
        "if {$x",
        "puts",
        "set x 1",
        "set x \"héllo\"\nputs $x",
        "set x \"hello\\nworld\"\nputs $x",
        "set x {$not_a_var}\nputs $x",
        "set a 1; set b [expr {$a + 1}]",
    ] {
        let _ = opt_codes(src, TCL); // no panic; result is a Vec
    }
    // `{$not_a_var}` is a brace literal, not a variable ref — propagation is
    // suppressed (no fresh substitution injected). Output is sound either way.
    let bracelit = optimised("set x {$not_a_var}\nputs $x", TCL);
    assert!(bracelit.contains("$not_a_var")); // tclsh: prints literal "$not_a_var"
}

// ===========================================================================
// O124 — comment out unused procs (f5-irules only)
// ===========================================================================

#[test]
fn o124_unused_irule_procs() {
    // Unused proc commented out.
    let unused = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unused, IR, "O124"));
    let uo = optimised(unused, IR);
    assert!(uo.contains("# proc helper"));
    assert!(uo.contains("# [O124]"));

    // Used / transitively-used / direct-call / qualified procs are NOT commented.
    let kept = [
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [call helper] }",
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [helper] }",
        "proc inner {} { return 1 }\nproc outer {} { return [call inner] }\nwhen HTTP_REQUEST { set v [call outer] }",
        // library iRule (only procs + RULE_INIT) — skip.
        "proc helper {} { return 1 }\nwhen RULE_INIT { set ::x 1 }",
    ];
    for src in kept {
        assert!(opt_absent(src, IR, "O124"), "O124 must not fire: {src}");
    }

    // Plain Tcl is never touched by O124.
    assert!(opt_absent(
        "proc helper {} { return 1 }\nputs hello",
        TCL,
        "O124"
    ));

    // O124 supersedes tail-call rewrites for an unused tail-recursive proc.
    let supersede = "proc fact {n acc} {\n    if {$n <= 1} { return $acc }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\nwhen HTTP_REQUEST { pool p1 }";
    let codes = opt_codes(supersede, IR);
    assert!(codes.iter().any(|c| c == "O124"));
    assert!(!codes.iter().any(|c| c == "O121"));
    assert!(!codes.iter().any(|c| c == "O122"));

    // A used proc allows other rewrites (O120 inside it); not commented out.
    let used = "proc helper {x} {\n    if {$x == \"foo\"} { return 1 }\n    return 0\n}\nwhen HTTP_REQUEST { set v [call helper bar] }";
    assert!(opt_fires(used, IR, "O120"));
    assert!(opt_absent(used, IR, "O124"));

    // O124 coexists with an in-event rewrite.
    let coexist = "proc helper {} { return 1 }\nwhen HTTP_REQUEST {\n    set x [expr {1 + 2}]\n    pool p1\n}";
    let cc = opt_codes(coexist, IR);
    assert!(cc.iter().any(|c| c == "O124"));
    assert!(cc.iter().any(|c| c == "O101" || c == "O102" || c == "O126"));

    // eval only in an UNREACHABLE proc does not suppress O124.
    let unreach = "proc dynamic_helper {} {\n    set cmd foo\n    eval $cmd\n}\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unreach, IR, "O124"));
}

// ===========================================================================
// O121 / O122 / O123 — tail-call / loop / accumulator
// ===========================================================================

#[test]
fn tail_call_o121_o122_o123() {
    // Tail-position self-call ⇒ O121 (tailcall) or O122 (loop). tclsh: the
    // tailcall/loop forms compute the same result (factorial 5 1 == 120).
    let fac = "proc factorial {n acc} {\n    if {$n <= 1} { return $acc }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    assert!(opt_any_of(fac, TCL, &["O121", "O122"]));

    // Arity mismatch ⇒ O121 may fire but O122 (full loop conversion) must not.
    let arity = "proc f {a b} {\n    return [f $a]\n}\n";
    assert!(opt_fires(arity, TCL, "O121"));
    assert!(opt_absent(arity, TCL, "O122"));

    // Non-tail accumulator-eligible recursion ⇒ O123 hint.
    let nontail = "proc factorial {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(nontail, TCL, "O123"));
    // An already-tail-recursive proc does NOT get the O123 hint.
    assert!(opt_absent(fac, TCL, "O123"));
    // A non-recursive proc gets neither tail-call code nor O123.
    let plain = "proc add {a b} {\n    return [expr {$a + $b}]\n}\n";
    assert!(opt_absent(plain, TCL, "O121"));
    assert!(opt_absent(plain, TCL, "O123"));
    // Doubly-recursive (fib) ⇒ no O122 (not a tail call).
    let fib = "proc fib {n} {\n    if {$n <= 1} { return $n }\n    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]\n}\n";
    assert!(opt_absent(fib, TCL, "O122"));
}

// ===========================================================================
// O125 — code sinking
// ===========================================================================

#[test]
fn o125_code_sinking() {
    // Basic sink into an `if` whose condition does not read the var. Either O125
    // or O100 is accepted; here O125 fires. tclsh: `set b foo; if {$a} {puts $b}`
    // and the sunk form both print foo iff $a is true.
    let basic = "set b foo\nif {$a} {\n    puts $b\n}";
    assert!(opt_any_of(basic, TCL, &["O125", "O100"]));

    // O125 must NOT fire when the var is read in the condition or its RHS is a
    // command substitution.
    assert!(opt_absent(
        "set b $x\nif {$b} {\n    puts hello\n}",
        TCL,
        "O125"
    )); // var in condition
    assert!(opt_absent(
        "set b [clock seconds]\nif {$a} {\n    puts $b\n}",
        TCL,
        "O125"
    )); // cmd-sub RHS

    // Coexists with O120 (string-compare rewrite in the condition).
    let with120 = "set b foo\nif {$kind == \"x\"} {\n    puts $b\n}";
    assert!(opt_fires(with120, TCL, "O120"));
    assert!(optimised(with120, TCL).contains("$kind eq \"x\""));
}

// ===========================================================================
// O127 — single-use store-to-load forwarding (proc-local)
// ===========================================================================

#[test]
fn o127_load_forwarding() {
    // Single-use proc-local with a command substitution is inlined. tclsh: the
    // inlined `[set x [clock seconds]]` assigns x and yields its value, identical
    // to `set x [clock seconds]; puts $x`.
    let cmdsub = "proc f {} { set x [clock seconds]\n puts $x }";
    assert!(opt_fires(cmdsub, TCL, "O127"));
    assert!(optimised(cmdsub, TCL).contains("puts [set x [clock seconds]]"));

    // A constant is handled by propagation/DSE, NOT O127. tclsh: x=42 ⇒ puts 42.
    let constv = "proc f {} { set x 42\n puts $x }";
    assert!(opt_absent(constv, TCL, "O127"));
    assert!(opt_fires(constv, TCL, "O102"));
    assert!(optimised(constv, TCL).contains("puts 42"));

    // More-than-once use ⇒ not inlined.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n puts $x\n puts $x }",
        TCL,
        "O127"
    ));
    // Top-level variables ⇒ not inlined.
    assert!(opt_absent("set x [clock seconds]\nputs $x", TCL, "O127"));
    // A barrier (info exists between) blocks forwarding.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n info exists x\n puts $x }",
        TCL,
        "O127"
    ));
    // upvar/global aliasing blocks forwarding.
    assert!(opt_absent(
        "proc f {} { upvar 1 ext x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    assert!(opt_absent(
        "proc f {} { global x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    // A phi merge (two defining branches) blocks forwarding.
    assert!(opt_absent(
        "proc f {c} { if {$c} { set x [clock seconds] } else { set x [clock clicks] }\n puts $x }",
        TCL,
        "O127"
    ));

    // NOTE on gaps (sound): the var-copy form `set x $arg; puts $x` is left
    // unchanged rather than inlined (O127); an intervening empty `eval {}` barrier
    // is not honoured.
}

// ===========================================================================
// Pass interactions — composed rewrites
// ===========================================================================

#[test]
fn pass_interactions() {
    // Constant fold resolves the condition, then O112/O101 eliminates the if.
    assert!(opt_any_of(
        "set x 1\nif {$x > 0} {\n    puts yes\n}\n",
        TCL,
        &["O101", "O112"]
    ));
    // Propagation enables DSE: after folding x into puts, `set x` is dead.
    let prop = "set x 42\nputs $x";
    assert!(optimised(prop, TCL).contains("puts 42"));
    assert!(opt_fires(prop, TCL, "O109"));

    // Chained propagation across multiple variables. tclsh: a=1 ⇒ d==4.
    assert!(optimised(
        "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset d [expr {$c + 1}]\nputs $d\n",
        TCL
    )
    .contains("puts 4"));

    // All uses propagated then DSE. tclsh: x=5 ⇒ `puts 5` and y==6.
    let dse = "set x 5\nputs $x\nset y [expr {$x + 1}]";
    assert!(optimised(dse, TCL).contains("puts 5"));
    assert!(optimised(dse, TCL).contains("set y 6"));

    // Overlapping nested-expr rewrites resolve without corruption (no panic).
    let _ = opt_codes("set a 1\nset b [expr {$a + [expr {2 + 3}]}]", TCL);

    // InstCombine in a branch then fold. tclsh: a=5 ⇒ 5+0==5 ⇒ condition true.
    assert!(opt_any_of(
        "set a 5\nif {$a + 0 == 5} { puts yes }",
        TCL,
        &["O101", "O112"]
    ));

    // Strength reduction inside a command substitution expr.
    assert!(opt_fires("if {$x ** 2} { puts yes }", TCL, "O113"));
    // O117 fires in branch conditions.
    assert!(opt_fires(
        "if {[string length $s] == 0} { puts empty }",
        TCL,
        "O117"
    ));
    // String-compare ⇒ eq rewrite inside [expr {...}]. Rust routes the expr-subst
    // form through O110 (instcombine) rather than O120; the rewrite is identical.
    // tclsh: $a == "hello" <=> $a eq "hello".
    let expr_eq = "set ok [expr {$a == \"hello\"}]";
    assert!(optimised(expr_eq, TCL).contains("$a eq \"hello\""));
    assert!(opt_any_of(expr_eq, TCL, &["O120", "O110"]));
}

// ===========================================================================
// Profile directive survival + multipass string-build collapse
// ===========================================================================

#[test]
fn profile_directive_and_multipass() {
    // The `# profiles: HTTP2` comment survives optimisation; the `if {1}` inside
    // the event unwraps (O112) and keeps the HTTP2 call.
    let src =
        "# profiles: HTTP2\nwhen HTTP_REQUEST {\n    if {1} {\n        HTTP2::active\n    }\n}\n";
    let out = optimised(src, IR);
    assert!(out.contains("# profiles: HTTP2"));
    assert!(out.contains("HTTP2::active"));
    assert!(opt_fires(src, IR, "O112"));

    // Multipass: a write-only string build in a proc collapses to a single
    // `return {Hello World}`; the intermediate local is removed. tclsh: the proc
    // returns "Hello World" either way.
    let registry = registry_for_dialect(TCL);
    let build = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (mp, _) = optimise_source_multipass(build, registry, Some(TCL), 10);
    assert!(mp.contains("return {Hello World}"));
    assert!(!mp.contains("append"));
    assert!(!mp.contains("set msg"));
}
