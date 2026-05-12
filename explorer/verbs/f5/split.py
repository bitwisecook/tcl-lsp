"""``f5 split`` — write one file per partition into a directory."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.emit import emit_split_by_partition

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb


@verb(
    "split",
    aliases=(),
    help="Split an SCF into per-partition files under a directory (suitable for git).",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Write one file per partition: e.g. /Common/, /Common2/, ... "
        "Stanzas without an identifier-style partition are gathered "
        "under '_no_partition' (suffix `.conf` for `--format scf`, "
        "`.tmsh` for `--format tmsh`).  Source ordering and whitespace "
        "are preserved within each chunk."
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument("output", help="Output directory (created if needed).")
    add_format_arg(p, tmsh_default_verb="create")
    p.set_defaults(handler=_run_split)


def _run_split(args: argparse.Namespace) -> int:
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    out_dir = Path(args.output)
    out_dir.mkdir(parents=True, exist_ok=True)

    parts = emit_split_by_partition(source)
    suffix = "tmsh" if args.output_format == "tmsh" else "conf"
    for partition, text in parts.items():
        out_text = render_config(text, fmt=args.output_format, tmsh_verb="create")
        (out_dir / f"{partition}.{suffix}").write_text(out_text, encoding="utf-8")

    print(
        f"split into {len(parts)} partition file(s) under {out_dir}",
        file=sys.stderr,
    )
    return 0
