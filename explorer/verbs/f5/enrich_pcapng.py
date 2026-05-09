"""``f5 enrich-pcapng`` — annotate IPs in a capture with BIG-IP object names."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.parser import parse_bigip_conf
from core.bigip.pcap_enrich import build_merged_name_index, enrich_capture_file

from ._registry import verb


@verb(
    "enrich-pcapng",
    aliases=("enrich",),
    help=(
        "Inject a Name Resolution Block (and optional TLS keylog DSB) into a "
        "PCAPNG so Wireshark labels each BIG-IP IP with its object name."
    ),
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Read one or more bigip.conf / SCF files (LTM and GTM tiers can "
        "live in separate inputs — wide-IP / pool / server references "
        "resolve across the combined input), derive a hostname-style "
        "label for every IP that appears in the capture (virtual-server "
        "destination, pool member, SNAT-pool member, self-IP, node, "
        "wide-IP, attached iRule, or membership of a self-IP CIDR), and "
        "inject those mappings into the capture as a PCAPNG Name "
        "Resolution Block.  Wireshark then renders the labels alongside "
        "addresses, just like reverse DNS would.\n"
        "\n"
        "Labels are ``purpose-objname`` lowercased with ``-`` separators: "
        "``vs-common-vs1``, ``pool-common-web1-80``, "
        "``snat-common-my-snatpool``, ``self-common-external-self``, "
        "``net-common-external-self`` (subnet match), "
        "``node-common-web1``, ``irule-common-my-irule``, "
        "``wideip-common-example-com``.\n"
        "\n"
        "By default the NRB only carries records for IPs that actually "
        "appear in the capture; pass ``--all`` to emit every label from "
        "the inventory regardless of whether it is observed.\n"
        "\n"
        "Optionally, ``--keylog FILE`` injects a TLS NSS-format keylog "
        "as a Decryption Secrets Block so Wireshark can decrypt TLS "
        "sessions without an external SSLKEYLOGFILE.\n"
        "\n"
        "Plain libpcap input is auto-converted to PCAPNG via ``editcap`` "
        "or ``tshark`` (whichever is on PATH)."
    )
    p.add_argument(
        "-c",
        "--config",
        action="append",
        required=True,
        metavar="FILE",
        dest="configs",
        help=(
            "bigip.conf / SCF file with the object inventory (repeatable). "
            "Pass `-` to read one config from stdin.  Use multiple ``-c`` "
            "flags to combine GTM + LTM inputs or multi-tier deployments."
        ),
    )
    p.add_argument("input", help="Input capture (PCAPNG; libpcap is auto-converted).")
    p.add_argument("output", help="Output PCAPNG file (always pcapng).")
    p.add_argument(
        "--keylog",
        metavar="FILE",
        help=(
            "NSS-format TLS keylog file to embed as a Decryption Secrets "
            "Block (Wireshark decrypts TLS using these keys without "
            "needing SSLKEYLOGFILE)."
        ),
    )
    p.add_argument(
        "--all",
        dest="include_unobserved",
        action="store_true",
        help=(
            "Emit an NRB record for every label in the inventory, even "
            "addresses that never appear as a packet src/dst.  Default: "
            "filter to addresses observed in the capture."
        ),
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the address→label mapping that would be injected and exit.",
    )
    p.set_defaults(handler=_run_enrich_pcapng)


def _read_config_text(path_str: str) -> str:
    if path_str == "-":
        return sys.stdin.read()
    return Path(path_str).read_text(encoding="utf-8", errors="replace")


def _load_configs(paths: list[str]):  # -> list[tuple[BigipConfig, str]]
    pairs = []
    saw_stdin = False
    for path_str in paths:
        if path_str == "-":
            if saw_stdin:
                raise ValueError("`-` (stdin) can only be supplied once")
            saw_stdin = True
        text = _read_config_text(path_str)
        try:
            config = parse_bigip_conf(text)
        except Exception as exc:  # noqa: BLE001 — surface the parser error verbatim.
            raise ValueError(f"cannot parse {path_str}: {exc}") from exc
        pairs.append((config, text))
    return pairs


def _run_enrich_pcapng(args: argparse.Namespace) -> int:
    try:
        configs_with_sources = _load_configs(args.configs)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    name_index = build_merged_name_index(configs_with_sources)

    if args.dry_run:
        for addr, names in sorted(name_index.v4.items()):
            for name in names:
                print(f"{addr}\t{name}")
        for addr, names in sorted(name_index.v6.items()):
            for name in names:
                print(f"{addr}\t{name}")
        for net, name in name_index.v4_subnets:
            print(f"{net}\t{name}")
        for net, name in name_index.v6_subnets:
            print(f"{net}\t{name}")
        print(
            f"enrich-pcapng (dry-run): {len(name_index.v4)} v4 / "
            f"{len(name_index.v6)} v6 address(es), "
            f"{len(name_index.v4_subnets) + len(name_index.v6_subnets)} subnet(s), "
            f"{name_index.total_names()} label(s)",
            file=sys.stderr,
        )
        return 0

    keylog_text: str | None = None
    if args.keylog:
        try:
            keylog_text = Path(args.keylog).read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            print(f"error: cannot read keylog: {exc}", file=sys.stderr)
            return 2

    in_path = Path(args.input)
    out_path = Path(args.output)
    if not in_path.is_file():
        print(f"error: not a file: {args.input}", file=sys.stderr)
        return 2

    try:
        result = enrich_capture_file(
            in_path,
            out_path,
            name_index,
            keylog_text=keylog_text,
            include_unobserved=args.include_unobserved,
        )
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        try:
            out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return 2

    converted_note = " (libpcap input converted to pcapng)" if result.converted_from_libpcap else ""
    keylog_note = (
        f", keylog {result.keylog_bytes} byte(s) embedded" if result.keylog_bytes else ""
    )
    observed_note = (
        f" out of {result.observed_ipv4} v4 / {result.observed_ipv6} v6 observed"
        if not args.include_unobserved
        else " (--all: every inventory entry)"
    )
    print(
        f"enrich-pcapng: {result.ipv4_records} v4 / {result.ipv6_records} v6 record(s), "
        f"{result.names_total} label(s){observed_note}{keylog_note}{converted_note}",
        file=sys.stderr,
    )
    return 0
