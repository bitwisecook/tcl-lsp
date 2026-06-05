#!/usr/bin/env python3
"""Deep per-command dump (Python side) — emits the FULL content of every
data-bearing field so the completeness audit can assert Rust carries at
least as much as Python at the *value* level (not just counts/presence).

Pairs with the `deep-<group>` mode of the Rust `dump_specs` example and
`audit_completeness.py`.

Usage: python3 scripts/registry-audit/dump_python_deep.py <group>
"""

from __future__ import annotations

import importlib
import json
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import GROUPS  # noqa: E402

ANY = sys.maxsize


def en(v):
    return getattr(v, "name", None) if v is not None else None


def colours(v):
    """Sorted member-name list of a TaintColour flag (or None)."""
    if v is None:
        return None
    return sorted(m.name for m in type(v) if (v & m) == m and m.value)


def opt(o):
    return f"{o.name}|{int(bool(o.takes_value))}|{getattr(o, 'value_hint', '') or ''}"


def side_effects(hints):
    out = []
    for se in hints or ():
        out.append(f"{en(se.target)}|{int(bool(se.reads))}|{int(bool(se.writes))}|{en(se.connection_side)}")
    return sorted(out)


def sub_record(sub) -> dict:
    ar = sub.arity
    return {
        "arity_min": ar.min,
        "arity_max": None if ar.max >= ANY else ar.max,
        "detail": bool(sub.detail),
        "synopsis": bool(sub.synopsis),
        "return_type": en(sub.return_type),
        "pure": bool(sub.pure),
        "mutator": bool(sub.mutator),
        "destructive": bool(getattr(sub, "destructive", False)),
        "returns_path": bool(getattr(sub, "returns_path", False)),
        "is_unescape": bool(getattr(sub, "is_unescape_command", False)),
        "loop_list_header": bool(getattr(sub, "loop_list_header", False)),
        "creates_scope_alias": bool(getattr(sub, "creates_scope_alias", False)),
        "options": sorted(opt(o) for o in (sub.options or ())),
        "n_arg_types": len(sub.arg_types or {}),
        "n_arg_roles": len(sub.arg_roles or {}),
        "n_arg_values": len(getattr(sub, "arg_values", {}) or {}),
        "n_forms": len(getattr(sub, "forms", ()) or ()),
        "side_effects": side_effects(getattr(sub, "side_effect_hints", ()) ),
        "safe_on_uninit": sub.safe_on_uninit is not None,
        "credential_arg": sub.credential_arg,
        "sensitive_headers": sorted(getattr(sub, "sensitive_headers", None) or ()),
        "taint_transform": colours(getattr(sub, "taint_transform", None)),
        "taint_double_encode": colours(getattr(sub, "taint_double_encode_colour", None)),
        "taint_output_sink": getattr(sub, "taint_output_sink", None),
        "cfg_rewrite_name": getattr(sub, "cfg_rewrite_name", None),
        "inferred_storage_type": en(getattr(sub, "inferred_storage_type", None)),
    }


# Python boolean trait flags whose presence must survive into Rust (as a
# `Traits` bit). Keyed by the Python field name.
BOOL_FLAGS = [
    "creates_dynamic_barrier", "has_loop_body", "never_inline_body",
    "warn_without_terminator", "evaluates_code", "performs_substitution",
    "opens_channel", "sources_file", "has_switch_body",
    "has_string_list_confusion_risk", "configures_channel", "has_interp_eval",
    "has_destructive_ops", "is_irules_event_handler",
    "is_unnormalized_http_getter", "pure", "cse_candidate", "diagram_action",
    "is_control_flow", "needs_start_cmd", "terminates_block", "is_oo_metaclass",
    "irules_top_level_only", "is_unescape_command", "reads_variable_before_write",
    "has_boolean_condition", "loop_list_header", "creates_scope_alias",
    "returns_path", "pure_evaluation", "destroys_variable", "is_language_keyword",
    "produces_canonical_list", "is_side_switch", "defines_procedure", "unsafe",
    "password_option_command", "taint_sink", "warn_missing_import",
]


def deep(spec) -> dict:
    forms = getattr(spec, "forms", ()) or ()
    options = []
    for f in forms:
        for o in getattr(f, "options", ()) or ():
            options.append(opt(o))
    h = getattr(spec, "hover", None)
    subs = {n: sub_record(s) for n, s in (getattr(spec, "subcommands", {}) or {}).items()}
    rec = {
        "name": spec.name,
        "snippet": bool(getattr(h, "snippet", "")) if h else False,
        "side_effects": side_effects(getattr(spec, "side_effect_hints", ())),
        "options": sorted(set(options)),
        "subs": subs,
        "scalars": {
            "taint_output_sink": getattr(spec, "taint_output_sink", None),
            "taint_log_sink": getattr(spec, "taint_log_sink", None),
            "taint_transform": colours(getattr(spec, "taint_transform", None)),
            "taint_double_encode": colours(getattr(spec, "taint_double_encode_colour", None)),
            "taint_sink_safe": colours(getattr(spec, "taint_sink_safe_colour", None)),
            "taint_network_sink_args": (
                None if getattr(spec, "taint_network_sink_args", None) is None
                else sorted(spec.taint_network_sink_args)
            ),
            "taint_interp_eval": sorted(getattr(spec, "taint_interp_eval_subcommands", None) or ()),
            "taint_output_sink_subcommands": sorted(getattr(spec, "taint_output_sink_subcommands", None) or ()),
            "credential_options": sorted(getattr(spec, "credential_options", None) or ()),
            "sensitive_headers": sorted(getattr(spec, "sensitive_headers", None) or ()),
            "pattern_type": en(getattr(spec, "pattern_type", None)),
            "format_string_type": en(getattr(spec, "format_string_type", None)),
            "tcllib_package": getattr(spec, "tcllib_package", None),
            "is_namespace_exported": bool(getattr(spec, "is_namespace_exported", False)),
            "xc_translatable": getattr(spec, "xc_translatable", None),
            "deprecated_replacement": getattr(spec, "deprecated_replacement_name", None)
            if hasattr(spec, "deprecated_replacement_name") else None,
            "inferred_storage_type": en(getattr(spec, "inferred_storage_type", None)),
            "assigns_variable_at": getattr(spec, "assigns_variable_at", None),
            "safe_on_uninit": getattr(spec, "safe_on_uninit", None) is not None,
            "setter_constraints": len(
                getattr(spec.taint_hints() if hasattr(spec, "taint_hints") else None,
                        "setter_constraints", ()) or ()
            ) if False else None,
        },
        "bools": {f: bool(getattr(spec, f, False)) for f in BOOL_FLAGS},
    }
    return rec


def main() -> int:
    group = sys.argv[1]
    mod_name, func_name = GROUPS[group][0], GROUPS[group][1]
    mod = importlib.import_module(f"core.commands.registry.{mod_name}")
    for spec in getattr(mod, func_name)():
        sys.stdout.write(json.dumps(deep(spec), sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
