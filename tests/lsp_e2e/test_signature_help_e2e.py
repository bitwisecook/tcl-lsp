"""Signature help, end-to-end against the packaged server.

Ported from ``tests/test_signature_help.py``.
"""

from __future__ import annotations


def _doc(sig: dict) -> str:
    doc = sig.get("documentation")
    if isinstance(doc, dict):
        return str(doc.get("value", ""))
    return str(doc or "")


class TestSignatureHelpUserProcs:
    def test_proc_first_arg(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'proc greet {name greeting} { puts "$greeting $name" }\ngreet World\n'
        lsp_server.open_ready(uri, src)
        result = lsp_server.signature_help(uri, 1, 12)
        assert result is not None
        sigs = result["signatures"]
        assert len(sigs) == 1
        assert "greet" in sigs[0]["label"]
        assert len(sigs[0]["parameters"]) == 2

    def test_proc_active_param_tracking(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc add {a b} { expr {$a + $b} }\nadd 1 ")
        result = lsp_server.signature_help(uri, 1, 6)
        assert result is not None
        assert result["activeParameter"] == 1

    def test_proc_with_defaults_marks_optional(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'proc greet {name {greeting Hello}} { puts "$greeting $name" }\ngreet World\n'
        lsp_server.open_ready(uri, src)
        result = lsp_server.signature_help(uri, 1, 12)
        params = result["signatures"][0]["parameters"]
        assert len(params) == 2
        assert "?" in params[1]["label"]


class TestSignatureHelpBuiltins:
    def test_set_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x ")
        result = lsp_server.signature_help(uri, 0, 6)
        assert result is not None
        assert "set" in result["signatures"][0]["label"]

    def test_cursor_on_command_name_returns_none(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set")
        assert not lsp_server.signature_help(uri, 0, 2)

    def test_unknown_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "nonexistent_command arg ")
        assert not lsp_server.signature_help(uri, 0, 24)

    def test_empty_file(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        assert not lsp_server.signature_help(uri, 0, 0)


class TestSignatureHelpDocumentation:
    def test_set_has_documentation(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x ")
        result = lsp_server.signature_help(uri, 0, 6)
        assert "With one argument" in _doc(result["signatures"][0])

    def test_socket_has_documentation(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket localhost ")
        result = lsp_server.signature_help(uri, 0, 17)
        doc = _doc(result["signatures"][0])
        assert "listening socket" in doc or "server" in doc.lower()

    def test_proc_still_has_documentation(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World ')
        result = lsp_server.signature_help(uri, 1, 12)
        assert result["signatures"][0]["label"].startswith("greet")

    def test_proc_doc_comment_in_signature_help(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = '# Says hello\nproc greet {name} { puts "Hello $name" }\ngreet World '
        lsp_server.open_ready(uri, src)
        assert "Says hello" in _doc(lsp_server.signature_help(uri, 2, 12)["signatures"][0])


class TestSignatureHelpSubcommands:
    def test_string_length(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string length ")
        result = lsp_server.signature_help(uri, 0, 14)
        assert result is not None
        assert any(s["label"] == "string length string" for s in result["signatures"])

    def test_string_index_subcommand(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string index ")
        result = lsp_server.signature_help(uri, 0, 13)
        assert result is not None
        assert any(s["label"] == "string index string charIndex" for s in result["signatures"])

    def test_string_unknown_subcommand_no_signature(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string bogus ")
        assert not lsp_server.signature_help(uri, 0, 13)
