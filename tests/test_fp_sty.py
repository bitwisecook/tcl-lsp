"""TP/FP regression tests for the STY (style / usage) family.

Each test pairs to an ``FP-STY-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

STY family covers style/usage warnings that fired on idiomatic Tcl
patterns and have FP suppressions in place:

* W001 — unknown subcommand (Tk geometry-manager shortcut form)
* W104 — string-concat list building (usage/template notation exempt)
* W120 — command without ``package require`` (file is the provider)
* W122/W124 — IPv4-shaped literal (OID-like dotted chains exempt)
* W126 — non-channel value (lattice fix for ``lassign`` destructure)
* W214 — empty-body proc stubs + snit quoted-keyword markers
* W302 — catch without result var (fire-and-forget idiom, bare +
  subcommand-aware)
* W306 — literal-expected substitution (escaped ``\\[``/``\\$`` exempt)

Ground truth: real C tclsh 9.0.3 (and Tk 8.6 for the geometry codes).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-STY-01 — W001 Tk geometry-manager shortcut form (grid/pack/place pathName)


def test_FP_STY_01_grid_pathname_no_w001():
    """FP: ``grid .x`` (no subcommand, just a pathName) is the Tk
    geometry-manager *shortcut* form — equivalent to ``grid configure .x``.
    Tk's grid(n), pack(n), place(n) all accept a window name as the first
    arg as a shorthand."""
    assert _codes("grid .x", "W001") == []


def test_FP_STY_01_pack_pathname_no_w001():
    """FP: ``pack .x`` shortcut form."""
    assert _codes("pack .x", "W001") == []


def test_FP_STY_01_place_pathname_no_w001():
    """FP: ``place .x`` shortcut form."""
    assert _codes("place .x", "W001") == []


def test_FP_STY_01_genuine_unknown_subcommand_still_fires():
    """TP control: a real unknown subcommand still fires W001."""
    assert _codes("grid bogus .x", "W001"), "real unknown subcommand must still fire W001"


# FP-STY-02 — W306 escaped ``\\[`` / ``\\$`` in quoted regexp patterns


def test_FP_STY_02_escaped_bracket_no_w306():
    """FP: ``regexp "\\[abc\\]" $s`` -- the backslashes escape ``[``/``]``
    so they are *literal* regex characters (a char-class bracket), NOT
    Tcl command substitution.  Pre-fix W306 fired because the resolved
    arg text couldn't tell escaped from unescaped; the fix scans the
    raw source slice for a *live* (unescaped) ``[``/``$``."""
    src = r'regexp "\[abc\]" $s'
    assert _codes(src, "W306") == []


def test_FP_STY_02_escaped_dollar_no_w306():
    """FP: ``regexp "\\$end" $s`` -- backslash-dollar is a literal ``$``
    in the regex (an end-anchor), not a Tcl substitution."""
    src = r'regexp "\$end" $s'
    assert _codes(src, "W306") == []


def test_FP_STY_02_live_dollar_in_quoted_pattern_still_fires():
    """TP control: an UNescaped ``$pat`` in a quoted regex pattern IS a
    live substitution and must still fire W306."""
    src = 'regexp "$pat" $s'
    assert _codes(src, "W306"), "live $pat in quoted regexp pattern must fire W306"


def test_FP_STY_02_live_cmdsub_in_quoted_pattern_still_fires():
    """TP control: a live ``[clock seconds]`` in a quoted regex pattern
    must still fire W306."""
    src = 'regexp "[clock seconds]" $s'
    assert _codes(src, "W306"), "live cmd-sub in quoted regexp pattern must fire W306"


# FP-STY-03 — W104 usage / template notation (``?arg?``, ``<placeholder>``, ``...``)


def test_FP_STY_03_optarg_question_marks_no_w104():
    """FP: ``?optarg?`` is the documented Tcl convention for optional
    parameters in usage/help strings, not a list element — must not fire
    W104 (string-concat list-building)."""
    src = 'append usage "?arg?"'
    assert _codes(src, "W104") == []


def test_FP_STY_03_placeholder_angle_no_w104():
    """FP: ``<placeholder>`` template notation."""
    src = 'append usage "<placeholder>"'
    assert _codes(src, "W104") == []


def test_FP_STY_03_ellipsis_no_w104():
    """FP: ``...`` continuation/varargs notation."""
    src = 'append usage "..."'
    assert _codes(src, "W104") == []


def test_FP_STY_03_genuine_list_building_still_fires():
    """TP control: a real string-concat list-building pattern still fires."""
    src = """\
proc f {items} {
    foreach i $items {
        append result " $i"
    }
}
"""
    assert _codes(src, "W104"), 'genuine ``append result " $i"`` must still fire W104'


# FP-STY-04 — W126 non-channel value: lattice fix for lassign destructure


def test_FP_STY_04_lassign_destructure_channels_no_w126():
    """FP: ``lassign [chan pipe] ch wch`` destructures the LIST result of
    ``chan pipe`` into per-element channel-typed locals.  Pre-fix the
    analyser typed the lassign def-targets as LIST (the source type),
    causing W126 to fire when those locals were later used as channels.
    Fix: lassign def-targets are typed UNKNOWN (sound conservative); the
    list type only applies to a captured-rest binding."""
    src = "lassign [chan pipe] ch wch\nputs $ch x"
    assert _codes(src, "W126") == []


# FP-STY-05 — W302 catch fire-and-forget (bare + subcommand-aware)


def test_FP_STY_05_bare_close_fire_and_forget_no_w302():
    """FP: ``catch {close $fh}`` is the documented Tcl idiom for
    "close if open, otherwise ignore" — fire-and-forget cleanup."""
    assert _codes("catch {close $fh}", "W302") == []


def test_FP_STY_05_ensemble_close_fire_and_forget_no_w302():
    """FP: ``catch {chan close $fh}`` is the same idiom in ensemble form."""
    assert _codes("catch {chan close $fh}", "W302") == []


def test_FP_STY_05_constructive_subcommand_still_fires():
    """TP control: ``catch {chan configure}`` is constructive (sets an
    option, expects success), not fire-and-forget; W302 must still
    fire (need a result var to know if it worked)."""
    src = "catch {chan configure $fh -opt val}"
    assert _codes(src, "W302"), "constructive subcommand must still fire W302"


def test_FP_STY_05_user_call_still_fires():
    """TP control: ``catch {my_proc $arg}`` is a user call — generally
    needs a result var; W302 fires."""
    src = "catch {my_proc $arg}"
    assert _codes(src, "W302"), "user-call catch must still fire W302"


# FP-STY-06 — W122/W124 OID-like dotted chains (not IPv4)


def test_FP_STY_06_oid_chain_no_w122_w124():
    """FP: ``1.3.6.1.4.1.4203.1.11.3`` (LDAP PEN OID) is an enterprise
    OID, NOT an IPv4 address.  The naive regex matched an embedded
    4-component slice (``4203.1.11.3``) where octet 4203 > 255; W122/W124
    fired falsely.  Fix: skip when the matched quad is preceded or
    followed by ``.<digit>`` (part of a longer dotted chain)."""
    src = "set oid 1.3.6.1.4.1.4203.1.11.3"
    assert _codes(src, "W122") == []
    assert _codes(src, "W124") == []


def test_FP_STY_06_real_ipv4_shaped_still_fires():
    """TP control: a real IPv4-shaped literal with an out-of-range octet
    still fires W124 (4-component dotted chain, no extension)."""
    src = "set ip 192.168.4203.1"
    assert _codes(src, "W124"), "real IPv4-shaped out-of-range must still fire W124"


# FP-STY-07 — W120 package self-call (file is the provider)


def test_FP_STY_07_provider_self_call_no_w120():
    """FP: a file declaring ``package provide msgcat 1.0`` IS the
    implementation of `msgcat`; subsequent ``msgcat::mc`` calls inside
    that file don't need a ``package require msgcat``.  Fix: union the
    file's ``package_provides`` set into the imported-set check."""
    src = "package provide msgcat 1.0\nmsgcat::mc hello"
    assert _codes(src, "W120") == []


def test_FP_STY_07_no_provide_still_fires():
    """TP control: using a package's command without either a require or
    a provide must still fire W120."""
    src = "msgcat::mc hello"
    assert _codes(src, "W120"), "missing package require must still fire W120"


# FP-STY-08 — W214 empty-body proc stubs


def test_FP_STY_08_empty_body_stub_no_w214():
    """FP: ``proc stub {a b} {}`` is the canonical Tcl signature-stub
    pattern (e.g. tcllib grammar_fa/faop.tcl declares 14 such empty
    procs as the FA algebra API; overlay files plug in the real bodies
    later).  Every parameter is necessarily "unused" because there is
    no body to use them — flagging W214 on every param is noise."""
    src = "proc stub {a b} {}"
    assert _codes(src, "W214") == []


def test_FP_STY_08_non_empty_body_unused_param_still_fires():
    """TP control: a proc with a non-empty body but a truly unused
    parameter still fires W214."""
    src = "proc f {a b} { return $a }"
    w214 = [d for d in _codes(src, "W214") if "'b'" in (d.message or "")]
    assert w214, "non-empty body with unused 'b' must still fire W214"


def test_FP_STY_08_snit_quoted_keyword_marker_no_w214():
    """FP: snit-style DSL uses ``{"as" ""}`` as a positional keyword
    marker -- the parameter name is the literal ``"as"`` and the body
    can't reference it as a variable.  Detect via param name starting
    + ending with ``"`` and exempt from W214.

    Companion fix to the empty-body-stub exemption above; both are
    captured in FP-STY-08 in the catalog."""
    src = 'proc xyz {"as" v} { return $v }'
    assert _codes(src, "W214") == [], (
        f'quoted-keyword marker `"as"` must NOT fire W214; got {get_diagnostics(src)}'
    )


def test_FP_STY_08_bare_keyword_param_still_fires():
    """TP control: the same shape with NO quote marker (``proc xyz {as
    v}``) MUST still fire W214 on the unused ``as`` -- proves the
    exemption is keyed strictly on the quote-marker shape."""
    src = "proc xyz {as v} { return $v }"
    w214 = [d for d in _codes(src, "W214") if "'as'" in (d.message or "")]
    assert w214, "bare unused 'as' param (no quote marker) must still fire W214"


# FP-STY-09 — W214 dispatcher needs arity-compatible peer (D3-P9/D4-F4)


FP_STY_09_REPRO = """\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
    proc unrelated {cmd} { $cmd x }
}
"""


def test_FP_STY_09_arity_incompatible_dispatcher_does_not_suppress():
    """TP: a 1-arg ``$cmd x`` dispatcher in the same namespace as 2-arg
    peers a/b/c is NOT evidence for the peer protocol -- the arities
    don't match.  W214 must still fire 3 times on ``token``.

    Locks in the D4-F4 closure: ``var_command_sites`` records each
    dispatch's positional arg count, and the protocol-match heuristic
    filters out arity-incompatible dispatchers before the
    "≥1 compatible dispatcher" check (analyser/checks/_param.py)."""
    diags = [
        d
        for d in get_diagnostics(FP_STY_09_REPRO)
        if d.code == "W214" and "'token'" in (d.message or "")
    ]
    assert len(diags) == 3, (
        "W214 must fire on 'token' in all three peers (a/b/c); the 1-arg "
        f"unrelated dispatcher is arity-incompatible.  Got {len(diags)}: {diags}"
    )


def test_FP_STY_09_arity_compatible_dispatcher_suppresses():
    """TN control: when a same-namespace dispatcher's arity matches the
    peer signature (2-arg ``$cmd $ctx $token``), the protocol-match
    heuristic correctly applies and W214 stays silent."""
    src = """\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
    proc dispatch {cmd ctx token} { $cmd $ctx $token }
}
"""
    diags = [d for d in get_diagnostics(src) if d.code == "W214" and "'token'" in (d.message or "")]
    assert not diags, f"2-arg dispatcher must suppress 2-arg peer family W214; got {diags}"


# FP-STY-10 — scan_provably_no_match soundness (%n / Inf / format whitespace, D4-F1)


FP_STY_10_REPRO = """proc f {} { scan {} %n n; puts $n }\n"""


def test_FP_STY_10_scan_percent_n_on_empty_input_silent():
    """FP: ``scan "" %n n`` ALWAYS succeeds (writes the count of
    consumed characters with no input required); reading ``$n`` after
    must NOT fire W210.  Locks the D4-F1 closure: %n maps to the new
    ``always``-match directive kind in compiler/scan_format.py."""
    assert _codes(FP_STY_10_REPRO, "W210") == []


def test_FP_STY_10_scan_float_accepts_inf_silent():
    """FP: ``scan Inf %f f`` succeeds in tclsh (Tcl_GetDouble accepts
    ``Inf``/``Infinity``/``NaN``).  Closure extends the float-kind
    no-match predicate to accept those tokens."""
    src = "proc f {} { scan Inf %f f; puts $f }\n"
    assert _codes(src, "W210") == []


def test_FP_STY_10_scan_format_whitespace_cr_silent():
    """FP: format ``\\r`` is whitespace that skips input whitespace;
    ``scan " 123" "\\r%d" n`` parses 123 successfully.  Closure
    extends the format-whitespace set to include ``\\r\\f\\v``."""
    src = 'proc f {} { scan { 123} "\\r%d" n; puts $n }\n'
    assert _codes(src, "W210") == []


def test_FP_STY_10_scan_genuine_no_match_still_fires():
    """TP control: a real provable no-match (``scan abc %d n``) must
    still fire W210 -- confirms the soundness sweep didn't disable
    the predicate entirely."""
    src = "proc f {} { scan abc %d n; puts $n }\n"
    assert _codes(src, "W210"), "real provable no-match (abc vs %d) must still fire W210"


# FP-STY-11 — variadic var-write resolver for scan / lassign / binary scan (D4-F2)


FP_STY_11_REPRO = """\
proc f {} {
    scan {x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15 x16 x17 x18 x19} {%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} v0 v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v15 v16 v17 v18 v19
    return $v19
}
"""


def test_FP_STY_11_scan_20_vars_no_false_w210():
    """FP: pre-fix the scan spec hard-coded VAR_WRITE for slots [2..19]
    only; the 20th var (v19) wasn't recognised and the subsequent
    ``return $v19`` falsely fired W210.  D4-F2 closure adds the dynamic
    ``arg_role_resolver`` callback (dialects/tcl/scan.py:46) which
    walks the actual args list, slot-budget-free."""
    assert _codes(FP_STY_11_REPRO, "W210") == [], (
        "scan with 20 vars must not false-fire W210 on v18/v19"
    )


def test_FP_STY_11_lassign_many_vars_no_false_w210():
    """FP: same fix for ``lassign`` (dialects/tcl/lassign.py:47).  All
    positional args after the list arg classified VAR_WRITE."""
    src = (
        "proc f {l} { lassign $l a b c d e f g h i j k l2 m n o p q r s t u v w x y; return $y }\n"
    )
    assert _codes(src, "W210") == []


def test_FP_STY_11_binary_scan_many_vars_no_false_w210():
    """FP: same fix for ``binary scan`` (dialects/tcl/binary.py:105)."""
    src = "proc f {} { binary scan {} {i i i i i i i i i i i i i i i i i i i i} a b c d e f g h i j k l m n o p q r s t; return $t }\n"
    assert _codes(src, "W210") == []


def test_FP_STY_11_scan_fewer_specifiers_than_vars_still_fires():
    """TP control: when scan has fewer ``%``-directives than var slots
    (only %s in the format but two var-args), the second var IS
    provably unset and the read must fire W210.  Confirms the new
    resolver doesn't blanket-mark every trailing arg as defined."""
    src = "proc f {} { scan abc %s v1; return $v2 }\n"
    assert _codes(src, "W210"), (
        "scan with one %s but a ``return $v2`` must fire W210 on v2 (never written)"
    )
