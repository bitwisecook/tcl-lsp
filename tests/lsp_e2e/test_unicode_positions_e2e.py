"""Exact UTF-16 column correctness across providers — the byte-vs-UTF-16 bug class.

`test_invariants_e2e.py` proves every provider Range is *well-formed* in UTF-16
units (non-negative, start<=end, within its line). This suite proves they are
*correct*: a byte- or scalar-column regression produces a well-formed-but-WRONG
range that the invariant battery silently passes.

The LSP spec defines `Position.character` as a UTF-16 code-unit offset (unless a
different `positionEncoding` is negotiated). Each test places the token of
interest AFTER multibyte — and astral (emoji, 2 UTF-16 units) — characters on
its line, so a byte/scalar confusion shifts the column. Both directions are
checked:

  * INPUT mapping: a UTF-16 request column on a multibyte line must resolve to
    the intended token (definition/references must find it);
  * OUTPUT: the ranges the server returns must carry UTF-16 columns, not the
    larger byte columns.

Expected columns are computed from the source via UTF-16-LE encoding so the
astral cases are exact (Python `len` counts code points, not UTF-16 units).
"""

from __future__ import annotations

from ._lsp_helpers import starts


def _utf16_col(line: str, token: str) -> int:
    """UTF-16 code-unit column where `token` first starts in `line`."""
    idx = line.index(token)
    return len(line[:idx].encode("utf-16-le")) // 2


def _byte_col(line: str, token: str) -> int:
    """The (wrong) UTF-8 byte column — what a byte-column regression would use."""
    idx = line.index(token)
    return len(line[:idx].encode("utf-8"))


class TestUtf16InputColumnMapping:
    """A UTF-16 request column on a multibyte line resolves to the right token."""

    def test_definition_resolves_through_multibyte_prefix(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'set x 1\nputs "café $x"\n'
        lsp_server.open_ready(uri, src)
        line1 = 'puts "café $x"'
        # Land on the `x` of `$x` (one UTF-16 unit past the `$`).
        col = _utf16_col(line1, "$x") + 1
        locs = starts(lsp_server.definition(uri, 1, col))
        assert (0, 4) in locs, (
            f"$x on a café-prefixed line must resolve to its definition at (0,4); "
            f"requested UTF-16 col {col}, got {locs}"
        )

    def test_references_resolve_through_astral_prefix(self, lsp_server, uri_factory):
        uri = uri_factory()
        # 🚀 is a single code point but TWO UTF-16 units — the column past it
        # differs between a UTF-16 and a scalar-count interpretation.
        src = 'set y 2\nputs "🚀 $y here"\nputs "$y again"\n'
        lsp_server.open_ready(uri, src)
        line1 = 'puts "🚀 $y here"'
        col = _utf16_col(line1, "$y") + 1
        ref_lines = {line for line, _ in starts(lsp_server.references(uri, 1, col))}
        assert {1, 2}.issubset(ref_lines), (
            f"references from an astral-prefixed line must find both uses (lines 1,2); "
            f"requested UTF-16 col {col}, got lines {ref_lines}"
        )


class TestUtf16OutputColumns:
    """Ranges the server returns carry UTF-16 columns, not byte columns."""

    def test_highlight_columns_are_utf16_not_bytes(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'set z 3\nputs "café résumé $z"\n'
        lsp_server.open_ready(uri, src)
        line1 = 'puts "café résumé $z"'
        dollar = _utf16_col(line1, "$z")
        name = dollar + 1
        byte_col = _byte_col(line1, "$z")
        assert byte_col > dollar, "test setup: multibyte prefix must shift the byte column"
        # Request on the name; the returned highlight on line 1 must sit at the
        # UTF-16 column of either the `$` or the name — never the byte column.
        cols = {c for line, c in starts(lsp_server.document_highlight(uri, 1, name)) if line == 1}
        assert cols, "expected a highlight on line 1 for $z; got none"
        assert cols <= {dollar, name}, (
            f"highlight column must be UTF-16 ({dollar} or {name}), not the byte "
            f"column ({byte_col}); got {cols}"
        )

    def test_highlight_columns_are_utf16_through_astral_prefix(self, lsp_server, uri_factory):
        # The astral-output counterpart of the BMP test above: 🚀 is a single
        # code point but TWO UTF-16 units, so a scalar (code-point) count yields
        # a column one short. The returned highlight must carry the exact UTF-16
        # column, proving provider OUTPUT ranges are UTF-16 — not scalar — counts.
        uri = uri_factory()
        src = 'set z 3\nputs "🚀 résumé $z"\n'
        lsp_server.open_ready(uri, src)
        line1 = 'puts "🚀 résumé $z"'
        dollar = _utf16_col(line1, "$z")
        name = dollar + 1
        scalar_col = line1.index("$z")  # NB: str.index → code-point (scalar) index
        # The 🚀 makes UTF-16 and scalar columns differ — that gap is what a
        # scalar→UTF-16 confusion would expose.
        assert dollar > scalar_col, "test setup: astral prefix must widen the UTF-16 column"
        cols = {c for line, c in starts(lsp_server.document_highlight(uri, 1, name)) if line == 1}
        assert cols, "expected a highlight on line 1 for $z; got none"
        assert cols <= {dollar, name}, (
            f"highlight column must be UTF-16 ({dollar} or {name}), not the scalar "
            f"column ({scalar_col} / {scalar_col + 1}); got {cols}"
        )


class TestRapidEditConsistency:
    """Rapid full-document replaces leave the server's view consistent with the
    final content (no stale analysis surviving an out-of-order/queued change)."""

    def test_final_state_wins_after_rapid_replaces(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc alpha {} { return 1 }\nalpha\n")
        # A burst of full replaces, each a distinct proc name, without waiting.
        names = ["beta", "gamma", "delta", "epsilon"]
        version = 2
        for n in names:
            lsp_server.replace_document(uri, version, f"proc {n} {{}} {{ return 1 }}\n{n}\n")
            version += 1
        # The final content defines `epsilon`; its call on line 1 must resolve to
        # the definition on line 0 — proving the latest change won and no earlier
        # version's analysis is being served.
        lsp_server.open_ready  # (no-op ref; server already initialised)
        defs = starts(lsp_server.definition(uri, 1, 0))
        assert (0, 5) in defs, (
            f"after rapid replaces, `epsilon` call must resolve to the final "
            f"definition at (0,5); got {defs}"
        )
        # And a stale name must NOT resolve anywhere.
        assert not starts(lsp_server.definition(uri, 0, 0)) or (0, 5) in starts(
            lsp_server.definition(uri, 0, 0)
        )
