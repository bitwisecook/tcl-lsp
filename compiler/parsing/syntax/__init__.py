"""Canonical Tcl concrete syntax tree (red-green CST).

A lossless, position-independent green tree (:mod:`.green`) with a lazy
absolute-position red overlay (:mod:`.red`), built from the lexer stream
(:mod:`.build`).  Intended as the single representation the segmenter, AOT
lowering, formatter, minifier, and per-command tooling all consume.

See ``docs/design/compiler/syntax-tree.md``.
"""

from __future__ import annotations

from .build import build_document
from .descend import CommandBody, Descended, descend_command, descend_token
from .green import (
    GreenNode,
    GreenToken,
    GreenTrivia,
    SyntaxKind,
    TriviaKind,
)
from .red import SyntaxNode, SyntaxToken, SyntaxTree
from .segment import segments_from_document, segments_from_tree

__all__ = [
    "CommandBody",
    "Descended",
    "GreenNode",
    "GreenToken",
    "GreenTrivia",
    "SyntaxKind",
    "SyntaxNode",
    "SyntaxToken",
    "SyntaxTree",
    "TriviaKind",
    "build_document",
    "descend_command",
    "descend_token",
    "segments_from_document",
    "segments_from_tree",
]
