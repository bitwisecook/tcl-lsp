"""Shared file-input helpers for f5 verbs.

The cleanup/grep verbs duplicated this trivially; centralising it
keeps every new verb consistent.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

from core.bigip.model import BigipConfig, BigipRule
from core.bigip.parser import parse_bigip_conf

_IRULE_SUFFIXES = frozenset({".tcl", ".irul", ".irule"})
_BIGIP_SUFFIXES = frozenset({".conf", ".scf"})
_UCS_SUFFIXES = frozenset({".ucs"})


def read_path(path_str: str) -> tuple[str, str]:
    """Return ``(uri, source)`` for *path_str*.  ``-`` reads stdin."""
    if path_str == "-":
        return ("stdin://input", sys.stdin.read())
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    return (path.as_uri(), path.read_text(encoding="utf-8", errors="replace"))


def load_paths(paths: list[str]) -> tuple[dict[str, str], dict[str, BigipConfig]]:
    sources: dict[str, str] = {}
    configs: dict[str, BigipConfig] = {}
    for p in paths:
        uri, src = read_path(p)
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


def _is_gzip(data: bytes) -> bool:
    return len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B


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

    # UCS — gzipped tar of /config — extract and treat as a SCF.
    if suffix in _UCS_SUFFIXES or _is_gzip(raw):
        from explorer.f5_remote.ucs import is_ucs_bytes, ucs_to_scf

        if not is_ucs_bytes(raw):
            raise ValueError(f"{path_str}: not a valid UCS archive")
        text = ucs_to_scf(raw)
        cfg = parse_bigip_conf(text)
        label = path_str if path_str != "-" else "<stdin>"
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
) -> tuple[list[IruleInput], dict[str, BigipConfig], dict[str, str]]:
    """Resolve a mix of paths and inline sources into iRule bodies.

    Each path may be:

    - ``-`` (stdin): bytes are sniffed; gzip → UCS, otherwise text is
      parsed as a bigip.conf/SCF; if no rules are found the whole text
      is treated as a single iRule.
    - ``.tcl`` / ``.irul`` / ``.irule``: standalone iRule body.
    - ``.ucs``: gzipped tar; canonical SCF members are concatenated and
      every ``ltm rule`` is emitted.
    - ``.conf`` / ``.scf`` (or any other extension): parsed as a bigip
      config; every ``ltm rule`` is emitted.  If no rules are found the
      whole file is treated as a single iRule.

    *inline_sources* is a list of literal iRule snippets supplied via
    ``--source``; each becomes its own :class:`IruleInput`.

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
        loaded, loaded_configs, loaded_sources = _load_one(path_str)
        inputs.extend(loaded)
        configs.update(loaded_configs)
        sources.update(loaded_sources)

    return inputs, configs, sources
