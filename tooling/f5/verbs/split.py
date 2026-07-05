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

"""``f5 split`` — write one file per partition into a directory."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.emit import emit_split_by_partition

from ._emit import add_format_arg, render_config
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
        "under '_no_partition' (suffix `.conf` for `--format scf`,\n"
        "`.tmsh` for `--format tmsh`).  Source ordering and whitespace\n"
        "are preserved within each chunk, which makes the resulting\n"
        "tree well-suited to version control (per-partition diffs)."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 split bigip.conf partitions/\n"
        "  f5 split prod.scf out/   # creates out/Common.conf, out/Prod.conf, ...\n"
        "  f5 split prod.scf out/ --format tmsh   # writes .tmsh scripts instead\n"
        "  f5 split - partitions/ < bigip.conf\n"
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
        out_text = render_config(
            text,
            fmt=args.output_format,
            tmsh_verb="create",
            transaction=getattr(args, "output_transaction", False),
        )
        (out_dir / f"{partition}.{suffix}").write_text(out_text, encoding="utf-8")

    print(
        f"split into {len(parts)} partition file(s) under {out_dir}",
        file=sys.stderr,
    )
    return 0
