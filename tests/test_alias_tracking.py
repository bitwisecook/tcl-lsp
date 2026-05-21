"""Cross-tool tracking of variable aliasing (upvar / namespace upvar / global / variable).

An alias makes two *names* refer to one storage cell.  These tests pin that the
alias is tracked consistently everywhere it matters:

* the memory-SSA alias lattice — names sharing one caller cell merge into one
  set (``upvar 1 x a; upvar 1 x b`` puts ``a`` and ``b`` together);
* the lifecycle diagnostics — a write through an alias escapes the frame, so it
  is neither a dead store (W220) nor a set-but-never-used variable (W211);
* taint propagation — taint written through one alias name is observable through
  another name that shares the same storage;
* the LSP references / rename providers — occurrences stay scoped to the local
  alias name and never bleed into the differently-named caller variable.

Regression coverage for three correctness fixes made together:

1. ``memory_ssa.compute_aliases`` did not merge ``a`` and ``b`` for
   ``upvar 1 x a; upvar 1 x b`` even though both alias caller var ``x`` (the
   caller-side node encoded the local name, so the two pairs never shared a
   union-find node).
2. ``set`` of an ``upvar`` / ``global`` / ``variable`` / ``namespace upvar``
   alias was wrongly flagged W211/W220 — the lifecycle pass did not know the
   name escaped the local frame.
3. taint did not flow across intra-procedural aliases at all.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis import analyse
from core.compiler.cfg import build_cfg
from core.compiler.lowering import lower_to_ir
from core.compiler.memory_ssa import build_memory_ssa
from core.compiler.ssa import build_ssa
from core.compiler.taint import find_taint_warnings
from lsp.features.references import get_references
from lsp.features.rename import get_rename_edits, prepare_rename

TEST_URI = "file:///alias.tcl"


def _mem(source: str, proc: str):
    """Build memory-SSA for the first procedure whose qname contains *proc*."""
    cfg = build_cfg(lower_to_ir(source))
    for qname, fn in cfg.procedures.items():
        if proc in qname:
            return build_memory_ssa(build_ssa(fn))
    raise AssertionError(f"proc {proc!r} not found in {list(cfg.procedures)}")


def _codes(source: str) -> set[str]:
    return {d.code for d in analyse(source).diagnostics}


def _taint_codes(source: str) -> list[str]:
    return sorted(w.code for w in find_taint_warnings(source))


def _pos(source: str, needle: str, occurrence: int = 0) -> tuple[int, int]:
    """Return (line, character) one char into the *occurrence*-th *needle*.

    The +1 lands the cursor inside the token rather than on its left boundary,
    matching how an editor reports a click.
    """
    idx = -1
    for _ in range(occurrence + 1):
        idx = source.index(needle, idx + 1)
    before = source[:idx]
    line = before.count("\n")
    character = idx - (before.rfind("\n") + 1)
    return line, character + 1


# Memory-SSA alias lattice


class TestAliasLatticeMerge:
    def test_two_upvars_to_same_caller_merge(self):
        # Both ``a`` and ``b`` alias caller var ``x`` — a write through ``a`` is
        # visible through ``b``, so they must be one alias set.  Regression for
        # the caller-node-keyed-on-local bug that split them.
        mem = _mem("proc f {} { upvar 1 x a\n upvar 1 x b\n set a 1\n set b 2 }", "f")
        assert len(mem.alias_sets) == 1
        names = mem.alias_sets[0].names
        assert {"a", "b", "x"} <= names

    def test_two_upvars_may_alias_each_other(self):
        mem = _mem("proc f {} { upvar 1 x a\n upvar 1 x b\n set a 1 }", "f")
        assert mem.may_alias("a", "b")
        assert mem.may_alias("b", "a")

    def test_upvar_pair_may_alias_is_symmetric(self):
        mem = _mem("proc f {} { upvar 1 caller local\n set local 1 }", "f")
        assert mem.may_alias("caller", "local")
        assert mem.may_alias("local", "caller")

    def test_independent_callers_stay_separate(self):
        # ``a`` aliases ``x``; ``b`` aliases ``y`` — different caller cells, so
        # they must NOT be merged (the fix must not over-merge).
        mem = _mem("proc f {} { upvar 1 x a\n upvar 1 y b\n set a 1\n set b 2 }", "f")
        assert not mem.may_alias("a", "b")
        assert len(mem.alias_sets) == 2


# Lifecycle diagnostics: an aliased write escapes the frame


class TestAliasEscapesLifecycle:
    def test_upvar_write_only_not_unused_or_dead(self):
        # ``set x 5`` mutates the caller's variable — a real side effect, not a
        # dead store / unused local.
        codes = _codes("proc f {} { upvar 1 caller x\n set x 5 }")
        assert "W211" not in codes
        assert "W220" not in codes

    def test_global_write_only_not_unused_or_dead(self):
        codes = _codes("proc f {} { global g\n set g 5 }")
        assert "W211" not in codes
        assert "W220" not in codes

    def test_variable_write_only_not_unused_or_dead(self):
        codes = _codes("proc f {} { variable v\n set v 5 }")
        assert "W211" not in codes
        assert "W220" not in codes

    def test_namespace_upvar_write_only_not_unused_or_dead(self):
        # ``namespace upvar`` lowers to IR command ``namespace`` (not a name in
        # the alias-command set), so a naive command-name guard misses it.  The
        # var_scoping grammar still recognises ``dst`` as an alias.
        codes = _codes("proc f {} { namespace upvar ::ns src dst\n set dst 5 }")
        assert "W211" not in codes
        assert "W220" not in codes

    def test_plain_local_still_flagged(self):
        # Control: the escape suppression must not silence a genuinely unused
        # local.
        codes = _codes("proc f {} { set y 5 }")
        assert "W211" in codes
        assert "W220" in codes

    def test_aliased_overwrite_is_not_dead_store(self):
        # ``set a 1`` is read through the aliased name ``b`` — not dead even
        # though there is no read of ``a`` itself.
        codes = _codes("proc f {} { upvar 1 c a\n upvar 1 c b\n set a 1\n return $b }")
        assert "W220" not in codes


# Taint propagation through aliases


class TestTaintThroughAlias:
    def test_taint_flows_between_aliases_of_same_caller(self):
        # ``a`` and ``b`` are the same storage; taint written through ``a`` must
        # be observed at the ``eval $b`` sink.
        src = (
            "proc f {} {\n"
            "    upvar 1 c a\n"
            "    upvar 1 c b\n"
            "    set a [HTTP::payload]\n"
            "    eval $b\n"
            "}\n"
        )
        assert "T100" in _taint_codes(src)

    def test_untainted_alias_does_not_warn(self):
        # Control: a constant flowing through the alias must not raise a taint
        # sink warning (the closure must not invent taint).
        src = (
            'proc f {} {\n    upvar 1 c a\n    upvar 1 c b\n    set a "constant"\n    eval $b\n}\n'
        )
        assert "T100" not in _taint_codes(src)

    def test_direct_sink_still_warns(self):
        # Baseline that does not depend on aliasing at all.
        assert "T100" in _taint_codes("set x [HTTP::payload]\neval $x")


# LSP references — scoped to the local alias name


class TestReferencesAlias:
    SOURCE = (
        "proc fixed {} {\n"
        "    upvar 1 caller_x alias_x\n"
        "    set alias_x 5\n"
        "    incr alias_x\n"
        "    return $alias_x\n"
        "}\n"
        "set caller_x 0\n"
    )

    def test_references_cover_all_local_occurrences(self):
        # Cursor on the ``$alias_x`` read; references must include the upvar
        # declaration's local-name token plus every use within the proc.
        line, char = _pos(self.SOURCE, "$alias_x")
        refs = get_references(self.SOURCE, TEST_URI, line, char + 1)  # past the '$'
        ref_lines = sorted(loc.range.start.line for loc in refs)
        # decl (line 1), set (line 2), incr (line 3), read (line 4)
        assert ref_lines == [1, 2, 3, 4]

    def test_references_do_not_include_caller_variable(self):
        # The caller variable ``caller_x`` lives on line 6 and has a different
        # name — it must never appear among the alias's references.
        line, char = _pos(self.SOURCE, "$alias_x")
        refs = get_references(self.SOURCE, TEST_URI, line, char + 1)
        assert all(loc.range.start.line != 6 for loc in refs)

    def test_same_named_alias_in_other_proc_is_separate(self):
        source = (
            "proc a {} { upvar 1 src x\n set x 1\n return $x }\n"
            "proc b {} { upvar 1 src x\n set x 2\n return $x }\n"
        )
        # ``$x`` read inside proc a (first occurrence) resolves only to proc a.
        line, char = _pos(source, "$x", 0)
        refs = get_references(source, TEST_URI, line, char + 1)
        ref_lines = {loc.range.start.line for loc in refs}
        # proc a spans lines 0-2; proc b lines 3-5.
        assert ref_lines and ref_lines <= {0, 1, 2}


# LSP rename — rewrites the local name only, never the caller variable


class TestRenameAlias:
    SOURCE = (
        "proc fixed {} {\n"
        "    upvar 1 caller_x alias_x\n"
        "    set alias_x 5\n"
        "    return $alias_x\n"
        "}\n"
        "set caller_x 0\n"
    )

    def test_prepare_rename_targets_local_name(self):
        line, char = _pos(self.SOURCE, "$alias_x")
        placeholder = prepare_rename(self.SOURCE, TEST_URI, line, char + 1)
        assert placeholder is not None
        assert placeholder.placeholder == "alias_x"

    def test_rename_rewrites_local_occurrences(self):
        line, char = _pos(self.SOURCE, "$alias_x")
        edit = get_rename_edits(self.SOURCE, TEST_URI, line, char + 1, "renamed")
        assert edit is not None
        new_texts = [e.new_text for e in edit.changes[TEST_URI]]
        # The declaration + ``set`` write rename to the bare name; the ``$``
        # read keeps its sigil.
        assert "renamed" in new_texts
        assert "$renamed" in new_texts

    def test_rename_does_not_touch_caller_variable(self):
        line, char = _pos(self.SOURCE, "$alias_x")
        edit = get_rename_edits(self.SOURCE, TEST_URI, line, char + 1, "renamed")
        assert edit is not None
        # No edit may land on line 5 where ``caller_x`` is defined: the caller
        # variable has a different name and must be left intact.
        assert all(e.range.start.line != 5 for e in edit.changes[TEST_URI])
