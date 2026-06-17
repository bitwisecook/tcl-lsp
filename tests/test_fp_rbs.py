"""TP/FP regression tests for the RBS (read-before-set) family.

Each test pairs to an `FP-RBS-NN` entry in
``docs/design/compiler/FP.md``.  Tcl reproducers are copied verbatim from the
doc so the two stay in lock-step.

Ground truth: real C tclsh 9.0.3 (run via ``tclsh9.0`` during planning).
"""

from __future__ import annotations

import sys
from pathlib import Path

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


# OPEN precision gap (xfail): regexp/scan only write on success.  The
# current name-level recovery in command_sub_write_names exempts the
# variable everywhere in the proc, so even reads on the no-match path
# (where the variable is truly unset) don't fire RBS.  This is sound
# (over-approximation) but loses a real TP class -- documented as an
# open finding in FP-RBS-02.  When a branch-aware refinement lands,
# the xfail flips and prompts its own removal.


def test_FP_RBS_02_regexp_provably_no_match_fires_w210():
    """TP / SCCP-driven match analysis: ``regexp {x} y -> v`` -- the
    pattern ``x`` provably does NOT match the literal input ``y``
    (Python's re.search returns None).  The variable ``v`` is unset
    on the only feasible execution path, so reading ``$v`` is a real
    W210.

    Locked the precision gap closure: ``compiler/core_analyses.py``
    now post-processes ``_read_before_set`` results, scanning for
    regexp/scan calls whose pattern + input are both bare literals,
    proving no-match via Python's re (regexp) or a conservative
    %d-only simulator (scan), and marking output vars as provably
    unset.  Trust-the-match Tcl idioms (dynamic patterns, success
    branches) stay silent because the no-match proof requires
    statically-known + statically-non-matching args."""
    src = """\
proc f {} {
    regexp {x} y -> v
    puts $v
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert w210, "regexp no-match path read of $v should fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


def test_FP_RBS_02_regexp_with_options_does_not_fire():
    """FP guard: a ``regexp`` with switches like ``-nocase``,
    ``-lineanchor``, ``-expanded`` has different semantics than the
    no-option form; the no-match proof must not fire.  The literal-
    only restriction (pattern contains a regex metacharacter) handles
    most cases automatically, but options like ``-nocase`` could turn
    a "no" into a "yes" for case-shifted text -- our analyser must
    not fire.  PR #498/#499 follow-up finding 1."""
    for src in [
        "proc f {} { regexp -nocase {(x)} X -> v\n puts $v }",
        "proc f {} { regexp -lineanchor {^(x)} {a\nx} -> v\n puts $v }",
        "proc f {} { regexp -expanded {(x) # comment} x -> v\n puts $v }",
    ]:
        diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
        assert not diags, f"regexp with options must not fire W210: {src!r} → {diags}"


def test_FP_RBS_02_regexp_no_match_reaches_dominated_block_fires():
    """TP: regexp no-match followed by a use in a DOMINATED block
    (not just the same block) fires W210 -- the post-pass now does
    a CFG dominance check.  PR #498/#499 follow-up finding 2."""
    src = "proc f {} { regexp {x} y -> v\n if {1} { puts $v } }"
    diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert diags, f"reached-by-dominator W210 must fire, got {get_diagnostics(src)}"


def test_FP_RBS_02_regexp_in_negated_condition_fires():
    """TP: regexp embedded in ``if {![regexp ...]} { ... }`` -- the
    body executes ONLY on no-match, so the var is unset there.  The
    F2 closure walks the branch's condition expression for embedded
    regexp/scan calls and tracks which target corresponds to the
    no-match outcome."""
    src = "proc f {} { if {![regexp {x} y -> v]} { puts $v } }"
    diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert diags, f"no-match branch use must fire W210; got {get_diagnostics(src)}"


def test_FP_RBS_02_regexp_in_positive_condition_success_branch_silent():
    """FP / control: ``if {[regexp ...]} { puts $v }`` -- the body
    executes ONLY on match, so v IS set.  Even when the pattern is
    statically provable no-match (so the body is "dead code" at
    runtime), W210 must not fire here -- the branch-aware tracker
    correctly attributes the unset state to the false-target only."""
    src = "proc f {} { if {[regexp {x} y -> v]} { puts $v } }"
    diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert not diags, f"success-branch use must NOT fire W210; got {diags}"


def test_FP_RBS_02_regexp_nocase_silent_on_literal_match():
    """FP / control: ``regexp -nocase`` makes the literal-match path
    case-insensitive, so ``regexp -nocase {x} X v`` MATCHES and sets
    ``v``.  The no-match estimator now case-folds both sides when
    ``-nocase`` is in the switches, so this must stay silent.  (F5 of
    the PR #498 special-casing review.)"""
    src = "proc f {} { regexp -nocase {x} X v; puts $v }"
    diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert not diags, f"-nocase literal match must NOT fire W210; got {diags}"


def test_FP_RBS_02_regexp_unknown_switch_bails():
    """FP / safety: the estimator must bail (return cannot-prove) on
    any unrecognised ``regexp`` switch -- a future Tcl release could
    add a switch that weakens the literal-match assumption.  Tested
    with a synthetic ``-bogus`` and the real ``-about`` switch (which
    returns metadata, not a match result)."""
    for opt in ("-bogus", "-about"):
        src = f"proc f {{}} {{ regexp {opt} {{x}} X v; puts $v }}"
        diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
        assert not diags, f"{opt} must conservatively NOT fire W210; got {diags}"


def test_FP_RBS_02_regexp_safe_switch_still_fires():
    """TP / control: switches that don't change match-vs-no-match for a
    literal pattern (``-line``/``-lineanchor``/``-linestop``/
    ``-expanded``/``-indices``/``-inline``/``-all``/``-start``/``--``)
    must NOT inhibit the no-match verdict.  Pattern ``x`` vs input ``X``
    is a real no-match under every one of those, so W210 must fire."""
    for opt in ("-line", "-lineanchor", "-linestop", "-expanded", "-indices", "-inline", "-all"):
        src = f"proc f {{}} {{ regexp {opt} {{x}} X v; puts $v }}"
        diags = [d for d in get_diagnostics(src) if d.code == "W210" and "'v'" in (d.message or "")]
        assert diags, f"{opt} must still allow W210 to fire; got {get_diagnostics(src)}"


def test_FP_RBS_02_scan_provably_no_match_fires_w210():
    """TP / SCCP-driven match analysis: ``scan abc %d n`` -- the
    format ``%d`` requires the input to start with a digit (or sign),
    but ``abc`` starts with ``a``.  The variable ``n`` is unset on
    the only feasible execution path; reading ``$n`` is a real W210.

    Locked by the same machinery as the regexp case.  The scan
    simulator is conservative (currently handles ``%d`` only;
    other format specifiers fall back to silent)."""
    src = """\
proc f {} {
    scan abc %d n
    puts $n
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'n'" in (d.message or "")]
    assert w210, "scan no-match path read of $n should fire W210; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# OPEN precision gap: ``dict with`` / ``dict update`` whole-proc
# suppression (Finding 6 of PR #498 deep review).  These commands
# legitimately unpack the dict's keys as locals, but the current
# suppression silences ALL version-0 unknown-variable reads in the
# proc -- even ones the dict isn't responsible for.


def test_FP_dict_with_does_not_suppress_unrelated_missing_var():
    """TP / dict-with scoping guard: when the dict literal is statically
    known (``set d {}`` -> empty dict, no keys unpacked), ``dict with d
    {}`` brings NO names into scope.  An unrelated ``$missing`` read
    in the same proc is still a real W210.

    Locked the precision gap closure: the dict-with suppression in
    ``compiler/core_analyses.py::_read_before_set`` now walks backwards
    to the most recent ``IRAssignConst(d)`` in the same block and uses
    THAT literal value to compute the suppression set.  When SCCP can
    determine the dict has no keys, no suppression is applied."""
    src = """\
proc f {} {
    set d {}
    dict with d {}
    puts $missing
}
"""
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'missing'" in (d.message or "")]
    assert w210, "$missing read after dict-with on empty dict must fire W210; got " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )


# Finding 8: W214 dispatch-protocol from peer count alone (no
# dispatcher evidence).  Three ordinary helpers sharing a parameter
# list shouldn't infer a contract without actual dispatcher evidence.


def test_FP_dispatch_protocol_requires_dispatcher_evidence():
    """TP / dispatcher-evidence guard: three ordinary helpers sharing
    ``{ctx token}`` with no variable-command dispatch anywhere in the
    program is NOT a dispatch protocol -- ``token`` may be a real
    unused param and W214 must fire.

    Locked the precision gap closure: ``_dispatch_protocol_signatures``
    in ``analyser/_analyser/_diag_var_lifecycle.py`` now requires BOTH
    a ≥3-peer count AND at least one variable-command dispatch site
    in the program (the "dispatcher evidence").  Real tcllib protocol
    families have dispatch sites; ordinary helper peers don't."""
    src = """\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
}
"""
    w214 = [d for d in get_diagnostics(src) if d.code == "W214" and "'token'" in (d.message or "")]
    assert w214, "unused 'token' in 3 peer helpers (no dispatcher evidence) should still fire W214"


# Finding 10: call-by-name VAR_READ / VAR_WRITE conflate callee-local
# dynamic-name use with caller-frame upvar aliasing.  ``info exists
# $target`` and ``scan ... $name`` use the value of ``target`` /
# ``name`` as a CALLEE-LOCAL variable name -- they do NOT consume the
# caller's variable.  But the analyser's call-by-name suppression
# treats VAR_READ as caller-frame consumption.


def test_FP_call_by_name_info_exists_dynamic_target_not_caller_read():
    """TP / trait-split lock-in: ``proc maybe {target} { info exists
    \\$target }`` uses target's value as a CALLEE-local dynamic var
    name (trait DYNAMIC_NAME_LOCAL), NOT a caller-frame upvar alias
    (trait VAR_READ/VAR_WRITE).  The caller's ``set x 1; maybe x``
    does NOT consume the caller's ``x``; W211/W220/O126 must fire
    on ``x`` (truly set-but-never-used).

    Locked the precision gap closure: ``shared/proc_traits.py`` now
    has a ``DYNAMIC_NAME_LOCAL`` trait distinct from VAR_READ /
    VAR_WRITE, and the call-by-name caller-side suppression in
    ``compiler/proc_arg_traits.py`` only honours VAR_READ /
    VAR_WRITE (genuine upvar aliasing)."""
    src = """\
proc maybe {target} {
    info exists $target
}
proc caller {} {
    set x 1
    maybe x
}
"""
    suppressed_codes = [
        d
        for d in get_diagnostics(src)
        if d.code in ("W211", "W220", "O126", "O109") and "'x'" in (d.message or "")
    ]
    assert suppressed_codes, (
        "caller's $x set-but-never-used must fire (callee uses target as "
        "callee-LOCAL dynamic name, not a caller alias); got "
        + ", ".join(f"{d.code}:{d.message}" for d in get_diagnostics(src))
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


# FP-RBS-12 — regexp/scan output-var conditional defs reach both reviewer cases (D1-4)


FP_RBS_12_REPRO = "proc f {} { regexp {x} y -> v; if {1} { puts $v } }\n"


def test_FP_RBS_12_regexp_unconditional_read_after_no_match_fires():
    """TP (reviewer case A): the regexp pattern ``{x}`` against the
    literal input ``y`` is provably non-matching.  The subsequent
    unconditional ``if {1} { puts $v }`` reads $v on a path where the
    regexp never wrote it.  D1-4 closure (F2 same-statement dominator
    walk) detects this and fires W210."""
    w210 = [d for d in _rbs(FP_RBS_12_REPRO) if d.code == "W210" and "'v'" in (d.message or "")]
    assert w210, (
        "post-regexp unconditional read of $v after provably-no-match "
        "must fire W210; got: " + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_12_REPRO))
    )


def test_FP_RBS_12_regexp_in_negated_if_arm_fires():
    """TP (reviewer case B): ``if {![regexp {x} y -> v]} { puts $v }``
    -- the if-arm executes when regexp returns 0 (no match); on that
    path $v was not written.  D1-4 closure F2 extension for embedded
    conditions propagates the "no-match implies unset" fact into the
    negated condition arm."""
    src = "proc f {} { if {![regexp {x} y -> v]} { puts $v } }\n"
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert w210, (
        "regexp output var read inside the no-match arm of an if-negated "
        "condition must fire W210; got: " + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


def test_FP_RBS_12_regexp_match_arm_read_silent():
    """TN control: ``if {[regexp {x} y -> v]} { puts $v }`` -- read
    only on the MATCH arm where regexp definitely wrote $v.  W210 must
    NOT fire."""
    src = "proc f {} { if {[regexp {x} y -> v]} { puts $v } }\n"
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'v'" in (d.message or "")]
    assert not w210, "regexp match-arm read of $v is safe; W210 must NOT fire; got: " + ", ".join(
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


def test_FP_RBS_05_namespace_upvar_silent():
    """FP / namespace-upvar alias def: ``namespace upvar ns src alias``
    is a real caller-scope def of ``alias`` (tclsh-verified,
    behaviourally identical to ``upvar 1 ::ns::src alias``).

    Locked the precision gap closure: the namespace dialect spec for
    ``namespace upvar`` now declares an ``arg_role_resolver`` that
    marks the local-alias positions (every other arg starting at
    index 2) as ``ArgRole.VAR_WRITE``.  The standard lowering then
    records them in ``IRCall.defs`` and SSA-driven W210 honours them."""
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


# PR #498/#499 follow-up: DYNAMIC_NAME_LOCAL trait must reach the
# manual scan/regexp/regsub/lassign handlers, not just the generic
# registry path.  (Finding 6.)


def test_FP_callbyname_scan_target_not_caller_alias():
    src = "proc maybe {target} { scan 42 %d $target }\nproc caller {} { set x 1; maybe x }"
    diags = [
        d for d in get_diagnostics(src) if d.code in ("W211", "W220") and "'x'" in (d.message or "")
    ]
    assert diags, "scan $target -- caller's x must still fire W211/W220"


def test_FP_callbyname_regexp_target_not_caller_alias():
    src = "proc maybe {target} { regexp {(.)} a -> $target }\nproc caller {} { set x 1; maybe x }"
    diags = [
        d for d in get_diagnostics(src) if d.code in ("W211", "W220") and "'x'" in (d.message or "")
    ]
    assert diags, "regexp $target -- caller's x must still fire W211/W220"


def test_FP_callbyname_regsub_target_not_caller_alias():
    src = "proc maybe {target} { regsub a a b $target }\nproc caller {} { set x 1; maybe x }"
    diags = [
        d for d in get_diagnostics(src) if d.code in ("W211", "W220") and "'x'" in (d.message or "")
    ]
    assert diags, "regsub $target -- caller's x must still fire W211/W220"


def test_FP_callbyname_lassign_target_not_caller_alias():
    src = "proc maybe {target} { lassign {1} $target }\nproc caller {} { set x 1; maybe x }"
    diags = [
        d for d in get_diagnostics(src) if d.code in ("W211", "W220") and "'x'" in (d.message or "")
    ]
    assert diags, "lassign $target -- caller's x must still fire W211/W220"


def test_FP_callbyname_upvar_alias_still_suppresses():
    """TP control: when the callee actually uses ``upvar $target v``,
    the caller-side x IS being read/written through the alias -- the
    DYNAMIC_NAME_LOCAL refinement must not break that."""
    src = (
        "proc maybe {target} { upvar 1 $target v\n scan 42 %d v }\n"
        "proc caller {} { set x 1; maybe x }"
    )
    diags = [
        d for d in get_diagnostics(src) if d.code in ("W211", "W220") and "'x'" in (d.message or "")
    ]
    assert not diags, "upvar-aliased callee write must continue to suppress caller W211/W220"


# FP-RBS-13 — `tailcall` replaces the frame; code after it never runs


FP_RBS_13_REPRO = """\
proc f {cond} {
    # tailcall g replaces this frame: the `return $result` below is only
    # reached via the else branch, where result is always set.
    if {$cond} {
        tailcall g
    } else {
        set result 1
    }
    return $result
}
"""


def test_FP_RBS_13_tailcall_with_args_silent():
    """FP: ``tailcall g`` ends the proc's straight-line flow (TclNRTailcallObjCmd
    returns TCL_RETURN), so ``return $result`` is reached only via the else
    branch where result is set.  No RBS code may fire."""
    assert _rbs(FP_RBS_13_REPRO) == [], (
        "tailcall-terminated branch must not leave 'result' read-before-set; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_13_REPRO))
    )


def test_FP_RBS_13_bare_tailcall_silent():
    """FP: bare ``tailcall`` (no args) is *also* a terminator -- the C
    implementation returns TCL_RETURN for any arg count (the arg count only
    decides what runs after the frame pops).  tclsh-verified: a bare tailcall
    ends the proc returning ``""``.  So this shape is silent too."""
    src = (
        "proc f {cond} {\n"
        "    if {$cond} { tailcall } else { set result 1 }\n"
        "    return $result\n"
        "}\n"
    )
    assert _rbs(src) == [], (
        "bare tailcall is a terminator; 'result' must not be read-before-set; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


def test_FP_RBS_13_non_terminating_branch_still_fires():
    """TP control: replace ``tailcall`` with a normal (completing) command and
    ``result`` becomes genuinely maybe-unset on the then-path, so W210 fires.
    Proves the suppression is specific to the terminator, not the if/return
    shape."""
    src = (
        "proc f {cond} {\n    if {$cond} { puts hi } else { set result 1 }\n    return $result\n}\n"
    )
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'result'" in (d.message or "")]
    assert w210, (
        "non-terminating then-branch leaves 'result' maybe-unset; W210 must fire; got: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


# FP-RBS-14 — opaque-switch arm that can't complete normally is excluded from must-define


FP_RBS_14_REPRO = """\
proc f {x} {
    # the a* arm returns, so it never reaches `puts $y`; the only path that
    # does (default) sets y -> y is definitely defined.
    switch -glob $x {
        a* { return 0 }
        default { set y 2 }
    }
    puts $y
}
"""


def test_FP_RBS_14_returning_arm_silent():
    """FP: the ``a*`` arm returns, so it never reaches ``puts $y``; every path
    that does (default) assigns ``y``.  The opaque-switch must-define excludes
    non-completing arms, so ``y`` is definitely defined -- no W210."""
    assert _rbs(FP_RBS_14_REPRO) == [], (
        "returning switch arm must not drop 'y' from the must-define set; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(FP_RBS_14_REPRO))
    )


def test_FP_RBS_14_erroring_arm_silent():
    """FP: an arm that always ``error``s likewise cannot complete normally and
    is excluded from the must-define intersection."""
    src = (
        "proc f {x} {\n"
        "    switch -glob $x { a* { error bad } default { set y 2 } }\n"
        "    puts $y\n"
        "}\n"
    )
    assert _rbs(src) == [], (
        "erroring switch arm must not drop 'y' from the must-define set; current: "
        + ", ".join(f"{d.code}:{d.message}" for d in _rbs(src))
    )


def test_FP_RBS_14_omitting_arm_still_fires():
    """TP control: an arm that *completes normally* without assigning ``y``
    (falls through) leaves ``y`` genuinely maybe-unset, so W210 must fire.
    Proves the exclusion is limited to non-completing arms."""
    src = (
        "proc f {x} {\n    switch -glob $x { a* { set z 9 } default { set y 2 } }\n    puts $y\n}\n"
    )
    w210 = [d for d in _rbs(src) if d.code == "W210" and "'y'" in (d.message or "")]
    assert w210, "omitting arm leaves 'y' maybe-unset; W210 must fire; got: " + ", ".join(
        f"{d.code}:{d.message}" for d in _rbs(src)
    )
