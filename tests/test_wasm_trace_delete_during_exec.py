"""Execution traces whose callback deletes the traced command.

When a ``trace add execution foo …`` callback runs ``rename foo {}``, Tcl
must (a) NOT fire ``foo``'s ``leave`` / ``leavestep`` traces once the
command is gone — C gates ``TEOV_RunLeaveTraces`` on
``!(cmdPtr->flags & CMD_DYING)`` — and (b) tear down every trace keyed on
the deleted command (``TclDeleteCommandFromToken`` →
``CallCommandTraces`` + trace removal) so stale callbacks don't leak onto
a later, reused command bucket.

Two prior bugs:
  * the leave callback re-ran ``rename foo {}`` on the now-gone command,
    so ``can't delete "foo": command doesn't exist`` clobbered the real
    result (the body's value, or the ``invalid command name "foo"`` from
    the failed dispatch) — trace-25.8/25.9/25.10/25.11;
  * ``rename foo {}`` left ``foo``'s enter/leave/step entries registered,
    so a redefined ``foo`` inherited phantom callbacks.

These run the bodies INTERPRETED (``eval``) so the execution-step trace
dispatch is exercised, matching how trace.test runs.  Reference: Tcl 9
``tclTrace.c`` / ``tclBasic.c``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_PRE = (
    "proc traceDelete {cmd args} { rename $cmd {}; global info; set info $args }\n"
    "proc foo {a} { set b $a }\n"
)


def _run_interp(body: str) -> str:
    src = "set __s {" + _PRE + body + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(src), capture_stdout=True)
    return stdout.rstrip("\n")


def _add(*ops: str) -> str:
    lines = "\n".join(f"trace add execution foo {op} [list traceDelete foo]" for op in ops)
    return (
        "set info {}\n" + lines + "\nputs [list [catch {foo 1} err] $err $info "
        "[catch {trace info execution foo} r] $r]"
    )


def test_25_8_enter_leave_enterstep_leavestep() -> None:
    # enter deletes foo → dispatch fails with invalid command name; the
    # leave/step traces are torn down and never re-fire.
    got = _run_interp(_add("enter", "leave", "enterstep", "leavestep"))
    assert got == '1 {invalid command name "foo"} {{foo 1} enter} 1 {unknown command "foo"}'


def test_25_9_enter_leave_leavestep() -> None:
    got = _run_interp(_add("enter", "leave", "leavestep"))
    assert got == '1 {invalid command name "foo"} {{foo 1} enter} 1 {unknown command "foo"}'


def test_25_10_leave_leavestep_self_delete() -> None:
    # No enter trace: foo runs, leavestep (on `set b 1`) deletes foo, so
    # foo's own leave must be skipped — result is the body value "1", not
    # a "can't delete" error.
    got = _run_interp(_add("leave", "leavestep"))
    assert got == '0 1 {{set b 1} 0 1 leavestep} 1 {unknown command "foo"}'


def test_25_11_enter_enterstep() -> None:
    got = _run_interp(_add("enter", "enterstep"))
    assert got == '1 {invalid command name "foo"} {{foo 1} enter} 1 {unknown command "foo"}'
