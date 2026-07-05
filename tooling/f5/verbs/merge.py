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

"""``f5 merge`` — concatenate split SCFs back into a single file."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.emit import emit_merged

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb


@verb(
    "merge",
    aliases=(),
    help="Concatenate split per-partition SCFs back into a single bigip.conf.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Inverse of `f5 split`: read every *.conf file under DIR (or\n"
        "the explicit list of paths) in lexicographic order,\n"
        "concatenate them into one SCF, and emit the result to stdout\n"
        "or --output."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 merge partitions/ -o bigip.conf\n"
        "  f5 merge common.conf prod.conf gtm.conf -o bigip.conf\n"
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
    output = render_config(
        output,
        fmt=args.output_format,
        tmsh_verb="create",
        transaction=getattr(args, "output_transaction", False),
    )
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 0
