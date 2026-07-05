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

"""Highlight verb: emit syntax-highlighted source output."""

from __future__ import annotations

import argparse

from compiler.registry.runtime import configure_signatures
from tooling.cli._utils import (
    _add_colour_arguments,
    _add_input_arguments,
    _combine_sources,
    _highlight_source_ansi,
    _highlight_source_html,
    _read_input_documents,
    _resolve_tab_width,
    _resolve_use_colour,
    _write_text_output,
)

from ._registry import verb


@verb(
    "highlight",
    aliases=("hl",),
    help="Emit syntax-highlighted source output.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_highlight(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Emit the source with token-level syntax highlighting applied:\n"
        "commands, sub-commands, variable refs, command substitutions,\n"
        "braced strings, comments, and {*}-expansion get distinct colours\n"
        "or CSS classes.  Choose `ansi` for terminal output (default) or\n"
        "`html` for a `<pre>`-wrapped HTML fragment with inline styles.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} highlight script.tcl --colour\n"
        f"  {prog_name} highlight script.tcl --format html -o out.html\n"
        f"  {prog_name} highlight irule.irul --dialect f5-irules --format ansi\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--format",
        choices=("ansi", "html"),
        default="ansi",
        help="Output format (default: ansi).",
    )
    _add_colour_arguments(p)
    p.set_defaults(handler=_run_highlight)


def _run_highlight(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    if args.format == "html":
        output = _highlight_source_html(source)
    else:
        output = _highlight_source_ansi(source, use_colour=_resolve_use_colour(args))

    tw = _resolve_tab_width(args)
    if args.output == "-" and tw > 0:
        output = output.expandtabs(tw)
    _write_text_output(args.output, output)
    return 0
