"""Cross-check every W230/W231/W232 trigger against real Tcl 9.0.3.

For each diagnostic case, the test runs the same Tcl snippet through
``tclsh`` (9.0.3, built from source) and asserts that the runtime
behaviour matches the category we claim:

* **W230** — silent (empty return / clamped result / no-op).
* **W231** — runtime error raised by Tcl.
* **W232** — silent (empty / clamped).

If ``tclsh9.0`` is not available the tests are skipped.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse


def _is_tcl_9(tclsh_path: str) -> bool:
    """Return True if *tclsh_path* reports a 9.x patchlevel."""
    try:
        proc = subprocess.run(
            [tclsh_path],
            input="puts [info patchlevel]",
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        return False
    return proc.returncode == 0 and proc.stdout.strip().startswith("9.")


def _find_tcl() -> str | None:
    # Accept only a Tcl 9.x interpreter.  Older binaries (8.6) have
    # different index and loop semantics, so they would make this
    # cross-check unsound.
    candidates = [
        shutil.which("tclsh9.0"),
        shutil.which("tclsh"),
        "/tmp/tcl9-full/unix/tclsh",
    ]
    for candidate in candidates:
        if candidate and Path(candidate).exists() and _is_tcl_9(candidate):
            return str(candidate)
    return None


TCLSH = _find_tcl()
pytestmark = pytest.mark.skipif(
    TCLSH is None,
    reason="Tcl 9.0 tclsh not installed — skipping cross-check.",
)


def _run_tcl(script: str) -> tuple[str, str]:
    """Run *script* via tclsh.  Return ``(kind, result)``.

    ``kind`` is one of:

    * ``"error"`` — the interpreter raised a Tcl error.
    * ``"empty"`` — the script returned the empty string.
    * ``"ok"``    — the script returned a non-empty value.
    """
    assert TCLSH is not None
    wrapped = (
        "if {[catch {" + script + '} res]} {puts stdout "ERR: $res"} else {puts stdout "OK: $res"}'
    )
    proc = subprocess.run(
        [TCLSH],
        input=wrapped,
        capture_output=True,
        text=True,
        timeout=5,
    )
    out = proc.stdout.strip()
    if out.startswith("ERR:"):
        return "error", out[4:].strip()
    if out == "OK:":
        return "empty", ""
    return "ok", out[3:].strip() if out.startswith("OK:") else out


def _fires(source: str, code: str) -> bool:
    result = analyse(source)
    return any(d.code == code for d in result.diagnostics)


# W230: silent-empty / no-op list operations


@pytest.mark.parametrize(
    "snippet,setup",
    [
        ("lindex {a b c} 5", ""),
        ("lindex {a b c} -1", ""),
        ("lindex {a b c} end-5", ""),
        ("lindex {a b c} end+2", ""),
        ("lrange {a b c} 10 20", ""),
        ("lrange {a b c} end-99 -50", ""),
        ("lreplace {a b c} end-99 end-99 X", ""),
        ("lreplace {a b c} 10 20 X", ""),
    ],
)
def test_w230_runtime_behaviour_is_silent(snippet: str, setup: str):
    """W230 must fire and the Tcl runtime must be silent (not error)."""
    assert _fires(snippet, "W230"), f"W230 should fire on: {snippet}"
    script = (setup + "\n" + snippet) if setup else snippet
    kind, _ = _run_tcl(script)
    assert kind in ("empty", "ok"), (
        f"Expected silent behaviour from Tcl 9.0.3 for {snippet!r}, got {kind}"
    )


# W231: lset must actually raise a runtime error


@pytest.mark.parametrize(
    "snippet,setup",
    [
        ("lset L -1 X", "set L {a b c}"),
        ("lset L -100 X", "set L {a b c}"),
        ("lset L 5 X", "set L {a b c}"),
        ("lset L end-2 X", "set L {a}"),
        ("lset L end+2 X", "set L {a b c}"),
    ],
)
def test_w231_runtime_raises_error(snippet: str, setup: str):
    full = setup + "\n" + snippet
    assert _fires(full, "W231"), f"W231 should fire on: {snippet}"
    kind, msg = _run_tcl(full)
    assert kind == "error", (
        f"Expected Tcl 9.0.3 to raise for {snippet!r}, got kind={kind} msg={msg!r}"
    )
    assert "out of range" in msg.lower()


# Negative: lset append (end+1, == length) must NOT fire and must NOT error.


@pytest.mark.parametrize(
    "snippet,setup",
    [
        ("lset L end+1 X", "set L {a b c}"),
        ("lset L 3 X", "set L {a b c}"),
        ("lset L end+0 X", "set L {a}"),
    ],
)
def test_w231_negative_append_positions(snippet: str, setup: str):
    full = setup + "\n" + snippet
    assert not _fires(full, "W231"), f"W231 should NOT fire on legal append: {snippet}"
    kind, msg = _run_tcl(full)
    assert kind != "error", f"Tcl 9.0.3 should not error on append: {snippet!r}, got {msg!r}"


# W232: silent-empty / no-op string operations


@pytest.mark.parametrize(
    "snippet",
    [
        "string index abc 10",
        "string index abc -1",
        "string index abc end-5",
        "string range abc 10 20",
        "string range abc end-99 -50",
        "string replace abc -1 -1 X",
        "string replace abc 10 20 X",
        "string insert abc -5 X",
    ],
)
def test_w232_runtime_behaviour_is_silent(snippet: str):
    assert _fires(snippet, "W232"), f"W232 should fire on: {snippet}"
    kind, _ = _run_tcl(snippet)
    assert kind in ("empty", "ok"), (
        f"Expected silent behaviour from Tcl 9.0.3 for {snippet!r}, got {kind}"
    )


# Negative: in-range indices must not fire and must produce a real result.


@pytest.mark.parametrize(
    "snippet,expected",
    [
        ("lindex {a b c} 0", "a"),
        ("lindex {a b c} end", "c"),
        ("lindex {a b c} end-2", "a"),
        ("lrange {a b c} 0 2", "a b c"),
        ("string index abc 0", "a"),
        ("string index abc end", "c"),
        ("string range abc 0 2", "abc"),
    ],
)
def test_in_range_no_diagnostic(snippet: str, expected: str):
    assert not _fires(snippet, "W230"), f"W230 false-fired on: {snippet}"
    assert not _fires(snippet, "W232"), f"W232 false-fired on: {snippet}"
    kind, val = _run_tcl(snippet)
    assert kind == "ok" and val == expected


# W240: loop constant-false condition — confirm Tcl does not enter body.


def test_w240_body_never_executes():
    source = "while {0} { error BAD }"
    assert _fires(source, "W240")
    kind, _ = _run_tcl(source)
    # `while {0}` evaluates to empty string with no side effect.
    assert kind == "empty"


def test_w240_body_never_executes_false_keyword():
    source = "while {false} { error BAD }"
    assert _fires(source, "W240")
    kind, _ = _run_tcl(source)
    assert kind == "empty"


# W241: provably-infinite loop.  We can't run these safely — instead we
# verify that the same for-loop shape with a correct step terminates,
# and that Tcl never errors on the wrong-direction form (proving that
# infinity is the real runtime behaviour).


def test_w241_correct_for_loop_terminates():
    source = "for {set i 0} {$i < 10} {incr i} { }; set i"
    assert not _fires(source, "W241")
    kind, val = _run_tcl(source)
    assert kind == "ok" and val == "10"


def test_w241_wrong_direction_would_infloop():
    # We CAN'T run `for {set i 0} {$i < 10} {incr i -1} {}` — it hangs.
    # But we can verify the step's arithmetic: after one body the counter
    # moves further from the bound.
    source = "set i 0; incr i -1; set i"
    kind, val = _run_tcl(source)
    assert kind == "ok" and val == "-1"
    # And the original for-loop is flagged.
    assert _fires(
        "for {set i 0} {$i < 10} {incr i -1} { puts hi }",
        "W241",
    )


# Rigorous W241 validation: confirm the flagged loops *really do* run
# forever, by running them in tclsh under a short subprocess timeout.
# We pair each infinite case with a correct counterpart that terminates
# quickly, so the timeout harness itself is trustworthy.


def _tcl_terminates(script: str, *, timeout_s: float = 1.2) -> bool:
    """Return True if tclsh returns before the timeout."""
    assert TCLSH is not None
    try:
        subprocess.run(
            [TCLSH],
            input=script,
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
        return True
    except subprocess.TimeoutExpired:
        return False


@pytest.mark.parametrize(
    "source",
    [
        "while {1} { }",
        "while {1} { set x 0 }",
        "for {set i 0} {$i < 10} {incr i 0} { }",
        "for {set i 0} {$i < 10} {incr i -1} { }",
        "for {set i 10} {$i > 0} {incr i 2} { }",
        "for {set i 0} {$i != 10} {incr i 3} { }",
    ],
)
def test_w241_flagged_loop_really_hangs(source: str):
    assert _fires(source, "W241"), f"W241 should fire on: {source}"
    assert not _tcl_terminates(source), (
        f"Tcl 9.0.3 unexpectedly terminated a loop we flagged as infinite: {source!r}"
    )


@pytest.mark.parametrize(
    "source",
    [
        "for {set i 0} {$i < 10} {incr i} { }",
        "for {set i 10} {$i > 0} {incr i -1} { }",
        "while {1} { break }",
        "while {1} { return 0 }",
        "for {set i 0} {$i < 5} {incr i 2} { }",
    ],
)
def test_w241_clean_loop_terminates(source: str):
    assert not _fires(source, "W241"), f"W241 false-fired on a terminating loop: {source}"
    assert _tcl_terminates(source), (
        f"Expected Tcl 9.0.3 to terminate {source!r} but it ran to timeout."
    )


# W242: verify that loops we flag as "termination unprovable" actually
# run forever when the external condition variable never changes.


def test_w242_flagged_loop_without_external_update_hangs():
    # The body never modifies $x, so without an external side effect
    # there's nothing to end the loop.
    source = "set x 0; while {$x < 10} { set y 1 }"
    assert _fires(source, "W242")
    assert not _tcl_terminates(source)


def test_w242_when_body_modifies_counter_terminates():
    source = "set x 0; while {$x < 10} { incr x }; set x"
    assert not _fires(source, "W242")
    assert _tcl_terminates(source)


def test_w242_upvar_indirect_modification_is_unprovable_but_terminates():
    """W242 is a hint, not a proof of non-termination.

    This loop *does* terminate: ``bump`` mutates ``x`` via ``upvar``, but
    the body scan in ``_bounds.py`` is shallow (regex over the literal
    body text) and cannot chase into a called proc.  So W242 fires on a
    correct, terminating loop — which is fine because the severity is
    HINT and the check is off by default.  The test exists to codify
    that behaviour so we do not accidentally regress it into a false
    proof of non-termination.
    """
    source = """
proc bump {} {
    upvar 1 x x
    incr x
}
set x 0
while {$x < 5} {
    bump
}
"""
    assert _fires(source, "W242"), (
        "Expected W242 to fire on upvar-indirect loop — the shallow "
        "analyser cannot see the proc-body update."
    )
    # And it genuinely terminates in Tcl 9.0.3.
    assert _tcl_terminates(source), "Tcl 9.0.3 should terminate this loop despite W242 firing."


def test_w242_command_substitution_counter_update_is_unprovable_but_terminates():
    """Another unprovable-but-terminates shape: counter updated via
    command substitution on the right-hand side of a ``set``.

    ``set x [some-proc]`` is caught because it is a literal ``set x``.
    But ``set ${dynvar} ...`` where ``dynvar`` contains the counter
    name is invisible to the scanner.
    """
    source = """
proc next {} { return [expr {[clock clicks] % 4 + 1}] }
set dynvar x
set x 0
while {$x < 3} {
    set $dynvar [next]
    if {$x >= 3} { break }
    incr x
}
"""
    # `incr x` is visible, so W242 should NOT fire here.  This test
    # mostly documents that the scanner picks up the direct update path.
    assert not _fires(source, "W242")
    assert _tcl_terminates(source)
