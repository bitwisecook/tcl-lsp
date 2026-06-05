"""PROTOTYPE: does the registry's ArgRole classify unterminated `{` for recovery?

The current E203 heuristic treats every unterminated ``{`` the same — it looks
for a de-indented line starting with a known command.  But the registry already
knows what each argument *is*: ``_resolve_arg_roles(cmd, args)`` tags the brace
argument as ``BODY`` (a script), ``EXPR`` (a braced expression), or neither
(DATA).  Each wants different recovery:

  BODY  — a script; closing before a de-indented command is sensible.
  EXPR  — a braced expression; recovery should use the expr sublanguage.
  DATA  — an opaque value; a command-looking word inside is *data*, so closing
          on it is a likely false positive — be conservative.

This measures how the registry classification cross-tabulates with what the
current heuristic actually does, to see where role info would change the call:
``BODY+declined`` = a recovery we miss; ``DATA+recovered`` = a likely false
close; ``EXPR`` = cases that need expr-aware recovery the current path lacks.

    uv run python tests/proto_role_recovery.py
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.recovery import _is_suspicious_str, compute_virtual_insertions
from compiler.registry.runtime import ArgRole, _resolve_arg_roles

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_recovering_lexer_differential import _rand_recovery_source  # noqa: E402


def brace_role(source: str) -> str | None:
    """Classify the unterminated ``{`` argument as BODY / EXPR / DATA, or None.

    None means there is no suspicious unterminated brace to classify.
    """
    cmds = segment_commands(source)
    for cmd in cmds:
        for tok in cmd.all_tokens:
            if tok.type.name == "STR" and _is_suspicious_str(tok, source, 0):
                name = cmd.name
                args = list(cmd.args)
                if not name or not args:
                    return "DATA"
                roles, base = _resolve_arg_roles(name, args)
                # The unterminated brace is the last argument; map its full
                # index through the subcommand base offset.
                rel = (len(args) - 1) - base
                rs = roles.get(rel, frozenset()) or roles.get(len(args) - 1, frozenset())
                if ArgRole.BODY in rs:
                    return "BODY"
                if ArgRole.EXPR in rs:
                    return "EXPR"
                return "DATA"
    return None


def current_recovers(source: str) -> bool:
    """True if today's heuristic inserts a ``}`` (recovers the brace)."""
    return "}" in compute_virtual_insertions(source).values()


# De-indented layouts: indented content, then a de-indented known command —
# the exact shape the current heuristic recovers on.  The registry tells BODY
# (recovery is right) from DATA (the de-indented "command" is really data — a
# false close).
_BODY_TEMPLATES = [
    "proc p {{x y}} {{\n    {inner}\n{cmd}\n",
    "while {{$i < 3}} {{\n    {inner}\n{cmd}\n",
    "foreach x $l {{\n    {inner}\n{cmd}\n",
    "if {{$x}} {{\n    {inner}\n{cmd}\n",
]
_DATA_TEMPLATES = [
    "set x {{\n    {inner}\n{cmd}\n",
    "lappend xs {{\n    {inner}\n{cmd}\n",
    "dict set d k {{\n    {inner}\n{cmd}\n",
]
_EXPR_TEMPLATES = [
    "if {{$x > \nputs hi\n",
    "while {{$i + \nputs hi\n",
    "expr {{1 + \nputs hi\n",
]
_INNERS = ["puts hi", "some data here", "a b c"]
_CMDS = ["return", "set y 2", "puts done"]


def main() -> int:
    cases: list[str] = []
    for t in _BODY_TEMPLATES + _DATA_TEMPLATES:
        for inner in _INNERS:
            for cmd in _CMDS:
                cases.append(t.format(inner=inner, cmd=cmd))
    cases += _EXPR_TEMPLATES
    rng = random.Random(20240605)
    for _ in range(20000):
        cases.append(_rand_recovery_source(rng))

    from collections import Counter

    tab: Counter = Counter()
    examples: dict[tuple, str] = {}
    for src in cases:
        role = brace_role(src)
        if role is None:
            continue
        rec = current_recovers(src)
        key = (role, "recovered" if rec else "declined")
        tab[key] += 1
        examples.setdefault(key, src)

    print("registry-role  x  current-heuristic")
    for role in ("BODY", "EXPR", "DATA"):
        for outcome in ("recovered", "declined"):
            key = (role, outcome)
            print(f"  {role:5} + {outcome:9} : {tab.get(key, 0):5}")
    print("\nillustrative cases:")
    for role in ("BODY", "EXPR", "DATA"):
        for outcome in ("recovered", "declined"):
            ex = examples.get((role, outcome))
            if ex:
                print(f"  [{role}+{outcome}] {ex!r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
