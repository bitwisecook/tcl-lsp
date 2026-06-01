"""Adversarial / differential tests for the optimiser, checked against C Tcl 9.

The strategy is differential equivalence: for each tricky snippet we run the
*original* program on ``tclsh9.0``, optimise it (aggressive, multi-pass), and
run the *optimised* program too.  A behaviour change (different exit status or
stdout) means the optimiser produced incorrect code.

The snippets are designed to trip the optimiser up — variable aliasing
(``global`` / ``variable`` / ``upvar`` / ``uplevel``), traces, command
substitution side effects, ``expr`` corner cases (bignums, floats, signed
division, hex/octal), dynamic command (re)definition, and dead code that is
only dead under a misread of the semantics.

A second group asserts *missed-optimisation* coverage: provably-correct
rewrites the optimiser should make (and that must stay behaviour-preserving).
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.optimiser import optimise_source_multipass
from compiler.registry.runtime import configure_signatures


def _find_tcl9() -> str | None:
    for cand in (shutil.which("tclsh9.0"), shutil.which("tclsh")):
        if not cand or not Path(cand).exists():
            continue
        try:
            proc = subprocess.run(
                [cand], input="puts [info patchlevel]", capture_output=True, text=True, timeout=5
            )
        except (subprocess.TimeoutExpired, OSError):
            continue
        if proc.stdout.strip().startswith("9."):
            return cand
    return None


_TCLSH9 = _find_tcl9()
pytestmark = pytest.mark.skipif(_TCLSH9 is None, reason="tclsh9.0 not available")


def _run(source: str) -> tuple[int, str]:
    assert _TCLSH9 is not None
    proc = subprocess.run(
        [_TCLSH9], input=source, capture_output=True, text=True, timeout=15
    )
    return proc.returncode, proc.stdout


def _optimise(source: str) -> str:
    configure_signatures(dialect="tcl9.0")
    optimised, _opts, _iters = optimise_source_multipass(source, max_iterations=15)
    return optimised


# ---------------------------------------------------------------------------
# Correctness traps: optimised output MUST match the original on tclsh9.0.
# ---------------------------------------------------------------------------

# Each value is a complete script that prints a deterministic line.
_CORRECTNESS: dict[str, str] = {
    # --- variable aliasing: a callee mutating an outer-scope variable must
    #     invalidate the caller's constant knowledge of it ---
    "global_decl_write": (
        "set g 1\nproc f {} {global g; set g 5}\nf\nputs $g"
    ),
    "global_incr": (
        "set g 1\nproc f {} {global g; incr g}\nf\nf\nputs $g"
    ),
    "global_qualified": (
        "set ::g 1\nproc f {} {set ::g 5}\nf\nputs $::g"
    ),
    "global_interleaved": (
        "set g 5\nproc f {} {global g; set g 9}\nputs $g\nf\nputs $g"
    ),
    "namespace_variable_write": (
        "namespace eval n {variable v 1}\n"
        "proc n::f {} {variable v; set v 5}\nn::f\nputs $n::v"
    ),
    "global_array_elem": (
        "set arr(k) 1\nproc f {} {global arr; set arr(k) 9}\nf\nputs $arr(k)"
    ),
    "uplevel_zero_global": (
        "proc run {} {uplevel #0 {set g 9}}\nset g 1\nrun\nputs $g"
    ),
    "upvar_caller_static": (
        "proc setit {} {upvar 1 y x; set x 99}\n"
        "proc caller {} {set y 1; setit; return $y}\nputs [caller]"
    ),
    "upvar_caller_dynamic": (
        "proc setit {n} {upvar 1 $n x; set x 99}\n"
        "proc caller {} {set y 1; setit y; return $y}\nputs [caller]"
    ),
    "uplevel_caller_script": (
        "proc run {s} {uplevel 1 $s}\n"
        "proc caller {} {set y 1; run {set y 42}; return $y}\nputs [caller]"
    ),
    # --- traces make every write to a variable observable ---
    "trace_write_dead_store": (
        "set log {}\nset x 1\n"
        "trace add variable x write {apply {{a b c} {lappend ::log hit}}}\n"
        "set x 2\nset x 3\nputs $::log"
    ),
    "trace_append_chain": (
        "set log {}\nset s {}\n"
        "trace add variable s write {apply {{a b c} {lappend ::log w}}}\n"
        "append s a\nappend s b\nputs $s|$::log"
    ),
    # --- command substitution side effects must survive ---
    "cmdsub_side_effect": "set x [puts hi]\nputs \"got=$x\"",
    "dead_store_with_cmdsub": "set x [puts a]\nset x [puts b]\nputs done",
    # --- expr corner cases: folding must match Tcl 9 exactly ---
    "bignum_pow": "puts [expr {2 ** 100}]",
    "float_fold": "puts [expr {0.1 + 0.2}]",
    "signed_div_mod": "puts \"[expr {-7 / 2}]|[expr {-7 % 2}]|[expr {7 % -2}]\"",
    "hex_and_octal": "puts \"[expr {0x10 + 1}]|[expr {0o17 + 1}]\"",
    "ternary_string": "set a 1\nputs [expr {$a ? \"yes\" : \"no\"}]",
    "string_eq_number": "set x 10\nif {$x == \"10\"} {puts eq}",
    # --- incr-idiom soundness: a float var must NOT become incr ---
    "incr_idiom_float": "set x 1.5\nset x [expr {$x + 1}]\nputs $x",
    # --- multiply-by-one is identity only for numbers; string must error/keep ---
    "mul_by_one_string": "set x abc\nif {[catch {expr {$x * 1}} e]} {puts \"err\"}",
    # --- dynamic command (re)definition ---
    "redefine_proc": "proc f {} {return 1}\nproc f {} {return 2}\nputs [f]",
    "rename_user_proc": "proc f {} {return 7}\nrename f g\nputs [g]",
    # --- control flow with side effects ---
    "while_side_condition": "set i 0\nset n 0\nwhile {[incr i] <= 3} {incr n}\nputs $n",
    "foreach_multi_var": (
        "set out {}\nforeach {a b} {1 2 3 4} {lappend out $a$b}\nputs $out"
    ),
    "dead_code_after_return": "proc f {} {return 1\nputs unreachable}\nputs [f]",
    # --- misc value-shape traps ---
    "indirect_var_name": "set name x\nset x 42\nputs [set $name]",
    "list_special_chars": "set x [list \"a b\" {c} \\$d]\nputs $x",
    "empty_string_eq": "set x \"\"\nif {$x eq \"\"} {puts empty}",
    "set_returns_value": "puts [set x 42]",
}


@pytest.mark.parametrize("name", sorted(_CORRECTNESS))
def test_optimisation_preserves_behaviour(name: str) -> None:
    source = _CORRECTNESS[name]
    before = _run(source)
    optimised = _optimise(source)
    after = _run(optimised)
    assert after == before, (
        f"optimiser changed behaviour for {name!r}\n"
        f"  before={before!r}\n  after ={after!r}\n  optimised:\n{optimised}"
    )


# ---------------------------------------------------------------------------
# Known limitation: renaming/redefining a *builtin* defeats the optimiser's
# assumption that core commands keep their semantics.  Here ``incr`` is
# replaced by a proc that adds 100, but the optimiser still folds
# ``set c 0; incr c`` to ``1`` using builtin ``incr`` semantics.  Documented
# as a strict-xfail so it auto-flags if ever fixed.
# ---------------------------------------------------------------------------

@pytest.mark.xfail(
    reason="optimiser assumes builtins (incr/set/...) are not renamed/redefined",
    strict=True,
)
def test_rename_builtin_is_unsound() -> None:
    source = (
        "rename incr ::orig_incr\n"
        "proc incr {v} {upvar 1 $v x; ::orig_incr x 100}\n"
        "set c 0\nincr c\nputs $c"
    )
    before = _run(source)
    after = _run(_optimise(source))
    assert after == before


# ---------------------------------------------------------------------------
# Missed-optimisation coverage: provably-correct rewrites the optimiser should
# perform (and that must stay behaviour-preserving).
# ---------------------------------------------------------------------------

_SHOULD_OPTIMISE: dict[str, tuple[str, str]] = {
    # (script, substring that must appear in the optimised source)
    "const_set_return": (
        "proc f {} {set x 5\nreturn $x}\nputs [f]",
        "return 5",
    ),
    "constant_expr_fold": (
        "puts [expr {2 + 3 * 4}]",
        "puts 14",
    ),
    "nested_expr_unwrap": (
        "proc f {x} {return [expr {[expr {$x + 1}]}]}\nputs [f 4]",
        "return [expr {$x + 1}]",
    ),
    "pure_proc_fold": (
        "proc sq {n} {return [expr {$n * $n}]}\nputs [sq 6]",
        "puts 36",
    ),
    "string_build_chain_local": (
        "proc f {} {set s a\nappend s b\nappend s c\nreturn $s}\nputs [f]",
        "return abc",
    ),
    # Builtin command substitutions fold (O129).
    "builtin_cmdsub_fold": (
        "puts [string length abcde]",
        "puts 5",
    ),
    "format_padding_fold": (
        "puts [format %03d 7]",
        "puts 007",
    ),
    # A constant read *inside* a direct `[expr {...}]` command sub propagates
    # and folds (the SSA "gap B" reads, now recovered via reaching versions).
    "expr_cmdsub_const_prop": (
        "proc f {} {set x 5\nreturn [expr {$x + 1}]}\nputs [f]",
        "return 6",
    ),
    "expr_cmdsub_identity": (
        "proc f {} {set x 5\nreturn [expr {$x * 1}]}\nputs [f]",
        "return 5",
    ),
    # Constant string comparison (`eq`/`ne`/`lt`/…) folds.
    "string_ne_fold": (
        'puts [expr {"x" ne "y"}]',
        "puts 1",
    ),
    "string_lt_fold": (
        'puts [expr {"a" lt "b"}]',
        "puts 1",
    ),
}


@pytest.mark.parametrize("name", sorted(_SHOULD_OPTIMISE))
def test_optimiser_performs_correct_rewrite(name: str) -> None:
    source, expected_fragment = _SHOULD_OPTIMISE[name]
    optimised = _optimise(source)
    # The optimisation must actually fire …
    assert expected_fragment in optimised, (
        f"missed optimisation for {name!r}: {expected_fragment!r} not in:\n{optimised}"
    )
    # … and must not change behaviour.
    assert _run(optimised) == _run(source)
