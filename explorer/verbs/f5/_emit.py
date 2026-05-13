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
from core.bigip.tmsh_emit import emit_tmsh, emit_tmsh_delta

ConfigFormat = Literal["scf", "tmsh", "tmsh-delta"]
TmshVerb = Literal["create", "modify"]


def add_format_arg(
    parser: argparse.ArgumentParser,
    *,
    tmsh_default_verb: TmshVerb = "create",
) -> None:
    """Add a uniform ``--format scf|tmsh`` flag (plus ``--transaction``)
    to *parser*.

    *tmsh_default_verb* is shown in --help so users see whether the
    verb's tmsh output uses ``create`` (extractive verbs) or ``modify``
    (in-place rewriters).  The actual verb used at runtime is supplied
    to :func:`render_config` by the calling handler, not by the user.

    ``--transaction`` wraps tmsh output in
    ``cli transaction``/``submit-transaction`` so every command applies
    atomically — if any one fails, the device rolls back the whole
    script.  No-op for ``--format scf``.
    """
    parser.add_argument(
        "--format",
        choices=("scf", "tmsh", "tmsh-delta"),
        default="scf",
        dest="output_format",
        help=(
            "Output format.  `scf` (default) emits bigip.conf / SCF "
            "stanzas; `tmsh` re-renders every object as a `tmsh "
            f"{tmsh_default_verb}` command in dependency order; "
            "`tmsh-delta` emits ONLY the objects that changed between "
            "the original input and the rewrite, using `create` for "
            "added objects, `modify` for changed ones, and `delete` "
            "for removed ones.  `tmsh-delta` is the tight option for "
            "rolling a change onto a device that already has the "
            "skeleton; `tmsh` is the right one for a fresh replay.  "
            "All forms can be merged back into a device's config and "
            "are accepted as input by `f5 diff`."
        ),
    )
    parser.add_argument(
        "--transaction",
        action="store_true",
        dest="output_transaction",
        help=(
            "Wrap tmsh output in a ``cli transaction`` block so every "
            "command applies atomically — if any one fails, the device "
            "rolls back the entire script.  Requires ``--format tmsh`` "
            "(no-op for the SCF format).  Use this for production "
            "rollouts where partial-failure recovery would leave the "
            "device in a mixed state."
        ),
    )


def render_config(
    text: str,
    *,
    fmt: ConfigFormat,
    tmsh_verb: TmshVerb = "create",
    transaction: bool = False,
    original: str = "",
) -> str:
    """Render an SCF *text* as either SCF, a full tmsh script, or a tmsh delta.

    *fmt* is the user-selected ``--format`` value.  *tmsh_verb*
    controls whether full tmsh emission uses ``create`` (right for
    verbs that surface a fresh subset — extract, pull, grep, split,
    merge) or ``modify`` (right for in-place rewriters whose result
    is meant to overwrite already-present objects on a device —
    rename, redact, unredact).

    *fmt='tmsh-delta'* requires *original* (the pre-edit text) and
    emits only the objects that changed between *original* and
    *text*: ``tmsh create`` for added, ``tmsh modify`` for changed,
    ``tmsh delete`` for removed.  Callers that don't have an
    original (extract / pull / grep) should pass ``original=text``
    or stick with plain ``tmsh``.

    For *fmt='scf'* the text is returned verbatim so callers see no
    difference from the historical code path.  For *fmt='tmsh'* /
    *'tmsh-delta'* the text is parsed and re-emitted; when the parse
    yields no objects the helper emits a warning on stderr and falls
    back to the raw SCF so the user still gets something useful
    (this is the ``unredact`` non-config-input case).

    *transaction* (only meaningful with ``fmt='tmsh'`` / *'tmsh-delta'*)
    wraps the emitted script in :func:`wrap_tmsh_transaction` so the
    BIG-IP applies every command atomically.
    """
    if fmt == "scf":
        return text
    cfg = parse_bigip_conf(text)
    if not _has_any_object(cfg):
        print(
            f"warning: --format {fmt}: input did not parse as a BIG-IP config; "
            "emitting raw text instead.",
            file=sys.stderr,
        )
        return text
    if fmt == "tmsh-delta":
        # Delta mode needs both sides parsed so we can compare
        # per-kind containers and per-stanza text.  When the caller
        # didn't supply an *original* (or supplied the same text)
        # the delta is empty; emit nothing rather than re-render the
        # whole config.
        old_cfg = parse_bigip_conf(original) if original else type(cfg)()
        script = emit_tmsh_delta(
            old_cfg,
            cfg,
            old_source=original,
            new_source=text,
        )
    else:
        script = emit_tmsh(
            cfg,
            source=text,
            use_modify_for_existing=(tmsh_verb == "modify"),
        )
    if transaction:
        return wrap_tmsh_transaction(script.text)
    return script.text


def wrap_tmsh_transaction(tmsh_script: str) -> str:
    """Wrap *tmsh_script* in F5's ``cli transaction`` envelope.

    Every ``tmsh`` command between ``cli transaction`` and
    ``submit-transaction`` applies atomically on the device — if
    any one fails the entire transaction rolls back, leaving the
    device exactly as it was before.  Without the envelope, a
    partial-failure run can leave the config in a mixed state.

    The envelope is idempotent: if the input already starts with
    ``cli transaction``, this helper returns the text unchanged so
    callers can wrap without worrying about double-wrapping.

    Leading ``#`` comment lines are preserved before the envelope
    so the script's header banner stays at the top.
    """
    stripped = tmsh_script.lstrip()
    if stripped.startswith("cli transaction"):
        return tmsh_script  # already wrapped
    # Preserve leading comments (banner) above the envelope.
    lines = tmsh_script.splitlines(keepends=False)
    head_lines: list[str] = []
    body_start = 0
    for i, line in enumerate(lines):
        if line.startswith("#") or not line.strip():
            head_lines.append(line)
            body_start = i + 1
        else:
            break
    body_lines = lines[body_start:]
    if not body_lines:
        return tmsh_script  # nothing to wrap (only comments / blanks)
    head = "\n".join(head_lines)
    body = "\n".join(body_lines)
    head_block = (head + "\n") if head else ""
    return f"{head_block}cli transaction\n{body}\nsubmit-transaction\n"


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
