"""``f5 enrich-wireshark`` — generate a Wireshark profile directory."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.wireshark_profile import build_wireshark_profile

from ._registry import verb
from .enrich_pcapng import _load_configs


@verb(
    "enrich-wireshark",
    aliases=("ws-profile",),
    help=(
        "Generate a Wireshark profile directory (hosts / subnets / vlans / "
        "dfilters) from one or more bigip.conf / SCF files."
    ),
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.epilog = (
        "Examples:\n"
        "  f5 enrich-wireshark -c bigip.conf -o ws-profile/\n"
        "  f5 enrich-wireshark -c ltm.conf -c gtm.conf -o ./prod-profile/\n"
        "  f5 enrich-wireshark -c bigip.conf -o existing-profile/ --force\n"
    )
    p.description = (
        "Walk every BIG-IP object across one or more bigip.conf / SCF "
        "inputs (LTM and GTM tiers can live in separate files; references "
        "resolve across the combined input) and emit a Wireshark profile "
        "directory with these files:\n"
        "\n"
        "- ``hosts``     — IP → ``vs-…``, ``pool-…``, ``snat-…``, "
        "``self-…``, ``node-…``, ``irule-…``, ``wideip-…`` labels.\n"
        "- ``subnets``   — self-IP CIDR → ``net-…`` labels (Wireshark "
        "applies these to any host inside the subnet).\n"
        "- ``vlans``     — ``net vlan { tag N }`` → ``vlan-…``.\n"
        "- ``dfilters``  — saved display-filter buttons, one per "
        "virtual-server (``ip.addr == <dest>``) and per self-IP CIDR.\n"
        "- ``README.md`` — install instructions.\n"
        "\n"
        "Drop the resulting directory into your Wireshark profiles folder "
        "(`~/.config/wireshark/profiles/<name>/` on Linux/macOS, or "
        "`%APPDATA%\\Wireshark\\profiles\\<name>\\` on Windows) and "
        "select it from **Edit → Configuration Profiles**.  The same "
        "labels then show up regardless of capture format — including "
        "plain libpcap, where a PCAPNG NRB can't reach."
    )
    p.add_argument(
        "-c",
        "--config",
        action="append",
        required=True,
        metavar="FILE",
        dest="configs",
        help=(
            "bigip.conf / SCF file (repeatable).  Pass `-` to read one "
            "config from stdin.  Use multiple ``-c`` flags to combine "
            "GTM + LTM inputs or multi-tier deployments."
        ),
    )
    p.add_argument(
        "output",
        help="Output directory for the generated profile (created if missing).",
    )
    p.add_argument(
        "--force",
        action="store_true",
        help="Overwrite files in *output* even if it already contains a profile.",
    )
    p.set_defaults(handler=_run_enrich_wireshark)


def _run_enrich_wireshark(args: argparse.Namespace) -> int:
    try:
        configs_with_sources = _load_configs(args.configs)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    out_dir = Path(args.output)
    if out_dir.exists() and not out_dir.is_dir():
        print(f"error: not a directory: {args.output}", file=sys.stderr)
        return 2

    if out_dir.exists() and any(out_dir.iterdir()) and not args.force:
        print(
            f"error: {args.output} already exists and is not empty; pass --force to overwrite",
            file=sys.stderr,
        )
        return 2

    profile = build_wireshark_profile(configs_with_sources)

    try:
        result = profile.write_to(out_dir)
    except OSError as exc:
        print(f"error: cannot write profile: {exc}", file=sys.stderr)
        return 2

    print(
        f"enrich-wireshark: {result.hosts_lines} host(s), "
        f"{result.subnets_lines} subnet(s), "
        f"{result.vlans_lines} vlan(s), "
        f"{result.dfilters_lines} display-filter(s), "
        f"{result.services_lines} service(s), "
        f"{result.ethers_lines} ether(s), "
        f"{result.colorfilter_rules} colour rule(s) written to "
        f"{out_dir}",
        file=sys.stderr,
    )
    return 0
