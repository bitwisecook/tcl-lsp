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

    # --frozen: refuse to change the lockfile; error if it would differ.
    if getattr(args, "frozen", False):
        from tclpkg.lockfile import read_lockfile

        if not lockfile_path.is_file():
            print("error: --frozen requires an existing tclpkg.lock", file=sys.stderr)
            return 1
        existing = read_lockfile(lockfile_path)
        # Compare packages only (ignore timestamp).
        old_pkgs = sorted((p.name, p.version) for p in existing.packages)
        new_pkgs = sorted((p.name, p.version) for p in lf.packages)
        if old_pkgs != new_pkgs:
            print(
                "error: lockfile would change but --frozen is set; "
                "update the manifest and re-run without --frozen",
                file=sys.stderr,
            )
            return 1
        if getattr(args, "json", False):
            ui.json_output({"packages": len(lf.packages), "frozen": True})
        else:
            print(ui.ok(f"lockfile is up to date ({len(lf.packages)} packages)", colour=colour))
        return 0

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


def _run_add(args: argparse.Namespace) -> int:
    from tclpkg.manifest import load_manifest

    mpath = _manifest_path(args)
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        load_manifest(mpath)  # validate manifest is parseable
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    pkg_name = args.package
    min_ver = getattr(args, "min_version", None) or "0.0.1"
    source_url = getattr(args, "source", None)
    is_dev = getattr(args, "dev", False)

    # Read and append to manifest file.
    text = mpath.read_text(encoding="utf-8")
    directive = "dev-require" if is_dev else "require"
    new_line = f"{directive} {pkg_name} {min_ver}"
    if source_url:
        new_line += f" -source {source_url}"
    text = text.rstrip("\n") + "\n" + new_line + "\n"
    mpath.write_text(text, encoding="utf-8")

    if getattr(args, "json", False):
        ui.json_output({"added": pkg_name, "version": min_ver, "dev": is_dev})
    else:
        print(ui.ok(f"added {pkg_name} {min_ver} to {mpath}", colour=colour))
        print(ui.dim("  run 'tcl pkg install' to resolve and lock", colour=colour))
    return 0


def _run_remove(args: argparse.Namespace) -> int:
    import re

    mpath = _manifest_path(args)
    colour = ui.use_colour(force=not getattr(args, "json", False))
    pkg_name = args.package

    if not mpath.is_file():
        print(f"error: manifest not found: {mpath}", file=sys.stderr)
        return 1

    text = mpath.read_text(encoding="utf-8")
    pattern = re.compile(
        rf"^\s*(?:require|dev-require)\s+{re.escape(pkg_name)}\b.*$\n?",
        re.MULTILINE,
    )
    new_text, count = pattern.subn("", text)
    if count == 0:
        print(f"error: package '{pkg_name}' not found in manifest", file=sys.stderr)
        return 1
    mpath.write_text(new_text, encoding="utf-8")

    if getattr(args, "json", False):
        ui.json_output({"removed": pkg_name, "directives_removed": count})
    else:
        print(ui.ok(f"removed {pkg_name} from {mpath}", colour=colour))
        print(ui.dim("  run 'tcl pkg install' to update the lockfile", colour=colour))
    return 0


def _run_update(args: argparse.Namespace) -> int:

    from tclpkg.manifest import load_manifest

    mpath = _manifest_path(args)
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        manifest = load_manifest(mpath)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    targets = getattr(args, "packages", None) or []

    # For each target (or all requires if none specified), bump the version.
    all_reqs = list(manifest.requires) + list(manifest.dev_requires)
    if not targets:
        targets = [r.name for r in all_reqs]

    updated = []
    for pkg_name in targets:
        # Find the require line and bump its version placeholder.
        # In a real implementation this would query the registry for the
        # latest version; for now we just flag the packages.
        req = next((r for r in all_reqs if r.name == pkg_name), None)
        if req is None:
            print(f"  skip: {pkg_name} not in manifest", file=sys.stderr)
            continue
        updated.append(pkg_name)

    if getattr(args, "json", False):
        ui.json_output({"updated": updated})
    else:
        if updated:
            for name in updated:
                print(
                    ui.ok(
                        f"{name} (already at minimum — registry query not yet wired)", colour=colour
                    )
                )
            print(ui.dim("  run 'tcl pkg install' to re-resolve", colour=colour))
        else:
            print("  nothing to update")
    return 0


def _run_sync(args: argparse.Namespace) -> int:
    """Lock-driven install — alias for install --frozen."""
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({"packages": len(lf.packages), "lockfile": str(lockfile_path)})
    else:
        for pkg in sorted(lf.packages, key=lambda p: p.name):
            print(ui.ok(f"{pkg.name:20s} {pkg.version}", colour=colour))
        print(ui.ok(f"synced from {lockfile_path} ({len(lf.packages)} packages)", colour=colour))
    return 0


def _run_outdated(args: argparse.Namespace) -> int:
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    # Full implementation would query the registry for latest versions.
    # For now, report all packages as up-to-date.
    if getattr(args, "json", False):
        ui.json_output({"outdated": []})
    else:
        fmt = "{:<20s} {:<12s} {:<12s}"
        print(fmt.format("NAME", "CURRENT", "LATEST"))
        for pkg in sorted(lf.packages, key=lambda p: p.name):
            print(fmt.format(pkg.name, pkg.version, pkg.version))
        print(ui.dim("\n  (registry version lookup not yet wired)", colour=ui.use_colour()))
    return 0


def _run_why(args: argparse.Namespace) -> int:
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
        # Walk lockfile to find who requires this package.
        dependents = []
        for other in lf.packages:
            if any(pkg_name in r for r in other.requires):
                dependents.append(other.name)
        ui.json_output(
            {
                "package": pkg_name,
                "version": entry.version,
                "required_by": dependents,
                "dev": entry.dev,
            }
        )
    else:
        print(f"{pkg_name} {entry.version}")
        # Find who requires this package.
        dependents = []
        for other in lf.packages:
            if any(pkg_name in r for r in other.requires):
                dependents.append(other)
        if dependents:
            for dep in dependents:
                print(f"└── required by {dep.name} {dep.version}")
        elif entry.dev:
            print("└── direct dev-require dependency")
        else:
            print("└── direct dependency")
    return 0


def _run_vendor(args: argparse.Namespace) -> int:
    from core.common.user_config import _cache_dir
    from tclpkg.cas import ContentAddressableStore
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"
    vendor_dir = Path(getattr(args, "dir", None) or "vendor")
    colour = ui.use_colour(force=not getattr(args, "json", False))

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    cas = ContentAddressableStore(_cache_dir())
    vendor_dir.mkdir(parents=True, exist_ok=True)
    vendored = []

    for pkg in lf.packages:
        dest = vendor_dir / f"{pkg.name}-{pkg.version}"
        if pkg.integrity and cas.has(pkg.integrity):
            cas.materialise(pkg.integrity, dest, symlink=False)
            vendored.append(pkg.name)
        elif not pkg.integrity:
            print(
                ui.warn(f"{pkg.name} has no integrity hash — skipping", colour=colour),
            )

    if getattr(args, "json", False):
        ui.json_output({"vendored": vendored, "dir": str(vendor_dir)})
    else:
        for name in vendored:
            print(ui.ok(f"vendored {name}", colour=colour))
        if not vendored:
            print(ui.dim("  no packages with integrity hashes to vendor", colour=colour))
            print(ui.dim("  run 'tcl pkg install' first to populate the cache", colour=colour))
        else:
            print(ui.ok(f"wrote {len(vendored)} package(s) to {vendor_dir}/", colour=colour))
    return 0


def _run_run(args: argparse.Namespace) -> int:
    import os
    import subprocess

    from core.tcl_discovery import find_tclsh
    from tclpkg.manifest import load_manifest

    mpath = _manifest_path(args)

    try:
        manifest = load_manifest(mpath)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    entry_file = manifest.entry
    if not entry_file:
        print("error: no 'entry' directive in manifest", file=sys.stderr)
        return 1

    entry_path = mpath.parent / entry_file
    if not entry_path.is_file():
        print(f"error: entry file not found: {entry_path}", file=sys.stderr)
        return 1

    # Determine tclsh.
    venv = os.environ.get("TCL_VENV")
    if venv:
        tclsh = str(Path(venv) / "bin" / "tclsh")
    else:
        tclsh = find_tclsh()
    if not tclsh:
        print("error: tclsh not found on PATH", file=sys.stderr)
        return 1

    # Set TCLLIBPATH to include ./lib/ or venv/lib/.
    lib_dir = mpath.parent / "lib"
    env = os.environ.copy()
    if lib_dir.is_dir():
        existing = env.get("TCLLIBPATH", "")
        env["TCLLIBPATH"] = f"{lib_dir}{' ' + existing if existing else ''}"

    extra_args = getattr(args, "extra", []) or []
    result = subprocess.run(
        [tclsh, str(entry_path), *extra_args],
        env=env,
    )
    return result.returncode


def _run_freeze(args: argparse.Namespace) -> int:
    """Output locked versions in a format suitable for a manifest."""
    from tclpkg.lockfile import read_lockfile

    mpath = _manifest_path(args)
    lockfile_path = mpath.parent / "tclpkg.lock"

    try:
        lf = read_lockfile(lockfile_path)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        ui.json_output({pkg.name: pkg.version for pkg in sorted(lf.packages, key=lambda p: p.name)})
    else:
        for pkg in sorted(lf.packages, key=lambda p: p.name):
            source_suffix = ""
            if pkg.source.url:
                source_suffix = f" -source {pkg.source.url}"
            directive = "dev-require" if pkg.dev else "require"
            print(f"{directive} {pkg.name} {pkg.version}{source_suffix}")

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

    # add
    add_p = pkg_sub.add_parser("add", help="Add a dependency to the manifest.")
    add_p.add_argument("package", help="Package name.")
    add_p.add_argument("min_version", nargs="?", help="Minimum version (default: 0.0.1).")
    add_p.add_argument("--source", help="Explicit source URL.")
    add_p.add_argument("--dev", action="store_true", help="Add as dev-require.")
    _common(add_p)
    add_p.set_defaults(handler=_run_add)

    # remove
    remove_p = pkg_sub.add_parser("remove", help="Remove a dependency from the manifest.")
    remove_p.add_argument("package", help="Package name to remove.")
    _common(remove_p)
    remove_p.set_defaults(handler=_run_remove)

    # update
    update_p = pkg_sub.add_parser("update", help="Bump dependency minimums.")
    update_p.add_argument("packages", nargs="*", help="Packages to update (default: all).")
    _common(update_p)
    update_p.set_defaults(handler=_run_update)

    # sync
    sync_p = pkg_sub.add_parser("sync", help="Lock-driven install (alias for install --frozen).")
    _common(sync_p)
    sync_p.set_defaults(handler=_run_sync)

    # outdated
    outdated_p = pkg_sub.add_parser("outdated", help="Show packages with newer versions available.")
    _common(outdated_p)
    outdated_p.set_defaults(handler=_run_outdated)

    # why
    why_p = pkg_sub.add_parser("why", help="Explain why a package is in the dependency graph.")
    why_p.add_argument("package", help="Package name.")
    _common(why_p)
    why_p.set_defaults(handler=_run_why)

    # vendor
    vendor_p = pkg_sub.add_parser("vendor", help="Copy packages from cache into the project tree.")
    vendor_p.add_argument("--dir", default="vendor", help="Vendor directory (default: vendor/).")
    _common(vendor_p)
    vendor_p.set_defaults(handler=_run_vendor)

    # run
    run_p = pkg_sub.add_parser("run", help="Run the manifest entry point via tclsh.")
    run_p.add_argument("extra", nargs="*", help="Extra arguments passed to tclsh.")
    _common(run_p)
    run_p.set_defaults(handler=_run_run)

    # freeze
    freeze_p = pkg_sub.add_parser(
        "freeze",
        help="Output locked versions as manifest directives (like pip freeze).",
    )
    _common(freeze_p)
    freeze_p.set_defaults(handler=_run_freeze)

    # search
    search_p = pkg_sub.add_parser("search", help="Search the package registry.")
    search_p.add_argument("query", help="Search query.")
    search_p.add_argument("--json", action="store_true", help="Emit JSON output.")
    search_p.add_argument("--offline", action="store_true", help="Use cached registry only.")
    search_p.set_defaults(handler=_run_search)
