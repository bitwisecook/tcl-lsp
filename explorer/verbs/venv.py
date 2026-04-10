"""``tcl venv`` verb group — virtual environment management CLI."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tclpkg import ui


def _run_create(args: argparse.Namespace) -> int:
    from tclpkg.venv import VenvError, create_venv

    venv_path = Path(getattr(args, "path", ".venv"))
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        created = create_venv(
            venv_path,
            tcl_version=getattr(args, "tcl", None),
            system_site_packages=getattr(args, "system_site_packages", False),
            prompt=getattr(args, "prompt", None),
        )
    except VenvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({"path": str(created)})
    else:
        print(ui.ok(f"created {created}", colour=colour))
        print(ui.dim(f"  activate: source {created}/bin/activate", colour=colour))
    return 0


def _run_delete(args: argparse.Namespace) -> int:
    from tclpkg.venv import VenvError, delete_venv

    venv_path = Path(getattr(args, "path", ".venv"))
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        delete_venv(venv_path)
    except VenvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({"deleted": str(venv_path)})
    else:
        print(ui.ok(f"deleted {venv_path}", colour=colour))
    return 0


def _run_info(args: argparse.Namespace) -> int:
    from tclpkg.venv import VenvError, read_venv_config

    venv_path = Path(getattr(args, "path", ".venv"))

    try:
        config = read_venv_config(venv_path)
    except VenvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output(config)
    else:
        for key, value in sorted(config.items()):
            print(f"  {key:30s}  {value}")
    return 0


def _run_activate(args: argparse.Namespace) -> int:
    """Print activation snippet to stdout (user evals it)."""
    venv_path = Path(getattr(args, "path", ".venv")).resolve()
    shell = getattr(args, "shell", None)

    if shell == "fish":
        script_path = venv_path / "bin" / "activate.fish"
    else:
        script_path = venv_path / "bin" / "activate"

    if not script_path.is_file():
        print(f"error: activation script not found: {script_path}", file=sys.stderr)
        return 1

    print(script_path.read_text(encoding="utf-8"))
    return 0


def add_venv_subparser(
    sub: argparse._SubParsersAction,
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> None:
    """Register the ``venv`` verb group and all of its sub-subparsers."""
    venv_p = sub.add_parser(
        "venv",
        help="Manage Tcl virtual environments.",
        description="Create, activate, and manage Tcl virtual environments.",
    )
    venv_sub = venv_p.add_subparsers(dest="venv_action", required=True)

    # create
    create_p = venv_sub.add_parser("create", help="Create a new virtual environment.")
    create_p.add_argument(
        "path", nargs="?", default=".venv", help="Venv directory (default .venv)."
    )
    create_p.add_argument("--tcl", help="Pin a specific Tcl version (e.g. 8.6, 9.0).")
    create_p.add_argument(
        "--system-site-packages",
        action="store_true",
        help="Allow fallback to host auto_path.",
    )
    create_p.add_argument("--prompt", help="Custom shell prompt label.")
    create_p.add_argument("--force", action="store_true", help="Overwrite existing directory.")
    create_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    create_p.set_defaults(handler=_run_create)

    # delete
    delete_p = venv_sub.add_parser("delete", help="Remove a virtual environment.")
    delete_p.add_argument(
        "path", nargs="?", default=".venv", help="Venv directory (default .venv)."
    )
    delete_p.add_argument("--force", action="store_true", help="Force deletion even if active.")
    delete_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    delete_p.set_defaults(handler=_run_delete)

    # info
    info_p = venv_sub.add_parser("info", help="Show virtual environment details.")
    info_p.add_argument("path", nargs="?", default=".venv", help="Venv directory (default .venv).")
    info_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    info_p.set_defaults(handler=_run_info)

    # activate
    act_p = venv_sub.add_parser("activate", help="Print activation script to stdout.")
    act_p.add_argument("path", nargs="?", default=".venv", help="Venv directory (default .venv).")
    act_p.add_argument(
        "--shell",
        choices=("bash", "zsh", "fish", "csh", "powershell"),
        default=None,
        help="Shell flavour (default: auto-detect).",
    )
    act_p.set_defaults(handler=_run_activate)
