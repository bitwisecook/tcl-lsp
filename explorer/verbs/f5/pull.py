"""``f5 pull`` — fetch one object from a live device as an SCF stanza."""

from __future__ import annotations

import argparse
import json
import sys

from explorer.f5_remote.auth import resolve_credentials
from explorer.f5_remote.object_io import object_to_scf_stanza, pull_object

from ._emit import add_format_arg, render_config
from ._registry import verb


@verb(
    "pull",
    aliases=(),
    help="Fetch a single object (virtual/pool/node/rule) from a live BIG-IP.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "GET one object via iControl REST and emit its SCF stanza on "
        "stdout.  Useful for surgical edits: pull, edit locally, push "
        "back.  Pass --json to get the raw iControl JSON instead."
    )
    p.add_argument(
        "kind",
        choices=("virtual", "pool", "node", "rule"),
        help="Object kind to fetch.",
    )
    p.add_argument("full_path", help="Full path of the object (e.g. /Common/vs_app).")
    p.add_argument("--host", help="Device hostname / IP / alias.")
    p.add_argument("--user", help="Username.")
    p.add_argument("--password", help="Password.")
    p.add_argument("--port", type=int, help="HTTPS port (default 443).")
    p.add_argument("--no-prompt", action="store_true", help="Fail rather than prompt.")
    p.add_argument(
        "--insecure",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Skip TLS certificate verification (default: yes).",
    )
    p.add_argument("--json", action="store_true", help="Emit raw iControl REST JSON.")
    p.add_argument(
        "--timeout", type=float, default=60.0, help="Per-request timeout (default: 60s)."
    )
    add_format_arg(p, tmsh_default_verb="create")
    p.set_defaults(handler=_run_pull)


def _run_pull(args: argparse.Namespace) -> int:
    try:
        creds = resolve_credentials(
            host=args.host,
            user=args.user,
            password=args.password,
            port=args.port,
            interactive=not args.no_prompt,
        )
    except (ValueError, OSError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        obj = pull_object(
            creds,
            kind=args.kind,
            full_path=args.full_path,
            insecure=args.insecure,
            timeout=args.timeout,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"error: pull failed: {exc}", file=sys.stderr)
        return 2

    if args.json:
        sys.stdout.write(json.dumps(obj, indent=2) + "\n")
    else:
        scf = object_to_scf_stanza(args.kind, obj)
        sys.stdout.write(render_config(scf, fmt=args.output_format, tmsh_verb="create"))
    return 0
