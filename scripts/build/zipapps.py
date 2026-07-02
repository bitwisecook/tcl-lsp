"""Build a zipapp (.pyz) for the LSP server, the CLIs, or the AI/MCP servers.

Usage:
    python scripts/build/zipapps.py <profile> --version VERSION --output PATH

Profiles:

    tcl              Unified Tcl tools CLI       (entry: scripts/zipapp-main/tcl.py)
    f5               F5 BIG-IP CLI               (entry: tooling.f5.main)
    lsp              LSP server                  (entry: scripts/zipapp-main/lsp.py)
    ai               AI analysis CLI             (entry: scripts/zipapp-main/ai.py)
    mcp              MCP server                  (entry: scripts/zipapp-main/mcp.py)
    claude-skills    Bundled Claude Code skill set (--ai-pyz REQUIRED)

The compiler-explorer CLI/GUI and the Tcl→WASM compiler are no longer Python
zipapps: they ship inside the native `tcl` binary (`tcl explore`,
`tcl explore --serve`, `tcl compwasm`).

Each profile declares which of the seven concern packages it ships
(`shared`, `compiler`, `dialects`, `analyser`, `server`, `tooling`,
`ai`), which third-party pip packages it bundles, and its entry
point.  See `PROFILES` below.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import zipapp
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

# Minimum Python a shipped zipapp must run on — keep in lockstep with the
# ``requires-python`` floor in pyproject.toml.  Bundled pip dependencies are
# resolved *for this version* (not the build interpreter) so conditional,
# marker-gated deps like ``exceptiongroup; python_version < "3.11"`` (a cattrs
# requirement reached via pygls) get bundled even when the release is built on
# a newer Python.  Without this, a 3.11+ build silently drops them and the
# zipapp dies with ``ModuleNotFoundError`` on a 3.10 host.
MIN_PYTHON = "3.10"

# All seven concern packages — keep in dependency order (top-down).
ALL_PACKAGES: tuple[str, ...] = (
    "shared",
    "compiler",
    "dialects",
    "analyser",
    "server",
    "tooling",
    "ai",
)


@dataclass(frozen=True)
class Profile:
    """Build recipe for one zipapp variant.

    `packages` is the subset of `ALL_PACKAGES` to copy into the
    staging dir.  `pip_packages` are bundled into the zipapp via
    `uv pip install --target stage` (pure-Python only — C extensions
    are stripped post-install).  Exactly one of `entry_script` or
    `entry_module` describes how the zipapp boots:

    - `entry_script` points at a hand-written `scripts/zipapp-main/*.py`
      that becomes the zipapp's `__main__.py` verbatim.
    - `entry_module` is a `module.path:function` reference; the helper
      synthesises a one-line `__main__.py` that imports and calls it.
    """

    name: str
    packages: tuple[str, ...]
    pip_packages: tuple[str, ...] = ()
    entry_script: Path | None = None
    entry_module: str | None = None
    # When True, exclude the explorer's static/ web bundle from the copy.
    # Always exclude __pycache__ and *.pyc.
    exclude_explorer_static: bool = True
    # Extra description bytes to include in the smoke-test output.
    extra: dict[str, str] = field(default_factory=dict)


def _ignore_patterns(profile: Profile, package: str):
    base = ["__pycache__", "*.pyc"]
    if profile.exclude_explorer_static and package == "tooling":
        # Any leftover tooling/*/static/ web assets are dev-only; no zipapp
        # ships a web payload (the GUI lives in the native `tcl` binary).
        base.append("static")
    name_ignore = shutil.ignore_patterns(*base)
    if package != "tooling":
        return name_ignore

    # The differential fuzzer's recorded findings corpus (~15 MB) and its
    # test suite are dev-only — no zipapp ships the fuzzer — so drop them
    # from every profile's tooling/ copy.  Path-scoped so a ``findings``
    # or ``tests`` dir elsewhere in tooling/ is unaffected.
    fuzz_dir = (ROOT / "tooling" / "fuzzing").resolve()

    def ignore(dir: str, names: list[str]) -> set[str]:
        ignored = set(name_ignore(dir, names))
        if Path(dir).resolve() == fuzz_dir:
            ignored.update(n for n in ("findings", "tests") if n in names)
        return ignored

    return ignore


def _copy_concern_packages(stage: Path, profile: Profile) -> None:
    for pkg in profile.packages:
        shutil.copytree(
            ROOT / pkg,
            stage / pkg,
            ignore=_ignore_patterns(profile, pkg),
        )


def _pip_install_pure(stage: Path, packages: tuple[str, ...]) -> None:
    """Install pure-Python packages into the staging directory.

    Strips C extensions (.so/.pyd), dist-info, and __pycache__ so the
    result can be packed into a zipapp.
    """
    if not packages:
        return
    subprocess.check_call(
        [
            "uv",
            "pip",
            "install",
            "--target",
            str(stage),
            # Resolve environment markers for the minimum supported Python
            # rather than the (possibly newer) build interpreter, so
            # version-gated transitive deps are bundled.  See MIN_PYTHON.
            "--python-version",
            MIN_PYTHON,
            "--quiet",
            *packages,
        ]
    )
    for ext in stage.rglob("*.so"):
        ext.unlink()
    for ext in stage.rglob("*.pyd"):
        ext.unlink()
    for p in stage.rglob("*.dist-info"):
        shutil.rmtree(p)
    for p in stage.rglob("__pycache__"):
        shutil.rmtree(p)


def _write_entry_point(stage: Path, profile: Profile) -> None:
    main_py = stage / "__main__.py"
    if profile.entry_script is not None:
        shutil.copy2(profile.entry_script, main_py)
    elif profile.entry_module is not None:
        module, _, attr = profile.entry_module.partition(":")
        attr = attr or "main"
        main_py.write_text(
            f'"""{profile.name} zipapp entry point."""\n'
            f"import sys\n"
            f"from {module} import {attr}\n"
            f"sys.exit({attr}())\n"
        )
    else:
        raise RuntimeError(
            f"profile {profile.name!r} declares neither entry_script nor entry_module"
        )


def _create_archive(source: Path, target: str) -> None:
    zipapp.create_archive(
        source=source,
        target=target,
        interpreter="/usr/bin/env python3",
        compressed=True,
    )


def _report(output: Path) -> None:
    size = output.stat().st_size
    if size >= 1024 * 1024:
        print(f"Built: {output}\n  Size: {size / (1024 * 1024):.1f} MB")
    else:
        print(f"Built: {output}\n  Size: {size / 1024:.0f} KB")


def build_profile(profile: Profile, version: str, output: Path) -> None:
    """Build the named profile into *output*."""
    del version  # version travels via shared/_build_info.py, not the recipe.
    with tempfile.TemporaryDirectory(prefix=f"zipapp-{profile.name}-") as tmp:
        stage = Path(tmp)
        _copy_concern_packages(stage, profile)
        _pip_install_pure(stage, profile.pip_packages)
        _write_entry_point(stage, profile)
        _create_archive(stage, str(output))
    _report(output)


# ---------------------------------------------------------------------------
# Profile registry
# ---------------------------------------------------------------------------

# Every CLI / server / analysis zipapp needs the full bottom-of-stack
# (shared + compiler + dialects + analyser) plus a varying top end.
_BASE_BACKEND = ("shared", "compiler", "dialects", "analyser")

# ``tomli`` is the Python < 3.11 fallback for the stdlib ``tomllib``.  Shipped
# modules in ``dialects`` (F5 bigip trailer / redact-map) and ``tooling`` (F5
# remote auth) do ``import tomllib`` on 3.11+ else ``import tomli as tomllib``;
# every profile bundles ``dialects`` (and ``tooling``), so every profile needs
# it.  It is not a transitive dep of any other bundled package, so it must be
# listed explicitly.  Bundling it unconditionally is safe: a 3.11+ *run* host
# uses the stdlib ``tomllib`` and never touches this copy.  Mirrors the
# ``tomli>=2.0; python_version < "3.11"`` runtime dep in pyproject.toml.
_TOMLI_FALLBACK: tuple[str, ...] = ("tomli>=2.0",)

PROFILES: dict[str, Profile] = {
    "f5": Profile(
        name="f5",
        packages=_BASE_BACKEND + ("server", "tooling"),
        pip_packages=_TOMLI_FALLBACK + ("argcomplete>=3.0",),
        entry_module="tooling.f5.main:main",
    ),
    "tcl": Profile(
        name="tcl",
        # The unified `tcl` CLI runs both the analyser stack and the VM,
        # so it ships everything except ai/.
        packages=_BASE_BACKEND + ("server", "tooling"),
        pip_packages=_TOMLI_FALLBACK + ("argcomplete>=3.0",),
        entry_script=ROOT / "scripts" / "zipapp-main" / "tcl.py",
    ),
    "lsp": Profile(
        name="lsp",
        # server/ uses tooling/ for refactorings, formatter, codemods,
        # tclpkg — those have to ship with the LSP zipapp.
        packages=_BASE_BACKEND + ("server", "tooling"),
        pip_packages=_TOMLI_FALLBACK + ("pygls>=2.0", "lsprotocol>=2024.0.0"),
        entry_script=ROOT / "scripts" / "zipapp-main" / "lsp.py",
    ),
    "ai": Profile(
        name="ai",
        packages=_BASE_BACKEND + ("server", "tooling", "ai"),
        pip_packages=_TOMLI_FALLBACK + ("jinja2>=3.1",),
        entry_script=ROOT / "scripts" / "zipapp-main" / "ai.py",
    ),
    "mcp": Profile(
        name="mcp",
        packages=_BASE_BACKEND + ("server", "tooling", "ai"),
        pip_packages=_TOMLI_FALLBACK + ("lsprotocol>=2024.0.0", "jinja2>=3.1"),
        entry_script=ROOT / "scripts" / "zipapp-main" / "mcp.py",
    ),
}


def build_claude_skills(version: str, output: Path, ai_pyz: Path) -> None:
    """Build the claude-skills release zip: ai.pyz + prompts/ + skills/."""
    with tempfile.TemporaryDirectory(prefix="claude-skills-") as tmp:
        stage = Path(tmp) / f"tcl-lsp-claude-skills-{version}"
        stage.mkdir()
        shutil.copy2(ai_pyz, stage / "tcl-ai.pyz")
        shutil.copytree(ROOT / "ai" / "prompts", stage / "prompts")
        shutil.copytree(
            ROOT / "ai" / "claude" / "skills",
            stage / "skills",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )
        # Rewrite SKILL.md paths for the bundled distribution.
        for skill_md in stage.rglob("SKILL.md"):
            text = skill_md.read_text()
            text = text.replace(
                "uv run --no-dev python ai/claude/tcl_ai.py",
                "python3 .claude/tcl-ai.pyz",
            )
            text = text.replace("ai/prompts/", ".claude/prompts/")
            skill_md.write_text(text)
        shutil.make_archive(
            str(output).removesuffix(".zip"),
            "zip",
            root_dir=str(stage.parent),
            base_dir=stage.name,
        )
    _report(output)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description="Build zipapp for tcl-lsp")
    sub = parser.add_subparsers(dest="command", required=True)

    for name in PROFILES:
        p = sub.add_parser(name, help=f"Build the {name} zipapp")
        p.add_argument("--version", required=True)
        p.add_argument("--output", required=True, type=Path)

    skills_p = sub.add_parser("claude-skills", help="Build Claude skills release zip")
    skills_p.add_argument("--version", required=True)
    skills_p.add_argument("--output", required=True, type=Path)
    skills_p.add_argument("--ai-pyz", required=True, type=Path)

    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)

    if args.command in PROFILES:
        build_profile(PROFILES[args.command], args.version, args.output)
    elif args.command == "claude-skills":
        build_claude_skills(args.version, args.output, args.ai_pyz)
    else:  # pragma: no cover — argparse rejects unknown commands
        raise SystemExit(f"unknown command: {args.command!r}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
