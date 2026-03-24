#!/usr/bin/env python3
"""Generate editor settings from the central diagnostic manifest.

Reads ``core/common/diagnostic_manifest.json`` and regenerates the
diagnostic/optimisation settings blocks in:

- ``editors/vscode/package.json``  (JSON structure-aware replacement)
- ``editors/jetbrains/.../TclLspSettings.kt``  (marker comments)
- ``editors/jetbrains/.../TclLspSettingsPanel.kt``  (marker comments)

Run ``make gen-editor-settings`` to regenerate, or
``make check-editor-settings`` to verify they are up to date.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "core" / "common" / "diagnostic_manifest.json"

# ---------------------------------------------------------------------------
# VS Code section grouping
# ---------------------------------------------------------------------------

CATEGORY_TO_VSCODE_SECTION = {
    "error": "Diagnostics — Errors",
    "warning": "Diagnostics — Style & Best Practice",
    "variable": "Diagnostics — Variables",
    "security": "Diagnostics — Security",
    "hint": "Diagnostics — Hints",
    "shimmer": "Diagnostics — Shimmer",
    "taint": "Diagnostics — Taint",
    "irules": "Diagnostics — iRules",
    "irules_security": "Diagnostics — iRules",
    "irules_variable": "Diagnostics — iRules",
}

VSCODE_SECTION_ORDER = {
    "Diagnostics — Errors": 7,
    "Diagnostics — Style & Best Practice": 8,
    "Diagnostics — Variables": 9,
    "Diagnostics — Security": 10,
    "Diagnostics — Hints": 10,
    "Diagnostics — Shimmer": 11,
    "Diagnostics — Taint": 12,
    "Diagnostics — iRules": 13,
    "Optimiser": 14,
}

# Sections that are generated (will be replaced)
_GENERATED_TITLES = frozenset(VSCODE_SECTION_ORDER.keys())


def _load_manifest() -> dict:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def _short_label(code: str, description: str) -> str:
    """Generate a short checkbox label."""
    desc = description.split("—")[0].split("–")[0].strip().rstrip(".")
    if len(desc) > 55:
        desc = desc[:52] + "..."
    return f"{code}: {desc}"


# ---------------------------------------------------------------------------
# Generic marker-based replacement (for Kotlin files)
# ---------------------------------------------------------------------------


def _replace_between_markers(
    text: str,
    begin_marker: str,
    end_marker: str,
    replacement: str,
) -> str:
    """Replace content between marker lines (exclusive of the marker lines)."""
    pattern = re.compile(
        rf"(^[^\n]*{re.escape(begin_marker)}[^\n]*\n)"
        rf"(.*?)"
        rf"(^[^\n]*{re.escape(end_marker)}[^\n]*$)",
        re.MULTILINE | re.DOTALL,
    )
    match = pattern.search(text)
    if not match:
        print(f"ERROR: Marker not found: {begin_marker}", file=sys.stderr)
        sys.exit(1)
    return text[: match.start(2)] + replacement + text[match.start(3) :]


# ---------------------------------------------------------------------------
# VS Code package.json — JSON-aware replacement
# ---------------------------------------------------------------------------


def _build_vscode_diagnostic_sections(diagnostics: list[dict]) -> list[dict]:
    """Build VS Code configuration section dicts for diagnostics."""
    # Group by section
    sections: dict[str, list[dict]] = {}
    for d in diagnostics:
        section = CATEGORY_TO_VSCODE_SECTION.get(d["category"], "Diagnostics — Other")
        sections.setdefault(section, []).append(d)

    section_order = [
        "Diagnostics — Errors",
        "Diagnostics — Style & Best Practice",
        "Diagnostics — Variables",
        "Diagnostics — Security",
        "Diagnostics — Hints",
        "Diagnostics — Shimmer",
        "Diagnostics — Taint",
        "Diagnostics — iRules",
    ]

    result = []
    for title in section_order:
        diags = sections.get(title, [])
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
            props[f"tclLsp.diagnostics.{d['code']}"] = {
                "type": "boolean",
                "default": d["default"],
                "markdownDescription": f"**{d['code']}:** {d['description']}",
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
                "order": VSCODE_SECTION_ORDER[title],
                "properties": props,
            }
        )

    return result


def _build_vscode_optimiser_section(optimisations: list[dict]) -> dict:
    """Build VS Code configuration section dict for optimiser."""
    props: dict[str, dict] = {
        "tclLsp.optimiser.enabled": {
            "type": "boolean",
            "default": True,
            "description": "Enable optimiser suggestions as diagnostics.",
            "order": 0,
        }
    }
    for i, o in enumerate(optimisations, start=1):
        props[f"tclLsp.optimiser.{o['code']}"] = {
            "type": "boolean",
            "default": o["default"],
            "markdownDescription": f"**{o['code']}:** {o['description']}",
            "order": i,
        }
    return {
        "title": "Optimiser",
        "order": VSCODE_SECTION_ORDER["Optimiser"],
        "properties": props,
    }


def generate_vscode(manifest: dict, *, dry_run: bool = False) -> str | None:
    """Regenerate VS Code package.json diagnostic/optimiser settings."""
    path = ROOT / "editors" / "vscode" / "package.json"
    text = path.read_text(encoding="utf-8")
    data = json.loads(text)

    # Find the configuration array
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
    new_diag_sections = _build_vscode_diagnostic_sections(manifest["diagnostics"])
    new_opt_section = _build_vscode_optimiser_section(manifest["optimisations"])
    new_generated = new_diag_sections + [new_opt_section]

    # Reconstruct: before-generated + new-generated + after-generated
    before = config_groups[:first_gen_idx]
    # Find last generated section
    last_gen_idx = first_gen_idx
    for i in range(first_gen_idx, len(config_groups)):
        if config_groups[i].get("title") in _GENERATED_TITLES:
            last_gen_idx = i
    after = config_groups[last_gen_idx + 1 :]

    data["contributes"]["configuration"] = before + new_generated + after

    result = json.dumps(data, indent=2, ensure_ascii=False) + "\n"

    if dry_run:
        return result
    path.write_text(result, encoding="utf-8")
    return None


# ---------------------------------------------------------------------------
# JetBrains TclLspSettings.kt generation
# ---------------------------------------------------------------------------


def _gen_kt_diagnostic_vars(diagnostics: list[dict]) -> str:
    lines = []
    for d in diagnostics:
        default = "true" if d["default"] else "false"
        lines.append(f"    var diagnostic{d['code']}: Boolean = {default}")
    return "\n".join(lines) + "\n"


def _gen_kt_optimiser_vars(optimisations: list[dict]) -> str:
    lines = ["    var optimiserEnabled: Boolean = true"]
    for o in optimisations:
        default = "true" if o["default"] else "false"
        lines.append(f"    var optimiser{o['code']}: Boolean = {default}")
    return "\n".join(lines) + "\n"


def _gen_kt_diagnostic_map(diagnostics: list[dict]) -> str:
    lines = []
    for d in diagnostics:
        lines.append(f'                "{d["code"]}" to diagnostic{d["code"]},')
    return "\n".join(lines) + "\n"


def _gen_kt_optimiser_map(optimisations: list[dict]) -> str:
    lines = ['                "enabled" to optimiserEnabled,']
    for o in optimisations:
        lines.append(f'                "{o["code"]}" to optimiser{o["code"]},')
    return "\n".join(lines) + "\n"


def generate_jetbrains_settings(manifest: dict, *, dry_run: bool = False) -> str | None:
    """Regenerate TclLspSettings.kt."""
    path = (
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
        / "TclLspSettings.kt"
    )
    text = path.read_text(encoding="utf-8")

    text = _replace_between_markers(
        text,
        "@generated:diagnostic-vars:begin",
        "@generated:diagnostic-vars:end",
        _gen_kt_diagnostic_vars(manifest["diagnostics"]),
    )
    text = _replace_between_markers(
        text,
        "@generated:optimiser-vars:begin",
        "@generated:optimiser-vars:end",
        _gen_kt_optimiser_vars(manifest["optimisations"]),
    )
    text = _replace_between_markers(
        text,
        "@generated:diagnostic-map:begin",
        "@generated:diagnostic-map:end",
        _gen_kt_diagnostic_map(manifest["diagnostics"]),
    )
    text = _replace_between_markers(
        text,
        "@generated:optimiser-map:begin",
        "@generated:optimiser-map:end",
        _gen_kt_optimiser_map(manifest["optimisations"]),
    )

    if dry_run:
        return text
    path.write_text(text, encoding="utf-8")
    return None


# ---------------------------------------------------------------------------
# JetBrains TclLspSettingsPanel.kt generation
# ---------------------------------------------------------------------------

_JB_PANEL_SECTIONS = [
    ("error", "Diagnostics — Errors"),
    ("warning", "Diagnostics — Warnings"),
    ("variable", "Diagnostics — Variables"),
    ("security", "Diagnostics — Security"),
    ("hint", "Diagnostics — Hints"),
    ("shimmer", "Diagnostics — Shimmer"),
    ("taint", "Diagnostics — Taint"),
    ("irules", "Diagnostics — iRules"),
    ("irules_security", "Diagnostics — iRules"),
    ("irules_variable", "Diagnostics — iRules"),
]


def _group_diags(diagnostics: list[dict]) -> dict[str, list[dict]]:
    groups: dict[str, list[dict]] = {}
    for d in diagnostics:
        for cat_name, section_title in _JB_PANEL_SECTIONS:
            if d["category"] == cat_name:
                groups.setdefault(section_title, []).append(d)
                break
    return groups


_SECTION_ORDER = [
    "Diagnostics — Errors",
    "Diagnostics — Warnings",
    "Diagnostics — Variables",
    "Diagnostics — Security",
    "Diagnostics — Hints",
    "Diagnostics — Shimmer",
    "Diagnostics — Taint",
    "Diagnostics — iRules",
]

_PANEL_NAMES = {
    "Diagnostics — Errors": "diagErrorPanel",
    "Diagnostics — Warnings": "diagWarnPanel",
    "Diagnostics — Variables": "diagVarPanel",
    "Diagnostics — Security": "diagSecPanel",
    "Diagnostics — Hints": "diagHintPanel",
    "Diagnostics — Shimmer": "diagShimmerPanel",
    "Diagnostics — Taint": "diagTaintPanel",
    "Diagnostics — iRules": "diagIRulePanel",
}


def _gen_panel_diag_checkboxes(diagnostics: list[dict]) -> str:
    groups = _group_diags(diagnostics)
    lines = []
    for title in _SECTION_ORDER:
        diags = groups.get(title, [])
        if not diags:
            continue
        lines.append(f"    // {title}")
        for d in diags:
            label = _short_label(d["code"], d["description"])
            lines.append(f'    private val diag{d["code"]} = JBCheckBox("{label}")')
        lines.append("")
    return "\n".join(lines)


def _gen_panel_opt_checkboxes(optimisations: list[dict]) -> str:
    lines = ['    private val optEnabled = JBCheckBox("Enable optimiser suggestions")']
    for o in optimisations:
        lines.append(f'    private val opt{o["code"]} = JBCheckBox("{o["code"]}")')
    return "\n".join(lines) + "\n"


def _gen_panel_diag_ui(diagnostics: list[dict]) -> str:
    groups = _group_diags(diagnostics)
    lines = []
    for title in _SECTION_ORDER:
        diags = groups.get(title, [])
        if not diags:
            continue
        pname = _PANEL_NAMES[title]
        lines.append(f'        builder.addComponent(TitledSeparator("{title}"))')
        lines.append(f"        val {pname} = JPanel(java.awt.GridLayout(0, 2, 8, 2))")
        lines.append("        listOf(")
        refs = [f"diag{d['code']}" for d in diags]
        for i in range(0, len(refs), 6):
            chunk = refs[i : i + 6]
            lines.append("            " + ", ".join(chunk) + ",")
        lines.append(f"        ).forEach {{ {pname}.add(it) }}")
        lines.append(f"        builder.addComponent({pname})")
        lines.append("")
    return "\n".join(lines)


def _gen_panel_opt_ui(optimisations: list[dict]) -> str:
    lines = [
        '        builder.addComponent(TitledSeparator("Optimiser"))',
        "        builder.addComponent(optEnabled)",
        "        val optPanel = JPanel(java.awt.GridLayout(0, 4, 8, 2))",
        "        listOf(",
    ]
    refs = [f"opt{o['code']}" for o in optimisations]
    for i in range(0, len(refs), 6):
        chunk = refs[i : i + 6]
        lines.append("            " + ", ".join(chunk) + ",")
    lines.append("        ).forEach { optPanel.add(it) }")
    lines.append("        builder.addComponent(optPanel)")
    return "\n".join(lines) + "\n"


def _gen_panel_diag_dirty(diagnostics: list[dict]) -> str:
    lines = []
    for d in diagnostics:
        c = d["code"]
        lines.append(f"            diag{c}.isSelected != s.diagnostic{c} ||")
    return "\n".join(lines) + "\n"


def _gen_panel_opt_dirty(optimisations: list[dict]) -> str:
    lines = ["            optEnabled.isSelected != s.optimiserEnabled ||"]
    for o in optimisations:
        c = o["code"]
        lines.append(f"            opt{c}.isSelected != s.optimiser{c} ||")
    return "\n".join(lines) + "\n"


def _gen_panel_diag_apply(diagnostics: list[dict]) -> str:
    lines = []
    for d in diagnostics:
        c = d["code"]
        lines.append(f"        s.diagnostic{c} = diag{c}.isSelected")
    return "\n".join(lines) + "\n"


def _gen_panel_opt_apply(optimisations: list[dict]) -> str:
    lines = ["        s.optimiserEnabled = optEnabled.isSelected"]
    for o in optimisations:
        c = o["code"]
        lines.append(f"        s.optimiser{c} = opt{c}.isSelected")
    return "\n".join(lines) + "\n"


def _gen_panel_diag_reset(diagnostics: list[dict]) -> str:
    lines = []
    for d in diagnostics:
        c = d["code"]
        lines.append(f"        diag{c}.isSelected = s.diagnostic{c}")
    return "\n".join(lines) + "\n"


def _gen_panel_opt_reset(optimisations: list[dict]) -> str:
    lines = ["        optEnabled.isSelected = s.optimiserEnabled"]
    for o in optimisations:
        c = o["code"]
        lines.append(f"        opt{c}.isSelected = s.optimiser{c}")
    return "\n".join(lines) + "\n"


def generate_jetbrains_panel(manifest: dict, *, dry_run: bool = False) -> str | None:
    """Regenerate TclLspSettingsPanel.kt."""
    path = (
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
        / "TclLspSettingsPanel.kt"
    )
    text = path.read_text(encoding="utf-8")

    diags = manifest["diagnostics"]
    opts = manifest["optimisations"]

    text = _replace_between_markers(
        text,
        "@generated:diag-checkboxes:begin",
        "@generated:diag-checkboxes:end",
        _gen_panel_diag_checkboxes(diags),
    )
    text = _replace_between_markers(
        text,
        "@generated:opt-checkboxes:begin",
        "@generated:opt-checkboxes:end",
        _gen_panel_opt_checkboxes(opts),
    )
    text = _replace_between_markers(
        text, "@generated:diag-ui:begin", "@generated:diag-ui:end", _gen_panel_diag_ui(diags)
    )
    text = _replace_between_markers(
        text, "@generated:opt-ui:begin", "@generated:opt-ui:end", _gen_panel_opt_ui(opts)
    )
    text = _replace_between_markers(
        text,
        "@generated:diag-dirty:begin",
        "@generated:diag-dirty:end",
        _gen_panel_diag_dirty(diags),
    )
    text = _replace_between_markers(
        text, "@generated:opt-dirty:begin", "@generated:opt-dirty:end", _gen_panel_opt_dirty(opts)
    )
    text = _replace_between_markers(
        text,
        "@generated:diag-apply:begin",
        "@generated:diag-apply:end",
        _gen_panel_diag_apply(diags),
    )
    text = _replace_between_markers(
        text, "@generated:opt-apply:begin", "@generated:opt-apply:end", _gen_panel_opt_apply(opts)
    )
    text = _replace_between_markers(
        text,
        "@generated:diag-reset:begin",
        "@generated:diag-reset:end",
        _gen_panel_diag_reset(diags),
    )
    text = _replace_between_markers(
        text, "@generated:opt-reset:begin", "@generated:opt-reset:end", _gen_panel_opt_reset(opts)
    )

    if dry_run:
        return text
    path.write_text(text, encoding="utf-8")
    return None


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

    manifest = _load_manifest()

    if args.check:
        stale = []
        generators = [
            ("VS Code package.json", generate_vscode, ("editors", "vscode", "package.json")),
            (
                "JetBrains TclLspSettings.kt",
                generate_jetbrains_settings,
                (
                    "editors",
                    "jetbrains",
                    "src",
                    "main",
                    "kotlin",
                    "com",
                    "tcllsp",
                    "jetbrains",
                    "settings",
                    "TclLspSettings.kt",
                ),
            ),
            (
                "JetBrains TclLspSettingsPanel.kt",
                generate_jetbrains_panel,
                (
                    "editors",
                    "jetbrains",
                    "src",
                    "main",
                    "kotlin",
                    "com",
                    "tcllsp",
                    "jetbrains",
                    "settings",
                    "TclLspSettingsPanel.kt",
                ),
            ),
        ]
        for name, gen_fn, path_parts in generators:
            path = ROOT.joinpath(*path_parts)
            current = path.read_text(encoding="utf-8")
            expected = gen_fn(manifest, dry_run=True)
            if current != expected:
                stale.append(name)
        if stale:
            print(
                f"ERROR: Generated editor settings are stale: {', '.join(stale)}", file=sys.stderr
            )
            print("Run 'make gen-editor-settings' to regenerate.", file=sys.stderr)
            sys.exit(1)
        print("Generated editor settings are up to date.")
    else:
        generate_vscode(manifest)
        generate_jetbrains_settings(manifest)
        generate_jetbrains_panel(manifest)
        print("Regenerated editor settings from diagnostic manifest.")


if __name__ == "__main__":
    main()
