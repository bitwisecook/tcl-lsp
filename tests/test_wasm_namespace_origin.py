"""WASM execution tests for ``namespace origin`` and the
``namespace forget`` tombstone behaviour.

Pinned regressions:

* ``namespace origin CMD`` returns the FQN of CMD (or, if CMD is an
  imported alias, the FQN of the underlying source).  Reference
  Tcl 9 raises ``invalid command name "X"`` when the lookup misses;
  before this contract landed, the runtime silently returned an
  empty TclObj — which then caused tcltest's
  ``[list namespace import [namespace origin Replace::puts]]``
  pattern to build ``namespace import {}`` and trap with ``empty
  import pattern``.

* ``namespace forget PAT`` not only marks the import descriptor
  dead but also tombstones the importing-namespace's cmd_table
  bucket so subsequent ``rename`` / ``namespace which`` /
  ``ns_cmd_find`` see the alias name as genuinely vacant.  Without
  this, tcltest's ``Eval`` cleanup
  (``namespace forget puts ; rename Replace::Puts ::puts``) refused
  the rename and aborted the bundle; now the cleanup completes.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_dynamic_var_names import _run_and_read_global  # noqa: E402


class TestNamespaceOrigin:
    """``namespace origin`` semantics."""

    def test_origin_of_local_proc_returns_fq_self(self):
        result = _run_and_read_global(
            """\
proc target {} { return ok }
set ::r [namespace origin target]
""",
            "::r",
        )
        assert result == b"::target"

    def test_origin_of_imported_alias_from_within_target_ns(self):
        # ``namespace eval ::dest { namespace origin work }`` inside
        # the importing namespace: relative resolution finds the
        # redirect, the unwrap loop walks ``ImportedCmdData.real_cmd``
        # to the source, and we surface the source's stored FQN.
        result = _run_and_read_global(
            """\
namespace eval ::src { proc work {} { return real } ; namespace export work }
namespace eval ::dest { namespace import ::src::work }
namespace eval ::dest { set ::r [namespace origin work] }
""",
            "::r",
        )
        assert result == b"::src::work"

    def test_tcltest_replace_puts_origin_pattern(self):
        # The exact tcltest::Eval pattern that previously trapped
        # the trace.test bundle:
        #   rename ::puts tcltest::Replace::Puts
        #   namespace eval :: [list namespace import [namespace origin Replace::puts]]
        # Used to land ``namespace import {}`` because
        # ``[namespace origin Replace::puts]`` returned empty.
        result = _run_and_read_global(
            """\
namespace eval ::tcltest {
    proc Replace::puts {args} { return fake-$args }
    set ::r [namespace origin Replace::puts]
}
""",
            "::r",
        )
        assert result == b"::tcltest::Replace::puts"


class TestNamespaceForgetTombstones:
    """``namespace forget`` removes the importing namespace's slot
    so a subsequent ``rename`` can re-bind it."""

    def test_forget_then_rename_round_trip(self):
        # The exact tcltest::Eval cleanup sequence:
        #   rename ::puts tcltest::Replace::Puts
        #   namespace import (in :: of) tcltest::Replace::puts
        #   <user code>
        #   namespace forget puts (in ::)
        #   rename tcltest::Replace::Puts ::puts
        # The last rename used to fail with "command already exists"
        # because the dead-import redirect for ``::puts`` was still
        # present in ``::``'s cmd_table.
        result = _run_and_read_global(
            """\
namespace eval ::tcltest {}
proc ::tcltest::Replace::puts {args} { return fake }
namespace eval :: [list namespace import [namespace origin ::tcltest::Replace::puts]]
namespace eval :: namespace forget puts
rename ::tcltest::Replace::puts ::resurrected_puts
set ::r [::resurrected_puts hello]
""",
            "::r",
        )
        assert result == b"fake"

    def test_forget_makes_alias_unavailable(self):
        # After forget, the alias name should not resolve; reads
        # through ``info commands`` no longer see it.
        result = _run_and_read_global(
            """\
namespace eval ::src { proc thing {} { return src-thing } ; namespace export thing }
namespace eval ::dest { namespace import ::src::thing }
namespace eval ::dest namespace forget thing
namespace eval ::dest {
    set ::r [info commands thing]
}
""",
            "::r",
        )
        assert result == b""
