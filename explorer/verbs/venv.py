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


def _run_list(args: argparse.Namespace) -> int:
    """List discoverable virtual environments."""
    found: list[dict[str, str]] = []

    # Check CWD and common locations.
    candidates = [Path.cwd() / ".venv", Path.cwd() / "venv"]
    home_pool = Path.home() / ".tcl-venvs"
    if home_pool.is_dir():
        candidates.extend(sorted(home_pool.iterdir()))

    for candidate in candidates:
        cfg = candidate / "tclvenv.cfg"
        if cfg.is_file():
            from tclpkg.venv import read_venv_config

            try:
                config = read_venv_config(candidate)
            except Exception:
                config = {}
            import os

            active = os.environ.get("TCL_VENV", "")
            is_active = bool(active) and Path(active).resolve() == candidate.resolve()
            found.append(
                {
                    "path": str(candidate),
                    "tcl_version": config.get("tcl_version", "?"),
                    "prompt": config.get("prompt", candidate.name),
                    "active": "yes" if is_active else "",
                }
            )

    if getattr(args, "json", False):
        ui.json_output(found)
    else:
        if not found:
            print("  No virtual environments found.")
            return 0
        fmt = "{:<30s} {:<12s} {:<10s} {:<6s}"
        print(fmt.format("PATH", "TCL", "PROMPT", "ACTIVE"))
        for entry in found:
            print(fmt.format(entry["path"], entry["tcl_version"], entry["prompt"], entry["active"]))
    return 0


def _run_update(args: argparse.Namespace) -> int:
    """Re-link a venv to a newer tclsh."""
    from tclpkg.venv import VenvError, read_venv_config

    venv_path = Path(getattr(args, "path", ".venv")).resolve()
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        read_venv_config(venv_path)  # validate the venv exists
    except VenvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    new_tcl = getattr(args, "tcl", None)
    if not new_tcl:
        print("error: --tcl VERSION is required for update", file=sys.stderr)
        return 1

    import shutil
    import stat

    tclsh_name = f"tclsh{new_tcl}"
    tclsh_path = shutil.which(tclsh_name) or shutil.which("tclsh")
    if not tclsh_path:
        print(f"error: {tclsh_name} not found on PATH", file=sys.stderr)
        return 1

    # Update tclvenv.cfg.
    cfg_file = venv_path / "tclvenv.cfg"
    lines = cfg_file.read_text(encoding="utf-8").splitlines()
    new_lines = []
    for line in lines:
        if line.startswith("tcl_version"):
            new_lines.append(f"tcl_version = {new_tcl}")
        elif line.startswith("tcl_executable"):
            new_lines.append(f"tcl_executable = {tclsh_path}")
        else:
            new_lines.append(line)
    cfg_file.write_text("\n".join(new_lines) + "\n", encoding="utf-8")

    # Rewrite tclsh wrapper.
    wrapper = venv_path / "bin" / "tclsh"
    import textwrap

    script = textwrap.dedent(f"""\
        #!/bin/sh
        # Auto-generated by tclpkg — do not edit.
        VENV="$(cd "$(dirname "$0")/.." && pwd)"
        : "${{TCLLIBPATH:=}}"
        export TCLLIBPATH="$VENV/lib${{TCLLIBPATH:+ $TCLLIBPATH}}"
        export TCL_VENV="$VENV"
        exec "{tclsh_path}" "$@"
    """)
    wrapper.write_text(script, encoding="utf-8")
    wrapper.chmod(wrapper.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    if getattr(args, "json", False):
        ui.json_output({"path": str(venv_path), "tcl": new_tcl, "tclsh": tclsh_path})
    else:
        print(ui.ok(f"updated {venv_path} to tclsh {new_tcl} ({tclsh_path})", colour=colour))
    return 0


def _run_deactivate(args: argparse.Namespace) -> int:
    """Print deactivation snippet to stdout."""
    shell = getattr(args, "shell", None)
    if shell == "fish":
        print("deactivate")
    else:
        print("deactivate")
    return 0


def _run_venv_run(args: argparse.Namespace) -> int:
    """Run a command inside a venv without manual activation."""
    import os
    import subprocess

    venv_path = Path(getattr(args, "path", ".venv")).resolve()
    cfg = venv_path / "tclvenv.cfg"
    if not cfg.is_file():
        print(f"error: not a tclpkg venv: {venv_path}", file=sys.stderr)
        return 1

    command = getattr(args, "command", [])
    if not command:
        print("error: no command specified after --", file=sys.stderr)
        return 1

    env = os.environ.copy()
    env["TCL_VENV"] = str(venv_path)
    lib_dir = str(venv_path / "lib")
    existing = env.get("TCLLIBPATH", "")
    env["TCLLIBPATH"] = f"{lib_dir}{' ' + existing if existing else ''}"
    env["PATH"] = f"{venv_path / 'bin'}{os.pathsep}{env.get('PATH', '')}"

    result = subprocess.run(command, env=env)
    return result.returncode


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

    # deactivate
    deact_p = venv_sub.add_parser("deactivate", help="Print deactivation script to stdout.")
    deact_p.add_argument(
        "--shell",
        choices=("bash", "zsh", "fish", "csh", "powershell"),
        default=None,
        help="Shell flavour (default: auto-detect).",
    )
    deact_p.set_defaults(handler=_run_deactivate)

    # list
    list_p = venv_sub.add_parser("list", help="List discoverable virtual environments.")
    list_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    list_p.set_defaults(handler=_run_list)

    # update
    upd_p = venv_sub.add_parser("update", help="Re-link a venv to a newer tclsh.")
    upd_p.add_argument("path", nargs="?", default=".venv", help="Venv directory (default .venv).")
    upd_p.add_argument("--tcl", required=True, help="New Tcl version to pin.")
    upd_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    upd_p.set_defaults(handler=_run_update)

    # run
    run_p = venv_sub.add_parser("run", help="Run a command inside a venv.")
    run_p.add_argument("path", nargs="?", default=".venv", help="Venv directory (default .venv).")
    run_p.add_argument("command", nargs=argparse.REMAINDER, help="Command to run (after --).")
    run_p.set_defaults(handler=_run_venv_run)
