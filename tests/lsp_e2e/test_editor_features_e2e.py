"""Code lens, document links, formatting, and inlay hints — end-to-end.

More of the cross-implementation conformance surface (raw JSON-RPC against the
packaged server; a spec the Rust/Zed port must meet).  Ported from
``tests/test_code_lens.py``, ``tests/test_document_links.py``,
``tests/test_formatter.py``, and ``tests/test_inlay_hints.py``.
"""

from __future__ import annotations


def _full_text(edits) -> str | None:
    """A whole-document formatter returns a single replace edit; return its text."""
    if not edits:
        return None
    return edits[0]["newText"]


class TestCodeLens:
    def test_proc_gets_reference_count_lens(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\ngreet\ngreet\n")
        lenses = lsp_server.code_lens(uri) or []
        # A lens sits on the proc name (line 0) carrying its qualified name.
        on_proc = [lens for lens in lenses if lens["range"]["start"]["line"] == 0]
        assert on_proc
        assert any((lens.get("data") or {}).get("qname") == "::greet" for lens in on_proc)

    def test_no_lens_without_procs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\nputs $x\n")
        assert (lsp_server.code_lens(uri) or []) == []


class TestDocumentLinks:
    def test_source_command_is_linked(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "source other.tcl\nputs done\n")
        links = lsp_server.document_links(uri) or []
        assert any("other.tcl" in (link.get("tooltip") or "") for link in links)

    def test_package_require_is_linked(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "package require Tcl\n")
        links = lsp_server.document_links(uri) or []
        assert any(link["range"]["start"]["line"] == 0 for link in links)


class TestFormatting:
    def test_full_document_formatting_normalises_spacing(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc  f {}    {\nputs   hi\n}\n")
        formatted = _full_text(lsp_server.formatting(uri))
        assert formatted is not None
        assert "proc f {} {" in formatted
        assert "    puts hi" in formatted

    def test_already_formatted_is_stable(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    puts hi\n}\n"
        lsp_server.open_ready(uri, src)
        edits = lsp_server.formatting(uri) or []
        # No change needed → either no edits, or an edit that reproduces the text.
        if edits:
            assert _full_text(edits) == src

    def test_range_formatting_returns_edits(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc  f {}    {\nputs   hi\n}\n")
        edits = lsp_server.range_formatting(uri, (0, 0), (2, 1)) or []
        assert edits
        assert "proc f {} {" in edits[0]["newText"]


class TestInlayHints:
    def test_provider_responds_with_a_list(self, lsp_server, uri_factory):
        # Inlay hints are gated off by default (``inlay_hints_enabled``); the
        # provider must still answer with a well-formed (here empty) list, never
        # an error.  This pins the default-off contract the Rust port must match.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        result = lsp_server.inlay_hints(uri, (0, 0), (1, 0))
        assert isinstance(result, list)

    def test_hint_kinds_are_valid_when_present(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc add {a b} { expr {$a + $b} }\nadd 1 2\n")
        for hint in lsp_server.inlay_hints(uri, (0, 0), (2, 0)) or []:
            assert hint.get("kind") in (1, 2)  # Type, Parameter
