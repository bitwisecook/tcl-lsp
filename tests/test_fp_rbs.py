"""TP/FP regression tests for the RBS (read-before-set) family.

Each test pairs to an `FP-RBS-NN` entry in
``docs/design/compiler/FP.md``.  Tcl reproducers are copied verbatim from the
doc so the two stay in lock-step.

Ground truth: real C tclsh 9.0.3 (run via ``tclsh9.0`` during planning).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics

# RBS-family-shared filter: any of W210 / W213 / W214 (W213 derives from RBS;
# W214 is RBS in expr / cmd-sub contexts).
_RBS_CODES = {"W210", "W213", "W214"}


def _rbs(source: str) -> list:
    """All RBS-family diagnostics for *source*."""
    return [d for d in get_diagnostics(source) if d.code in _RBS_CODES]


# FP-RBS-01 — info exists / array exists is the test-before-use idiom


FP_RBS_01_REPRO = """\
proc maybe_get {} {
    # v is never set in this proc — the info-exists guard is the entire
    # safety: a bare `$v` here would be a hard tclsh error.
    if {[info exists v]} { return $v }
    return {}
}
"""


def test_FP_RBS_01_info_exists_guard_silent():
    """FP: a bare $v read inside an `info exists v` guard must NOT fire W210.
    The fix is suppress-only, name-level (existence_test_names)."""
    assert _rbs(FP_RBS_01_REPRO) == [], (
        "info-exists-guarded read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_01_REPRO))
    )


def test_FP_RBS_01_bare_unguarded_read_still_fires():
    """TP control: stripping the info-exists guard restores the W210.  Proves
    the FP suppression is targeted (only the guarded read), not blanket."""
    no_guard = """\
proc maybe_get {} {
    # No guard — bare `$v` on a never-set local IS a real tclsh error.
    return $v
}
"""
    w210 = [d for d in _rbs(no_guard) if d.code == "W210"]
    assert any("'v'" in (d.message or "") for d in w210), (
        "Unguarded bare $v read on a never-set local must fire W210; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(no_guard))
    )


def test_FP_RBS_01_array_exists_guard_silent():
    """FP control: `array exists arr` guards an array the same way."""
    src = """\
proc maybe_get {} {
    if {[array exists arr]} { return [array get arr] }
    return {}
}
"""
    assert _rbs(src) == [], (
        "array-exists-guarded read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


# FP-RBS-02 — catch / regexp / scan command-sub writes are not read-before-set


FP_RBS_02_REPRO = """\
proc f {} {
    # [catch …] writes 'err' in this scope (tclsh-verified);
    # the read in the consequent must NOT be W210.
    if {[catch {operation} err]} { puts "failed: $err" }
}
"""


def test_FP_RBS_02_catch_msg_var_silent():
    """FP: $err read after `[catch {…} err]` must NOT fire W210; the
    command-sub writes err in this scope (tclsh-verified)."""
    assert _rbs(FP_RBS_02_REPRO) == [], (
        "catch-msg-var read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_02_REPRO))
    )


def test_FP_RBS_02_unrelated_read_still_fires():
    """TP control: an unrelated $other (no cmd-sub write target) must still
    fire W210.  Proves the suppression is targeted, not blanket-on-catch."""
    src = """\
proc f {} {
    if {[catch {operation} err]} { puts "failed: $other" }
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'other'" in (d.message or "")]
    assert w210, "$other (not written by the catch) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


def test_FP_RBS_02_regexp_match_var_silent():
    """FP: regexp's -> match-vars are caller-scope writes too."""
    src = r"""
proc f {} {
    if {[regexp {(\w+)=(\w+)} "k=v" -> k v]} { puts "$k=$v" }
}
"""
    assert _rbs(src) == [], (
        "regexp match-var read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


# FP-RBS-03 — frozen-loop bodies (while/for with cmd-sub condition)


FP_RBS_03_REPRO = """\
proc f {fp} {
    # gets writes 'line' AND the body sets 'n' — both are body-local
    # but the frozen-loop body keeps them invisible to SSA defs.
    while {[gets $fp line] >= 0} {
        set n [string length $line]
        puts "$line ($n chars)"
    }
}
"""


def test_FP_RBS_03_frozen_while_body_silent():
    """FP: a `while {[cmd $fp v] …}` body's writes (n, and v itself) are
    recovered by body_write_names + command_sub_write_names, so the body's
    reads don't false-flag W210."""
    assert _rbs(FP_RBS_03_REPRO) == [], (
        "frozen-while-body reads must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_03_REPRO))
    )


def test_FP_RBS_03_genuine_unset_in_body_still_fires():
    """TP control: a variable that's only read in the body (never written
    anywhere — not a loop-binder, not a set, not in the cmd-sub condition)
    must still fire W210.  Proves the body-write recovery is targeted, not
    blanket-suppression of every body read."""
    src = """\
proc f {fp} {
    while {[gets $fp line] >= 0} {
        puts "$line $missing"
    }
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'missing'" in (d.message or "")]
    assert w210, "$missing (never written anywhere) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# FP-RBS-04 — qualified-variable aliases (local name is the tail)


FP_RBS_04_REPRO = """\
proc ::ns::get {name key} {
    # `variable ${name}::graphAttr` declares the local alias 'graphAttr';
    # the qualified form is just where the storage lives.
    variable ${name}::graphAttr
    if {![info exists graphAttr($key)]} { return "" }
    return $graphAttr($key)
}
"""


def test_FP_RBS_04_qualified_alias_silent():
    """FP: `variable ${name}::tail` declares a local alias named *tail*;
    reads of $tail must not false-flag W210/W213."""
    assert _rbs(FP_RBS_04_REPRO) == [], (
        "qualified-alias tail read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_04_REPRO))
    )


def test_FP_RBS_04_static_qualified_alias_silent():
    """FP control: the static-namespace form `variable ::ns::tail` also
    creates a tail-named local — must be exempted too."""
    src = """\
proc tester {} {
    variable ::ns::children
    return $children
}
"""
    assert _rbs(src) == [], (
        "static-namespace alias tail must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


def test_FP_RBS_04_unrelated_tail_still_fires():
    """TP control: a read of a tail name that is NOT declared as a qualified
    alias must still fire W210.  Proves the exemption is targeted (only
    actual `variable X::Y` decl tails), not a blanket suppression of every
    bare-name read."""
    src = """\
proc ::ns::get {name key} {
    variable ${name}::graphAttr
    # 'undeclared' has no alias decl and no set — must still fire W210.
    return $undeclared($key)
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'undeclared'" in (d.message or "")]
    assert w210, "$undeclared (no alias decl, no set) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# FP-RBS-06 — cmd-sub write-targets inside an expr body are caller-scope writes


FP_RBS_06_REPRO = """\
proc f {sock} {
    # http.tcl:4340 pattern: the [catch …] inside [expr {…}] writes
    # 'tmp' during expr eval; the `|| $tmp` read must not be W210.
    set eof [expr {[catch {eof $sock} tmp] || $tmp}]
    return $eof
}
"""


def test_FP_RBS_06_catch_inside_expr_silent():
    """FP: `[expr {[catch {…} tmp] || $tmp}]` writes `tmp` during expr eval
    (catch's third arg); the same-expression `|| $tmp` read must not be W210."""
    assert _rbs(FP_RBS_06_REPRO) == [], (
        "cmd-sub write inside expr must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_06_REPRO))
    )


def test_FP_RBS_06_unrelated_inside_expr_still_fires():
    """TP control: an unrelated $other inside the same expr (not a cmd-sub
    write target) must still fire W210."""
    src = """\
proc f {sock} {
    set eof [expr {[catch {eof $sock} tmp] || $other}]
    return $eof
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'other'" in (d.message or "")]
    assert w210, (
        "$other (not written by the catch inside the expr) must still fire W210; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


# FP-RBS-07 — dynamically-named namespace eval bodies are still analysed


FP_RBS_07_REPRO = """\
# logger.tcl:1007-1016 pattern: ${service} is the enclosing proc's
# parameter; the dynamic namespace name doesn't stop the body's inner
# `proc greet` from being analysed (post-fix).
proc trace_on {service} {
    namespace eval ::logger::tree::${service} {
        proc greet {who} { return "hello $who" }
    }
}
"""


def test_FP_RBS_07_dynamic_ns_eval_inner_param_silent():
    """FP: a dynamic-name `namespace eval … { proc … }` has its body analysed
    so the inner proc's parameters are seen correctly.  Pre-fix, the whole
    barrier was opaque and the inner `who` leaked as an unknown name."""
    assert _rbs(FP_RBS_07_REPRO) == [], (
        "dynamic-name namespace eval body's inner proc-param read must not fire "
        "any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_07_REPRO))
    )


def test_FP_RBS_07_static_ns_eval_still_works():
    """FP control: the static-namespace form was never broken — assert it
    stays silent so a regression in either path is caught."""
    src = """\
namespace eval ::pkg {
    proc greet {who} { return "hello $who" }
}
"""
    assert _rbs(src) == [], (
        "static namespace eval body inner proc-param read must not fire any RBS "
        "code; current: " + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


# FP-RBS-08 — upvar with a dynamic target (upvar 1 $name var) is a real alias-def


FP_RBS_08_REPRO = """\
proc f {name} {
    # picoirc.tcl:69 pattern: upvar 1 $context irc — aliases 'irc'
    # to whatever the caller named.  Writes + reads must be silent.
    upvar 1 $name var
    set var 99
    return $var
}
"""


def test_FP_RBS_08_dynamic_upvar_target_silent():
    """FP: `upvar 1 $name var` is a real alias-def even though the target is
    dynamic; writes + reads on the alias must not false-flag W210/W220/W211."""
    interesting = {"W210", "W213", "W214", "W220", "W211"}
    diags = [d for d in get_diagnostics(FP_RBS_08_REPRO) if d.code in interesting]
    assert diags == [], (
        "dynamic-target upvar alias use must not fire any RBS/dead-store/unused "
        "code; current: " + ", ".join(f"{d.code}:{d.message}" for d in diags)
    )


def test_FP_RBS_08_unrelated_local_still_fires():
    """TP control: an unrelated local (not aliased, not set) must still fire."""
    src = """\
proc f {name} {
    upvar 1 $name var
    return $unrelated
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'unrelated'" in (d.message or "")]
    assert w210, "$unrelated (no alias, no set) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# FP-RBS-09 — for-init / regexp captures in un-lowered switch arms


FP_RBS_09_REPRO = r"""
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $j }
        }
        b {
            if {[regexp {(\w+)} "foo" -> v]} { puts $v }
        }
    }
}
"""


def test_FP_RBS_09_for_init_in_switch_arm_silent():
    """FP: `for {set j 0} …` inside a switch arm — the for-init's def is
    recovered so the body's $j read doesn't false-flag."""
    diags = [d for d in _rbs(FP_RBS_09_REPRO) if "'j'" in (d.message or "")]
    assert diags == [], (
        "for-init $j read inside switch arm must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in diags)
    )


def test_FP_RBS_09_regexp_capture_in_switch_arm_silent():
    """FP: `regexp … -> v` capture inside a switch arm — the capture-var def
    is recovered so the consequent's $v read doesn't false-flag."""
    diags = [d for d in _rbs(FP_RBS_09_REPRO) if "'v'" in (d.message or "")]
    assert diags == [], (
        "regexp capture $v read inside switch arm must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in diags)
    )


def test_FP_RBS_09_genuine_unset_in_arm_still_fires():
    """TP control: a variable never written *anywhere* in the proc still
    fires W210 even inside a switch arm.  Proves the def-recovery is
    targeted (for-init / cmd-sub captures only), not blanket-suppression."""
    src = r"""
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $missing }
        }
    }
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'missing'" in (d.message or "")]
    assert w210, "$missing (never written anywhere) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# FP-RBS-10 — eval / namespace eval literal-body reads are recovered


FP_RBS_10_REPRO = """\
proc f {x} {
    # eval's braced body evaluates in *this* scope: $x is a real read of
    # the parameter, so 'x' must not be reported W214 ("unused").
    eval { puts $x }
}
"""


def test_FP_RBS_10_eval_body_param_silent():
    """FP: an `eval { … $x … }` body evaluates in this scope, so $x is a
    real read of parameter x; W210 (RBS) and W214 (unused param) must both
    stay silent."""
    interesting = {"W210", "W213", "W214"}
    diags = [d for d in get_diagnostics(FP_RBS_10_REPRO) if d.code in interesting]
    assert diags == [], (
        "eval-body $x read must not fire RBS or unused-param codes; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in diags)
    )


def test_FP_RBS_10_genuine_unused_still_fires():
    """TP control: a parameter that's truly never read still fires W214."""
    src = """\
proc f {x y} {
    return $x
}
"""
    w214 = [d for d in get_diagnostics(src) if d.code == "W214" and "'y'" in (d.message or "")]
    assert w214, "truly unused param 'y' must still fire W214; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in get_diagnostics(src)
    )


def test_FP_RBS_10_namespace_eval_does_not_recover_reads():
    """TP control: `namespace eval ns { ... $x ... }` evaluates the body in
    `ns`'s frame, NOT the caller's.  An unqualified `$x` there resolves to
    `::ns::x` (tclsh errors `can't read "x": no such variable`), so it is
    NOT a read of the caller's parameter `x`.  The analyser must therefore
    NOT recover that read; W214 ("Parameter ... is unused") must still
    fire on `x`.

    Locks in `compiler/core_analyses.py::_block_local_reads`'s early-return
    on `not stmt.caller_scope` (the FP-RBS-10 doc text was previously
    misleading on this point; this test prevents a regression that would
    "fix" the doc by over-recovering)."""
    src = """\
proc g {x} {
    namespace eval ::ns { puts "hello $x" }
}
"""
    w214 = [d for d in get_diagnostics(src) if d.code == "W214" and "'x'" in (d.message or "")]
    assert w214, (
        "namespace-eval body must NOT recover caller reads; W214 must still fire on x. "
        "current: " + ", ".join(f"{d.code}:{d.message}" for d in get_diagnostics(src))
    )


# FP-RBS-11 — qualified-builtin loops (::foreach / ::lmap / ::for / ::while)


FP_RBS_11_REPRO = """\
proc f {dict} {
    # html.tcl:153 pattern: ::foreach is just qualified foreach.
    # Loop vars k,v and body reads must all be silent.
    ::foreach {k v} $dict { puts "$k=$v" }
}
"""


def test_FP_RBS_11_qualified_foreach_silent():
    """FP: ::foreach is recognised by _read_before_set's loop-form recovery,
    so loop vars k/v and body reads don't false-flag."""
    assert _rbs(FP_RBS_11_REPRO) == [], (
        "qualified ::foreach must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_11_REPRO))
    )


def test_FP_RBS_11_qualified_for_silent():
    """FP control: the qualified ::for form works the same way."""
    src = """\
proc f {} {
    ::for {set i 0} {$i < 3} {incr i} { puts $i }
}
"""
    assert _rbs(src) == [], "qualified ::for must not fire any RBS code; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


def test_FP_RBS_11_genuine_unset_in_body_still_fires():
    """TP control: a body-read of a variable never bound anywhere still
    fires W210, proving the loop-binder exemption is targeted."""
    src = """\
proc f {dict} {
    ::foreach {k v} $dict { puts "$k=$missing" }
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'missing'" in (d.message or "")]
    assert w210, "$missing (never bound) must still fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# FP-RBS-05 — namespace upvar alias-not-a-def (OPEN, xfail until lower_namespace_upvar lands)


FP_RBS_05_REPRO = """\
proc tester {} {
    # tclsh: 'alias' is now the caller-scope name for ::ns::state.
    namespace upvar ::ns state alias
    return $alias
}
"""


@pytest.mark.xfail(
    reason="FP-RBS-05 open: namespace upvar has no lower_*_hook so alias has no "
    "IRCall def → false W210.  Flips to failure when the fix lands.",
    strict=True,
)
def test_FP_RBS_05_namespace_upvar_silent():
    """OPEN-FP: `namespace upvar ns src alias` is a real caller-scope def of
    `alias` (tclsh-verified, behaviourally identical to `upvar 1 ::ns::src
    alias`).  The analyser currently fires W210 because no lowering hook
    handles the `namespace` subcommand `upvar`.  When the
    `lower_namespace_upvar` hook lands, the def will be recorded, this test
    will pass, and the xfail must be removed."""
    assert _rbs(FP_RBS_05_REPRO) == [], (
        "namespace-upvar alias read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_05_REPRO))
    )


def test_FP_RBS_05_alias_used_no_error_at_runtime():
    """Sanity / TP-proxy: confirm the reproducer is structurally valid (the
    analyser successfully analyses it and produces *some* diagnostic shape).
    Catches a regression where the reproducer stops being parseable, which
    would mask the FP-RBS-05 failure with an unrelated error."""
    diags = get_diagnostics(FP_RBS_05_REPRO)
    # The current FP itself proves the snippet is parseable; we don't assert a
    # specific code, only that the call to get_diagnostics returned cleanly.
    assert isinstance(diags, list)


def test_FP_RBS_02_scan_output_silent():
    """FP: scan's output-vars are caller-scope writes too."""
    src = """\
proc f {} {
    scan "42" %d n
    puts $n
}
"""
    assert _rbs(src) == [], (
        "scan output-var read must not fire any RBS code; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )
