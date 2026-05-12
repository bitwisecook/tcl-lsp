"""Normalise tmsh command output to SCF text.

``f5 tmsh`` (and a real BIG-IP shell) emits stanzas of the form::

    tmsh create ltm pool /Common/p1 { members { … } }
    tmsh modify ltm virtual /Common/vs_app { pool /Common/p2 }

Stripping the ``tmsh create`` / ``tmsh modify`` verb pair yields valid
SCF — ``ltm pool /Common/p1 { … }`` — which :func:`parse_bigip_conf`
already understands.  Prefixes that appear *inside* a brace body (for
example, embedded as a string inside an iRule) are left alone.
"""

from __future__ import annotations

_TMSH_PREFIXES: tuple[str, ...] = (
    "tmsh create ",
    "tmsh modify ",
)


def looks_like_tmsh_output(text: str) -> bool:
    """Return True when *text* opens with a tmsh create/modify command.

    Only the first non-blank, non-comment line is inspected — if it
    starts with one of the recognised prefixes we treat the whole
    document as tmsh output.  SCF text never starts with ``tmsh ``.
    """
    for line in text.splitlines():
        stripped = line.lstrip()
        if not stripped or stripped.startswith("#"):
            continue
        return any(stripped.startswith(prefix) for prefix in _TMSH_PREFIXES)
    return False


def normalise_tmsh_to_scf(text: str) -> str:
    """Strip ``tmsh create`` / ``tmsh modify`` prefixes from stanza headers.

    Only prefixes at brace depth zero are stripped, so an embedded
    ``tmsh create …`` literal inside an iRule body survives intact.
    """
    out: list[str] = []
    depth = 0
    for line in text.splitlines(keepends=True):
        if depth == 0:
            indent_len = len(line) - len(line.lstrip(" \t"))
            indent, rest = line[:indent_len], line[indent_len:]
            for prefix in _TMSH_PREFIXES:
                if rest.startswith(prefix):
                    line = indent + rest[len(prefix) :]
                    break
        out.append(line)
        depth += _net_brace_delta(line)
    return "".join(out)


def _net_brace_delta(line: str) -> int:
    """Return ``{`` count minus ``}`` count outside quoted strings and comments.

    A ``#`` outside a quoted string starts a comment that runs to end of
    line — Tcl-style, mirroring how an iRule body would lex.  Without
    this the counter would treat ``# }`` inside a rule body as a real
    closing brace and drop the tracked nesting depth prematurely, which
    then causes top-level ``tmsh create`` / ``tmsh modify`` lines that
    follow the rule to slip through unstripped.
    """
    depth = 0
    in_string = False
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c == "\\" and i + 1 < n:
            i += 2
            continue
        if c == '"':
            in_string = not in_string
        elif not in_string:
            if c == "#":
                break
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
        i += 1
    return depth
