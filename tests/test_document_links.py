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
        source = (
            "ltm rule /Common/r {\n"
            "when HTTP_REQUEST { pool /Common/missing }\n"
            "}\n"
        )
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        assert links
        assert links[0].target is None
        assert links[0].tooltip and "no definition" in links[0].tooltip

    def test_no_irule_no_links(self):
        from lsp.features._bigip_links import get_bigip_document_links

        source = "ltm pool /Common/p { }\n"
        links = get_bigip_document_links(source, uri="file:///tmp/x.conf", workspace_configs={})
        assert links == []
