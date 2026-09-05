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

"""Command-line entry point: ``f5-report`` / ``python -m f5report``.

Turns one or more BIG-IP configs or UCS archives into a single self-contained
HTML report, using the native query engine for every fact it shows.
"""

from __future__ import annotations

import argparse
import base64
import json
import mimetypes
import sys
import uuid as _uuid

from . import __version__, engine_version, load_paths
from .report import build_report, collect_model


def _logo_data_uri(path: str) -> str:
    """Read an image file and return it as an inlined ``data:`` URI.

    The report is a single self-contained file, so the logo is embedded rather
    than referenced. The MIME type is guessed from the extension (``.svg`` →
    ``image/svg+xml``); an unknown type falls back to ``application/octet-stream``.
    """
    with open(path, "rb") as fh:
        data = fh.read()
    mime, _ = mimetypes.guess_type(path)
    if mime is None:
        mime = (
            "image/svg+xml"
            if path.lower().endswith(".svg")
            else "application/octet-stream"
        )
    return f"data:{mime};base64,{base64.b64encode(data).decode('ascii')}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="f5-report",
        description="Generate an interactive HTML report from F5 BIG-IP configs "
        "or UCS archives, powered by the f5-query engine (via PyO3).",
    )
    parser.add_argument(
        "inputs",
        nargs="+",
        metavar="PATH",
        help="bigip.conf / SCF files or .ucs archives (plain or encrypted)",
    )
    parser.add_argument(
        "-o",
        "--output",
        default="bigip-report.html",
        help="output HTML path (default: %(default)s; '-' for stdout)",
    )
    parser.add_argument(
        "-t", "--title", default="F5 BIG-IP Configuration Report", help="report title"
    )
    parser.add_argument(
        "--passphrase",
        default=None,
        help="passphrase for an encrypted UCS (or set $F5_UCS_PASSPHRASE)",
    )
    parser.add_argument(
        "--f5mku",
        metavar="KEY",
        default=None,
        help="base64 unit master key (f5mku -K) — decrypts the config's "
        "$M$ secrets (SSL key passphrases, monitor/RADIUS/SNMP secrets)",
    )
    parser.add_argument(
        "--f5mku-file",
        metavar="FILE",
        default=None,
        help="read the base64 master key from FILE (e.g. f5mku -K > key.txt)",
    )
    parser.add_argument(
        "--include-extras",
        action="store_true",
        help="fold every config/*.conf UCS member (partitions, GTM) into the report",
    )
    parser.add_argument(
        "--no-console",
        action="store_true",
        help="omit the in-browser WASM query console (smaller page; "
        "suitable for hosting where a strict CSP blocks WebAssembly)",
    )
    parser.add_argument(
        "--copyright",
        metavar="TEXT",
        default=None,
        help="copyright / confidentiality notice shown in the report "
        "footer (screen, mobile and print); or read from --copyright-file",
    )
    parser.add_argument(
        "--copyright-file",
        metavar="FILE",
        default=None,
        help="read the footer copyright notice from FILE",
    )
    parser.add_argument(
        "--front-matter",
        metavar="FILE",
        default=None,
        help="Markdown file shown in a 'Front matter' tab at the front of "
        "the report (rendered to HTML at generation time)",
    )
    parser.add_argument(
        "--logo",
        metavar="FILE",
        default=None,
        help="image file (PNG/JPEG/SVG/GIF/WebP) shown as the report's "
        "header logo; inlined as a data: URI so the report stays one file",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="emit the report model as JSON instead of HTML",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"f5-report {__version__} (engine {engine_version()})",
    )
    args = parser.parse_args(argv)

    master_key = args.f5mku
    if args.f5mku_file:
        try:
            with open(args.f5mku_file, encoding="utf-8") as fh:
                master_key = fh.read().strip()
        except OSError as exc:
            print(f"f5-report: could not read --f5mku-file: {exc}", file=sys.stderr)
            return 2

    copyright_notice = args.copyright or ""
    if args.copyright_file:
        try:
            with open(args.copyright_file, encoding="utf-8") as fh:
                copyright_notice = fh.read().strip()
        except OSError as exc:
            print(f"f5-report: could not read --copyright-file: {exc}", file=sys.stderr)
            return 2

    front_matter = ""
    if args.front_matter:
        try:
            with open(args.front_matter, encoding="utf-8") as fh:
                front_matter = fh.read()
        except OSError as exc:
            print(f"f5-report: could not read --front-matter: {exc}", file=sys.stderr)
            return 2

    logo = ""
    if args.logo:
        try:
            logo = _logo_data_uri(args.logo)
        except OSError as exc:
            print(f"f5-report: could not read --logo: {exc}", file=sys.stderr)
            return 2

    try:
        sources = load_paths(
            args.inputs, passphrase=args.passphrase, include_extras=args.include_extras
        )
    except Exception as exc:  # noqa: BLE001 — surface a clean CLI error
        print(f"f5-report: failed to load inputs: {exc}", file=sys.stderr)
        return 2

    try:
        if args.json:
            out = json.dumps(
                collect_model(
                    sources,
                    title=args.title,
                    master_key=master_key,
                    copyright=copyright_notice,
                    front_matter=front_matter,
                    logo=logo,
                ),
                indent=2,
                default=str,
            )
        else:
            out = build_report(
                sources,
                title=args.title,
                embed_console=False if args.no_console else None,
                master_key=master_key,
                report_id=str(_uuid.uuid4()),
                copyright=copyright_notice,
                front_matter=front_matter,
                logo=logo,
            )
    except Exception as exc:  # noqa: BLE001 — surface a clean CLI error (e.g. wrong master key)
        print(f"f5-report: {exc}", file=sys.stderr)
        return 2

    if args.output == "-":
        sys.stdout.write(out)
    else:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(out)
        kind = "model" if args.json else "report"
        print(
            f"f5-report: wrote {kind} for {len(sources)} device(s) to {args.output} "
            f"({len(out):,} bytes)",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
