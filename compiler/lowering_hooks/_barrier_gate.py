"""Shared token-level gate for barrier relaxation.

When the ``eval``/``uplevel`` hooks consider relaxing a braced-literal
body to inline IR, they first walk the body's command words to reject
nested dynamic barriers.  A static braced ``eval {uplevel 1 $x}``
still contains a ``$x`` body that cannot be lowered statically —
relaxing the outer barrier would produce IR that executes a compiled
``uplevel`` with a dynamic body (which is still wrong), so we bail.

The walk is deliberately shallow and token-based: we segment the body
into commands, and for each command whose name matches
``_DYNAMIC_BARRIER_COMMANDS`` we check whether its own body-shaped
argument is a braced literal.  If any nested barrier has a
still-dynamic shape, the whole outer relaxation is poisoned.

This is NOT a full static-analysis pass — we do not chase ``set body
{…}; eval $body`` across variable definitions.  The gate is:

1. Is this command name in the barrier set?  If no, skip.
2. If yes, does its *own* body-shaped argument (last arg for ``eval``
   and ``uplevel``) appear to be a braced literal free of
   substitutions?  If no, poison.
3. Recurse into the nested body.
"""

from __future__ import annotations

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.lexer import TclParseError
from compiler.registry import REGISTRY
from shared.tokens import Token, TokenType

_DYNAMIC_BARRIER_COMMANDS = REGISTRY.dynamic_barrier_commands()


def body_has_dynamic_barrier(body_text: str, body_tok: Token | None) -> bool:
    """Return True if *body_text* contains a nested dynamic-shape barrier.

    A nested barrier is "dynamic-shape" when its own body argument is
    anything other than a pure braced literal — i.e. a ``$var``
    reference, a ``[cmd]`` substitution, or a bare word that would
    undergo substitution before eval.  Such cases defeat the
    conservative gate and keep the caller on the ``IRBarrier``
    fallback.

    Returns True on parse errors too: if we cannot confidently walk
    the body, assume the worst and keep the fallback.
    """
    try:
        commands = segment_commands(body_text, body_tok)
    except TclParseError:
        return True

    for sc in commands:
        argv = sc.argv
        texts = sc.texts
        if not argv or not texts:
            continue
        name = texts[0]
        if name not in _DYNAMIC_BARRIER_COMMANDS:
            # Recurse into structured bodies (if/while/for/foreach/proc
            # bodies etc.) so a nested ``if { … } { eval $x }`` still
            # trips the gate.  We do not attempt to enumerate every
            # command that takes a body — segment_commands handles the
            # top level, and the lowering pass will see the inner
            # barriers when the body is lowered anyway.  For the gate,
            # conservatively check any arg that is itself a braced
            # block for nested barriers.
            for i, tok in enumerate(argv):
                if tok.type is not TokenType.STR:
                    continue
                if i == 0:
                    continue
                arg_text = texts[i] if i < len(texts) else ""
                if body_has_dynamic_barrier(arg_text, tok):
                    return True
            continue

        # Name is in the barrier set — inspect its own body.  For
        # ``eval`` the body is args[0..]; for ``uplevel`` the body is
        # args[1..] when args[0] is a level, else args[0..].  Static
        # shape requires the *last* argument to be a pure STR token.
        # (Tcl eval/uplevel concat multiple body words; we require the
        # trailing word to be braced, which is the common case and
        # keeps the gate simple.)
        args = texts[1:]
        arg_tokens = argv[1:]
        if not args:
            # ``eval`` / ``uplevel`` with no args — malformed, poison
            # so the outer hook falls back to IRBarrier where the
            # runtime can report the error.
            return True
        if name == "uplevel":
            # Skip the level arg if present.
            level = args[0]
            if level.lstrip("-").isdigit() or level.startswith("#"):
                if len(args) < 2:
                    return True
                body_idx = len(args) - 1
                level_tok = arg_tokens[0]
                if level_tok.type is not TokenType.ESC:
                    # Braced or substituted level — dynamic.
                    return True
            else:
                body_idx = len(args) - 1
        else:
            body_idx = len(args) - 1
        body_tok_nested = arg_tokens[body_idx]
        if body_tok_nested.type is not TokenType.STR:
            # Nested body is ``$var`` / ``[cmd]`` / ESC word —
            # dynamic.
            return True
        # Nested body itself is a braced literal — recurse so a
        # braced ``eval {eval $x}`` still trips.
        nested_text = args[body_idx]
        if body_has_dynamic_barrier(nested_text, body_tok_nested):
            return True

    return False
