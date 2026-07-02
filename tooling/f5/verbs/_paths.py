"""Shared file-input helpers for f5 verbs.

The cleanup/grep verbs duplicated this trivially; centralising it
keeps every new verb consistent.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# The iRule/config file loader now lives with the F5 model in
# `dialects.f5.bigip.inputs`; re-exported here for the f5 CLI verbs and any
# consumer that still imports it from this module.
from dialects.f5.bigip.inputs import (  # noqa: E402,F401  (re-export)
    _BIGIP_SUFFIXES,
    _IRULE_SUFFIXES,
    _UCS_SUFFIXES,
    IruleInput,
    _looks_like_ucs,
    load_irule_inputs,
)
from dialects.f5.bigip.model import BigipConfig
from dialects.f5.bigip.parser import parse_bigip_conf
from tooling.f5.f5_remote.ucs import (
    DEFAULT_PASSPHRASE_ENV,
    PassphraseProvider,
    make_passphrase_provider,
    ucs_archive_to_scf,
)


def add_passphrase_args(parser: argparse.ArgumentParser) -> None:
    """Add the shared ``--passphrase-env`` / ``--no-passphrase-prompt`` flags.

    Verbs that may be pointed at an encrypted ``.ucs`` call this so the
    passphrase source is discoverable in ``--help``.  Even without these
    flags every verb honours ``$F5_UCS_PASSPHRASE`` and prompts on a TTY,
    because :func:`read_path` & friends fall back to a default provider.
    """
    group = parser.add_argument_group("encrypted UCS")
    group.add_argument(
        "--passphrase-env",
        metavar="VAR",
        default=DEFAULT_PASSPHRASE_ENV,
        help=(
            "environment variable holding the passphrase for an encrypted "
            "UCS archive (default: %(default)s)."
        ),
    )
    group.add_argument(
        "--no-passphrase-prompt",
        action="store_true",
        help=(
            "never prompt on the terminal for an encrypted-UCS passphrase; "
            "require the environment variable instead."
        ),
    )


def provider_from_args(args: argparse.Namespace) -> PassphraseProvider:
    """Build a passphrase provider from :func:`add_passphrase_args` options."""
    return make_passphrase_provider(
        env_var=getattr(args, "passphrase_env", None) or DEFAULT_PASSPHRASE_ENV,
        allow_prompt=not getattr(args, "no_passphrase_prompt", False),
    )


def read_path(
    path_str: str,
    *,
    strict: bool = False,
    passphrase_provider: PassphraseProvider | None = None,
) -> tuple[str, str]:
    """Return ``(uri, source)`` for *path_str*.  ``-`` reads stdin.

    When *strict* is true, undecodable bytes raise
    :class:`UnicodeDecodeError` instead of silently being replaced
    with U+FFFD.  Mutating commands (``f5 query --in-place`` /
    ``f5 rename --in-place``) should pass ``strict=True`` so they
    don't permanently lose data on the round-trip — once the source
    has lost a byte to a replacement char, writing the rewritten
    text back overwrites the original byte for good.

    ``.ucs`` archives — plain *or* encrypted (OpenPGP), and gzipped /
    OpenPGP streams from stdin — are transparently extracted to SCF via
    :func:`ucs_archive_to_scf`, so every verb that reads via
    ``read_path`` (``query``, ``rename``, ``merge``, ``convert``,
    ``unredact``, …) accepts a UCS the same way it accepts
    ``.scf``/``.conf``.  Encrypted archives resolve their passphrase via
    *passphrase_provider* (default: ``$F5_UCS_PASSPHRASE`` / TTY prompt).
    """
    errors = "strict" if strict else "replace"
    if path_str == "-":
        raw = sys.stdin.buffer.read()
        if _looks_like_ucs(raw, suffix="", is_stdin=True):
            return (
                "stdin://input",
                ucs_archive_to_scf(raw, passphrase_provider=passphrase_provider, label="stdin"),
            )
        if strict:
            return ("stdin://input", raw.decode("utf-8"))
        return ("stdin://input", raw.decode("utf-8", errors="replace"))
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    raw = path.read_bytes()
    suffix = path.suffix.lower()
    if _looks_like_ucs(raw, suffix=suffix, is_stdin=False):
        return (
            path.as_uri(),
            ucs_archive_to_scf(raw, passphrase_provider=passphrase_provider, label=path_str),
        )
    if suffix in _UCS_SUFFIXES:
        raise ValueError(f"{path_str}: not a valid UCS archive")
    return (path.as_uri(), raw.decode("utf-8", errors=errors))


def load_paths(
    paths: list[str],
    *,
    passphrase_provider: PassphraseProvider | None = None,
) -> tuple[dict[str, str], dict[str, BigipConfig]]:
    sources: dict[str, str] = {}
    configs: dict[str, BigipConfig] = {}
    for p in paths:
        uri, src = read_path(p, passphrase_provider=passphrase_provider)
        sources[uri] = src
        configs[uri] = parse_bigip_conf(src)
    return sources, configs


# ---------------------------------------------------------------------------
# Unified iRule input loading
# ---------------------------------------------------------------------------
