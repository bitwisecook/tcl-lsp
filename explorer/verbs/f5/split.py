"""``f5 split`` — write one file per partition into a directory."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.emit import emit_split_by_partition

from ._paths import read_path
from ._registry import verb


@verb(
    "split",
    aliases=(),
    help="Split an SCF into per-partition files under a directory (suitable for git).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Write one file per partition: e.g. /Common/, /Common2/, ...\n"
        "Stanzas without an identifier-style partition are gathered\n"
        "under '_no_partition.conf'.  Source ordering and whitespace\n"
        "are preserved within each chunk, which makes the resulting\n"
        "tree well-suited to version control (per-partition diffs)."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 split bigip.conf partitions/\n"
        "  f5 split prod.scf out/   # creates out/Common.conf, out/Prod.conf, ...\n"
        "  f5 split - partitions/ < bigip.conf\n"
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument("output", help="Output directory (created if needed).")
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
    for partition, text in parts.items():
        (out_dir / f"{partition}.conf").write_text(text, encoding="utf-8")

    print(
        f"split into {len(parts)} partition file(s) under {out_dir}",
        file=sys.stderr,
    )
    return 0
