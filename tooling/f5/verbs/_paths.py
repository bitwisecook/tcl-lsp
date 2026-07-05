# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Shared file-input helpers for f5 verbs.

The cleanup/grep verbs duplicated this trivially; centralising it
keeps every new verb consistent.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

from dialects.f5.bigip.model import BigipConfig, BigipRule
from dialects.f5.bigip.parser import parse_bigip_conf
from tooling.f5.f5_remote.ucs import (
    DEFAULT_PASSPHRASE_ENV,
    PassphraseProvider,
    is_pgp_bytes,
    make_passphrase_provider,
    ucs_archive_to_scf,
)

_IRULE_SUFFIXES = frozenset({".tcl", ".irul", ".irule"})
_BIGIP_SUFFIXES = frozenset({".conf", ".scf"})
_UCS_SUFFIXES = frozenset({".ucs"})


def _is_gzip(data: bytes) -> bool:
    return len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B


def _looks_like_ucs(raw: bytes, *, suffix: str, is_stdin: bool) -> bool:
    """Whether *raw* should be extracted/decrypted as a UCS archive.

    OpenPGP magic is unambiguous (a bigip.conf is never an OpenPGP
    message), so an encrypted UCS is recognised regardless of file
    extension.  Plain gzip magic is a weaker signal — lots of unrelated
    ``.gz`` files share it — so it is only treated as a UCS for ``.ucs``
    paths or stdin (where there is no suffix to go on).
    """
    if is_pgp_bytes(raw):
        return True
    if _is_gzip(raw):
        return is_stdin or suffix in _UCS_SUFFIXES
    return False


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


@dataclass(frozen=True, slots=True)
class IruleInput:
    """A single iRule body resolved from a CLI input.

    *origin* is the path / URI of the file the body came from.  When the
    iRule was extracted from a bigip.conf / SCF / UCS, *rule_full_path*
    carries the BIG-IP path of the rule (e.g. ``/Common/foo``); for
    standalone .irule files it is ``None``.  *label* is a display name
    suitable for diagnostics and for synthesising output filenames.
    """

    label: str
    source: str
    origin: str
    rule_full_path: str | None = None


def _read_path_bytes(path_str: str) -> tuple[str, bytes]:
    """Return ``(origin, raw_bytes)`` for *path_str*; ``-`` reads stdin.

    File origins are returned as ``file://`` URIs to match the
    convention used by :func:`read_path` / :func:`load_paths`, so
    callers that mix the two helpers see a single canonical key
    format.
    """
    if path_str == "-":
        return ("stdin://input", sys.stdin.buffer.read())
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    return (path.as_uri(), path.read_bytes())


def _config_to_irule_inputs(
    *,
    origin: str,
    label_prefix: str,
    config: BigipConfig,
) -> list[IruleInput]:
    return [
        IruleInput(
            label=f"{label_prefix}::{rule_path}" if label_prefix else rule_path,
            source=rule.source,
            origin=origin,
            rule_full_path=rule_path,
        )
        for rule_path, rule in config.rules.items()
    ]


def _classify_text_input(
    text: str,
    *,
    origin: str,
    label: str,
) -> tuple[list[IruleInput], BigipConfig | None]:
    """Decide whether *text* is a bigip.conf with rules or a raw iRule.

    If the parser finds at least one ``ltm rule`` stanza, the resulting
    rules are emitted; otherwise the whole text is treated as a single
    iRule body and a synthetic single-rule config is returned.
    """
    config = parse_bigip_conf(text)
    if config.rules:
        return _config_to_irule_inputs(origin=origin, label_prefix=label, config=config), config

    synth_path = f"/{Path(label).stem or 'irule'}" if label else "/irule"
    rule = BigipRule(name=Path(label).stem or "irule", full_path=synth_path, source=text)
    synth = BigipConfig(rules={synth_path: rule})
    return [IruleInput(label=label, source=text, origin=origin, rule_full_path=None)], synth


def _load_one(
    path_str: str,
    *,
    passphrase_provider: PassphraseProvider | None = None,
) -> tuple[list[IruleInput], dict[str, BigipConfig], dict[str, str]]:
    """Resolve a single CLI path to iRule inputs, configs, and source text.

    The third element maps each input's *origin* URI to the post-decode
    source text that was fed into :func:`parse_bigip_conf` — this is
    the text that downstream consumers should use for source-slicing
    (verbatim quoting in error messages, AI context bundles, etc.).
    For UCS inputs it is the *extracted* SCF text, not the gzipped
    archive bytes.
    """
    origin, raw = _read_path_bytes(path_str)
    suffix = Path(path_str).suffix.lower() if path_str != "-" else ""

    # UCS — gzipped tar of /config, possibly OpenPGP-encrypted — extract
    # (decrypting first if needed) and treat as a SCF.  A ``.ucs`` that is
    # neither gzip nor OpenPGP still routes here so it fails with a clear
    # "not a valid UCS archive" rather than being parsed as garbage text.
    if suffix in _UCS_SUFFIXES or _looks_like_ucs(raw, suffix=suffix, is_stdin=path_str == "-"):
        label = path_str if path_str != "-" else "<stdin>"
        text = ucs_archive_to_scf(raw, passphrase_provider=passphrase_provider, label=label)
        cfg = parse_bigip_conf(text)
        return (
            _config_to_irule_inputs(origin=origin, label_prefix=label, config=cfg),
            {origin: cfg},
            {origin: text},
        )

    text = raw.decode("utf-8", errors="replace")
    label = path_str if path_str != "-" else "<stdin>"

    # Standalone iRule file: never try to parse as bigip.conf.
    if suffix in _IRULE_SUFFIXES:
        synth_path = f"/{Path(label).stem or 'irule'}"
        rule = BigipRule(name=Path(label).stem or "irule", full_path=synth_path, source=text)
        synth = BigipConfig(rules={synth_path: rule})
        return (
            [IruleInput(label=label, source=text, origin=origin, rule_full_path=None)],
            {origin: synth},
            {origin: text},
        )

    # bigip.conf / SCF / unknown extension / stdin text — sniff.
    inputs, cfg = _classify_text_input(text, origin=origin, label=label)
    configs = {origin: cfg} if cfg is not None else {}
    return inputs, configs, {origin: text}


def load_irule_inputs(
    paths: list[str],
    *,
    inline_sources: list[str] | None = None,
    passphrase_provider: PassphraseProvider | None = None,
) -> tuple[list[IruleInput], dict[str, BigipConfig], dict[str, str]]:
    """Resolve a mix of paths and inline sources into iRule bodies.

    Each path may be:

    - ``-`` (stdin): bytes are sniffed; gzip/OpenPGP → UCS, otherwise
      text is parsed as a bigip.conf/SCF; if no rules are found the whole
      text is treated as a single iRule.
    - ``.tcl`` / ``.irul`` / ``.irule``: standalone iRule body.
    - ``.ucs``: gzipped tar — plain or OpenPGP-encrypted; canonical SCF
      members are concatenated and every ``ltm rule`` is emitted.
    - ``.conf`` / ``.scf`` (or any other extension): parsed as a bigip
      config; every ``ltm rule`` is emitted.  If no rules are found the
      whole file is treated as a single iRule.

    *inline_sources* is a list of literal iRule snippets supplied via
    ``--source``; each becomes its own :class:`IruleInput`.  Encrypted
    UCS archives resolve their passphrase via *passphrase_provider*
    (default: ``$F5_UCS_PASSPHRASE`` / interactive prompt).

    Returns ``(inputs, configs, sources)``.  *configs* maps each
    input's origin to the parsed (or synthetic single-rule)
    :class:`BigipConfig`, which is what the lint / trace verbs
    consume.  *sources* maps the same origin keys to the post-decode
    source text (the *extracted* SCF for UCS, not the gzip bytes), so
    callers that need to slice the original config text — context
    bundles, error formatters, etc. — get a single canonical view
    keyed identically to *configs*.

    Raises :class:`FileNotFoundError` for missing files,
    :class:`ValueError` for malformed UCS archives.
    """
    inputs: list[IruleInput] = []
    configs: dict[str, BigipConfig] = {}
    sources: dict[str, str] = {}

    for index, source_text in enumerate(inline_sources or [], start=1):
        label = f"<inline:{index}>"
        synth_name = f"inline_{index}"
        synth_path = f"/{synth_name}"
        rule = BigipRule(name=synth_name, full_path=synth_path, source=source_text)
        synth_cfg = BigipConfig(rules={synth_path: rule})
        origin = f"inline://{index}"
        inputs.append(
            IruleInput(
                label=label,
                source=source_text,
                origin=origin,
                rule_full_path=None,
            )
        )
        configs[origin] = synth_cfg
        sources[origin] = source_text

    for path_str in paths:
        loaded, loaded_configs, loaded_sources = _load_one(
            path_str, passphrase_provider=passphrase_provider
        )
        inputs.extend(loaded)
        configs.update(loaded_configs)
        sources.update(loaded_sources)

    return inputs, configs, sources
