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
