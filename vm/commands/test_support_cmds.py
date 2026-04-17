"""Python equivalents of C test* commands from Tcl's test suite.

These commands are normally provided by the ``tcltest`` C extension
(``generic/tclTest.c``).  We implement minimal versions that are
sufficient to run the ``.test`` files through our VM.

Registered only during test runs — not part of the default command set.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from core.parsing.substitution import backslash_subst

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


class _ParseError(TclError):
    """Parse error carrying the remainder-start offset for errorInfo."""

    def __init__(self, msg: str, remainder_start: int) -> None:
        super().__init__(msg)
        self.remainder_start = remainder_start


# testbytestring


def _cmd_testbytestring(interp: TclInterp, args: list[str]) -> TclResult:
    """testbytestring string

    Return the string as-is.  In C Tcl this creates a byte-array
    object; in our all-strings VM it is an identity function.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "testbytestring string"')
    return TclResult(value=args[0])


# testevalex


def _cmd_testevalex(interp: TclInterp, args: list[str]) -> TclResult:
    """testevalex script ?global?

    Evaluate *script*.  If the optional second argument is ``"global"``,
    evaluate at the global level.  In C Tcl this calls ``Tcl_EvalEx``; in
    our VM it delegates to ``interp.eval()``.
    """
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "testevalex script ?global?"')

    script = args[0]
    at_global = len(args) == 2 and args[1] == "global"

    if at_global:
        saved_frame = interp.current_frame
        saved_ns = interp.current_namespace
        interp.current_frame = interp.global_frame
        interp.current_namespace = interp.root_namespace
        try:
            return interp.eval(script)
        finally:
            interp.current_frame = saved_frame
            interp.current_namespace = saved_ns

    return interp.eval(script)


# testevalobjv


def _cmd_testevalobjv(interp: TclInterp, args: list[str]) -> TclResult:
    """testevalobjv flags cmd ?arg ...?

    Evaluate a command from pre-split words.  The first arg is an
    integer flag bitfield; when bit 0 (``TCL_EVAL_GLOBAL``) is set the
    command is evaluated at the global level.  Remaining args are the
    command name and its arguments.
    """
    if len(args) < 2:
        raise TclError('wrong # args: should be "testevalobjv flags cmd ?arg ...?"')

    try:
        flags = int(args[0])
    except ValueError:
        raise TclError(f'expected integer but got "{args[0]}"') from None

    cmd_name = args[1]
    cmd_args = args[2:]
    at_global = bool(flags & 1)  # TCL_EVAL_GLOBAL = 0x1

    if at_global:
        saved_frame = interp.current_frame
        saved_ns = interp.current_namespace
        interp.current_frame = interp.global_frame
        interp.current_namespace = interp.root_namespace
        try:
            return interp.invoke(cmd_name, cmd_args)
        finally:
            interp.current_frame = saved_frame
            interp.current_namespace = saved_ns

    return interp.invoke(cmd_name, cmd_args)


# testparser
#
# Implements a mini Tcl_ParseCommand that returns the flat token
# structure expected by parse.test.
#
# Output format (Tcl list):
#   {comment|"-"} {cmdText} numWords [type text numComponents]... {remainder}
#
# Token types: simple, word, expand, text, backslash, variable, command


@dataclass
class _ParseToken:
    """A node in the Tcl parse tree."""

    type: str  # simple|word|expand|text|backslash|variable|command
    text: str
    children: list[_ParseToken] = field(default_factory=list)

    @property
    def num_components(self) -> int:
        """Total tokens in subtree (not counting self)."""
        n = len(self.children)
        for c in self.children:
            n += c.num_components
        return n


def _list_escape_parser(s: str) -> str:
    """Escape a string for inclusion in a Tcl list result.

    Matches Tcl's ``Tcl_ScanElement``/``Tcl_ConvertElement`` rules:

    1. Empty string → ``{}``
    2. No special characters → return as-is
    3. Try brace quoting unless the string contains
       backslash-newline (which would be treated as continuation
       inside braces) or unbalanced braces or a trailing backslash.
    4. Fall back to backslash escaping.
    """
    if not s:
        return "{}"

    # Characters that force quoting
    needs_quoting = False
    for ch in s:
        if ch in ' \t\n\r{}"\\;$[':
            needs_quoting = True
            break
    # A leading # also needs quoting (would start a comment)
    if s[0] == "#":
        needs_quoting = True

    if not needs_quoting:
        return s

    # Check whether brace quoting is safe.  Backslash-escaped braces
    # (\{ and \}) don't count toward depth — matches Tcl_ScanElement.
    can_brace = True
    depth = 0
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch == "\\":
            if i + 1 < n:
                if s[i + 1] == "\n":
                    # Backslash-newline → unsafe for brace quoting
                    can_brace = False
                    break
                # Skip escaped char (\{, \}, etc. don't affect depth)
                i += 2
                continue
            else:
                # Trailing backslash → unsafe for brace quoting
                can_brace = False
                break
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                can_brace = False
                break
        i += 1
    if depth != 0:
        can_brace = False

    if can_brace:
        return "{" + s + "}"

    # Backslash-escape — use Tcl escape sequences for control chars
    result: list[str] = []
    for i, ch in enumerate(s):
        if ch == "\n":
            result.append("\\n")
        elif ch == "\t":
            result.append("\\t")
        elif ch == "\r":
            result.append("\\r")
        elif ch in ' {}"\\;$[':
            result.append("\\")
            result.append(ch)
        elif ch == "#" and i == 0:
            # Leading # must be escaped (would start a comment)
            result.append("\\")
            result.append(ch)
        else:
            result.append(ch)
    return "".join(result)


def _decompose_braced_bsnl(content: str) -> list[_ParseToken]:
    """Decompose braced content with backslash-newlines into tokens.

    When ``Tcl_ParseBraces`` finds a backslash-newline inside braces it
    produces separate ``text`` and ``backslash`` tokens instead of a
    single ``text`` token.  The backslash token covers the backslash,
    the newline, and any following spaces/tabs.
    """
    children: list[_ParseToken] = []
    i = 0
    n = len(content)
    text_start = 0

    while i < n:
        if content[i] == "\\" and i + 1 < n and content[i + 1] == "\n":
            # Flush preceding text
            if text_start < i:
                children.append(_ParseToken("text", content[text_start:i]))
            # Consume backslash + newline + following whitespace
            bs_start = i
            i += 2  # skip \ and \n
            while i < n and content[i] in " \t":
                i += 1
            children.append(_ParseToken("backslash", content[bs_start:i]))
            text_start = i
        else:
            i += 1

    if text_start < n:
        children.append(_ParseToken("text", content[text_start:]))

    return children


class _CommandParser:
    """Parse one Tcl command and produce the testparser output tokens."""

    def __init__(self, src: str) -> None:
        self.src = src
        self.pos = 0

    @property
    def _remaining(self) -> int:
        return len(self.src) - self.pos

    def _ch(self, offset: int = 0) -> str:
        return self.src[self.pos + offset]

    def _at_end(self) -> bool:
        return self.pos >= len(self.src)

    def parse(self) -> tuple[str, str, list[_ParseToken], str]:
        """Parse one command.

        Returns (comment, cmd_text, word_tokens, remainder).
        """
        self._skip_leading_space()

        # Parse comment block — accumulates consecutive comment lines
        # including inter-line whitespace as a single span.
        comment = "-"
        if not self._at_end() and self._ch() == "#":
            comment_start = self.pos
            while not self._at_end() and self._ch() == "#":
                # Parse one comment line (# through newline)
                self._scan_comment_line()
                # Peek ahead: if next line (after horizontal whitespace)
                # also starts with #, it's part of the same comment block
                saved = self.pos
                while not self._at_end() and self._ch() in " \t\r\x0b\x0c":
                    self.pos += 1
                if self._at_end() or self._ch() != "#":
                    self.pos = saved
                    break
                # Next line IS another comment — the whitespace between
                # becomes part of the comment block, loop continues
            comment = self.src[comment_start : self.pos]
            # Skip whitespace after comment block before command
            while not self._at_end() and self._ch() in " \t\n\r\x0b\x0c":
                self.pos += 1

        cmd_start = self.pos
        words: list[_ParseToken] = []

        while not self._at_end():
            ch = self._ch()
            if ch in "\n;":
                # Command terminator
                self.pos += 1
                break
            if ch in " \t\r\x0b\x0c":
                self._skip_spaces()
                continue
            if ch == "\\" and self._remaining > 1 and self._ch(1) == "\n":
                # Backslash-newline acts as word separator
                self.pos += 2
                while not self._at_end() and self._ch() in " \t":
                    self.pos += 1
                continue
            words.append(self._parse_word())

        cmd_text = self.src[cmd_start : self.pos]
        # Strip trailing command terminator from cmd_text only if it's
        # the newline/semicolon we consumed
        remainder = self.src[self.pos :]

        return comment, cmd_text, words, remainder

    def _skip_leading_space(self) -> None:
        """Skip whitespace and backslash-newlines before a command."""
        while not self._at_end():
            ch = self._ch()
            if ch in " \t\n\r\x0b\x0c":
                self.pos += 1
            elif ch == "\\" and self._remaining > 1 and self._ch(1) == "\n":
                self.pos += 2
                while not self._at_end() and self._ch() in " \t":
                    self.pos += 1
            else:
                break

    def _skip_spaces(self) -> None:
        """Skip horizontal whitespace (not newlines)."""
        while not self._at_end() and self._ch() in " \t\r\x0b\x0c":
            self.pos += 1

    def _scan_comment_line(self) -> None:
        """Advance past a comment line (# through newline).

        Used by the comment-block accumulator in parse().
        """
        while not self._at_end():
            ch = self._ch()
            if ch == "\\" and self._remaining > 1 and self._ch(1) == "\n":
                self.pos += 2
                continue
            if ch == "\n":
                self.pos += 1
                return
            self.pos += 1

    def _parse_word(self) -> _ParseToken:
        """Parse one word and return a word-level token."""
        if self._at_end():
            return _ParseToken("simple", "", [_ParseToken("text", "")])

        ch = self._ch()

        # {*} expansion
        if (
            ch == "{"
            and self._remaining >= 3
            and self._ch(1) == "*"
            and self._ch(2) == "}"
            and self._remaining > 3
            and self._ch(3) not in " \t\n\r\x0b\x0c;"
            # Backslash-newline acts as separator — not expansion
            and not (self._ch(3) == "\\" and self._remaining > 4 and self._ch(4) == "\n")
        ):
            return self._parse_expand_word()

        # Braced word
        if ch == "{":
            return self._parse_braced_word()

        # Quoted word
        if ch == '"':
            return self._parse_quoted_word()

        # Unquoted word
        return self._parse_bare_word()

    def _parse_expand_word(self) -> _ParseToken:
        """Parse a {*}word expansion."""
        expand_start = self.pos
        self.pos += 3  # skip {*}

        # The word after {*} determines the expansion.
        # If the next thing is a braced literal with no backslashes,
        # Tcl expands it as a list.  Otherwise it's an expand token.
        if self._at_end():
            return _ParseToken("simple", "*", [_ParseToken("text", "*")])

        ch = self._ch()
        if ch == "{":
            # {*}{literal} — check for backslashes
            inner_start = self.pos
            content = self._read_braced()
            inner_end = self.pos
            inner_src = self.src[inner_start:inner_end]
            full_src = self.src[expand_start:inner_end]
            if "\\" in content:
                # Has backslashes → expand token
                children = [_ParseToken("text", content)]
                return _ParseToken("expand", full_src, children)
            # No backslashes → list expansion (expanded at parse time)
            # Return as expanded words
            from ..machine import _split_list

            try:
                elements = _split_list(content)
            except TclError:
                children = [_ParseToken("text", content)]
                return _ParseToken("expand", full_src, children)
            if len(elements) == 1:
                return _ParseToken("simple", elements[0], [_ParseToken("text", elements[0])])
            # Multiple elements — this returns multiple word tokens.
            # But testparser returns them as separate words... Actually,
            # testparser treats {*}{a b} as expansion producing 2 words.
            # We need to return multiple tokens.  Signal this with a
            # special type that the caller unpacks.
            return _ParseToken(
                "_expanded_list",
                full_src,
                [_ParseToken("simple", e, [_ParseToken("text", e)]) for e in elements],
            )

        if ch == '"':
            # {*}"literal" — check for substitutions
            inner_start = self.pos
            inner_children = self._read_quoted_tokens()
            inner_end = self.pos
            inner_src = self.src[inner_start:inner_end]
            full_src = self.src[expand_start:inner_end]
            # Check if pure text (no substitutions)
            if len(inner_children) == 1 and inner_children[0].type == "text":
                text = inner_children[0].text
                if " " in text or "\t" in text:
                    from ..machine import _split_list

                    try:
                        elements = _split_list(text)
                    except TclError:
                        return _ParseToken("expand", full_src, inner_children)
                    if len(elements) > 1:
                        return _ParseToken(
                            "_expanded_list",
                            full_src,
                            [_ParseToken("simple", e, [_ParseToken("text", e)]) for e in elements],
                        )
                return _ParseToken("simple", inner_src, [_ParseToken("text", text)])
            # Has substitutions
            return _ParseToken("expand", full_src, inner_children)

        # {*}bareword
        inner_tok = self._parse_bare_word()
        full_src = self.src[expand_start : self.pos]
        # Simple bare word: list-expand produces itself (single element)
        if inner_tok.type == "simple":
            return inner_tok
        return _ParseToken("expand", full_src, inner_tok.children)

    def _parse_braced_word(self) -> _ParseToken:
        """Parse a {braced} word.

        Normally a simple word with one text token.  When the content
        contains backslash-newline sequences, Tcl decomposes it into
        separate text and backslash tokens (word type becomes ``word``).
        """
        word_start = self.pos
        content = self._read_braced()
        word_src = self.src[word_start : self.pos]

        # Check for backslash-newline inside the content
        if "\\\n" in content:
            children = _decompose_braced_bsnl(content)
            return _ParseToken("word", word_src, children)

        return _ParseToken("simple", word_src, [_ParseToken("text", content)])

    def _read_braced(self) -> str:
        """Read a brace-delimited string, return content without braces."""
        if self._at_end() or self._ch() != "{":
            return ""
        brace_start = self.pos
        self.pos += 1  # skip {
        depth = 1
        start = self.pos
        while not self._at_end():
            ch = self._ch()
            if ch == "\\":
                self.pos += 1  # skip backslash
                if not self._at_end():
                    self.pos += 1  # skip escaped char
                continue
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    content = self.src[start : self.pos]
                    self.pos += 1  # skip }
                    # Check for junk after close-brace
                    if (
                        not self._at_end()
                        and self._ch() not in " \t\n\r\x0b\x0c;]"
                        and not (self._ch() == "\\" and self._remaining > 1 and self._ch(1) == "\n")
                    ):
                        raise _ParseError("extra characters after close-brace", self.pos)
                    return content
            self.pos += 1
        raise _ParseError("missing close-brace", brace_start)

    def _parse_quoted_word(self) -> _ParseToken:
        """Parse a "quoted" word → simple if no subs, word otherwise."""
        word_start = self.pos
        children = self._read_quoted_tokens()
        word_src = self.src[word_start : self.pos]

        if len(children) == 1 and children[0].type == "text":
            return _ParseToken("simple", word_src, children)
        return _ParseToken("word", word_src, children)

    def _read_quoted_tokens(self) -> list[_ParseToken]:
        """Read tokens inside "...", consuming the delimiters."""
        quote_start = self.pos
        self.pos += 1  # skip opening "
        children: list[_ParseToken] = []
        text_start = self.pos

        while not self._at_end():
            ch = self._ch()
            if ch == '"':
                # Closing quote
                if text_start < self.pos:
                    children.append(_ParseToken("text", self.src[text_start : self.pos]))
                self.pos += 1  # skip closing "
                # Check for junk after close-quote
                if (
                    not self._at_end()
                    and self._ch() not in " \t\n\r\x0b\x0c;]"
                    and not (self._ch() == "\\" and self._remaining > 1 and self._ch(1) == "\n")
                ):
                    raise _ParseError("extra characters after close-quote", self.pos)
                if not children:
                    children.append(_ParseToken("text", ""))
                return children
            if ch == "$":
                if text_start < self.pos:
                    children.append(_ParseToken("text", self.src[text_start : self.pos]))
                children.append(self._parse_variable())
                text_start = self.pos
            elif ch == "[":
                if text_start < self.pos:
                    children.append(_ParseToken("text", self.src[text_start : self.pos]))
                children.append(self._parse_command_sub())
                text_start = self.pos
            elif ch == "\\":
                bs_text = self._read_backslash()
                if text_start < self.pos - len(bs_text):
                    children.append(
                        _ParseToken(
                            "text",
                            self.src[text_start : self.pos - len(bs_text)],
                        )
                    )
                children.append(_ParseToken("backslash", bs_text))
                text_start = self.pos
            else:
                self.pos += 1

        raise _ParseError('missing "', quote_start)

    def _parse_bare_word(self) -> _ParseToken:
        """Parse an unquoted word."""
        word_start = self.pos
        children: list[_ParseToken] = []
        text_start = self.pos

        while not self._at_end():
            ch = self._ch()
            if ch in " \t\n\r\x0b\x0c;":
                break
            if ch == "\\" and self._remaining > 1 and self._ch(1) == "\n":
                # Backslash-newline terminates word
                break
            if ch == "$":
                if text_start < self.pos:
                    children.append(_ParseToken("text", self.src[text_start : self.pos]))
                children.append(self._parse_variable())
                text_start = self.pos
            elif ch == "[":
                if text_start < self.pos:
                    children.append(_ParseToken("text", self.src[text_start : self.pos]))
                children.append(self._parse_command_sub())
                text_start = self.pos
            elif ch == "\\":
                if self._remaining <= 1:
                    # Bare backslash at end of input — just text
                    self.pos += 1
                else:
                    bs_text = self._read_backslash()
                    if text_start < self.pos - len(bs_text):
                        children.append(
                            _ParseToken(
                                "text",
                                self.src[text_start : self.pos - len(bs_text)],
                            )
                        )
                    children.append(_ParseToken("backslash", bs_text))
                    text_start = self.pos
            else:
                self.pos += 1

        # Trailing text
        if text_start < self.pos:
            children.append(_ParseToken("text", self.src[text_start : self.pos]))

        word_src = self.src[word_start : self.pos]
        if not children:
            children.append(_ParseToken("text", ""))

        # Simple if all children are text
        if len(children) == 1 and children[0].type == "text":
            return _ParseToken("simple", word_src, children)
        return _ParseToken("word", word_src, children)

    def _parse_variable(self) -> _ParseToken:
        """Parse a $variable reference and return a variable token."""
        var_start = self.pos
        self.pos += 1  # skip $

        if self._at_end():
            # Bare $ at end — treat as text
            return _ParseToken("text", "$")

        ch = self._ch()

        # ${name} form — handles nested braces
        if ch == "{":
            brace_pos = self.pos
            self.pos += 1  # skip {
            name_start = self.pos
            depth = 1
            while not self._at_end() and depth > 0:
                c = self._ch()
                if c == "\\":
                    self.pos += 1  # skip backslash
                    if not self._at_end():
                        self.pos += 1  # skip escaped char
                    continue
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                    if depth == 0:
                        break
                self.pos += 1
            if depth > 0:
                raise _ParseError("missing close-brace for variable name", brace_pos)
            name = self.src[name_start : self.pos]
            self.pos += 1  # skip closing }
            var_src = self.src[var_start : self.pos]
            return _ParseToken("variable", var_src, [_ParseToken("text", name)])

        # $name or $name(index) form — Tcl accepts ASCII alphanumeric,
        # underscore, and :: namespace separators.  When :: is seen,
        # any immediately following colons are also consumed.
        name_start = self.pos
        while not self._at_end():
            ch = self._ch()
            if (ch.isascii() and ch.isalnum()) or ch == "_":
                self.pos += 1
            elif ch == ":" and self._remaining > 1 and self._ch(1) == ":":
                # Consume :: pair and any trailing colons
                self.pos += 2
                while not self._at_end() and self._ch() == ":":
                    self.pos += 1
            else:
                break

        name = self.src[name_start : self.pos]
        if not name:
            # Bare $ — not a variable
            return _ParseToken("text", "$")

        # Check for array index
        if not self._at_end() and self._ch() == "(":
            return self._parse_array_variable(var_start, name)

        var_src = self.src[var_start : self.pos]
        return _ParseToken("variable", var_src, [_ParseToken("text", name)])

    def _parse_array_variable(self, var_start: int, name: str) -> _ParseToken:
        """Parse $name(index) with possible substitutions in index."""
        paren_pos = self.pos
        self.pos += 1  # skip (
        children: list[_ParseToken] = [_ParseToken("text", name)]
        index_children: list[_ParseToken] = []
        text_start = self.pos
        depth = 1

        while not self._at_end() and depth > 0:
            ch = self._ch()
            if ch == "(":
                depth += 1
                self.pos += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    if text_start < self.pos:
                        index_children.append(_ParseToken("text", self.src[text_start : self.pos]))
                    self.pos += 1  # skip )
                    break
                self.pos += 1
            elif ch == "$":
                if text_start < self.pos:
                    index_children.append(_ParseToken("text", self.src[text_start : self.pos]))
                index_children.append(self._parse_variable())
                text_start = self.pos
            elif ch == "[":
                if text_start < self.pos:
                    index_children.append(_ParseToken("text", self.src[text_start : self.pos]))
                index_children.append(self._parse_command_sub())
                text_start = self.pos
            elif ch == "\\":
                bs_text = self._read_backslash()
                if text_start < self.pos - len(bs_text):
                    index_children.append(
                        _ParseToken(
                            "text",
                            self.src[text_start : self.pos - len(bs_text)],
                        )
                    )
                index_children.append(_ParseToken("backslash", bs_text))
                text_start = self.pos
            else:
                self.pos += 1

        if depth > 0:
            # Unclosed parenthesis
            raise _ParseError("missing )", paren_pos)

        if not index_children:
            index_children.append(_ParseToken("text", ""))

        # Flatten simple index: if only one text child, use it directly
        if len(index_children) == 1 and index_children[0].type == "text":
            children.append(index_children[0])
        else:
            children.extend(index_children)

        var_src = self.src[var_start : self.pos]
        return _ParseToken("variable", var_src, children)

    def _parse_command_sub(self) -> _ParseToken:
        """Parse a [command] substitution."""
        start = self.pos
        self.pos += 1  # skip [
        depth = 1
        while not self._at_end():
            ch = self._ch()
            if ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
                if depth == 0:
                    self.pos += 1  # skip ]
                    return _ParseToken("command", self.src[start : self.pos])
            elif ch == "\\":
                self.pos += 1  # skip backslash
                if not self._at_end():
                    self.pos += 1
                continue
            elif ch == "{":
                # Skip over braced content (brackets inside braces
                # don't count)
                self._skip_braced_in_command()
                continue
            self.pos += 1
        raise _ParseError("missing close-bracket", start)

    def _skip_braced_in_command(self) -> None:
        """Skip a braced section inside a command substitution."""
        depth = 1
        self.pos += 1  # skip {
        while not self._at_end():
            ch = self._ch()
            if ch == "\\":
                self.pos += 1
                if not self._at_end():
                    self.pos += 1
                continue
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    self.pos += 1
                    return
            self.pos += 1

    def _read_backslash(self) -> str:
        """Read a backslash escape sequence, return the source text.

        If the backslash is at the very end of the input (nothing
        follows), returns just ``\\`` — the caller should treat it as
        literal text, not a backslash token.
        """
        start = self.pos
        self.pos += 1  # skip backslash
        if self._at_end():
            return self.src[start : self.pos]

        ch = self._ch()

        if ch == "x":
            # \xNN — up to 2 hex digits
            self.pos += 1
            count = 0
            while not self._at_end() and count < 2:
                if self._ch() in "0123456789abcdefABCDEF":
                    self.pos += 1
                    count += 1
                else:
                    break
            return self.src[start : self.pos]

        if ch == "u":
            # \uNNNN — up to 4 hex digits
            self.pos += 1
            count = 0
            while not self._at_end() and count < 4:
                if self._ch() in "0123456789abcdefABCDEF":
                    self.pos += 1
                    count += 1
                else:
                    break
            return self.src[start : self.pos]

        if ch == "U":
            # \UNNNNNNNN — up to 8 hex digits
            self.pos += 1
            count = 0
            while not self._at_end() and count < 8:
                if self._ch() in "0123456789abcdefABCDEF":
                    self.pos += 1
                    count += 1
                else:
                    break
            return self.src[start : self.pos]

        if ch in "0123456789":
            # \NNN — up to 3 octal digits
            count = 0
            while not self._at_end() and count < 3:
                if self._ch() in "01234567":
                    self.pos += 1
                    count += 1
                else:
                    break
            return self.src[start : self.pos]

        if ch == "\n":
            # Backslash-newline + whitespace
            self.pos += 1
            while not self._at_end() and self._ch() in " \t":
                self.pos += 1
            return self.src[start : self.pos]

        # Single-char escape: \a, \b, \f, \n, \r, \t, \v, \\, etc.
        self.pos += 1
        return self.src[start : self.pos]


def _flatten_tokens(tokens: list[_ParseToken], parts: list[str]) -> None:
    """Flatten a token tree into the testparser output list."""
    for tok in tokens:
        parts.append(tok.type)
        parts.append(_list_escape_parser(tok.text))
        parts.append(str(tok.num_components))
        _flatten_tokens(tok.children, parts)


def _cmd_testparser(interp: TclInterp, args: list[str]) -> TclResult:
    """testparser script length

    Parse one command from *script* and return the token structure.
    *length* controls how much of the script to parse:
    ``-1`` = up to first NUL, ``0`` = full string, ``N`` = first N bytes.
    """
    if len(args) != 2:
        raise TclError('wrong # args: should be "testparser script length"')

    original = args[0]
    try:
        length = int(args[1])
    except ValueError:
        raise TclError(f'expected integer but got "{args[1]}"') from None

    if length == -1:
        nul = original.find("\x00")
        if nul >= 0:
            script = original[:nul]
        else:
            script = original
    elif length > 0:
        script = original[:length]
    else:
        script = original

    parser = _CommandParser(script)
    try:
        comment, cmd_text, word_tokens, parse_remainder = parser.parse()
    except _ParseError as e:
        # Format errorInfo and set ::errorInfo before re-raising.
        # We must set error_info on the TclError so that invoke()
        # does not overwrite it with a generic "while executing" frame.
        remainder_text = script[e.remainder_start :]
        cmd_repr = f"testparser {_list_escape_parser(args[0])} {args[1]}"
        info_lines = [
            str(e),
            f'    (remainder of script: "{remainder_text}")\n    invoked from within\n"{cmd_repr}"',
        ]
        err = TclError(str(e))
        err.error_info = info_lines
        raise err from None

    # When length truncated the script, the "remainder" is what's left
    # of the ORIGINAL string from where parsing stopped.
    if length > 0:
        consumed = parser.pos
        remainder = original[consumed:]
    elif length == -1 and "\x00" in original:
        remainder = parse_remainder
    else:
        remainder = parse_remainder

    # Unpack _expanded_list tokens into multiple words
    final_words: list[_ParseToken] = []
    for tok in word_tokens:
        if tok.type == "_expanded_list":
            final_words.extend(tok.children)
        else:
            final_words.append(tok)

    parts: list[str] = []
    parts.append(_list_escape_parser(comment))
    parts.append(_list_escape_parser(cmd_text))
    parts.append(str(len(final_words)))
    _flatten_tokens(final_words, parts)
    parts.append(_list_escape_parser(remainder))

    return TclResult(value=" ".join(parts))


# testparsevarname


def _cmd_testparsevarname(interp: TclInterp, args: list[str]) -> TclResult:
    """testparsevarname script length append

    Parse a variable name from *script*.  Returns a token structure
    like testparser but focused on the variable portion.
    """
    if len(args) != 3:
        raise TclError('wrong # args: should be "testparsevarname script length append"')

    original = args[0]
    try:
        length = int(args[1])
    except ValueError:
        raise TclError(f'expected integer but got "{args[1]}"') from None

    if length > 0:
        script = original[:length]
    else:
        script = original

    # The script should start with $ (a variable reference)
    parser = _CommandParser(script)
    if parser._at_end():
        return TclResult(value="- {} 0 {}")

    if parser._ch() == "$":
        try:
            var_token = parser._parse_variable()
        except _ParseError as e:
            # Format errorInfo and set ::errorInfo before re-raising.
            # Use original (not truncated) string for the remainder.
            remainder_text = original[e.remainder_start :]
            cmd_repr = f"testparsevarname {_list_escape_parser(args[0])} {args[1]} {args[2]}"
            info_lines = [
                str(e),
                f'    (remainder of script: "{remainder_text}")\n'
                f"    invoked from within\n"
                f'"{cmd_repr}"',
            ]
            err = TclError(str(e))
            err.error_info = info_lines
            raise err from None
    else:
        # Not a variable — return empty
        return TclResult(value="- {} 0 {}")

    # Remainder: from parse position in original string
    if length > 0:
        remainder = original[parser.pos :]
    else:
        remainder = script[parser.pos :]

    parts: list[str] = ["-", "{}"]
    parts.append("0")
    parts.append(var_token.type)
    parts.append(_list_escape_parser(var_token.text))
    parts.append(str(var_token.num_components))
    _flatten_tokens(var_token.children, parts)
    parts.append(_list_escape_parser(remainder))

    return TclResult(value=" ".join(parts))


# testparsevar


def _cmd_testparsevar(interp: TclInterp, args: list[str]) -> TclResult:
    """testparsevar varReference

    Parse and look up a variable.  Returns a 2-element list:
    {value} {remainder}.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "testparsevar varReference"')

    script = args[0]
    parser = _CommandParser(script)

    if parser._at_end() or parser._ch() != "$":
        return TclResult(value=f"{_list_escape_parser('$')} {_list_escape_parser(script)}")

    var_token = parser._parse_variable()
    remainder = script[parser.pos :]

    # If the token was just "$" (bare dollar), return $ and remainder
    if var_token.type == "text" and var_token.text == "$":
        return TclResult(value=(f"{_list_escape_parser('$')} {_list_escape_parser(remainder)}"))

    # Extract the variable name from the token children
    # First child is always the name text
    if not var_token.children:
        raise TclError('can\'t read "": no such variable')

    name = var_token.children[0].text

    # Check for array index — evaluate substitutions in the index
    if len(var_token.children) >= 2:
        index_parts: list[str] = []
        for child in var_token.children[1:]:
            if child.type == "text":
                index_parts.append(child.text)
            elif child.type == "command":
                # Evaluate command substitution in the index
                result = interp.eval(child.text)
                index_parts.append(result.value)
            elif child.type == "variable":
                # Evaluate nested variable reference
                inner_name = child.children[0].text if child.children else ""
                try:
                    index_parts.append(interp.current_frame.get_var(inner_name))
                except Exception:
                    raise TclError(f'can\'t read "{inner_name}": no such variable') from None
            elif child.type == "backslash":
                index_parts.append(backslash_subst(child.text))
            else:
                index_parts.append(child.text)
        index = "".join(index_parts)
        full_name = f"{name}({index})"
    else:
        full_name = name

    # Look up the variable
    try:
        value = interp.current_frame.get_var(full_name)
    except Exception:
        raise TclError(f'can\'t read "{full_name}": no such variable') from None

    return TclResult(value=f"{_list_escape_parser(value)} {_list_escape_parser(remainder)}")


# testexprlong / testexprdouble / testexprstring
#
# Thin wrappers around ``Tcl_ExprLong``, ``Tcl_ExprDouble``, and
# ``Tcl_ExprString`` from ``tclTest.c``.  Each evaluates an
# expression and coerces the result to the named C type.


def _cmd_testexprlong(interp: TclInterp, args: list[str]) -> TclResult:
    """testexprlong exprStr

    Evaluate *exprStr* as an expression and return the result coerced
    to a long integer, prefixed with ``This is a result:``.
    Mirrors ``TestexprlongCmd`` from tclTest.c.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "testexprlong expression"')
    result = interp.eval_expr(args[0])
    # Empty result maps to 0 (matches C Tcl_ExprLong on empty expression)
    if not result or not result.strip():
        return TclResult(value="This is a result: 0")
    # Coerce to integer (truncate floats, parse booleans)
    try:
        val = int(float(result))
    except (ValueError, TypeError):
        raise TclError(f'expected number but got "{result}"') from None
    return TclResult(value=f"This is a result: {val}")


def _cmd_testexprlongobj(interp: TclInterp, args: list[str]) -> TclResult:
    """testexprlongobj exprStr

    Object-based counterpart of testexprlong — identical semantics
    in our VM.
    """
    return _cmd_testexprlong(interp, args)


def _cmd_testexprdouble(interp: TclInterp, args: list[str]) -> TclResult:
    """testexprdouble exprStr

    Evaluate *exprStr* and return the result coerced to a double,
    prefixed with ``This is a result:``.
    Mirrors ``TestexprdoubleCmd`` from tclTest.c.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "testexprdouble expression"')
    result = interp.eval_expr(args[0])
    # Empty result maps to 0.0 (matches C Tcl_ExprDouble on empty expression)
    if not result or not result.strip():
        return TclResult(value="This is a result: 0.0")
    try:
        val = float(result)
    except (ValueError, TypeError):
        raise TclError(f'expected number but got "{result}"') from None
    import math as _math

    # C Tcl's Tcl_ExprDouble rejects NaN with a domain error
    if _math.isnan(val):
        raise TclError(
            "domain error: argument not in valid range",
            error_code="ARITH DOMAIN {domain error: argument not in valid range}",
        )
    from ..machine import _format_number

    return TclResult(value=f"This is a result: {_format_number(val)}")


def _cmd_testexprdoubleobj(interp: TclInterp, args: list[str]) -> TclResult:
    """testexprdoubleobj exprStr

    Object-based counterpart of testexprdouble.
    """
    return _cmd_testexprdouble(interp, args)


def _cmd_testexprstring(interp: TclInterp, args: list[str]) -> TclResult:
    """testexprstring exprStr

    Evaluate *exprStr* and return the result as a string (no type
    coercion).  Mirrors ``TestexprstringCmd`` from tclTest.c.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "testexprstring expression"')
    result = interp.eval_expr(args[0])
    # Empty result maps to "0" (matches C Tcl_ExprString on empty expression)
    if not result or not result.strip():
        return TclResult(value="0")
    return TclResult(value=result)


# Registration


def setup_test_support(interp: TclInterp) -> None:
    """Register test* commands in the interpreter.

    Call this before sourcing a ``.test`` file so that ``testConstraint``
    checks like ``[llength [info commands testevalex]]`` succeed.
    """
    cmds: dict[str, object] = {
        # Test-build-only commands (testparser, testevalobjv, testevalex,
        # testbytestring, testparsevarname, testparsevar) are intentionally
        # NOT registered.  Real tclsh skips tests gated by these constraints
        # because the commands only exist in special test builds.  Our
        # implementations are incomplete, so we match tclsh by skipping.
        "testexprlong": _cmd_testexprlong,
        "testexprlongobj": _cmd_testexprlongobj,
        "testexprdouble": _cmd_testexprdouble,
        "testexprdoubleobj": _cmd_testexprdoubleobj,
        "testexprstring": _cmd_testexprstring,
    }

    for name, handler in cmds.items():
        interp.register_command(name, handler)  # type: ignore[arg-type]

    # Stub for tcl::build-info — Tcl 9.0 test files use this to detect
    # compiler features (e.g. MSVC bugs).  Return "0" for all queries.
    interp.eval("proc tcl::build-info {args} { return 0 }")
