"""Resolve BIG-IP config / iRule file inputs into the F5 model.

The home for turning CLI-style paths (``.conf`` / ``.scf`` / ``.ucs`` /
``.irule`` / ``-``) and inline snippets into parsed :class:`BigipConfig`
objects + iRule bodies. It lives with the F5 model (``dialects.f5.bigip``)
rather than the ``tooling.f5`` verb layer so any F5-model consumer — the CLI
verbs, ``explain_flow``, and the AI context tools — shares one loader without
depending on the retiring analysis-engine tooling.

The only non-model dependency is the UCS archive crypto in
``tooling.f5.f5_remote`` (OpenPGP / gzip-tar handling), which is F5
remote-transport infrastructure.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

from dialects.f5.bigip.model import BigipConfig, BigipRule
from dialects.f5.bigip.parser import parse_bigip_conf
from tooling.f5.f5_remote.ucs import (
    PassphraseProvider,
    is_pgp_bytes,
    ucs_archive_to_scf,
)

_IRULE_SUFFIXES = frozenset({".tcl", ".irul", ".irule"})
_BIGIP_SUFFIXES = frozenset({".conf", ".scf"})
_UCS_SUFFIXES = frozenset({".ucs"})


@dataclass(frozen=True, slots=True)
class IruleInput:
    """A single iRule body resolved from a CLI input.

    *origin* is the path / URI of the file the body came from. When the iRule
    was extracted from a bigip.conf / SCF / UCS, *rule_full_path* carries the
    BIG-IP path of the rule (e.g. ``/Common/foo``); for standalone .irule files
    it is ``None``. *label* is a display name suitable for diagnostics.
    """

    label: str
    source: str
    origin: str
    rule_full_path: str | None = None


def _is_gzip(data: bytes) -> bool:
    return len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B


def _looks_like_ucs(raw: bytes, *, suffix: str, is_stdin: bool) -> bool:
    """Whether *raw* should be extracted/decrypted as a UCS archive.

    OpenPGP magic is unambiguous; plain gzip magic is only treated as a UCS for
    ``.ucs`` paths or stdin (no suffix to go on).
    """
    if is_pgp_bytes(raw):
        return True
    if _is_gzip(raw):
        return is_stdin or suffix in _UCS_SUFFIXES
    return False


def _read_path_bytes(path_str: str) -> tuple[str, bytes]:
    """Return ``(origin, raw_bytes)`` for *path_str*; ``-`` reads stdin."""
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
    """Decide whether *text* is a bigip.conf with rules or a raw iRule."""
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
    """Resolve a single CLI path to iRule inputs, configs, and source text."""
    origin, raw = _read_path_bytes(path_str)
    suffix = Path(path_str).suffix.lower() if path_str != "-" else ""

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

    if suffix in _IRULE_SUFFIXES:
        synth_path = f"/{Path(label).stem or 'irule'}"
        rule = BigipRule(name=Path(label).stem or "irule", full_path=synth_path, source=text)
        synth = BigipConfig(rules={synth_path: rule})
        return (
            [IruleInput(label=label, source=text, origin=origin, rule_full_path=None)],
            {origin: synth},
            {origin: text},
        )

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

    Returns ``(inputs, configs, sources)``. *configs* maps each input's origin
    to the parsed (or synthetic single-rule) :class:`BigipConfig`; *sources*
    maps the same origin keys to the post-decode source text (the *extracted*
    SCF for UCS, not the gzip bytes).

    Raises :class:`FileNotFoundError` for missing files, :class:`ValueError`
    for malformed UCS archives.
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
            IruleInput(label=label, source=source_text, origin=origin, rule_full_path=None)
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
