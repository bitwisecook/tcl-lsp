"""Token types and position-aware token dataclass for Tcl lexing.

The classes are provided by the Rust `tcl_lsp_rust` extension module
(see `rust/tcl-lexer/src/tokens.rs` for the pure-Rust types and
`rust/tcl-lsp-rust/src/tokens.rs` for the PyO3 wrappers). A pure-Python
fallback is kept in this file for developer environments and platforms
where the Rust wheel is not yet available or has not been built; it will
remain until a later stage of the Python-to-Rust rewrite described in
`docs/rust-rewrite.md`.
"""

from __future__ import annotations

try:
    from tcl_lsp_rust import SourcePosition as _RustSourcePosition  # ty: ignore[unresolved-import]
    from tcl_lsp_rust import Token as _RustToken  # ty: ignore[unresolved-import]
    from tcl_lsp_rust import TokenType as _RustTokenType  # ty: ignore[unresolved-import]
except ImportError:
    _RustTokenType = None
    _RustSourcePosition = None
    _RustToken = None


if _RustTokenType is not None and _RustSourcePosition is not None and _RustToken is not None:
    TokenType = _RustTokenType
    SourcePosition = _RustSourcePosition
    Token = _RustToken
else:
    from dataclasses import dataclass
    from enum import Enum, auto

    class TokenType(Enum):
        """Pure-Python fallback for the `TokenType` enum.

        Kept in lock-step with the Rust implementation; update both in
        the same commit until the fallback is removed. See
        ``rust/tcl-lexer/src/tokens.rs`` for the source of truth.
        """

        ESC = auto()  # plain string fragment (possibly with escape sequences)
        STR = auto()  # braced string {…}
        CMD = auto()  # command substitution [… ]
        VAR = auto()  # variable substitution $name
        SEP = auto()  # whitespace separator
        EOL = auto()  # end-of-line / semicolon
        EOF = auto()  # end-of-input
        COMMENT = auto()  # comment (# to end of line)
        EXPAND = auto()  # {*} expansion prefix

    @dataclass(frozen=True, slots=True)
    class SourcePosition:
        """Pure-Python fallback for the `SourcePosition` dataclass."""

        line: int  # 0-based line number
        character: int  # 0-based column (UTF-16 code units per LSP spec)
        offset: int  # byte offset into the source string

    @dataclass(frozen=True, slots=True)
    class Token:
        """Pure-Python fallback for the `Token` dataclass."""

        type: TokenType
        text: str
        start: SourcePosition
        end: SourcePosition
        in_quote: bool = False
