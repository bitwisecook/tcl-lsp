"""``tcl docker`` verb group — Dockerfile generation for Tcl projects.

Registers the ``docker`` subparser and its sub-subcommands (``create``,
``recipe``, ``info``) onto the main CLI parser.

The heavy lifting lives in ``tclpkg.docker`` — handlers here are thin
wrappers that parse CLI args, call the library, and format output.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tclpkg import ui


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_create(args: argparse.Namespace) -> int:
    """Generate a Dockerfile for a Tcl project."""
    from tclpkg.docker import DockerError, DockerfileSpec, write_dockerfile

    colour = ui.use_colour(force=not getattr(args, "json", False))
    output = Path(getattr(args, "output", "Dockerfile"))

    spec = DockerfileSpec(
        base_image=args.image,
        tcl_version=getattr(args, "tcl_version", "8.6"),
        workdir=getattr(args, "workdir", "/app"),
        copy_project=not getattr(args, "no_copy", False),
        install_packages=not getattr(args, "no_packages", False),
        create_venv=getattr(args, "venv", False),
        entrypoint=getattr(args, "entrypoint", None),
        cli_version=getattr(args, "cli_version", None),
    )

    # Parse --label key=value pairs.
    for label in getattr(args, "label", None) or []:
        if "=" not in label:
            print(f"error: --label must be key=value, got: {label}", file=sys.stderr)
            return 1
        key, _, value = label.partition("=")
        spec.labels[key] = value

    # Parse --env key=value pairs.
    for env_pair in getattr(args, "env", None) or []:
        if "=" not in env_pair:
            print(f"error: --env must be key=value, got: {env_pair}", file=sys.stderr)
            return 1
        key, _, value = env_pair.partition("=")
        spec.env[key] = value

    # Parse --extra-package additions.
    spec.extra_packages = getattr(args, "extra_package", None) or []

    force = getattr(args, "force", False)

    try:
        written = write_dockerfile(output, spec, overwrite=force)
    except DockerError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        from tclpkg.docker import generate_dockerfile

        ui.json_output({
            "path": str(written),
            "base_image": spec.base_image,
            "tcl_version": spec.tcl_version,
            "content": generate_dockerfile(spec),
        })
    else:
        print(ui.ok(f"wrote {written}", colour=colour))
        print(
            ui.dim(
                f"  base: {spec.base_image}  tcl: {spec.tcl_version}",
                colour=colour,
            )
        )
        print(ui.dim(f"  docker build -t myapp .", colour=colour))
    return 0


def _run_recipe(args: argparse.Namespace) -> int:
    """Show the Tcl install recipe for a given image and version."""
    from tclpkg.docker import DockerError, tcl_install_recipe

    try:
        recipe = tcl_install_recipe(args.image, args.tcl_version)
    except DockerError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({
            "image": args.image,
            "tcl_version": args.tcl_version,
            "recipe": recipe,
        })
    else:
        print(recipe)
    return 0


def _run_info(args: argparse.Namespace) -> int:
    """Show available base-image families and Tcl versions."""
    from tclpkg.docker import SUPPORTED_TCL_VERSIONS, available_recipes

    recipes = available_recipes()

    if getattr(args, "json", False):
        ui.json_output({
            "supported_tcl_versions": list(SUPPORTED_TCL_VERSIONS),
            "families": recipes,
        })
    else:
        colour = ui.use_colour()
        print(ui.bold("Supported Tcl versions:", colour=colour))
        for v in SUPPORTED_TCL_VERSIONS:
            print(f"  {v}")
        print()
        print(ui.bold("Install recipe families:", colour=colour))
        for family, versions in recipes.items():
            print(f"  {family:12s}  {', '.join(versions)}")
    return 0


# ---------------------------------------------------------------------------
# Subparser builder
# ---------------------------------------------------------------------------


def add_docker_subparser(
    sub: argparse._SubParsersAction,
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> None:
    """Register the ``docker`` verb group and all of its sub-subparsers."""
    docker_p = sub.add_parser(
        "docker",
        help="Generate Dockerfiles for Tcl projects.",
        description="Generate and manage Dockerfiles for Tcl projects with the right Tcl version.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} docker create debian:bookworm-slim --tcl-version 8.6\n"
            f"  {prog_name} docker create alpine:3.19 --tcl-version 9.0 --venv\n"
            f"  {prog_name} docker recipe ubuntu:22.04 --tcl-version 8.6\n"
            f"  {prog_name} docker info\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    docker_sub = docker_p.add_subparsers(dest="docker_action", required=True)

    # ── create ─────────────────────────────────────────────────────────
    create_p = docker_sub.add_parser(
        "create",
        help="Generate a Dockerfile for a Tcl project.",
        description="Generate a production-ready Dockerfile with Tcl installed.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} docker create debian:bookworm-slim\n"
            f"  {prog_name} docker create alpine:3.19 --tcl-version 9.0\n"
            f"  {prog_name} docker create ubuntu:22.04 --tcl-version 8.6 --entrypoint main.tcl\n"
            f"  {prog_name} docker create fedora:39 --tcl-version 8.6 --venv --extra-package tk\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    create_p.add_argument(
        "image",
        help="Base Docker image (e.g. debian:bookworm-slim, alpine:3.19, ubuntu:22.04).",
    )
    create_p.add_argument(
        "--tcl-version",
        default="8.6",
        choices=("8.4", "8.5", "8.6", "9.0"),
        help="Tcl version to install (default: 8.6).",
    )
    create_p.add_argument(
        "--output", "-o",
        default="Dockerfile",
        help="Output file path (default: Dockerfile).",
    )
    create_p.add_argument(
        "--workdir",
        default="/app",
        help="Container WORKDIR (default: /app).",
    )
    create_p.add_argument(
        "--entrypoint",
        help="Tcl script to run as CMD (e.g. main.tcl).",
    )
    create_p.add_argument(
        "--venv",
        action="store_true",
        help="Create a Tcl virtual environment inside the container.",
    )
    create_p.add_argument(
        "--no-copy",
        action="store_true",
        help="Skip COPY . . (useful for multi-stage builds).",
    )
    create_p.add_argument(
        "--no-packages",
        action="store_true",
        help="Skip tclpkg sync step.",
    )
    create_p.add_argument(
        "--extra-package",
        action="append",
        help="Additional OS package to install (repeatable).",
    )
    create_p.add_argument(
        "--label",
        action="append",
        help="Docker LABEL as key=value (repeatable).",
    )
    create_p.add_argument(
        "--env",
        action="append",
        help="Docker ENV as key=value (repeatable).",
    )
    create_p.add_argument(
        "--cli-version",
        default=None,
        help="tcl CLI zipapp version to download (default: latest known).",
    )
    create_p.add_argument(
        "--force",
        action="store_true",
        help="Overwrite existing Dockerfile.",
    )
    create_p.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON output.",
    )
    create_p.set_defaults(handler=_run_create)

    # ── recipe ─────────────────────────────────────────────────────────
    recipe_p = docker_sub.add_parser(
        "recipe",
        help="Show the Tcl install recipe for a base image.",
        description="Print the Dockerfile snippet that installs Tcl on the given base image.",
    )
    recipe_p.add_argument(
        "image",
        help="Base Docker image (e.g. alpine:3.19, ubuntu:22.04).",
    )
    recipe_p.add_argument(
        "--tcl-version",
        default="8.6",
        choices=("8.4", "8.5", "8.6", "9.0"),
        help="Tcl version (default: 8.6).",
    )
    recipe_p.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON output.",
    )
    recipe_p.set_defaults(handler=_run_recipe)

    # ── info ───────────────────────────────────────────────────────────
    info_p = docker_sub.add_parser(
        "info",
        help="Show available base-image families and Tcl versions.",
    )
    info_p.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON output.",
    )
    info_p.set_defaults(handler=_run_info)
