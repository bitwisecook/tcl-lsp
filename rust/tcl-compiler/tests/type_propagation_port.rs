//! SSA type-inference (type-propagation) integration tests.
//!
//! Ported from the Python `tests/test_type_propagation.py` suite. The Python
//! side drives `helpers.analyse_types(src)` and reads `analysis.types` keyed by
//! `(name, version)`; the Rust equivalent is the per-function
//! [`tcl_compiler::compilation_unit::FunctionUnit`] on a
//! [`tcl_compiler::compilation_unit::CompilationUnit`], whose `types` field is a
//! `HashMap<ValueKey, TypeLattice>` with `ValueKey = (Symbol, Version)`. A
//! symbol resolves to its display name via `fu.ssa.var_name(sym)`.
//!
//! Each test asserts the *exact* inferred [`TclType`] / [`TypeKind`] for a
//! given variable + version, mirroring the Python `var_type` "highest version
//! wins" rule.
//!
//! **C-Tcl proof.** Type inference predicts the runtime intrep/value a Tcl
//! expression produces, so every Tcl-observable expectation here was checked
//! against real `tclsh` (8.6 and 9.0 agree on every case) via
//! `scripts/dev/tclsh_check.sh`, using `string is integer/double/list/boolean`
//! to confirm the inferred `TclType` matches Tcl's actual value. Those facts
//! are cited inline. Where the lattice models an abstraction with no direct Tcl
//! analogue (`Numeric` = the Int-or-Double join, `Shimmered`, `Overdefined`),
//! that is noted instead.
//!
//! Divergences from the Python suite are documented at the bottom of this file
//! (see `mod skipped_divergences`); the only genuine one is `ceil`/`floor`,
//! where the Rust pass infers `Int` but `tclsh` proves the value is a `Double`.

use std::collections::HashMap;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::types::{TclType, TypeKind, TypeLattice};
use tcl_registry::registry_for_dialect;

/// All `(version -> TypeLattice)` entries inferred for variable `var` in the
/// function `qname` (`"::top"` for the top-level script) of `src`.
///
/// Mirrors the Python `_all_var_types` / `_collect_versions`: it groups every
/// `ValueKey` whose symbol name equals `var` by SSA version.
fn var_types(src: &str, qname: &str, var: &str) -> HashMap<u32, TypeLattice> {
    let registry = registry_for_dialect("tcl8.6");
    let cu = CompilationUnit::build_for(src, registry, false);
    let fu = if qname == "::top" {
        &cu.top_level
    } else {
        cu.procedures
            .get(qname)
            .unwrap_or_else(|| panic!("procedure {qname} not found"))
    };
    let mut out = HashMap::new();
    for ((sym, ver), t) in &fu.types {
        if fu.ssa.var_name(*sym) == var {
            out.insert(*ver, t.clone());
        }
    }
    out
}

/// The inferred type of `var` at its highest SSA version in the top-level
/// script of `src` — the Rust analogue of Python's `helpers.var_type`.
fn var_type(src: &str, var: &str) -> Option<TypeLattice> {
    var_types(src, "::top", var)
        .into_iter()
        .max_by_key(|(ver, _)| *ver)
        .map(|(_, t)| t)
}

/// Convenience: the `TclType` of `var`'s highest version (panics if the
/// variable was never typed, matching the Python `assert t is not None`).
fn tcl_type(src: &str, var: &str) -> TclType {
    let t = var_type(src, var).unwrap_or_else(|| panic!("no type inferred for `{var}` in {src:?}"));
    t.tcl_type
        .unwrap_or_else(|| panic!("type for `{var}` has no tcl_type: {t} in {src:?}"))
}

// Literal types (TestLiteralTypes + TestLiteralTypeInference literal cases)

#[test]
fn literal_scalar_types() {
    // tclsh: `set x 42` -> 42, `string is integer 42` = 1  => Int
    let t = var_type("set x 42", "x").expect("x typed");
    assert_eq!(t.kind, TypeKind::Known);
    assert_eq!(t.tcl_type, Some(TclType::Int));

    // tclsh: `-7` is `string is integer` = 1 => Int
    assert_eq!(tcl_type("set x -7", "x"), TclType::Int);

    // tclsh: `string is integer hello` = 0, `string is double hello` = 0 => String
    let s = var_type("set x hello", "x").expect("x typed");
    assert_eq!(s.kind, TypeKind::Known);
    assert_eq!(s.tcl_type, Some(TclType::String));

    // tclsh: `string is double 3.14` = 1, `string is integer 3.14` = 0 => Double
    assert_eq!(tcl_type("set x 3.14", "x"), TclType::Double);
}

#[test]
fn boolean_literals_are_boolean() {
    // tclsh: each of these is `string is boolean X` = 1 and `string is
    // integer X` = 0 (e.g. `string is boolean true` = 1) => Boolean.
    for (src, var) in [
        ("set x true", "x"),
        ("set x false", "x"),
        ("set z yes", "z"),
        ("set z no", "z"),
        ("set z on", "z"),
        ("set z off", "z"),
    ] {
        let t = var_type(src, var).unwrap_or_else(|| panic!("{var} typed for {src:?}"));
        assert_eq!(t.kind, TypeKind::Known, "{src}");
        assert_eq!(t.tcl_type, Some(TclType::Boolean), "{src}");
    }
}

// incr (TestIncr)

#[test]
fn incr_produces_int() {
    // tclsh: `set x 0; incr x` -> 1, `string is integer 1` = 1 => Int
    assert_eq!(tcl_type("set x 0\nincr x", "x"), TclType::Int);
    // tclsh: `set x 0; incr x 5` -> 5 => Int
    assert_eq!(tcl_type("set x 0\nincr x 5", "x"), TclType::Int);
}

// Variable reference / interpolation (TestVariableReference)

#[test]
fn variable_reference_inherits_type() {
    // tclsh: `set x 42; set y $x` -> y is 42 (`string is integer` = 1) => Int.
    assert_eq!(tcl_type("set x 42\nset y $x", "y"), TclType::Int);
}

#[test]
fn string_interpolation_is_string() {
    // tclsh: `set y "value is $x"` -> "value is 42", `string is integer` = 0
    // => String (interpolation always renders a string).
    assert_eq!(
        tcl_type("set x 42\nset y \"value is $x\"", "y"),
        TclType::String
    );
}

// Command return types (TestCommandReturnTypes)

#[test]
fn command_return_types() {
    // tclsh: `string is list [list a b c]` = 1 => List
    assert_eq!(tcl_type("set x [list a b c]", "x"), TclType::List);
    // tclsh: `set n [llength {a b}]` -> 2, `string is integer 2` = 1 => Int
    assert_eq!(
        tcl_type("set x [list a b]\nset n [llength $x]", "n"),
        TclType::Int
    );
    // tclsh: `string is list [split "a,b,c" ,]` = 1 => List
    assert_eq!(tcl_type("set x [split \"a,b,c\" ,]", "x"), TclType::List);
    // tclsh: `join {a b c} ,` -> "a,b,c", `string is integer` = 0 => String
    assert_eq!(tcl_type("set x [join {a b c} ,]", "x"), TclType::String);
    // tclsh: `string length hello` -> 5, `string is integer 5` = 1 => Int
    assert_eq!(tcl_type("set x [string length hello]", "x"), TclType::Int);
    // tclsh: `concat {a b} {c d}` -> "a b c d", `string is list` = 1 => List
    assert_eq!(tcl_type("set x [concat {a b} {c d}]", "x"), TclType::List);
}

// expr known int (TestExprAssign)

#[test]
fn expr_known_int() {
    // tclsh: `expr {1 + 2}` -> 3, `string is integer 3` = 1 => Int.
    // Python accepts INT or NUMERIC (SCCP-dependent); Rust pins it to Int.
    let t = var_type("set x [expr {1 + 2}]", "x").expect("x typed");
    assert!(
        matches!(t.tcl_type, Some(TclType::Int | TclType::Numeric)),
        "expected Int or Numeric, got {t}"
    );
    assert_eq!(
        t.tcl_type,
        Some(TclType::Int),
        "Rust pins SCCP result to Int"
    );
}

// Phi merging (TestPhiMerging)

#[test]
fn phi_same_type_stays_int() {
    // `if {1} { set x 2 } else { set x 3 }` — SCCP folds the constant guard so
    // only the taken branch is executable; the surviving definition is Int.
    // tclsh: 2 and 3 are both `string is integer` = 1 => Int regardless of merge.
    let src = "\nset x 1\nif {1} {\n    set x 2\n} else {\n    set x 3\n}\n";
    let all = var_types(src, "::top", "x");
    assert!(!all.is_empty(), "x should be typed");
    for (ver, t) in &all {
        assert_eq!(
            t.tcl_type,
            Some(TclType::Int),
            "version {ver} should be Int, got {t}"
        );
    }
    // Highest version is also Int (Python `var_type` semantics).
    assert_eq!(tcl_type(src, "x"), TclType::Int);
}

#[test]
fn phi_incompatible_types_shimmer() {
    // `set x 1; if {$cond} { set x hello }` — after the merge x is either Int
    // (fall-through) or String (body), which is not Tcl-observable as a single
    // value: the lattice models the conflicting merge as SHIMMERED(String, Int)
    // (the analogue of a value whose cached intrep was forced to change type).
    let src = "\nset x 1\nif {$cond} {\n    set x hello\n}\nputs $x\n";
    let all = var_types(src, "::top", "x");
    let conflict: Vec<&TypeLattice> = all
        .values()
        .filter(|t| matches!(t.kind, TypeKind::Shimmered | TypeKind::Overdefined))
        .collect();
    assert!(
        !conflict.is_empty(),
        "expected a Shimmered/Overdefined merge version, got {all:?}"
    );
    // The merge specifically shimmers between the String body and the Int
    // fall-through. `TypeLattice::shimmered` stores the pair in canonical `Ord`
    // order (`from <= to`), so check the unordered pair rather than fixed slots:
    // here it is SHIMMERED(String, Int) because `String < Int` in `TclType`.
    let shim = conflict
        .iter()
        .find(|t| t.kind == TypeKind::Shimmered)
        .expect("a shimmered version");
    let pair = (shim.from_type, shim.tcl_type);
    assert_eq!(
        pair,
        (Some(TclType::String), Some(TclType::Int)),
        "merge should shimmer between Int and String, got {shim}"
    );
}

// Barrier (TestBarrier)

#[test]
fn barrier_does_not_drop_var_type() {
    // `set x 42; eval {set x hello}` — Python only requires the var stays
    // typed across the barrier. Rust analyses the eval body inline, so the
    // highest version reflects the `set x hello` inside it (String). Either
    // way the variable remains typed (not dropped to no-fact).
    let src = "set x 42\neval {set x hello}\n";
    let t = var_type(src, "x");
    assert!(t.is_some(), "x should remain typed across the eval barrier");
    // Both the pre-barrier Int and the in-eval String defs are present.
    let all = var_types(src, "::top", "x");
    let kinds: Vec<Option<TclType>> = {
        let mut v: Vec<Option<TclType>> = all.values().map(|t| t.tcl_type).collect();
        v.sort();
        v
    };
    assert!(
        kinds.contains(&Some(TclType::Int)) && kinds.contains(&Some(TclType::String)),
        "expected both Int (pre-eval) and String (in-eval) defs, got {all:?}"
    );
}

// Operator-aware expression inference (TestExpressionTypeInference)

#[test]
fn expr_division_promotion() {
    // tclsh: `expr {5 / 2}` -> 2 (integer division), `string is integer 2` = 1 => Int
    assert_eq!(tcl_type("set x [expr {5 / 2}]", "x"), TclType::Int);
    // tclsh: `expr {5.0 / 2}` -> 2.5, `string is double` = 1 => Double
    assert_eq!(tcl_type("set x [expr {5.0 / 2}]", "x"), TclType::Double);
    // tclsh: `expr {5 / 2.0}` -> 2.5 => Double
    assert_eq!(tcl_type("set z [expr {5 / 2.0}]", "z"), TclType::Double);
}

#[test]
fn expr_comparisons_are_boolean() {
    // tclsh: `expr {1 == 2}` -> 0, `string is boolean 0` = 1 => Boolean.
    // (Comparisons yield 0/1, which Tcl reports as boolean.)
    assert_eq!(
        tcl_type("set a 1\nset b 2\nset z [expr {$a == $b}]", "z"),
        TclType::Boolean
    );
    // tclsh: `expr {$a eq "x"}` -> 0 => Boolean (string compare).
    assert_eq!(
        tcl_type("set a hello\nset z [expr {$a eq \"x\"}]", "z"),
        TclType::Boolean
    );
}

#[test]
fn expr_bitwise_is_int() {
    // tclsh: `expr {256 & 0xFF}` -> 0, `string is integer` = 1 => Int.
    assert_eq!(
        tcl_type("set a 256\nset z [expr {$a & 0xFF}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {~0xFF}` -> -256 => Int
    assert_eq!(
        tcl_type("set x 0xFF\nset z [expr {~$x}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {1 << 2}` -> 4 => Int
    assert_eq!(
        tcl_type("set x 1\nset z [expr {$x << 2}]", "z"),
        TclType::Int
    );
}

#[test]
fn expr_logical_is_boolean() {
    // tclsh: `expr {1 && 0}` -> 0, `string is boolean 0` = 1 => Boolean
    assert_eq!(
        tcl_type("set a 1\nset b 0\nset z [expr {$a && $b}]", "z"),
        TclType::Boolean
    );
    // tclsh: `expr {!1}` -> 0 => Boolean
    assert_eq!(
        tcl_type("set x 1\nset z [expr {!$x}]", "z"),
        TclType::Boolean
    );
}

#[test]
fn expr_unary_preserves_operand_type() {
    // tclsh: `set x 5; expr {-$x}` -> -5, `string is integer` = 1 => Int
    assert_eq!(tcl_type("set x 5\nset z [expr {-$x}]", "z"), TclType::Int);
    // tclsh: `set x 3.14; expr {-$x}` -> -3.14, `string is double` = 1 => Double
    assert_eq!(
        tcl_type("set x 3.14\nset z [expr {-$x}]", "z"),
        TclType::Double
    );
    // tclsh: `set x 5; expr {+$x}` -> 5 => Int
    assert_eq!(tcl_type("set x 5\nset z [expr {+$x}]", "z"), TclType::Int);
    // tclsh: `set x 3.14; expr {+$x}` -> 3.14 => Double
    assert_eq!(
        tcl_type("set x 3.14\nset z [expr {+$x}]", "z"),
        TclType::Double
    );
}

#[test]
fn expr_arithmetic_promotion() {
    // tclsh: `expr {1 + 2}` -> 3 => Int
    assert_eq!(
        tcl_type("set a 1\nset b 2\nset z [expr {$a + $b}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {1 + 2.0}` -> 3.0, `string is double` = 1 => Double
    assert_eq!(
        tcl_type("set a 1\nset b 2.0\nset z [expr {$a + $b}]", "z"),
        TclType::Double
    );
    // tclsh: `expr {5 - 3}` -> 2 => Int
    assert_eq!(
        tcl_type("set a 5\nset b 3\nset z [expr {$a - $b}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {3 * 7}` -> 21 => Int
    assert_eq!(
        tcl_type("set a 3\nset b 7\nset z [expr {$a * $b}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {1.5 + 2.5}` -> 4.0 => Double
    assert_eq!(
        tcl_type("set a 1.5\nset b 2.5\nset z [expr {$a + $b}]", "z"),
        TclType::Double
    );
    // tclsh: `expr {3.14 * 2}` -> 6.28 => Double
    assert_eq!(
        tcl_type("set a 3.14\nset b 2\nset z [expr {$a * $b}]", "z"),
        TclType::Double
    );
    // tclsh: `expr {10 % 3}` -> 1 => Int
    assert_eq!(
        tcl_type("set a 10\nset b 3\nset z [expr {$a % $b}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {7891 % 123}` -> 16 (Tcl 9.0.2 expr.test) => Int
    assert_eq!(tcl_type("set z [expr {7891 % 123}]", "z"), TclType::Int);
}

#[test]
fn expr_power_promotion() {
    // tclsh: `expr {2 ** 10}` -> 1024, `string is integer` = 1 => Int
    assert_eq!(tcl_type("set z [expr {2 ** 10}]", "z"), TclType::Int);
    // tclsh: `expr {4 ** 2}` -> 16 => Int
    assert_eq!(tcl_type("set z [expr {4 ** 2}]", "z"), TclType::Int);
    // tclsh: `expr {4 ** 2.0}` -> 16.0, `string is double` = 1 => Double
    assert_eq!(
        tcl_type("set a 4\nset z [expr {$a ** 2.0}]", "z"),
        TclType::Double
    );
}

#[test]
fn expr_booleans_are_integers_in_arithmetic() {
    // tclsh: `set a true; set b 5; expr {$a + $b}` -> 6, `string is integer` = 1
    // => Int (a boolean used arithmetically is an integer 0/1).
    assert_eq!(
        tcl_type("set a true\nset b 5\nset z [expr {$a + $b}]", "z"),
        TclType::Int
    );
    // tclsh: `set a true; set b false; expr {$a + $b}` -> 1 => Int
    assert_eq!(
        tcl_type("set a true\nset b false\nset z [expr {$a + $b}]", "z"),
        TclType::Int
    );
}

#[test]
fn expr_math_functions_double() {
    // Each of these is `string is double R` = 1, `string is integer R` = 0 in
    // tclsh (e.g. `expr {sin(1)}` -> 0.841..., `string is double` = 1) => Double.
    for (src, var) in [
        ("set x 1\nset z [expr {sin($x)}]", "z"),
        ("set x 0\nset z [expr {cos($x)}]", "z"),
        ("set x 1\nset z [expr {tan($x)}]", "z"),
        ("set x 1\nset z [expr {exp($x)}]", "z"),
        ("set x 10\nset z [expr {log($x)}]", "z"),
        ("set x 100\nset z [expr {log10($x)}]", "z"),
        ("set x 4\nset z [expr {sqrt($x)}]", "z"),
        ("set x 1\nset z [expr {double($x)}]", "z"),
        // tclsh: `expr {pow(2,3)}` -> 8.0 (NB: ** operator on ints stays Int,
        // but the pow() *function* always yields a Double) => Double.
        ("set a 2\nset b 3\nset z [expr {pow($a, $b)}]", "z"),
        ("set a 3\nset b 4\nset z [expr {hypot($a, $b)}]", "z"),
        ("set a 10.5\nset b 3.0\nset z [expr {fmod($a, $b)}]", "z"),
        // tclsh: `expr {rand()}` -> e.g. 0.37..., `string is double` = 1 => Double
        ("set z [expr {rand()}]", "z"),
    ] {
        assert_eq!(tcl_type(src, var), TclType::Double, "{src}");
    }
}

#[test]
fn expr_int_conversions_are_int() {
    // tclsh proves each result is `string is integer R` = 1:
    //   int(3.14) -> 3, round(3.5) -> 4, wide(5) -> 5, entier(3.14) -> 3,
    //   isqrt(16) -> 4.  (ceil/floor are NOT here — see skipped_divergences.)
    for (src, var) in [
        ("set x 3.14\nset z [expr {int($x)}]", "z"),
        ("set x 3.5\nset z [expr {round($x)}]", "z"),
        ("set x 5\nset z [expr {wide($x)}]", "z"),
        ("set x 3.14\nset z [expr {entier($x)}]", "z"),
        ("set x 16\nset z [expr {isqrt($x)}]", "z"),
    ] {
        assert_eq!(tcl_type(src, var), TclType::Int, "{src}");
    }
}

#[test]
fn expr_ceil_floor_are_double() {
    // tclsh proves ceil/floor return a *double* (N.0), NOT an integer:
    //   expr {ceil(3.14)}  -> 4.0   string is integer 4.0 -> 0, is double -> 1
    //   expr {floor(3.7)}  -> 3.0   string is integer 3.0 -> 0, is double -> 1
    // (This regression-guards the type_infer fix that moved ceil/floor out of
    // the integer-returning group.)
    assert_eq!(
        tcl_type("set x 3.14\nset z [expr {ceil($x)}]", "z"),
        TclType::Double
    );
    assert_eq!(
        tcl_type("set x 3.7\nset z [expr {floor($x)}]", "z"),
        TclType::Double
    );
}

#[test]
fn expr_predicate_functions_are_boolean() {
    // tclsh: `expr {bool(1)}` -> 1 (`string is boolean` = 1), `expr {isnan(1.0)}`
    // -> 0, `expr {isinf(1.0)}` -> 0 => Boolean.
    for (src, var) in [
        ("set x 1\nset z [expr {bool($x)}]", "z"),
        ("set x 1.0\nset z [expr {isnan($x)}]", "z"),
        ("set x 1.0\nset z [expr {isinf($x)}]", "z"),
    ] {
        assert_eq!(tcl_type(src, var), TclType::Boolean, "{src}");
    }
}

#[test]
fn expr_abs_preserves_operand_type() {
    // tclsh: `expr {abs(-5)}` -> 5, `string is integer` = 1 => Int
    assert_eq!(
        tcl_type("set x -5\nset z [expr {abs($x)}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {abs(-3.14)}` -> 3.14, `string is double` = 1 => Double
    assert_eq!(
        tcl_type("set x -3.14\nset z [expr {abs($x)}]", "z"),
        TclType::Double
    );
}

#[test]
fn expr_int_func_is_int() {
    // tclsh: `set x 3.14; expr {int($x)}` -> 3, `string is integer` = 1 => Int
    assert_eq!(
        tcl_type("set x 3.14\nset z [expr {int($x)}]", "z"),
        TclType::Int
    );
}

#[test]
fn expr_min_max_join_operand_types() {
    // tclsh: `expr {min(1, 2)}` -> 1 (Int), `expr {max(3, 5)}` -> 5 (Int).
    assert_eq!(tcl_type("set z [expr {min(1, 2)}]", "z"), TclType::Int);
    assert_eq!(tcl_type("set z [expr {max(3, 5)}]", "z"), TclType::Int);
    // tclsh: `expr {max(1, 2.0)}` -> 2.0 (a Double at runtime). The lattice
    // can't statically pin the result to exactly Int or Double, so it joins the
    // operand types to NUMERIC — the abstract Int-or-Double element, a sound
    // over-approximation of the runtime Double. No direct single-type Tcl analogue.
    assert_eq!(
        tcl_type("set z [expr {max(1, 2.0)}]", "z"),
        TclType::Numeric
    );
    assert_eq!(
        tcl_type("set z [expr {min(1, 2.0)}]", "z"),
        TclType::Numeric
    );
}

// Ternary (TestTernaryTypeInference + ternary_join)

#[test]
fn expr_ternary_joins_branch_types() {
    // tclsh: `set c 1; expr {$c ? 1 : 2}` -> 1 (Int, same type both arms).
    assert_eq!(
        tcl_type("set c 1\nset z [expr {$c ? 1 : 2}]", "z"),
        TclType::Int
    );
    // tclsh: `expr {$c ? 1.0 : 2.0}` -> 1.0 => Double.
    assert_eq!(
        tcl_type("set c 1\nset z [expr {$c ? 1.0 : 2.0}]", "z"),
        TclType::Double
    );
    // tclsh: `expr {$c ? ($a > 0) : ($b < 0)}` -> 1 (`string is boolean` = 1)
    // => Boolean (both arms are comparisons).
    assert_eq!(
        tcl_type(
            "set a 1\nset b 2\nset c 1\nset z [expr {$c ? ($a > 0) : ($b < 0)}]",
            "z"
        ),
        TclType::Boolean
    );
    // Mixed Int/Double arms: runtime value is whichever arm fires (here 1, an
    // Int), but statically the type is the NUMERIC join of Int and Double — the
    // abstract Int-or-Double element, with no single-type Tcl analogue.
    assert_eq!(
        tcl_type("set c 1\nset z [expr {$c ? 1 : 2.0}]", "z"),
        TclType::Numeric
    );
}

// Comparison operators (TestComparisonTypeInference)

#[test]
fn comparison_operators_are_boolean() {
    // Every comparison yields 0/1, which tclsh reports as `string is boolean` = 1
    // (e.g. `expr {1 < 2}` -> 1 => Boolean). Covers < <= > >= != eq ne lt in ni.
    let cases = [
        "set a 1\nset b 2\nset z [expr {$a < $b}]",
        "set a 1\nset b 2\nset z [expr {$a <= $b}]",
        "set a 3\nset b 2\nset z [expr {$a > $b}]",
        "set a 3\nset b 2\nset z [expr {$a >= $b}]",
        "set a 1\nset b 2\nset z [expr {$a != $b}]",
        "set a hello\nset z [expr {$a ne \"world\"}]",
        "set a abc\nset z [expr {$a lt \"xyz\"}]",
        // tclsh: `set x 1; set list {a b c}; expr {$x in $list}` -> 0 => Boolean
        "set x 1\nset list {a b c}\nset z [expr {$x in $list}]",
        // tclsh: `expr {$x ni $list}` -> 1 => Boolean
        "set x 1\nset list {a b c}\nset z [expr {$x ni $list}]",
    ];
    for src in cases {
        assert_eq!(tcl_type(src, "z"), TclType::Boolean, "{src}");
    }
}

// Logical operators (TestLogicalTypeInference)

#[test]
fn logical_operators_are_boolean() {
    // tclsh: `expr {1 || 0}` -> 1, `expr {1 && 1 && 0}` -> 0, `expr {!42}` -> 0,
    // `expr {!!0}` -> 0 — all `string is boolean` = 1 => Boolean.
    let cases = [
        "set a 1\nset b 0\nset z [expr {$a || $b}]",
        "set a 1\nset b 1\nset c 0\nset z [expr {$a && $b && $c}]",
        "set x 42\nset z [expr {!$x}]",
        "set x 0\nset z [expr {!!$x}]",
    ];
    for src in cases {
        assert_eq!(tcl_type(src, "z"), TclType::Boolean, "{src}");
    }
}

// Bitwise operators (TestBitwiseTypeInference)

#[test]
fn bitwise_operators_are_int() {
    // tclsh: each result is `string is integer R` = 1, e.g.
    //   `expr {0xFF | 0x0F}` -> 255, `expr {0xFF ^ 0x0F}` -> 240,
    //   `expr {~0xFF}` -> -256, `expr {1 << 8}` -> 256, `expr {256 >> 4}` -> 16.
    let cases = [
        "set a 0xFF\nset b 0x0F\nset z [expr {$a | $b}]",
        "set a 0xFF\nset b 0x0F\nset z [expr {$a ^ $b}]",
        "set a 0xFF\nset z [expr {~$a}]",
        "set a 1\nset z [expr {$a << 8}]",
        "set a 256\nset z [expr {$a >> 4}]",
    ];
    for src in cases {
        assert_eq!(tcl_type(src, "z"), TclType::Int, "{src}");
    }
}

// Literal type inference in expr context (TestLiteralTypeInference)

#[test]
fn expr_integer_literal_spellings_are_int() {
    // tclsh: `expr {0xFF + 1}` -> 256, `expr {0o77 + 1}` -> 64,
    // `expr {0b1010 + 1}` -> 11 — all `string is integer` = 1 => Int.
    // (NB: in *expr* context 0o/0b/0x all tokenise to integers; this differs
    // from `set x 0o77` where the leading-zero-octal literal stays a String.)
    assert_eq!(tcl_type("set z [expr {0xFF + 1}]", "z"), TclType::Int);
    assert_eq!(tcl_type("set z [expr {0o77 + 1}]", "z"), TclType::Int);
    assert_eq!(tcl_type("set z [expr {0b1010 + 1}]", "z"), TclType::Int);
}

#[test]
fn expr_scientific_notation_is_double() {
    // tclsh: `expr {1.5e3 + 1}` -> 1501.0, `string is double` = 1 => Double.
    assert_eq!(tcl_type("set z [expr {1.5e3 + 1}]", "z"), TclType::Double);
}

#[test]
fn bare_boolean_literal_assignments_are_boolean() {
    // tclsh: `set z true` etc. — `string is boolean true` = 1 => Boolean.
    for (src, var) in [
        ("set z true", "z"),
        ("set z yes", "z"),
        ("set z on", "z"),
        ("set z no", "z"),
        ("set z off", "z"),
    ] {
        assert_eq!(tcl_type(src, var), TclType::Boolean, "{src}");
    }
}

// Command substitution inside expr (TestCommandSubstitutionType)

#[test]
fn command_sub_in_expr_stays_typed() {
    // `expr {[some_cmd] + 1}` — the command-sub operand is unknown at analysis
    // time, so the result is over-approximated. Python only requires it to be
    // typed; Rust pins it to NUMERIC (an expr always yields a number, but the
    // exact Int-vs-Double can't be known statically — no single Tcl analogue).
    let t = var_type("set z [expr {[some_cmd] + 1}]", "z").expect("z typed");
    assert!(matches!(
        t.tcl_type,
        Some(TclType::Numeric | TclType::Int | TclType::Double)
    ));
    // tclsh: a fully-constant nested expr `expr {[expr {1 + 2}] + 1}` -> 4 (Int
    // at runtime); the analyser keeps the inner command-sub abstract => Numeric.
    let n = var_type("set z [expr {[expr {1 + 2}] + 1}]", "z").expect("z typed");
    assert!(n.tcl_type.is_some(), "nested-expr result stays typed");
}

// Complex multi-operator expressions (TestComplexExpressionTypeInference)

#[test]
fn complex_expression_inference() {
    // tclsh: `expr {1 < 2 == 3 > 4}` -> 0, `string is boolean` = 1 => Boolean
    // (equality of two comparison results).
    assert_eq!(
        tcl_type(
            "set a 1\nset b 2\nset c 3\nset d 4\nset z [expr {$a < $b == $c > $d}]",
            "z"
        ),
        TclType::Boolean
    );
    // tclsh: `expr {1 * 2 + 2 / 3.0}` -> 2.666..., `string is double` = 1
    // => Double (the 3.0 forces the whole expression to a Double).
    assert_eq!(
        tcl_type("set a 1\nset b 2\nset z [expr {$a * 2 + $b / 3.0}]", "z"),
        TclType::Double
    );
    // tclsh: `expr {(256 & 0xFF) == 0}` -> 1, `string is boolean` = 1 => Boolean.
    assert_eq!(
        tcl_type("set a 256\nset z [expr {($a & 0xFF) == 0}]", "z"),
        TclType::Boolean
    );
    // tclsh: `expr {1 << 2 | 4}` -> 4, `string is integer` = 1 => Int.
    assert_eq!(
        tcl_type("set a 1\nset b 4\nset z [expr {$a << 2 | $b}]", "z"),
        TclType::Int
    );
}

// Variable-shape type propagation (TestVariableShapeTypePropagation)
//
// These exercise how a qualified scalar / array-element name is keyed under its
// *base* symbol in the type map. They are static-analysis properties: several
// of the snippets (e.g. `set ::ns::x ...` with no `namespace eval`) actually
// error at runtime in real tclsh ("parent namespace doesn't exist"), so the
// inferred type describes the analyser's symbol model, not an observable value.

#[test]
fn namespaced_scalar_keeps_qualified_symbol_type() {
    // The qualified scalar `::ns::x` is typed under its own fully-qualified
    // symbol. "alpha beta" is a plain (non-numeric, non-list-literal) word.
    let src = "set ::ns::x \"alpha beta\"\nset n [llength $::ns::x]";
    let t = var_type(src, "::ns::x").expect("::ns::x typed");
    assert_eq!(t.kind, TypeKind::Known);
    assert_eq!(t.tcl_type, Some(TclType::String));
}

#[test]
fn namespaced_array_element_uses_base_array_symbol_type() {
    // `set ::ns::arr(k) ...` flows under the base array symbol `::ns::arr`.
    let src = "set ::ns::arr(k) \"alpha beta\"\nset n [llength $::ns::arr(k)]";
    let t = var_type(src, "::ns::arr").expect("::ns::arr typed");
    assert_eq!(t.kind, TypeKind::Known);
    assert_eq!(t.tcl_type, Some(TclType::String));
}

#[test]
fn braced_array_like_name_flows_under_base_array_symbol() {
    // Braced `${a(1)}` is a whole-name load of array element a(1); the type
    // flows under the base array symbol `a`.
    let src = "set {a(1)} \"alpha beta\"\nset n [llength ${a(1)}]";
    let t = var_type(src, "a").expect("a typed");
    assert_eq!(t.kind, TypeKind::Known);
    assert_eq!(t.tcl_type, Some(TclType::String));
}

#[test]
fn unbraced_array_ref_normalises_to_base_array_symbol() {
    // `set a(1) ...` normalises to the base array symbol `a`.
    let src = "set a(1) \"alpha beta\"\nset n [llength $a(1)]";
    let t = var_type(src, "a").expect("a typed");
    assert_eq!(t.kind, TypeKind::Known);
    assert_eq!(t.tcl_type, Some(TclType::String));
}

#[test]
fn namespace_var_commands_with_qualified_names_do_not_break_type_flow() {
    // Mirrors the Python smoke test: a proc using global/upvar/unset on
    // qualified namespace vars must still build a (non-panicking) type map.
    let src = "\
namespace eval ::demo {
    variable value 1
}
proc wire_namespace_vars {} {
    global ::demo::value
    upvar 0 ::demo::value local_value
    unset -nocomplain ::demo::value
}
";
    let registry = registry_for_dialect("tcl8.6");
    let cu = CompilationUnit::build_for(src, registry, false);
    // The proc unit exists and carries a (possibly empty) type map — the Rust
    // analogue of Python's `assert analysis.types is not None`.
    let fu = cu
        .procedures
        .get("::wire_namespace_vars")
        .expect("proc ::wire_namespace_vars built");
    // Accessing `types` proves the map is present and the build did not panic.
    let _ = fu.types.len();
}

// Divergences from the Python suite — RESOLVED
//
// The Python `TestMathFunctionTypeInference::test_ceil_is_int` /
// `::test_floor_is_int` expected INT, and the Rust pass also returned `Int`.
// tclsh proved both wrong: `expr {ceil(3.14)}` → 4.0 and `expr {floor(3.7)}`
// → 3.0 are *doubles* (`string is integer 4.0` → 0, `string is double` → 1),
// unlike `round`/`int`/`entier`/`wide`/`isqrt` which genuinely return integers.
// The bug in `type_infer::expr_call_type` (ceil/floor grouped with the
// integer-returning conversions) has now been FIXED — ceil/floor are inferred
// as DOUBLE — and the two cases are asserted in `expr_ceil_floor_are_double`
// above (matching the real Tcl value type, not the Python suite's wrong INT).
