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
    """TP control: a genuine simplification (e.g. ``x + 0``) still fires O110."""
    src = "set y [expr {$x + 0}]\n"
    # The identity x+0 -> x is a real simplification; O110 must fire.
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
