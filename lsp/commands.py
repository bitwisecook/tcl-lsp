"""Workspace command handlers for the Tcl LSP server."""

from __future__ import annotations

import asyncio
import logging
from typing import TYPE_CHECKING

from lsprotocol import types
from pygls.lsp.server import LanguageServer

import lsp.state as _state
from core.analysis import analyse
from core.analysis.irules_checks import DEFAULT_GENERIC_VARIABLE_PATTERNS
from core.commands.registry import REGISTRY
from core.commands.registry.info import effective_event_requires
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import configure_signatures
from core.common.document_buffer import DocumentBuffer
from core.common.optimisation_profiles import (
    DEFAULT_ACTION_PROFILE,
    profile_from_name,
    profile_to_disabled,
    resolve_profile,
)
from core.common.user_config import save_settings_to_config
from core.compiler.optimiser import optimise_source
from core.minifier import minify_tcl
from explorer.pipeline import run_pipeline as explorer_run_pipeline
from explorer.serialise import serialise_result as explorer_serialise_result

from .feature_config import FeatureConfig
from .features.diagnostics import get_basic_diagnostics
from .features.package_suggestions import rank_package_suggestions
from .workspace.scanner import uri_to_path

if TYPE_CHECKING:
    pass

log = logging.getLogger(__name__)

_server: LanguageServer | None = None

_DIALECT_COMMAND = "tcl-lsp.setDialect"
_EXPORT_CONFIG_COMMAND = "tcl-lsp.exportConfig"
_DIALECT_LABELS = {
    "tcl8.4": "Tcl 8.4",
    "tcl8.5": "Tcl 8.5",
    "tcl8.6": "Tcl 8.6",
    "tcl9.0": "Tcl 9.0",
    "f5-irules": "F5 iRules",
    "f5-iapps": "F5 iApps",
    "eda-tools": "EDA Tools",
}


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


def on_optimise_document(uri: str, profile: str = "full") -> dict | None:
    from core.compiler.optimiser import optimise_source_multipass

    source = _state._get_doc_source(uri)
    disabled, multi_pass, max_iterations = resolve_profile(
        profile,
        default=DEFAULT_ACTION_PROFILE,
    )
    if multi_pass:
        optimised, opts, _iters = optimise_source_multipass(
            source,
            max_iterations=max_iterations,
            disabled=disabled,
        )
    else:
        optimised, opts = optimise_source(source, disabled=disabled)
    items = []
    for o in opts:
        item: dict = {
            "code": o.code,
            "message": o.message,
            "startLine": o.range.start.line,
            "startCharacter": o.range.start.character,
            "endLine": o.range.end.line,
            "endCharacter": o.range.end.character,
            "replacement": o.replacement,
        }
        if o.group is not None:
            item["group"] = o.group
        if o.hint_only:
            item["hintOnly"] = True
        items.append(item)
    return {
        "optimisations": items,
        "source": optimised,
    }


def on_minify_document(
    uri: str, compact: bool = False, aggressive: bool = False, isolated: bool = False
) -> dict | None:
    """Minify the Tcl document: strip comments, collapse whitespace, join commands."""
    source = _state._get_doc_source(uri)
    if aggressive:
        result = minify_tcl(source, aggressive=True, isolated=isolated)
        return {
            "source": result.source,
            "originalLength": result.original_length,
            "minifiedLength": result.minified_length,
            "symbolMap": result.symbol_map.format(),
            "optimisationsApplied": result.optimisations_applied,
        }
    if compact:
        minified, symbol_map = minify_tcl(source, compact_names=True, isolated=isolated)
        return {
            "source": minified,
            "originalLength": len(source),
            "minifiedLength": len(minified),
            "symbolMap": symbol_map.format(),
        }
    minified = minify_tcl(source)
    return {
        "source": minified,
        "originalLength": len(source),
        "minifiedLength": len(minified),
    }


def on_unminify_error(
    error_message: str,
    symbol_map: str,
    minified_source: str = "",
    original_source: str = "",
) -> dict:
    """Translate a minified-code error message back to original names."""
    from core.minifier import unminify_error

    translated = unminify_error(
        error_message,
        symbol_map=symbol_map,
        minified_source=minified_source or None,
        original_source=original_source or None,
    )
    return {
        "originalError": error_message,
        "translatedError": translated,
        "changed": translated != error_message,
    }


def on_describe_irule_event(event_name: str) -> dict:
    """Return deterministic registry metadata for an iRules event."""
    event = (event_name or "").strip()
    when_values = REGISTRY.argument_values("when", 0, "f5-irules")
    known_events = {value.value for value in when_values}
    deprecated_events = {
        value.value for value in when_values if "deprecated" in (value.detail or "").lower()
    }

    is_known = event in known_events
    if not event or not is_known:
        valid_commands: list[str] = []
    else:
        event_set = REGISTRY.commands_for_event("f5-irules", event)
        valid_commands = sorted(event_set.valid_commands)
    return {
        "event": event,
        "known": is_known,
        "deprecated": event in deprecated_events,
        "validCommandCount": len(valid_commands),
        "sampleCommands": valid_commands[:80],
    }


def on_describe_irule_command(command_name: str) -> dict:
    """Return deterministic registry metadata for an iRules command."""
    name = (command_name or "").strip()
    spec = REGISTRY.get(name, "f5-irules")
    if spec is None and name:
        lowered = name.lower()
        for candidate in REGISTRY.command_names("f5-irules"):
            if candidate.lower() == lowered:
                spec = REGISTRY.get(candidate, "f5-irules")
                name = candidate
                break

    if spec is None:
        return {
            "found": False,
            "command": name,
        }

    synopsis = tuple(spec.hover.synopsis) if spec.hover and spec.hover.synopsis else ()

    result: dict = {
        "found": True,
        "command": name,
        "summary": spec.hover.summary if spec.hover else "",
        "synopsis": list(synopsis),
        "switches": list(spec.switch_names()),
    }

    requires = effective_event_requires(name, spec.event_requires, dialect="f5-irules")
    if requires is not None:
        result["validEvents"] = EVENT_REGISTRY.events_matching(requires)
        result["anyEvent"] = not (
            requires.client_side
            or requires.server_side
            or requires.transport is not None
            or bool(requires.profiles)
            or bool(requires.also_in)
            or requires.init_only
            or requires.flow
            or requires.capability is not None
        )
        result["eventRequires"] = {
            "clientSide": requires.client_side,
            "serverSide": requires.server_side,
            "transport": requires.transport,
            "profiles": sorted(requires.profiles),
        }

    return result


def _apply_non_overlapping_fixes(
    source: str,
    fixes: list[tuple[int, int, str, str, str]],
) -> tuple[str, list[dict[str, str]]]:
    """Apply non-overlapping text replacements using source offsets."""
    valid = [fix for fix in fixes if 0 <= fix[0] <= fix[1] < len(source)]
    valid.sort(key=lambda item: (item[0], item[1]))

    accepted: list[tuple[int, int, str, str, str]] = []
    current_end = -1
    for fix in valid:
        start, end, _text, _code, _description = fix
        if start <= current_end:
            continue
        accepted.append(fix)
        current_end = end

    rewritten = source
    applied: list[dict[str, str]] = []
    for start, end, new_text, code, description in reversed(accepted):
        rewritten = rewritten[:start] + new_text + rewritten[end + 1 :]
        applied.append(
            {
                "code": code,
                "description": description or "Apply safe fix",
            }
        )
    applied.reverse()
    return rewritten, applied


def _collect_pending_safe_fixes(
    source: str,
) -> list[tuple[int, int, str, str, str]]:
    """Collect first safe fix per diagnostic for *source*."""
    analysis = analyse(source)
    pending: list[tuple[int, int, str, str, str]] = []
    for diagnostic in analysis.diagnostics:
        if diagnostic.code not in _state._SAFE_FIX_CODES:
            continue
        if not diagnostic.fixes:
            continue
        fix = diagnostic.fixes[0]
        pending.append(
            (
                fix.range.start.offset,
                fix.range.end.offset,
                fix.new_text,
                diagnostic.code,
                fix.description,
            )
        )
    return pending


def _apply_safe_fixes_iteratively(
    source: str,
    *,
    max_passes: int = 4,
) -> tuple[str, list[dict[str, str]]]:
    """Apply safe fixes in multiple passes until source stabilises."""
    rewritten = source
    applied_total: list[dict[str, str]] = []
    for _ in range(max_passes):
        pending = _collect_pending_safe_fixes(rewritten)
        if not pending:
            break
        next_source, applied = _apply_non_overlapping_fixes(rewritten, pending)
        if not applied or next_source == rewritten:
            break
        applied_total.extend(applied)
        rewritten = next_source
    return rewritten, applied_total


def on_fix_all_safe_issues(uri: str) -> dict | None:
    """Apply all non-overlapping safe code fixes for a document."""
    source = _state._get_doc_source(uri)
    rewritten, applied = _apply_safe_fixes_iteratively(source)
    return {
        "source": rewritten,
        "applied": applied,
    }


def on_list_irule_events() -> dict:
    """Return all known iRules event names from the registry."""
    when_values = REGISTRY.argument_values("when", 0, "f5-irules")
    events = sorted(v.value for v in when_values if v.value)
    return {
        "events": events,
        "count": len(events),
    }


def on_list_subcommands(command_name: str) -> dict:
    """Return subcommand metadata for a command from the registry."""
    name = (command_name or "").strip()
    spec = REGISTRY.get_any(name)
    if spec is None or not spec.subcommands:
        return {"command": name, "subcommands": []}
    return {
        "command": name,
        "subcommands": [
            {
                "name": sub.name,
                "detail": sub.detail or "",
                "synopsis": sub.synopsis or "",
                "pure": sub.pure,
                "mutator": sub.mutator,
                "deprecated": sub.deprecated_replacement is not None,
            }
            for sub in sorted(spec.subcommands.values(), key=lambda s: s.name)
        ],
    }


def on_list_known_packages() -> dict:
    """Return all package names discovered by PackageResolver."""
    return {
        "packages": sorted(_state.package_resolver.all_package_names()),
    }


def on_suggest_packages_for_symbol(symbol: str) -> dict:
    """Suggest package names for a symbol/command heuristic."""
    query = (symbol or "").strip()
    if not query:
        return {"symbol": query, "suggestions": []}

    suggestions = rank_package_suggestions(
        query,
        _state.package_resolver.all_package_names(),
        20,
    )
    return {
        "symbol": query,
        "suggestions": suggestions,
    }


def on_search_help(query: str = "", include_screenshots: bool = False) -> dict:
    """Search the KCS help database for features and documentation."""
    try:
        from core.help.kcs_db import (
            get_feature,
            get_screenshot_base64,
            list_features,
            search_help,
        )
    except Exception:
        return {"error": "KCS help database not available"}

    if query:
        results = search_help(query)
        if not results:
            feat = get_feature(query)
            if feat:
                results = [feat]

        screenshots: dict = {}
        if include_screenshots:
            for r in results:
                from core.help.kcs_db import list_screenshots_for_feature

                for ss in list_screenshots_for_feature(r.get("file", "")):
                    if ss["has_image"]:
                        img = get_screenshot_base64(ss["ref_id"])
                        if img:
                            screenshots[ss["ref_id"]] = img

        return {"results": results, "screenshots": screenshots}

    catalogue = list_features()
    return {"catalogue": catalogue}


def on_compiler_explorer(source: str, dialect: str) -> dict | None:
    """Run the full compiler explorer pipeline and return serialised JSON."""
    if not source or not source.strip():
        return {
            "error": "No source was received from the editor.",
            "details": "Open a Tcl/iRule file in the active editor and try again.",
        }
    try:
        result = explorer_run_pipeline(source, dialect=dialect or None)
        if not result.snapshots:
            basic_diags, _, _ = get_basic_diagnostics(
                source,
                optimiser_enabled=False,
                disabled_diagnostics=_state.feature_config.disabled_diagnostics,
                disabled_optimisations=_state.feature_config.disabled_optimisations,
                line_length=_state.feature_config.line_length,
            )
            error_diags = [
                diagnostic
                for diagnostic in basic_diags
                if diagnostic.severity == types.DiagnosticSeverity.Error
            ]
            if error_diags:
                first = error_diags[0]
                code = first.code if isinstance(first.code, str) else "E000"
                line = first.range.start.line + 1
                col = first.range.start.character + 1
                details = f"{code} at line {line}, column {col}: {first.message}"
                diagnostics = []
                for diagnostic in error_diags[:5]:
                    diag_code = diagnostic.code if isinstance(diagnostic.code, str) else "E000"
                    diagnostics.append(
                        {
                            "code": diag_code,
                            "message": diagnostic.message,
                            "line": diagnostic.range.start.line + 1,
                            "column": diagnostic.range.start.character + 1,
                        }
                    )
                return {
                    "error": "Source script issues prevented compiler explorer output.",
                    "details": details,
                    "diagnostics": diagnostics,
                }
            return {
                "error": "Compiler explorer could not build IR from the active source.",
                "details": "No compilable Tcl/iRule commands were found in the current editor.",
            }
        return explorer_serialise_result(result)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


def on_tk_preview(source: str) -> dict | None:
    """Extract Tk widget tree from source code for GUI preview."""
    if not source or not source.strip():
        return None
    try:
        from core.tk.extract import extract_tk_layout

        return extract_tk_layout(source)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


def on_diagram_data(source: str) -> dict | None:
    """Extract structured flow data from an iRule for diagram generation."""
    if not source or not source.strip():
        return None
    try:
        from core.diagram.extract import extract_diagram_data

        return extract_diagram_data(source)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


def on_xc_translate(source: str, output_format: str = "both") -> dict | None:
    """Translate an iRule to F5 XC configuration."""
    if not source or not source.strip():
        return None
    try:
        from core.xc.json_api import render_json
        from core.xc.terraform import render_terraform
        from core.xc.translator import translate_irule

        configure_signatures(dialect="f5-irules")
        result = translate_irule(source)

        output: dict = {}
        if output_format in ("terraform", "both"):
            output["terraform"] = render_terraform(result)
        if output_format in ("json", "both"):
            output["json_api"] = render_json(result)

        output["coverage_pct"] = result.coverage_pct
        output["translatable_count"] = result.translatable_count
        output["partial_count"] = result.partial_count
        output["untranslatable_count"] = result.untranslatable_count
        output["advisory_count"] = result.advisory_count
        output["items"] = [
            {
                "status": item.status.name.lower(),
                "kind": item.kind.name.lower(),
                "command": item.irule_command,
                "xc_description": item.xc_description,
                "note": item.note,
                "diagnostic_code": item.diagnostic_code,
            }
            for item in result.items
        ]
        return output
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


def on_extract_rule(uri: str, offset: int) -> dict | None:
    """Find the ``ltm rule`` / ``gtm rule`` containing *offset* and return it."""
    from core.bigip.rule_extract import find_rule_at_offset

    try:
        doc = _server.workspace.get_text_document(uri)  # type: ignore[union-attr]
    except Exception:
        return None
    rule = find_rule_at_offset(doc.source, offset)
    if rule is None:
        return None
    return {
        "name": rule.name,
        "fullPath": rule.full_path,
        "body": rule.body,
        "bodyStartOffset": rule.body_start_offset,
        "bodyEndOffset": rule.body_end_offset,
        "uri": uri,
    }


def on_list_rules(uri: str) -> list[dict] | None:
    """Return all ``ltm rule`` / ``gtm rule`` blocks in the given document."""
    from core.bigip.rule_extract import find_embedded_rules

    try:
        doc = _server.workspace.get_text_document(uri)  # type: ignore[union-attr]
    except Exception:
        return None
    rules = find_embedded_rules(doc.source)
    return [
        {
            "name": r.name,
            "fullPath": r.full_path,
            "body": r.body,
            "bodyStartOffset": r.body_start_offset,
            "bodyEndOffset": r.body_end_offset,
            "blockStartLine": r.range.start.line,
            "uri": uri,
        }
        for r in rules
    ]


def on_extract_linked_objects(
    uri: str,
    offset: int,
    max_depth: int = 4,
    max_nodes: int = 250,
    extra_offsets: list | None = None,
) -> dict | None:
    """Return a transitive BIG-IP object subgraph around one or more cursors."""
    from core.bigip.link_extract import extract_linked_bigip_objects

    seed_positions: list[tuple[str, int]] = [(uri, offset)]
    if extra_offsets:
        for item in extra_offsets:
            if isinstance(item, (list, tuple)) and len(item) >= 2:
                seed_positions.append((str(item[0]), int(item[1])))

    seed_uris: set[str] = {u for u, _ in seed_positions}

    configs = _state.background_scanner.bigip_configs
    for seed_uri in seed_uris:
        if seed_uri in configs:
            continue
        try:
            src = _state._get_doc_source(seed_uri)
        except Exception:
            return None
        parsed = _state.background_scanner.parse_bigip_source(seed_uri, src)
        if parsed is None:
            return None
        configs = _state.background_scanner.bigip_configs

    sources: dict[str, str] = {}
    for cfg_uri in configs:
        try:
            sources[cfg_uri] = _state._get_doc_source(cfg_uri)
            continue
        except Exception:
            pass

        path = uri_to_path(cfg_uri)
        if not path:
            continue
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                sources[cfg_uri] = handle.read()
        except OSError:
            continue

    for seed_uri in seed_uris:
        if seed_uri not in sources:
            try:
                sources[seed_uri] = _state._get_doc_source(seed_uri)
            except Exception:
                return None

    return extract_linked_bigip_objects(
        offsets=seed_positions,
        sources=sources,
        configs=configs,
        max_depth=max_depth,
        max_nodes=max_nodes,
    )


def on_bigip_cleanup(
    uris: list | None = None,
    *,
    keep: list | None = None,
    no_keep_common: bool = False,
) -> dict | None:
    """Generate a tmsh cleanup script for unreferenced BIG-IP objects.

    *uris* selects which workspace BIG-IP configurations to analyse.  When
    empty / ``None`` every BIG-IP config the workspace scanner has parsed
    is included.  Returns a JSON-serialisable report (see
    :func:`core.bigip.cleanup.report_to_dict`) or ``None`` if no BIG-IP
    config is loaded.
    """
    from core.bigip.cleanup import compute_cleanup, report_to_dict

    configs = _state.background_scanner.bigip_configs
    if not configs:
        return None

    selected_uris: list[str]
    if uris:
        selected_uris = [u for u in uris if u in configs]
        if not selected_uris:
            return None
    else:
        selected_uris = list(configs.keys())

    sources: dict[str, str] = {}
    selected_configs = {}
    for cfg_uri in selected_uris:
        try:
            sources[cfg_uri] = _state._get_doc_source(cfg_uri)
        except Exception:
            path = uri_to_path(cfg_uri)
            if not path:
                continue
            try:
                with open(path, encoding="utf-8", errors="replace") as handle:
                    sources[cfg_uri] = handle.read()
            except OSError:
                continue
        selected_configs[cfg_uri] = configs[cfg_uri]

    if not sources:
        return None

    keep_paths: set[str] = set()
    extra_prefixes: set[str] = set()
    for entry in keep or []:
        if isinstance(entry, str) and entry.endswith("/"):
            extra_prefixes.add(entry)
        elif isinstance(entry, str):
            keep_paths.add(entry)
    if no_keep_common:
        keep_partitions = frozenset(extra_prefixes)
    else:
        keep_partitions = frozenset(extra_prefixes | {"/Common/"})

    report = compute_cleanup(
        sources=sources,
        configs=selected_configs,
        keep_paths=frozenset(keep_paths),
        keep_partitions=keep_partitions,
    )
    return report_to_dict(report)


def on_write_rule_back(
    uri: str,
    body_start_offset: int,
    body_end_offset: int,
    new_body: str,
) -> bool:
    """Replace the iRule body in the configuration file."""
    try:
        doc = _server.workspace.get_text_document(uri)  # type: ignore[union-attr]
        source = doc.source
    except Exception:
        return False

    buf = DocumentBuffer.from_source(source)
    start_pos = buf.offset_to_position(body_start_offset)
    end_pos = buf.offset_to_position(body_end_offset)
    start = types.Position(line=start_pos.line, character=start_pos.character)
    end = types.Position(line=end_pos.line, character=end_pos.character)

    edit = types.WorkspaceEdit(
        changes={
            uri: [
                types.TextEdit(
                    range=types.Range(start=start, end=end),
                    new_text=new_body,
                )
            ]
        }
    )
    _server.workspace_apply_edit(  # type: ignore[union-attr]
        types.ApplyWorkspaceEditParams(edit=edit, label="Write iRule back to config")
    )
    return True


def on_tclpkg_install(package_name: str, uri: str = "") -> dict:
    """Install a Tcl package via the tclpkg manifest (stub)."""
    return {
        "success": False,
        "message": f"tclpkg install for '{package_name}' not yet wired (use the CLI: tcl pkg add {package_name})",
    }


def on_tclpkg_search(query: str) -> dict:
    """Search the tclpkg registry for packages matching *query*."""
    try:
        from core.common.user_config import _cache_dir
        from tclpkg.registry import RegistryClient

        client = RegistryClient(_cache_dir(), offline=True)
        results = client.search(query)
        return {"results": [{"name": e.name, "description": e.description} for e in results[:20]]}
    except Exception as exc:
        return {"results": [], "error": str(exc)}


def on_set_dialect(dialect: str = "") -> dict:
    """Switch the active Tcl dialect."""
    return _switch_dialect(dialect)


def on_export_config() -> dict:
    """Export the current server configuration."""
    return _export_config()


def _switch_dialect(dialect: str) -> dict:
    """Switch the active dialect and re-publish diagnostics."""
    from core.commands.registry.dialects import KNOWN_DIALECTS
    from core.common.dialect import active_dialect

    if dialect and dialect not in KNOWN_DIALECTS:
        return {"success": False, "error": f"Unknown dialect: {dialect!r}"}

    prev = active_dialect()
    changed = configure_signatures(dialect=dialect or None)
    current = active_dialect()
    _state.feature_config.dialect_explicitly_set = True
    log.info("Dialect set to %s (was %s)", current, prev)

    if changed:
        _server.window_show_message(  # type: ignore[union-attr]
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=f"Switched dialect to {_DIALECT_LABELS.get(current, current)}.",
            )
        )
        _state.diagnostic_scheduler.cancel_all()
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = None

        from lsp.diagnostics_pipeline import _publish_diagnostics, _publish_diagnostics_sync

        for uri, state in _state.workspace_state.items():
            if loop is not None:
                loop.create_task(
                    _publish_diagnostics(
                        uri,
                        state.source,
                        state.version,
                        force_reanalyse=True,
                    )
                )
            else:
                _publish_diagnostics_sync(
                    uri,
                    state.source,
                    state.version,
                    force_reanalyse=True,
                )

    return {"success": True, "dialect": current}


def _export_config() -> dict:
    """Export the current effective settings to the config file."""
    from lsp.settings import _ALL_DIAGNOSTIC_CODES, _ALL_OPTIMISATION_CODES, _FEATURE_TOGGLE_KEYS

    settings: dict[str, object] = {}

    diag: dict[str, object] = {}
    for code in _ALL_DIAGNOSTIC_CODES:
        if code in _state.feature_config.disabled_diagnostics:
            diag[code] = False
    current_patterns = list(_state.feature_config.generic_variable_patterns)
    if current_patterns and current_patterns != list(DEFAULT_GENERIC_VARIABLE_PATTERNS):
        diag["genericVariablePatterns"] = current_patterns
    if diag:
        settings["diagnostics"] = diag

    opt: dict[str, object] = {"enabled": _state.feature_config.optimiser_enabled}
    opt["profile"] = _state.feature_config.optimiser_profile
    profile_baseline = profile_to_disabled(
        profile_from_name(_state.feature_config.optimiser_profile)
    )
    for code in _ALL_OPTIMISATION_CODES:
        in_disabled = code in _state.feature_config.disabled_optimisations
        in_baseline = code in profile_baseline
        if in_disabled and not in_baseline:
            opt[code] = False
        elif not in_disabled and in_baseline:
            opt[code] = True
    settings["optimiser"] = opt

    settings["shimmer"] = {"enabled": _state.feature_config.shimmer_enabled}
    settings["xcDiagnostics"] = {"enabled": _state.feature_config.xc_diagnostics_enabled}

    feat: dict[str, object] = {}
    for json_key, attr in _FEATURE_TOGGLE_KEYS.items():
        feat[json_key] = getattr(_state.feature_config, attr)
    settings["features"] = feat

    settings["style"] = {"lineLength": _state.feature_config.line_length}

    default_cfg = FeatureConfig()
    default_features: dict[str, object] = {
        json_key: getattr(default_cfg, attr) for json_key, attr in _FEATURE_TOGGLE_KEYS.items()
    }
    defaults: dict[str, object] = {
        "features": default_features,
        "optimiser": {
            "enabled": default_cfg.optimiser_enabled,
            "profile": default_cfg.optimiser_profile,
        },
        "shimmer": {"enabled": default_cfg.shimmer_enabled},
        "xcDiagnostics": {"enabled": default_cfg.xc_diagnostics_enabled},
        "style": {"lineLength": default_cfg.line_length},
    }

    try:
        path = save_settings_to_config(settings, only_non_default=True, defaults=defaults)
        _server.window_show_message(  # type: ignore[union-attr]
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=f"Settings exported to {path}",
            )
        )
        return {"success": True, "path": path}
    except Exception as exc:
        log.error("Failed to export config", exc_info=True)
        return {"success": False, "error": str(exc)}


def register(server_instance: LanguageServer) -> None:
    """Register all workspace command handlers with the server."""
    configure(server_instance)
    server_instance.command("tcl-lsp.optimiseDocument")(on_optimise_document)
    server_instance.command("tcl-lsp.minifyDocument")(on_minify_document)
    server_instance.command("tcl-lsp.unminifyError")(on_unminify_error)
    server_instance.command("tcl-lsp.describeIruleEvent")(on_describe_irule_event)
    server_instance.command("tcl-lsp.describeIruleCommand")(on_describe_irule_command)
    server_instance.command("tcl-lsp.fixAllSafeIssues")(on_fix_all_safe_issues)
    server_instance.command("tcl-lsp.listIruleEvents")(on_list_irule_events)
    server_instance.command("tcl-lsp.listSubcommands")(on_list_subcommands)
    server_instance.command("tcl-lsp.listKnownPackages")(on_list_known_packages)
    server_instance.command("tcl-lsp.suggestPackagesForSymbol")(on_suggest_packages_for_symbol)
    server_instance.command("tcl-lsp.searchHelp")(on_search_help)
    server_instance.command("tcl-lsp.compilerExplorer")(on_compiler_explorer)
    server_instance.command("tcl-lsp.tkPreview")(on_tk_preview)
    server_instance.command("tcl-lsp.diagramData")(on_diagram_data)
    server_instance.command("tcl-lsp.xcTranslate")(on_xc_translate)
    server_instance.command("tcl-lsp.extractRule")(on_extract_rule)
    server_instance.command("tcl-lsp.listRules")(on_list_rules)
    server_instance.command("tcl-lsp.extractLinkedObjects")(on_extract_linked_objects)
    server_instance.command("tcl-lsp.bigipCleanup")(on_bigip_cleanup)
    server_instance.command("tcl-lsp.writeRuleBack")(on_write_rule_back)
    server_instance.command("tcl-lsp.tclpkg.install")(on_tclpkg_install)
    server_instance.command("tcl-lsp.tclpkg.search")(on_tclpkg_search)
    server_instance.command(_DIALECT_COMMAND)(on_set_dialect)
    server_instance.command(_EXPORT_CONFIG_COMMAND)(on_export_config)
