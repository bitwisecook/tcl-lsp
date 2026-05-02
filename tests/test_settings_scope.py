"""VS Code ``package.json`` ``tclLsp.*`` scope regression (issue #230).

Multi-folder VS Code workspaces reject any setting with the default
``window`` scope when authored in folder-level ``.vscode/settings.json``.
The fix for issue #230 was to declare an appropriate non-``window``
scope on every ``tclLsp.*`` setting:

- ``machine-overridable`` for host paths (interpreter/server/cache),
- ``language-overridable`` for project + per-language settings
  (dialect, formatting, features, style, ``extraCommands``),
- ``resource`` for the rest (per-folder analysis toggles).

The single exception is ``tcl-lsp.trace.server`` (the LSP transport
trace level), which legitimately stays at the default ``window`` scope.

This test pins the categorisation so a future setting addition that
forgets ``scope`` re-introduces the original bug only if it's the
trace key — every other setting will fail this check.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_JSON = ROOT / "editors" / "vscode" / "package.json"


# Settings legitimately allowed to keep the default ``window`` scope.
# Adding to this set is a deliberate decision — keep it small.
_WINDOW_SCOPE_ALLOWLIST: frozenset[str] = frozenset(
    {
        "tcl-lsp.trace.server",  # LSP transport trace level
    }
)

_VALID_SCOPES: frozenset[str] = frozenset(
    {
        "application",
        "machine",
        "machine-overridable",
        "window",
        "resource",
        "language-overridable",
    }
)

# Path prefixes — a host path that points at an executable or directory.
_PATH_KEYS: frozenset[str] = frozenset(
    {
        "tclLsp.pythonPath",
        "tclLsp.serverPath",
        "tclLsp.runtimeValidation.tclshPath",
        "tclLsp.packageManager.cacheDir",
        "tclLsp.packageManager.registryUrl",
    }
)


def _all_properties() -> dict[str, dict]:
    """Return ``{key: schema}`` for every contributed configuration property."""
    data = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    properties: dict[str, dict] = {}
    for group in data["contributes"]["configuration"]:
        for key, schema in group.get("properties", {}).items():
            properties[key] = schema
    return properties


def test_every_tcl_lsp_setting_has_a_non_window_scope():
    """Every ``tclLsp.*`` property must declare ``scope`` != ``window``.

    Multi-folder VS Code workspaces reject ``window``-scoped settings in
    folder ``.vscode/settings.json``.  The allowlist captures the few
    settings (``tcl-lsp.trace.server``) where ``window`` is intentional.
    """
    properties = _all_properties()
    offenders: list[str] = []
    for key, schema in properties.items():
        if key in _WINDOW_SCOPE_ALLOWLIST:
            continue
        scope = schema.get("scope")
        if scope is None or scope == "window":
            offenders.append(f"{key} (scope={scope!r})")

    assert not offenders, (
        "Settings without a non-window scope (would be rejected in "
        "folder-level .vscode/settings.json — see issue #230):\n  " + "\n  ".join(sorted(offenders))
    )


def test_all_declared_scopes_are_valid_lsp_values():
    """Every ``scope`` value must be a recognised VS Code scope."""
    properties = _all_properties()
    bad: list[str] = []
    for key, schema in properties.items():
        scope = schema.get("scope")
        if scope is not None and scope not in _VALID_SCOPES:
            bad.append(f"{key}: {scope!r}")
    assert not bad, "Unknown scope values:\n  " + "\n  ".join(bad)


def test_path_settings_are_machine_overridable():
    """Host paths use ``machine-overridable`` so they can't vary per-folder.

    A folder cannot meaningfully override the path to the Python
    interpreter or the language server — those bind to a process and
    must be set at workspace/user level.
    """
    properties = _all_properties()
    wrong: list[str] = []
    for key in _PATH_KEYS:
        if key not in properties:
            continue
        scope = properties[key].get("scope")
        if scope != "machine-overridable":
            wrong.append(f"{key} (scope={scope!r}, expected machine-overridable)")
    assert not wrong, "Path settings with wrong scope:\n  " + "\n  ".join(wrong)


@pytest.mark.parametrize(
    "prefix,expected_scopes",
    [
        # Per-folder + per-language settings.
        ("tclLsp.formatting.", {"language-overridable"}),
        ("tclLsp.features.", {"language-overridable"}),
        ("tclLsp.style.", {"language-overridable"}),
        # Per-folder analysis toggles (no language overrides needed).
        ("tclLsp.diagnostics.", {"resource"}),
        ("tclLsp.optimiser.", {"resource"}),
        ("tclLsp.shimmer.", {"resource"}),
        ("tclLsp.xcDiagnostics.", {"resource"}),
        ("tclLsp.ai.", {"resource"}),
    ],
)
def test_settings_under_prefix_share_scope(prefix: str, expected_scopes: set[str]):
    """All settings under a section prefix must share one of the expected scopes."""
    properties = _all_properties()
    matching = {k: v for k, v in properties.items() if k.startswith(prefix)}
    assert matching, f"No settings found under {prefix!r} — manifest may have been refactored"
    wrong: list[str] = []
    for key, schema in matching.items():
        scope = schema.get("scope")
        if scope not in expected_scopes:
            wrong.append(f"{key} (scope={scope!r}, expected one of {sorted(expected_scopes)})")
    assert not wrong, f"Mismatched scope under {prefix!r}:\n  " + "\n  ".join(wrong)


def test_dialect_setting_is_language_overridable():
    """``tclLsp.dialect`` is per-folder and per-language."""
    properties = _all_properties()
    dialect = properties.get("tclLsp.dialect")
    assert dialect is not None
    assert dialect.get("scope") == "language-overridable"


def test_runtime_validation_path_separated_from_toggles():
    """``runtimeValidation.tclshPath`` is a path; the rest are per-folder toggles."""
    properties = _all_properties()
    path_scope = properties["tclLsp.runtimeValidation.tclshPath"].get("scope")
    assert path_scope == "machine-overridable"
    for key in (
        "tclLsp.runtimeValidation.enabled",
        "tclLsp.runtimeValidation.adapter",
        "tclLsp.runtimeValidation.timeoutMs",
    ):
        assert properties[key].get("scope") == "resource", (
            f"{key} should be resource-scoped, got {properties[key].get('scope')!r}"
        )
