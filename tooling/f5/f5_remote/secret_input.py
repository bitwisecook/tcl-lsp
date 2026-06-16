"""Shared secret-input resolution for the ``f5`` CLI.

A single, idiomatic way to obtain a secret (passphrase, master key,
password, …) from the four sources the CLI supports, tried in order:

1. an explicit value (e.g. a ``--password`` flag),
2. a file (e.g. ``--f5mku-file key.txt``),
3. an environment variable, and
4. a secure, no-echo terminal prompt via :func:`getpass.getpass`.

Centralising this keeps the tty detection correct in one place — the
prompt is offered whenever *either* stdin or stderr is a terminal, and
``getpass`` itself reads from the controlling ``/dev/tty``, so a prompt
still works when the config is piped in on stdin (``f5 decrypt-secrets
-``).  :func:`resolve_secret` returns ``None`` when no source yields a
value, leaving the caller to decide whether that is an error; a
cancelled prompt (Ctrl-C / EOF) raises :class:`SecretInputError`.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

__all__ = ["resolve_secret", "secret_source_available", "SecretInputError"]


class SecretInputError(ValueError):
    """Raised when interactive secret entry is cancelled."""


def _has_tty() -> bool:
    """Whether a controlling terminal is available to prompt on."""
    for stream in (sys.stdin, sys.stderr):
        try:
            if stream is not None and stream.isatty():
                return True
        except (ValueError, AttributeError):  # pragma: no cover - closed stream
            pass
    return False


def secret_source_available(
    *,
    explicit: str | None = None,
    env_var: str | None = None,
    file: str | os.PathLike[str] | None = None,
    allow_prompt: bool = True,
) -> bool:
    """Whether :func:`resolve_secret` could yield a value without prompting,
    or a prompt is possible.  Useful for a fast pre-flight check."""
    if explicit:
        return True
    if file:
        return True
    if env_var and os.environ.get(env_var):
        return True
    return allow_prompt and _has_tty()


def resolve_secret(
    *,
    explicit: str | None = None,
    env_var: str | None = None,
    file: str | os.PathLike[str] | None = None,
    allow_prompt: bool = True,
    prompt: str = "Secret: ",
    strip: bool = False,
) -> str | None:
    """Resolve a secret from an explicit value, a file, an env var, or a prompt.

    Sources are tried in that fixed order and the first non-empty one
    wins.  Returns ``None`` when nothing yields a value (the caller
    decides whether that is fatal).

    *strip* controls whitespace trimming of the resolved value.  Leave it
    ``False`` for passphrases and passwords, where surrounding whitespace
    can be significant; set it ``True`` for tokens that are never meant to
    carry whitespace (a base64 key read from ``f5mku -K > key.txt`` picks
    up a trailing newline, for instance).

    A cancelled prompt (Ctrl-C / EOF) raises :class:`SecretInputError`.
    """

    def _norm(value: str | None) -> str | None:
        if value is None:
            return None
        return value.strip() if strip else value

    if explicit:
        return _norm(explicit)

    if file:
        text = _norm(Path(file).read_text(encoding="utf-8"))
        if text:
            return text

    if env_var:
        env_val = os.environ.get(env_var)
        if env_val:
            return _norm(env_val)

    if allow_prompt and _has_tty():
        import getpass

        try:
            entered = _norm(getpass.getpass(prompt))
        except (EOFError, KeyboardInterrupt) as exc:  # pragma: no cover - tty only
            raise SecretInputError("secret entry cancelled") from exc
        if entered:
            return entered

    return None
