"""Diagnostic-repro minimizer.

Take an emitted diagnostic and produce the **minimum self-contained code** that
still fires the *same* diagnostic code, with user identifiers renamed to short
``a b c …`` names — so a user can paste a clean minimal repro into a bug report.

The reduction is classic **delta-debugging (ddmin)** over source lines: split
the source into chunks, try removing each, and keep a removal only if the target
diagnostic still fires.  Brace-unbalancing removals naturally make the
diagnostic disappear, so ddmin rejects them.  After structural reduction, user
variable names are renamed to ``a b c …`` (verify-gated: a rename that stops the
diagnostic firing is reverted).

Invariant: every reduction/rename step is kept only if the target diagnostic is
still present — the result always reproduces.  Lives in ``server`` (it depends
on ``get_diagnostics``) so both the ``tcl minimize`` CLI verb and the LSP code
action can use it.
"""

from __future__ import annotations

from dataclasses import dataclass

from shared.tokens import TokenType

# Variable-target commands: arguments at these positions name a variable (not a
# value), so renaming must rewrite them alongside ``$`` references.
_VAR_TARGET_CMDS: dict[str, tuple[int, ...]] = {
    "set": (0,),
    "variable": (0,),
    "global": (0, 1, 2, 3, 4, 5, 6, 7),  # all args are names
    "incr": (0,),
    "append": (0,),
    "lappend": (0,),
    "unset": (0, 1, 2, 3, 4, 5, 6, 7),
    "lassign": (1, 2, 3, 4, 5, 6, 7),  # arg0 is the list, rest are targets
}

# Never rename these — Tcl specials / globals whose names are semantically load-
# bearing (renaming would change behaviour or break the repro).
_RESERVED_NAMES = frozenset(
    {
        "argv",
        "argv0",
        "argc",
        "env",
        "errorInfo",
        "errorCode",
        "tcl_platform",
        "auto_path",
        "this",
        "self",
        "type",
        "win",
        "selfns",
    }
)


@dataclass(frozen=True, slots=True)
class MinimizeResult:
    """Outcome of minimising a diagnostic to a minimal reproducer."""

    source: str
    code: str
    original_lines: int
    reduced_lines: int
    renamed: bool
    reproduces: bool


def _has_code(diags: list, code: str) -> bool:
    return any(d.code == code for d in diags)


def _ddmin(units: list[str], test) -> list[str]:
    """Zeller delta-debugging: minimise *units* keeping ``test`` true.

    Returns the smallest sublist of *units* for which ``test(sublist)`` holds,
    found by removing progressively finer complements.
    """
    n = 2
    while len(units) >= 2:
        chunk = max(1, len(units) // n)
        removed_any = False
        # Try removing each complement (the whole minus one chunk).
        i = 0
        while i < len(units):
            complement = units[:i] + units[i + chunk :]
            if complement and test(complement):
                units = complement
                n = max(n - 1, 2)
                removed_any = True
                break
            i += chunk
        if not removed_any:
            if n >= len(units):
                break
            n = min(n * 2, len(units))
    return units


def _collect_rename_edits(source: str, names_seen: dict[str, str]) -> list[tuple[int, int, str]]:
    """Return ``(start, end, new_text)`` edits renaming user variables.

    Covers ``$``/``${}`` references (VAR tokens, base name only — array index
    preserved) and variable-target command arguments (``set x``, ``global a b``,
    …).  *names_seen* accumulates name→short-name across the whole source so the
    mapping is stable and consistent.
    """
    from compiler.parsing.green_tree import tokenise

    edits: list[tuple[int, int, str]] = []

    def _short_for(name: str) -> str | None:
        if name in _RESERVED_NAMES or "::" in name or not name or name[0].isdigit():
            return None
        existing = names_seen.get(name)
        if existing is not None:
            return existing
        # a, b, …, z, a1, b1, …
        idx = len(names_seen)
        letter = chr(ord("a") + idx % 26)
        suffix = idx // 26
        short = letter if suffix == 0 else f"{letter}{suffix}"
        names_seen[name] = short
        return short

    # 1. ``$``/``${}`` references — VAR tokens (rename the scalar/array base).
    lex_tokens, _ = tokenise(source, 0, 0, 0)
    for tok in lex_tokens:
        if tok.type is TokenType.EOL and tok.text == "":
            continue
        if tok.type is TokenType.VAR:
            base = tok.text.split("(", 1)[0]
            short = _short_for(base)
            if short is not None:
                # The VAR token offset points at the ``$`` sigil; the name
                # begins after ``$`` (``$x``) or ``${`` (``${x}``).
                off = tok.start.offset
                if off < len(source) and source[off] == "$":
                    name_start = off + 2 if source[off + 1 : off + 2] == "{" else off + 1
                else:
                    name_start = off
                edits.append((name_start, name_start + len(base), short))

    # 2. Variable-target command arguments (def sites: ``set x``, ``global a``).
    from compiler.parsing.recovery import segment_with_recovery

    try:
        cmds, _ = segment_with_recovery(source, None)
    except Exception:
        cmds = []  # segmentation can fail on malformed input; $-refs still help

    for cmd in cmds:
        if not cmd.texts:
            continue
        positions = _VAR_TARGET_CMDS.get(cmd.texts[0])
        if not positions:
            continue
        arg_tokens = cmd.argv[1:] if len(cmd.argv) > 1 else []
        for pos in positions:
            if pos >= len(arg_tokens):
                continue
            atok = arg_tokens[pos]
            # Only plain bare-literal names (not $-substituted / quoted / braced).
            if atok.type is not TokenType.ESC:
                continue
            base = atok.text.split("(", 1)[0]
            short = _short_for(base)
            if short is not None and base == atok.text:
                start = atok.start.offset
                edits.append((start, start + len(base), short))

    return edits


def _apply_edits(source: str, edits: list[tuple[int, int, str]]) -> str:
    """Apply non-overlapping ``(start, end, text)`` edits right-to-left."""
    seen: set[tuple[int, int]] = set()
    uniq: list[tuple[int, int, str]] = []
    for s, e, t in edits:
        if (s, e) in seen:
            continue
        seen.add((s, e))
        uniq.append((s, e, t))
    for s, e, t in sorted(uniq, key=lambda x: x[0], reverse=True):
        source = source[:s] + t + source[e:]
    return source


def _dedent(source: str) -> str:
    """Strip common leading whitespace from non-blank lines."""
    lines = source.split("\n")
    indents = [len(ln) - len(ln.lstrip()) for ln in lines if ln.strip()]
    if not indents:
        return source
    common = min(indents)
    if common == 0:
        return source
    return "\n".join(ln[common:] if ln.strip() else ln for ln in lines)


def minimize_diagnostic(
    source: str,
    code: str,
    *,
    rename: bool = True,
    get_diagnostics_fn=None,
) -> MinimizeResult:
    """Reduce *source* to the minimum code still firing diagnostic *code*.

    *get_diagnostics_fn* defaults to the real analyser entry point; tests may
    inject a stub.  Raises ``ValueError`` if *code* is not present in *source*.
    """
    if get_diagnostics_fn is None:
        from server.features.diagnostics import get_diagnostics as get_diagnostics_fn

    def fires(src: str) -> bool:
        try:
            return _has_code(get_diagnostics_fn(src), code)
        except Exception:
            return False

    original_lines = source.count("\n") + 1
    if not fires(source):
        raise ValueError(f"diagnostic {code!r} does not fire on the given source")

    # 1. Structural reduction over lines.
    reduced = "\n".join(_ddmin(source.split("\n"), lambda u: fires("\n".join(u))))
    # Dedent must not change the result.  If it does (a rare brace/heredoc-
    # sensitive case), revert to the already-minimised pre-dedent text — it is
    # known to fire — rather than re-running the whole ddmin from the *original*
    # source, which both doubles the work and discards the reduction.
    dedented = _dedent(reduced)
    if fires(dedented):
        reduced = dedented

    # 2. Verify-gated identifier rename.
    did_rename = False
    if rename:
        edits = _collect_rename_edits(reduced, {})
        if edits:
            candidate = _apply_edits(reduced, edits)
            if fires(candidate):
                reduced = candidate
                did_rename = True

    return MinimizeResult(
        source=reduced,
        code=code,
        original_lines=original_lines,
        reduced_lines=reduced.count("\n") + 1,
        renamed=did_rename,
        reproduces=fires(reduced),
    )


__all__ = ["MinimizeResult", "minimize_diagnostic"]
