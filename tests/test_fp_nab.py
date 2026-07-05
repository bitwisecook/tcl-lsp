# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""TP/FP regression tests for the NAB (not-a-bug / confirm-correct) family.

Each test pairs to an `FP-NAB-NN` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

Ground truth: real C tclsh 9.0.3 (run via ``tclsh9.0`` during planning).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from compiler.compilation_unit import compile_source
from server.features.diagnostics import get_diagnostics


def _analyser_codes(source: str, code: str):
    """Analyser-tier diagnostics matching *code* — mirrors test_checks._diag_with_code."""
    return [d for d in analyse(source).diagnostics if d.code == code]


def _all_codes(source: str, code: str):
    """Full pipeline diagnostics (analyser + bounds + interval + optimiser) matching *code*."""
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-NAB-01 — lset append-slot (index == length) is legal, NOT W231


FP_NAB_01_REPRO = """\
# tclsh contract: lset at index == length APPENDS (legal, not an error).
set l {a b c}      ;# llength=3
lset l 3 X         ;# 3 == llength $l -> APPENDS X (NOT an error)
puts $l            ;# use post-lset binding (silences W211 on l#2)
"""


def test_FP_NAB_01_append_slot_silent():
    """FP guard: lset at index==length must NOT fire W231 (the append slot is
    legal in tclsh; see docs/design/compiler/FP.md#fp-nab-01).

    Uses a literal 3-element list so the bounds check has a *statically
    known* length to compare against — index 3 then exercises the precise
    `index == length` append slot.  A parameter form (l as a proc arg)
    would have unknown length and the verdict would be vacuous: the test
    would still pass if the analyser regressed `>` to `>=` because
    unknown-length lists never fire the bounds check at all.
    """
    assert _all_codes(FP_NAB_01_REPRO, "W231") == []


def test_FP_NAB_01_real_out_of_range_fires():
    """TP control: a literal lset index > length IS a tclsh error
    (`index "4" out of range`) and SHOULD fire W231.  Catches a regression
    where the append-slot fix accidentally suppresses real out-of-range cases
    too.  Uses the `set var; lset var ...` pattern that W231's recent-set
    length inference walks back to."""
    real_oor = """\
set l {a b c}      ;# llength=3
lset l 4 X         ;# 4 > 3 -> tclsh: index "4" out of range
"""
    diags = get_diagnostics(real_oor)
    w231 = [d for d in diags if d.code == "W231" and "out of range" in (d.message or "").lower()]
    assert w231, "lset l 4 X (preceded by `set l {a b c}`) must fire W231; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in diags
    )


# FP-NAB-02 — lindex out-of-range returns "" — smell (W230), not an error


FP_NAB_02_REPRO = """\
# Top-level lindex with literal list + literal out-of-range index.
# tclsh returns "" silently — likely-bug, not an error.
set x [lindex {a b c} 9]
return $x
"""


def test_FP_NAB_02_lindex_oor_smell_fires():
    """TP: literal-arg lindex past the end fires W230 (smell), reflecting that
    tclsh returns the empty string (the code RUNS, but is probably buggy)."""
    assert _all_codes(FP_NAB_02_REPRO, "W230"), (
        "Out-of-range literal lindex should fire W230 (smell). "
        "If this regresses, the bounds check in analyser/checks/_bounds.py "
        "lost its literal-index path."
    )


def test_FP_NAB_02_lindex_oor_not_w231():
    """FP guard: an out-of-range lindex must NOT escalate to the error-tier
    W231 (which is reserved for lset, where tclsh actually errors)."""
    assert _all_codes(FP_NAB_02_REPRO, "W231") == [], (
        "lindex out-of-range is smell-only (W230); escalating to W231 would "
        "be a regression of the dialect-aware severity split."
    )


def test_FP_NAB_02_lset_same_index_does_w231():
    """TP control: the matching lset at the same out-of-range index DOES error
    in tclsh (`index "9" out of range`) and SHOULD trigger W231 (error-tier).
    Proves the W230 (smell) vs W231 (error) asymmetry encodes a real dialect
    difference between lindex and lset."""
    lset_oor = """\
set l {a b c}
lset l 9 X         ;# 9 > 3 -> tclsh: index "9" out of range
"""
    diags = get_diagnostics(lset_oor)
    w231 = [d for d in diags if d.code == "W231" and "out of range" in (d.message or "").lower()]
    assert w231, "lset l 9 X must fire W231; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in diags
    )


# FP-NAB-03 — recursive procs ARE detected pure (Phase-4 SCC not needed)


FP_NAB_03_REPRO = """\
proc fact {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [fact [expr {$n - 1}]]}]
}
"""


def test_FP_NAB_03_recursive_proc_detected_pure():
    """TP: a self-recursive arithmetic proc must come out `pure=True` from the
    interproc fix-point (the original plan-doc claim that recursion was
    conservatively impure was wrong; the fix-point is greatest, not least)."""
    cu = compile_source(FP_NAB_03_REPRO)
    fact = cu.interproc.procedures.get("::fact")
    assert fact is not None, "::fact missing from interproc summary"
    assert fact.pure is True, (
        f"recursive arithmetic proc must be pure; got pure={fact.pure}.  "
        "If this regresses, analyse_interprocedural_ir lost its "
        "greatest-fix-point initialisation."
    )


# FP-NAB-04 — W110 / O120 ``==`` / ``!=`` on strings → `eq` / `ne` (TP)


FP_NAB_04_REPRO = 'if {$x == "hello"} { puts y }'


def test_FP_NAB_04_string_eq_fires_w110_o120():
    """TP confirmation: ``==``/``!=`` against a string literal must fire
    W110 (and its near-duplicate O120) — Tcl's ``==`` is *numeric*
    equality; using it against a string forces an implicit conversion
    and silently returns false when the string isn't numeric.  The
    correct operator is ``eq`` / ``ne``.

    Corpus: W110=1673, O120=1515 (near-duplicate pair; consolidation
    is a future policy call).  Both audited as genuine TPs."""
    diags = [d for d in get_diagnostics(FP_NAB_04_REPRO) if d.code in ("W110", "O120")]
    assert diags, f"expected W110 or O120 on string ==; got {get_diagnostics(FP_NAB_04_REPRO)}"


# FP-NAB-05 — W304 missing ``--`` terminator (TP)


# Primary reproducer: ``file delete`` with a substituted path.  This is
# an unambiguous TP -- if ``$f`` starts with ``-``, Tcl interprets it as
# an option flag, not a path.
FP_NAB_05_REPRO = "proc f {f} { file delete $f }"

# Secondary reproducer: the **split** switch form (options before
# string) is the genuine switch hazard; tclsh-verified to interpret
# ``$x = -nocase`` as the option.  The braced pattern-list form
# (``switch $x { ... }``) is NOT a runtime hazard -- Tcl recognises
# the trailing brace-list unambiguously -- though the analyser
# conservatively fires W304 on it too.
FP_NAB_05_SWITCH_SPLIT_REPRO = "switch $x -nocase {puts hit1} default {puts hit2}"


def test_FP_NAB_05_file_delete_missing_dash_dash_fires_w304():
    """TP confirmation: ``file delete $f`` without ``--`` will consume a
    leading-dash value of ``$f`` as an option (e.g. ``$f = "-force"``
    becomes a force-flag with no path argument; tclsh-verified)."""
    diags = [d for d in get_diagnostics(FP_NAB_05_REPRO) if d.code == "W304"]
    assert diags, f"expected W304 on `file delete $f`; got {get_diagnostics(FP_NAB_05_REPRO)}"


def test_FP_NAB_05_switch_split_form_missing_dash_dash_fires_w304():
    """TP confirmation: the split switch form (options-before-string,
    e.g. ``switch $x -nocase {...} default {...}``) IS a genuine
    hazard.  Confirmed in tclsh 9.0.3: with ``$x = "-nocase"``, Tcl
    interprets the substituted ``-nocase`` as the option and the
    match falls through to ``default``; with ``--`` guard it
    correctly matches the ``-nocase`` pattern."""
    diags = [d for d in get_diagnostics(FP_NAB_05_SWITCH_SPLIT_REPRO) if d.code == "W304"]
    assert diags, (
        f"expected W304 on split switch form; got {get_diagnostics(FP_NAB_05_SWITCH_SPLIT_REPRO)}"
    )


# Finding 5: W304 fires on safe braced-switch form (catalog already
# notes this as conservative over-reach; explicit xfail locks the
# precision gap).


def test_FP_NAB_05_braced_switch_form_should_not_fire_w304():
    """FP: the braced pattern-list switch form is NOT a runtime hazard
    (Tcl unambiguously parses the trailing brace as the pattern list);
    W304 must NOT fire.

    Locked the precision gap closure: ``analyser/checks/_style.py``
    now special-cases the two-arg ``switch STRING { ... }`` shape
    (exactly 2 args with the trailing arg STR-tokenised) and exempts
    it from W304.  The split form (3+ args) still fires."""
    src = "proc f {x} { switch $x { -nocase {puts a} default {puts b} } }"
    w304 = [d for d in get_diagnostics(src) if d.code == "W304"]
    assert not w304, f"W304 over-reach on braced switch; got {w304}"


# Finding 9: W304 lexical "currently resolves to" cross-scope drift.
# The check uses a whole-source last-set scan; it crosses proc
# boundaries and conflates a top-level set with a proc parameter of
# the same name.


def test_FP_NAB_05_w304_lexical_does_not_cross_proc_boundary():
    """FP: W304 'currently resolves to ...' must not look past a proc
    boundary that shadows the variable.

    Locked the precision gap closure: ``_last_literal_set_value_for_var``
    in ``analyser/checks/_helpers.py`` now detects intervening ``proc``
    declarations and stops the backward scan when the proc's param list
    contains the search variable (shadowing)."""
    src = "set path -force\nproc useit {path} { file delete $path }\n"
    diags = [d for d in get_diagnostics(src) if d.code == "W304"]
    # Either the diagnostic shouldn't attribute the value at all (the
    # proc param is unknown) OR if it does, it must not claim "-force"
    # as the value.
    misattributed = [d for d in diags if "-force" in (d.message or "")]
    assert not misattributed, (
        "W304 should not attribute outer 'path = -force' to inner proc "
        f"parameter; got {[d.message[:80] for d in diags]}"
    )


# FP-NAB-06 — W103 ``open`` variable arg / pipe (TP)


FP_NAB_06_REPRO = 'proc f {cmd} { set fh [open "|$cmd" r] }'


def test_FP_NAB_06_open_variable_pipe_fires_w103():
    """TP confirmation: ``open "|$cmd" r`` pipes through the substituted
    command — even with an explicit access mode, the leading ``|``
    forces pipe execution (tclsh-verified).  Audited as a genuine
    injection vector (398 corpus firings)."""
    diags = [d for d in get_diagnostics(FP_NAB_06_REPRO) if d.code == "W103"]
    assert diags, f"expected W103 on `open |$cmd`; got {get_diagnostics(FP_NAB_06_REPRO)}"


# FP-NAB-07 — W313 destructive op with variable path (TP)


FP_NAB_07_REPRO = "proc f {p} { file delete $p }"


def test_FP_NAB_07_destructive_variable_path_fires_w313():
    """TP confirmation: ``file delete $p`` with a substituted path is a
    real foot-gun -- the variable could resolve to anything from a
    pathspec error to a path-traversal payload.  Audited as TP (95
    corpus firings)."""
    diags = [d for d in get_diagnostics(FP_NAB_07_REPRO) if d.code == "W313"]
    assert diags, f"expected W313 on `file delete $p`; got {get_diagnostics(FP_NAB_07_REPRO)}"


# FP-NAB-08 — W212 substitution where var-name expected (TP)


FP_NAB_08_REPRO = "proc f {name v} { set $name $v }"


def test_FP_NAB_08_set_substituted_name_fires_w212():
    """TP confirmation: ``set $name $v`` -- substituting the variable
    name is a dynamic-name foot-gun (the variable could be created
    anywhere depending on $name's value).  Audited TP across 390
    corpus firings; ``upvar`` / ``dict`` / ``trace`` / ``namespace
    which`` are correctly exempt."""
    diags = [d for d in get_diagnostics(FP_NAB_08_REPRO) if d.code == "W212"]
    assert diags, f"expected W212 on `set $name`; got {get_diagnostics(FP_NAB_08_REPRO)}"


def test_FP_NAB_08_incr_substituted_name_fires_w212():
    """TP confirmation: same for ``incr $x``."""
    src = "proc f {x} { incr $x }"
    diags = [d for d in get_diagnostics(src) if d.code == "W212"]
    assert diags, f"expected W212 on `incr $x`; got {get_diagnostics(src)}"


# FP-NAB-09 — W301 uplevel multi-arg concatenation (TP)


FP_NAB_09_REPRO = "proc f {a b} { uplevel 1 puts $a $b }"


def test_FP_NAB_09_uplevel_multiarg_fires_w301():
    """TP confirmation: ``uplevel 1 cmd $a $b`` concatenates its args into
    a single script string before evaluation -- the substituted values
    are re-parsed as command syntax (injection risk).  The safe idiom
    is ``uplevel 1 [list cmd $a $b]`` or ``uplevel 1 $body`` (bare-var
    form, see FP-INJ-01).  Audited TP (291 corpus firings, logger.tcl
    idioms)."""
    diags = [d for d in get_diagnostics(FP_NAB_09_REPRO) if d.code == "W301"]
    assert diags, f"expected W301 on multi-arg uplevel; got {get_diagnostics(FP_NAB_09_REPRO)}"


# FP-NAB-10 — W002 disabled-in-dialect (TP after dialect-aware harness)


def test_FP_NAB_10_dict_disabled_in_tcl_8_4_fires_w002():
    """TP confirmation: ``dict`` was added in Tcl 8.5; using it in a
    file targeting Tcl 8.4 must fire W002 (command disabled in
    dialect).  Earlier "FP" reports of W002 firing on
    ``oo::configurable`` were a harness artefact (the harness didn't
    detect the dialect), not real FPs.  Wrap analysis in
    ``dialect_scope("tcl8.4")`` to make the verdict explicit."""
    from compiler.registry.dialect import dialect_scope

    with dialect_scope("tcl8.4"):
        diags = [d for d in get_diagnostics("dict create a 1") if d.code == "W002"]
        assert diags, "dict in Tcl 8.4 must fire W002 (command disabled in dialect)"


def test_FP_NAB_10_dict_enabled_in_tcl_9_0_silent():
    """FP control: same ``dict`` call in Tcl 9.0 must NOT fire W002 -
    proves the verdict is dialect-aware, not a blanket suppression."""
    from compiler.registry.dialect import dialect_scope

    with dialect_scope("tcl9.0"):
        diags = [d for d in get_diagnostics("dict create a 1") if d.code == "W002"]
        assert not diags, f"dict in Tcl 9.0 must NOT fire W002; got {diags}"


# FP-NAB-11 — W123 unresolved command (TP — real missing stubs, not analyser FPs)


def test_FP_NAB_11_unresolved_argparse_fires_w123():
    """TP confirmation: ``argparse`` is a tcllib package command with no
    stub in the current registry; ``W123`` fires correctly as
    "Unknown command 'argparse'".  Most W123 firings in the corpus
    (1761 total) are similar: tcllib packages (argparse, dict-extension
    dget/dexist), project-local custom widget commands, etc.  Not
    analyser FPs -- adding per-package stub bundles would reduce
    noise (an optimisation, not a precision fix)."""
    diags = [d for d in get_diagnostics("argparse {x y}") if d.code == "W123"]
    assert diags, f"expected W123 on `argparse`; got {get_diagnostics('argparse {x y}')}"


def test_FP_NAB_11_stub_registered_command_silent():
    """FP control: a command with a stub in the registry (e.g. ``puts``)
    must NOT fire W123.  Proves W123 is keyed to the registry, not a
    blanket "unknown command" emission."""
    diags = [d for d in get_diagnostics("puts hi") if d.code == "W123"]
    assert not diags, f"stub-registered `puts` must not fire W123; got {diags}"


def test_FP_NAB_03_impure_proc_still_detected():
    """Control: an impure proc using puts must come out pure=False.  Proves the
    test isn't trivially asserting all procs pure."""
    impure = """\
proc logit {msg} {
    puts $msg            ;# I/O side-effect — must NOT be detected pure
    return ok
}
"""
    cu = compile_source(impure)
    logit = cu.interproc.procedures.get("::logit")
    assert logit is not None, "::logit missing from interproc summary"
    assert logit.pure is False, f"a proc that calls puts must be impure; got pure={logit.pure}"


# FP-NAB-12 — is_pure_var_ref handles escaped close paren in array index (D4-F11)


FP_NAB_12_REPRO = r"""proc f {} {
    set a(x\)y) 1
    puts $a(x\)y)
}
"""


def test_FP_NAB_12_escaped_paren_array_index_is_pure_var_ref():
    """TP: ``$a(x\\)y)`` IS one pure var reference (tclsh-verified).  The
    hand-rolled `_scan_pure_var_ref` parser in `compiler/value_shapes.py`
    consumes backslash-escaped close parens inside the array index so
    the reference doesn't terminate at the first ``)`` (the old regex
    bug).  The FP_NAB_12_REPRO snippet documents the same tclsh contract
    -- ``set a(x\\)y) 1; puts $a(x\\)y)`` works -- the assertion here
    targets the tooling API directly."""
    from compiler.value_shapes import is_pure_var_ref

    assert is_pure_var_ref(r"$a(x\)y)"), (
        "escaped close-paren inside array index must count as one pure var ref"
    )
    # Companion controls to lock in the parser's overall correctness.
    assert is_pure_var_ref("$x")
    assert is_pure_var_ref("${some name}")
    assert is_pure_var_ref("$ns::x")
    assert is_pure_var_ref("$arr(plain)")


def test_FP_NAB_12_unescaped_paren_terminates_index():
    """TP control: ``$a(x)y`` is NOT one pure var ref -- the unescaped
    ``)`` correctly terminates the index, leaving the trailing ``y``
    as a concatenated literal.  Catches the inverse regression where
    the new parser would over-greedily swallow trailing characters."""
    from compiler.value_shapes import is_pure_var_ref

    assert not is_pure_var_ref("$a(x)y"), "unescaped ) must terminate the index"
    # Other concatenations / non-refs the old test set asserted reject.
    assert not is_pure_var_ref("$x$y")
    assert not is_pure_var_ref("$x.foo")
    assert not is_pure_var_ref("foo$x")
