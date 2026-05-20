"""``f5 redact`` — strip secrets and remap real IPs for sharing."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.redact_map import (
    DEFAULT_TARGET_CIDRS_V4,
    DEFAULT_TARGET_CIDRS_V6,
    RedactionMap,
)
from dialects.f5.bigip.rewrite import redact_secrets

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb


@verb(
    "redact",
    aliases=("sanitize",),
    help="Strip passwords/PEM blocks and remap public IPs into a configurable CIDR pool.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.epilog = (
        "Examples:\n"
        "  f5 redact bigip.conf -o sanitised.conf --map-file map.toml\n"
        "  f5 redact bigip.conf --keep-ips -o secrets-only.conf\n"
        "  f5 redact bigip.conf --target-cidr 10.0.0.0/8 --target-cidr fd00::/8\n"
        "  f5 redact bigip.conf --source-cidr 1.2.3.0/24 --remap-private\n"
        "  f5 redact bigip.conf --shuffle --seed 42 --map-file map.toml\n"
    )
    p.description = (
        "Produce a copy of the bigip.conf / SCF safe to share externally: "
        "strip values from secret-bearing keys (passphrase, password, "
        "secret, community, encrypted-password, ...), replace PEM-encoded "
        "certs/keys with `<REDACTED>`, and consistently remap any public "
        "IPv4/IPv6 addresses into a configurable target CIDR pool (RFC1918 "
        "by default).  CIDR relationships are preserved: two real IPs "
        "sharing a /24 always land in the same target /24, so subnetting "
        "in the redacted output mirrors the original.  A sidecar map file "
        "records every assignment so `f5 unredact` can recover the "
        "originals."
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument(
        "-o", "--output", metavar="FILE", help="Write redacted config here (default: stdout)."
    )
    p.add_argument(
        "--keep-ips",
        action="store_true",
        help="Preserve original IP addresses (only redact secrets and PEM blocks).",
    )
    p.add_argument(
        "--target-cidr",
        action="append",
        default=[],
        metavar="CIDR",
        help=(
            "Target CIDR for remapped addresses (repeatable; mix v4 and v6).  "
            "Defaults to RFC1918 (10/8, 172.16/12, 192.168/16) plus fd00::/8."
        ),
    )
    p.add_argument(
        "--shuffle",
        action="store_true",
        help=(
            "Permute host bits within each source CIDR via a deterministic "
            "key (default: identity within target CIDR).  Hides host-bit "
            "patterns; same input always yields the same output given the "
            "same --seed."
        ),
    )
    p.add_argument(
        "--remap-private",
        action="store_true",
        help=(
            "Also remap RFC1918 / fc00::/7 (private) addresses.  By default "
            "those are left alone.  Loopback / link-local / multicast / "
            "unspecified are always preserved (they have protocol semantics).  "
            "Source CIDRs are guaranteed not to overlap target CIDRs even "
            "when both pools are RFC1918, so source 10.0.0.0/24 is never "
            "allocated 10.0.0.0/24 as its target."
        ),
    )
    p.add_argument(
        "--seed",
        default="",
        help="Seed for --shuffle (deterministic; recorded in the map file).",
    )
    p.add_argument(
        "--map-file",
        metavar="FILE",
        help=(
            "Write the redaction map TOML here.  Defaults to "
            "<output>.redact.toml when --output is set, otherwise required "
            "for any meaningful round-trip.  If the file already exists, "
            "its assignments are loaded and reused so that IPs from prior "
            "redactions stay mapped to the same redacted addresses — "
            "essential when iterating with F5 support across multiple "
            "redacted captures."
        ),
    )
    p.add_argument(
        "--source-cidr",
        action="append",
        default=[],
        metavar="CIDR",
        help=(
            "Pre-declare a source CIDR (repeatable).  By default we infer "
            "/24 (v4) or /64 (v6) enclosing networks from the input data; "
            "use this flag to force a coarser or finer grouping."
        ),
    )
    add_format_arg(p, tmsh_default_verb="modify")
    p.set_defaults(handler=_run_redact)


def _split_pools(cidrs: list[str]) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Partition CIDRs into v4 and v6 pools (defaults if none supplied)."""
    v4: list[str] = []
    v6: list[str] = []
    for raw in cidrs:
        if ":" in raw:
            v6.append(raw)
        else:
            v4.append(raw)
    return (
        tuple(v4) if v4 else DEFAULT_TARGET_CIDRS_V4,
        tuple(v6) if v6 else DEFAULT_TARGET_CIDRS_V6,
    )


def _run_redact(args: argparse.Namespace) -> int:
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    target_v4, target_v6 = _split_pools(args.target_cidr)

    map_path = _resolve_map_path(args.map_file, args.output)
    existing_map: RedactionMap | None = None
    if map_path is not None and map_path.is_file():
        try:
            existing_map = RedactionMap.from_toml(map_path.read_text(encoding="utf-8"))
        except (ValueError, RuntimeError) as exc:
            print(f"error: cannot load existing map {map_path}: {exc}", file=sys.stderr)
            return 2

    report = redact_secrets(
        source,
        remap_ips=not args.keep_ips,
        target_pool_v4=target_v4,
        target_pool_v6=target_v6,
        mode="shuffle" if args.shuffle else "direct",
        seed=args.seed,
        explicit_source_cidrs=tuple(args.source_cidr),
        existing_map=existing_map,
        remap_private=args.remap_private,
    )

    rendered = render_config(
        report.new_source,
        fmt=args.output_format,
        tmsh_verb="modify",
        transaction=getattr(args, "output_transaction", False),
    )
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)

    if not args.keep_ips and map_path is not None and report.redaction_map is not None:
        map_path.parent.mkdir(parents=True, exist_ok=True)
        map_path.write_text(report.redaction_map.to_toml(), encoding="utf-8")

    print(
        f"redacted: {report.redactions_count} secret(s), "
        f"{report.pem_blocks_replaced} PEM block(s), "
        f"{report.ips_remapped} IP(s) remapped" + (f"; map -> {map_path}" if map_path else ""),
        file=sys.stderr,
    )
    return 0


def _resolve_map_path(explicit: str | None, output: str | None) -> Path | None:
    if explicit:
        return Path(explicit)
    if output:
        return Path(output + ".redact.toml")
    return None
