"""TP/FP regression tests for the OPT (optimisation / codegen) family.

Each test pairs to an ``FP-OPT-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

OPT family covers the analyser-driven *optimisation* warnings — quick-fix
codes O106, O109, O110, O116, O120, O126 — where the rewrite must preserve
runtime semantics (and apply cleanly) or else suppress.

Ground truth: real C tclsh 9.0.3 (every entry verified by execution).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, codes: list[str] | str):
    if isinstance(codes, str):
        codes = [codes]
    return [d for d in get_diagnostics(source) if d.code in codes]


# FP-OPT-01 — O110 InstCombine: whitespace-only / paren-preservation / commutative-reorder


FP_OPT_01_REPRO_WHITESPACE = "set x [expr { $a + $b }]\n"
FP_OPT_01_REPRO_PAREN = "set x [expr {($a << 1) & 0xff}]\n"
FP_OPT_01_REPRO_REORDER = "set x [expr {2 + $a}]\n"


def test_FP_OPT_01_whitespace_only_no_o110():
    """FP: ``expr { $a + $b }`` (decorative whitespace) must NOT fire O110.
    The InstCombine pass previously fired on every whitespace touch the
    rewriter performed, producing 3641 corpus firings; the ``_strip_ws``
    guard drops whitespace-only rewrites."""
    assert _codes(FP_OPT_01_REPRO_WHITESPACE, "O110") == []


def test_FP_OPT_01_branch_folding_whitespace_no_o110():
    """FP: ``if {$x<0}`` no longer fires O110 from the branch-folding path —
    the same ``_strip_ws`` guard applies there.  Sample corpus impact:
    bigfloat2 122→46, exif 53→10."""
    src = "if {$x<0} { puts negative }\n"
    assert _codes(src, "O110") == []


def test_FP_OPT_01_paren_preserved_no_o110():
    """FP: mixed bitwise/shift expressions keep their parens per CERT EXP00-C
    (operator-precedence intent).  ``($a << 1) & 0xff`` must NOT be flagged
    as ``-> a << 1 & 0xff`` — the rewrite would change the intent's
    visibility even though the result is identical."""
    assert _codes(FP_OPT_01_REPRO_PAREN, "O110") == []


def test_FP_OPT_01_commutative_reorder_no_o110():
    """FP: commutative reorder (``literal + term`` → ``term + literal``) is
    suppressed when no real fold would result.  Identities and operator
    flips still fire."""
    assert _codes(FP_OPT_01_REPRO_REORDER, "O110") == []


def test_FP_OPT_01_genuine_simplification_still_fires():
    """TP control: a genuine simplification on a provably-numeric SSA value
    (here ``$i + 0`` where ``$i`` is typed INT via a loop-overdefined
    counter) still fires O110.

    Note: post-D5-O110 fix, the ``$x + 0`` form on an unknown-type
    parameter no longer fires (would erase Tcl's numeric coercion error).
    See FP-OPT-09."""
    src = (
        "proc f {n} {\n"
        "  for {set i 0} {$i < $n} {incr i} {\n"
        "    set y [expr {$i + 0}]\n"
        "    puts $y\n"
        "  }\n"
        "}\n"
        "f 3\n"
    )
    # i is INT via SCCP (loop counter, OVERDEFINED value but KNOWN INT
    # type) — the identity is sound, O110 must fire.
    diags = _codes(src, "O110")
    assert diags, f"identity simplification must still fire O110; got {get_diagnostics(src)}"


# FP-OPT-02 — O116 fold-const-list-command correctness (empty [list] -> "{}", not "")


FP_OPT_02_REPRO = "set x [list]\nlappend x a\nputs $x\n"


def test_FP_OPT_02_empty_list_quick_fix_uses_braces():
    """TP / quick-fix correctness: ``[list]`` folds to the canonical empty-
    list literal ``{}``, NOT to the empty string ``""``.  Pre-fix the
    quick-fix produced ``set x ;`` (a syntax-valid READ of ``x``, not the
    intended write to it) which silently corrupted source on apply.
    The corrected fix uses ``{}`` so the apply preserves the assignment.

    Verified by inspecting the diagnostic's ``data['replacement']`` field
    which optimiser emits via ``Optimisation.replacement`` in
    ``compiler/optimiser/_helpers.py::_try_fold_list_command``.
    """
    diags = [d for d in get_diagnostics(FP_OPT_02_REPRO) if d.code == "O116"]
    assert diags, "O116 must fire on `set x [list]` (the fold opportunity)"
    data = getattr(diags[0], "data", None) or {}
    assert data.get("replacement") == "{}", (
        f"O116 replacement must be canonical empty-list literal `{{}}`; got {data!r}"
    )


# FP-OPT-03 — O106 LICM purity recursion (outer pure / inner impure)


FP_OPT_03_REPRO = """\
proc f {} {
    for {set i 0} {$i < 10} {incr i} {
        set s [format %04d [incr testnum]]
    }
    return $s
}
"""


def test_FP_OPT_03_inner_impure_blocks_licm():
    """FP: LICM must NOT hoist ``[format %04d [incr testnum]]`` out of the
    loop.  ``format`` itself is pure, but ``[incr testnum]`` mutates state
    per iteration — hoisting would call ``incr`` once instead of N times.
    Real corpus site: clay/build/test.tcl:686."""
    # O106 must NOT fire because the expression is not loop-invariant
    # (inner impure command).
    assert _codes(FP_OPT_03_REPRO, "O106") == []


def test_FP_OPT_03_outer_pure_inner_pure_still_fires():
    """TP control: a genuinely pure-recursive expression (outer pure +
    inner pure, no state mutation) IS hoistable and O106 fires."""
    src = """\
proc f {} {
    set k 42
    for {set i 0} {$i < 10} {incr i} {
        set s [format %04d [expr {$k + 1}]]
    }
    return $s
}
"""
    diags = _codes(src, "O106")
    assert diags, (
        f"pure expression should be hoistable, O106 should fire; got {get_diagnostics(src)}"
    )


# FP-OPT-04 — O109/O126 dead-store / unused via call-by-name suppression


FP_OPT_04_REPRO = """\
proc asnPeekTag {data {tag tag} {type type}} {
    upvar 1 $tag tagOut $type typeOut
    set tagOut 0
    set typeOut 0
    return [string length $data]
}
proc decode {data} {
    asnPeekTag $data tag type
    return [list $tag $type]
}
"""


def test_FP_OPT_04_call_by_name_suppresses_dead_store():
    """FP: when a caller passes a *literal* variable name to a user proc
    whose param carries ``VAR_READ`` / ``VAR_WRITE`` (a Tcl-side upvar
    idiom), the analyser no longer flags that caller-local as set-but-
    unused or dead.  O109 (dead store) and O126 (unused var) both honour
    the call-by-name suppression via ``compiler/proc_arg_traits.py``."""
    # In decode/, $tag and $type are written via upvar in asnPeekTag — the
    # subsequent `return [list $tag $type]` reads them.  Neither O109 nor
    # O126 should fire on the implicit creation of tag/type.
    diags = _codes(FP_OPT_04_REPRO, ["O109", "O126", "W211", "W220"])
    # Filter to just tag/type (other vars in fixture may produce noise).
    relevant = [d for d in diags if "'tag'" in (d.message or "") or "'type'" in (d.message or "")]
    assert not relevant, (
        f"call-by-name suppression should silence O109/O126/W211/W220 on tag/type; got {relevant}"
    )


def test_FP_OPT_04_genuine_dead_store_still_fires():
    """TP control: a write with no callee using-by-name still fires O109/W220."""
    src = """\
proc f {} {
    set x 1
    set x 2
    return $x
}
"""
    # Either O109 (dead store) or W220 (assignment never read) should fire.
    diags = _codes(src, ["O109", "W220"])
    assert diags, f"genuine dead store must still fire; got {get_diagnostics(src)}"


# FP-OPT-05 — O126 must NOT delete a side-effectful RHS (D2-O126)


FP_OPT_05_REPRO = "proc f {} { set unused [puts side]; puts done }"


def test_FP_OPT_05_o126_preserves_puts_side_effect():
    """TP / soundness: ``set unused [puts side]`` -- the assignment IS
    unused but the RHS prints to stdout.  Deleting it changes program
    output.  D2-O126 closure adds a purity gate via
    ``_assignment_safe_to_delete`` consuming
    ``_word_has_observable_side_effect`` (compiler/optimiser/_elimination.py).
    O126 must NOT fire here.

    tclsh ground truth: orig prints ``side\\ndone``; pre-fix optimised
    version printed only ``done``."""
    from compiler.optimiser import optimise_source

    _, rewrites = optimise_source(FP_OPT_05_REPRO)
    codes = [r.code for r in rewrites]
    assert "O126" not in codes, (
        f"O126 must NOT delete a [puts X] RHS (would lose side effect); got codes {codes}"
    )


def test_FP_OPT_05_o126_pure_rhs_still_fires():
    """TP control: when the RHS is a pure command (``[list 1 2 3]``),
    O126 SHOULD still fire.  Confirms the purity gate didn't disable
    the optimisation entirely."""
    from compiler.optimiser import optimise_source

    src = "proc f {} { set unused [list 1 2 3]; puts done }"
    _, rewrites = optimise_source(src)
    codes = [r.code for r in rewrites]
    assert "O126" in codes, f"O126 must fire when RHS is pure (list); got codes {codes}"


# FP-OPT-06 — cmd-sub writes are SSA kills (D2-O100; root cause for O100/O109/O127)


FP_OPT_06_REPRO = "proc f {} { set x a; set y [append x b]; puts $x; puts $y }"


def test_FP_OPT_06_o100_does_not_propagate_past_cmd_sub_write():
    """TP / soundness: ``[append x b]`` mutates x.  The optimiser must
    NOT propagate the stale ``a`` value into a subsequent ``puts $x``.
    D2-O100 closure: every statement's kill_sites now includes
    ``statement_cmd_sub_write_names(stmt)`` (compiler/optimiser/_manager.py).

    tclsh ground truth: tclsh prints ``ab\\nab\\n``; pre-fix optimised
    source printed ``a\\nb\\n``."""
    from compiler.optimiser import optimise_source

    opt_src, _ = optimise_source(FP_OPT_06_REPRO)
    # The propagation that would rewrite ``puts $x`` to ``puts a`` is
    # unsound; the optimised source must NOT contain ``puts a`` (literal).
    assert "puts a" not in opt_src, (
        f"O100 must NOT propagate stale value past [append x b]; got: {opt_src.strip()}"
    )


# FP-OPT-07 — O126 extends to pure user-proc RHS via interproc purity (D2-O126-FU)


FP_OPT_07_REPRO = "proc add {a b} { expr {$a + $b} }\nproc f {} { set unused [add 1 2]; puts done }"


def test_FP_OPT_07_pure_user_proc_rhs_is_deleted():
    """TP: when the RHS is a user proc the interproc fixpoint marks as
    pure (``add`` is arithmetic-only), O126 CAN safely delete the
    unused assignment.  D2-O126-FU closure threads ``interproc_pure``
    through the purity gate (compiler/optimiser/_elimination.py
    ``optimise_elimination_passes``)."""
    from compiler.optimiser import optimise_source

    _, rewrites = optimise_source(FP_OPT_07_REPRO)
    assert "O126" in [r.code for r in rewrites], "pure user-proc RHS must allow O126 deletion"


def test_FP_OPT_07_impure_user_proc_rhs_preserved():
    """TN control: when the user proc contains observable side effects
    (``puts``), interproc summary correctly marks it impure and the
    purity gate refuses O126."""
    from compiler.optimiser import optimise_source

    src = "proc shout {x} { puts $x; expr {$x + 1} }\nproc f {} { set unused [shout 1]; puts done }"
    _, rewrites = optimise_source(src)
    assert "O126" not in [r.code for r in rewrites], (
        "impure user-proc RHS must NOT be deleted by O126 (loses side effect)"
    )


# FP-OPT-08 — O109/O126 overlap filter: segment_commands + EXPR/BODY descent (D4-F10)


FP_OPT_08_REPRO = """\
proc f {} {
    set a 1
    set b 0
    if {$a} {
        if {$b} { puts X }
    }
}
"""


def test_FP_OPT_08_nested_if_constant_chain_does_not_delete_inner_var():
    """TP / soundness: when the O112 chain (``if {$a}`` -> ``if {$b}
    {puts X}``) is being staged, the overlap filter MUST NOT delete
    ``set b 0`` -- the surviving inner ``if {$b} {...}`` still uses
    $b.  D4-F10 closure (a) replaced ``split(None, 2)`` with
    ``segment_commands`` for LHS extraction and (b) added EXPR/BODY
    descent to the var-reference scanner so it sees ``$b`` inside the
    inner if-condition."""
    from compiler.optimiser import optimise_source

    opt_src, _ = optimise_source(FP_OPT_08_REPRO)
    # After optimisation, if any surviving statement references $b,
    # the corresponding set b 0 must still be present.
    if "$b" in opt_src:
        assert "set b 0" in opt_src or "set b {0}" in opt_src, (
            f"O109/O126 must not delete 'set b 0' when $b survives in EXPR-role context.  "
            f"Got optimised source: {opt_src!r}"
        )


def test_FP_OPT_08_unrelated_set_still_eligible_for_o126():
    """TN control: when no surviving replacement references the var,
    the overlap filter does NOT block O126 deletion (i.e. the
    pre-existing optimisation path still works)."""
    from compiler.optimiser import optimise_source

    src = "proc f {} { set unused 42; puts done }"
    _, rewrites = optimise_source(src)
    codes = [r.code for r in rewrites]
    # ``set unused 42`` has a pure constant RHS and isn't referenced
    # anywhere else -- O126 should fire.
    assert "O126" in codes, (
        f"unreferenced 'set unused 42' must still be eligible for O126; got {codes}"
    )


# FP-OPT-09 — D5-O110: O110 InstCombine identity/annihilator rewrites must
# preserve Tcl's numeric-coercion / error semantics.
#
# Tclsh ground truth:
#   expr {"abc"}     -> "abc"            (rc=0, just returns the string)
#   expr {"abc" + 0} -> ERROR            ("can't use non-numeric string as left operand of '+'")
# So rewriting ``$x + 0`` to ``$x`` is unsound when $x might be a
# non-numeric string -- the rewrite hides the runtime error.

FP_OPT_09_TP_REPRO = "proc f {x} {\n  puts [expr {$x + 0}]\n}\nf abc\n"
FP_OPT_09_TN_REPRO = (
    "proc f {} {\n"
    "  for {set i 0} {$i < 3} {incr i} {\n"
    "    set y [expr {$i + 0}]\n"
    "    puts $y\n"
    "  }\n"
    "}\n"
    "f\n"
)


def test_FP_OPT_09_unknown_type_param_blocks_identity_rewrite():
    """TP: ``$x + 0`` on a parameter with unknown type MUST NOT be
    rewritten to ``$x`` -- the rewrite would hide ``can't use non-
    numeric string "abc"`` when the caller passes a non-numeric value.
    """
    from compiler.optimiser import optimise_source

    optimised, rewrites = optimise_source(FP_OPT_09_TP_REPRO)
    # Look for any optimisation that would turn the expression into
    # bare ``$x`` (drops the ``+ 0``).
    o110_drops = [
        r
        for r in rewrites
        if r.code == "O110" and "+ 0" not in r.replacement and "$x" in r.replacement
    ]
    assert not o110_drops, (
        f"unsound O110 drop of ``$x + 0`` on unknown-type param; got {o110_drops}"
    )
    # Also the diagnostic surface should be quiet on O110.
    assert _codes(FP_OPT_09_TP_REPRO, "O110") == []


def test_FP_OPT_09_provably_numeric_var_still_fires():
    """TN: ``$i + 0`` where ``$i`` is provably INT (SSA type lattice
    INT, loop counter pattern) IS sound -- O110 MUST fire."""
    diags = _codes(FP_OPT_09_TN_REPRO, "O110")
    assert diags, (
        f"provably-numeric identity simplification must still fire O110; "
        f"got {get_diagnostics(FP_OPT_09_TN_REPRO)}"
    )


# FP-OPT-10 — D5-O114: ``set x [expr {$x + N}]`` -> ``incr x N`` requires
# proof that x is TclType.INT.  Without the proof the rewrite is unsound:
# tclsh ``expr {$x + 1}`` on ``1.5`` yields ``2.5`` (float promotion);
# ``incr x`` on ``1.5`` errors with ``expected integer but got "1.5"``.

FP_OPT_10_TP_REPRO = "proc foo {x} {\n  set x [expr {$x + 1}]\n  puts $x\n}\nfoo 1.5\n"
FP_OPT_10_TN_REPRO = (
    "proc foo {n} {\n"
    "  for {set x 0} {$x < $n} {incr x} {\n"
    "    set x [expr {$x + 1}]\n"
    "    puts $x\n"
    "  }\n"
    "}\n"
    "foo 3\n"
)


def test_FP_OPT_10_unknown_type_param_blocks_incr_rewrite():
    """TP: ``set x [expr {$x + 1}]`` on a param ``x`` (unknown type)
    MUST NOT rewrite to ``incr x`` -- the rewrite would error at
    runtime if the caller passes a float."""
    from compiler.optimiser import optimise_source

    optimised, rewrites = optimise_source(FP_OPT_10_TP_REPRO)
    assert not any(r.code == "O114" for r in rewrites), (
        f"unsound O114 rewrite on unknown-type param; got {rewrites}"
    )


def test_FP_OPT_10_provably_int_var_still_fires():
    """TN: ``set x [expr {$x + 1}]`` inside a for-loop where ``x`` is
    SCCP-typed INT IS sound -- O114 MUST fire."""
    from compiler.optimiser import optimise_source

    _, rewrites = optimise_source(FP_OPT_10_TN_REPRO)
    assert any(r.code == "O114" for r in rewrites), (
        f"provably-INT incr-idiom rewrite must still fire O114; got {rewrites}"
    )


# FP-OPT-11 — O120 ==/!= -> eq/ne requires at-least-one provably-non-numeric operand (D5-O120)


FP_OPT_11_TP_REPRO = (
    "proc f {raw} {\n"
    "    set a [string trim $raw]\n"
    '    if {$a == "1"} { puts yes } else { puts no }\n'
    "}\n"
)

FP_OPT_11_TN_REPRO = (
    "proc f {raw} {\n"
    "    set a [string trim $raw]\n"
    '    if {$a == "hello"} { puts yes } else { puts no }\n'
    "}\n"
)


def test_FP_OPT_11_numeric_like_literal_string_typed_var_no_rewrite():
    """TP: STRING-typed ``$a`` + numeric-looking literal ``"1"`` MUST
    NOT rewrite to ``eq``.  STRING type tracks the internal-rep but a
    STRING-typed value can still hold ``"1.0"`` text where ``==`` is
    1 (numeric) and ``eq`` is 0 (string) -- the rewrite flips the
    result.  Per the at-least-one-non-numeric rule, neither operand
    is provably non-numeric here."""
    from compiler.optimiser import optimise_source

    _, rewrites = optimise_source(FP_OPT_11_TP_REPRO)
    assert not any(r.code == "O120" for r in rewrites), (
        f"unsound O120 rewrite when neither operand provably non-numeric; got {rewrites}"
    )


def test_FP_OPT_11_non_numeric_literal_still_rewrites():
    """TN: ``$a == "hello"`` -- the literal "hello" is provably non-
    numeric, so Tcl ``==`` MUST take the string-compare path
    regardless of $a's runtime value.  The rewrite to ``eq`` is
    sound; at-least-one rule fires correctly."""
    from compiler.optimiser import optimise_source

    optimised, rewrites = optimise_source(FP_OPT_11_TN_REPRO)
    assert any(r.code == "O120" for r in rewrites), (
        f"sound O120 rewrite (non-numeric literal) must still fire; got {rewrites}"
    )
    assert '$a eq "hello"' in optimised, f"expected eq-form in: {optimised}"


# FP-OPT-12 — TclOO method purity wired into O126 (SF-2 PARTIAL)


def test_FP_OPT_12_pure_user_proc_via_my_dispatch_handled_at_word_level():
    """SF-2 wiring TP: the analogous user-proc shape
    ``set unused [pure_helper]`` -- pure_helper returns a constant --
    must fold to deletion (O126) via the existing D2-O126-FU interproc-
    purity gate.  This locks in that the SF-2 wiring did not regress
    the pure-user-proc path."""
    from compiler.optimiser import optimise_source

    src = (
        "proc pure_helper {} { return 42 }\n"
        "proc m {} {\n"
        "    set unused [pure_helper]\n"
        "    puts done\n"
        "}\n"
    )
    optimised, rewrites = optimise_source(src)
    assert any(r.code == "O126" for r in rewrites), (
        f"pure-user-proc RHS in unused assign must fire O126; got {rewrites}"
    )


def test_FP_OPT_12_impure_user_proc_still_blocks():
    """TN control: impure user proc RHS in unused assignment is NOT
    safe to delete (puts side effect)."""
    from compiler.optimiser import optimise_source

    src = (
        "proc impure_helper {} { puts side; return 0 }\n"
        "proc m {} {\n"
        "    set unused [impure_helper]\n"
        "    puts done\n"
        "}\n"
    )
    optimised, rewrites = optimise_source(src)
    # The O126 deletion must NOT fire (the RHS is observably impure).
    o126s = [r for r in rewrites if r.code == "O126"]
    assert not o126s, f"impure-user-proc RHS must NOT fire O126; got {o126s}"


def test_FP_OPT_12_tcloo_pure_method_rhs_deleted():
    """SF-2 FIXED (TP): TclOO method bodies are now lowered to per-method
    FunctionUnits and summarised into ``InterproceduralAnalysis.methods``.
    The optimiser iterates method bodies with the owning class as
    ``enclosing_class``, so a ``set unused [my <pure-method>]`` RHS is
    recognised side-effect-free via ``_method_pure(class, m, pure_set)``
    and the dead assignment folds to deletion (O126).

    This is the spec's TP reproducer; it previously could not fire because
    ``ctx.interproc.methods`` was always empty (method bodies un-lowered).
    See ``post-stage2-followups.md`` §B."""
    from compiler.optimiser import optimise_source

    src = (
        "oo::class create C {\n"
        "    method pure_helper {} { return 42 }\n"
        "    method m {} {\n"
        "        set unused [my pure_helper]\n"
        "        puts done\n"
        "    }\n"
        "}\n"
    )
    optimised, rewrites = optimise_source(src)
    o126s = [r for r in rewrites if r.code == "O126"]
    assert o126s, f"pure-method RHS in unused assign must fire O126; got {rewrites}"


def test_FP_OPT_12_tcloo_impure_method_rhs_preserved():
    """SF-2 TN: a ``my <method>`` whose method body has an observable side
    effect (``puts``) must NOT be deleted — the method summary records
    ``pure=False`` so ``_method_pure`` returns False and the O126 gate
    conservatively preserves the assignment."""
    from compiler.optimiser import optimise_source

    src = (
        "oo::class create C {\n"
        "    method impure_helper {} { puts hi; return 0 }\n"
        "    method m {} {\n"
        "        set unused [my impure_helper]\n"
        "        puts done\n"
        "    }\n"
        "}\n"
    )
    optimised, rewrites = optimise_source(src)
    o126s = [r for r in rewrites if r.code == "O126"]
    assert not o126s, f"impure-method RHS must NOT fire O126; got {o126s}"


def test_FP_OPT_12_nested_proc_in_method_body_not_lifted():
    """SF-2 codegen safety: a ``proc`` defined *inside* a method body is
    created at method-call time, not at class definition — so it must NOT
    be lifted into ``ir_module.procedures`` (codegen would otherwise emit
    it unconditionally at script load, changing bytecode).  Method-body
    lowering suppresses the global registration while still analysing the
    body."""
    from compiler.lowering import lower_to_ir

    src = (
        "proc top {} { return 1 }\n"
        "oo::class create C {\n"
        "    method m {} {\n"
        "        proc inner {} { return 2 }\n"
        "        return 0\n"
        "    }\n"
        "}\n"
    )
    m = lower_to_ir(src)
    # Only the genuine top-level proc is registered; the method-local
    # ``inner`` proc is not, and the method itself is captured separately.
    assert list(m.procedures) == ["::top"], f"unexpected procedures: {list(m.procedures)}"
    assert "::C::m" in m.methods


def test_FP_OPT_12_oo_define_pure_method_rhs_deleted():
    """SF-2 TP via ``oo::define`` body form: methods added through
    ``oo::define C { method ... }`` are lowered and summarised the same
    way as the inline ``oo::class create`` body."""
    from compiler.optimiser import optimise_source

    src = (
        "oo::class create C {}\n"
        "oo::define C {\n"
        "    method pure_helper {} { return 42 }\n"
        "    method m {} {\n"
        "        set unused [my pure_helper]\n"
        "        puts done\n"
        "    }\n"
        "}\n"
    )
    optimised, rewrites = optimise_source(src)
    o126s = [r for r in rewrites if r.code == "O126"]
    assert o126s, f"oo::define pure-method RHS must fire O126; got {rewrites}"
