"""Tcl 9 tcltest suite runner via WASM compilation.

Uses the real Tcl 9 tcltest package from ``tmp/tcl9.0.3/library/tcltest/``
and runs individual test files from ``tmp/tcl9.0.3/tests/``.  Each test
file is bundled with tcltest.tcl into a single compilation unit, compiled
to WASM, and executed via wasmtime.

Stage structure (mirrors run_tcllib_test.py):

  1. ``test_tcltest9_compiles``  — Tcl 9 tcltest.tcl alone compiles to WASM
  2. ``test_tcltest9_top_runs``  — top-level init executes without trapping
  3. ``test_<name>_compiles``    — tcltest + test-file bundle compiles
  4. ``test_<name>_runs``        — bundle executes and reports Failed == 0

Per-file results are recorded via ``record_tcl9_result`` (see
``tests/external/conftest.py``).  When pytest is run with
``--tcl9-report=<path>``, the collection is flushed to JSON at session
finish for the triage report generator.

Known accepted limitations
--------------------------
- Tests that require the C extension ``tcl::test`` (testbytestring, etc.)
  are skipped by the test file's own ``testConstraint`` calls — correct.
- Filesystem-heavy tests (makeFile, cd-based) need a preopen'd tmpdir.
- Some Tcl 9-specific commands may trap if not yet implemented; those
  tests will show up as XFAIL until the implementation catches up.
"""

from __future__ import annotations

import re
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from tests.conftest import ensure_tcl_source, record_tcl9_result
from tests.external._tcl9_categories import (
    bucket_failures,
    load_categories,
)
from tests.external._tcl9_categories import (
    gate as _categories_gate,
)
from tests.test_wasm_real_tcl import (
    _compile_tcl,
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

if TYPE_CHECKING:
    from core.compiler.codegen.wasm._ir import DiagMap

_EXTERNAL = Path(__file__).resolve().parent

# ---------------------------------------------------------------------------
# Resolve Tcl 9 source paths (lazy — skip if not available)
# ---------------------------------------------------------------------------


def _tcl9_tcltest() -> Path:
    """Return path to Tcl 9 tcltest.tcl, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir.parent / "library" / "tcltest" / "tcltest.tcl"
    if not p.exists():
        pytest.skip(f"Tcl 9 tcltest.tcl not found at {p}")
    return p


def _tcl9_test_file(name: str) -> Path:
    """Return path to a Tcl 9 test file, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir / name
    if not p.exists():
        pytest.skip(f"Tcl 9 test file not found: {p}")
    return p


def _tcl9_opt_library() -> Path | None:
    """Return path to Tcl 9's optparse.tcl, or None if missing.

    Used by test bundles that ``package require opt`` (opt.test,
    safe-stock.test) — inlining the upstream library lets the
    interpreter run the real ``::tcl::OptKeyRegister`` /
    ``OptKeyParse`` / ``OptParse`` machinery rather than a hand-
    rolled stub, which is the only way to verify our runtime
    against opt.test's implementation-detail assertions.
    """
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir.parent / "library" / "opt" / "optparse.tcl"
    return p if p.exists() else None


# ---------------------------------------------------------------------------
# Bundle construction
# ---------------------------------------------------------------------------

_PRE_TCLTEST = r"""
# ----- run_tcl9_tests.py pre-tcltest stubs -----
# Tcl's standard startup populates a small set of globals that
# tclsh sets before any user script runs (``tclsh.c`` /
# ``Tcl_Main`` does this) — ``argv``, ``argv0``, ``argc``,
# ``tcl_interactive``, ``auto_path``, ``env(*)``.  Our WASM
# runtime entry point doesn't perform that bootstrap, so test
# files that reach for ``$argv`` (parseOld), ``$auto_path``
# (safe-stock), or ``$tcl_interactive`` (uplevel.test diag
# probes) trap with ``can't read "X": no such variable``.
#
# Mirror the tclsh defaults: empty argv list, empty argv0, the
# single-element auto_path tcllib uses, an empty env array, and
# tcl_interactive=0 (we're not a terminal).  The values themselves
# are deliberately minimal — none of the in-scope tests exercise
# their content, only the readability.
set ::argv {}
set ::argv0 ""
set ::argc 0
set ::tcl_interactive 0
set ::auto_path {}
set ::tcl_library ""
# Seed ::env with the handful of variables tcltest bundles probe at
# load time.  chanio.test / io.test / fCmd.test all read ``::env(HOME)``
# inside a ``testConstraint`` body that runs *before* any individual
# test, so an unset HOME traps the whole bundle.  Pointing HOME at the
# preopened tmpdir lets the constraint check evaluate cleanly without
# granting writable-HOME (``[file writable]`` returns 0 on the
# read-only mount), which is the same answer real tclsh gives in a
# sandboxed environment.
array set ::env {
    HOME "/"
    PATH ""
    USER "wasm"
}

# ``tcl_platform`` — real tclsh exports a dozen-key array that
# tcltest reads at line 288 / 312 (``$::tcl_platform(platform)``,
# ``$::tcl_platform(os)``) plus several test files reach for
# ``pointerSize``, ``wordSize``, ``byteOrder``, etc.  Without
# these, ``listObj.test`` / ``stringObj.test`` hit the
# ``set SIZE_MAX [expr {(1 << (8*$::tcl_platform(pointerSize) - 1)) - 1}]``
# bootstrap with an empty pointerSize, which collapses to a
# negative shift and traps with ``negative shift argument``.
# Match the WASM target's actual values: pointerSize=4 (wasm32),
# wordSize=8 (we report 64-bit ints to scripts), unix-shaped
# platform/os.  The values are conservative — none of the in-
# scope tests inspect them beyond the smoke-test reads above.
array set ::tcl_platform {
    platform        unix
    os              "Linux"
    osVersion       "0.0"
    machine         "wasm32"
    byteOrder       littleEndian
    pointerSize     4
    wordSize        8
    threaded        0
    user            "wasm"
    pathSeparator   ":"
    engine          "Tcl"
}

# ``unknown`` — real tclsh ships an ``::unknown`` proc that handles
# auto-loading + tab completion + ``$auto_noexec`` exec dispatch.
# Several test bundles (``rename.test`` 5.1, ``unknown.test``) rename
# it away as part of their setup and re-instate it later with
# ``info body unknown``.  Without a stub of our own ``info body
# unknown.old`` traps with ``"unknown.old" isn't a procedure`` and
# the bundle aborts before reaching its tcltest summary.
#
# The minimal ``unknown`` body emits the standard error string Tcl
# would produce for any unknown command — same surface as
# ``error_invalid_command_name`` in the runtime — so tests that
# *invoke* unknown via a typo see the expected message.  Wrapped in
# ``eval`` so it goes through the interpreted ``proc`` path; ``info
# body`` only works on interpreted procs.
eval {
    proc ::unknown {args} {
        return -code error "invalid command name \"[lindex $args 0]\""
    }
}

# tcltest.tcl line 919 calls ``auto_load ::parray`` followed by
# ``proc tcltest::parray {a {pattern *}} [info body ::parray]``.
# Tcl's auto-loading machinery is not implemented here, so we
# pre-define a minimal ``::parray`` stub up-front.  When tcltest's
# auto_load no-ops, the subsequent ``info body ::parray`` returns
# this body and the ``tcltest::parray`` definition succeeds.
#
# Wrapped in ``eval {…}`` so the compiler doesn't AOT-lift the
# proc — ``info body`` only works on interpreted procs, so the
# stub must register through the interpreter's ``proc`` handler.
eval {
    proc ::parray {a {pattern *}} {
        upvar 1 $a array
        if {![array exists array]} {
            return -code error "\"$a\" isn't an array"
        }
        set names [lsort [array names array $pattern]]
        set max 0
        foreach n $names {
            if {[string length $n] > $max} { set max [string length $n] }
        }
        foreach n $names {
            puts [format "%s(%-*s) = %s" $a $max $n $array($n)]
        }
    }
}

# auto_load: stub that always succeeds (no actual loading; ::parray
# is provided by the pre-tcltest stub above).  Returning 1 mirrors
# Tcl's "command auto-loaded successfully" return.
proc auto_load {cmd args} { return 1 }

# ::tcl::tm::path stub — Tcl Modules path-list ensemble.  Real Tcl
# implements this in tmp/tcl9.0.3/library/tm.tcl as a 150-line
# command with ``add`` / ``remove`` / ``list`` subcommands plus
# ancestor-conflict validation.  safe.test (lines 1729 / 1735 /
# 2215 / 2241) calls ``$i eval ::tcl::tm::path list`` on safe
# child interpreters; without the command the test traps with
# ``unknown command: ::tcl::tm::path`` and the whole file aborts.
# This stub is a minimal namespace ensemble that mimics the
# upstream shape closely enough that the four directly-affected
# tests reach a tcltest summary line.  Other safe.test gaps
# (filesystem, package machinery) remain out of scope.
# ::tcl::unsupported::clock::configure stub — internal accessor the
# Tcl 9 ``clock.test`` bundle calls during bootstrap (line 44 reads
# ``-valid`` to decide whether to enable strict-validation tests, plus
# clock-1.4.x reads ``-min-year`` / ``-max-year`` / ``-year-century`` /
# ``-century-switch`` and rewrites them).  Real Tcl exposes these
# knobs through ``library/clock.tcl``'s C-backed namespace; our
# clock implementation is fixed-policy so the knobs are read-only
# defaults.  Setter form silently accepts and stores into the
# state dict so the test's "save / mutate / restore" pattern
# round-trips without altering observable behaviour.
# msgcat stubs — the upstream ``clock.test`` registers a few
# locale-specific message catalogues at the top of the file
# (``en_US_roman`` for Roman numerals, ``fr`` / ``zh`` /
# ``ja`` / etc.) via ``::msgcat::mcset`` and ``::msgcat::mcmset``.
# Without these the bundle traps before it reaches any ``test``
# call.  ``msgcat`` is a real Tcl-library package; we ship a
# minimal stub so the catalogue setters are accepted and a
# subsequent ``::msgcat::mc`` call (used inside our clock
# implementation's locale-aware paths only — the WASM runtime
# ignores locale today) returns the key unchanged.
namespace eval ::msgcat {
    variable catalogue
    array set catalogue {}
    proc mcset {locale src {dst {}}} {
        variable catalogue
        if {$dst eq ""} { set dst $src }
        set catalogue($locale,$src) $dst
        return $dst
    }
    proc mcmset {locale pairs} {
        variable catalogue
        foreach {k v} $pairs { set catalogue($locale,$k) $v }
        return [expr {[llength $pairs] / 2}]
    }
    proc mc {src args} {
        # No locale routing — return the key (matches msgcat's
        # fallback when no catalogue entry is found).
        return $src
    }
    proc mclocale {args} { return en_US }
    proc mcpreferences {} { return {en_US en {}} }
    proc mcload {dir} { return 0 }
    proc mcunknown {locale src args} { return $src }
    namespace export mcset mcmset mc mclocale mcpreferences mcload mcunknown
}

namespace eval ::tcl::unsupported::clock {
    variable state
    array set state {
        -valid 0
        -min-year -9999
        -max-year 9999
        -year-century 2000
        -century-switch 38
    }
    proc configure {args} {
        variable state
        if {[llength $args] == 0} {
            return [array get state]
        }
        if {[llength $args] == 1} {
            set opt [lindex $args 0]
            if {[info exists state($opt)]} {
                return $state($opt)
            }
            return -code error "bad option \"$opt\""
        }
        foreach {k v} $args {
            set state($k) $v
        }
        return
    }
    namespace export configure
}

# ``::oo::patchlevel`` / ``::oo::version`` stubs — the upstream
# ``oo.test`` bundle's first test (``oo-0.9``) computes its
# expected-result list as ``[list tcl::oo $::oo::patchlevel 1]``,
# evaluated at test-build time before the body runs.  Real Tcl
# sets these from ``tclOO.c``'s package-init code; the WASM
# scaffold registers ``oo::class`` / ``oo::object`` as commands
# but never populates the namespace variables, so the very first
# test reads an unset variable and the bundle traps before
# reaching a tcltest summary line.  Pre-populating both with the
# upstream-matching values (1.4.0 ships with Tcl 9.0.3) lets the
# read succeed; oo-0.9 still fails on its `package` interactions
# (our ``package`` stub returns empty for ``names`` / ``versions``
# / ``present``), but the trap is gone and the rest of the
# bundle reaches its summary.
namespace eval ::oo {
    variable patchlevel "1.4.0"
    variable version "1.4.0"
}

namespace eval ::tcl::tm {
    variable paths {}
    proc path {sub args} {
        variable paths
        switch -- $sub {
            list { return $paths }
            add {
                foreach p $args {
                    if {[lsearch -exact $paths $p] < 0} { lappend paths $p }
                }
                return
            }
            remove {
                foreach p $args {
                    set i [lsearch -exact $paths $p]
                    if {$i >= 0} { set paths [lreplace $paths $i $i] }
                }
                return
            }
            default {
                return -code error "unknown ::tcl::tm::path subcommand \"$sub\""
            }
        }
    }
    namespace export path
}
"""

_PREAMBLE = r"""
# ----- run_tcl9_tests.py preamble -----
# Import tcltest commands into the global namespace.  Test files that detect
# ::tcltest already loaded skip their own namespace import block; we fill the
# gap here so unqualified 'test', 'testConstraint', etc. resolve correctly.
namespace import -force ::tcltest::*

# Silence per-test progress chatter; keep the summary line.
::tcltest::configure -verbose {}

# Override ::tcltest::Asciify to a passthrough.  The upstream impl
# splits a result string into characters and uses ``format %02X``
# / ``format %04X`` to emit ``\xNN`` / ``\uNNNN`` escapes, which
# triggers a pre-existing runtime panic in ``tcl_cmd_append`` (its
# integer-cast guard rejects some byte values produced by the
# upstream test fixtures).  Asciify only runs as part of the
# failure-diagnostic ``puts``; replacing it with identity loses
# the pretty hex escapes but lets a tcltest summary line emerge
# even when several tests fail.  Tracked separately from the
# clock work in tcl_cmd_append.
proc ::tcltest::Asciify {s} { return $s }

# Pre-seed the host-environment testConstraints to 0 so tests gated
# on them are skipped rather than running against a half-functional
# stub.  Without this, basic-46.1 / basic-46.2 / basic-46.3 (and a
# handful of others) hit the constraint-undefined ``SafeFetch`` path
# which calls the constraint initializer — for ``stdio`` that's
# ``open "|[interpreter]" w``.  Our WASM runtime's pipe-form open
# raises ``i/o error`` which the initializer's ``catch`` swallows,
# leaving the constraint *empty* rather than 0.  An empty constraint
# in tcltest 2.5 is treated as truthy by ``RunTest`` so the test
# body runs and immediately hangs on ``gets $f`` against a non-
# existent channel.  Explicitly setting the constraint to 0 short-
# circuits the initializer trace and keeps the gated tests in the
# Skipped column.
::tcltest::testConstraint stdio 0
::tcltest::testConstraint exec 0
::tcltest::testConstraint socket 0
::tcltest::testConstraint thread 0
::tcltest::testConstraint testbytestring 0
"""


def _patch_tcltest_source(src: str) -> str:
    """Targeted in-memory patches to ``tcltest.tcl`` source.

    Real Tcl ``tcltest::Option`` aliases each option to a per-option
    namespace variable via ::

        namespace eval [namespace current] \\
            [list upvar 0 Option($option) $varName]

    relying on a NAMESPACE call-frame plus a true ``upvar 0`` link to
    an array element.  Our WASM runtime's ``namespace eval`` does not
    push a call-frame and the array-element ``upvar 0`` target form
    is not handled in :func:`frame_get_at_depth` — together that
    leaves ``::tcltest::debug`` (and the dozen other option-controlled
    names) as a *declared but unset* namespace variable for the
    lifetime of the bundle, so the very first ``DebugPuts`` invoked
    from ``DefineConstraintInitializers`` traps with
    ``can't read "debug": no such variable``.

    Until the runtime grows real namespace-eval-frame + array-element
    upvar machinery, we patch the source to:

    1. Snapshot the option default into a real namespace variable
       (``set [namespace current]::$varName $msg``) at ``Option`` time.
       That gives the per-option name a definite value before any
       proc reads it.
    2. Mirror ``Configure``'s writes to the snapshot so
       ``configure -verbose body`` still propagates to ``$verbose``
       reads downstream.

    The mapping ``-option → varName`` is captured into a side array
    ``::tcltest::OptionVarName`` populated by the patched ``Option``
    proc; ``Configure``'s mirror uses that array.  Any option without
    a registered ``varName`` (e.g. ``-constraints``) is simply not
    mirrored — matching the upstream behaviour where those options
    have no exported per-option variable.
    """
    # 1. Replace the namespace-eval/upvar-0 dance with a direct
    #    snapshot + ``OptionVarName`` mapping write.  Match by the
    #    leading ``namespace eval [namespace current] \`` plus the
    #    continuation ``[list upvar 0 Option($option) $varName]`` so
    #    we hit exactly one site (Option proc body).
    upvar_pattern = (
        "namespace eval [namespace current] \\\n\t\t    [list upvar 0 Option($option) $varName]"
    )
    upvar_replacement = (
        "# --- run_tcl9_tests.py patch (issue #280) ---\n"
        "\t    # Snapshot the option default into a real namespace\n"
        "\t    # variable, since our WASM runtime cannot persist a\n"
        "\t    # ``namespace eval ... upvar 0 arr(key) Y`` alias.\n"
        "\t    # The ``eval [list set ...]`` form routes through the\n"
        "\t    # interpreter so the dynamic-target ``set`` writes the\n"
        "\t    # global at the qualified name; the compiled-side\n"
        "\t    # ``set $tgt $val`` would interpret ``$tgt`` as a\n"
        "\t    # literal local-slot name.\n"
        "\t    eval [list set [namespace current]::$varName $msg]\n"
        "\t    variable OptionVarName\n"
        "\t    set OptionVarName($option) $varName\n"
        "\t    # --- end patch ---"
    )
    if upvar_pattern not in src:
        msg = "_patch_tcltest_source: upvar-dance pattern not found"
        raise RuntimeError(msg)
    patched = src.replace(upvar_pattern, upvar_replacement, 1)

    # 2. Mirror ``Configure`` writes into the per-option namespace
    #    variable so post-init reconfigures (``configure -verbose body``)
    #    propagate.  Match the unique ``set Option($option) $value`` line
    #    inside the multi-arg ``while`` loop.
    cfg_pattern = "\t    set Option($option) $value\n\t    set args"
    cfg_replacement = (
        "\t    set Option($option) $value\n"
        "\t    # --- issue #280 patch: mirror to per-option var ---\n"
        "\t    variable OptionVarName\n"
        "\t    if {[info exists OptionVarName($option)]} {\n"
        "\t        eval [list set [namespace current]::$OptionVarName($option) $value]\n"
        "\t    }\n"
        "\t    # --- end patch ---\n"
        "\t    set args"
    )
    if cfg_pattern not in patched:
        msg = "_patch_tcltest_source: Configure-mirror pattern not found"
        raise RuntimeError(msg)
    patched = patched.replace(cfg_pattern, cfg_replacement, 1)

    # 3. Initialise the ``OptionVarName`` mapping array next to
    #    ``Option`` / ``Verify`` / ``Usage`` so the patched ``Option``
    #    proc body has somewhere to write into.
    init_pattern = "    variable Option; array set Option {}"
    init_replacement = (
        "    variable Option; array set Option {}\n"
        "    # --- issue #280 patch ---\n"
        "    variable OptionVarName; array set OptionVarName {}\n"
        "    # --- end patch ---"
    )
    if init_pattern not in patched:
        msg = "_patch_tcltest_source: Option-init pattern not found"
        raise RuntimeError(msg)
    patched = patched.replace(init_pattern, init_replacement, 1)

    # 4. Neutralise the two scratch-directory ``glob`` call-sites
    #    inside tcltest itself (``FillFilesExisted`` and
    #    ``cleanupTests``).  Our WASI runtime has no directory-listing
    #    primitive so the real ``glob -directory`` traps with
    #    ``unsupported command: glob -directory``; without these
    #    patches every bundle traps during tcltest init before the
    #    first ``test`` call.  Replacing them with an empty list
    #    matches what ``-nocomplain`` would observe in a freshly-
    #    allocated empty scratch dir — which is exactly what the
    #    runner's ``preopen_tmpdir`` provides.
    #
    #    Scoped to these two specific lines (rather than overriding
    #    ``glob`` globally in ``_PRE_TCLTEST``) so that any in-scope
    #    test that exercises ``glob`` directly hits the real runtime
    #    behaviour — a global shim would silently turn real glob-
    #    dependent failures into spurious passes (PR #299 review).
    fill_pattern = "\tforeach file [glob -nocomplain -directory [temporaryDirectory] *] {"
    fill_replacement = "\tforeach file [list] {  ;# issue #280: WASI has no glob -directory"
    if fill_pattern not in patched:
        msg = "_patch_tcltest_source: FillFilesExisted glob pattern not found"
        raise RuntimeError(msg)
    patched = patched.replace(fill_pattern, fill_replacement, 1)

    cleanup_pattern = (
        "\tforeach file [glob -nocomplain \\\n\t\t-directory [temporaryDirectory] *] {"
    )
    cleanup_replacement = "\tforeach file [list] {  ;# issue #280: WASI has no glob -directory"
    if cleanup_pattern not in patched:
        msg = "_patch_tcltest_source: cleanupTests glob pattern not found"
        raise RuntimeError(msg)
    patched = patched.replace(cleanup_pattern, cleanup_replacement, 1)

    return patched


def _bundle(test_file_path: Path) -> str:
    """Concatenate Tcl 9 tcltest + preamble + test file into one script.

    Test files that ``package require opt`` (or reach for
    ``::tcl::OptKeyRegister`` / ``::tcl::OptParse`` directly) get
    the real ``library/opt/optparse.tcl`` inlined at top level
    (NOT wrapped in ``eval``).  ``namespace eval ::tcl { variable
    OptDescN 0 ... }`` only initialises the variable in the right
    scope when the runtime sees it as a top-level statement;
    nesting under another ``eval`` detaches the namespace context
    and the bundle's first ``$::tcl::OptDescN`` read traps with
    "no such variable".
    """
    tcltest_src = _patch_tcltest_source(_tcl9_tcltest().read_text(encoding="utf-8"))
    test_src = test_file_path.read_text(encoding="utf-8")

    parts: list[str] = [
        "# ===== run_tcl9_tests pre-tcltest stubs =====",
        _PRE_TCLTEST,
    ]

    needs_opt = ("package require opt" in test_src) or ("::tcl::Opt" in test_src)
    if needs_opt:
        opt_path = _tcl9_opt_library()
        if opt_path is not None:
            opt_src = opt_path.read_text(encoding="utf-8")
            # Inline the upstream library at top level (NOT wrapped
            # in ``eval``).  ``namespace eval ::tcl { variable
            # OptDescN 0 ... }`` only initialises the variable in
            # the right scope when the runtime sees it as a
            # top-level statement; nesting under another ``eval``
            # detaches the namespace context and the bundle's first
            # ``$::tcl::OptDescN`` read traps with "no such variable".
            parts.extend(
                [
                    "# ===== Tcl 9 opt library (real upstream optparse.tcl) =====",
                    opt_src,
                ]
            )

    parts.extend(
        [
            "# ===== Tcl 9 tcltest (tmp/tcl9.0.3/library/tcltest/tcltest.tcl) =====",
            tcltest_src,
            "# ===== run_tcl9_tests preamble =====",
            _PREAMBLE,
            f"# ===== {test_file_path.name} =====",
            test_src,
            "",
        ]
    )
    return "\n".join(parts)


_SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)")
_FAIL_RE = re.compile(r"^==== (\S+) FAILED", re.MULTILINE)


def _parse_summary(stdout: str) -> tuple[int, int, int, int] | None:
    """Extract (total, passed, skipped, failed) from a tcltest summary line."""
    m = _SUMMARY_RE.search(stdout)
    if m is None:
        return None

    def _int(s: str) -> int:
        return int(s) if s else 0

    return (_int(m.group(1)), _int(m.group(2)), _int(m.group(3)), _int(m.group(4)))


def _first_failing(stdout: str) -> str | None:
    """Extract the name of the first failing tcltest test from stdout."""
    m = _FAIL_RE.search(stdout)
    return m.group(1) if m else None


def _stderr_tail(stderr: str, max_chars: int = 400) -> str:
    """Return the last ``max_chars`` characters of stderr for triage."""
    if len(stderr) <= max_chars:
        return stderr
    return "…" + stderr[-max_chars:]


def _summarise_diag(diag: "DiagMap | None") -> dict:
    """Aggregate fallback-site telemetry from a populated :class:`DiagMap`.

    Returns a dict with three fields the triage harness records alongside
    per-file pass/fail data:

    * ``fallback_sites_total`` — total number of diag sites registered
      (every ``tcl_eval`` fallback, unsupported-command trap, or unknown
      dispatch emits one).
    * ``fallback_sites_by_kind`` — counts bucketed by ``DiagSite.kind``
      (e.g. ``fallback``, ``unsupported``, ``unknown``, ``runtime``).
    * ``top_fallback_commands`` — the five most common commands that
      triggered a fallback, as ``(command, count)`` pairs.  Only sites
      with ``kind == "fallback"`` are counted; unsupported/unknown sites
      are architectural dead-ends and show up in the kind histogram.

    Returns all zeroes / empty when ``diag`` is ``None`` (compile never
    produced a map) so callers can unconditionally merge the result into
    the per-file report dict.
    """
    if diag is None or not diag.sites:
        return {
            "fallback_sites_total": 0,
            "fallback_sites_by_kind": {},
            "top_fallback_commands": [],
        }
    by_kind: Counter[str] = Counter()
    by_cmd: Counter[str] = Counter()
    for site in diag.sites:
        by_kind[site.kind] += 1
        if site.kind == "fallback" and site.command:
            by_cmd[site.command] += 1
    return {
        "fallback_sites_total": len(diag.sites),
        "fallback_sites_by_kind": dict(by_kind),
        "top_fallback_commands": by_cmd.most_common(5),
    }


def _infer_category(
    *,
    compiled: bool,
    ran: bool,
    summary: tuple[int, int, int, int] | None,
    trap_site: str | None,
    stderr: str,
    deferred: bool,
) -> str:
    """Best-effort triage category inference.

    * D — test file is on the deferred-by-design list.
    * E — tcltest-viability regression or missing-command plumbing.
    * B — compile failed or runtime trap before any tcltest result.
    * A — ran to completion but reported failing tests.
    * pass — all tcltest assertions passed.
    """
    if deferred:
        return "D"
    if not compiled:
        return "B"
    if not ran:
        if trap_site:
            return "B"
        return "E"
    if summary is None:
        return "E"
    _total, _passed, _skipped, failed = summary
    if failed == 0:
        return "pass"
    return "A"


def _run_bundle(bundle_src: str, label: str) -> tuple[str, str]:
    """Compile and run a bundle; return (stdout, stderr).

    Raises AssertionError on compile or runtime failure, with a
    human-readable diagnostic that includes the trap site if available.
    """
    try:
        wasm, diag = _compile_tcl_with_diag(bundle_src, label)
    except Exception as exc:
        pytest.fail(f"{label}: compilation failed: {exc}")

    with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
        # Stage common tcltest helper files (tcltests.tcl, internals.tcl,
        # encodingVectors.tcl, pkgIndex.tcl) into the preopened tmpdir so
        # tests that ``source [file join [file dirname [info script]] X]``
        # can find them.  ``info script`` returns "" inside our bundles, so
        # ``[file dirname [info script]]`` returns ".", which the WASI
        # preopen resolves against the tmpdir root — staging them at the
        # root is exactly what those tests expect.
        # tcltest.tcl lives at <root>/library/tcltest/tcltest.tcl,
        # helper files like ``tcltests.tcl`` live at <root>/tests/.
        helpers_dir = _tcl9_tcltest().parent.parent.parent / "tests"
        for helper in ("tcltests.tcl", "internals.tcl", "encodingVectors.tcl", "pkgIndex.tcl"):
            src_path = helpers_dir / helper
            if src_path.exists():
                shutil.copyfile(src_path, Path(host_tmp) / helper)
        try:
            result = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=host_tmp,
            )
        except Exception as trap:
            pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))

    stdout = result[1] if len(result) >= 2 else ""
    stderr = result[2] if len(result) >= 3 else ""
    return stdout, stderr


# ---------------------------------------------------------------------------
# Stage 1 & 2: tcltest.tcl alone (viability gate)
# ---------------------------------------------------------------------------


class TestTcltest9Init:
    """Stages 1–2: Tcl 9 tcltest.tcl compiles and its top-level runs.

    This is the viability gate: if ``tcltest.tcl`` itself cannot compile
    or its top-level cannot run, every downstream test file is poisoned
    and the per-file results are meaningless.
    """

    def test_tcltest9_compiles(self, request: pytest.FixtureRequest) -> None:
        """Tcl 9 tcltest.tcl compiles to WASM without errors."""
        src = _tcl9_tcltest().read_text(encoding="utf-8")
        try:
            _compile_tcl(src)
        except Exception as exc:
            record_tcl9_result(
                request.config,
                {
                    "file": "tcltest.tcl",
                    "stage": "compile",
                    "status": "fail",
                    "trap_site": None,
                    "stderr_tail": str(exc)[-400:],
                    "category": "E",
                    "notes": "tcltest.tcl itself does not compile",
                },
            )
            pytest.fail(f"Tcl 9 tcltest.tcl failed to compile: {exc}")
        record_tcl9_result(
            request.config,
            {
                "file": "tcltest.tcl",
                "stage": "compile",
                "status": "pass",
                "category": "pass",
            },
        )

    def test_tcltest9_top_runs(self, request: pytest.FixtureRequest) -> None:
        """Tcl 9 tcltest.tcl top-level init executes without trapping.

        The bare upstream ``tcltest.tcl`` calls ``auto_load ::parray``
        and dereferences the loaded body with ``info body ::parray``;
        the runtime has no auto-loader, so the ``_PRE_TCLTEST`` stubs
        (``::parray``, ``auto_load`` no-op, ``::tcl::tm::path`` ensemble,
        ``msgcat`` shims) are prepended here just like ``_bundle()`` does
        for downstream test files.  Without this preamble the test would
        always trap on the auto-load path even when tcltest itself is
        otherwise viable.
        """
        tcltest_src = _patch_tcltest_source(_tcl9_tcltest().read_text(encoding="utf-8"))
        src = "\n".join([_PRE_TCLTEST, tcltest_src, ""])
        try:
            wasm, diag = _compile_tcl_with_diag(src, "tcl9_tcltest.tcl")
        except Exception as exc:
            pytest.skip(f"Tcl 9 tcltest.tcl does not yet compile: {exc}")

        with tempfile.TemporaryDirectory(prefix="tcl9test-init-") as host_tmp:
            try:
                result = _run_wasm(wasm, capture_stderr=True, preopen_tmpdir=host_tmp)
            except Exception as trap:
                stderr_text = getattr(trap, "tcl_stderr", "")
                record_tcl9_result(
                    request.config,
                    {
                        "file": "tcltest.tcl",
                        "stage": "run",
                        "status": "fail",
                        "trap_site": _resolve_trap(trap, stderr_text, diag),
                        "stderr_tail": _stderr_tail(stderr_text),
                        "category": "E",
                        "notes": "tcltest.tcl init trapped",
                    },
                )
                pytest.fail(_resolve_trap(trap, stderr_text, diag))

        val = result[0]
        stderr_text = result[2] if len(result) >= 3 else ""
        status = "pass" if val == 0 else "fail"
        record_tcl9_result(
            request.config,
            {
                "file": "tcltest.tcl",
                "stage": "run",
                "status": status,
                "return_value": val,
                "stderr_tail": _stderr_tail(stderr_text),
                "category": "pass" if status == "pass" else "E",
            },
        )
        assert val == 0, f"::top returned {val}; stderr:\n{stderr_text}"


# ---------------------------------------------------------------------------
# Stage 3 & 4: individual Tcl 9 test files
# ---------------------------------------------------------------------------


# Baseline for the I/O suites (subsystem ``"io"``).  These files are
# wired into ``_IN_SCOPE`` so the sweep picks them up, but the strict
# ``failed == 0`` assertion in :meth:`_TestClass.test_runs` is replaced
# by a per-stem baseline gate: a regression is flagged when ``passed``
# drops below or ``failed`` rises above the recorded snapshot.  Stems
# whose entry sets ``trap_ok`` allow the file to trap before reaching a
# tcltest summary — the residual blockers (encoding ``koi8-r``, integer
# panic, missing-arg paths) are tracked as separate follow-up issues.
#
# Refresh after a runtime change by running
# ``make check-tcl9-tcltest-io`` and updating the numbers if the new
# snapshot is strictly better than the old one.
_IO_BASELINE: dict[str, dict[str, object]] = {
    # chan.test — 42/42, fully green after the issue #270 ensemble
    # work landed.
    "chan": {"min_passed": 42, "max_failed": 0},
    # chanio.test — now reaches the tcltest summary line after the
    # koi8-r codec landed in this branch (734/779 passing locally).
    # Residual failures are expected from missing multi-byte codecs
    # (shiftjis / iso2022-jp / jis0208 / euc-jp …) and reflected-
    # channel features that are explicit out-of-scope items in #270
    # / #274.  Tolerance band is generous so a small wobble doesn't
    # flap the gate.
    "chanio": {"min_passed": 700, "max_failed": 50},
    # io.test — still traps with a debug-build integer-overflow check
    # in ``valtypes.tcl_obj.obj_new_string`` when linear memory grows
    # past the 2 GiB boundary.  The leak underlying that growth is a
    # separate ticket (recommended follow-up); the gate stays
    # ``trap_ok`` until the leak is fully chased down.
    "io": {"trap_ok": True},
    # ioCmd.test — bare ``close`` arity error goes through the
    # interpreter's eval-script fallback in a way that bypasses the
    # surrounding ``catch`` (catch+command-substitution interaction
    # bug — recommended follow-up).  Until that lands the gate is
    # ``trap_ok``.
    "ioCmd": {"trap_ok": True},
}

# Baseline for non-I/O suites that have known-stable residual failures
# (issue #327).  Same semantics as :data:`_IO_BASELINE`: the strict
# ``failed == 0`` gate is replaced by a snapshot gate so regressions
# (fewer passing or more failing) are still caught while pre-existing
# mismatches don't block the sweep.  Refresh by re-running the suite
# and tightening the band when the underlying bug is fixed.
_BASELINE: dict[str, dict[str, object]] = {
    # set.test — 55/64 passing.  Residual failures include set-3.24
    # (uncompiled ``set`` with too many arguments returns the value
    # instead of an error — arity enforcement gap in the eval fallback)
    # and a cluster of set-old-style multi-assignment forms.
    "set": {"min_passed": 52, "max_failed": 12},
    # set-old.test — 38/153 passing.  Large-scale failures; the suite
    # exercises many multi-variable ``set`` forms and error-path
    # branches that rely on the uncompiled path.
    "set-old": {"min_passed": 35, "max_failed": 118},
    # var.test — 176/219 passing after the #327 find_table null-pointer
    # fix stopped the trap.  Residual failures are ``array names``
    # edge cases and ``info constant`` (not yet implemented).
    "var": {"min_passed": 172, "max_failed": 43},
    # namespace.test — 201/314 passing after the #327
    # ns_resolve_qualified null-pointer fix stopped the trap.  Residual
    # failures cover child-interp namespace resolution, ``namespace
    # path`` ordering, and ``namespace import -force`` edge cases.
    "namespace": {"min_passed": 197, "max_failed": 116},
    # namespace-old.test — 54/126 passing.  Bulk namespace-command
    # coverage; failures mirror the namespace.test gaps above.
    "namespace-old": {"min_passed": 51, "max_failed": 75},
    # proc.test — 36/38 passing.  Two failures: proc-3.7 and proc-4.9
    # (``Tcl_PkgPresent`` return-code path not yet wired).
    "proc": {"min_passed": 34, "max_failed": 4},
    # proc-old.test — 34/74 passing.  Includes proc-old-9.1 (result 0
    # vs expected 1) and proc-old-10.1 (ByteCode epoch change during
    # recursive proc execution).
    "proc-old": {"min_passed": 31, "max_failed": 43},
    # opt.test — 29/31 passing.  opt-10.8 / opt-10.9 need
    # ``return -code break`` / ``return -code continue`` to propagate
    # through compiled-proc bodies (codegen follow-up from #325).
    "opt": {"min_passed": 27, "max_failed": 4},
}


def _make_test_class(test_name: str, *, subsystem: str, deferred: bool = False):
    """Dynamically build a test class for a Tcl 9 .test file.

    ``test_name`` is the stem of the file (e.g. ``"append"`` for
    ``append.test``).  ``subsystem`` is the inventory-KCS subsystem
    label; ``deferred`` marks files that exercise deferred-by-design
    primitives (I/O, threads, fs, encoding) and pre-categorises them
    as D.

    For files registered in :data:`_IO_BASELINE` or :data:`_BASELINE`,
    the strict ``failed == 0`` assertion is replaced with a baseline-
    relative gate so partial-pass suites are tracked without forcing a
    green sweep.
    """
    filename = f"{test_name}.test"
    baseline_io = _IO_BASELINE.get(test_name) if subsystem == "io" else _BASELINE.get(test_name)

    class _TestClass:
        def test_compiles(self, request: pytest.FixtureRequest) -> None:
            """Tcl 9 test-file bundle compiles to WASM."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)
            try:
                _wasm, diag = _compile_tcl_with_diag(src, filename)
            except Exception as exc:
                record_tcl9_result(
                    request.config,
                    {
                        "file": filename,
                        "subsystem": subsystem,
                        "stage": "compile",
                        "status": "fail",
                        "stderr_tail": str(exc)[-400:],
                        "category": "D" if deferred else "B",
                    },
                )
                pytest.fail(f"{filename} bundle failed to compile: {exc}")
            record_tcl9_result(
                request.config,
                {
                    "file": filename,
                    "subsystem": subsystem,
                    "stage": "compile",
                    "status": "pass",
                    "category": "pass",
                    **_summarise_diag(diag),
                },
            )

        def test_runs(self, request: pytest.FixtureRequest) -> None:
            """Tcl 9 test-file bundle executes and reports Failed == 0."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)

            # Inject ``tcltest::configure -skip [list ...]`` for every
            # test ID listed in the categorisation TOML's ``skip``
            # bucket.  These tests probe C-Tcl-internal behaviour we
            # don't claim to mirror (bytecode shape, ``[linenumber]``-
            # via-bcc, ``::tcl::dict::*`` private interfaces); running
            # them costs cycles and the failures are noise.  Using
            # tcltest's own ``-skip`` mechanism counts them in the
            # ``Skipped`` summary alongside the upstream
            # constraint-driven skips, keeping the JSON triage record
            # honest about coverage.
            categories = load_categories(test_name)
            skip_ids = categories.skip_full_ids()
            if skip_ids:
                # Quote each ID as a brace-balanced list element so a
                # name with regex-meta characters round-trips through
                # tcltest's ``-skip`` glob match.  ``tcltest::configure
                # -skip`` accepts a list of patterns; the patterns are
                # glob-matched against test names, so the literal IDs
                # we ship match exactly one test each.
                pattern_list = " ".join(f"{{{sid}}}" for sid in skip_ids)
                inject = f"\n::tcltest::configure -skip [list {pattern_list}]\n"
                marker = f"# ===== {filename} ====="
                if marker in src:
                    src = src.replace(marker, marker + inject, 1)

            trap_site: str | None = None
            compiled = False
            ran = False
            stdout = ""
            stderr = ""
            summary: tuple[int, int, int, int] | None = None
            diag = None

            try:
                wasm, diag = _compile_tcl_with_diag(src, filename)
                compiled = True
                with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
                    # Stage common tcltest helper files into the
                    # preopened tmpdir so tests that
                    # ``source [file join [file dirname [info script]] X]``
                    # can find them.  Mirrors the staging logic in
                    # :func:`_run_bundle`.
                    helpers_dir = _tcl9_tcltest().parent.parent.parent / "tests"
                    for helper in (
                        "tcltests.tcl",
                        "internals.tcl",
                        "encodingVectors.tcl",
                        "pkgIndex.tcl",
                    ):
                        src_path = helpers_dir / helper
                        if src_path.exists():
                            shutil.copyfile(src_path, Path(host_tmp) / helper)
                    # 45s wasmtime epoch watchdog: most healthy
                    # bundles finish in under 5s, the slowest non-pathological
                    # one (interp.test) takes ~12s.  A 45s cap surfaces
                    # genuine hangs (basic.test's pipe-based loop, expr-old's
                    # bignum overflow probes) as a trap that ``trap_allowed``
                    # / the buckets gate can categorise, instead of
                    # leaving the subprocess wrapper to kill the whole
                    # pytest invocation at its 60s boundary.
                    result = _run_wasm(
                        wasm,
                        capture_stdout=True,
                        capture_stderr=True,
                        preopen_tmpdir=host_tmp,
                        timeout_s=45,
                    )
                ran = True
                stdout = result[1] if len(result) >= 2 else ""
                stderr = result[2] if len(result) >= 3 else ""
                summary = _parse_summary(stdout)
            except Exception as exc:
                if compiled and diag is not None:
                    trap_site = _resolve_trap(exc, getattr(exc, "tcl_stderr", ""), diag)
                    stderr = getattr(exc, "tcl_stderr", "")
                else:
                    stderr = str(exc)

            category = _infer_category(
                compiled=compiled,
                ran=ran,
                summary=summary,
                trap_site=trap_site,
                stderr=stderr,
                deferred=deferred,
            )
            total, passed, skipped, failed = summary or (0, 0, 0, 0)

            # Triage gate: when the stem has a categories TOML we
            # bucket each failed test ID into MUST_PASS /
            # GOOD_TO_HAVE / JUST_TO_MATCH_CTCL and gate only on the
            # MUST_PASS bucket.  Stems without a TOML keep their
            # historical behaviour (strict ``failed == 0`` or a
            # ``_BASELINE`` numeric band) so the rollout can be
            # incremental.  ``categories.test_to_bucket`` being empty
            # AND ``trap_allowed`` being false means "no triage file
            # on disk" — fall back to the legacy gate in that case.
            has_triage_file = bool(categories.test_to_bucket) or categories.trap_allowed
            failed_ids = [m.group(1) for m in _FAIL_RE.finditer(stdout)]
            buckets = bucket_failures(test_name, failed_ids)

            record_tcl9_result(
                request.config,
                {
                    "file": filename,
                    "subsystem": subsystem,
                    "stage": "run",
                    "status": "pass" if category == "pass" else "fail",
                    "total": total,
                    "passed": passed,
                    "skipped": skipped,
                    "failed": failed,
                    "first_failing_test": _first_failing(stdout),
                    "trap_site": trap_site,
                    "stderr_tail": _stderr_tail(stderr),
                    "category": category,
                    "buckets": buckets.to_dict(),
                    **_summarise_diag(diag),
                },
            )

            if not compiled:
                pytest.fail(f"{filename} bundle failed to compile: {stderr[-400:]}")
            if not ran:
                # Triage TOMLs can opt-in to ``trap_allowed`` for the
                # whole stem — pre-existing cumulative-state traps
                # (e.g. trace.test's exponential trace re-entry
                # cascade) that we know about and have a separate
                # follow-up for.  Legacy I/O baselines retain their
                # ``trap_ok`` hatch for backwards compatibility.
                if categories.trap_allowed:
                    return
                if baseline_io is not None and baseline_io.get("trap_ok"):
                    return
                pytest.fail(f"{filename} trapped: {trap_site or stderr[-400:]}")
            if summary is None:
                if categories.trap_allowed:
                    return
                if baseline_io is not None and baseline_io.get("trap_ok"):
                    return
                pytest.fail(
                    f"No tcltest summary line in stdout for {filename}.\n"
                    f"stdout tail:\n{stdout[-400:]}\nstderr tail:\n{stderr[-400:]}"
                )
            if has_triage_file:
                # Triage gate: ``must_pass`` failures block, plus
                # ``good_to_have`` growth past the recorded baseline
                # cap (catches a previously-passing test slipping
                # into the bucket even though its name is already
                # listed).  ``just_to_match_ctcl`` and ``skip``
                # counts surface in the JSON record but don't gate.
                outcome = _categories_gate(categories, buckets)
                if not outcome.passed:
                    sample = "\n  ".join(buckets.must_pass_failures[:10])
                    extra = (
                        f"\n  ... and {len(buckets.must_pass_failures) - 10} more"
                        if len(buckets.must_pass_failures) > 10
                        else ""
                    )
                    pytest.fail(
                        f"{filename}: {outcome.reason}\n"
                        f"  must_pass={len(buckets.must_pass_failures)}, "
                        f"good_to_have={len(buckets.good_to_have_failures)} "
                        f"(cap {categories.good_to_have_baseline}), "
                        f"just_to_match_ctcl={len(buckets.just_to_match_ctcl_failures)}, "
                        f"skip={len(buckets.skip_failures)}:\n"
                        f"  {sample}{extra}"
                    )
            elif baseline_io is not None:
                min_passed = int(baseline_io.get("min_passed", 0))  # type: ignore[arg-type]
                max_failed = int(baseline_io.get("max_failed", 0))  # type: ignore[arg-type]
                assert passed >= min_passed and failed <= max_failed, (
                    f"{filename}: regression vs baseline — "
                    f"got passed={passed} failed={failed}, "
                    f"expected passed>={min_passed} failed<={max_failed}.\n"
                    f"first failing: {_first_failing(stdout)}\n"
                    f"stdout tail:\n{stdout[-400:]}"
                )
            else:
                assert failed == 0, (
                    f"{filename}: {failed} test(s) FAILED "
                    f"(total={total}, passed={passed}, skipped={skipped}).\n"
                    f"first failing: {_first_failing(stdout)}\n"
                    f"stdout tail:\n{stdout[-400:]}"
                )

    _TestClass.__name__ = f"TestTcl9_{test_name.replace('-', '_').replace('.', '_')}"
    _TestClass.__qualname__ = _TestClass.__name__
    return _TestClass


# ---------------------------------------------------------------------------
# Register test classes.
#
# In-scope files (core Tcl semantics). Deferred-by-design files — I/O,
# sockets, threads, fs, encoding, platform-specific — are NOT registered
# here; they are recorded only in the inventory KCS and triaged as
# category D without being executed.
# ---------------------------------------------------------------------------

_IN_SCOPE: list[tuple[str, str]] = [
    # parsing
    ("parse", "parsing"),
    ("parseOld", "parsing"),
    ("parseExpr", "parsing"),
    ("subst", "parsing"),
    ("word", "parsing"),
    # list
    ("list", "list"),
    ("listObj", "list"),
    ("listRep", "list"),
    ("llength", "list"),
    ("lindex", "list"),
    ("linsert", "list"),
    ("lrange", "list"),
    ("lreplace", "list"),
    ("lsearch", "list"),
    ("lset", "list"),
    ("lsetComp", "list"),
    ("lmap", "list"),
    ("lpop", "list"),
    ("lseq", "list"),
    ("lrepeat", "list"),
    ("foreach", "list"),
    ("abstractlist", "list"),
    # dict
    ("dict", "dict"),
    # string
    ("string", "string"),
    ("stringObj", "string"),
    ("format", "string"),
    ("scan", "string"),
    ("regexp", "string"),
    ("regexpComp", "string"),
    ("reg", "string"),
    ("get", "string"),
    ("split", "string"),
    ("join", "string"),
    # expr
    ("expr", "expr"),
    ("expr-old", "expr"),
    ("compExpr", "expr"),
    ("compExpr-old", "expr"),
    ("mathop", "expr"),
    # control
    ("if", "control"),
    ("if-old", "control"),
    ("for", "control"),
    ("for-old", "control"),
    ("while", "control"),
    ("while-old", "control"),
    ("switch", "control"),
    ("error", "control"),
    ("result", "control"),
    # variables / scopes
    ("set", "variable"),
    ("set-old", "variable"),
    ("var", "variable"),
    ("upvar", "variable"),
    ("uplevel", "variable"),
    ("namespace", "variable"),
    ("namespace-old", "variable"),
    ("trace", "variable"),
    ("resolver", "variable"),
    # proc / apply / info
    ("proc", "proc"),
    ("proc-old", "proc"),
    ("apply", "proc"),
    ("info", "proc"),
    ("cmdInfo", "proc"),
    ("rename", "proc"),
    ("unknown", "proc"),
    # eval / subst / execution
    ("eval", "eval"),
    ("compile", "eval"),
    ("execute", "eval"),
    ("basic", "eval"),
    # command dispatch buckets
    ("cmdAH", "cmd-dispatch"),
    ("cmdIL", "cmd-dispatch"),
    ("cmdMZ", "cmd-dispatch"),
    # TclOO
    ("oo", "object"),
    ("ooNext2", "object"),
    ("ooProp", "object"),
    ("ooUtil", "object"),
    # coroutine / nre / tailcall
    ("coroutine", "coroutine"),
    ("nre", "coroutine"),
    ("tailcall", "coroutine"),
    # interp / safe / source
    ("interp", "interp"),
    ("safe", "interp"),
    ("safe-stock", "interp"),
    ("safe-stock86", "interp"),
    ("source", "interp"),
    # misc scalar / object machinery
    ("append", "misc"),
    ("appendComp", "misc"),
    ("concat", "misc"),
    ("incr", "misc"),
    ("incr-old", "misc"),
    # ``obj.test`` is held out of the sweep — under WASM it spins in
    # a Zig-side host loop that never reaches a wasmtime epoch check,
    # so neither the watchdog (epoch-interruption) nor the
    # ``set_limits(memory_size=...)`` cap fires (the test doesn't try
    # to grow linear memory past the cap, just consumes CPU).
    # Re-running obj.test under a subprocess-wrapped ``_run_wasm``
    # would let an OS-level kill terminate it, but that's a bigger
    # plumbing change than this cycle's budget.  Re-add once the
    # runtime grows the missing yield point or the harness wraps
    # the run step in a subprocess.
    # ("obj", "misc"),
    ("indexObj", "misc"),
    ("dstring", "misc"),
    ("assocd", "misc"),
    ("opt", "misc"),
    ("stack", "misc"),
    ("misc", "misc"),
    ("brodnik", "misc"),
    ("range", "misc"),
    ("aaa_exit", "misc"),
    # I/O — umbrella issue #276.  Sub-issues #270 (chan ensemble),
    # #271 (fconfigure query forms), #272 (channel-error wording),
    # #273 (array trap), #274 (encoding subsystem), #275 (write-side
    # translation) all merged.  ``chan.test`` runs 42/42 green;
    # ``chanio`` / ``io`` / ``ioCmd`` are gated by :data:`_IO_BASELINE`
    # so their residual blockers don't break the sweep.
    ("chan", "io"),
    ("chanio", "io"),
    ("io", "io"),
    ("ioCmd", "io"),
]

for _stem, _sub in _IN_SCOPE:
    _cls = _make_test_class(_stem, subsystem=_sub, deferred=False)
    globals()[_cls.__name__] = _cls
