"""Source-rewriting helpers shared by ``f5 rename`` and ``f5 redact``.

Both verbs operate on the source text and reassemble it with minimal
disruption.  Everything here is text-level — no parser round-trip — so
comments, whitespace, and ordering survive.

Functions:

- :func:`rename_object` — rename one full-path (and update every
  reference: property values, embedded iRule body command arguments).
- :func:`redact_secrets` — replace credential/secret-bearing property
  values and (consistently) remap real IPs to RFC1918.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import TYPE_CHECKING

from .parser import parse_bigip_conf

if TYPE_CHECKING:
    from .redact_map import RedactionMap

# ── rename ──────────────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class RenameReport:
    old: str
    new: str
    occurrences: int  # total replacements made
    new_source: str


def _build_name_pattern(old: str) -> re.Pattern[str]:
    # Match the full-path as a standalone token: bounded by characters that
    # don't appear in BIG-IP identifiers.  Identifiers can contain
    # /, -, _, alphanumerics, and dots.
    escaped = re.escape(old)
    return re.compile(rf"(?<![A-Za-z0-9_/.\-]){escaped}(?![A-Za-z0-9_/.\-])")


def rename_object(source: str, old: str, new: str) -> RenameReport:
    """Rename *old* to *new* everywhere in *source* (text-level).

    Only full-path references are rewritten.  The match is bounded so
    substring collisions don't fire (``/Common/foo`` won't accidentally
    match ``/Common/foobar``).

    Short-name references (``foo`` instead of ``/Common/foo``) are
    *not* rewritten: they're unsafe to handle by regex because the same
    token can legitimately appear in comments, string literals, or
    arbitrary property values without referring to the object.  Callers
    that need short-name handling should rewrite their iRule bodies to
    use full paths first (``f5 grep --cidr`` / your editor of choice).
    """
    if not old:
        raise ValueError("old name is empty")
    if not new:
        raise ValueError("new name is empty")
    pattern = _build_name_pattern(old)
    new_source, count = pattern.subn(new, source)

    # Sanity check: the renamed source must still parse.
    try:
        parse_bigip_conf(new_source)
    except Exception as exc:  # noqa: BLE001
        raise ValueError(f"rename produced invalid SCF: {exc}") from exc

    return RenameReport(old=old, new=new, occurrences=count, new_source=new_source)


# ── redact ──────────────────────────────────────────────────────────

_SECRET_KEYS = (
    "passphrase",
    "password",
    "encrypted-password",
    "secret",
    "shared-secret",
    "community",  # SNMP
    "auth-password",
    "priv-password",
    "rcv",  # monitor receive strings often contain creds
)

_PEM_RE = re.compile(
    r"-----BEGIN [A-Z ]+-----.*?-----END [A-Z ]+-----",
    re.DOTALL,
)


@dataclass(frozen=True, slots=True)
class RedactReport:
    secrets_replaced: int
    pem_blocks_replaced: int
    ips_remapped: int
    new_source: str
    redaction_map: "RedactionMap | None" = None


def _redact_property_values(source: str) -> tuple[str, int]:
    count = 0
    out = source
    for key in _SECRET_KEYS:
        # Match: word-boundary KEY whitespace VALUE.  VALUE is one
        # token — either a quoted string or a sequence of non-whitespace,
        # non-brace characters — so we work on both single-line bodies
        # (`{ key val }`) and multi-line ones (`\n    key val\n`).
        pattern = re.compile(
            rf'(?<![\w-])({re.escape(key)})\s+("(?:[^"\\]|\\.)*"|[^\s{{}}]+)',
        )

        def _sub(match: re.Match[str]) -> str:
            nonlocal count
            count += 1
            return f"{match.group(1)} <REDACTED>"

        out = pattern.sub(_sub, out)
    return out, count


_PEM_HEADER_RE = re.compile(r"-----BEGIN [A-Z ]+-----")
_PEM_FOOTER_RE = re.compile(r"-----END [A-Z ]+-----")


def _redact_pem_blocks(source: str) -> tuple[str, int]:
    count = 0

    def _sub(match: re.Match[str]) -> str:
        nonlocal count
        count += 1
        whole = match.group(0)
        hdr = _PEM_HEADER_RE.search(whole)
        ftr = _PEM_FOOTER_RE.search(whole)
        header = hdr.group(0) if hdr else "-----BEGIN BLOCK-----"
        footer = ftr.group(0) if ftr else "-----END BLOCK-----"
        return f"{header}<REDACTED>{footer}"

    out = _PEM_RE.sub(_sub, source)
    return out, count


def redact_secrets(
    source: str,
    *,
    remap_ips: bool = True,
    target_pool_v4: tuple[str, ...] | None = None,
    target_pool_v6: tuple[str, ...] | None = None,
    mode: str = "direct",
    seed: str = "",
    explicit_source_cidrs: tuple[str, ...] = (),
    existing_map: "RedactionMap | None" = None,
    remap_private: bool = False,
) -> RedactReport:
    """Redact secrets in *source*; optionally remap public IPs.

    When *existing_map* is supplied, prior assignments are reused so the
    same IP keeps mapping to the same redacted address across runs —
    essential for iterative support workflows.  Returns the rewritten
    text plus the (possibly extended) :class:`RedactionMap`.
    """
    from .redact_map import (
        DEFAULT_TARGET_CIDRS_V4,
        DEFAULT_TARGET_CIDRS_V6,
        RedactionMap,
        apply_map,
        build_map,
    )

    out, secrets = _redact_property_values(source)
    out, pem = _redact_pem_blocks(out)
    if remap_ips:
        rm = build_map(
            text=out,
            target_pool_v4=target_pool_v4 or DEFAULT_TARGET_CIDRS_V4,
            target_pool_v6=target_pool_v6 or DEFAULT_TARGET_CIDRS_V6,
            mode=mode,
            seed=seed,
            explicit_source_cidrs=explicit_source_cidrs,
            existing=existing_map,
            remap_private=remap_private,
        )
        out, ips = apply_map(rm, out, reverse=False)
    else:
        rm = RedactionMap()
        ips = 0
    return RedactReport(
        secrets_replaced=secrets,
        pem_blocks_replaced=pem,
        ips_remapped=ips,
        new_source=out,
        redaction_map=rm,
    )
