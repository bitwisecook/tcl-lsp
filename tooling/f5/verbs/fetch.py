"""``f5 fetch`` — pull SCF/UCS from a live BIG-IP device.

Two transports are supported:

- ``rest`` — iControl REST over HTTPS (stdlib ``http.client``).
- ``ssh``  — system ``ssh`` + ``scp`` (no paramiko dependency).
- ``auto`` — try REST first, fall back to SSH on connection error.

Credentials resolve in this order: CLI flag > env var > XDG config
file (``~/.config/f5/hosts.toml``) > interactive prompt.

By default the result is cached under ``$XDG_CACHE_HOME/f5/<host>/<UTC-timestamp>/``
and a stable ``latest`` symlink in the same parent points at it.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tooling.f5.f5_remote import FetchResult, fetch_scf
from tooling.f5.f5_remote.auth import Credentials, resolve_credentials, xdg_cache_dir

from ._registry import verb


@verb(
    "fetch",
    aliases=("get",),
    help="Pull SCF or UCS from a live BIG-IP device (REST or SSH).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Connect to a BIG-IP, save its running config, and download\n"
        "either an SCF text file, a UCS archive, or both.  UCS inputs\n"
        "are converted to SCF on the fly so downstream verbs always\n"
        "have a usable text artefact.  Credentials may come from --user\n"
        "/ --password, F5_USER / F5_PASSWORD env vars, or an interactive\n"
        "prompt.  Use --transport auto (default) to try REST first and\n"
        "fall back to SSH if the iControl endpoint isn't available."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 fetch --host bigip.example.com\n"
        "  F5_USER=admin F5_PASSWORD=... f5 fetch --host 10.0.0.1\n"
        "  f5 fetch --host bigip --format ucs -o backups/\n"
        "  f5 fetch --host bigip --format both --transport ssh\n"
        "  f5 fetch --host bigip -o - > running.scf\n"
    )
    p.add_argument("--host", help="Device hostname / IP / alias (or set F5_HOST).")
    p.add_argument("--user", help="Username (or set F5_USER).")
    p.add_argument(
        "--password",
        help="Password (or set F5_PASSWORD).  Prompted interactively if absent.",
    )
    p.add_argument("--port", type=int, help="iControl REST HTTPS port (default 443).")
    p.add_argument("--ssh-port", type=int, help="SSH port for SSH transport (default 22).")
    p.add_argument(
        "--transport",
        choices=("auto", "rest", "ssh"),
        default="auto",
        help="Transport: REST, SSH, or auto-fallback (default).",
    )
    p.add_argument(
        "--format",
        choices=("scf", "ucs", "both"),
        default="scf",
        dest="fmt",
        help="What to download (default: scf).",
    )
    p.add_argument(
        "--insecure",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Skip TLS certificate verification (default: yes — BIG-IP self-signs).",
    )
    p.add_argument(
        "--timeout",
        type=float,
        default=120.0,
        help="Per-request timeout in seconds (default: 120).",
    )
    p.add_argument(
        "-o",
        "--output",
        metavar="DIR",
        help=(
            "Write artefacts to DIR.  Default: $XDG_CACHE_HOME/f5/<host>/<timestamp>/. "
            "Pass `-` to stream the SCF to stdout (UCS is dropped)."
        ),
    )
    p.add_argument(
        "--no-prompt",
        action="store_true",
        help="Fail rather than prompt for missing credentials.",
    )
    p.add_argument(
        "--print-path",
        action="store_true",
        help="Print the cache directory path on stdout after a successful fetch.",
    )
    p.set_defaults(handler=_run_fetch)


def _run_fetch(args: argparse.Namespace) -> int:
    try:
        credentials: Credentials = resolve_credentials(
            host=args.host,
            user=args.user,
            password=args.password,
            port=args.port,
            ssh_port=args.ssh_port,
            interactive=not args.no_prompt,
        )
    except (ValueError, OSError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        result = fetch_scf(
            credentials=credentials,
            transport=args.transport,
            fmt=args.fmt,
            insecure=args.insecure,
            timeout=args.timeout,
        )
    except Exception as exc:  # noqa: BLE001 — surface any transport error uniformly
        print(f"error: fetch failed: {exc}", file=sys.stderr)
        return 2

    if args.output == "-":
        sys.stdout.write(result.scf)
        return 0

    out_dir = _resolve_output_dir(args.output, result)
    _write_result(result, out_dir)
    if args.print_path:
        print(str(out_dir))
    else:
        print(
            f"fetched {result.fmt} from {result.host} via {result.transport} -> {out_dir}",
            file=sys.stderr,
        )
    return 0


def _resolve_output_dir(explicit: str | None, result: FetchResult) -> Path:
    if explicit:
        out = Path(explicit)
        out.mkdir(parents=True, exist_ok=True)
        return out
    timestamp = result.fetched_at.strftime("%Y%m%dT%H%M%SZ")
    base = xdg_cache_dir() / _safe_host_dir(result.host)
    out = base / timestamp
    out.mkdir(parents=True, exist_ok=True)
    _update_latest_symlink(base / "latest", out)
    return out


def _safe_host_dir(host: str) -> str:
    return "".join(c if c.isalnum() or c in "._-" else "_" for c in host)


def _update_latest_symlink(link: Path, target: Path) -> None:
    try:
        if link.is_symlink() or link.exists():
            link.unlink()
        link.symlink_to(target.name)
    except OSError:
        # Filesystems without symlink support (rare) — silently skip.
        pass


def _write_result(result: FetchResult, out_dir: Path) -> None:
    (out_dir / "config.scf").write_text(result.scf, encoding="utf-8")
    if result.ucs is not None:
        (out_dir / "config.ucs").write_bytes(result.ucs)
    meta = (
        f"host = {result.host}\n"
        f"transport = {result.transport}\n"
        f"format = {result.fmt}\n"
        f"fetched_at = {result.fetched_at.isoformat()}\n"
    )
    (out_dir / "fetch.meta").write_text(meta, encoding="utf-8")
