"""``f5 merge`` — concatenate split SCFs back into a single file."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.emit import emit_merged

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb


@verb(
    "merge",
    aliases=(),
    help="Concatenate split per-partition SCFs back into a single bigip.conf.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Inverse of `f5 split`: read every *.conf file under DIR (or "
        "the explicit list of paths) in lexicographic order, "
        "concatenate them into one SCF, and emit the result to stdout "
        "or --output."
    )
    p.add_argument("paths", nargs="+", help="Per-partition .conf files (or one directory).")
    p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    add_format_arg(p, tmsh_default_verb="create")
    p.set_defaults(handler=_run_merge)


def _run_merge(args: argparse.Namespace) -> int:
    files: list[Path] = []
    for raw in args.paths:
        path = Path(raw)
        if path.is_dir():
            files.extend(sorted(path.glob("*.conf")))
        else:
            files.append(path)

    chunks: list[str] = []
    for f in files:
        try:
            _, src = read_path(str(f))
        except OSError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        chunks.append(src)

    output = emit_merged(chunks)
    if not output.endswith("\n"):
        output += "\n"
    output = render_config(output, fmt=args.output_format, tmsh_verb="create")
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 0
