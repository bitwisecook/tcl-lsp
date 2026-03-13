"""Package command implementation."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING, TypedDict, cast

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


class _PackageEntry(TypedDict):
    version: str | None
    loaded: bool
    ifneeded: dict[str, str]


def _cmd_package(interp: TclInterp, args: list[str]) -> TclResult:
    """package subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "package option ?arg ...?"')

    subcmd = args[0]
    rest = args[1:]

    match subcmd:
        case "require":
            return _pkg_require(interp, rest)
        case "provide":
            return _pkg_provide(interp, rest)
        case "ifneeded":
            return _pkg_ifneeded(interp, rest)
        case "names":
            return _pkg_names(interp, rest)
        case "versions":
            return _pkg_versions(interp, rest)
        case "present":
            return _pkg_present(interp, rest)
        case "forget":
            return _pkg_forget(interp, rest)
        case "prefer":
            return _pkg_prefer(interp, rest)
        case "vcompare":
            return _pkg_vcompare(interp, rest)
        case "vsatisfies":
            return _pkg_vsatisfies(interp, rest)
        case "unknown":
            return _pkg_unknown(interp, rest)
        case _:
            raise TclError(
                f'bad option "{subcmd}": must be forget, ifneeded, names, '
                f"prefer, present, provide, require, unknown, vcompare, "
                f"or vsatisfies"
            )


def _get_pkg(interp: TclInterp, name: str) -> _PackageEntry:
    """Get or create a package entry."""
    if name not in interp.packages:
        interp.packages[name] = {
            "version": None,
            "loaded": False,
            "ifneeded": {},  # version -> script
        }
    return cast(_PackageEntry, interp.packages[name])


def _pkg_require(interp: TclInterp, args: list[str]) -> TclResult:
    """package require ?-exact? name ?version?"""
    exact = False
    name_args = list(args)
    if name_args and name_args[0] == "-exact":
        exact = True
        name_args = name_args[1:]

    if not name_args:
        raise TclError(
            'wrong # args: should be "package require ?-exact? package ?requirement ...?"'
        )

    pkg_name = name_args[0]
    required_version = name_args[1] if len(name_args) > 1 else None

    pkg = _get_pkg(interp, pkg_name)

    # Already loaded?
    if pkg["loaded"]:
        version = str(pkg["version"] or "")
        if required_version and exact:
            if version != required_version:
                raise TclError(
                    f'version conflict for package "{pkg_name}": '
                    f"have {version}, need exactly {required_version}"
                )
        return TclResult(value=version)

    # Try ifneeded scripts
    ifneeded = pkg["ifneeded"]
    if ifneeded:
        # Find best matching version
        script = None
        best_version = None
        if required_version and exact:
            script = ifneeded.get(required_version)
            best_version = required_version
        else:
            # Pick the highest version that satisfies
            versions = sorted(ifneeded.keys(), key=_version_key, reverse=True)
            for v in versions:
                if required_version is None or _version_satisfies(v, required_version):
                    script = ifneeded[v]
                    best_version = v
                    break

        if script is not None:
            interp.eval(str(script))
            # After eval, check if package provide was called
            if not pkg["loaded"]:
                # Auto-provide if the script didn't
                pkg["loaded"] = True
                if best_version and not pkg["version"]:
                    pkg["version"] = best_version
            return TclResult(value=str(pkg["version"] or ""))

    # Try the package unknown handler
    unknown_handler = interp._package_unknown
    if unknown_handler:
        interp.eval(f"{unknown_handler} {pkg_name} {required_version or ''}")

        # The handler may have called ``package provide`` directly …
        if pkg["loaded"]:
            return TclResult(value=str(pkg["version"] or ""))

        # … or registered an ``ifneeded`` script. Re-check.
        ifneeded = pkg["ifneeded"]
        if ifneeded:
            script = None
            best_version = None
            if required_version and exact:
                script = ifneeded.get(required_version)
                best_version = required_version
            else:
                versions = sorted(ifneeded.keys(), key=_version_key, reverse=True)
                for v in versions:
                    if required_version is None or _version_satisfies(v, required_version):
                        script = ifneeded[v]
                        best_version = v
                        break

            if script is not None:
                interp.eval(str(script))
                if not pkg["loaded"]:
                    pkg["loaded"] = True
                    if best_version and not pkg["version"]:
                        pkg["version"] = best_version
                return TclResult(value=str(pkg["version"] or ""))

    raise TclError(f"can't find package {pkg_name}")


def _pkg_provide(interp: TclInterp, args: list[str]) -> TclResult:
    """package provide name ?version?"""
    if not args:
        raise TclError('wrong # args: should be "package provide package ?version?"')

    pkg_name = args[0]
    pkg = _get_pkg(interp, pkg_name)

    if len(args) < 2:
        # Query current version
        version = pkg["version"]
        return TclResult(value=str(version) if version else "")

    version = args[1]
    pkg["version"] = version
    pkg["loaded"] = True
    return TclResult()


def _pkg_ifneeded(interp: TclInterp, args: list[str]) -> TclResult:
    """package ifneeded name version ?script?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "package ifneeded package version ?script?"')

    pkg_name = args[0]
    version = args[1]
    pkg = _get_pkg(interp, pkg_name)

    ifneeded = pkg["ifneeded"]

    if len(args) < 3:
        # Query
        script = ifneeded.get(version)
        return TclResult(value=str(script) if script else "")

    ifneeded[version] = args[2]
    return TclResult()


def _pkg_names(interp: TclInterp, _args: list[str]) -> TclResult:
    """package names"""
    names = sorted(interp.packages.keys())
    return TclResult(value=" ".join(names))


def _pkg_versions(interp: TclInterp, args: list[str]) -> TclResult:
    """package versions name"""
    if not args:
        raise TclError('wrong # args: should be "package versions package"')

    pkg_name = args[0]
    pkg = interp.packages.get(pkg_name)
    if pkg is None:
        return TclResult(value="")
    typed_pkg = cast(_PackageEntry, pkg)

    return TclResult(value=" ".join(sorted(typed_pkg["ifneeded"].keys())))


def _pkg_present(interp: TclInterp, args: list[str]) -> TclResult:
    """package present ?-exact? name ?version?"""
    exact = False
    name_args = list(args)
    if name_args and name_args[0] == "-exact":
        exact = True
        name_args = name_args[1:]

    if not name_args:
        raise TclError(
            'wrong # args: should be "package present ?-exact? package ?requirement ...?"'
        )

    pkg_name = name_args[0]
    required_version = name_args[1] if len(name_args) > 1 else None

    pkg = interp.packages.get(pkg_name)
    if pkg is None:
        raise TclError(f"package {pkg_name} is not present")
    typed_pkg = cast(_PackageEntry, pkg)
    if not typed_pkg["loaded"]:
        raise TclError(f"package {pkg_name} is not present")

    version = str(typed_pkg["version"] or "")
    if required_version and exact and version != required_version:
        raise TclError(
            f'version conflict for package "{pkg_name}": '
            f"have {version}, need exactly {required_version}"
        )
    return TclResult(value=version)


def _pkg_forget(interp: TclInterp, args: list[str]) -> TclResult:
    """package forget ?name ...?"""
    for name in args:
        interp.packages.pop(name, None)
    return TclResult()


def _pkg_prefer(interp: TclInterp, args: list[str]) -> TclResult:
    """package prefer ?latest|stable?"""
    # Stub — always return "stable"
    if args:
        return TclResult(value=args[0])
    return TclResult(value="stable")


def _pkg_vcompare(interp: TclInterp, args: list[str]) -> TclResult:
    """package vcompare version1 version2"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "package vcompare version1 version2"')

    k1 = _version_key(args[0])
    k2 = _version_key(args[1])
    if k1 < k2:
        return TclResult(value="-1")
    if k1 > k2:
        return TclResult(value="1")
    return TclResult(value="0")


def _pkg_vsatisfies(interp: TclInterp, args: list[str]) -> TclResult:
    """package vsatisfies version requirement ..."""
    if len(args) < 2:
        raise TclError(
            'wrong # args: should be "package vsatisfies version requirement ?requirement ...?"'
        )

    version = args[0]
    for req in args[1:]:
        if _version_satisfies(version, req):
            return TclResult(value="1")
    return TclResult(value="0")


def _pkg_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """package unknown ?command?"""
    if not args:
        handler = interp._package_unknown
        return TclResult(value=str(handler) if handler else "")
    interp._package_unknown = args[0]
    return TclResult()


# Version comparison helpers


def _version_key(version: str) -> tuple[int, ...]:
    """Convert a version string to a comparable tuple."""
    parts = re.split(r"[.ab]", version)
    result: list[int] = []
    for p in parts:
        try:
            result.append(int(p))
        except ValueError:
            result.append(0)
    return tuple(result)


def _version_satisfies(version: str, requirement: str) -> bool:
    """Check if *version* satisfies *requirement*.

    Tcl version requirements can be:
    - ``X.Y`` — version >= X.Y and < (X+1).0
    - ``X.Y-`` — version >= X.Y (no upper bound)
    - ``X.Y-X2.Y2`` — version >= X.Y and <= X2.Y2
    """
    if "-" in requirement:
        parts = requirement.split("-", 1)
        lower = _version_key(parts[0])
        upper_str = parts[1]
        if not upper_str:
            # Trailing hyphen: ``X.Y-`` means >= X.Y with no upper bound
            return _version_key(version) >= lower
        return _version_key(version) >= lower and _version_key(version) <= _version_key(upper_str)

    req_key = _version_key(requirement)
    ver_key = _version_key(version)

    if not req_key:
        return True

    # version must be >= requirement and have the same major version
    return ver_key >= req_key and ver_key[0] == req_key[0]


def register() -> None:
    """Register package commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("package", _cmd_package)
