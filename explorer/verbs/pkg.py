"""``tcl pkg`` verb group — Tcl package management CLI.

Registers the ``pkg`` subparser and all of its sub-subcommands
(``init``, ``add``, ``install``, ``list``, ``tree``, ``verify``, etc.)
onto the main CLI parser.

The heavy lifting lives in ``tclpkg.*`` modules — handlers here are thin
wrappers that parse CLI args, call the library, and format output.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tclpkg import ui


def _find_project_root(start: Path | None = None) -> Path | None:
    """Walk up from *start* (default CWD) looking for ``tclpkg.tcl``."""
    current = (start or Path.cwd()).resolve()
    for _ in range(20):
        if (current / "tclpkg.tcl").is_file():
            return current
        parent = current.parent
        if parent == current:
            break
        current = parent
    return None


def _manifest_path(args: argparse.Namespace) -> Path:
    override = getattr(args, "manifest", None)
    if override:
        return Path(override)
    root = _find_project_root()
    if root:
        return root / "tclpkg.tcl"
    return Path.cwd() / "tclpkg.tcl"


# Handlers


def _run_init(args: argparse.Namespace) -> int:

    path = Path.cwd() / "tclpkg.tcl"
    if path.exists() and not getattr(args, "force", False):
        print(f"error: {path} already exists (use --force to overwrite)", file=sys.stderr)
        return 1

    name = getattr(args, "name", None) or path.parent.name
    version = getattr(args, "init_version", None) or "0.1.0"
    license_ = getattr(args, "init_license", None) or "MIT"
    tcl = getattr(args, "tcl", None) or ">=8.6"

    lines = [
        f"package     {name}",
        f"version     {version}",
        f"license     {license_}",
        f"tcl         {tcl}",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    colour = ui.use_colour(force=not getattr(args, "json", False))
    if getattr(args, "json", False):
        ui.json_output({"path": str(path), "name": name, "version": version})
    else:
        print(ui.ok(f"wrote {path}", colour=colour))
    return 0


def _run_install(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import LockedPackage, LockFile, SourceSpec, write_lockfile
    from tclpkg.manifest import load_manifest
    from tclpkg.resolver import ExcludeSpec, PackageRef, ReplaceSpec, resolve

    mpath = _manifest_path(args)
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        manifest = load_manifest(mpath)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    # Build resolver inputs from manifest.
    direct = [PackageRef(name=r.name, version=r.minimum) for r in manifest.requires]
    dev_direct = [PackageRef(name=r.name, version=r.minimum) for r in manifest.dev_requires]
    replaces = [ReplaceSpec(name=r.name, version=r.version) for r in manifest.replaces]
    excludes = [ExcludeSpec(name=e.name, version=e.version) for e in manifest.excludes]

    include_dev = not getattr(args, "no_dev", False)

    try:
        resolved = resolve(
            direct,
            dev_direct,
            replaces=replaces,
            excludes=excludes,
            include_dev=include_dev,
        )
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    # Build the lockfile.
    lf = LockFile(name=manifest.name, tcl=manifest.tcl_constraint)
    lf.stamp()
    for rp in resolved:
        lf.packages.append(
            LockedPackage(
                name=rp.ref.name,
                version=str(rp.ref.version),
                source=SourceSpec(type="tarball", url=""),
                integrity="",
                dev=rp.dev,
                requires=[str(r) for r in rp.requires],
            )
        )

    lockfile_path = mpath.parent / "tclpkg.lock"
    write_lockfile(lf, lockfile_path)

    if getattr(args, "json", False):
        ui.json_output({"packages": len(lf.packages), "lockfile": str(lockfile_path)})
    else:
        for pkg in lf.packages:
            dev_tag = " (dev)" if pkg.dev else ""
            print(ui.ok(f"{pkg.name:20s} {pkg.version}{dev_tag}", colour=colour))
        print(ui.ok(f"wrote {lockfile_path}", colour=colour))

    return 0


def _run_list(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({"packages": [p.to_dict() for p in lf.packages]})
    else:
        fmt = "{:<20s} {:<12s} {:<8s} {:<8s}"
        print(fmt.format("NAME", "VERSION", "KIND", "DEV"))
        for pkg in sorted(lf.packages, key=lambda p: p.name):
            kind = "direct" if not pkg.requires else "trans"
            dev = "dev" if pkg.dev else ""
            print(fmt.format(pkg.name, pkg.version, kind, dev))

    return 0


def _run_verify(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    failures = 0
    for pkg in lf.packages:
        if pkg.integrity:
            print(ui.ok(f"{pkg.name:20s} {pkg.version:12s} {pkg.integrity[:30]}…", colour=colour))
        else:
            print(ui.warn(f"{pkg.name:20s} {pkg.version:12s} no integrity hash", colour=colour))
            failures += 1

    if failures:
        print(
            f"\n{failures} package(s) have no integrity hash — run 'tcl pkg install' to populate.",
            file=sys.stderr,
        )
        return 1
    return 0


def _run_tree(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({"name": lf.name, "packages": [p.to_dict() for p in lf.packages]})
    else:
        print(f"{lf.name}")
        for i, pkg in enumerate(sorted(lf.packages, key=lambda p: p.name)):
            connector = "└── " if i == len(lf.packages) - 1 else "├── "
            dev_tag = " [dev]" if pkg.dev else ""
            print(f"{connector}{pkg.name} {pkg.version}{dev_tag}")

    return 0


def _run_info(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"
    pkg_name = args.package

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    entry = lf.lookup(pkg_name)
    if entry is None:
        print(f"error: package '{pkg_name}' not found in lockfile", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output(entry.to_dict())
    else:
        print(f"Name:      {entry.name}")
        print(f"Version:   {entry.version}")
        print(f"Source:    {entry.source.type} {entry.source.url}")
        print(f"Integrity: {entry.integrity or '(not computed)'}")
        print(f"Licence:   {entry.license or '(unknown)'}")
        print(f"Dev:       {'yes' if entry.dev else 'no'}")
        if entry.requires:
            print(f"Requires:  {', '.join(entry.requires)}")
        if entry.provides:
            print(f"Provides:  {', '.join(entry.provides)}")

    return 0


def _run_search(args: argparse.Namespace) -> int:
    from core.common.user_config import _cache_dir
    from tclpkg.registry import RegistryClient

    client = RegistryClient(
        _cache_dir(),
        offline=getattr(args, "offline", False),
    )
    try:
        results = client.search(args.query)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output([{"name": e.name, "description": e.description} for e in results])
    else:
        if not results:
            print("No matches found.")
            return 0
        for entry in results:
            print(f"  {entry.name:20s}  {entry.description}")

    return 0


# Subparser builder


def add_pkg_subparser(
    sub: argparse._SubParsersAction,
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> None:
    """Register the ``pkg`` verb group and all of its sub-subparsers."""
    pkg_p = sub.add_parser(
        "pkg",
        help="Manage Tcl packages and lockfiles.",
        description="Manage Tcl packages, dependencies, and the tclpkg.lock lockfile.",
    )
    pkg_sub = pkg_p.add_subparsers(dest="pkg_action", required=True)

    # Shared arguments helper.
    def _common(parser: argparse.ArgumentParser) -> None:
        parser.add_argument("--json", action="store_true", help="Emit JSON output.")
        parser.add_argument("--manifest", metavar="PATH", help="Override tclpkg.tcl location.")
        parser.add_argument("--offline", action="store_true", help="Never touch the network.")

    # init
    init_p = pkg_sub.add_parser("init", help="Create a new tclpkg.tcl manifest.")
    init_p.add_argument("--name", help="Package name (default: directory name).")
    init_p.add_argument("--version", dest="init_version", help="Initial version.")
    init_p.add_argument("--license", dest="init_license", help="SPDX licence identifier.")
    init_p.add_argument("--tcl", help="Tcl version constraint (default: >=8.6).")
    init_p.add_argument("--force", action="store_true", help="Overwrite existing manifest.")
    init_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    init_p.set_defaults(handler=_run_init)

    # install
    install_p = pkg_sub.add_parser("install", help="Resolve + fetch + materialise packages.")
    _common(install_p)
    install_p.add_argument("--no-dev", action="store_true", help="Skip dev-require packages.")
    install_p.add_argument("--frozen", action="store_true", help="Refuse to change lockfile.")
    install_p.set_defaults(handler=_run_install)

    # list
    list_p = pkg_sub.add_parser("list", help="List installed packages.")
    _common(list_p)
    list_p.set_defaults(handler=_run_list)

    # tree
    tree_p = pkg_sub.add_parser("tree", help="Show dependency tree.")
    _common(tree_p)
    tree_p.set_defaults(handler=_run_tree)

    # verify
    verify_p = pkg_sub.add_parser("verify", help="Verify integrity hashes.")
    _common(verify_p)
    verify_p.set_defaults(handler=_run_verify)

    # info
    info_p = pkg_sub.add_parser("info", help="Show details for a package.")
    info_p.add_argument("package", help="Package name.")
    _common(info_p)
    info_p.set_defaults(handler=_run_info)

    # search
    search_p = pkg_sub.add_parser("search", help="Search the package registry.")
    search_p.add_argument("query", help="Search query.")
    search_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    search_p.add_argument("--offline", action="store_true", help="Use cached registry only.")
    search_p.set_defaults(handler=_run_search)
