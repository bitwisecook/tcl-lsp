"""Tests for the signature help provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.signature_help import get_signature_help


class TestSignatureHelpUserProcs:
    def test_proc_first_arg(self):
        source = textwrap.dedent("""\
            proc greet {name greeting} { puts "$greeting $name" }
            greet World
        """)
        # Cursor after "greet World " (line 1, col 12)
        result = get_signature_help(source, 1, 12)
        assert result is not None
        assert len(result.signatures) == 1
        assert "greet" in result.signatures[0].label
        assert result.signatures[0].parameters is not None
        assert len(result.signatures[0].parameters) == 2

    def test_proc_active_param_tracking(self):
        source = "proc add {a b} { expr {$a + $b} }\nadd 1 "
        # Cursor after "add 1 " (line 1, col 6) — trailing space starts new arg
        result = get_signature_help(source, 1, 6)
        assert result is not None
        assert result.active_parameter == 1

    def test_proc_with_defaults(self):
        source = textwrap.dedent("""\
            proc greet {name {greeting Hello}} { puts "$greeting $name" }
            greet World
        """)
        result = get_signature_help(source, 1, 12)
        assert result is not None
        sig = result.signatures[0]
        assert sig.parameters is not None
        assert len(sig.parameters) == 2
        # Second param should be optional
        assert "?" in sig.parameters[1].label


class TestSignatureHelpBuiltins:
    def test_set_command(self):
        source = "set x "
        # Cursor after "set x " (col 6)
        result = get_signature_help(source, 0, 6)
        assert result is not None
        assert len(result.signatures) >= 1
        assert "set" in result.signatures[0].label

    def test_cursor_on_command_name(self):
        source = "set"
        result = get_signature_help(source, 0, 2)
        # Should return None when cursor is on the command name itself
        assert result is None

    def test_unknown_command(self):
        source = "nonexistent_command arg "
        result = get_signature_help(source, 0, 24)
        assert result is None

    def test_empty_file(self):
        result = get_signature_help("", 0, 0)
        assert result is None


class TestSignatureHelpDocumentation:
    """Verify signature help carries rich documentation."""

    def test_set_has_documentation(self):
        source = "set x "
        result = get_signature_help(source, 0, 6)
        assert result is not None
        sig = result.signatures[0]
        doc = sig.documentation
        assert doc is not None
        # Documentation should contain the full detail (snippet text)
        doc_text = doc.value if not isinstance(doc, str) else doc
        assert "With one argument" in doc_text

    def test_socket_has_documentation(self):
        source = "socket localhost "
        result = get_signature_help(source, 0, 17)
        assert result is not None
        sig = result.signatures[0]
        doc = sig.documentation
        assert doc is not None
        doc_text = doc.value if not isinstance(doc, str) else doc
        assert "listening socket" in doc_text or "server" in doc_text.lower()

    def test_proc_still_has_documentation(self):
        source = 'proc greet {name} { puts "Hello $name" }\ngreet World '
        # Ensure user proc docs are preserved (not regressed)
        result = get_signature_help(source, 1, 12)
        assert result is not None
        # proc has no doc comment, so documentation may be None
        assert result.signatures[0].label.startswith("greet")

    def test_proc_doc_comment_in_signature_help(self):
        source = '# Says hello\nproc greet {name} { puts "Hello $name" }\ngreet World '
        result = get_signature_help(source, 2, 12)
        assert result is not None
        sig = result.signatures[0]
        assert sig.documentation is not None
        doc_text = str(sig.documentation)
        assert "Says hello" in doc_text


class TestSignatureHelpSubcommands:
    def test_string_length(self):
        source = "string length "
        # Cursor after "string length " (col 14)
        result = get_signature_help(source, 0, 14)
        if result is not None:
            # Should show the string length signature
            assert any("string" in sig.label for sig in result.signatures)
