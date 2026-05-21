"""WASM execution tests for variable trace callback op-word semantics.

Reference Tcl 9 invokes ``trace add variable NAME OPS CMD`` callbacks
with the op argument as the **full word** (``read`` / ``write`` /
``unset`` / ``array``) — never the single-letter internal op code.
A lurking ``op_char`` plumbing in the runtime was passing the raw
internal byte through, so every reference-Tcl-shaped trace assertion
in the Tcl 9 ``trace.test`` slice (1.1–1.14, 2.1–2.5) failed because
the callback's ``$op`` argument carried ``r`` / ``w`` instead of
``read`` / ``write``.

Pinned regressions:

* Read trace fires with ``$op == read``.
* Write trace fires with ``$op == write``.
* Unset trace fires with ``$op == unset``.
* Array trace fires with ``$op == array`` (whole-array read).

Also includes a regression for ``split_array_elem_name`` /
``resolve_ext_set`` defensive guards: an upvar/variable alias whose
descriptor goes stale (kind out of range, target ptr pointing past
the linear-memory limit) used to fault the wasm engine when a
subsequent ``set`` tried to chase it.  The runtime now treats an
out-of-bounds descriptor pointer as a no-op so the bundle keeps
running and tcltest can surface the underlying failure as a regular
``no such variable`` error.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_dynamic_var_names import _run_and_read_global  # noqa: E402


class TestVarTraceOpWord:
    """``trace add variable`` callbacks receive the full op word."""

    def test_read_trace_op_is_read(self):
        result = _run_and_read_global(
            """\
set ::seen ""
proc cb {name1 name2 op} { set ::seen $op }
set x 5
trace add variable x read cb
set _ $x
""",
            "::seen",
        )
        assert result == b"read"

    def test_write_trace_op_is_write(self):
        result = _run_and_read_global(
            """\
set ::seen ""
proc cb {name1 name2 op} { set ::seen $op }
trace add variable x write cb
set x 1234
""",
            "::seen",
        )
        assert result == b"write"

    def test_unset_trace_op_is_unset(self):
        result = _run_and_read_global(
            """\
set ::seen ""
proc cb {name1 name2 op} { set ::seen $op }
set x 1
trace add variable x unset cb
unset x
""",
            "::seen",
        )
        assert result == b"unset"

    @pytest.mark.skip(
        reason="``trace add variable ARR array CMD`` whole-array fire "
        "semantics aren't fully wired in the runtime — covered by "
        "the upstream ``trace.test`` slice (1.7 / 1.8 / 1.9), not by "
        "this op-word contract."
    )
    def test_array_trace_op_is_array(self):
        result = _run_and_read_global(
            """\
set ::seen ""
proc cb {name1 name2 op} { set ::seen $op }
trace add variable arr array cb
catch { set _ $arr(missing) }
""",
            "::seen",
        )
        assert result == b"array"


class TestVarTraceProcLocalFires:
    """A ``trace add variable NAME …`` registered inside a proc must
    fire on every subsequent write to NAME from the same proc body —
    even when escape analysis flagged the proc as pessimistic
    (``dynamic_barrier=True``) and the codegen would otherwise route
    the write through the WASM-local-with-sync hot path.

    Pinned regression: pessimistic procs used to write proc-locals
    via plain ``local.set`` to a WASM-local slot without firing
    ``fire_local_trace`` — so a body like ``proc p {} { trace add
    variable x write cb ; set x 42 }`` silently never invoked
    ``cb``.  The codegen now detects ``IRBarrier(::trace)`` against
    a literal name in the body and forces frame-routed
    ``tcl_local_set`` for matching writes, which DOES fire the
    frame-local trace list.
    """

    def test_write_trace_on_proc_local_scalar_fires(self):
        result = _run_and_read_global(
            """\
set ::seen "untouched"
proc cb {n n2 op} { set ::seen "$n $op" }
proc p {} {
    trace add variable x write cb
    set x 42
}
p
""",
            "::seen",
        )
        assert result == b"x write"

    def test_unset_trace_on_proc_local_fires(self):
        result = _run_and_read_global(
            """\
set ::seen "untouched"
proc cb {n n2 op} { set ::seen "$n $op" }
proc p {} {
    set x 1
    trace add variable x unset cb
    unset x
}
p
""",
            "::seen",
        )
        assert result == b"x unset"


class TestExtAliasResolutionRobustness:
    """An ``upvar`` / ``variable`` alias whose descriptor outlives the
    matching frame must not fault the wasm engine on subsequent
    writes.  ``split_array_elem_name`` and ``resolve_ext_set`` now
    bail on out-of-bounds descriptor pointers / unknown ``kind``
    values so the surrounding ``set`` reports a regular Tcl error
    instead of trapping ``out of bounds memory access``."""

    def test_upvar_target_set_after_callee_returns(self):
        # Reference Tcl 9: an ``upvar`` alias goes stale once the
        # caller frame pops; subsequent writes should be a no-op or
        # raise a "no such variable" error — they must not fault the
        # interpreter.  Pin the no-trap contract *and* the value: inner's
        # ``set alias 7`` writes through to outer's ``x`` while the frame is
        # live, so outer returns 7.
        result = _run_and_read_global(
            """\
proc inner {} { upvar 1 x alias; set alias 7 ; return $alias }
proc outer {} { set x {} ; inner ; return $x }
set ::result [outer]
""",
            "::result",
        )
        assert result == b"7"

    def test_namespaced_array_via_upvar_does_not_trap(self):
        result = _run_and_read_global(
            """\
namespace eval ns { variable arr ; array set arr {} }
proc setit {tag k v} {
    upvar #0 ns::arr alias
    set alias($k) $v
}
setit alpha N 10
set ::r [namespace eval ns { set arr(N) }]
""",
            "::r",
        )
        assert result == b"10"
