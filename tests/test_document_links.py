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
