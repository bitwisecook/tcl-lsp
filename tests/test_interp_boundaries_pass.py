"""Tests for the IRInterpBoundary insertion pass."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.ir import IRBarrier, IRInterpBoundary
from compiler.lowering import lower_to_ir
from compiler.passes.interp_boundaries import insert_interp_boundaries


def _stmts(module, qname: str = "::foo"):
    return module.procedures[qname].body.statements


def _eval_proc_ir():
    """A proc whose body lowers to an ``IRBarrier`` (dynamic-body eval).

    A literal-body eval (``eval {set x 1}``) lowers to ``IRBlock``
    rather than ``IRBarrier`` because the lowering pass relaxes
    safe shapes; an interpreter-level eval is what we want here.
    """
    src = textwrap.dedent(
        """\
        proc foo {body} {
            eval $body
        }
        """
    )
    return lower_to_ir(src)


class TestInsertion:
    def test_barrier_gets_boundary_marker(self):
        module = insert_interp_boundaries(_eval_proc_ir())
        stmts = _stmts(module)
        # First non-barrier-preserving statement should be the boundary
        # marker, followed by the original IRBarrier for the eval.
        kinds = [type(s).__name__ for s in stmts]
        assert "IRInterpBoundary" in kinds
        boundary_idx = kinds.index("IRInterpBoundary")
        assert kinds[boundary_idx + 1] == "IRBarrier"

    def test_boundary_kind_matches_barrier(self):
        module = insert_interp_boundaries(_eval_proc_ir())
        stmts = _stmts(module)
        boundary = next(s for s in stmts if isinstance(s, IRInterpBoundary))
        # eval / uplevel / source / subst all map to "eval" kind today.
        assert boundary.kind == "eval"


class TestIdempotence:
    """The pass must not duplicate boundaries on re-run.

    Codex review (PR #356) flagged that the original implementation
    unconditionally prepended a new IRInterpBoundary for every
    IRBarrier — an incremental recompile of the same module would
    accumulate ``IRInterpBoundary, IRInterpBoundary, IRBarrier``
    triples and emit redundant frame syncs.  The fix is to skip
    insertion when the immediately-preceding statement is already a
    boundary; these tests pin that contract.
    """

    def test_double_run_is_no_op(self):
        once = insert_interp_boundaries(_eval_proc_ir())
        twice = insert_interp_boundaries(once)
        # Same identity — idempotent runs should not allocate new IR.
        assert twice is once or _stmts(twice) == _stmts(once)

    def test_no_duplicate_boundaries(self):
        once = insert_interp_boundaries(_eval_proc_ir())
        twice = insert_interp_boundaries(once)
        # Walk the proc body and check no two consecutive statements
        # are both IRInterpBoundary.
        prev_was_boundary = False
        for stmt in _stmts(twice):
            is_boundary = isinstance(stmt, IRInterpBoundary)
            assert not (prev_was_boundary and is_boundary), (
                "duplicate IRInterpBoundary nodes accumulated across "
                "re-runs of insert_interp_boundaries"
            )
            prev_was_boundary = is_boundary

    def test_barriers_still_preceded_by_boundary(self):
        once = insert_interp_boundaries(_eval_proc_ir())
        twice = insert_interp_boundaries(once)
        # After re-run, every IRBarrier should still have an
        # IRInterpBoundary immediately before it.
        stmts = _stmts(twice)
        for i, stmt in enumerate(stmts):
            if isinstance(stmt, IRBarrier):
                assert i > 0 and isinstance(stmts[i - 1], IRInterpBoundary), (
                    f"IRBarrier at index {i} is not preceded by an IRInterpBoundary"
                )
