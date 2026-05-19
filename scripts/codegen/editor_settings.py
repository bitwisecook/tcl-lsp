#!/usr/bin/env python3
"""Generate editor settings from the self-registering code registry.

Imports the compiler's code registry and generates:

- ``editors/jetbrains/.../generated/DiagnosticCatalog.kt``  (Kotlin data)
- ``editors/jetbrains/.../TclLspSettings.kt``  (@generated blocks)
- ``editors/jetbrains/.../TclLspSettingsPanel.kt``  (@generated blocks)
- ``editors/vscode/src/generated/diagnosticCatalog.ts``     (TypeScript data)
- ``editors/vscode/package.json``  (VS Code configuration sections)
- ``docs/generated/diagnostic_tables.md``  (README-includable combined tables)
- ``docs/generated/diagnostic_codes.md``   (diagnostic codes only)
- ``docs/generated/optimisation_codes.md`` (optimisation codes only)

Run ``make gen-editor-settings`` to regenerate, or
``make check-editor-settings`` to verify they are up to date.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import jinja2

ROOT = Path(__file__).resolve().parents[2]


# TypeScript formatting helpers (match prettier output without requiring it)

_TS_PRINT_WIDTH = 100


def _ts_string_literal(value: str) -> str:
    """Format a string literal using the same quoting rules as prettier."""
    if '"' in value:
        escaped = value.replace("\\", "\\\\").replace("'", "\\'")
        return f"'{escaped}'"
    return json.dumps(value, ensure_ascii=False)


def _ts_description_field(value: str, *, indent: int = 4) -> str:
    """Format a TS description field, wrapping like prettier if too long."""
    literal = _ts_string_literal(value)
    prefix = " " * indent
    one_line = f"{prefix}description: {literal},"
    if len(one_line) <= _TS_PRINT_WIDTH:
        return one_line
    return f"{prefix}description:\n{prefix}  {literal},"


# Import registry (triggers all code registrations)

# Ensure repo root is on sys.path so the concern packages are importable
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

import server._codes_init  # noqa: F401, E402
from shared.codes import (  # noqa: E402
    SECTIONS,
    ai_category_overrides,
    codes_by_section,
    conversion_map,
    diagnostics_sorted,
    optimisations_sorted,
)
from shared.optimisation_profiles import (  # noqa: E402
    READABILITY_CODES,
    OptimisationProfile,
    profile_spec,
)
from tooling.formatter.config import (  # noqa: E402
    FORMATTER_SETTINGS_CATALOGUE,
    FormatterConfig,
)

_STANDARD_CODES = profile_spec(OptimisationProfile.STANDARD).enabled_codes


def _snake_to_camel(name: str) -> str:
    """Convert snake_case to camelCase for VS Code setting keys."""
    parts = name.split("_")
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def _build_vscode_formatter_sections() -> list[dict]:
    """Build VS Code configuration sections for formatter settings.

    Reads ``FORMATTER_SETTINGS_CATALOGUE`` from ``tooling.formatter.config``
    (the single source of truth) and builds VS Code schema entries with
    defaults pulled directly from ``FormatterConfig``.
    """
    cfg = FormatterConfig()
    cfg_dict = cfg.to_dict()
    _FORMATTER_ORDER_BASE = 2

    sections = []
    for idx, section in enumerate(FORMATTER_SETTINGS_CATALOGUE):
        props: dict[str, dict] = {}
        for i, spec in enumerate(section.settings):
            camel_key = _snake_to_camel(spec.field_name)
            setting_key = f"tclLsp.formatting.{camel_key}"
            default = cfg_dict.get(spec.field_name, None)
            schema = spec.to_vscode_schema(default)
            schema["order"] = i
            props[setting_key] = schema
        sections.append(
            {
                "title": section.title,
                "order": _FORMATTER_ORDER_BASE + idx,
                "properties": props,
            }
        )
    return sections


_FORMATTER_TITLES = frozenset(s.title for s in FORMATTER_SETTINGS_CATALOGUE)

# Jinja2 environment


def _jinja_env(template_dir: Path) -> jinja2.Environment:
    """Create a Jinja2 environment rooted at *template_dir*."""
    # Editor-settings templates render JSON / XML / Kotlin / TypeScript
    # (no HTML), so HTML autoescape would corrupt string literals and
    # quote characters in the generated code.  Limit autoescape to the
    # HTML/XML extensions we don't actually emit — silences CodeQL's
    # ``py/jinja2/autoescape-false`` without breaking real templates.
    return jinja2.Environment(
        loader=jinja2.FileSystemLoader(str(template_dir)),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
        autoescape=jinja2.select_autoescape(["html", "htm", "xml"]),
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
        label = label.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$")
    return label


# Generate Kotlin catalog

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


def _code_to_kt_var(code: str) -> str:
    """Convert a diagnostic code like 'IRULE5006' to a Kotlin variable name like 'diagnosticIRULE5006'."""
    return f"diagnostic{code}"


def _code_to_kt_checkbox(code: str) -> str:
    """Convert a diagnostic code to a Kotlin checkbox field name like 'diagIRULE5006'."""
    return f"diag{code}"


def _opt_to_kt_var(code: str) -> str:
    """Convert an optimiser code like 'O100' to a Kotlin variable name like 'optimiserO100'."""
    return f"optimiser{code}"


def _opt_to_kt_checkbox(code: str) -> str:
    """Convert an optimiser code to a Kotlin checkbox field name like 'optO100'."""
    return f"opt{code}"


def _replace_generated_block(content: str, marker: str, replacement: str) -> str:
    """Replace content between ``// @generated:<marker>:begin`` and ``// @generated:<marker>:end``.

    Preserves the indentation of the begin marker line.
    """
    begin_tag = f"// @generated:{marker}:begin"
    end_tag = f"// @generated:{marker}:end"

    begin_idx = content.find(begin_tag)
    end_idx = content.find(end_tag)
    if begin_idx == -1 or end_idx == -1:
        return content

    # Find the indentation of the begin marker
    line_start = content.rfind("\n", 0, begin_idx)
    indent = content[line_start + 1 : begin_idx]

    before = content[: begin_idx + len(begin_tag)]
    after = content[end_idx:]

    return before + "\n" + replacement + indent + after


def _generate_diag_section_groups() -> list[tuple[str, list]]:
    """Group diagnostics by their deduplicated section title."""
    diags = diagnostics_sorted()
    # Deduplicate section titles
    seen: set[str] = set()
    title_for_section: dict[str, str] = {}
    ordered_titles: list[str] = []
    for key, title in SECTIONS:
        title_for_section[key] = title
        if title not in seen:
            ordered_titles.append(title)
            seen.add(title)

    # Group diagnostics by title
    groups: dict[str, list] = {}
    for d in diags:
        title = title_for_section.get(d.section, d.section)
        groups.setdefault(title, []).append(d)

    return [(title, groups.get(title, [])) for title in ordered_titles if groups.get(title)]


_JB_SETTINGS_PATH = (
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

_JB_PANEL_PATH = _JB_SETTINGS_PATH.parent / "TclLspSettingsPanel.kt"


def generate_jetbrains_settings(*, dry_run: bool = False) -> tuple[Path, str]:
    """Regenerate @generated blocks in TclLspSettings.kt."""
    content = _JB_SETTINGS_PATH.read_text(encoding="utf-8")
    diags = diagnostics_sorted()
    opts = optimisations_sorted()

    # diagnostic-vars block
    vars_lines = []
    for d in diags:
        default = "true" if d.default else "false"
        vars_lines.append(f"    var {_code_to_kt_var(d.code)}: Boolean = {default}\n")
    content = _replace_generated_block(content, "diagnostic-vars", "".join(vars_lines))

    # diagnostic-map block
    map_lines = []
    for d in diags:
        map_lines.append(f'                "{d.code}" to {_code_to_kt_var(d.code)},\n')
    content = _replace_generated_block(content, "diagnostic-map", "".join(map_lines))

    # optimiser-vars block
    opt_var_lines = ["    var optimiserEnabled: Boolean = true\n"]
    for o in opts:
        default = "true" if o.default else "false"
        opt_var_lines.append(f"    var {_opt_to_kt_var(o.code)}: Boolean = {default}\n")
    content = _replace_generated_block(content, "optimiser-vars", "".join(opt_var_lines))

    # optimiser-map block
    opt_map_lines = ['                "enabled" to optimiserEnabled,\n']
    for o in opts:
        opt_map_lines.append(f'                "{o.code}" to {_opt_to_kt_var(o.code)},\n')
    content = _replace_generated_block(content, "optimiser-map", "".join(opt_map_lines))

    if not dry_run:
        _JB_SETTINGS_PATH.write_text(content, encoding="utf-8")
    return _JB_SETTINGS_PATH, content


def generate_jetbrains_panel(*, dry_run: bool = False) -> tuple[Path, str]:
    """Regenerate @generated blocks in TclLspSettingsPanel.kt."""
    content = _JB_PANEL_PATH.read_text(encoding="utf-8")
    diags = diagnostics_sorted()
    opts = optimisations_sorted()
    groups = _generate_diag_section_groups()

    # diag-checkboxes block
    cb_lines = []
    for title, section_diags in groups:
        cb_lines.append(f"    // {title}\n")
        for d in section_diags:
            label = _short_label(d.code, d.description, escape_kotlin=True)
            cb_lines.append(
                f'    private val {_code_to_kt_checkbox(d.code)} = JBCheckBox("{label}")\n'
            )
        cb_lines.append("\n")
    # Remove trailing blank line
    if cb_lines and cb_lines[-1] == "\n":
        cb_lines.pop()
    content = _replace_generated_block(content, "diag-checkboxes", "".join(cb_lines))

    # diag-ui block
    ui_lines = []
    panel_var_names = {
        "Diagnostics — Errors": "diagErrorPanel",
        "Diagnostics — Style & Best Practice": "diagWarnPanel",
        "Diagnostics — Variables": "diagVarPanel",
        "Diagnostics — Security": "diagSecPanel",
        "Diagnostics — Hints": "diagHintPanel",
        "Diagnostics — Shimmer": "diagShimmerPanel",
        "Diagnostics — Taint": "diagTaintPanel",
        "Diagnostics — iRules": "diagIRulePanel",
    }
    for title, section_diags in groups:
        panel_var = panel_var_names.get(title, "diagPanel")
        ui_lines.append(f'        builder.addComponent(TitledSeparator("{title}"))\n')
        ui_lines.append(f"        val {panel_var} = JPanel(java.awt.GridLayout(0, 2, 8, 2))\n")
        ui_lines.append("        listOf(\n")
        # Format checkbox references, 6 per line
        refs = [_code_to_kt_checkbox(d.code) for d in section_diags]
        for i in range(0, len(refs), 6):
            chunk = refs[i : i + 6]
            ui_lines.append(f"            {', '.join(chunk)},\n")
        ui_lines.append(f"        ).forEach {{ {panel_var}.add(it) }}\n")
        ui_lines.append(f"        builder.addComponent({panel_var})\n")
        ui_lines.append("\n")
    if ui_lines and ui_lines[-1] == "\n":
        ui_lines.pop()
    content = _replace_generated_block(content, "diag-ui", "".join(ui_lines))

    # diag-dirty block
    dirty_lines = []
    for d in diags:
        cb = _code_to_kt_checkbox(d.code)
        var = _code_to_kt_var(d.code)
        dirty_lines.append(f"            {cb}.isSelected != s.{var} ||\n")
    content = _replace_generated_block(content, "diag-dirty", "".join(dirty_lines))

    # diag-apply block
    apply_lines = []
    for d in diags:
        cb = _code_to_kt_checkbox(d.code)
        var = _code_to_kt_var(d.code)
        apply_lines.append(f"        s.{var} = {cb}.isSelected\n")
    content = _replace_generated_block(content, "diag-apply", "".join(apply_lines))

    # diag-reset block
    reset_lines = []
    for d in diags:
        cb = _code_to_kt_checkbox(d.code)
        var = _code_to_kt_var(d.code)
        reset_lines.append(f"        {cb}.isSelected = s.{var}\n")
    content = _replace_generated_block(content, "diag-reset", "".join(reset_lines))

    # opt-checkboxes block
    opt_cb_lines = ['    private val optEnabled = JBCheckBox("Enable optimiser suggestions")\n']
    for o in opts:
        label = _short_label(o.code, o.description, escape_kotlin=True)
        opt_cb_lines.append(
            f'    private val {_opt_to_kt_checkbox(o.code)} = JBCheckBox("{label}")\n'
        )
    content = _replace_generated_block(content, "opt-checkboxes", "".join(opt_cb_lines))

    # opt-ui block
    opt_ui_lines = [
        '        builder.addComponent(TitledSeparator("Optimiser"))\n',
        "        builder.addComponent(optEnabled)\n",
        "        val optPanel = JPanel(java.awt.GridLayout(0, 4, 8, 2))\n",
        "        listOf(\n",
    ]
    opt_refs = [_opt_to_kt_checkbox(o.code) for o in opts]
    for i in range(0, len(opt_refs), 6):
        chunk = opt_refs[i : i + 6]
        opt_ui_lines.append(f"            {', '.join(chunk)},\n")
    opt_ui_lines.append("        ).forEach { optPanel.add(it) }\n")
    opt_ui_lines.append("        builder.addComponent(optPanel)\n")
    content = _replace_generated_block(content, "opt-ui", "".join(opt_ui_lines))

    # opt-dirty block
    opt_dirty_lines = ["            optEnabled.isSelected != s.optimiserEnabled ||\n"]
    for o in opts:
        cb = _opt_to_kt_checkbox(o.code)
        var = _opt_to_kt_var(o.code)
        opt_dirty_lines.append(f"            {cb}.isSelected != s.{var} ||\n")
    content = _replace_generated_block(content, "opt-dirty", "".join(opt_dirty_lines))

    # opt-apply block
    opt_apply_lines = ["        s.optimiserEnabled = optEnabled.isSelected\n"]
    for o in opts:
        cb = _opt_to_kt_checkbox(o.code)
        var = _opt_to_kt_var(o.code)
        opt_apply_lines.append(f"        s.{var} = {cb}.isSelected\n")
    content = _replace_generated_block(content, "opt-apply", "".join(opt_apply_lines))

    # opt-reset block
    opt_reset_lines = ["        optEnabled.isSelected = s.optimiserEnabled\n"]
    for o in opts:
        cb = _opt_to_kt_checkbox(o.code)
        var = _opt_to_kt_var(o.code)
        opt_reset_lines.append(f"        {cb}.isSelected = s.{var}\n")
    content = _replace_generated_block(content, "opt-reset", "".join(opt_reset_lines))

    if not dry_run:
        _JB_PANEL_PATH.write_text(content, encoding="utf-8")
    return _JB_PANEL_PATH, content


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

    if not dry_run:
        _JB_CATALOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        _JB_CATALOG_PATH.write_text(content, encoding="utf-8")
    return _JB_CATALOG_PATH, content


# Generate TypeScript catalog

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
                "description_line": _ts_description_field(d.description),
                "default": d.default,
            }
            for d in diags
        ],
        optimisations=[
            {
                "code": o.code,
                "description_line": _ts_description_field(o.description),
                "default": o.default,
            }
            for o in opts
        ],
        section_titles_vscode=deduped,
        section_order=[s for s, _ in deduped],
    )

    if not dry_run:
        _TS_CATALOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        _TS_CATALOG_PATH.write_text(content, encoding="utf-8")
    return _TS_CATALOG_PATH, content


# Generate VS Code package.json configuration sections


def _build_vscode_diagnostic_sections() -> list[dict]:
    """Build VS Code contributes.configuration section dicts for diagnostics."""
    sections_data = codes_by_section()

    # Group by title (multiple sections can share a title) and track
    # the VS Code "order" field — derived from position in SECTIONS.
    # Diagnostic sections start after formatter sections.
    _VSCODE_ORDER_BASE = 2 + len(FORMATTER_SETTINGS_CATALOGUE)

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
                "order": -2,
            }
            props["tclLsp.style.nonAscii"] = {
                "type": "string",
                "default": "confusables",
                "enum": ["strict", "confusables", "common", "off"],
                "enumDescriptions": [
                    "Flag every non-ASCII character (most restrictive). Default for iRules/iApps.",
                    "Flag Unicode confusables (homoglyphs per unicode.org/Public/security confusables.txt) plus copy-paste artifacts. Default for Tcl.",
                    "Allow intentional Unicode (letters, digits, symbols in any script); only flag confusables and control characters.",
                    "Disable W108 entirely.",
                ],
                "markdownDescription": (
                    "Controls how **W108** (non-ASCII character) is detected.\n\n"
                    "- **strict**: flag all non-ASCII characters (default for iRules/iApps).\n"
                    "- **confusables** *(default for Tcl)*: flag Unicode confusables (homoglyphs from the Unicode security spec) and copy-paste artifacts.\n"
                    "- **common**: allow Unicode letters, symbols, and punctuation; flag only confusables and control characters.\n"
                    "- **off**: disable W108."
                ),
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
                "default": [],
                "markdownDescription": (
                    "Regex patterns for generic `static::` variable names (IRULE4002). "
                    "Each pattern is matched case-insensitively against the bare name "
                    "after stripping `static::`. Also configurable via "
                    "the tcl-lsp config file (see docs/kcs/kcs-xdg-config.md)."
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


def _build_vscode_optimiser_section(*, order: int) -> dict:
    """Build VS Code configuration section dict for optimiser."""
    opts = optimisations_sorted()
    props: dict[str, dict] = {
        "tclLsp.optimiser.enabled": {
            "type": "boolean",
            "default": True,
            "description": "Enable optimiser suggestions as diagnostics.",
            "order": 0,
        },
        "tclLsp.optimiser.profile": {
            "type": "string",
            "enum": ["off", "readability", "standard", "full", "aggressive"],
            "default": "readability",
            "enumDescriptions": [
                "All optimisations disabled.",
                "Readability improvements only — idiomatic rewrites, no code removal.",
                "Readability + constant folding and pattern recognition.",
                "All optimisations enabled (single pass).",
                "All optimisations with multi-pass to fixpoint.",
            ],
            "markdownDescription": (
                "Optimisation profile controlling which passes run as diagnostics. "
                "Individual `O1xx` toggles below override the profile when explicitly set."
            ),
            "order": 1,
        },
    }
    for i, o in enumerate(opts, start=2):
        props[f"tclLsp.optimiser.{o.code}"] = {
            "type": ["boolean", "null"],
            "default": None,
            "markdownDescription": (
                f"**{o.code}:** {o.description} (`null` = inherit from profile)"
            ),
            "order": i,
        }
    return {
        "title": "Optimiser",
        "order": order,
        "properties": props,
    }


# Titles that are generated (will be replaced in package.json)
_GENERATED_TITLES = (
    frozenset(list(dict.fromkeys(title for _, title in SECTIONS)) + ["Optimiser"])
    | _FORMATTER_TITLES
)


# Per-folder configuration scope categorisation (issue #230).
#
# Multi-folder VS Code workspaces reject ``window``-scoped settings in
# folder ``.vscode/settings.json``.  Every generated ``tclLsp.*`` setting
# must declare a non-``window`` scope.  Categorisation:
#
# - ``language-overridable`` for project + per-language settings
#   (formatting, style — same toggles can vary per Tcl dialect).
# - ``resource`` for per-folder analysis toggles
#   (diagnostics, optimiser, shimmer).
#
# Hand-edited sections (General, Features, Runtime Validation, AI,
# Package Manager, XC Migration, Trace) declare their own scopes —
# pinned by ``tests/test_settings_scope.py``.
_VSCODE_SCOPE_PREFIXES: tuple[tuple[str, str], ...] = (
    ("tclLsp.formatting.", "language-overridable"),
    ("tclLsp.style.", "language-overridable"),
    ("tclLsp.diagnostics.", "resource"),
    ("tclLsp.optimiser.", "resource"),
    ("tclLsp.shimmer.", "resource"),
)


def _resolve_vscode_scope(setting_key: str) -> str | None:
    """Return the scope string for a generated ``tclLsp.*`` setting key."""
    for prefix, scope in _VSCODE_SCOPE_PREFIXES:
        if setting_key.startswith(prefix):
            return scope
    return None


def _inject_vscode_scopes(sections: list[dict]) -> list[dict]:
    """Insert ``scope`` as the first key on every generated property.

    The position matters only for diff readability; VS Code accepts any
    key order.  Generated sections are deterministic, so the resulting
    package.json is stable across runs.
    """
    for section in sections:
        properties = section.get("properties")
        if not isinstance(properties, dict):
            continue
        new_props: dict[str, dict] = {}
        for key, schema in properties.items():
            scope = _resolve_vscode_scope(key)
            if scope is not None and "scope" not in schema:
                new_props[key] = {"scope": scope, **schema}
            else:
                new_props[key] = schema
        section["properties"] = new_props
    return sections


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
    new_formatter_sections = _build_vscode_formatter_sections()
    new_diag_sections = _build_vscode_diagnostic_sections()
    next_order = max(s["order"] for s in new_diag_sections) + 1 if new_diag_sections else 7
    new_opt_section = _build_vscode_optimiser_section(order=next_order)
    new_generated = _inject_vscode_scopes(
        new_formatter_sections + new_diag_sections + [new_opt_section]
    )

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


# Generate README tables

_GENERATED_DOCS = ROOT / "docs" / "generated"
_TABLES_PATH = _GENERATED_DOCS / "diagnostic_tables.md"
_DIAG_CODES_PATH = _GENERATED_DOCS / "diagnostic_codes.md"
_OPT_CODES_PATH = _GENERATED_DOCS / "optimisation_codes.md"


def _diag_dicts() -> list[dict]:
    return [
        {
            "code": d.code,
            "section": d.section,
            "description": d.description,
            "default": d.default,
        }
        for d in diagnostics_sorted()
    ]


def _opt_dicts() -> list[dict]:
    return [
        {
            "code": o.code,
            "description": o.description,
            "default": o.default,
            "category": o.opt_category or "",
            "readability": o.code in READABILITY_CODES,
            "standard": o.code in _STANDARD_CODES,
        }
        for o in optimisations_sorted()
    ]


def generate_readme_tables(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate combined markdown tables from registry."""
    env = _jinja_env(_TABLES_PATH.parent)
    template = env.get_template("diagnostic_tables.md.j2")
    content = template.render(diagnostics=_diag_dicts(), optimisations=_opt_dicts())

    if not dry_run:
        _TABLES_PATH.parent.mkdir(parents=True, exist_ok=True)
        _TABLES_PATH.write_text(content, encoding="utf-8")
    return _TABLES_PATH, content


def generate_diagnostic_codes(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate diagnostic-only markdown table from registry."""
    env = _jinja_env(_GENERATED_DOCS)
    template = env.get_template("diagnostic_codes.md.j2")
    content = template.render(diagnostics=_diag_dicts())

    if not dry_run:
        _GENERATED_DOCS.mkdir(parents=True, exist_ok=True)
        _DIAG_CODES_PATH.write_text(content, encoding="utf-8")
    return _DIAG_CODES_PATH, content


def generate_optimisation_codes(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate optimisation-only markdown table from registry."""
    env = _jinja_env(_GENERATED_DOCS)
    template = env.get_template("optimisation_codes.md.j2")
    content = template.render(optimisations=_opt_dicts())

    if not dry_run:
        _GENERATED_DOCS.mkdir(parents=True, exist_ok=True)
        _OPT_CODES_PATH.write_text(content, encoding="utf-8")
    return _OPT_CODES_PATH, content


# ---------------------------------------------------------------------------
# Generate ai/shared/diagnostics.json — categories from code registry
# ---------------------------------------------------------------------------

_DIAGNOSTICS_JSON_PATH = ROOT / "ai" / "shared" / "diagnostics.json"

# Mapping: codes.py section -> diagnostics.json category key.
_SECTION_TO_CATEGORY: dict[str, str] = {
    "error": "error",
    "warning": "style",
    "variable": "style",
    "security": "security",
    "hint": "style",
    "shimmer": "performance",
    "taint": "taint",
    "irules": "irules",
    "irules_security": "security",
    "irules_variable": "thread_safety",
}

_DIAGNOSTICS_JSON_STATIC: dict[str, object] = {
    "prefix_rules": [
        {"prefix": "E", "category": "error"},
        {"prefix": "O", "category": "optimiser"},
        {"prefix": "S", "category": "performance"},
        {"prefix": "IRULE", "category": "irules"},
        {"prefix": "TK", "category": "tk"},
    ],
    "default_category": "style",
    "review_categories": ["security", "taint", "thread_safety"],
    "irules_event_pattern": r"^\s*when\s+([A-Z][A-Z0-9_]{2,})\b",
}

_CATEGORY_DEFS: list[tuple[str, str]] = [
    ("error", "Errors"),
    ("security", "Security"),
    ("taint", "Taint Analysis"),
    ("thread_safety", "Thread Safety"),
    ("control_flow", "Control Flow"),
    ("performance", "Performance"),
    ("style", "Style & Best Practice"),
    ("optimiser", "Optimiser"),
    ("irules", "iRules"),
    ("tk", "Tk GUI"),
]


def _build_diagnostics_json_categories() -> list[dict]:
    """Build the categories array from the code registry."""
    section_codes = codes_by_section()
    overrides = ai_category_overrides()
    cat_codes: dict[str, list[str]] = {key: [] for key, _ in _CATEGORY_DEFS}
    for section_key, infos in section_codes.items():
        default_cat = _SECTION_TO_CATEGORY.get(section_key, "style")
        for info in infos:
            cat = overrides.get(info.code, default_cat)
            if cat in cat_codes:
                cat_codes[cat].append(info.code)
    cat_codes["tk"] = ["TK1001", "TK1002", "TK1003"]
    for lst in cat_codes.values():
        lst.sort()
    return [
        {"key": key, "label": label, "codes": cat_codes.get(key, [])}
        for key, label in _CATEGORY_DEFS
    ]


def generate_diagnostics_json(*, dry_run: bool = False) -> tuple[Path, str]:
    """Generate ai/shared/diagnostics.json from the code registry."""
    conversions = conversion_map()
    data: dict[str, object] = {
        "categories": _build_diagnostics_json_categories(),
        "convertible_codes": list(conversions),
        "conversion_map": conversions,
    }
    data.update(_DIAGNOSTICS_JSON_STATIC)
    content = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
    if not dry_run:
        _DIAGNOSTICS_JSON_PATH.write_text(content, encoding="utf-8")
    return _DIAGNOSTICS_JSON_PATH, content


# ---------------------------------------------------------------------------
# Generate AI prompt and skill files from Jinja2 templates
# ---------------------------------------------------------------------------

_AI_PROMPTS_DIR = ROOT / "ai" / "prompts"
_AI_SKILLS_DIR = ROOT / "ai" / "claude" / "skills"


def _ai_template_context() -> dict:
    """Build the template context for AI prompt/skill Jinja2 templates."""
    from shared.codes import CodeKind, all_codes  # noqa: E402
    from shared.optimisation_profiles import (  # noqa: E402
        CODE_MOTION_CODES,
        CONSTANT_FOLDING_CODES,
        DCE_CODES,
        PATTERN_CODES,
        RECURSION_CODES,
    )

    registry = all_codes()
    section_codes = codes_by_section()

    # Build per-section code lists as [{code, description}, ...]
    def _section_list(key: str) -> list[dict]:
        return [
            {"code": c.code, "desc": c.description.rstrip(".")}
            for c in section_codes.get(key, [])
            if not c.internal
        ]

    # Build optimiser list
    opt_infos = sorted(
        (i for i in registry.values() if i.kind is CodeKind.OPTIMISATION),
        key=lambda i: i.code,
    )

    def _fmt_codes(codes: frozenset[str]) -> str:
        return ", ".join(sorted(codes))

    return {
        "errors": _section_list("error"),
        "style": _section_list("warning") + _section_list("variable") + _section_list("hint"),
        "security": _section_list("security"),
        "shimmer": _section_list("shimmer"),
        "taint": _section_list("taint"),
        "irules": _section_list("irules"),
        "irules_security": _section_list("irules_security"),
        "irules_variable": _section_list("irules_variable"),
        "optimisations": [{"code": o.code, "desc": o.description.rstrip(".")} for o in opt_infos],
        "opt_readability": _fmt_codes(READABILITY_CODES),
        "opt_constant_folding": _fmt_codes(CONSTANT_FOLDING_CODES),
        "opt_pattern": _fmt_codes(PATTERN_CODES),
        "opt_dce": _fmt_codes(DCE_CODES),
        "opt_code_motion": _fmt_codes(CODE_MOTION_CODES),
        "opt_recursion": _fmt_codes(RECURSION_CODES),
    }


def _generate_from_j2(
    template_path: Path,
    output_path: Path,
    extra_context: dict | None = None,
    *,
    dry_run: bool = False,
) -> tuple[Path, str]:
    """Render a Jinja2 template with the AI template context."""
    env = _jinja_env(template_path.parent)
    template = env.get_template(template_path.name)
    ctx = _ai_template_context()
    if extra_context:
        ctx.update(extra_context)
    content = template.render(ctx)
    if not dry_run:
        output_path.write_text(content, encoding="utf-8")
    return output_path, content


# AI prompt/skill templates and their output paths.
_AI_GENERATED_FILES: list[tuple[Path, Path, dict | None]] = [
    (
        _AI_PROMPTS_DIR / "tcl_system.md.j2",
        _AI_PROMPTS_DIR / "tcl_system.md",
        None,
    ),
    (
        _AI_PROMPTS_DIR / "irules_system.md.j2",
        _AI_PROMPTS_DIR / "irules_system.md",
        None,
    ),
    (
        _AI_SKILLS_DIR / "tcl-optimise" / "SKILL.md.j2",
        _AI_SKILLS_DIR / "tcl-optimise" / "SKILL.md",
        None,
    ),
    (
        _AI_SKILLS_DIR / "irule-optimise" / "SKILL.md.j2",
        _AI_SKILLS_DIR / "irule-optimise" / "SKILL.md",
        None,
    ),
]


def generate_ai_files(*, dry_run: bool = False) -> list[tuple[Path, str]]:
    """Generate all AI prompt and skill files from Jinja2 templates."""
    results = [generate_diagnostics_json(dry_run=dry_run)]
    for template_path, output_path, extra in _AI_GENERATED_FILES:
        if template_path.exists():
            results.append(_generate_from_j2(template_path, output_path, extra, dry_run=dry_run))
    return results


# Render all — used by tests and --check mode


def render_all(*, dry_run: bool = False) -> list[tuple[Path, str]]:
    """Render all generated files. Returns list of (path, content) pairs."""
    results = [
        generate_jetbrains_catalog(dry_run=dry_run),
        generate_jetbrains_settings(dry_run=dry_run),
        generate_jetbrains_panel(dry_run=dry_run),
        generate_vscode_catalog(dry_run=dry_run),
        generate_vscode_package_json(dry_run=dry_run),
        generate_readme_tables(dry_run=dry_run),
        generate_diagnostic_codes(dry_run=dry_run),
        generate_optimisation_codes(dry_run=dry_run),
    ]
    results.extend(generate_ai_files(dry_run=dry_run))
    return results


# CLI


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
