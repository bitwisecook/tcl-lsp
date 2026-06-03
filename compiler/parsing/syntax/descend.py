"""Lazy descent into braced bodies and command substitutions.

At its own level a ``{…}`` body or ``[…]`` command substitution is a single
``STR`` / ``CMD`` token whose ``raw`` owns the full span — delimiters included.
*Descending* re-lexes its inner text as a child concrete syntax tree, anchored
one byte past the opener, so the child owns the delimiter-excluding interior with
absolute positions matching where that interior sits in the document. That is the
"a delimited region is a node owning its full span with delimiter-excluding
children" shape, realised for nested bodies.

This mirrors :func:`compiler.parsing.green_tree.descend_token` /
:func:`compiler.parsing.green_tree.descend_command` — the descent the analyser
uses today — but yields a structural :class:`SyntaxTree` rather than a flat token
list, and flags an unterminated region as *recovered* (the lossless
representation of malformed input). Because both this module's
:func:`build_document` and ``green_tree.node_for`` tokenise through the same
shared memo, the descended token stream is identical to today's by construction.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from shared.tokens import Token, TokenType

from .build import build_document
from .red import SyntaxTree

if TYPE_CHECKING:
    from .green import GreenNode

_OPAQUE = (TokenType.STR, TokenType.CMD)


def _terminated(token: Token, source: str) -> bool:
    """Return whether *token*'s closing delimiter is present in *source*.

    The closer sits one byte past the ``len(token.text)`` bytes of inner content
    after the opener.  Deriving it from the *length* (not ``token.end.offset``)
    is what classifies an empty ``{}`` / ``[]`` correctly — the lexer reports
    ``end`` *on* the closer there, which ``end + 1`` would overshoot (#527).
    """
    idx = token.start.offset + 1 + len(token.text)
    if idx < 0 or idx >= len(source):
        return False
    close = "}" if token.type is TokenType.STR else "]"
    return source[idx] == close


@dataclass(frozen=True, slots=True)
class Descended:
    """The child CST of a descended body / command substitution.

    ``tree`` is the inner region's concrete syntax tree, anchored at its absolute
    position; ``terminated`` is whether the closing delimiter was present in the
    parent region (a *recovered* region when not — the lossless representation of
    an unterminated ``{…}`` / ``[…]``).
    """

    tree: SyntaxTree
    terminated: bool

    @property
    def root(self):  # noqa: ANN201 - re-exports the red root
        return self.tree.root

    @property
    def document(self) -> GreenNode:
        return self.tree.green


def descend_token(token: Token, source: str, *, insidequote: bool = False) -> Descended:
    """Descend an absolutely-positioned ``STR`` / ``CMD`` (or recovery) token.

    For an opaque ``{…}`` / ``[…]`` token the child is anchored one byte past the
    opener; any other token (an ``ESC`` recovery body) is anchored at its own
    position and is always *recovered*.  *source* is the region containing
    *token*, used only to decide whether the delimiter is terminated.
    """
    if token.type in _OPAQUE:
        b_off = token.start.offset + 1
        b_line = token.start.line
        b_col = token.start.character + 1
        terminated = _terminated(token, source)
    else:
        b_off = token.start.offset
        b_line = token.start.line
        b_col = token.start.character
        terminated = False
    document, _ = build_document(token.text, b_off, b_line, b_col, insidequote=insidequote)
    tree = SyntaxTree(document, b_off, b_line, b_col, text=token.text)
    return Descended(tree, terminated)


@dataclass(frozen=True, slots=True)
class CommandBody:
    """A registry-resolved ``BODY`` argument of a command, descended.

    ``index`` is the real argument index, ``text`` the body word's inner text,
    ``token`` the owning ``STR`` token, and ``descended`` its child CST.
    """

    index: int
    text: str
    token: Token
    descended: Descended


def descend_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    source: str,
) -> list[CommandBody]:
    """Descend each ``ArgRole.BODY`` argument of a command as a child CST.

    The single registry-aware descent entry point — the analogue of
    ``green_tree.descend_command``, resolving body arguments through the same
    ``iter_body_arguments`` so the set of descended bodies matches exactly. A
    body that is not a non-empty ``STR`` word is skipped (it is data, not a
    script the tree should re-lex).  ``ArgRole.EXPR`` arguments are deliberately
    not routed here — they are handled by ``expr_lexer``.

    The registry import is deferred so importing this module never triggers the
    registry ↔ parsing load cycle.
    """
    from compiler.registry.runtime import iter_body_arguments

    bodies: list[CommandBody] = []
    for body in iter_body_arguments(cmd_name, args, arg_tokens):
        if body.token.type is not TokenType.STR or not body.text.strip():
            continue
        bodies.append(
            CommandBody(
                index=body.index,
                text=body.text,
                token=body.token,
                descended=descend_token(body.token, source),
            )
        )
    return bodies
