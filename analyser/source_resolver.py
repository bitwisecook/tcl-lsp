"""Resolve ``source`` command targets to file URIs.

Used to build the file dependency graph for cross-file analysis.
"""

from __future__ import annotations

import os

from compiler.parsing.token_scanning import parse_single_command, single_command_substitution
from shared.tokens import TokenType


def _file_join_info_script_rel(raw_path: str) -> str | None:
    """Return the relative tail of a ``file join [file dirname [info script]] …``.

    Recognises the common Tcl idiom ``source [file join [file dirname [info
    script]] X]`` (with or without the outer ``[...]`` wrapper) by tokenising
    the command rather than pattern-matching its text.  The argument must be the
    ``file join`` command whose first joined component is exactly ``[file dirname
    [info script]]``; the tail is the verbatim remainder (every component after
    that, with original spacing preserved).  Returns ``None`` when *raw_path* is
    not that idiom.
    """
    # Peel the optional outer command substitution: ``[file join …]``.
    inner_tok = single_command_substitution(raw_path)
    inner = inner_tok.text if inner_tok is not None else raw_path

    parsed = parse_single_command(inner, piece=lambda t: t.text)
    if parsed is None:
        return None
    texts, tokens, _single = parsed
    # file join [file dirname [info script]] <tail…>  (>= 4 words)
    if len(texts) < 4 or texts[0] != "file" or texts[1] != "join":
        return None
    if tokens[2].type is not TokenType.CMD:
        return None
    # The first joined component must itself be ``file dirname [info script]``.
    dirname = parse_single_command(tokens[2].text, piece=lambda t: t.text)
    if dirname is None:
        return None
    d_texts, d_tokens, _ = dirname
    if len(d_texts) != 3 or d_texts[0] != "file" or d_texts[1] != "dirname":
        return None
    if d_tokens[2].type is not TokenType.CMD or d_tokens[2].text.strip() != "info script":
        return None
    # Verbatim tail: from the start of the first word after the dirname
    # component through the command end, preserving original inter-word spacing
    # (mirrors the old regex capture group).  Anchoring on the next word's start
    # avoids the ``[...]`` token-span quirk where a ``CMD`` token's end excludes
    # its outer closing bracket.
    tail = inner[tokens[3].start.offset :].strip()
    return tail or None


def resolve_source_target(
    raw_path: str,
    is_literal: bool,
    script_path: str,
    workspace_roots: list[str] | None = None,
) -> str | None:
    """Resolve a ``source`` target to an absolute file path.

    Returns the resolved path, or ``None`` if unresolvable.

    Parameters
    ----------
    raw_path:
        The literal text of the source command's file argument.
    is_literal:
        ``False`` when the path contains ``$`` or ``[`` substitutions.
    script_path:
        Absolute path of the file containing the ``source`` command.
    workspace_roots:
        Workspace root directories to try for relative paths.
    """
    if not is_literal:
        # Try to extract a relative path from the [file join ...] idiom.
        rel = _file_join_info_script_rel(raw_path)
        if rel is not None:
            # The relative portion itself may still contain substitutions.
            if "$" in rel or "[" in rel:
                return None
            script_dir = os.path.dirname(script_path)
            candidate = os.path.normpath(os.path.join(script_dir, rel))
            if os.path.isfile(candidate):
                return candidate
        return None

    # Literal path — try relative to the script's own directory first.
    script_dir = os.path.dirname(script_path)
    candidate = os.path.normpath(os.path.join(script_dir, raw_path))
    if os.path.isfile(candidate):
        return candidate

    # Try relative to each workspace root.
    for root in workspace_roots or []:
        candidate = os.path.normpath(os.path.join(root, raw_path))
        if os.path.isfile(candidate):
            return candidate

    # Absolute path?
    if os.path.isabs(raw_path) and os.path.isfile(raw_path):
        return raw_path

    return None
