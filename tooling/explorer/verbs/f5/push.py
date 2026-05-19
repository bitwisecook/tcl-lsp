"""``f5 push`` — send one object to a live device via iControl REST."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from tooling.explorer.f5_remote.auth import resolve_credentials
from tooling.explorer.f5_remote.object_io import push_object

from ._registry import verb


@verb(
    "push",
    aliases=(),
    help="Replace or create a single object on a live BIG-IP via iControl REST.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Send a JSON payload (typically the output of `f5 pull --json`\n"
        "after editing) to a live device.  Defaults to PUT (replace);\n"
        "use --create to POST a brand-new object.  Pair with --dry-run\n"
        "to print the HTTP request without sending it."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 pull virtual /Common/vs --host bigip --json > vs.json\n"
        "  $EDITOR vs.json\n"
        "  f5 push virtual vs.json --host bigip                # replace\n"
        "  f5 push pool new_pool.json --host bigip --create    # create\n"
        "  f5 push virtual vs.json --host bigip --dry-run\n"
    )
    p.add_argument(
        "kind",
        choices=("virtual", "pool", "node", "rule"),
        help="Object kind to push.",
    )
    p.add_argument(
        "payload",
        help="Path to a JSON file (`-` for stdin) holding the iControl REST payload.",
    )
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
    p.add_argument(
        "--create",
        action="store_true",
        help="POST as a new object rather than PUT-replacing an existing one.",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show the payload and target endpoint without sending.",
    )
    p.add_argument(
        "--timeout", type=float, default=60.0, help="Per-request timeout (default: 60s)."
    )
    p.set_defaults(handler=_run_push)


def _run_push(args: argparse.Namespace) -> int:
    try:
        if args.payload == "-":
            raw = sys.stdin.read()
        else:
            raw = Path(args.payload).read_text(encoding="utf-8")
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON payload: {exc}", file=sys.stderr)
        return 2

    if args.dry_run:
        target = payload.get("fullPath") or payload.get("name", "(unnamed)")
        verb_str = "POST (create)" if args.create else "PUT (replace)"
        print(f"would {verb_str} {args.kind} {target}", file=sys.stderr)
        sys.stdout.write(json.dumps(payload, indent=2) + "\n")
        return 0

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
        result = push_object(
            creds,
            kind=args.kind,
            payload=payload,
            create=args.create,
            insecure=args.insecure,
            timeout=args.timeout,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"error: push failed: {exc}", file=sys.stderr)
        return 2

    sys.stdout.write(json.dumps(result, indent=2) + "\n")
    return 0
