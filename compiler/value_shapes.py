"""Shared Tcl value-shape helpers used across compiler passes."""

from __future__ import annotations

# A single ``$name`` / ``$ns::name`` / ``$arr(idx)`` reference and nothing else.


def _scan_pure_var_ref(text: str, i: int) -> int:
    """Scan one Tcl variable reference starting at *text[i]*.  Returns
    the index past the end of the reference, or -1 if the input doesn't
    start with a valid reference.

    Handles the three forms documented in the Tcl 9.0 ``Tcl_ParseVar``
    spec:

      * ``$name`` -- bare name, characters in ``[a-zA-Z0-9_:]``.  ``::``
        runs are part of the name (namespace-qualified).
      * ``${name}`` -- braced; any character except ``}`` (escapes
        inside the braces are taken literally).
      * ``$name(index)`` -- array element.  The index is everything up
        to the matching ``)``, with backslash-escaped close parens
        treated literally (so ``$a(x\\)y)`` is one reference whose
        index text is ``x\\)y``).  Variable and command substitution
        DO happen inside the index at runtime, but they don't terminate
        the index here -- we only need the textual extent.

    Returns -1 if no valid reference starts at *i*.  (D4-F11 closure --
    replaces the old ``re.compile(r"\\$[\\w:]+(\\([^)]*\\))?")`` which
    incorrectly terminated the index at the first ``)`` even when
    escaped.  tclsh accepts ``set a(x\\)y) 1`` as a valid array set.)
    """
    if i >= len(text) or text[i] != "$":
        return -1
    i += 1
    if i >= len(text):
        return -1
    # Braced form: ``${...}``.
    if text[i] == "{":
        j = i + 1
        while j < len(text) and text[j] != "}":
            j += 1
        if j >= len(text):
            return -1
        return j + 1
    # Bare name: alnum + ``_`` + ``:``.
    start = i
    while i < len(text) and (text[i].isalnum() or text[i] in "_:"):
        i += 1
    if i == start:
        return -1
    # Optional array index ``(index)`` -- backslash-escape aware.
    if i < len(text) and text[i] == "(":
        j = i + 1
        while j < len(text):
            if text[j] == "\\" and j + 1 < len(text):
                j += 2  # skip escaped char
                continue
            if text[j] == ")":
                return j + 1
            j += 1
        return -1  # unterminated index
    return i


def is_pure_var_ref(text: str) -> bool:
    """Return True if *text* is exactly one variable reference (``$x`` /
    ``${x}`` / ``$ns::x`` / ``$arr(idx)``) with no surrounding or concatenated
    syntax.

    Examples of true single-var refs (accepted):
        ``$x``, ``${x}``, ``$ns::x``, ``$arr(key)``, ``$arr(x\\)y)``

    Examples of concatenations / non-refs (rejected):
        ``$x$y``, ``$x_$y``, ``$x.foo``, ``$x bar``, ``foo$x``

    The check is critical for the uplevel/safe-eval idioms: ``uplevel
    1 $body`` is single-substitution-safe only when ``$body`` is one
    pure ref; anything else double-substitutes the composed value.
    """
    end = _scan_pure_var_ref(text, 0)
    return end == len(text)


def is_braced_whole_name_array_ref(text: str) -> bool:
    """Return True if *text* is a braced array-shaped var ref ``${a(1)}``.

    tclsh 9 loads such a reference by its WHOLE literal name (``a(1)``
    resolved at runtime as array element ``a(1)``), so the value is a
    *variable reference*, never a constant — analyses must treat it as
    conservatively unknown (overdefined), the same as ``${ns::y}``.

    Matches a braced ``${inner}`` whose *inner* is array-shaped
    (contains ``(``, ends with ``)``, no ``}`` inside).  A plain braced
    scalar ``${foo}`` (no parens) is NOT matched here — it flows through
    the normal var-resolution path.
    """
    if not text.startswith("${") or not text.endswith("}"):
        return False
    inner = text[2:-1]
    return "(" in inner and inner.endswith(")") and "}" not in inner


def parse_command_substitution(text: str) -> tuple[str, tuple[str, ...]] | None:
    """Extract command name and args from ``[cmd ...]`` using the Tcl
    lexer.

    Properly handles braced words (``[foo {a b} c]``), quoted words
    (``[foo "a b" c]``), nested substitutions (``[foo [bar] baz]``),
    namespaced command names (``[::ns::foo a]``), and any other Tcl
    word-splitting rule -- delegates to the shared
    ``compiler.parsing.command_segmenter.segment_commands`` rather
    than re-implementing word splitting.

    Returns ``None`` when *text* isn't surrounded by ``[`` / ``]``,
    when the segmenter can't parse the contents into at least one
    command, or when the cmd-sub contains multiple semicolon-
    separated commands (we don't know which one's return value is
    bound, so caller treats it as opaque).
    """
    from compiler.parsing.command_segmenter import segment_commands

    stripped = text.strip()
    if not (stripped.startswith("[") and stripped.endswith("]")):
        return None
    inner = stripped[1:-1]
    try:
        cmds = segment_commands(inner)
    except Exception:
        return None
    # A cmd-sub that contains multiple commands (``[cmd1; cmd2]``)
    # returns the LAST command's value; we don't model that here
    # because callers want to identify a single called proc.  Stay
    # conservative and bail.
    if len(cmds) != 1:
        return None
    cmd = cmds[0]
    if not cmd.texts:
        return None
    return cmd.texts[0], tuple(cmd.texts[1:])
