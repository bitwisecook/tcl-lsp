"""Build a zipapp (.pyz) for Tcl tools, explorer variants, LSP, AI, or MCP.

Usage:
    python scripts/build_zipapp.py tcl     --version VERSION --output PATH
    python scripts/build_zipapp.py cli     --version VERSION --output PATH
    python scripts/build_zipapp.py gui     --version VERSION --output PATH --static-dir PATH
    python scripts/build_zipapp.py gui-cdn --version VERSION --output PATH --static-dir PATH
    python scripts/build_zipapp.py lsp     --version VERSION --output PATH
    python scripts/build_zipapp.py ai      --version VERSION --output PATH
    python scripts/build_zipapp.py mcp     --version VERSION --output PATH
    python scripts/build_zipapp.py wasm    --version VERSION --output PATH
    python scripts/build_zipapp.py claude-skills --version VERSION --output PATH --ai-pyz PATH
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import zipapp
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _create_archive(source: Path, target: str, interpreter: str) -> None:
    """Create a compressed .pyz archive."""
    zipapp.create_archive(
        source=source,
        target=target,
        interpreter=interpreter,
        compressed=True,
    )


def _pip_install_pure(stage: Path, *packages: str) -> None:
    """Install pure-Python packages into the staging directory.

    Strips C extensions (.so/.pyd), dist-info, and __pycache__ so the
    result can be packed into a zipapp.
    """
    subprocess.check_call(
        [
            "uv",
            "pip",
            "install",
            "--target",
            str(stage),
            "--quiet",
            *packages,
        ]
    )
    # Remove C extensions (markupsafe._speedups etc.) — pure-Python fallback works
    for ext in stage.rglob("*.so"):
        ext.unlink()
    for ext in stage.rglob("*.pyd"):
        ext.unlink()
    # Clean up metadata and bytecache
    for p in stage.rglob("*.dist-info"):
        shutil.rmtree(p)
    for p in stage.rglob("__pycache__"):
        shutil.rmtree(p)


def build_cli(version: str, output: Path) -> None:
    """Build the CLI zipapp: core/ + lsp/ + explorer/ packages."""
    with tempfile.TemporaryDirectory(prefix="zipapp-cli-") as tmp:
        stage = Path(tmp)

        # Copy core/ package (entire tree, excluding __pycache__)
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy lsp/ package (for build metadata and shared runtime modules)
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy explorer/ package (excluding static/)
        shutil.copytree(
            ROOT / "explorer",
            stage / "explorer",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "static"),
        )

        # Create __main__.py in staging dir (zipapp entry point)
        main_py = stage / "__main__.py"
        main_py.write_text(
            '"""CLI zipapp entry point."""\n'
            "import sys\n"
            "from explorer.cli import main\n"
            "sys.exit(main())\n"
        )

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_f5(version: str, output: Path) -> None:
    """Build the F5 BIG-IP CLI zipapp.

    Ships ``core/`` (which carries ``core/bigip/`` plus
    ``core/bigip/data/irules_object_refs_graph.json``), ``lsp/`` (build
    metadata + shared utilities), and ``explorer/`` (verb registry +
    ``explorer.f5_cli`` entry point).  The zipapp's ``__main__.py``
    dispatches to :func:`explorer.f5_cli.main`, so the produced
    ``f5-<version>.pyz`` runs as a single-file ``f5 cleanup …`` tool
    on any host with Python 3.10+.
    """
    with tempfile.TemporaryDirectory(prefix="zipapp-f5-") as tmp:
        stage = Path(tmp)

        # Copy core/ — includes core/bigip/data/*.json data assets.
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy lsp/ for build metadata (lsp._build_info) and shared utils.
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy explorer/ (verb registry + explorer.f5_cli) without static/.
        shutil.copytree(
            ROOT / "explorer",
            stage / "explorer",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "static"),
        )

        # Single-file entry point — dispatch into explorer.f5_cli.main().
        main_py = stage / "__main__.py"
        main_py.write_text(
            '"""F5 BIG-IP CLI zipapp entry point."""\n'
            "import sys\n"
            "from explorer.f5_cli import main\n"
            "sys.exit(main())\n"
        )

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_tcl(version: str, output: Path) -> None:
    """Build the unified Tcl zipapp: core/ + explorer/ + lsp/ + vm/ + tclpkg/."""
    with tempfile.TemporaryDirectory(prefix="zipapp-tcl-") as tmp:
        stage = Path(tmp)

        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        shutil.copytree(
            ROOT / "explorer",
            stage / "explorer",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "static"),
        )

        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        shutil.copytree(
            ROOT / "vm",
            stage / "vm",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        shutil.copytree(
            ROOT / "tclpkg",
            stage / "tclpkg",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        shutil.copy2(ROOT / "scripts" / "zipapp_tcl_main.py", stage / "__main__.py")
        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_gui(version: str, output: Path, static_dir: Path) -> None:
    """Build the GUI zipapp: static files served from zip."""
    if not (static_dir / "index.html").exists():
        print(f"error: {static_dir}/index.html not found", file=sys.stderr)
        print("Run 'make explorer-build' first.", file=sys.stderr)
        sys.exit(1)

    if not (static_dir / "pyodide" / "pyodide.js").exists():
        print(f"error: {static_dir}/pyodide/pyodide.js not found", file=sys.stderr)
        print("Run 'make explorer-build' first.", file=sys.stderr)
        sys.exit(1)

    with tempfile.TemporaryDirectory(prefix="zipapp-gui-") as tmp:
        stage = Path(tmp)

        # Copy the GUI entry-point script as __main__.py
        shutil.copy2(ROOT / "scripts" / "zipapp_gui_main.py", stage / "__main__.py")

        # Copy all static files into static/ subdirectory within the zip
        shutil.copytree(static_dir, stage / "static")

        _create_archive(stage, str(output), "/usr/bin/env python3")

    size_mb = output.stat().st_size / (1024 * 1024)
    print(f"Built: {output}")
    print(f"  Size: {size_mb:.1f} MB")


def build_gui_cdn(version: str, output: Path, static_dir: Path) -> None:
    """Build the CDN GUI zipapp: lightweight, loads Pyodide from CDN."""
    if not (static_dir / "index.html").exists():
        print(f"error: {static_dir}/index.html not found", file=sys.stderr)
        print("Run 'make explorer-build-cdn' first.", file=sys.stderr)
        sys.exit(1)

    if not (static_dir / "worker.js").exists():
        print(f"error: {static_dir}/worker.js not found", file=sys.stderr)
        print("Run 'make explorer-build-cdn' first.", file=sys.stderr)
        sys.exit(1)

    with tempfile.TemporaryDirectory(prefix="zipapp-gui-cdn-") as tmp:
        stage = Path(tmp)

        # Copy the GUI entry-point script as __main__.py
        shutil.copy2(ROOT / "scripts" / "zipapp_gui_main.py", stage / "__main__.py")

        # Copy all CDN static files into static/ subdirectory within the zip
        shutil.copytree(static_dir, stage / "static")

        _create_archive(stage, str(output), "/usr/bin/env python3")

    size_kb = output.stat().st_size / 1024
    print(f"Built: {output}")
    print(f"  Size: {size_kb:.0f} KB")


def build_lsp(version: str, output: Path) -> None:
    """Build the LSP server zipapp: lsp/ + core/ + pygls + lsprotocol bundled."""
    with tempfile.TemporaryDirectory(prefix="zipapp-lsp-") as tmp:
        stage = Path(tmp)

        # Copy lsp/ package
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy core/ package (compiler/parser/analysis core)
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy explorer/ package (needed by lsp.server imports)
        shutil.copytree(
            ROOT / "explorer",
            stage / "explorer",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "static"),
        )

        # Install pygls + lsprotocol (and their deps) into the staging dir
        subprocess.check_call(
            [
                "uv",
                "pip",
                "install",
                "--target",
                str(stage),
                "--quiet",
                "pygls>=2.0",
                "lsprotocol>=2024.0.0",
            ]
        )

        # Remove unnecessary dist-info dirs and __pycache__ from pip install
        for p in stage.rglob("*.dist-info"):
            shutil.rmtree(p)
        for p in stage.rglob("__pycache__"):
            shutil.rmtree(p)

        # Copy the LSP entry-point script as __main__.py
        shutil.copy2(ROOT / "scripts" / "zipapp_lsp_main.py", stage / "__main__.py")

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_ai(version: str, output: Path) -> None:
    """Build the AI analysis zipapp: core/ + lsp/ + ai/ + jinja2 bundled."""
    with tempfile.TemporaryDirectory(prefix="zipapp-ai-") as tmp:
        stage = Path(tmp)

        # Copy core/ package (analysis, compiler, registry — all pure Python)
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy lsp/ package (build metadata + shared feature modules)
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy ai/ package (tcl_ai.py entry point)
        shutil.copytree(
            ROOT / "ai",
            stage / "ai",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Install jinja2 (needed by ai.claude.templates for test generation)
        _pip_install_pure(stage, "jinja2>=3.1")

        # Copy the AI entry-point script as __main__.py
        shutil.copy2(ROOT / "scripts" / "zipapp_ai_main.py", stage / "__main__.py")

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_mcp(version: str, output: Path) -> None:
    """Build the MCP server zipapp: lsp/ + core/ + ai/ + lsprotocol bundled.

    The MCP protocol layer is implemented directly (no heavy SDK), so the
    only pip dependency is lsprotocol (needed by lsp.features.*).
    """
    with tempfile.TemporaryDirectory(prefix="zipapp-mcp-") as tmp:
        stage = Path(tmp)

        # Copy lsp/ package (LSP feature/runtime surface)
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy core/ package (analysis/compiler/registry core)
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy ai/ package (MCP server module)
        shutil.copytree(
            ROOT / "ai",
            stage / "ai",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Install pip dependencies
        _pip_install_pure(stage, "lsprotocol>=2024.0.0", "jinja2>=3.1")

        # Copy the MCP entry-point script as __main__.py
        shutil.copy2(ROOT / "scripts" / "zipapp_mcp_main.py", stage / "__main__.py")

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_wasm(version: str, output: Path) -> None:
    """Build the WASM compiler zipapp: core/ + explorer/ (wasm_cli)."""
    with tempfile.TemporaryDirectory(prefix="zipapp-wasm-") as tmp:
        stage = Path(tmp)

        # Copy core/ package
        shutil.copytree(
            ROOT / "core",
            stage / "core",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Copy explorer/ package (only wasm_cli.py needed, but include all)
        shutil.copytree(
            ROOT / "explorer",
            stage / "explorer",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "static"),
        )

        # Copy lsp/ package (for build metadata)
        shutil.copytree(
            ROOT / "lsp",
            stage / "lsp",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Create __main__.py
        main_py = stage / "__main__.py"
        main_py.write_text(
            '"""WASM compiler zipapp entry point."""\n'
            "import sys\n"
            "from explorer.wasm_cli import main\n"
            "sys.exit(main())\n"
        )

        _create_archive(stage, str(output), "/usr/bin/env python3")

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def build_claude_skills(version: str, output: Path, ai_pyz: Path) -> None:
    """Build the claude_skills zip: tcl-ai.pyz + prompts/ + skills/."""
    with tempfile.TemporaryDirectory(prefix="claude-skills-") as tmp:
        stage = Path(tmp) / f"tcl-lsp-claude-skills-{version}"
        stage.mkdir()

        # Copy the pre-built .pyz with a stable name (no version)
        shutil.copy2(ai_pyz, stage / "tcl-ai.pyz")

        # Copy prompts and skills
        shutil.copytree(
            ROOT / "ai" / "prompts",
            stage / "prompts",
        )
        shutil.copytree(
            ROOT / "ai" / "claude" / "skills",
            stage / "skills",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )

        # Rewrite paths in SKILL.md files for distribution:
        # - tcl_ai.py invocation → .pyz invocation
        # - source prompt paths → installed prompt paths
        for skill_md in stage.rglob("SKILL.md"):
            text = skill_md.read_text()
            text = text.replace(
                "uv run --no-dev python ai/claude/tcl_ai.py",
                "python3 .claude/tcl-ai.pyz",
            )
            text = text.replace(
                "ai/prompts/",
                ".claude/prompts/",
            )
            skill_md.write_text(text)

        # Create the zip
        shutil.make_archive(
            str(output).removesuffix(".zip"),
            "zip",
            root_dir=str(stage.parent),
            base_dir=stage.name,
        )

    print(f"Built: {output}")
    print(f"  Size: {output.stat().st_size / 1024:.0f} KB")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build zipapp for tcl-lsp")
    sub = parser.add_subparsers(dest="command", required=True)

    tcl_p = sub.add_parser("tcl", help="Build unified Tcl tool zipapp")
    tcl_p.add_argument("--version", required=True)
    tcl_p.add_argument("--output", required=True, type=Path)

    cli_p = sub.add_parser("cli", help="Build CLI zipapp")
    cli_p.add_argument("--version", required=True)
    cli_p.add_argument("--output", required=True, type=Path)

    f5_p = sub.add_parser("f5", help="Build F5 BIG-IP CLI zipapp")
    f5_p.add_argument("--version", required=True)
    f5_p.add_argument("--output", required=True, type=Path)

    gui_p = sub.add_parser("gui", help="Build GUI zipapp (standalone)")
    gui_p.add_argument("--version", required=True)
    gui_p.add_argument("--output", required=True, type=Path)
    gui_p.add_argument("--static-dir", required=True, type=Path)

    cdn_p = sub.add_parser("gui-cdn", help="Build GUI zipapp (CDN)")
    cdn_p.add_argument("--version", required=True)
    cdn_p.add_argument("--output", required=True, type=Path)
    cdn_p.add_argument("--static-dir", required=True, type=Path)

    lsp_p = sub.add_parser("lsp", help="Build LSP server zipapp")
    lsp_p.add_argument("--version", required=True)
    lsp_p.add_argument("--output", required=True, type=Path)

    ai_p = sub.add_parser("ai", help="Build AI analysis zipapp")
    ai_p.add_argument("--version", required=True)
    ai_p.add_argument("--output", required=True, type=Path)

    mcp_p = sub.add_parser("mcp", help="Build MCP server zipapp")
    mcp_p.add_argument("--version", required=True)
    mcp_p.add_argument("--output", required=True, type=Path)

    wasm_p = sub.add_parser("wasm", help="Build WASM compiler zipapp")
    wasm_p.add_argument("--version", required=True)
    wasm_p.add_argument("--output", required=True, type=Path)

    skills_p = sub.add_parser("claude-skills", help="Build Claude skills release zip")
    skills_p.add_argument("--version", required=True)
    skills_p.add_argument("--output", required=True, type=Path)
    skills_p.add_argument("--ai-pyz", required=True, type=Path)

    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)

    if args.command == "tcl":
        build_tcl(args.version, args.output)
    elif args.command == "cli":
        build_cli(args.version, args.output)
    elif args.command == "f5":
        build_f5(args.version, args.output)
    elif args.command == "gui":
        build_gui(args.version, args.output, args.static_dir)
    elif args.command == "gui-cdn":
        build_gui_cdn(args.version, args.output, args.static_dir)
    elif args.command == "lsp":
        build_lsp(args.version, args.output)
    elif args.command == "ai":
        build_ai(args.version, args.output)
    elif args.command == "mcp":
        build_mcp(args.version, args.output)
    elif args.command == "wasm":
        build_wasm(args.version, args.output)
    elif args.command == "claude-skills":
        build_claude_skills(args.version, args.output, args.ai_pyz)

    return 0


if __name__ == "__main__":
    sys.exit(main())
