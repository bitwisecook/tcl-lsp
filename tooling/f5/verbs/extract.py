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

"""``f5 extract`` — convert a local UCS archive to an SCF text.

The inverse of ``tmsh load sys ucs`` for offline analysis: given a
UCS file someone has already pulled off a device, write out the
concatenated ``bigip*.conf`` content so it can be fed to the rest
of the ``f5`` CLI (``cleanup``, ``grep``, ``diff``, …).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tooling.f5.f5_remote.ucs import extract_ucs_file

from ._emit import add_format_arg, render_config
from ._paths import add_passphrase_args, provider_from_args
from ._registry import verb


@verb(
    "extract",
    aliases=("ucs2scf",),
    help="Convert a local UCS archive to an SCF text file.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Read a BIG-IP UCS backup archive (.ucs is gzip+tar of /config)\n"
        "and write a concatenated Single Configuration File text suitable\n"
        "for the rest of the f5 CLI verbs.  By default only the canonical\n"
        "bigip_base / bigip / bigip_gtm / bigip_user / bigip_script members\n"
        "are included; use --include-extras to pull every additional\n"
        "config/*.conf member found inside the archive.\n"
        "\n"
        "Encrypted UCS archives (tmsh save sys ucs ... passphrase ...) are\n"
        "decrypted transparently; the passphrase is read from the\n"
        "F5_UCS_PASSPHRASE environment variable or prompted for securely."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 extract prod.ucs > prod.scf\n"
        "  f5 extract prod.ucs -o prod.scf\n"
        "  f5 extract prod.ucs --include-extras -o full.scf\n"
        "  F5_UCS_PASSPHRASE=s3cret f5 extract encrypted.ucs -o prod.scf\n"
    )
    p.add_argument(
        "ucs",
        help="Path to a .ucs file.",
    )
    p.add_argument(
        "-o",
        "--output",
        metavar="FILE",
        help="Write SCF text here (default: stdout).",
    )
    p.add_argument(
        "--include-extras",
        action="store_true",
        help=(
            "Include any additional config/*.conf members not in the "
            "canonical bigip_base/bigip/bigip_gtm/bigip_user/bigip_script set."
        ),
    )
    add_format_arg(p, tmsh_default_verb="create")
    add_passphrase_args(p)
    p.set_defaults(handler=_run_extract)


def _run_extract(args: argparse.Namespace) -> int:
    ucs_path = Path(args.ucs)
    if not ucs_path.is_file():
        print(f"error: not a file: {args.ucs}", file=sys.stderr)
        return 2
    try:
        scf = extract_ucs_file(
            ucs_path,
            passphrase_provider=provider_from_args(args),
            include_extras=args.include_extras,
        )
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    output = render_config(
        scf,
        fmt=args.output_format,
        tmsh_verb="create",
        transaction=getattr(args, "output_transaction", False),
    )
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 0
