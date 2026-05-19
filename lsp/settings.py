"""Settings parsing, feature toggles, debounce, and configuration application."""

from __future__ import annotations

import asyncio
import copy
import logging
import re
import threading
from collections.abc import Sequence
from typing import TYPE_CHECKING, Any

from lsprotocol import types

import lsp.state as _state
import shared.codes_all  # noqa: F401  # trigger all code registrations
from core.commands.registry.runtime import configure_signatures
from core.formatting import FormatterConfig
from shared.codes import default_disabled_diagnostics, diagnostic_codes, optimisation_codes
from shared.optimisation_profiles import (
    DEFAULT_EDITOR_PROFILE,
    PROFILE_NAMES,
    profile_from_name,
    profile_to_disabled,
)
from shared.user_config import merge_settings_layers

if TYPE_CHECKING:
    from pygls.lsp.server import LanguageServer

    from .feature_config import FeatureConfig

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

# Loaded from the self-registering code registry (shared.codes).
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

# Schema default for ``tclLsp.dialect`` — kept in sync with the
# ``editors/vscode/package.json`` schema.  VS Code's
# ``workspace/configuration`` returns this value for every scope that
# doesn't carry its own ``tclLsp.dialect`` entry; we use it to
# distinguish "user explicitly set the dialect" from "client echoed
# back the schema default" so the iRules / iApps auto-switch in
# ``did_open`` doesn't get blocked by a non-override.
_DIALECT_SCHEMA_DEFAULT = "tcl8.6"


# Server reference injection

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


def _require_server() -> LanguageServer:
    if _server is None:
        raise RuntimeError("lsp.settings: server not configured")
    return _server


# Feature settings application


def _apply_feature_settings(
    tcl_settings: dict,
    target: "FeatureConfig | None" = None,
) -> bool:
    """Apply feature toggles and diagnostic/optimiser filters to ``target``.

    Mutates ``target`` (defaulting to the workspace-level ``feature_config``)
    in place and returns True if any diagnostics need republishing.

    A pulled ``tclLsp.dialect`` matching :data:`_DIALECT_SCHEMA_DEFAULT`
    is treated as the package.json schema default echoed back via
    ``workspace/configuration`` rather than an explicit user override —
    VS Code returns the schema default for every scope that doesn't
    carry its own entry, and the iRules / iApps auto-switch in
    ``did_open`` depends on ``dialect_explicitly_set`` staying false
    when the user hasn't actually pinned the dialect.
    """
    if target is None:
        target = _state.feature_config

    changed = False

    def _set_toggle(attr: str, val: object) -> bool:
        nonlocal changed
        if not isinstance(val, bool):
            return False
        if val == getattr(target, attr):
            return False
        setattr(target, attr, val)
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
        if new_disabled != target.disabled_diagnostics:
            target.disabled_diagnostics = new_disabled
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
        if isinstance(ll, int) and ll > 0 and ll != target.line_length:
            target.line_length = ll
            changed = True
        non_ascii = style_section.get("nonAscii")
        if isinstance(non_ascii, str) and non_ascii in ("strict", "confusables", "common", "off"):
            # Per-folder W108 mode (issue #407).  The LSP request handlers
            # open a ``non_ascii_mode_scope`` for the resolved value so this
            # setting applies to documents in this folder only.  The
            # workspace-fallback target's value also feeds
            # ``set_non_ascii_mode`` below to keep the process-wide default
            # in sync for callers that don't open a scope.
            if non_ascii != target.non_ascii_mode:
                target.non_ascii_mode = non_ascii
                changed = True
            if target is _state.feature_config:
                from core.analysis.checks._style import set_non_ascii_mode

                set_non_ascii_mode(non_ascii)

    # Shimmer detection toggle  (tclLsp.shimmer.enabled)
    shimmer_section = tcl_settings.get("shimmer")
    if isinstance(shimmer_section, dict):
        shimmer_master = shimmer_section.get("enabled")
        if isinstance(shimmer_master, bool) and shimmer_master != target.shimmer_enabled:
            target.shimmer_enabled = shimmer_master
            changed = True

    # XC diagnostics toggle  (tclLsp.xcDiagnostics.enabled)
    xc_section = tcl_settings.get("xcDiagnostics")
    if isinstance(xc_section, dict):
        xc_enabled = xc_section.get("enabled")
        if isinstance(xc_enabled, bool) and xc_enabled != target.xc_diagnostics_enabled:
            target.xc_diagnostics_enabled = xc_enabled
            changed = True

    # Optimiser master switch, profile, + per-code  (tclLsp.optimiser.*)
    optimiser_section = tcl_settings.get("optimiser")
    if isinstance(optimiser_section, dict):
        master = optimiser_section.get("enabled")
        if isinstance(master, bool) and master != target.optimiser_enabled:
            target.optimiser_enabled = master
            changed = True

        profile_name = optimiser_section.get("profile")
        if isinstance(profile_name, str) and profile_name in PROFILE_NAMES:
            if profile_name != target.optimiser_profile:
                target.optimiser_profile = profile_name
                changed = True
        elif isinstance(profile_name, str) and profile_name:
            log.warning("Unknown optimiser profile %r (ignored)", profile_name)

        try:
            base_disabled = set(profile_to_disabled(profile_from_name(target.optimiser_profile)))
        except ValueError:
            base_disabled = set(profile_to_disabled(DEFAULT_EDITOR_PROFILE))

        for code in _ALL_OPTIMISATION_CODES:
            val = optimiser_section.get(code)
            if val is True:
                base_disabled.discard(code)
            elif val is False:
                base_disabled.add(code)

        if base_disabled != target.disabled_optimisations:
            target.disabled_optimisations = base_disabled
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

    # Per-folder dialect / extra commands (issue #407).  These mirror the
    # process-wide ``configure_signatures(...)`` call in
    # ``_apply_merged_settings_now`` but live on the FeatureConfig so each
    # workspace folder can carry its own value.  The LSP request handlers
    # wrap their work in ``dialect_scope(...)`` using these fields, so
    # documents in folders configured for different dialects don't fight
    # over a single process-wide setting.
    folder_dialect = tcl_settings.get("dialect")
    if isinstance(folder_dialect, str) and folder_dialect:
        from core.commands.registry.runtime import _canonical_dialect

        canonical = _canonical_dialect(folder_dialect)
        # VS Code's ``workspace/configuration`` returns the package.json
        # schema default for every scope without an explicit entry.
        # Don't promote that echoed default into a *per-folder* override
        # (leave ``cfg.dialect`` as ``None`` so
        # ``resolve_dialect_for_uri`` falls through to the workspace
        # value), and don't flip ``dialect_explicitly_set`` either —
        # both would block the iRules / iApps auto-switch in
        # ``did_open`` for every ``.irul`` / ``.iapp`` file opened in a
        # workspace the user hasn't customised.  The workspace-level
        # target still records the value so ``configure_signatures``
        # has a sensible default to wire up.
        looks_inherited = canonical == _DIALECT_SCHEMA_DEFAULT
        is_workspace_target = target is _state.feature_config
        if canonical is not None and (is_workspace_target or not looks_inherited):
            if canonical != target.dialect:
                target.dialect = canonical
                changed = True
        if canonical is not None and not target.dialect_explicitly_set and not looks_inherited:
            target.dialect_explicitly_set = True
            changed = True
    elif folder_dialect is None and target.dialect is not None and "dialect" in tcl_settings:
        # Explicit null clears the override; absent key keeps the current value.
        target.dialect = None
        target.dialect_explicitly_set = False
        changed = True

    folder_extras_raw = tcl_settings.get("extraCommands")
    if folder_extras_raw is None:
        folder_extras_raw = tcl_settings.get("extra_commands")
    if isinstance(folder_extras_raw, list):
        # Filter to actual string entries before stripping — otherwise
        # ``str(None).strip()`` would silently register a ``"None"``
        # command name (and similar artefacts for any non-string entry).
        normalised = tuple(
            sorted(
                {
                    name.strip()
                    for name in folder_extras_raw
                    if isinstance(name, str) and name.strip()
                }
            )
        )
        if normalised != target.extra_commands:
            target.extra_commands = normalised
            changed = True

    folder_paths_raw = tcl_settings.get("libraryPaths")
    if folder_paths_raw is None:
        folder_paths_raw = tcl_settings.get("library_paths")
    if isinstance(folder_paths_raw, list):
        normalised_paths = tuple(str(p) for p in folder_paths_raw if isinstance(p, str) and p)
        if normalised_paths != target.library_paths:
            target.library_paths = normalised_paths
            changed = True

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
            if new_patterns != target.generic_variable_patterns:
                target.generic_variable_patterns = new_patterns
                changed = True

    return changed


# Settings debounce

_pending_apply_handle: asyncio.TimerHandle | None = None
_SETTINGS_DEBOUNCE_S = 0.3


def _merged_settings(folder_uri: str = "") -> dict:
    """Merge all configuration layers for ``folder_uri`` in precedence order.

    Lowest to highest priority:
      1. Global user config (``~/.config/tcl-lsp/config.ini``) — applies
         everywhere, not per-folder.
      2. Editor settings (``workspace/configuration`` pull payload, per
         workspace folder).
      3. Project config (``<folder>/.tcl-lsp.ini``, per workspace folder).

    ``folder_uri=""`` selects the workspace/user-level fallback layers.
    Per-folder layers fall back to the workspace-level layers when not
    explicitly populated.

    Inline ``# noqa`` and top-of-file ``# tcl-lsp: disable=`` directives are
    applied later, in the per-document diagnostics pipeline — they do not
    feed into server-level ``feature_config``.
    """
    editor = _state.editor_config_settings_per_folder.get(folder_uri, _state.editor_config_settings)
    project = _state.project_config_settings_per_folder.get(
        folder_uri, _state.project_config_settings
    )
    return merge_settings_layers(
        _state.global_config_settings,
        editor,
        project,
    )


def _apply_all_settings(tcl_settings: dict, folder_uri: str = "") -> None:
    """Update the editor-config layer for ``folder_uri`` and re-apply (debounced)."""
    if folder_uri == "":
        _state.editor_config_settings = tcl_settings
    else:
        _state.editor_config_settings_per_folder[folder_uri] = tcl_settings
    _schedule_apply_merged()


def apply_project_settings(tcl_settings: dict, folder_uri: str = "") -> None:
    """Update the project-config layer for ``folder_uri`` and re-apply."""
    if folder_uri == "":
        _state.project_config_settings = tcl_settings
    else:
        _state.project_config_settings_per_folder[folder_uri] = tcl_settings
    _schedule_apply_merged()


def apply_global_settings(tcl_settings: dict) -> None:
    """Update the global user-config layer (not per-folder) and re-apply."""
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


def _apply_settings_to_target(folder_uri: str, tcl_settings: dict) -> bool:
    """Apply ``tcl_settings`` to the FeatureConfig/FormatterConfig for ``folder_uri``.

    ``folder_uri=""`` targets the workspace/user-level fallback (the
    ``feature_config`` / ``formatter_config`` module attributes).  Returns
    True if any feature settings changed.
    """
    if folder_uri == "":
        feature_target = _state.feature_config
    else:
        feature_target = _state.get_or_init_folder_feature_config(folder_uri)

    formatting = tcl_settings.get("formatting")
    if isinstance(formatting, dict) and formatting:
        if folder_uri == "":
            current = _state.formatter_config.to_dict()
        else:
            current = _state.get_or_init_folder_formatter_config(folder_uri).to_dict()
        current.update(_normalise_formatter_settings(formatting))
        _state.set_folder_formatter_config(folder_uri, FormatterConfig.from_dict(current))

    return _apply_feature_settings(tcl_settings, target=feature_target)


def _apply_merged_settings_now() -> None:
    """Apply merged config layers — workspace fallback + every known folder."""
    global _pending_apply_handle
    _pending_apply_handle = None

    # Snapshot each open document's resolved settings *before* any cfg
    # mutates so we can detect per-folder changes that don't show up in
    # ``signatures_changed`` below (which only tracks the workspace-level
    # ``configure_signatures`` call).  Without this, a folder whose
    # ``tclLsp.dialect`` (or other analyser-baked setting) arrives late
    # via ``workspace/configuration`` -- a normal race on session start,
    # when ``did_open`` for already-open editors fires before the pull
    # callback returns -- leaves W002 / W108 / W123 baked into the cached
    # analysis even after the per-folder setting resolves to the right
    # value.  ``disabled_diagnostics`` is in the tuple too because the
    # analyser gates W123 / W242 / W307 / W308 emission on it (see
    # ``_effective_disabled_diagnostics``); user-toggled diagnostic
    # enablement therefore changes what's in the cached AnalysisResult,
    # not just the post-filter.  See issue #407.
    pre_apply_doc_resolution: dict[str, tuple] = {}
    for uri, doc_state in _state.workspace_state.items():
        dialect, extras = _state.resolve_dialect_for_uri(uri, doc_state.source)
        cfg_for_uri = _state.config_for_uri(uri)
        pre_apply_doc_resolution[uri] = (
            dialect,
            extras,
            cfg_for_uri.non_ascii_mode,
            frozenset(cfg_for_uri.disabled_diagnostics),
        )

    fallback_settings = _merged_settings()

    # Process-wide settings: signatures (dialect / extraCommands), library
    # paths, scanner configuration.  These are read from the workspace
    # fallback layer — multi-folder per-dialect support would require the
    # signature registry to be folder-aware, which is a larger refactor.
    extra_commands_setting = fallback_settings.get("extraCommands")
    if extra_commands_setting is None:
        extra_commands_setting = fallback_settings.get("extra_commands")
    if extra_commands_setting is None:
        extra_commands = None
    elif isinstance(extra_commands_setting, list):
        # Filter to actual string entries — ``str(None)`` would otherwise
        # silently register a ``"None"`` command name (and similar
        # artefacts for any non-string entry).
        extra_commands = [cmd for cmd in extra_commands_setting if isinstance(cmd, str)]
    else:
        extra_commands = []

    library_paths_setting = fallback_settings.get("libraryPaths")
    if library_paths_setting is None:
        library_paths_setting = fallback_settings.get("library_paths")
    if isinstance(library_paths_setting, list):
        library_paths = [p for p in library_paths_setting if isinstance(p, str) and p]
        _state.background_scanner.configure(library_paths=library_paths)
        resolver_paths = _state.background_scanner.workspace_roots + library_paths
        _state.package_resolver.configure(search_paths=resolver_paths)
        _state._loaded_packages.clear()
        import lsp.workspace_init as _wi

        threading.Thread(target=_wi._run_background_scan, daemon=True).start()

    dialect_setting = fallback_settings.get("dialect")
    if isinstance(dialect_setting, str) and dialect_setting:
        # Same heuristic as ``_apply_feature_settings``: only treat the
        # dialect as an explicit user override when it differs from the
        # schema default that VS Code echoes back via
        # ``workspace/configuration``.  Otherwise the iRules / iApps
        # auto-switch in ``did_open`` would be silently disabled for
        # every user who hasn't customised ``tclLsp.dialect``.
        if dialect_setting != _DIALECT_SCHEMA_DEFAULT:
            _state.feature_config.dialect_explicitly_set = True
    signatures_changed = configure_signatures(
        dialect=dialect_setting if isinstance(dialect_setting, str) else None,
        extra_commands=extra_commands,
    )
    if signatures_changed:
        from shared.dialect import active_dialect

        log.info(
            "Dialect changed to %s (explicit=%s)",
            active_dialect(),
            _state.feature_config.dialect_explicitly_set,
        )

    # Per-folder dialect overrides (issue #407) are honoured via
    # ``FeatureConfig.dialect`` + ``dialect_scope`` at request time, so the
    # "only one dialect can be active at a time" caveat that used to live
    # here no longer applies.  The workspace-fallback ``configure_signatures``
    # call above sets the default that applies to documents outside every
    # known folder; folder-specific dialects flow through the FeatureConfig
    # populated by ``_apply_feature_settings``.

    # Per-folder + fallback feature/formatter application.
    diags_were_enabled = _state.feature_config.diagnostics_enabled
    features_changed = _apply_settings_to_target("", fallback_settings)

    folder_uris = sorted(
        set(_state.editor_config_settings_per_folder.keys())
        | set(_state.project_config_settings_per_folder.keys())
        | set(_state.workspace_folder_uris())
    )
    for folder_uri in folder_uris:
        if folder_uri == "":
            continue
        folder_settings = _merged_settings(folder_uri)
        if _apply_settings_to_target(folder_uri, folder_settings):
            features_changed = True

    # Per-folder PackageResolver configuration (issue #407).  Each folder
    # with its own ``tclLsp.libraryPaths`` gets a dedicated resolver
    # configured with that folder's paths plus the workspace_roots.  Only
    # spin one up when the folder's paths actually *differ* from the
    # workspace fallback — VS Code typically returns folder configuration
    # already merged with workspace values, so library_paths is populated
    # for every folder even when no folder-level override exists.  Without
    # this guard we'd end up with N duplicate resolvers all scanning the
    # same set of paths.
    workspace_roots = _state.background_scanner.workspace_roots
    fallback_paths = (
        tuple(_state.feature_config.library_paths) if _state.feature_config.library_paths else ()
    )
    for folder_uri in folder_uris:
        if folder_uri == "":
            continue
        folder_cfg = _state.get_or_init_folder_feature_config(folder_uri)
        if folder_cfg.library_paths is None:
            # Folder cleared its libraryPaths override — drop any stale
            # per-folder resolver so ``package_resolver_for_uri`` falls
            # back to the workspace resolver, and prune ``_loaded_packages``
            # entries keyed to it so a follow-up rescan picks the
            # workspace files instead of the stale folder ones.
            dropped = _state._per_folder_package_resolvers.pop(folder_uri, None)
            if dropped is not None:
                dropped_key = id(dropped)
                _state._loaded_packages.difference_update(
                    entry for entry in list(_state._loaded_packages) if entry[0] == dropped_key
                )
            continue
        folder_paths = tuple(folder_cfg.library_paths)
        if folder_paths == fallback_paths:
            # Folder mirrors the workspace fallback — drop any stale
            # per-folder resolver so ``package_resolver_for_uri`` falls
            # back to the shared workspace resolver instead of
            # double-scanning.  Also prune any ``_loaded_packages`` entries
            # keyed to the dropped resolver so a follow-up rescan can
            # re-load packages via the workspace resolver.
            dropped = _state._per_folder_package_resolvers.pop(folder_uri, None)
            if dropped is not None:
                dropped_key = id(dropped)
                _state._loaded_packages.difference_update(
                    entry for entry in list(_state._loaded_packages) if entry[0] == dropped_key
                )
            continue
        folder_resolver = _state.get_or_init_folder_package_resolver(folder_uri)
        new_search_paths = workspace_roots + list(folder_paths)
        if folder_resolver._search_paths != new_search_paths:
            # Search paths changed — drop the ``(resolver_id, name)``
            # entries in ``_loaded_packages`` for this resolver so that
            # packages that resolved to the old paths get re-loaded
            # from the new ones.
            resolver_key = id(folder_resolver)
            _state._loaded_packages.difference_update(
                entry for entry in list(_state._loaded_packages) if entry[0] == resolver_key
            )
            folder_resolver.configure(search_paths=new_search_paths)

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
        # Did this doc's resolved dialect / extras / non-ASCII mode /
        # disabled_diagnostics change?  If so the cached analysis was
        # built with stale per-folder settings — dialect-baked checks
        # (W002, W108, W123, irules_checks etc.) need a re-analyse, not
        # just a re-publish.  See issue #407.
        new_dialect, new_extras = _state.resolve_dialect_for_uri(uri, doc_state.source)
        new_cfg = _state.config_for_uri(uri)
        new_resolution = (
            new_dialect,
            new_extras,
            new_cfg.non_ascii_mode,
            frozenset(new_cfg.disabled_diagnostics),
        )
        doc_force_reanalyse = signatures_changed or (
            pre_apply_doc_resolution.get(uri) != new_resolution
        )
        if doc_force_reanalyse and loop is not None:
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
                force_reanalyse=doc_force_reanalyse,
            )

    if diags_were_enabled and not _state.feature_config.diagnostics_enabled:
        for uri, doc_state in _state.workspace_state.items():
            if doc_state.analysis is None:
                _publish_diags_to_client(uri, [], doc_state.version)


def _workspace_folder_uris_from_server() -> list[str]:
    """List workspace folder URIs from the pygls server (empty if none).

    pygls 2.x stores workspace folders in ``Workspace.folders`` as a
    ``dict[str, WorkspaceFolder]`` keyed by URI.
    """
    if _server is None:
        return []
    folders = getattr(_server.workspace, "folders", None)
    if folders is None:
        # Fall back to legacy attribute names from older pygls releases.
        folders = getattr(_server.workspace, "workspace_folders", None) or []
    if isinstance(folders, dict):
        iterable = folders.values()
    else:
        iterable = folders
    return [getattr(f, "uri", "") for f in iterable if getattr(f, "uri", "")]


def _pull_and_apply_configuration() -> None:
    """Pull tclLsp config per workspace folder + the workspace fallback."""
    folder_uris = _workspace_folder_uris_from_server()
    items: list[types.ConfigurationItem] = [
        types.ConfigurationItem(scope_uri=uri, section="tclLsp") for uri in folder_uris
    ]
    # Always include the unscoped fallback (workspace/user settings) — used
    # for files outside any workspace folder.
    items.append(types.ConfigurationItem(section="tclLsp"))
    params = types.ConfigurationParams(items=items)

    def _on_result(result: Sequence[Any | None]) -> None:
        if not result:
            return
        # Result order matches request order: per-folder items first,
        # then the fallback item last.  Update every editor layer in
        # one batch so the merged-settings application runs once with
        # all folders populated — calling ``_apply_all_settings`` per
        # folder would fire the leading-edge debounce repeatedly and
        # leave intermediate states briefly visible.
        any_applied = False
        for folder_uri, item in zip(folder_uris, result, strict=False):
            if isinstance(item, dict):
                _state.editor_config_settings_per_folder[folder_uri] = item
                any_applied = True
        if len(result) > len(folder_uris):
            fallback_item = result[len(folder_uris)]
            if isinstance(fallback_item, dict):
                _state.editor_config_settings = fallback_item
                any_applied = True
        if any_applied:
            _schedule_apply_merged()

    try:
        _require_server().workspace_configuration(params, callback=_on_result)
    except Exception:
        log.debug("workspace/configuration pull failed", exc_info=True)


# Registration


def _deep_merge_into(dst: dict, src: dict) -> None:
    """Recursively merge ``src`` into ``dst`` in place.

    Nested ``dict`` values are merged key-by-key so a push that touches
    ``{"features": {"references": False}}`` doesn't clobber an unrelated
    ``features.hover`` setting living in ``dst``.  Scalar / list values
    in ``src`` replace whatever is in ``dst``.
    """
    for key, value in src.items():
        if isinstance(value, dict) and isinstance(dst.get(key), dict):
            _deep_merge_into(dst[key], value)
        else:
            dst[key] = copy.deepcopy(value)


# Keys whose value is genuinely per-folder rather than workspace-merged:
# the workspace ``didChangeConfiguration`` push carries the WORKSPACE-level
# value, but each folder may override these via its own
# ``.vscode/settings.json`` / multi-root folder configuration, and the
# authoritative per-folder value only arrives via the ``workspace/configuration``
# pull.  ``_propagate_push_to_folder_caches`` must NOT clobber an existing
# per-folder cache entry for these keys -- doing so flips per-folder state
# (notably ``FeatureConfig.dialect_explicitly_set``) in ways the pull
# cannot undo, breaking ``did_open``'s iRules / iApps auto-switch and the
# ``Dialect Detection defaults .tcl files to Tcl 8.6`` regression on PR #415.
_PER_FOLDER_OVERRIDE_KEYS: frozenset[str] = frozenset(
    {
        "dialect",
        "extraCommands",
        "extra_commands",
        "libraryPaths",
        "library_paths",
    }
)


def _propagate_push_to_folder_caches(extracted: dict) -> None:
    """Deep-merge a workspace-level push into every per-folder editor cache.

    ``vscode-languageclient`` pushes ``workspace/didChangeConfiguration``
    with the new workspace-merged ``tclLsp.*`` payload, but only updates
    the workspace-fallback layer (:data:`editor_config_settings`).  Each
    folder's ``editor_config_settings_per_folder`` entry is the snapshot
    of its last ``workspace/configuration`` pull response — the pull
    that would refresh those entries is async and trails the push.

    Between push and the matching pull, ``_apply_merged_settings_now``'s
    folder loop reads each folder's stale per-folder cache and applies
    it on top of the workspace fallback, so per-folder ``FeatureConfig``
    attributes (and any other key checked at handler-entry time, not
    just the dialect-baked ones #407 already covers) keep their previous
    value.  Under CPU pressure the pull can take longer than the 500 ms
    most tests wait, surfacing as flakes like the three
    ``configSettings.test.ts`` toggles in #415.

    Surgically deep-merge the push payload into every existing
    per-folder editor cache so the leading-edge apply propagates the
    new workspace-merged values to per-folder configs immediately.  The
    trailing pull still arrives and overwrites with the authoritative
    per-folder values.

    Keys in :data:`_PER_FOLDER_OVERRIDE_KEYS` (``dialect`` /
    ``extraCommands`` / ``libraryPaths``) are intentionally excluded:
    the workspace push only carries the workspace-level value for these,
    but each folder may override them, and the pull is the authoritative
    refresh.  Propagating them here would clobber a folder's existing
    cached override AND flip ``FeatureConfig.dialect_explicitly_set``
    via :func:`_apply_feature_settings`'s ``looks_inherited`` branch --
    which would block the iRules / iApps auto-switch in ``did_open``,
    regressing the ``Dialect Detection defaults .tcl files to Tcl 8.6``
    case the VS Code suite covers.  The dialect / extras / libraryPaths
    races that #407 patched land via the pull's full-section payload, so
    they are not regressed by this exclusion.
    """
    if not extracted:
        return
    propagatable = {
        key: value for key, value in extracted.items() if key not in _PER_FOLDER_OVERRIDE_KEYS
    }
    if not propagatable:
        return
    for folder_uri in list(_state.editor_config_settings_per_folder.keys()):
        if folder_uri == "":
            continue
        existing = _state.editor_config_settings_per_folder.get(folder_uri)
        if not isinstance(existing, dict):
            existing = {}
        _deep_merge_into(existing, propagatable)
        _state.editor_config_settings_per_folder[folder_uri] = existing


def register(server_instance: LanguageServer) -> None:
    """Register the didChangeConfiguration handler."""

    @server_instance.feature(types.WORKSPACE_DID_CHANGE_CONFIGURATION)
    def did_change_configuration(params: types.DidChangeConfigurationParams) -> None:
        # Push payloads come from two sources:
        #
        # 1. ``vscode-languageclient`` ``synchronize.configurationSection``:
        #    the workspace-merged value of ``tclLsp.*``.  We apply to the
        #    fallback layer and then pull per-folder via
        #    ``workspace/configuration`` for folder-scoped overrides.
        #
        # 2. The extension's ``setServerDialect`` notification: a hand-
        #    crafted ``{tclLsp: {dialect: "..."}}`` payload that is *not*
        #    backed by the client's configuration store (auto-detected
        #    from shebang / file extension / directive).  Pulling after
        #    applying it would clobber the dialect with whatever the
        #    client has stored.
        #
        # Detect (2) by checking whether the extracted settings are a
        # dialect-only payload — in that case skip the re-pull.  Empty
        # payloads (e.g. clients that signal "something changed") fall
        # through to the pull.
        settings = params.settings
        if isinstance(settings, dict) and settings:
            extracted = _extract_tcl_lsp_settings(settings)
            # Propagate the push into per-folder editor caches *before*
            # scheduling the apply so the leading-edge fire sees fresh
            # folder layers instead of stale pull snapshots.  See
            # ``_propagate_push_to_folder_caches`` for the race detail.
            _propagate_push_to_folder_caches(extracted)
            _apply_all_settings(extracted, folder_uri="")
            if set(extracted.keys()) == {"dialect"}:
                return
        _pull_and_apply_configuration()
