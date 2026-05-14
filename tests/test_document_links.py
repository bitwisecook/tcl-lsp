"""Tests for the document link provider."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.document_links import get_document_links


class TestDocumentLinks:
    def test_source_command(self):
        source = "source utils.tcl\n"
        links = get_document_links(source)
        assert len(links) >= 1
        assert any("utils.tcl" in (link.tooltip or "") for link in links)

    def test_source_with_variable_skipped(self):
        source = "source $dir/utils.tcl\n"
        links = get_document_links(source)
        # Variable paths should be skipped
        source_links = [lnk for lnk in links if lnk.tooltip and "source" in lnk.tooltip.lower()]
        assert len(source_links) == 0

    def test_package_require(self):
        source = "package require Tcl 8.6\n"
        links = get_document_links(source)
        assert len(links) >= 1
        assert any("Tcl" in (link.tooltip or "") for link in links)

    def test_empty_file(self):
        links = get_document_links("")
        assert links == []

    def test_no_links_for_normal_code(self):
        source = "set x 42\nputs $x\n"
        links = get_document_links(source)
        assert len(links) == 0


class TestBigipDocumentLinks:
    def test_irule_body_pool_ref_emits_link(self):
        from lsp.features._bigip_links import get_bigip_document_links

        source = (
            "ltm pool /Common/web_pool { }\n"
            "ltm rule /Common/r {\n"
            "when HTTP_REQUEST { pool /Common/web_pool }\n"
            "}\n"
        )
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        assert links, "expected at least one document link for the iRule pool ref"
        # Same-file resolve: target points back at the same URI's pool stanza line.
        assert links[0].target is not None
        assert "/tmp/x.conf" in links[0].target

    def test_irule_body_unresolved_ref_still_emits_link_without_target(self):
        from lsp.features._bigip_links import get_bigip_document_links

        # Reference present but no matching definition — link still
        # emitted (so the user can see the range was recognised), but
        # ``target`` is ``None`` and the tooltip says "no definition".
        source = "ltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/missing }\n}\n"
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        assert links
        assert links[0].target is None
        assert links[0].tooltip and "no definition" in links[0].tooltip

    def test_property_value_emits_link_to_target_object(self):
        """Migrated TMSH properties expose document links pointing at
        the exact reference token (not the whole property line) so
        editor click-to-navigate lands precisely on the path."""
        from lsp.features._bigip_links import get_bigip_document_links

        source = (
            "ltm monitor http /Common/http_probe { }\n"
            "ltm pool /Common/web_pool {\n"
            "    monitor /Common/http_probe\n"
            "}\n"
        )
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        # The pool's ``monitor`` property contributes a link.
        targets = [link.target for link in links if link.target is not None]
        assert any("/tmp/x.conf" in t for t in targets)
        # The link range is bounded — not the whole line.
        link = next(
            link
            for link in links
            if link.target is not None and link.tooltip and "/Common/http_probe" in link.tooltip
        )
        assert link.range.start.character > 0
        # Registry ``SourceRange`` is half-open; the link layer
        # converts to LSP's exclusive end so the LSP range spans
        # exactly the path length (no off-by-one trailing
        # whitespace / brace).
        assert link.range.end.character - link.range.start.character == len("/Common/http_probe")

    def test_property_value_link_inside_keyed_list_item(self):
        """References inside a keyed-block list item (``profiles {
        /Common/clientssl { ... } }``) get individual links — the
        ListSpec records per-item ranges that the link layer
        consumes directly."""
        from lsp.features._bigip_links import get_bigip_document_links

        source = (
            "ltm virtual /Common/v {\n"
            "    destination /Common/0.0.0.0:443\n"
            "    profiles {\n"
            "        /Common/clientssl { context clientside }\n"
            "        /Common/http { }\n"
            "    }\n"
            "}\n"
            "ltm profile client-ssl /Common/clientssl { }\n"
            "ltm profile http /Common/http { }\n"
        )
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        tooltips = {link.tooltip for link in links if link.tooltip}
        assert "Go to /Common/clientssl" in tooltips
        assert "Go to /Common/http" in tooltips

    def test_no_irule_no_links(self):
        from lsp.features._bigip_links import get_bigip_document_links

        source = "ltm pool /Common/p { }\n"
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        assert links == []


class TestBigipReferencesAndRename:
    def test_references_finds_decl_and_irule_body_use(self):
        from lsp.features._bigip_refs import get_bigip_references

        src = (
            "ltm pool /Common/web_pool { }\n"
            "ltm rule /Common/r {\n"
            "when HTTP_REQUEST { pool /Common/web_pool }\n"
            "}\n"
        )
        # Cursor on the pool path in the stanza header.
        refs = get_bigip_references(
            src,
            uri="file:///t.conf",
            line=0,
            character=13,
            workspace_configs=None,
            workspace_sources=None,
        )
        assert len(refs) == 2
        assert {r.range.start.line for r in refs} == {0, 2}

    def test_prepare_rename_returns_token_range(self):
        from lsp.features._bigip_refs import prepare_bigip_rename

        src = "ltm pool /Common/web_pool { }\n"
        rng = prepare_bigip_rename(src, line=0, character=13)
        assert rng is not None
        assert rng.start.line == 0
        assert rng.end.character > rng.start.character

    def test_prepare_rename_returns_none_off_path(self):
        from lsp.features._bigip_refs import prepare_bigip_rename

        src = "ltm pool /Common/web_pool { }\n"
        # Cursor on the bare ``pool`` keyword (not a TMSH path).
        rng = prepare_bigip_rename(src, line=0, character=4)
        assert rng is None

    def test_rename_edits_produce_workspace_edit_per_file(self):
        from lsp.features._bigip_refs import get_bigip_rename_edits

        src = (
            "ltm pool /Common/web_pool { }\n"
            "ltm rule /Common/r {\n"
            "when HTTP_REQUEST { pool /Common/web_pool }\n"
            "}\n"
        )
        edit = get_bigip_rename_edits(
            src,
            uri="file:///t.conf",
            line=0,
            character=13,
            new_name="/Common/web_renamed",
            workspace_configs=None,
            workspace_sources=None,
        )
        assert edit is not None
        # ``WorkspaceEdit.changes`` is ``Optional[Mapping[...]]``; bind to
        # a local + assert non-None so the type checker can narrow before
        # the lookup.
        changes = edit.changes
        assert changes is not None
        assert "file:///t.conf" in changes
        rewritten = changes["file:///t.conf"][0].new_text
        assert "/Common/web_renamed" in rewritten
        assert "/Common/web_pool" not in rewritten
