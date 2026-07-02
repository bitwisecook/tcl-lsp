"""Tcl 9.1 dialect support, end-to-end over LSP.

Oracle: C Tcl 9.1b0 source (doc/unicode.n, doc/timer.n, doc/expr.n, doc/lseq.n).
The dialect is pinned with a `# tcl-dialect: tcl9.1` directive (server-side
source detection), and behaviour is observed through completion + diagnostics.
"""

from __future__ import annotations


def _labels(items):
    if isinstance(items, dict):
        items = items.get("items", [])
    return {i.get("label") for i in (items or [])}


def _codes(diags):
    return {str(d.get("code")) for d in diags}


def _complete_cmd(lsp_server, uri_factory, dialect, partial):
    """Open a buffer pinned to `dialect` and complete a command-position word."""
    uri = uri_factory()
    src = f"# tcl-dialect: {dialect}\n{partial}\n"
    lsp_server.open_ready(uri, src)
    return _labels(lsp_server.completion(uri, 1, len(partial)))


class TestTcl91Completion:
    def test_unicode_and_timer_offered_in_91(self, lsp_server, uri_factory):
        # doc/unicode.n, doc/timer.n — both are new commands in 9.1.
        assert "unicode" in _complete_cmd(lsp_server, uri_factory, "tcl9.1", "unic")
        assert "timer" in _complete_cmd(lsp_server, uri_factory, "tcl9.1", "time")

    def test_unicode_and_timer_absent_in_90(self, lsp_server, uri_factory):
        assert "unicode" not in _complete_cmd(lsp_server, uri_factory, "tcl9.0", "unic")
        assert "timer" not in _complete_cmd(lsp_server, uri_factory, "tcl9.0", "time")

    def test_math_and_lfilter_offered_in_91(self, lsp_server, uri_factory):
        # doc/divmod.n, doc/lfilter.n — new commands in 9.1 (C tclBasic.c).
        assert "divmod" in _complete_cmd(lsp_server, uri_factory, "tcl9.1", "divm")
        assert "lfilter" in _complete_cmd(lsp_server, uri_factory, "tcl9.1", "lfil")

    def test_math_absent_in_90(self, lsp_server, uri_factory):
        assert "divmod" not in _complete_cmd(lsp_server, uri_factory, "tcl9.0", "divm")
        assert "lfilter" not in _complete_cmd(lsp_server, uri_factory, "tcl9.0", "lfil")

    def test_90_commands_still_offered_in_91(self, lsp_server, uri_factory):
        # A `.1` release is additive: `lseq` (9.0) persists in 9.1.
        assert "lseq" in _complete_cmd(lsp_server, uri_factory, "tcl9.1", "lseq")


class TestTcl91Operators:
    # doc/expr.n — the `lt`/`le`/`gt`/`ge` string operators (TIP 461) are 9.0+.
    def test_lt_operator_no_w003_in_91(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "# tcl-dialect: tcl9.1\nexpr {$a lt $b}\n")
        assert "W003" not in _codes(diags)

    def test_lt_operator_flags_w003_in_86(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "# tcl-dialect: tcl8.6\nexpr {$a lt $b}\n")
        assert "W003" in _codes(diags)
