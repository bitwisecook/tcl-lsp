"""Position-aware Tcl lexer."""

from __future__ import annotations

import bisect

from .tokens import SourcePosition, Token, TokenType


class TclParseError(Exception):
    """Raised for Tcl syntax errors detected during lexing."""


class TclLexer:
    """Tokenises Tcl source with full position tracking.

    Produces Token objects carrying line/column/offset for both start and end.
    The dispatch logic mirrors Tcl tokenisation rules and adds:
      - line/col tracking in _advance()
      - ${name} and $ns::var and $arr(idx) variable forms
      - backslash-newline continuation
      - COMMENT token type

    When :attr:`strict_quoting` is ``True`` (class-level flag), the lexer
    raises :class:`TclParseError` for "extra characters after close-quote".
    The VM sets this flag during compilation; the LSP leaves it ``False``.

    When :attr:`expand_syntax` is ``True`` (class-level flag), the lexer
    recognises ``{*}`` at word start as the Tcl 8.5+ expansion prefix and
    emits a :attr:`TokenType.EXPAND` token.  Disabled for Tcl 8.4 and
    iRules dialects.
    """

    strict_quoting: bool = False
    expand_syntax: bool = True
    irules_brace_separator: bool = False

    def __init__(
        self,
        text: str,
        base_offset: int = 0,
        base_line: int = 0,
        base_col: int = 0,
        virtual_insertions: dict[int, str] | None = None,
    ) -> None:
        self.text = text
        self.pos = 0
        self._base_offset = base_offset
        self._base_line = base_line
        self._base_col = base_col
        self._line = base_line
        self._col = base_col
        self._start = 0
        self._end = 0
        self._type = TokenType.EOL
        self._at_command_start = True
        self.insidequote = False
        # Non-fatal warnings collected in non-strict mode.  Each entry is
        # ``(position, message)`` where *position* is the :class:`SourcePosition`
        # where the issue was detected and *message* is the Tcl-equivalent
        # error text.  The analyser/recovery layer harvests these and converts
        # them to LSP diagnostics.
        self.warnings: list[tuple[SourcePosition, str]] = []
        # Zero-width virtual characters that the lexer "sees" at specific
        # offsets without any text actually being inserted.  Used by
        # error recovery to inject missing delimiters (], }, {).
        self._virtuals: dict[int, str] = dict(virtual_insertions) if virtual_insertions else {}
        # Pending synthetic SEP token for iRules }{ word boundary.
        self._pending_sep: Token | None = None

    @property
    def remaining(self) -> int:
        r = len(self.text) - self.pos
        if r > 0:
            return r
        # A virtual token at the current position still counts as content.
        if self.pos in self._virtuals:
            return 1
        return 0

    def _cur(self) -> str:
        v = self._virtuals.get(self.pos)
        if v is not None:
            return v
        return self.text[self.pos]

    def _advance(self, n: int = 1) -> None:
        for _ in range(n):
            if self.pos in self._virtuals:
                del self._virtuals[self.pos]  # consumed — zero width
                continue  # don't advance pos
            if self.pos < len(self.text):
                if self.text[self.pos] == "\n":
                    self._line += 1
                    self._col = 0
                else:
                    self._col += 1
                self.pos += 1

    def _position(self) -> SourcePosition:
        return SourcePosition(
            line=self._line, character=self._col, offset=self.pos + self._base_offset
        )

    def _pos_at(self, offset: int) -> SourcePosition:
        """Compute SourcePosition for an arbitrary offset using a line index."""
        if not hasattr(self, "_line_starts"):
            # Build an index of offsets where each line begins (lazily, once).
            starts = [0]
            for i, ch in enumerate(self.text):
                if ch == "\n":
                    starts.append(i + 1)
            self._line_starts = starts
        line_idx = bisect.bisect_right(self._line_starts, offset) - 1
        col = offset - self._line_starts[line_idx]
        return SourcePosition(
            line=self._base_line + line_idx,
            character=(self._base_col + col) if line_idx == 0 else col,
            offset=offset + self._base_offset,
        )

    def _token_text(self) -> str:
        if self._end < self._start:
            return ""
        return self.text[self._start : self._end + 1]

    def _parse_sep(self) -> None:
        self._start = self.pos
        while self.remaining and self._cur() in " \t\r\x0b\x0c":
            self._advance()
        self._end = self.pos - 1
        self._type = TokenType.SEP

    def _parse_eol(self) -> None:
        self._start = self.pos
        while self.remaining and self._cur() in " \t\n\r\x0b\x0c;":
            self._advance()
        self._end = self.pos - 1
        self._type = TokenType.EOL
        self._at_command_start = True

    def _parse_command(self) -> None:
        level = 1
        blevel = 0
        in_quotes = False
        self._advance()  # skip opening '['
        self._start = self.pos
        while True:
            if not self.remaining:
                break
            ch = self._cur()
            if ch == '"' and blevel == 0:
                in_quotes = not in_quotes
            elif ch == "[" and blevel == 0 and not in_quotes:
                level += 1
            elif ch == "]" and blevel == 0 and not in_quotes:
                level -= 1
                if level == 0:
                    break
            elif ch == "\\":
                self._advance()
            elif ch == "$" and not in_quotes and blevel == 0:
                # ${ starts a variable name reference — skip past the
                # matching } so that the brace is not counted toward the
                # brace-depth used for word grouping.
                if self.pos + 1 < len(self.text) and self.text[self.pos + 1] == "{":
                    self._advance()  # skip $
                    self._advance()  # skip {
                    while self.remaining and self._cur() != "}":
                        self._advance()
                    if not self.remaining:
                        if TclLexer.strict_quoting:
                            raise TclParseError("missing close-brace for variable name")
                        self.warnings.append(
                            (
                                self._position(),
                                "missing close-brace for variable name",
                            )
                        )
                    # If } found, it will be advanced past at the end of the loop.
            elif ch == "{" and not in_quotes:
                blevel += 1
            elif ch == "}" and not in_quotes:
                if blevel:
                    blevel -= 1
            self._advance()
        self._end = self.pos - 1
        self._type = TokenType.CMD
        if self.remaining and self._cur() == "]":
            self._advance()
        elif level > 0:
            if TclLexer.strict_quoting:
                raise TclParseError("missing close-bracket")
            self.warnings.append(
                (
                    self._position(),
                    "missing close-bracket",
                )
            )

    def _parse_var(self) -> None:
        dollar_pos = self._position()
        self._advance()  # skip '$'

        # Handle ${name} form
        if self.remaining and self._cur() == "{":
            self._advance()  # skip '{'
            self._start = self.pos
            while self.remaining and self._cur() != "}":
                self._advance()
            self._end = self.pos - 1
            self._type = TokenType.VAR
            if self.remaining:
                self._advance()  # skip '}'
            elif TclLexer.strict_quoting:
                raise TclParseError("missing close-brace for variable name")
            else:
                self.warnings.append(
                    (
                        dollar_pos,
                        "missing close-brace for variable name",
                    )
                )
            return

        self._start = self.pos
        # Accept alnum, underscore, :: (namespace separator)
        while self.remaining:
            ch = self._cur()
            if ch.isalnum() or ch == "_":
                self._advance()
            elif ch == ":" and self.pos + 1 < len(self.text) and self.text[self.pos + 1] == ":":
                self._advance(2)  # skip '::'
            else:
                break

        # Handle array indexing $arr(idx) — also handles $() and $(idx)
        # which are array references with an empty variable name (C Tcl
        # parses these identically and they produce "can't read" errors).
        if self.remaining and self._cur() == "(":
            level = 1
            self._advance()
            while self.remaining and level > 0:
                ch = self._cur()
                if ch == "(":
                    level += 1
                elif ch == ")":
                    level -= 1
                elif ch == "$" and self.pos + 1 < len(self.text) and self.text[self.pos + 1] == "{":
                    # ${...} inside array index — scan for matching }
                    self._advance()  # skip $
                    self._advance()  # skip {
                    while self.remaining and self._cur() != "}":
                        self._advance()
                    if not self.remaining:
                        if TclLexer.strict_quoting:
                            raise TclParseError("missing close-brace for variable name")
                        self.warnings.append(
                            (
                                self._position(),
                                "missing close-brace for variable name",
                            )
                        )
                    # Found } — will be advanced at end of loop
                if level > 0:
                    self._advance()
            if self.remaining:
                self._advance()  # skip closing ')'
            else:
                if TclLexer.strict_quoting:
                    raise TclParseError("missing )")
                self.warnings.append(
                    (
                        self._position(),
                        "missing )",
                    )
                )
            self._end = self.pos - 1
            self._type = TokenType.VAR
            return

        if self._start == self.pos:
            # bare '$'
            self._start = self._end = self.pos - 1
            self._type = TokenType.STR
        else:
            self._end = self.pos - 1
            self._type = TokenType.VAR

    def _parse_brace(self) -> None:
        level = 1
        self._advance()  # skip opening '{'
        self._start = self.pos
        while True:
            if not self.remaining:
                # EOF without matching close-brace
                self._end = self.pos - 1
                if TclLexer.strict_quoting:
                    raise TclParseError("missing close-brace")
                self.warnings.append(
                    (
                        self._position(),
                        "missing close-brace",
                    )
                )
                self._type = TokenType.STR
                return
            ch = self._cur()
            if ch == "\\" and self.remaining >= 2:
                self._advance()
            elif ch == "}":
                level -= 1
                if level == 0:
                    self._end = self.pos - 1
                    self._advance()  # skip closing '}'
                    if self.remaining and self._cur() not in (
                        " ",
                        "\t",
                        "\n",
                        "\r",
                        "\x0b",
                        "\x0c",
                        ";",
                    ):
                        if TclLexer.irules_brace_separator and self._cur() == "{":
                            # iRules treats }{ as a word boundary — inject a
                            # zero-width SEP so the segmenter sees two words.
                            sep_pos = self._position()
                            self._pending_sep = Token(
                                type=TokenType.SEP,
                                text="",
                                start=sep_pos,
                                end=sep_pos,
                            )
                        elif TclLexer.strict_quoting:
                            raise TclParseError("extra characters after close-brace")
                        else:
                            self.warnings.append(
                                (
                                    self._position(),
                                    "extra characters after close-brace",
                                )
                            )
                    self._type = TokenType.STR
                    return
            elif ch == "{":
                level += 1
            self._advance()

    def _parse_expand(self) -> None:
        """Emit an EXPAND token for the ``{*}`` prefix.

        The token text is empty; the prefix just signals the segmenter
        that the following word tokens should be list-expanded.
        """
        self._start = self.pos
        self._advance(3)  # skip '{*}'
        self._end = self._start  # zero-width
        self._type = TokenType.EXPAND

    def _parse_string(self) -> None:
        newword = self._type in (TokenType.SEP, TokenType.EOL, TokenType.STR, TokenType.EXPAND)
        if newword and self.remaining and self._cur() == "{":
            # Check for {*} expansion prefix (Tcl 8.5+)
            if (
                TclLexer.expand_syntax
                and self.pos + 2 < len(self.text)
                and self.text[self.pos + 1] == "*"
                and self.text[self.pos + 2] == "}"
                # Must be followed by a non-separator (the word to expand)
                and self.pos + 3 < len(self.text)
                and self.text[self.pos + 3] not in " \t\n\r\x0b\x0c;"
            ):
                return self._parse_expand()
            return self._parse_brace()
        if newword and self.remaining and self._cur() == '"':
            self.insidequote = True
            self._advance()
        self._start = self.pos
        while True:
            if not self.remaining:
                if self.insidequote:
                    if TclLexer.strict_quoting:
                        raise TclParseError('missing "')
                    self.warnings.append(
                        (
                            self._position(),
                            'missing "',
                        )
                    )
                self._end = self.pos - 1
                self._type = TokenType.ESC
                return
            ch = self._cur()
            if ch == "\\":
                if self.remaining >= 2:
                    # Handle backslash-newline continuation
                    self._advance()
            elif ch in ("$", "["):
                self._end = self.pos - 1
                self._type = TokenType.ESC
                return
            elif ch in (" ", "\t", "\n", "\r", "\x0b", "\x0c", ";"):
                if not self.insidequote:
                    self._end = self.pos - 1
                    self._type = TokenType.ESC
                    return
            elif ch == '"':
                if self.insidequote:
                    self._end = self.pos - 1
                    self._type = TokenType.ESC
                    self._advance()
                    self.insidequote = False
                    # After closing quote, next char must be separator or EOF
                    if self.remaining and self._cur() not in (
                        " ",
                        "\t",
                        "\n",
                        "\r",
                        "\x0b",
                        "\x0c",
                        ";",
                        "]",
                    ):
                        if TclLexer.strict_quoting:
                            raise TclParseError("extra characters after close-quote")
                        self.warnings.append(
                            (
                                self._position(),
                                "extra characters after close-quote",
                            )
                        )
                    return
            self._advance()

    def _parse_comment(self) -> None:
        self._start = self.pos  # include the '#'
        while self.remaining:
            ch = self._cur()
            if ch == "\\":
                next_pos = self.pos + 1
                if next_pos < len(self.text):
                    next_ch = self.text[next_pos]
                    if next_ch == "\n":
                        # Backslash-newline continuation: extends the
                        # comment onto the next line.
                        self._advance()  # skip backslash
                        self._advance()  # skip newline
                        continue
                    else:
                        # Backslash followed by another char (e.g. \\):
                        # skip both so the second char doesn't trigger
                        # its own backslash-newline check.
                        self._advance()  # skip backslash
                        self._advance()  # skip escaped char
                        continue
            if ch == "\n":
                break
            self._advance()
        self._end = self.pos - 1
        self._type = TokenType.COMMENT

    # main entry point
    def get_token(self) -> Token | None:
        """Return the next token, or None at EOF."""
        # Drain any synthetic SEP queued by _parse_brace (iRules }{ boundary).
        if self._pending_sep is not None:
            sep = self._pending_sep
            self._pending_sep = None
            self._type = TokenType.SEP
            return sep
        while True:
            if not self.remaining:
                if self._type not in (TokenType.EOL, TokenType.EOF):
                    self._type = TokenType.EOL
                else:
                    self._type = TokenType.EOF
                    return None
                return Token(
                    type=TokenType.EOL,
                    text="",
                    start=self._position(),
                    end=self._position(),
                )
            start_pos = self._position()
            match self._cur():
                case " " | "\t" | "\r" | "\x0b" | "\x0c" if not self.insidequote:
                    self._parse_sep()
                case "\n" | ";" if not self.insidequote:
                    self._parse_eol()
                case "[":
                    self._parse_command()
                case "$":
                    self._parse_var()
                case "#" if self._at_command_start:
                    self._parse_comment()
                case _:
                    self._parse_string()
            text = self._token_text()
            end_pos = self._pos_at(max(self._end, self._start))
            # Track command-start position for comment detection:
            # _at_command_start stays True through SEP/EOL/COMMENT tokens
            # but resets for any actual command content.
            if self._type not in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
                self._at_command_start = False
            return Token(
                type=self._type,
                text=text,
                start=start_pos,
                end=end_pos,
                in_quote=self.insidequote,
            )

    def tokenise_all(self) -> list[Token]:
        """Tokenise the entire source, including SEP and EOL tokens."""
        tokens: list[Token] = []
        while True:
            tok = self.get_token()
            if tok is None:
                break
            tokens.append(tok)
        return tokens
