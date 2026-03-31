"""Position-aware Tcl lexer."""

from __future__ import annotations

import bisect
import threading

from .tokens import SourcePosition, Token, TokenType

_bisect_right = bisect.bisect_right

# Pre-computed character class sets for O(1) membership testing in the
# lexer hot path.  Using frozenset instead of sequential == checks
# eliminates 5-7 chained comparisons per token (measured ~2x speedup).
_SEP_CHARS = frozenset((" ", "\t", "\r", "\x0b", "\x0c"))
_EOL_CHARS = frozenset(("\n", ";"))
_SEPARATOR_CHARS = frozenset((" ", "\t", "\n", "\r", "\x0b", "\x0c", ";"))
_AFTER_CLOSE_BRACE = frozenset((" ", "\t", "\n", "\r", "\x0b", "\x0c", ";"))
_AFTER_CLOSE_QUOTE = frozenset((" ", "\t", "\n", "\r", "\x0b", "\x0c", ";", "]"))
# Thread-local storage for lexer flags that are modified during VM compilation.
# These must be thread-local because the VM runs in daemon threads and
# concurrent threads modifying class-level flags corrupt each other's state.
_thread_local = threading.local()


def _strict_quoting() -> bool:
    """Return the strict_quoting flag for the current thread."""
    return getattr(_thread_local, "strict_quoting", False)


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

    __slots__ = (
        "text",
        "_len",
        "pos",
        "_base_offset",
        "_base_line",
        "_base_col",
        "_line",
        "_col",
        "_start",
        "_end",
        "_type",
        "_at_command_start",
        "insidequote",
        "warnings",
        "_virtuals",
        "_has_virtuals",
        "_pending_sep",
        "_line_starts",
    )

    # strict_quoting is now thread-local; see _strict_quoting() below.
    # Retained as class attribute ONLY for backward-compat reads from non-VM
    # code; the VM always uses _thread_local.strict_quoting via the helper.
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
        line_starts: list[int] | tuple[int, ...] | None = None,
    ) -> None:
        self.text = text
        self._len = len(text)
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
        self._has_virtuals: bool = bool(self._virtuals)
        # Pending synthetic SEP token for iRules }{ word boundary.
        self._pending_sep: Token | None = None
        # Line index for _pos_at().  Shared across lexer instances for the
        # same source to avoid O(n) newline scanning per instance.
        if line_starts is not None:
            self._line_starts: list[int] | tuple[int, ...] = line_starts
        else:
            starts = [0]
            for i, ch in enumerate(text):
                if ch == "\n":
                    starts.append(i + 1)
            self._line_starts = starts

    @property
    def remaining(self) -> int:
        r = self._len - self.pos
        if r > 0:
            return r
        # A virtual token at the current position still counts as content.
        if self._has_virtuals and self.pos in self._virtuals:
            return 1
        return 0

    def _cur(self) -> str:
        if self._has_virtuals:
            v = self._virtuals.get(self.pos)
            if v is not None:
                return v
        return self.text[self.pos]

    def _advance(self, n: int = 1) -> None:
        text = self.text
        _len = self._len
        pos = self.pos
        if self._has_virtuals:
            virtuals = self._virtuals
            for _ in range(n):
                if pos in virtuals:
                    del virtuals[pos]
                    if not virtuals:
                        self._has_virtuals = False
                    continue
                if pos < _len:
                    if text[pos] == "\n":
                        self._line += 1
                        self._col = 0
                    else:
                        self._col += 1
                    pos += 1
        else:
            # Fast path — no virtual insertions (>99% of instances).
            for _ in range(n):
                if pos < _len:
                    if text[pos] == "\n":
                        self._line += 1
                        self._col = 0
                    else:
                        self._col += 1
                    pos += 1
        self.pos = pos

    def _position(self) -> SourcePosition:
        return SourcePosition(
            line=self._line, character=self._col, offset=self.pos + self._base_offset
        )

    def _pos_at(self, offset: int) -> SourcePosition:
        """Compute SourcePosition for an arbitrary offset using a line index."""
        line_starts = self._line_starts
        line_idx = _bisect_right(line_starts, offset) - 1
        col = offset - line_starts[line_idx]
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
        if not self._has_virtuals:
            text = self.text
            _len = self._len
            pos = self.pos
            col = self._col
            while pos < _len and text[pos] in _SEP_CHARS:
                col += 1
                pos += 1
            self.pos = pos
            self._col = col
        else:
            while self.remaining and self._cur() in _SEP_CHARS:
                self._advance()
        self._end = self.pos - 1
        self._type = TokenType.SEP

    def _parse_eol(self) -> None:
        self._start = self.pos
        if not self._has_virtuals:
            text = self.text
            _len = self._len
            pos = self.pos
            line = self._line
            col = self._col
            while pos < _len and text[pos] in " \t\n\r\x0b\x0c;":
                if text[pos] == "\n":
                    line += 1
                    col = 0
                else:
                    col += 1
                pos += 1
            self.pos = pos
            self._line = line
            self._col = col
        else:
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

        if not self._has_virtuals:
            # Fast path — direct text scanning.
            text = self.text
            _len = self._len
            pos = self.pos
            line = self._line
            col = self._col

            while pos < _len:
                ch = text[pos]
                if ch == '"' and blevel == 0:
                    in_quotes = not in_quotes
                    col += 1
                    pos += 1
                elif ch == "[" and blevel == 0 and not in_quotes:
                    level += 1
                    col += 1
                    pos += 1
                elif ch == "]" and blevel == 0 and not in_quotes:
                    level -= 1
                    if level == 0:
                        break
                    col += 1
                    pos += 1
                elif ch == "\\":
                    # Skip backslash.
                    col += 1
                    pos += 1
                    # Skip escaped char.
                    if pos < _len:
                        if text[pos] == "\n":
                            line += 1
                            col = 0
                        else:
                            col += 1
                        pos += 1
                elif ch == "$" and not in_quotes and blevel == 0:
                    # ${...} inside command — scan for matching }.
                    if pos + 1 < _len and text[pos + 1] == "{":
                        col += 1
                        pos += 1  # skip $
                        col += 1
                        pos += 1  # skip {
                        while pos < _len and text[pos] != "}":
                            if text[pos] == "\n":
                                line += 1
                                col = 0
                            else:
                                col += 1
                            pos += 1
                        if pos >= _len:
                            self.pos = pos
                            self._line = line
                            self._col = col
                            if _strict_quoting():
                                raise TclParseError("missing close-brace for variable name")
                            self.warnings.append(
                                (
                                    self._position(),
                                    "missing close-brace for variable name",
                                )
                            )
                        else:
                            # Advance past '}' — will be followed by the
                            # outer-loop advance that handles the next char.
                            if text[pos] == "\n":
                                line += 1
                                col = 0
                            else:
                                col += 1
                            pos += 1
                    else:
                        col += 1
                        pos += 1
                elif ch == "{" and not in_quotes:
                    blevel += 1
                    col += 1
                    pos += 1
                elif ch == "}" and not in_quotes:
                    if blevel:
                        blevel -= 1
                    col += 1
                    pos += 1
                else:
                    if ch == "\n":
                        line += 1
                        col = 0
                    else:
                        col += 1
                    pos += 1

            self.pos = pos
            self._line = line
            self._col = col
        else:
            # Slow path — virtual insertion support.
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
                    if self.pos + 1 < self._len and self.text[self.pos + 1] == "{":
                        self._advance()  # skip $
                        self._advance()  # skip {
                        while self.remaining and self._cur() != "}":
                            self._advance()
                        if not self.remaining:
                            if _strict_quoting():
                                raise TclParseError("missing close-brace for variable name")
                            self.warnings.append(
                                (
                                    self._position(),
                                    "missing close-brace for variable name",
                                )
                            )
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
            if _strict_quoting():
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
            elif _strict_quoting():
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
        _len = self._len
        # Accept alnum, underscore, :: (namespace separator)
        while self.remaining:
            ch = self._cur()
            if ch.isalnum() or ch == "_":
                self._advance()
            elif ch == ":" and self.pos + 1 < _len and self.text[self.pos + 1] == ":":
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
                elif ch == "$" and self.pos + 1 < _len and self.text[self.pos + 1] == "{":
                    # ${...} inside array index — scan for matching }
                    self._advance()  # skip $
                    self._advance()  # skip {
                    while self.remaining and self._cur() != "}":
                        self._advance()
                    if not self.remaining:
                        if _strict_quoting():
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
                if _strict_quoting():
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

        if not self._has_virtuals:
            # Fast path — direct text scanning without virtual checks.
            text = self.text
            _len = self._len
            pos = self.pos
            line = self._line
            col = self._col

            while pos < _len:
                ch = text[pos]
                if ch == "\\":
                    # Skip backslash (never a newline).
                    col += 1
                    pos += 1
                    # Skip escaped char if available.
                    if pos < _len:
                        if text[pos] == "\n":
                            line += 1
                            col = 0
                        else:
                            col += 1
                        pos += 1
                    continue
                elif ch == "}":
                    level -= 1
                    if level == 0:
                        self._end = pos - 1
                        # Advance past closing '}'.
                        col += 1
                        pos += 1
                        self.pos = pos
                        self._line = line
                        self._col = col
                        if pos < _len and text[pos] not in _AFTER_CLOSE_BRACE:
                            if (
                                text[pos] == "\\"
                                and pos + 1 < _len
                                and text[pos + 1] in ("\n", "\r")
                            ):
                                pass  # backslash-newline is a valid line continuation
                            elif TclLexer.irules_brace_separator and text[pos] == "{":
                                sep_pos = self._position()
                                self._pending_sep = Token(
                                    type=TokenType.SEP,
                                    text="",
                                    start=sep_pos,
                                    end=sep_pos,
                                )
                            elif _strict_quoting():
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
                # Advance — track line/col.
                if ch == "\n":
                    line += 1
                    col = 0
                else:
                    col += 1
                pos += 1

            # EOF without matching close-brace.
            self.pos = pos
            self._line = line
            self._col = col
            self._end = pos - 1
            if _strict_quoting():
                raise TclParseError("missing close-brace")
            self.warnings.append(
                (
                    self._position(),
                    "missing close-brace",
                )
            )
            self._type = TokenType.STR
            return

        # Slow path — virtual insertion support.
        while True:
            if not self.remaining:
                # EOF without matching close-brace
                self._end = self.pos - 1
                if _strict_quoting():
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
                    if self.remaining and self._cur() not in _AFTER_CLOSE_BRACE:
                        if (
                            self._cur() == "\\"
                            and self.remaining >= 2
                            and self.text[self.pos + 1] in ("\n", "\r")
                        ):
                            pass  # backslash-newline is a valid line continuation
                        elif TclLexer.irules_brace_separator and self._cur() == "{":
                            # iRules treats }{ as a word boundary — inject a
                            # zero-width SEP so the segmenter sees two words.
                            sep_pos = self._position()
                            self._pending_sep = Token(
                                type=TokenType.SEP,
                                text="",
                                start=sep_pos,
                                end=sep_pos,
                            )
                        elif _strict_quoting():
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
        _type = self._type
        newword = (
            _type is TokenType.SEP
            or _type is TokenType.EOL
            or _type is TokenType.STR
            or _type is TokenType.EXPAND
        )
        if newword and self.remaining and self._cur() == "{":
            # Check for {*} expansion prefix (Tcl 8.5+)
            _len = self._len
            if (
                TclLexer.expand_syntax
                and self.pos + 2 < _len
                and self.text[self.pos + 1] == "*"
                and self.text[self.pos + 2] == "}"
                # Must be followed by a non-separator (the word to expand)
                and self.pos + 3 < _len
                and self.text[self.pos + 3] not in " \t\n\r\x0b\x0c;"
            ):
                return self._parse_expand()
            return self._parse_brace()
        if newword and self.remaining and self._cur() == '"':
            self.insidequote = True
            self._advance()
        self._start = self.pos

        if not self._has_virtuals:
            # Fast path — direct text scanning.
            text = self.text
            _len = self._len
            pos = self.pos
            line = self._line
            col = self._col
            insidequote = self.insidequote

            while pos < _len:
                ch = text[pos]
                if ch == "\\":
                    if pos + 1 < _len:
                        # Advance past backslash.
                        col += 1
                        pos += 1
                        # Advance past escaped char.
                        if text[pos] == "\n":
                            line += 1
                            col = 0
                            pos += 1
                            # Backslash-newline at the very start of a
                            # new-word position is pure line-continuation
                            # whitespace.  Emit it as a SEP so the *next*
                            # token can recognise { or " as brace/quote
                            # delimiters (and plain words start fresh).
                            if newword and pos == self._start + 2:
                                self.pos = pos
                                self._line = line
                                self._col = col
                                self._end = pos - 1
                                self._type = TokenType.SEP
                                return
                            continue
                        else:
                            col += 1
                        pos += 1
                        continue
                    # Backslash at EOF — fall through to bottom advance.
                elif ch == "$" or ch == "[":
                    self._end = pos - 1
                    self._type = TokenType.ESC
                    self.pos = pos
                    self._line = line
                    self._col = col
                    return
                elif not insidequote and ch in _SEPARATOR_CHARS:
                    self._end = pos - 1
                    self._type = TokenType.ESC
                    self.pos = pos
                    self._line = line
                    self._col = col
                    return
                elif ch == '"' and insidequote:
                    self._end = pos - 1
                    self._type = TokenType.ESC
                    # Advance past closing '"'.
                    col += 1
                    pos += 1
                    self.insidequote = False
                    self.pos = pos
                    self._line = line
                    self._col = col
                    # After closing quote, next char must be separator or EOF.
                    if pos < _len and text[pos] not in _AFTER_CLOSE_QUOTE:
                        if text[pos] == "\\" and pos + 1 < _len and text[pos + 1] in ("\n", "\r"):
                            pass  # backslash-newline is a valid line continuation
                        elif _strict_quoting():
                            raise TclParseError("extra characters after close-quote")
                        else:
                            self.warnings.append(
                                (self._position(), "extra characters after close-quote")
                            )
                    return
                # Advance past current char.
                if ch == "\n":
                    line += 1
                    col = 0
                else:
                    col += 1
                pos += 1

            # EOF
            self.pos = pos
            self._line = line
            self._col = col
            if insidequote:
                if _strict_quoting():
                    raise TclParseError('missing "')
                self.warnings.append((self._position(), 'missing "'))
            self._end = pos - 1
            self._type = TokenType.ESC
            return

        # Slow path — virtual insertion support.
        while True:
            if not self.remaining:
                if self.insidequote:
                    if _strict_quoting():
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
                    next_ch = self.text[self.pos + 1] if self.pos + 1 < self._len else ""
                    if next_ch == "\n" and newword and self.pos == self._start:
                        # Backslash-newline at the very start of a
                        # new-word position — emit as SEP so the next
                        # token can recognise { or " delimiters.
                        self._advance(2)
                        self._end = self.pos - 1
                        self._type = TokenType.SEP
                        return
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
                    if self.remaining and self._cur() not in _AFTER_CLOSE_QUOTE:
                        if (
                            self._cur() == "\\"
                            and self.remaining >= 2
                            and self.text[self.pos + 1] in ("\n", "\r")
                        ):
                            pass  # backslash-newline is a valid line continuation
                        elif _strict_quoting():
                            raise TclParseError("extra characters after close-quote")
                        else:
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

        if not self._has_virtuals:
            # Fast path — direct text scanning.
            text = self.text
            _len = self._len
            pos = self.pos
            line = self._line
            col = self._col

            while pos < _len:
                ch = text[pos]
                if ch == "\\":
                    next_pos = pos + 1
                    if next_pos < _len:
                        next_ch = text[next_pos]
                        if next_ch == "\n":
                            # Backslash-newline continuation.
                            col += 1
                            pos += 1
                            line += 1
                            col = 0
                            pos += 1
                            continue
                        else:
                            # Backslash followed by another char.
                            col += 1
                            pos += 1
                            if text[pos] == "\n":
                                line += 1
                                col = 0
                            else:
                                col += 1
                            pos += 1
                            continue
                if ch == "\n":
                    break
                col += 1
                pos += 1

            self.pos = pos
            self._line = line
            self._col = col
        else:
            # Slow path — virtual insertion support.
            while self.remaining:
                ch = self._cur()
                if ch == "\\":
                    next_pos = self.pos + 1
                    if next_pos < self._len:
                        next_ch = self.text[next_pos]
                        if next_ch == "\n":
                            self._advance()  # skip backslash
                            self._advance()  # skip newline
                            continue
                        else:
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

        # Cache frequently-accessed values as locals.
        _base_offset = self._base_offset
        _SourcePosition = SourcePosition
        _Token = Token

        while True:
            pos = self.pos
            _len = self._len

            if pos >= _len and not (self._has_virtuals and pos in self._virtuals):
                if self._type is not TokenType.EOL and self._type is not TokenType.EOF:
                    self._type = TokenType.EOL
                else:
                    self._type = TokenType.EOF
                    return None
                p = _SourcePosition(
                    line=self._line,
                    character=self._col,
                    offset=pos + _base_offset,
                )
                return _Token(type=TokenType.EOL, text="", start=p, end=p)

            # Inline _position().
            start_pos = _SourcePosition(
                line=self._line,
                character=self._col,
                offset=pos + _base_offset,
            )

            # Inline _cur() for dispatch.
            if self._has_virtuals:
                ch = self._virtuals.get(pos)
                if ch is None:
                    ch = self.text[pos]
            else:
                ch = self.text[pos]

            # Dispatch — if/elif is faster than match/case for guarded
            # character checks because we test insidequote once up front.
            if ch == "[":
                self._parse_command()
            elif ch == "$":
                self._parse_var()
            elif not self.insidequote:
                if ch in _SEP_CHARS:
                    self._parse_sep()
                elif ch in _EOL_CHARS:
                    self._parse_eol()
                elif ch == "#" and self._at_command_start:
                    self._parse_comment()
                else:
                    self._parse_string()
            else:
                self._parse_string()

            # Inline _token_text().
            _start = self._start
            _end = self._end
            end_offset = _end if _end >= _start else _start
            if _end < _start:
                tok_text = ""
            else:
                tok_text = self.text[_start : _end + 1]

            # Inline _pos_at(end_offset).
            line_starts = self._line_starts
            line_idx = _bisect_right(line_starts, end_offset) - 1
            col = end_offset - line_starts[line_idx]
            _base_line = self._base_line
            end_pos = _SourcePosition(
                line=_base_line + line_idx,
                character=(self._base_col + col) if line_idx == 0 else col,
                offset=end_offset + _base_offset,
            )

            # Track command-start position for comment detection:
            # _at_command_start stays True through SEP/EOL/COMMENT tokens
            # but resets for any actual command content.
            _type = self._type
            if (
                _type is not TokenType.SEP
                and _type is not TokenType.EOL
                and _type is not TokenType.COMMENT
            ):
                self._at_command_start = False

            return _Token(
                type=_type,
                text=tok_text,
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
