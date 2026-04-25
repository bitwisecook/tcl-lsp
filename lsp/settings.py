"""Settings parsing, feature toggles, debounce, and configuration application."""

from __future__ import annotations

import asyncio
import logging
import re
import threading
from typing import TYPE_CHECKING

from lsprotocol import types

import core.common.codes_all  # noqa: F401  # trigger all code registrations
import lsp.state as _state
from core.commands.registry.runtime import configure_signatures
from core.common.codes import default_disabled_diagnostics, diagnostic_codes, optimisation_codes
from core.common.optimisation_profiles import (
    DEFAULT_EDITOR_PROFILE,
    PROFILE_NAMES,
    profile_from_name,
    profile_to_disabled,
)
from core.common.user_config import merge_settings_layers
from core.formatting import FormatterConfig

if TYPE_CHECKING:
    from pygls.lsp.server import LanguageServer

log = logging.getLogger(__name__)


def _camel_to_snake(name: str) -> str:
    """Convert lowerCamelCase/PascalCase names to snake_case."""
    first = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", first).lower()


def _normalise_formatter_settings(raw: dict) -> dict:
    """Map client formatter settings to FormatterConfig field names."""
    normalised: dict[str, object] = {}
    for key, value in raw.items():
        if not isinstance(key, str):
            continue
        field = _camel_to_snake(key)
        if field == "line_ending" and isinstance(value, str):
            mapping = {
                "lf": "\n",
                "crlf": "\r\n",
                "cr": "\r",
            }
            value = mapping.get(value.lower(), value)
        normalised[field] = value
    return normalised


_KNOWN_TCL_LSP_SECTIONS = frozenset(
    {
        "formatting",
        "diagnostics",
        "optimiser",
        "shimmer",
        "features",
        "style",
        "xcDiagnostics",
        "runtimeValidation",
        "ai",
        "packageManager",
    }
)
_KNOWN_TCL_LSP_TOPLEVEL = frozenset(
    {
        "dialect",
        "extraCommands",
        "libraryPaths",
    }
)


def _extract_tcl_lsp_settings(settings: dict) -> dict:
    """Extract extension/server settings from multiple client payload shapes.

    Handles three payload formats:
    1. Nested:   ``{"tclLsp": {"optimiser": {"O109": false}}}``
    2. Flat:     ``{"tclLsp.optimiser.O109": false}``
    3. Unwrapped (no ``tclLsp`` prefix — e.g. JetBrains pull-model response):
       ``{"optimiser": {"O109": false}, "dialect": "tcl8.6"}``
    """
    extracted: dict[str, object] = {}

    nested = settings.get("tclLsp")
    if isinstance(nested, dict):
        extracted.update(nested)

    for key, value in settings.items():
        if not isinstance(key, str):
            continue
        if key.startswith("tclLsp."):
            subkey = key[len("tclLsp.") :]
        else:
            continue

        section_handled = False
        for section in _KNOWN_TCL_LSP_SECTIONS:
            prefix = section + "."
            if subkey.startswith(prefix):
                section_key = subkey[len(prefix) :]
                current = extracted.get(section)
                if not isinstance(current, dict):
                    current = {}
                    extracted[section] = current
                current[section_key] = value
                section_handled = True
                break
        if not section_handled:
            extracted[subkey] = value

    if not extracted:
        if any(
            isinstance(k, str) and k in _KNOWN_TCL_LSP_SECTIONS | _KNOWN_TCL_LSP_TOPLEVEL
            for k in settings
        ):
            extracted.update(settings)

    return extracted


# Code registries

# Loaded from the self-registering code registry (core.common.codes).
_ALL_DIAGNOSTIC_CODES = diagnostic_codes()
_ALL_OPTIMISATION_CODES = optimisation_codes()

_FEATURE_TOGGLE_KEYS = {
    "hover": "hover_enabled",
    "completion": "completion_enabled",
    "diagnostics": "diagnostics_enabled",
    "semanticTokens": "semantic_tokens_enabled",
    "codeActions": "code_actions_enabled",
    "definition": "definition_enabled",
    "references": "references_enabled",
    "documentSymbols": "document_symbols_enabled",
    "folding": "folding_enabled",
    "rename": "rename_enabled",
    "signatureHelp": "signature_help_enabled",
    "workspaceSymbols": "workspace_symbols_enabled",
    "inlayHints": "inlay_hints_enabled",
    "callHierarchy": "call_hierarchy_enabled",
    "documentLinks": "document_links_enabled",
    "selectionRange": "selection_range_enabled",
    "documentHighlight": "document_highlight_enabled",
    "codeLens": "code_lens_enabled",
    "workspaceFileOps": "workspace_file_ops_enabled",
    "pullDiagnostics": "pull_diagnostics_enabled",
    "willSaveWaitUntil": "will_save_wait_until_enabled",
    "progress": "progress_enabled",
    "implementation": "implementation_enabled",
    "typeDefinition": "type_definition_enabled",
    "declaration": "declaration_enabled",
    "linkedEditingRange": "linked_editing_range_enabled",
}

# Feature toggles in this set are evaluated only at import time because the
# associated handler registration happens via the @server.feature decorator
# before any configuration is read.  Flipping them at runtime via
# didChangeConfiguration has no effect — a server restart is required.  The
# config loader logs a warning when it sees one change so users know.
_RESTART_REQUIRED_TOGGLES = frozenset({"pull_diagnostics_enabled"})

# Server reference injection

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


# Feature settings application


def _apply_feature_settings(tcl_settings: dict) -> bool:
    """Apply feature toggles and diagnostic/optimiser filters.

    Returns True if diagnostics need to be republished.
    """
    changed = False

    def _set_toggle(attr: str, val: object) -> bool:
        nonlocal changed
        if not isinstance(val, bool):
            return False
        if val == getattr(_state.feature_config, attr):
            return False
        setattr(_state.feature_config, attr, val)
        if attr in _RESTART_REQUIRED_TOGGLES:
            log.warning(
                "Feature toggle %r changed at runtime but takes effect only "
                "after a server restart (handler registration is fixed at "
                "startup).",
                attr,
            )
            return False
        changed = True
        return True

    # Feature-level toggles  (tclLsp.features.hover etc.)
    features = tcl_settings.get("features")
    if isinstance(features, dict):
        for json_key, attr in _FEATURE_TOGGLE_KEYS.items():
            val = features.get(json_key)
            if val is None:
                val = features.get(_camel_to_snake(json_key))
            if isinstance(val, bool):
                _set_toggle(attr, val)

    # Also accept flat keys: tclLsp.features.hover -> features.hover
    for key, value in tcl_settings.items():
        if isinstance(key, str) and key.startswith("features.") and isinstance(value, bool):
            json_key = key[len("features.") :]
            attr = _FEATURE_TOGGLE_KEYS.get(json_key) or _FEATURE_TOGGLE_KEYS.get(
                _camel_to_snake(json_key)
            )
            if attr:
                _set_toggle(attr, value)

    # Per-diagnostic-code filters  (tclLsp.diagnostics.W100 etc.)
    diagnostics_section = tcl_settings.get("diagnostics")
    if isinstance(diagnostics_section, dict):
        new_disabled: set[str] = set(default_disabled_diagnostics())
        for code in _ALL_DIAGNOSTIC_CODES:
            val = diagnostics_section.get(code)
            if isinstance(val, bool):
                if not val:
                    new_disabled.add(code)
                else:
                    new_disabled.discard(code)
        if new_disabled != _state.feature_config.disabled_diagnostics:
            _state.feature_config.disabled_diagnostics = new_disabled
            changed = True

        _DIAG_NON_CODE_KEYS = {"genericVariablePatterns", "generic_variable_patterns"}
        unknown_diag = {
            k
            for k, v in diagnostics_section.items()
            if v is False and k not in _ALL_DIAGNOSTIC_CODES and k not in _DIAG_NON_CODE_KEYS
        }
        if unknown_diag:
            log.warning(
                "Unrecognised diagnostic codes in settings (ignored): %s", sorted(unknown_diag)
            )

    # Style settings  (tclLsp.style.lineLength, tclLsp.style.nonAscii)
    style_section = tcl_settings.get("style")
    if isinstance(style_section, dict):
        ll = style_section.get("lineLength")
        if isinstance(ll, int) and ll > 0 and ll != _state.feature_config.line_length:
            _state.feature_config.line_length = ll
            changed = True
        non_ascii = style_section.get("nonAscii")
        if isinstance(non_ascii, str) and non_ascii in ("strict", "confusables", "common", "off"):
            from core.analysis.checks._style import set_non_ascii_mode

            set_non_ascii_mode(non_ascii)
            changed = True

    # Shimmer detection toggle  (tclLsp.shimmer.enabled)
    shimmer_section = tcl_settings.get("shimmer")
    if isinstance(shimmer_section, dict):
        shimmer_master = shimmer_section.get("enabled")
        if (
            isinstance(shimmer_master, bool)
            and shimmer_master != _state.feature_config.shimmer_enabled
        ):
            _state.feature_config.shimmer_enabled = shimmer_master
            changed = True

    # XC diagnostics toggle  (tclLsp.xcDiagnostics.enabled)
    xc_section = tcl_settings.get("xcDiagnostics")
    if isinstance(xc_section, dict):
        xc_enabled = xc_section.get("enabled")
        if (
            isinstance(xc_enabled, bool)
            and xc_enabled != _state.feature_config.xc_diagnostics_enabled
        ):
            _state.feature_config.xc_diagnostics_enabled = xc_enabled
            changed = True

    # Optimiser master switch, profile, + per-code  (tclLsp.optimiser.*)
    optimiser_section = tcl_settings.get("optimiser")
    if isinstance(optimiser_section, dict):
        master = optimiser_section.get("enabled")
        if isinstance(master, bool) and master != _state.feature_config.optimiser_enabled:
            _state.feature_config.optimiser_enabled = master
            changed = True

        profile_name = optimiser_section.get("profile")
        if isinstance(profile_name, str) and profile_name in PROFILE_NAMES:
            if profile_name != _state.feature_config.optimiser_profile:
                _state.feature_config.optimiser_profile = profile_name
                changed = True
        elif isinstance(profile_name, str) and profile_name:
            log.warning("Unknown optimiser profile %r (ignored)", profile_name)

        try:
            base_disabled = set(
                profile_to_disabled(profile_from_name(_state.feature_config.optimiser_profile))
            )
        except ValueError:
            base_disabled = set(profile_to_disabled(DEFAULT_EDITOR_PROFILE))

        for code in _ALL_OPTIMISATION_CODES:
            val = optimiser_section.get(code)
            if val is True:
                base_disabled.discard(code)
            elif val is False:
                base_disabled.add(code)

        if base_disabled != _state.feature_config.disabled_optimisations:
            _state.feature_config.disabled_optimisations = base_disabled
            changed = True

        unknown_opt = {
            k
            for k, v in optimiser_section.items()
            if v is False and k not in _ALL_OPTIMISATION_CODES and k not in ("enabled", "profile")
        }
        if unknown_opt:
            log.warning(
                "Unrecognised optimisation codes in settings (ignored): %s", sorted(unknown_opt)
            )

    # Generic variable patterns  (tclLsp.diagnostics.genericVariablePatterns)
    # An explicit empty list disables IRULE4002; absent key leaves the default.
    if isinstance(diagnostics_section, dict):
        has_generic_patterns = "genericVariablePatterns" in diagnostics_section
        if has_generic_patterns:
            patterns = diagnostics_section.get("genericVariablePatterns")
        else:
            has_generic_patterns = "generic_variable_patterns" in diagnostics_section
            patterns = diagnostics_section.get("generic_variable_patterns")
        if has_generic_patterns and isinstance(patterns, list):
            new_patterns = [str(p) for p in patterns if isinstance(p, str)]
            if new_patterns != _state.feature_config.generic_variable_patterns:
                _state.feature_config.generic_variable_patterns = new_patterns
                changed = True

    return changed


# Settings debounce

_pending_apply_handle: asyncio.TimerHandle | None = None
_SETTINGS_DEBOUNCE_S = 0.3


def _merged_settings() -> dict:
    """Merge all configuration layers in precedence order.

    Lowest to highest priority:
      1. Global user config (``~/.config/tcl-lsp/config.ini``)
      2. Editor settings (``workspace/didChangeConfiguration`` payload)
      3. Project config (``<workspace>/.tcl-lsp.ini``)

    Inline ``# noqa`` and top-of-file ``# tcl-lsp: disable=`` directives are
    applied later, in the per-document diagnostics pipeline — they do not
    feed into server-level ``feature_config``.
    """
    return merge_settings_layers(
        _state.global_config_settings,
        _state.editor_config_settings,
        _state.project_config_settings,
    )


def _apply_all_settings(tcl_settings: dict) -> None:
    """Update the editor config layer and apply the merged settings (debounced)."""
    _state.editor_config_settings = tcl_settings
    _schedule_apply_merged()


def apply_project_settings(tcl_settings: dict) -> None:
    """Update the project-config layer and apply the merged settings."""
    _state.project_config_settings = tcl_settings
    _schedule_apply_merged()


def apply_global_settings(tcl_settings: dict) -> None:
    """Update the global user-config layer and apply the merged settings."""
    _state.global_config_settings = tcl_settings
    _schedule_apply_merged()


def _schedule_apply_merged() -> None:
    """Debounced merged-settings application — leading-edge with trailing coalesce."""
    global _pending_apply_handle

    first_in_burst = _pending_apply_handle is None
    if _pending_apply_handle is not None:
        _pending_apply_handle.cancel()
    try:
        loop = asyncio.get_event_loop()
    except RuntimeError:
        _apply_merged_settings_now()
        return
    if first_in_burst:
        _apply_merged_settings_now()
    _pending_apply_handle = loop.call_later(
        _SETTINGS_DEBOUNCE_S,
        _apply_merged_settings_now,
    )


def _apply_merged_settings_now() -> None:
    """Apply the merged configuration layers (called after debounce timer)."""
    global _pending_apply_handle
    _pending_apply_handle = None

    tcl_settings = _merged_settings()

    formatting = tcl_settings.get("formatting")
    if isinstance(formatting, dict) and formatting:
        current = _state.formatter_config.to_dict()
        current.update(_normalise_formatter_settings(formatting))
        _state.formatter_config = FormatterConfig.from_dict(current)

    extra_commands_setting = tcl_settings.get("extraCommands")
    if extra_commands_setting is None:
        extra_commands_setting = tcl_settings.get("extra_commands")
    if extra_commands_setting is None:
        extra_commands = None
    elif isinstance(extra_commands_setting, list):
        extra_commands = [str(cmd) for cmd in extra_commands_setting]
    else:
        extra_commands = []

    library_paths_setting = tcl_settings.get("libraryPaths")
    if library_paths_setting is None:
        library_paths_setting = tcl_settings.get("library_paths")
    if isinstance(library_paths_setting, list):
        library_paths = [str(p) for p in library_paths_setting]
        _state.background_scanner.configure(library_paths=library_paths)
        resolver_paths = _state.background_scanner.workspace_roots + library_paths
        _state.package_resolver.configure(search_paths=resolver_paths)
        _state._loaded_packages.clear()
        import lsp.workspace_init as _wi

        threading.Thread(target=_wi._run_background_scan, daemon=True).start()

    dialect_setting = tcl_settings.get("dialect")
    if isinstance(dialect_setting, str) and dialect_setting:
        _state.feature_config.dialect_explicitly_set = True
    signatures_changed = configure_signatures(
        dialect=dialect_setting if isinstance(dialect_setting, str) else None,
        extra_commands=extra_commands,
    )
    if signatures_changed:
        from core.common.dialect import active_dialect

        log.info(
            "Dialect changed to %s (explicit=%s)",
            active_dialect(),
            _state.feature_config.dialect_explicitly_set,
        )

    diags_were_enabled = _state.feature_config.diagnostics_enabled
    features_changed = _apply_feature_settings(tcl_settings)

    if not signatures_changed and not features_changed:
        return

    if signatures_changed:
        _state.diagnostic_scheduler.cancel_all()

    import lsp.diagnostics_pipeline as _dp

    _publish_diagnostics = _dp._publish_diagnostics
    _publish_diagnostics_sync = _dp._publish_diagnostics_sync
    _publish_diags_to_client = _dp._publish_diags_to_client

    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None
    for uri, doc_state in _state.workspace_state.items():
        if signatures_changed and loop is not None:
            loop.create_task(
                _publish_diagnostics(
                    uri,
                    doc_state.source,
                    doc_state.version,
                    force_reanalyse=True,
                )
            )
        elif doc_state.analysis is not None:
            _publish_diagnostics_sync(
                uri,
                doc_state.source,
                doc_state.version,
                force_reanalyse=signatures_changed,
            )

    if diags_were_enabled and not _state.feature_config.diagnostics_enabled:
        for uri, doc_state in _state.workspace_state.items():
            if doc_state.analysis is None:
                _publish_diags_to_client(uri, [], doc_state.version)


def _pull_and_apply_configuration() -> None:
    """Pull configuration from the client via ``workspace/configuration``."""
    params = types.ConfigurationParams(items=[types.ConfigurationItem(section="tclLsp")])

    def _on_result(result: list[object] | None) -> None:
        if not result:
            return
        item = result[0]
        if not isinstance(item, dict):
            return
        _apply_all_settings(item)

    try:
        _server.workspace_configuration(params, callback=_on_result)  # type: ignore[union-attr]
    except Exception:
        log.debug("workspace/configuration pull failed", exc_info=True)


# Registration


def register(server_instance: LanguageServer) -> None:
    """Register the didChangeConfiguration handler."""

    @server_instance.feature(types.WORKSPACE_DID_CHANGE_CONFIGURATION)
    def did_change_configuration(params: types.DidChangeConfigurationParams) -> None:
        settings = params.settings
        if not settings:
            _pull_and_apply_configuration()
            return
        tcl_settings = _extract_tcl_lsp_settings(settings)
        _apply_all_settings(tcl_settings)
