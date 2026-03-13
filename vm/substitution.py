"""Runtime substitution engine for the Tcl VM.

Performs variable, command, and backslash substitution on Tcl strings.
Used by ``EVAL_STK``, the ``subst`` command, and any other path that
needs to resolve ``$var`` / ``[cmd ...]`` at runtime.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from core.parsing.lexer import TclLexer
from core.parsing.substitution import _BACKSLASH_MAP, backslash_subst
from core.parsing.tokens import TokenType

from .types import TclBreak, TclContinue, TclError, TclReturn


def _split_tcl_commands(script: str) -> list[str]:
    """Split a Tcl script into individual commands.

    Respects braces, double quotes, and backslash escapes so that
    ``;`` and newlines inside grouping constructs are not treated as
    command separators.  Returns a list of command strings (may include
    leading/trailing whitespace).
    """
    commands: list[str] = []
    i = 0
    n = len(script)
    start = 0

    while i < n:
        ch = script[i]
        if ch == "{":
            # Skip matched braces
            depth = 1
            i += 1
            while i < n and depth > 0:
                if script[i] == "\\":
                    i += 2
                    continue
                if script[i] == "{":
                    depth += 1
                elif script[i] == "}":
                    depth -= 1
                i += 1
        elif ch == '"':
            # Skip quoted string
            i += 1
            while i < n and script[i] != '"':
                if script[i] == "\\":
                    i += 1
                i += 1
            if i < n:
                i += 1  # skip closing "
        elif ch == "[":
            # Skip command substitution
            depth = 1
            i += 1
            while i < n and depth > 0:
                if script[i] == "[":
                    depth += 1
                elif script[i] == "]":
                    depth -= 1
                elif script[i] == "\\":
                    i += 1
                i += 1
        elif ch == "\\":
            i += 2  # skip escaped char
        elif ch == "#" and script[start:i].strip() == "":
            # Comment at start of command — skip to end of line
            while i < n and script[i] != "\n":
                if script[i] == "\\" and i + 1 < n and script[i + 1] == "\n":
                    i += 2  # backslash-newline continues comment
                else:
                    i += 1
            if i < n:
                i += 1  # skip newline
            start = i
        elif ch == ";" or ch == "\n":
            cmd = script[start:i]
            if cmd.strip():
                commands.append(cmd)
            i += 1
            start = i
        else:
            i += 1

    # Last command (no trailing separator)
    cmd = script[start:]
    if cmd.strip():
        commands.append(cmd)

    return commands


if TYPE_CHECKING:
    from .interp import TclInterp


def substitute(
    text: str,
    interp: TclInterp,
    *,
    nobackslashes: bool = False,
    nocommands: bool = False,
    novariables: bool = False,
) -> str:
    """Perform full Tcl substitution on *text* (lexer-based).

    Used by the PUSH handler in the VM to resolve ``$var`` / ``[cmd ...]``
    within word values.  Braces are treated as grouping (matching the Tcl
    parser's behaviour).

    For the ``subst`` command (which treats braces as literal characters),
    use :func:`subst_command` instead.
    """
    # Disable {*} expansion during substitution — expansion is a
    # command-level concept, not applicable within a word value.
    old_expand = TclLexer.expand_syntax
    TclLexer.expand_syntax = False
    try:
        lexer = TclLexer(text)
        tokens = lexer.tokenise_all()
    finally:
        TclLexer.expand_syntax = old_expand
    parts: list[str] = []

    for tok in tokens:
        if tok.type is TokenType.EOF:
            break
        match tok.type:
            case TokenType.VAR:
                if novariables:
                    parts.append("$" + tok.text)
                else:
                    var_name = tok.text
                    # Handle dynamic array indices: $arr($var) / $arr([cmd])
                    paren = var_name.find("(")
                    if paren != -1 and var_name.endswith(")"):
                        elem_expr = var_name[paren + 1 : -1]
                        if "$" in elem_expr or "[" in elem_expr:
                            elem_val = substitute(elem_expr, interp)
                            var_name = var_name[:paren] + "(" + elem_val + ")"
                    parts.append(interp.current_frame.get_var(var_name))
            case TokenType.CMD:
                if nocommands:
                    parts.append("[" + tok.text + "]")
                else:
                    result = interp.eval(tok.text)
                    parts.append(result.value)
            case TokenType.ESC:
                if nobackslashes:
                    parts.append(tok.text)
                else:
                    parts.append(backslash_subst(tok.text))
            case TokenType.STR:
                # Braced literal — no substitution
                parts.append(tok.text)
            case TokenType.SEP | TokenType.EOL | TokenType.COMMENT:
                # These shouldn't appear in a word but handle gracefully
                parts.append(tok.text)
            case _:
                parts.append(tok.text)

    return "".join(parts)


def subst_command(
    text: str,
    interp: TclInterp,
    *,
    nobackslashes: bool = False,
    nocommands: bool = False,
    novariables: bool = False,
    handle_exceptions: bool = False,
) -> str:
    """Perform Tcl ``subst`` substitution on *text*.

    Unlike the parser-based :func:`substitute`, the ``subst`` command
    treats braces as **literal** characters — ``{`` and ``}`` do not
    suppress substitution.  Only ``$``, ``[...]``, and ``\\`` are
    special.
    """
    parts: list[str] = []
    i = 0
    n = len(text)

    while i < n:
        c = text[i]

        # Backslash substitution
        if c == "\\" and i + 1 < n:
            if nobackslashes:
                parts.append(c)
                parts.append(text[i + 1])
                i += 2
            else:
                # Use backslash_subst on the escape sequence
                c2 = text[i + 1]
                if c2 == "\n":
                    i += 2
                    while i < n and text[i] in " \t":
                        i += 1
                    parts.append(" ")
                elif c2 in _BACKSLASH_MAP:
                    parts.append(_BACKSLASH_MAP[c2])
                    i += 2
                elif c2 == "x":
                    j = i + 2
                    while j < n and j < i + 4 and text[j] in "0123456789abcdefABCDEF":
                        j += 1
                    if j > i + 2:
                        parts.append(chr(int(text[i + 2 : j], 16)))
                        i = j
                    else:
                        parts.append("x")
                        i += 2
                elif c2 == "u":
                    j = i + 2
                    while j < n and j < i + 6 and text[j] in "0123456789abcdefABCDEF":
                        j += 1
                    if j > i + 2:
                        parts.append(chr(int(text[i + 2 : j], 16)))
                        i = j
                    else:
                        parts.append("u")
                        i += 2
                elif c2 in "01234567":
                    j = i + 1
                    while j < n and j < i + 4 and text[j] in "01234567":
                        j += 1
                    parts.append(chr(int(text[i + 1 : j], 8)))
                    i = j
                else:
                    parts.append(c2)
                    i += 2
            continue

        # Variable substitution
        if c == "$" and not novariables:
            var_name, consumed = _parse_var_ref(text, i + 1)
            if var_name:
                # Handle dynamic array indices within var name
                paren = var_name.find("(")
                if paren != -1 and var_name.endswith(")"):
                    elem_expr = var_name[paren + 1 : -1]
                    if "$" in elem_expr or "[" in elem_expr:
                        elem_val = subst_command(
                            elem_expr,
                            interp,
                            nobackslashes=nobackslashes,
                            nocommands=nocommands,
                        )
                        var_name = var_name[:paren] + "(" + elem_val + ")"
                parts.append(interp.current_frame.get_var(var_name))
                i += 1 + consumed
                continue
            # Bare $ — keep as literal
            parts.append("$")
            i += 1
            continue

        # Command substitution
        if c == "[":
            if nocommands:
                # -nocommands: [ and ] are ordinary characters
                parts.append("[")
                i += 1
            else:
                # Find matching ] — track quotes and braces so that
                # ] inside "..." or {} doesn't terminate prematurely
                depth = 1
                in_quotes = False
                brace_depth = 0
                j = i + 1
                while j < n and depth > 0:
                    ch = text[j]
                    if ch == '"' and brace_depth == 0:
                        in_quotes = not in_quotes
                    elif ch == "[" and not in_quotes and brace_depth == 0:
                        depth += 1
                    elif ch == "]" and not in_quotes and brace_depth == 0:
                        depth -= 1
                    elif ch == "$" and not in_quotes:
                        # ${ starts a variable reference — skip past
                        # matching } so the brace isn't tracked as depth
                        if j + 1 < n and text[j + 1] == "{":
                            j += 2  # skip $ and {
                            while j < n and text[j] != "}":
                                j += 1
                            # j now points at } or past end — will be
                            # incremented at the bottom of the loop
                    elif ch == "{" and not in_quotes:
                        brace_depth += 1
                    elif ch == "}" and not in_quotes:
                        if brace_depth:
                            brace_depth -= 1
                    elif ch == "\\":
                        j += 1  # skip escaped char
                    j += 1
                cmd_text = text[i + 1 : j - 1]
                if handle_exceptions:
                    # Evaluate incrementally — one command at a
                    # time — matching C Tcl semantics.
                    # Side effects from earlier commands execute
                    # even if a later command has a parse error.
                    #
                    # Exception handling:
                    #   break    → stop subst entirely, return partial
                    #   continue → empty result for this [...],
                    #              remaining are PARSED but NOT executed
                    #   return   → use return value, remaining are
                    #              PARSED but NOT executed
                    # Parse errors in remaining commands propagate
                    # even if continue/return already fired.
                    cmds = _split_tcl_commands(cmd_text)
                    cmd_result_value = ""
                    _subst_break = False
                    _short_circuit = False
                    for _single_cmd in cmds:
                        if _short_circuit:
                            # After continue/return: validate parse
                            # but don't execute (no side effects).
                            _stripped = _single_cmd.strip()
                            if _stripped:
                                from core.parsing.lexer import TclParseError

                                TclLexer.strict_quoting = True
                                try:
                                    from vm.compiler import compile_script

                                    compile_script(_stripped)
                                except TclParseError as _pe:
                                    raise TclError(str(_pe)) from None
                                finally:
                                    TclLexer.strict_quoting = False
                            continue
                        try:
                            result = interp.eval(_single_cmd)
                            cmd_result_value = result.value
                        except TclBreak:
                            _subst_break = True
                            break
                        except TclContinue:
                            cmd_result_value = ""
                            _short_circuit = True
                        except TclReturn as ret:
                            cmd_result_value = ret.value
                            _short_circuit = True
                        except TclError:
                            raise
                    if _subst_break:
                        return "".join(parts)
                    parts.append(cmd_result_value)
                else:
                    result = interp.eval(cmd_text)
                    parts.append(result.value)
                i = j
            continue

        # Literal character (including { and })
        parts.append(c)
        i += 1

    return "".join(parts)


def _parse_var_ref(text: str, start: int) -> tuple[str | None, int]:
    """Parse a variable reference starting after the ``$``.

    Returns ``(var_name, chars_consumed)`` or ``(None, 0)`` if no valid
    variable reference is found.

    Handles: ``$name``, ``$name(index)``, ``${name}``.
    """
    n = len(text)
    if start >= n:
        return None, 0

    # ${name} form
    if text[start] == "{":
        end = text.find("}", start + 1)
        if end == -1:
            raise TclError("missing close-brace for variable name")
        return text[start + 1 : end], end - start + 1

    # $name or $name(index) form
    i = start
    while i < n and (text[i].isalnum() or text[i] in "_:"):
        i += 1
    if i == start:
        return None, 0

    name = text[start:i]

    # Check for array index: name(index)
    if i < n and text[i] == "(":
        depth = 1
        j = i + 1
        while j < n and depth > 0:
            ch = text[j]
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
            elif ch == "$" and j + 1 < n and text[j + 1] == "{":
                # ${...} inside array index — check for matching }
                brace_end = text.find("}", j + 2)
                if brace_end == -1:
                    raise TclError("missing close-brace for variable name")
                j = brace_end  # will be incremented below
            elif ch == "\\":
                j += 1
            j += 1
        name = text[start:j]
        i = j

    return name, i - start
