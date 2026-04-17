"""``tclpkg.tcl`` manifest loader.

The manifest is a tiny Tcl file at the project root.  It is evaluated
inside a sandboxed ``vm.interp.TclInterp`` whose command dispatcher only
permits the directives in ``MANIFEST_DIRECTIVES`` — ``exec``, ``file``,
``source``, ``open`` and everything else is refused by the safe-mode
whitelist (see ``docs/kcs/kcs-tclpkg-manifest-contracts.md``).

The loader returns a :class:`ManifestAST` — a plain dataclass that the
rest of the package manager consumes.  Nothing else in tclpkg talks to
the VM directly; the rest of the pipeline is pure data.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from vm.interp import TclInterp
from vm.types import TclError, TclResult

from .errors import ManifestError
from .version import Version, VersionError

# SPDX-style licence identifier, lightly validated.  We do not maintain a
# full allowlist — users can write any non-empty value and tclpkg just
# records it verbatim.
_SPDX_RE = re.compile(r"^[A-Za-z0-9.\-+ ]+$")

# Tcl runtime constraint syntax: an optional operator followed by a
# semver-ish version string.  ``>=8.6`` / ``>=9.0`` / ``8.6`` (shorthand
# for ``>=8.6``) are all accepted.
_TCL_CONSTRAINT_RE = re.compile(r"^\s*(>=|>|==|=|<=|<)?\s*([0-9][0-9a-zA-Z.+\-]*)\s*$")


@dataclass
class Requirement:
    """A single ``require`` / ``dev-require`` entry."""

    name: str
    minimum: Version
    source_url: str | None = None
    dev: bool = False


@dataclass
class Replacement:
    """A single ``replace`` directive."""

    name: str
    version: Version
    source_url: str


@dataclass
class Exclusion:
    """A single ``exclude`` directive."""

    name: str
    version: Version


@dataclass
class ManifestAST:
    """Parsed, validated representation of a ``tclpkg.tcl`` manifest."""

    name: str = ""
    version: Version | None = None
    description: str = ""
    license: str = ""
    authors: list[str] = field(default_factory=list)
    homepage: str = ""
    tcl_constraint: str = ">=8.6"
    requires: list[Requirement] = field(default_factory=list)
    dev_requires: list[Requirement] = field(default_factory=list)
    replaces: list[Replacement] = field(default_factory=list)
    excludes: list[Exclusion] = field(default_factory=list)
    provides: list[str] = field(default_factory=list)
    entry: str = ""
    path: str = ""


# Commands permitted inside a ``tclpkg.tcl`` manifest.  This is the
# *total* allowlist — basic Tcl builtins (``set``, ``list``, etc.) are
# not permitted because the manifest should be pure data.
_MANIFEST_DIRECTIVES: frozenset[str] = frozenset(
    {
        "package",
        "version",
        "description",
        "license",
        "author",
        "homepage",
        "tcl",
        "require",
        "dev-require",
        "replace",
        "exclude",
        "provides",
        "entry",
    }
)


def _ok() -> TclResult:
    return TclResult(value="")


def _require_arity(name: str, args: list[str], *, at_least: int, at_most: int) -> None:
    count = len(args)
    if count < at_least or count > at_most:
        expected = (
            f"{at_least} arg(s)"
            if at_least == at_most
            else f"between {at_least} and {at_most} args"
        )
        raise TclError(f'wrong # args: "{name}" expects {expected}, got {count}')


def _parse_version(name: str, raw: str) -> Version:
    try:
        return Version.parse(raw)
    except VersionError as exc:
        raise TclError(f"{name}: {exc}") from exc


def _parse_source_option(args: list[str]) -> tuple[list[str], str | None]:
    """Strip an optional trailing ``-source URL`` pair from ``args``.

    Returns ``(remaining_args, source_url_or_none)``.
    """
    if len(args) >= 2 and args[-2] == "-source":
        return args[:-2], args[-1]
    return args, None


def _build_directives(ast: ManifestAST):
    """Create closure-bound directive handlers for a fresh AST.

    Each handler appends to the provided ``ast`` so the caller can
    inspect the result once evaluation finishes.
    """

    def package_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("package", args, at_least=1, at_most=1)
        if ast.name:
            raise TclError("package directive already set")
        ast.name = args[0]
        return _ok()

    def version_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("version", args, at_least=1, at_most=1)
        ast.version = _parse_version("version", args[0])
        return _ok()

    def description_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("description", args, at_least=1, at_most=1)
        ast.description = args[0]
        return _ok()

    def license_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("license", args, at_least=1, at_most=1)
        value = args[0]
        if not _SPDX_RE.match(value):
            raise TclError(f"license: invalid SPDX-style identifier: {value!r}")
        ast.license = value
        return _ok()

    def author_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("author", args, at_least=1, at_most=1)
        ast.authors.append(args[0])
        return _ok()

    def homepage_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("homepage", args, at_least=1, at_most=1)
        ast.homepage = args[0]
        return _ok()

    def tcl_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("tcl", args, at_least=1, at_most=1)
        raw = args[0]
        if not _TCL_CONSTRAINT_RE.match(raw):
            raise TclError(f"tcl: invalid version constraint: {raw!r}")
        # Normalise shorthand ``8.6`` to ``>=8.6``.
        if not any(raw.lstrip().startswith(op) for op in (">=", ">", "==", "=", "<=", "<")):
            raw = f">={raw.strip()}"
        ast.tcl_constraint = raw
        return _ok()

    def require_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("require", args, at_least=2, at_most=4)
        remainder, source = _parse_source_option(args)
        if len(remainder) != 2:
            raise TclError('wrong # args: "require" expects name, minimum, ?-source URL?')
        ast.requires.append(
            Requirement(
                name=remainder[0],
                minimum=_parse_version("require", remainder[1]),
                source_url=source,
                dev=False,
            )
        )
        return _ok()

    def dev_require_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("dev-require", args, at_least=2, at_most=4)
        remainder, source = _parse_source_option(args)
        if len(remainder) != 2:
            raise TclError('wrong # args: "dev-require" expects name, minimum, ?-source URL?')
        ast.dev_requires.append(
            Requirement(
                name=remainder[0],
                minimum=_parse_version("dev-require", remainder[1]),
                source_url=source,
                dev=True,
            )
        )
        return _ok()

    def replace_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("replace", args, at_least=3, at_most=3)
        ast.replaces.append(
            Replacement(
                name=args[0],
                version=_parse_version("replace", args[1]),
                source_url=args[2],
            )
        )
        return _ok()

    def exclude_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("exclude", args, at_least=2, at_most=2)
        ast.excludes.append(
            Exclusion(
                name=args[0],
                version=_parse_version("exclude", args[1]),
            )
        )
        return _ok()

    def provides_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("provides", args, at_least=1, at_most=1)
        ast.provides.append(args[0])
        return _ok()

    def entry_cmd(interp: TclInterp, args: list[str]) -> TclResult:
        _require_arity("entry", args, at_least=1, at_most=1)
        ast.entry = args[0]
        return _ok()

    return {
        "package": package_cmd,
        "version": version_cmd,
        "description": description_cmd,
        "license": license_cmd,
        "author": author_cmd,
        "homepage": homepage_cmd,
        "tcl": tcl_cmd,
        "require": require_cmd,
        "dev-require": dev_require_cmd,
        "replace": replace_cmd,
        "exclude": exclude_cmd,
        "provides": provides_cmd,
        "entry": entry_cmd,
    }


def load_manifest_text(source: str, *, path: str | None = None) -> ManifestAST:
    """Evaluate a manifest source string and return the AST.

    Raises :class:`ManifestError` on any failure.  ``path`` is recorded
    on the result and used in error messages.
    """
    ast = ManifestAST(path=path or "")

    interp = TclInterp(
        source_init=False,
        safe=True,
        safe_whitelist=_MANIFEST_DIRECTIVES,
    )
    for name, handler in _build_directives(ast).items():
        interp.register_command(name, handler)

    try:
        interp.eval(source)
    except TclError as exc:
        raise ManifestError(exc.message, path=path) from exc
    except Exception as exc:
        raise ManifestError(f"failed to evaluate manifest: {exc}", path=path) from exc

    _validate(ast)
    return ast


def load_manifest(path: str | Path) -> ManifestAST:
    """Read a manifest file from disk and evaluate it."""
    file_path = Path(path)
    try:
        text = file_path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise ManifestError(
            f"manifest not found: {file_path}",
            hint="run 'tcl pkg init' to create one",
        ) from exc
    except OSError as exc:
        raise ManifestError(f"cannot read manifest {file_path}: {exc}") from exc
    return load_manifest_text(text, path=str(file_path))


def _validate(ast: ManifestAST) -> None:
    """Post-parse validation of required fields and cross-references."""
    if not ast.name:
        raise ManifestError(
            "manifest missing required 'package' directive",
            path=ast.path or None,
        )
    if ast.version is None:
        raise ManifestError(
            "manifest missing required 'version' directive",
            path=ast.path or None,
        )
    # Direct conflicts between require and dev-require for the same name.
    seen_names: dict[str, str] = {}
    for req in ast.requires:
        seen_names[req.name] = "require"
    for req in ast.dev_requires:
        if req.name in seen_names:
            raise ManifestError(
                f"package {req.name!r} declared both as require and dev-require",
                path=ast.path or None,
            )
        seen_names[req.name] = "dev-require"


__all__ = [
    "Exclusion",
    "ManifestAST",
    "Replacement",
    "Requirement",
    "load_manifest",
    "load_manifest_text",
]
