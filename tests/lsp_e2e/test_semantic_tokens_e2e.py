"""Semantic tokens, end-to-end against the packaged server.

Ported from ``tests/test_semantic_tokens.py``.  The packaged server returns the
delta-encoded ``data`` array and advertises the legend in ``initialize``; the
test decodes the data and maps type indices back through that live legend, so a
legend/encoder drift in the shipped artifact is caught here.
"""

from __future__ import annotations

import pytest

from ._lsp_helpers import decode_semantic_tokens


@pytest.fixture
def legend(lsp_server):
    caps = lsp_server.initialize_result["capabilities"]
    return caps["semanticTokensProvider"]["legend"]["tokenTypes"]


def _typed(lsp_server, legend, uri):
    tokens = decode_semantic_tokens(lsp_server.semantic_tokens(uri))
    for tok in tokens:
        tok["type"] = legend[tok["type"]]
    return tokens


class TestSemanticTokens:
    def test_simple_puts(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        tokens = _typed(lsp_server, legend, uri)
        assert len(tokens) == 2
        assert tokens[0]["type"] == "function"
        assert tokens[0]["length"] == 4
        assert tokens[1]["type"] == "string"

    def test_variable(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x $y\n")
        types = [t["type"] for t in _typed(lsp_server, legend, uri)]
        assert "function" in types
        assert "variable" in types

    def test_number(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        nums = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "number"]
        assert len(nums) == 1
        assert nums[0]["length"] == 2

    def test_comment(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# hello world\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "comment"

    def test_proc_as_keyword(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc foo {x} {}\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "keyword"

    def test_tokens_track_an_edit(self, lsp_server, uri_factory, legend):
        # After an incremental edit the encoder must run over the *new* buffer:
        # rename `puts` to `incr` and the first token type flips function->keyword
        # only if recomputed against tracked content.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "function"
        # Replace `42` with a variable read `$y`.
        lsp_server.change_document(
            uri,
            2,
            [
                {
                    "range": {
                        "start": {"line": 0, "character": 6},
                        "end": {"line": 0, "character": 8},
                    },
                    "text": "$y",
                }
            ],
        )
        lsp_server.await_diagnostics(uri, version=2)
        types = [t["type"] for t in _typed(lsp_server, legend, uri)]
        assert "variable" in types
        assert "number" not in types
