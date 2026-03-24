#!/usr/bin/env python3
"""Generate editor settings from the self-registering code registry.

Imports the compiler's code registry and generates:

- ``editors/jetbrains/.../generated/DiagnosticCatalog.kt``  (Kotlin data)
- ``editors/vscode/src/generated/diagnosticCatalog.ts``     (TypeScript data)
- ``editors/vscode/package.json``  (VS Code configuration sections)
- ``docs/generated/diagnostic_tables.md``  (README-includable tables)

Run ``make gen-editor-settings`` to regenerate, or
``make check-editor-settings`` to verify they are up to date.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import jinja2

ROOT = Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# Optional formatter integration
# ---------------------------------------------------------------------------


def _format_typescript(content: str) -> str:
    """Run prettier on TypeScript content if available."""
    vscode_dir = ROOT / "editors" / "vscode"
    prettier = vscode_dir / "node_modules" / ".bin" / "prettier"
    if not prettier.exists():
        prettier_path = shutil.which("prettier")
        if not prettier_path:
            return content
        prettier = Path(prettier_path)
    try:
        result = subprocess.run(
            [str(prettier), "--parser", "typescript"],
            input=content,
            capture_output=True,
            text=True,
            cwd=str(vscode_dir),
            timeout=30,
        )
        if result.returncode == 0:
            return result.stdout
    except (subprocess.TimeoutExpired, OSError):
        pass
    return content


def _format_kotlin(content: str) -> str:
    """Run ktfmt on Kotlin content if available."""
    ktfmt = shutil.which("ktfmt")
    if not ktfmt:
        return content
    try:
        result = subprocess.run(
            [ktfmt, "--kotlinlang-style", "-"],
            input=content,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 0:
            return result.stdout
    except (subprocess.TimeoutExpired, OSError):
        pass
    return content


# ---------------------------------------------------------------------------
# Import registry (triggers all code registrations)
# ---------------------------------------------------------------------------

# Ensure core/ is importable
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

import core.common.codes_all  # noqa: F401, E402
from core.common.codes import (  # noqa: E402
    SECTION_KEYS,
    SECTIONS,
    codes_by_section,
    diagnostics_sorted,
    optimisations_sorted,
)

# ---------------------------------------------------------------------------
# Jinja2 environment
# ---------------------------------------------------------------------------


def _jinja_env(template_dir: Path) -> jinja2.Environment:
    """Create a Jinja2 environment rooted at *template_dir*."""
    return jinja2.Environment(
        loader=jinja2.FileSystemLoader(str(template_dir)),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
    )


def _short_label(code: str, description: str, *, escape_kotlin: bool = False) -> str:
    """Generate a short checkbox label from code + description."""
    desc = description.split("—")[0].split("–")[0].strip().rstrip(".")
    # Strip backtick-delimited code spans (Markdown formatting).
    desc = re.sub(r"`([^`]*)`", r"\1", desc)
    if len(desc) > 55:
        desc = desc[:52] + "..."
    label = f"{code}: {desc}"
    if escape_kotlin:
        label = label.replace("\\", "\\\\").replace('"', '\\"')
    return label


# ---------------------------------------------------------------------------
# Generate Kotlin catalog
# ---------------------------------------------------------------------------

_JB_CATALOG_PATH = (
    ROOT
    / "editors"
    / "jetbrains"
    / "src"
    / "main"
    / "kotlin"
    / "com"
    / "tcllsp"
    / "jetbrains"
    / "settings"
    / "generated"
    / "DiagnosticCatalog.kt"
)


def generate_jetbrains_catalog(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate DiagnosticCatalog.kt from registry."""
    diags = diagnostics_sorted()
    opts = optimisations_sorted()

    # Deduplicate section titles (irules/irules_security/irules_variable
    # all map to the same title)
    seen_titles: set[str] = set()
    section_titles = []
    section_order = []
    for key, title in SECTIONS:
        if title not in seen_titles:
            section_titles.append((key, title))
            section_order.append(key)
            seen_titles.add(title)

    env = _jinja_env(_JB_CATALOG_PATH.parent)
    template = env.get_template("DiagnosticCatalog.kt.j2")
    content = template.render(
        diagnostics=[
            {
                "code": d.code,
                "section": d.section,
                "label": _short_label(d.code, d.description, escape_kotlin=True),
                "default": d.default,
            }
            for d in diags
        ],
        optimisations=[
            {
                "code": o.code,
                "label": _short_label(o.code, o.description, escape_kotlin=True),
                "default": o.default,
            }
            for o in opts
        ],
        section_titles=section_titles,
        section_order=section_order,
    )
    content = _format_kotlin(content)

    if not dry_run:
        _JB_CATALOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        _JB_CATALOG_PATH.write_text(content, encoding="utf-8")
    return _JB_CATALOG_PATH, content


# ---------------------------------------------------------------------------
# Generate TypeScript catalog
# ---------------------------------------------------------------------------

_TS_CATALOG_PATH = ROOT / "editors" / "vscode" / "src" / "generated" / "diagnosticCatalog.ts"


def generate_vscode_catalog(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate diagnosticCatalog.ts from registry."""
    diags = diagnostics_sorted()
    opts = optimisations_sorted()

    # Deduplicate section titles (irules_* all map to same title)
    seen: set[str] = set()
    deduped = []
    for key, title in SECTIONS:
        if title not in seen:
            deduped.append((key, title))
            seen.add(title)

    env = _jinja_env(_TS_CATALOG_PATH.parent)
    template = env.get_template("diagnosticCatalog.ts.j2")
    content = template.render(
        diagnostics=[
            {
                "code": d.code,
                "section": d.section,
                "description": d.description,
                "default": d.default,
            }
            for d in diags
        ],
        optimisations=[
            {
                "code": o.code,
                "description": o.description,
                "default": o.default,
            }
            for o in opts
        ],
        section_titles_vscode=deduped,
        section_order=[s for s, _ in deduped],
    )
    content = _format_typescript(content)

    if not dry_run:
        _TS_CATALOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        _TS_CATALOG_PATH.write_text(content, encoding="utf-8")
    return _TS_CATALOG_PATH, content


# ---------------------------------------------------------------------------
# Generate VS Code package.json configuration sections
# ---------------------------------------------------------------------------


def _build_vscode_diagnostic_sections() -> list[dict]:
    """Build VS Code contributes.configuration section dicts for diagnostics."""
    sections_data = codes_by_section()

    # Group by title (multiple sections can share a title) and track
    # the VS Code "order" field — derived from position in SECTIONS.
    # First non-diagnostic section starts at order 7 (matching existing layout).
    _VSCODE_ORDER_BASE = 7

    title_groups: dict[str, list] = {}
    title_order: dict[str, int] = {}
    seen: set[str] = set()
    for idx, (key, title) in enumerate(SECTIONS):
        for info in sections_data.get(key, []):
            title_groups.setdefault(title, []).append(info)
        if title not in seen:
            title_order[title] = _VSCODE_ORDER_BASE + idx
            seen.add(title)

    # Ordered unique titles (preserves SECTIONS list order)
    ordered_titles = list(dict.fromkeys(title for _, title in SECTIONS))

    result = []
    for title in ordered_titles:
        diags = title_groups.get(title, [])
        if not diags:
            continue

        props: dict[str, dict] = {}

        # Special non-diagnostic properties
        if title == "Diagnostics — Style & Best Practice":
            props["tclLsp.style.lineLength"] = {
                "type": "integer",
                "default": 120,
                "minimum": 40,
                "markdownDescription": "Maximum line length for the **W111** diagnostic. Lines exceeding this limit are flagged.",
                "order": -1,
            }
        elif title == "Diagnostics — Shimmer":
            props["tclLsp.shimmer.enabled"] = {
                "type": "boolean",
                "default": True,
                "markdownDescription": "Enable shimmer detection (internal representation changes). When disabled, all S-series diagnostics are suppressed.",
                "order": -1,
            }

        for i, d in enumerate(diags):
            props[f"tclLsp.diagnostics.{d.code}"] = {
                "type": "boolean",
                "default": d.default,
                "markdownDescription": f"**{d.code}:** {d.description}",
                "order": i,
            }

        if title == "Diagnostics — iRules":
            props["tclLsp.diagnostics.genericVariablePatterns"] = {
                "type": "array",
                "items": {"type": "string"},
                "markdownDescription": (
                    "Regex patterns for generic `static::` variable names (IRULE4002). "
                    "Each pattern is matched case-insensitively against the bare name "
                    "after stripping `static::`. Also configurable via "
                    "`~/.config/tcl-lsp/config.ini`."
                ),
                "order": len(diags),
            }

        result.append(
            {
                "title": title,
                "order": title_order[title],
                "properties": props,
            }
        )

    return result


def _build_vscode_optimiser_section() -> dict:
    """Build VS Code configuration section dict for optimiser."""
    opts = optimisations_sorted()
    props: dict[str, dict] = {
        "tclLsp.optimiser.enabled": {
            "type": "boolean",
            "default": True,
            "description": "Enable optimiser suggestions as diagnostics.",
            "order": 0,
        }
    }
    for i, o in enumerate(opts, start=1):
        props[f"tclLsp.optimiser.{o.code}"] = {
            "type": "boolean",
            "default": o.default,
            "markdownDescription": f"**{o.code}:** {o.description}",
            "order": i,
        }
    return {
        "title": "Optimiser",
        "order": 14,
        "properties": props,
    }


# Titles that are generated (will be replaced in package.json)
_GENERATED_TITLES = frozenset(list(dict.fromkeys(title for _, title in SECTIONS)) + ["Optimiser"])


def generate_vscode_package_json(*, dry_run: bool = False) -> tuple[Path, str]:
    """Regenerate VS Code package.json diagnostic/optimiser settings."""
    path = ROOT / "editors" / "vscode" / "package.json"
    text = path.read_text(encoding="utf-8")
    data = json.loads(text)

    config_groups = data["contributes"]["configuration"]

    # Find insertion point: the index of the first generated section
    first_gen_idx = None
    for i, g in enumerate(config_groups):
        if g.get("title") in _GENERATED_TITLES:
            first_gen_idx = i
            break

    if first_gen_idx is None:
        print("ERROR: No generated sections found in package.json", file=sys.stderr)
        sys.exit(1)

    # Build new generated sections
    new_diag_sections = _build_vscode_diagnostic_sections()
    new_opt_section = _build_vscode_optimiser_section()
    new_generated = new_diag_sections + [new_opt_section]

    # Reconstruct: before-generated + new-generated + after-generated
    before = config_groups[:first_gen_idx]
    last_gen_idx = first_gen_idx
    for i in range(first_gen_idx, len(config_groups)):
        if config_groups[i].get("title") in _GENERATED_TITLES:
            last_gen_idx = i
    after = config_groups[last_gen_idx + 1 :]

    data["contributes"]["configuration"] = before + new_generated + after

    result = json.dumps(data, indent=2, ensure_ascii=False) + "\n"

    if not dry_run:
        path.write_text(result, encoding="utf-8")
    return path, result


# ---------------------------------------------------------------------------
# Generate README tables
# ---------------------------------------------------------------------------

_TABLES_PATH = ROOT / "docs" / "generated" / "diagnostic_tables.md"


def generate_readme_tables(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate markdown tables from registry."""
    diags = diagnostics_sorted()
    opts = optimisations_sorted()

    env = _jinja_env(_TABLES_PATH.parent)
    template = env.get_template("diagnostic_tables.md.j2")
    content = template.render(
        diagnostics=[
            {
                "code": d.code,
                "section": d.section,
                "description": d.description,
                "default": d.default,
            }
            for d in diags
        ],
        optimisations=[
            {
                "code": o.code,
                "description": o.description,
                "default": o.default,
            }
            for o in opts
        ],
    )

    if not dry_run:
        _TABLES_PATH.parent.mkdir(parents=True, exist_ok=True)
        _TABLES_PATH.write_text(content, encoding="utf-8")
    return _TABLES_PATH, content


# ---------------------------------------------------------------------------
# Render all — used by tests and --check mode
# ---------------------------------------------------------------------------


def render_all(*, dry_run: bool = False) -> list[tuple[Path, str]]:
    """Render all generated files. Returns list of (path, content) pairs."""
    return [
        generate_jetbrains_catalog(dry_run=dry_run),
        generate_vscode_catalog(dry_run=dry_run),
        generate_vscode_package_json(dry_run=dry_run),
        generate_readme_tables(dry_run=dry_run),
    ]


# ---------------------------------------------------------------------------
# Backwards-compatible API (used by existing tests during migration)
# ---------------------------------------------------------------------------


def _load_manifest() -> dict:
    """Load manifest from registry (backwards compat for existing tests)."""
    diags = diagnostics_sorted()
    opts = optimisations_sorted()
    return {
        "diagnostics": [
            {
                "code": d.code,
                "category": d.section,
                "description": d.description,
                "default": d.default,
            }
            for d in diags
        ],
        "optimisations": [
            {
                "code": o.code,
                "description": o.description,
                "default": o.default,
            }
            for o in opts
        ],
    }


# Keep old names importable during migration
_KNOWN_CATEGORIES = frozenset(SECTION_KEYS)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check that generated files are up to date (exit 1 if stale).",
    )
    args = parser.parse_args()

    if args.check:
        stale = []
        for path, expected in render_all(dry_run=True):
            if not path.exists():
                stale.append(str(path.relative_to(ROOT)))
                continue
            current = path.read_text(encoding="utf-8")
            if current != expected:
                stale.append(str(path.relative_to(ROOT)))
        if stale:
            print(
                f"ERROR: Generated editor settings are stale: {', '.join(stale)}",
                file=sys.stderr,
            )
            print("Run 'make gen-editor-settings' to regenerate.", file=sys.stderr)
            sys.exit(1)
        print("Generated editor settings are up to date.")
    else:
        results = render_all()
        for path, _ in results:
            print(f"  Generated {path.relative_to(ROOT)}")
        print("Regenerated editor settings from code registry.")


if __name__ == "__main__":
    main()
