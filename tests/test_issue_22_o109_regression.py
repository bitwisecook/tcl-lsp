"""Two-sided regression test for issue #22 ("Rule O109 Setting Ignored").

Issue #22 reported a false positive from the O109 optimisation under the
``synopsys-eda-tcl`` dialect.  The reporter's snippet builds an accumulator
collection with ``append_to_collection`` inside a ``foreach_in_collection``
loop and consumes it *after* the loop via ``remove_shapes $to_delete_shapes``::

    set to_delete_shapes {}
    foreach_in_collection shape $shapes {
        ...
        append_to_collection to_delete_shapes $shape
        ...
    }
    remove_shapes $to_delete_shapes

The optimiser wrongly flagged the post-loop read as an O109 redundant-copy /
dead-store candidate and proposed rewriting ``remove_shapes $to_delete_shapes``
to ``remove_shapes s`` -- a transform that silently breaks the program because
``append_to_collection`` mutates ``to_delete_shapes`` in place across loop
iterations (it is *not* a simple ``set``-then-read pair).

This test pins **both** sides:

* **fix holds** -- the accumulate-in-loop/consume-after-loop snippet produces
  no O109 and no rewrite touches the post-loop read; and
* **true positive still fires** -- a genuine dead-store/redundant-copy still
  raises O109, so the test cannot pass vacuously.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.optimiser import find_optimisations
from compiler.registry.runtime import configure_signatures


def _setup_eda_dialect() -> None:
    configure_signatures(dialect="synopsys-eda-tcl")


# Exact pattern from issue #22: accumulate via append_to_collection inside the
# loop, then consume the accumulator after the loop.
ISSUE_22_SNIPPET = textwrap.dedent("""\
    set to_delete_shapes {}
    foreach_in_collection shape $shapes {
        set layer [get_attr $shape layer_name]
        if {[lsearch $except_list $layer] == -1} {
            append_to_collection to_delete_shapes $shape
        } else {
            echo Keeping shape of layer $layer
        }
    }
    remove_shapes $to_delete_shapes
""")


class TestIssue22O109FalsePositive:
    """#22 -- O109 must not flag a collection accumulated in a loop."""

    def test_fix_holds_no_o109_on_accumulator(self):
        """The accumulate-then-consume snippet raises no O109."""
        _setup_eda_dialect()
        rewrites = find_optimisations(ISSUE_22_SNIPPET)
        o109 = [r for r in rewrites if r.code == "O109"]
        assert o109 == [], f"unexpected O109 rewrites: {[r.message for r in o109]}"

    def test_fix_holds_no_rewrite_renames_the_read(self):
        """No suggested edit may rename/remove the post-loop ``$to_delete_shapes`` read.

        The original bug proposed ``remove_shapes s``; guard against *any*
        rewrite whose replacement drops the accumulator name from the final
        ``remove_shapes`` command.
        """
        _setup_eda_dialect()
        rewrites = find_optimisations(ISSUE_22_SNIPPET)
        # The accumulator name must survive untouched: no rewrite should
        # propose a replacement that omits it while overlapping the read.
        read_line = ISSUE_22_SNIPPET.count("\n") - 1  # 0-based line of remove_shapes
        for r in rewrites:
            touches_read = r.range.start.line <= read_line <= r.range.end.line
            if touches_read:
                assert "to_delete_shapes" in (r.replacement or ""), (
                    f"rewrite {r.code} drops the accumulator name from the read: {r.replacement!r}"
                )


class TestIssue22O109TruePositiveStillFires:
    """The O109 rule must still fire on a genuine dead store/redundant copy."""

    def test_true_positive_redundant_copy_still_flagged(self):
        """A plain ``set y $x`` copy that is the sole use of ``x`` is O109."""
        _setup_eda_dialect()
        # x is set once and only copied into y; the copy is redundant and the
        # intermediate store is dead -- O109 should propose collapsing it.
        source = "set x 5\nset y $x\nputs $y\n"
        rewrites = find_optimisations(source)
        assert any(r.code == "O109" for r in rewrites), (
            "O109 no longer fires on a genuine redundant copy -- the "
            "false-positive test above could now pass vacuously"
        )
