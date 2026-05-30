"""TP/FP regression tests for the TNT (taint) family.

Each test pairs to an ``FP-TNT-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

TNT family covers T100 (taint flows into expr) and T101 (taint flows into
puts).  Each entry locks in a *position-aware* filter: the analyser must
distinguish tainted values that flow into the dangerous *argument slot*
of a sink (the expr operand, the puts content) from values that flow
into a structural slot (the puts channel id, an expr-internal cmd-sub
argument) where re-parsing does NOT happen.

Ground truth: real C tclsh 9.0.3.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-TNT-01 — T100 direct-operand expr filter


FP_TNT_01_REPRO = """\
proc f {} {
    set data [gets stdin]
    # $data is consumed as an argument to `string length`, not re-parsed
    # as expr text. The cmd-sub boundary protects it -- no T100.
    expr {[string length $data] / 8}
}
"""


def test_FP_TNT_01_cmd_sub_arg_position_no_t100():
    """FP: tainted value inside a cmd-sub argument (``[string length $data]``)
    is NOT a direct expr operand and must not fire T100.

    Pre-fix T100 fired on every tainted ``uses`` entry regardless of
    position in the parsed expr AST.  The fix introduces
    ``_direct_expr_operand_names`` which walks the ExprNode tree and
    collects ExprVar names OUTSIDE any ExprCommand subtree.  Sample
    cleared firings: tcllib blowfish.tcl:525, http.tcl:4338, mime.tcl:1962."""
    assert _codes(FP_TNT_01_REPRO, "T100") == []


def test_FP_TNT_01_direct_operand_still_fires():
    """TP control: when the tainted value IS a direct expr operand
    (``expr {$data + 1}``), T100 fires.

    NOTE: this is NOT a code-execution vector for the braced form --
    Tcl's braced ``expr`` does NOT re-parse a direct-operand value as
    expression text (tclsh-verified: ``set data {[puts X]}; expr
    {$data + 1}`` errors with "cannot use ... as left operand", does
    NOT execute the ``[puts]``).  T100 here is the
    numeric/type-coercion hazard: ``$data = "inf"`` yields ``inf``;
    ``$data = "0xff"`` is parsed as 255 even if the writer expected
    base-10.  See FP-TNT-01 in FP.md.  Code-execution injection in
    expr is the UNBRACED form ``expr $data + 1`` (covered by W101,
    FP-INJ-05)."""
    src = """\
proc f {} {
    set data [gets stdin]
    # $data is a DIRECT expr operand -- Tcl's numeric coercion path.
    expr {$data + 1}
}
"""
    assert _codes(src, "T100"), f"direct operand must fire T100; got {get_diagnostics(src)}"


def test_FP_TNT_01_function_arg_direct_operand_still_fires():
    """TP control: ``expr {abs($data)}`` -- $data is a direct argument of
    the expr-level function `abs` (still numeric-coercion sink),
    NOT inside a Tcl cmd-sub.  Still fires T100."""
    src = """\
proc f {} {
    set data [gets stdin]
    expr {abs($data)}
}
"""
    assert _codes(src, "T100"), (
        f"abs($data) direct operand must fire T100; got {get_diagnostics(src)}"
    )


# FP-TNT-02 — T101 puts channel-position filter


FP_TNT_02_REPRO = """\
proc f {} {
    set chan [gets stdin]
    # $chan is the destination channel id, NOT the content; T101 must
    # not fire on a positional channel-id arg.  Only the trailing
    # content arg can carry injectable content.
    puts -nonewline $chan "hello\\n"
}
"""


def test_FP_TNT_02_channel_id_position_no_t101():
    """FP: ``puts ?-nonewline? ?channelId? string`` -- T101 must filter
    its emission to the content (trailing positional) arg.  Tainted
    channel ids are not output-injection (they're file handles).

    Pre-fix T101 fired on the channel-id arg too.  Sample cleared
    firing: tcllib imap4.tcl ``puts -nonewline $chan "$t\\r\\n"``."""
    assert _codes(FP_TNT_02_REPRO, "T101") == []


def test_FP_TNT_02_content_arg_still_fires():
    """TP control: a tainted *content* arg to ``puts`` must still fire
    T101 — that's the genuine output-injection vector (terminal escape
    sequences, log poisoning, etc.)."""
    src = """\
proc f {} {
    set data [gets stdin]
    puts $data
}
"""
    assert _codes(src, "T101"), f"tainted puts content must fire T101; got {get_diagnostics(src)}"


def test_FP_TNT_02_content_with_channel_still_fires():
    """TP control: ``puts $chan $data`` -- $chan is filtered (channel id),
    $data is the content and must still fire T101."""
    src = """\
proc f {} {
    set chan stdout
    set data [gets stdin]
    puts $chan $data
}
"""
    assert _codes(src, "T101"), (
        f"tainted puts content arg must still fire T101 even with explicit channel; got {get_diagnostics(src)}"
    )
