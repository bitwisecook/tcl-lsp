"""Shared output-format plumbing for f5 verbs that emit configuration text.

Every config-producing verb (``f5 extract`` / ``pull`` / ``split`` /
``merge`` / ``grep`` / ``rename`` / ``redact`` / ``unredact``) gains a
``--format scf|tmsh`` flag whose ``scf`` (default) value preserves the
verb's historical output — a bigip.conf-style stanza dump — and whose
``tmsh`` value re-renders the same content as a ``tmsh create`` /
``tmsh modify`` script suitable for pasting into a BIG-IP shell or
piping into ``tmsh``.

The two forms are interchangeable as input on the receiving side:
``f5 diff`` and (transitively) every consumer that walks back through
:mod:`core.bigip.tmsh_parse` will accept either.
"""

from __future__ import annotations

import argparse
import sys
from typing import Literal

from core.bigip.parser import parse_bigip_conf
from core.bigip.tmsh_emit import emit_tmsh

ConfigFormat = Literal["scf", "tmsh"]
TmshVerb = Literal["create", "modify"]


def add_format_arg(
    parser: argparse.ArgumentParser,
    *,
    tmsh_default_verb: TmshVerb = "create",
) -> None:
    """Add a uniform ``--format scf|tmsh`` flag to *parser*.

    *tmsh_default_verb* is shown in --help so users see whether the
    verb's tmsh output uses ``create`` (extractive verbs) or ``modify``
    (in-place rewriters).  The actual verb used at runtime is supplied
    to :func:`render_config` by the calling handler, not by the user.
    """
    parser.add_argument(
        "--format",
        choices=("scf", "tmsh"),
        default="scf",
        dest="output_format",
        help=(
            "Output format.  `scf` (default) emits bigip.conf / SCF "
            "stanzas; `tmsh` re-renders the same objects as `tmsh "
            f"{tmsh_default_verb}` commands in dependency order, "
            "suitable for pasting into a BIG-IP shell.  Both forms can "
            "be merged back into a device's config and are accepted as "
            "input by `f5 diff`."
        ),
    )


def render_config(
    text: str,
    *,
    fmt: ConfigFormat,
    tmsh_verb: TmshVerb = "create",
) -> str:
    """Render an SCF *text* as either SCF or a tmsh script.

    *fmt* is the user-selected ``--format`` value.  *tmsh_verb*
    controls whether tmsh emission uses ``create`` (right for verbs
    that surface a fresh subset — extract, pull, grep, split, merge)
    or ``modify`` (right for in-place rewriters whose result is meant
    to overwrite already-present objects on a device — rename, redact,
    unredact).

    For *fmt='scf'* the text is returned verbatim so callers see no
    difference from the historical code path.  For *fmt='tmsh'* the
    text is parsed and re-emitted; when the parse yields no objects
    the helper emits a warning on stderr and falls back to the raw
    SCF so the user still gets something useful (this is the
    ``unredact`` non-config-input case).
    """
    if fmt == "scf":
        return text
    cfg = parse_bigip_conf(text)
    if not _has_any_object(cfg):
        print(
            "warning: --format tmsh: input did not parse as a BIG-IP config; "
            "emitting raw text instead.",
            file=sys.stderr,
        )
        return text
    script = emit_tmsh(
        cfg,
        source=text,
        use_modify_for_existing=(tmsh_verb == "modify"),
    )
    return script.text


def _has_any_object(cfg: object) -> bool:
    """Return True when *cfg* parsed into at least one structured object."""
    for attr in (
        "virtual_servers",
        "pools",
        "nodes",
        "monitors",
        "profiles",
        "data_groups",
        "snat_pools",
        "persistence",
        "rules",
        "policies",
        "generic_objects",
    ):
        if getattr(cfg, attr, None):
            return True
    return False
