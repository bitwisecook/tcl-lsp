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
    proc = subprocess.run([_TCLSH9], input=source, capture_output=True, text=True, timeout=15)
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
    "global_decl_write": ("set g 1\nproc f {} {global g; set g 5}\nf\nputs $g"),
    "global_incr": ("set g 1\nproc f {} {global g; incr g}\nf\nf\nputs $g"),
    "global_qualified": ("set ::g 1\nproc f {} {set ::g 5}\nf\nputs $::g"),
    "global_interleaved": ("set g 5\nproc f {} {global g; set g 9}\nputs $g\nf\nputs $g"),
    "namespace_variable_write": (
        "namespace eval n {variable v 1}\nproc n::f {} {variable v; set v 5}\nn::f\nputs $n::v"
    ),
    "global_array_elem": ("set arr(k) 1\nproc f {} {global arr; set arr(k) 9}\nf\nputs $arr(k)"),
    "uplevel_zero_global": ("proc run {} {uplevel #0 {set g 9}}\nset g 1\nrun\nputs $g"),
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
    "cmdsub_side_effect": 'set x [puts hi]\nputs "got=$x"',
    "dead_store_with_cmdsub": "set x [puts a]\nset x [puts b]\nputs done",
    # --- expr corner cases: folding must match Tcl 9 exactly ---
    "bignum_pow": "puts [expr {2 ** 100}]",
    "float_fold": "puts [expr {0.1 + 0.2}]",
    "signed_div_mod": 'puts "[expr {-7 / 2}]|[expr {-7 % 2}]|[expr {7 % -2}]"',
    "hex_and_octal": 'puts "[expr {0x10 + 1}]|[expr {0o17 + 1}]"',
    "ternary_string": 'set a 1\nputs [expr {$a ? "yes" : "no"}]',
    "string_eq_number": 'set x 10\nif {$x == "10"} {puts eq}',
    # --- incr-idiom soundness: a float var must NOT become incr ---
    "incr_idiom_float": "set x 1.5\nset x [expr {$x + 1}]\nputs $x",
    # --- multiply-by-one is identity only for numbers; string must error/keep ---
    "mul_by_one_string": 'set x abc\nif {[catch {expr {$x * 1}} e]} {puts "err"}',
    # --- dynamic command (re)definition ---
    "redefine_proc": "proc f {} {return 1}\nproc f {} {return 2}\nputs [f]",
    "rename_user_proc": "proc f {} {return 7}\nrename f g\nputs [g]",
    # --- control flow with side effects ---
    "while_side_condition": "set i 0\nset n 0\nwhile {[incr i] <= 3} {incr n}\nputs $n",
    "foreach_multi_var": ("set out {}\nforeach {a b} {1 2 3 4} {lappend out $a$b}\nputs $out"),
    "dead_code_after_return": "proc f {} {return 1\nputs unreachable}\nputs [f]",
    # --- misc value-shape traps ---
    "indirect_var_name": "set name x\nset x 42\nputs [set $name]",
    "list_special_chars": 'set x [list "a b" {c} \\$d]\nputs $x',
    "empty_string_eq": 'set x ""\nif {$x eq ""} {puts empty}',
    "set_returns_value": "puts [set x 42]",
    # --- lappend list-build chains (O130): these must stay equivalent whether
    #     or not they fold (escaping / traced / aliased vars must NOT fold) ---
    "lappend_traced": (
        "set log {}\nset l {}\n"
        "trace add variable l write {apply {{a b c} {lappend ::log w}}}\n"
        "lappend l a\nlappend l b\nputs $l|$::log"
    ),
    "lappend_global": ("set l {}\nproc f {} {global l; lappend l a; lappend l b}\nf\nputs $l"),
    "lappend_upvar": ("proc f {} {upvar 1 l x; lappend x a; lappend x b}\nset l {}\nf\nputs $l"),
    "lappend_read_between": ("set l {}\nlappend l a\nputs $l\nlappend l b\nputs $l"),
    "lappend_nested_elements": "set l {}\nlappend l {a b} {c d}\nputs $l",
    # A braced literal that merely *contains* brackets must not be mistaken for
    # a command substitution when folding a nested builtin (llength of the
    # literal string "[list a b]" is 3, not the 2 a wrongly-folded sub gives).
    "nested_cmdsub_braced_literal": "puts [llength {[list a b]}]",
    "nested_cmdsub_quoted_literal": 'puts [string length "[x]"]',
    # --- command-binding lattice: renamed / redefined commands must not fold
    #     with their original builtin / proc semantics ---
    "rename_builtin_redefined_proc": (
        "rename incr ::orig_incr\n"
        "proc incr {v} {upvar 1 $v x; ::orig_incr x 100}\n"
        "set c 0\nincr c\nputs $c"
    ),
    "rename_builtin_append_chain": (
        "rename append ::ap\nproc append {args} {return X}\nset s a\nappend s b\nputs $s"
    ),
    "rename_builtin_lappend_chain": (
        "rename lappend ::lp\nproc lappend {args} {return Z}\nset l {}\nlappend l a\nputs $l"
    ),
    "rename_proc_then_call_renamed_name": ("proc a {} {return 5}\nputs [a]\nrename a b\nputs [b]"),
    "redefine_builtin_string_as_proc": (
        "proc string {args} {return HACKED}\nputs [string length hi]"
    ),
    "interp_alias_over_builtin": ("interp alias {} llength {} return\nputs [llength {a b c}]"),
    # Calling the *old* name after a rename must not fold (it now hits unknown
    # → errors): same stdout-then-error on both original and optimised.
    "call_renamed_away_name_errors": ("proc a {} {return 5}\nputs [a]\nrename a b\nputs [a]"),
    # A rename buried in a proc body distrusts that builtin unit-wide.
    "rename_inside_proc_body": ("proc danger {} {rename string ::s2}\nputs [string length hi]"),
    # Return-value string interpolation must respect aliasing: ``v`` is an
    # upvar alias of the caller's variable, so ``return "v=$v"`` must NOT bake in
    # any stale same-block value.
    "return_interp_upvar_alias": (
        'proc setit {} {upvar 1 y v; set v 7; return "v=$v"}\n'
        "proc caller {} {set y 1; return [setit]}\nputs [caller]"
    ),
    # --- Gap-B reaching-version soundness: a variable read *inside* a hidden
    #     expr / nested command substitution is absent from ``stmt.uses``, so the
    #     semi-pruned SSA inserts no φ for it.  ``entry_versions`` then names a
    #     *pre-join* version after a conditional reassignment — folding the
    #     hidden read against that stale constant would change behaviour.  Only a
    #     def earlier in the *same* straight-line block may be pinned. ---
    "gapb_branch_join_expr_var": (
        "set x 5\nif {[string length q]==1} {set x 9}\nputs [expr {$x + 1}]"
    ),
    "gapb_branch_join_nested_cmdsub": (
        "set s abc\nif {[string length x]==1} {set s qrst}\nputs [expr {[string length $s]}]"
    ),
    "gapb_loop_carried_mutation": (
        "set s abc\nset n 0\nwhile {$n<2} {puts [expr {[string length $s]}]\nset s xyzw\nincr n}"
    ),
    "gapb_unset_then_reset": ("set s abc\nunset s\nset s wxyz\nputs [expr {[string length $s]}]"),
    # --- command subs embedded in interpolation strings must stay equivalent
    #     whether or not they fold (raw splice must not introduce substitutions
    #     or, in a *bare* word, extra word boundaries) ---
    "string_cmdsub_bare_space_result": "puts pre[list a b c]post",
    "string_cmdsub_seq_reassign": ('set x 3\nputs "a=[expr {$x}]"\nset x 99\nputs "b=[expr {$x}]"'),
    # --- subst folds only the literal form; backslash / command / option forms
    #     must be left to run ---
    "subst_backslash_runs": "puts [subst {a\\tb}]",
    "subst_command_runs": "puts [subst {x[set y 5]z}]",
    # --- pure proc returning a string with substitution chars must NOT bake in
    #     the raw (un-evaluated) return text ---
    "proc_string_escaped_dollar": 'proc f {} {return "a\\$b"}\nputs [f]',
    # --- CONSTSET soundness: a branch-merged variable (provably one of a finite
    #     set of constants, but not a *single* one) must NOT resolve to a single
    #     value when folding a command sub that reads it.  ``[clock seconds] > 0``
    #     is always true at run time but opaque to the optimiser, so ``s`` is the
    #     CONSTSET ``{abcde, xy}``; folding ``[string length $s]`` would splice
    #     the literal ``"None"`` (the CONSTSET's empty ``value`` field) and bake
    #     in a wrong length.  Both the whole-word and the in-string forms. ---
    "constset_cmdsub_wholeword": (
        "set s abcde\nif {[clock seconds] > 0} {set s xy}\nputs [string length $s]"
    ),
    "constset_cmdsub_in_string": (
        'set s abcde\nif {[clock seconds] > 0} {set s xy}\nputs "L=[string length $s]"'
    ),
    "constset_interpolation_assign": (
        'set s abcde\nif {[clock seconds] > 0} {set s xy}\nset y "v=$s"\nputs $y'
    ),
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
# Renaming / redefining a *builtin* is now handled soundly: the command-binding
# lattice marks the rebound name untrusted, so the optimiser refuses to fold it
# with builtin semantics.  Here ``incr`` is replaced by a proc that adds 100;
# ``set c 0; incr c`` must NOT fold to ``1``.
# ---------------------------------------------------------------------------


def test_rename_builtin_is_sound() -> None:
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
    # lappend list-build chain folds (O130).
    "lappend_chain_fold": (
        "proc f {} {set l {}\nlappend l a\nlappend l b\nlappend l c\nreturn $l}\nputs [f]",
        "return {a b c}",
    ),
    # scan of literal constants folds (O129 via the const-fold callback).
    "scan_int_fold": (
        "puts [scan 42 %d]",
        "puts 42",
    ),
    # list/lindex now fold via the registry (not the old single-element-only
    # hand-rolled folder) — multi-element and special-char lists included.
    "list_multi_element_fold": (
        "puts [list a b c]",
        "puts {a b c}",
    ),
    "list_special_char_fold": (
        'puts [list "a b" c]',
        "puts {{a b} c}",
    ),
    "lindex_fold": (
        "puts [lindex {a b c} 1]",
        "puts b",
    ),
    # A *safe* string constant (no Tcl-special chars) folds into an
    # interpolating string (O105), not just numeric constants.
    "string_const_interp": (
        'set greeting hello\nputs "$greeting world"',
        'puts "hello world"',
    ),
    # Mixed string of a folded *and* an unfolded var: the folded var
    # collapses while the live one survives (per-use-site DCE).  ``a`` is
    # defined before a branch (different block → not a same-block constant
    # here) but ``b`` is local, so only ``$b`` folds; ``set a`` must stay.
    "mixed_interp_keeps_live_def": (
        'set x 5\nset a [expr {$x + $x}]\nif {$a > 5} {puts hi}\nset b 99\nputs "$a $b"',
        'puts "$a 99"',
    ),
    # Nested builtin command subs fold inside-out: the inner [list a b c]
    # becomes the literal {a b c}, then [llength {a b c}] folds to 3.
    "nested_cmdsub_fold": (
        "puts [llength [list a b c]]",
        "puts 3",
    ),
    "nested_cmdsub_fold_string": (
        "puts [string length [string cat ab cd]]",
        "puts 4",
    ),
    # A safe constant folds into an interpolating *return* value, too (the
    # return terminator runs the same O105 propagation as ordinary statements).
    "return_string_interp": (
        'proc f {} {set x hi\nreturn "got $x"}\nputs [f]',
        'return "got hi"',
    ),
    "return_numeric_interp": (
        'proc f {} {set n 5\nreturn "n=$n"}\nputs [f]',
        'return "n=5"',
    ),
    # A pure builtin command sub embedded *inside* an expr folds (the inner
    # [string length abc] → 3, then 3 + 2 → 5).
    "expr_embedded_cmdsub_arith": (
        "puts [expr {[string length abc] + 2}]",
        "puts 5",
    ),
    "expr_embedded_cmdsub_string": (
        'puts [expr {[string toupper hi] eq "HI"}]',
        "puts 1",
    ),
    # …including in a branch condition (folds the whole `if` away).
    "expr_embedded_cmdsub_branch": (
        "if {[llength {a b c}] == 3} {puts three}",
        "puts three",
    ),
    # Flow-sensitive precision: a proc call *before* its rename still folds
    # (only calls after the rename are blocked).
    "fold_proc_before_its_rename": (
        "proc a {} {return 5}\nputs [a]\nrename a b\n",
        "puts 5",
    ),
    # Renaming one builtin must not stop folding of an *unrelated* builtin.
    "unrelated_builtin_still_folds_after_rename": (
        "rename incr ::orig_incr\nputs [string length hello]\n",
        "puts 5",
    ),
    # Part B1: a pure builtin command sub embedded *inside an interpolation
    # string* folds (the optimiser already folds whole-word subs + ``$var`` in
    # strings; this closes the embedded-sub gap).
    "string_interp_builtin_cmdsub": (
        'puts "v=[string length abc]"',
        'puts "v=3"',
    ),
    "string_interp_expr_cmdsub": (
        'set x 3\nputs "sq=[expr {$x*$x}]"',
        'puts "sq=9"',
    ),
    "string_interp_multi_cmdsub": (
        'puts "a[string length xy]b[string toupper hi]"',
        'puts "a2bHI"',
    ),
    # Part B2 (Gap-B): a constant read inside a *nested* ``[expr {[cmd $v]}]``
    # folds via the threaded reaching-version constant (same-block def only).
    "gapb_nested_cmdsub_folds": (
        "set s abcd\nputs [expr {[string length $s]}]",
        "puts 4",
    ),
    "gapb_nested_cmdsub_arith": (
        "set s abc\nputs [expr {[string length $s] * 2}]",
        "puts 6",
    ),
    # Part B3: a pure proc returning a *multi-word* string folds (previously only
    # numeric / bare-word results folded).
    "pure_proc_multiword_string": (
        'proc f {} {return "hi there"}\nputs [f]',
        "puts {hi there}",
    ),
    # Part B4: a literal ``[subst {...}]`` folds via the registry const_fold.
    "subst_literal_fold": (
        "puts [subst {hello world}]",
        "puts {hello world}",
    ),
    "subst_in_interp_string": (
        'puts "x=[subst {hi}]"',
        'puts "x=hi"',
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
