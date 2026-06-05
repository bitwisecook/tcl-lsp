#!/usr/bin/env python3
"""Registry-parity dumper (Python side).

Emits one JSON object per command spec (JSONL) for a given dialect
group, using a normalised schema shared with the Rust dumper
(``rust/tcl-registry/examples/dump_specs.rs``). Used by the rust-rewrite
registry audit to diff the Rust port against the Python source of truth.

Usage: python3 scripts/registry-audit/dump_python.py <group>
where <group> is one of:
  tcl stdlib tcllib irules iapps tk expect
  sdc-base synopsys cadence xilinx quartus mentor
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Allow running from anywhere: put the repo root (three levels up) on sys.path
# so `import core...` resolves.
_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

# group -> (module under core.commands.registry, factory function)
GROUPS: dict[str, tuple[str, str]] = {
    "tcl": ("tcl", "tcl_command_specs"),
    "stdlib": ("stdlib", "stdlib_command_specs"),
    "tcllib": ("tcllib", "tcllib_command_specs"),
    "irules": ("irules", "irules_command_specs"),
    "iapps": ("iapps", "iapps_command_specs"),
    "tk": ("tk", "tk_command_specs"),
    "expect": ("expect", "expect_command_specs"),
    "sdc-base": ("eda_sdc_base", "sdc_base_command_specs"),
    "synopsys": ("eda_synopsys", "synopsys_command_specs"),
    "cadence": ("eda_cadence", "cadence_command_specs"),
    "xilinx": ("eda_xilinx", "xilinx_command_specs"),
    "quartus": ("eda_quartus", "quartus_command_specs"),
    "mentor": ("eda_mentor", "mentor_command_specs"),
}

# Python Arity.ANY sentinel == sys.maxsize.
ANY = sys.maxsize


def _enum_name(v: object) -> str | None:
    return getattr(v, "name", None) if v is not None else None


def normalise(spec: object) -> dict:
    # dialects
    dialects = getattr(spec, "dialects", None)
    dialects_all = dialects is None
    dialect_list = sorted(dialects) if dialects else []

    # arity (under validation)
    validation = getattr(spec, "validation", None)
    arity = getattr(validation, "arity", None) if validation else None
    if arity is None:
        arity_min, arity_max = 0, None
    else:
        arity_min = arity.min
        arity_max = None if arity.max >= ANY else arity.max

    # hover
    hover = getattr(spec, "hover", None)
    if hover is None:
        has_hover = False
        summary, synopsis, source = "", [], ""
        examples = return_value = False
    else:
        has_hover = True
        summary = hover.summary or ""
        synopsis = sorted(hover.synopsis or ())
        source = hover.source or ""
        examples = bool(hover.examples)
        return_value = bool(hover.return_value)
    source_is_url = source.startswith("http")

    # options: Python keeps options under forms[].options
    forms = getattr(spec, "forms", ()) or ()
    option_names: set[str] = set()
    for f in forms:
        for opt in getattr(f, "options", ()) or ():
            option_names.add(opt.name)

    # subcommands: dict[str, SubCommand]
    subs = getattr(spec, "subcommands", {}) or {}
    subcommand_names = sorted(subs.keys())

    # side effects
    sehints = getattr(spec, "side_effect_hints", None) or ()
    n_side_effects = len(sehints)

    # event_requires
    er = getattr(spec, "event_requires", None)
    if er is None:
        event_profiles: list[str] = []
        event_also_in: list[str] = []
        event_requires_any = False
    else:
        event_profiles = sorted(er.profiles or ())
        event_also_in = sorted(er.also_in or ())
        event_requires_any = any(
            [
                er.client_side,
                er.server_side,
                er.transport is not None,
                bool(er.profiles),
                bool(er.also_in),
                er.init_only,
                er.flow,
                er.capability is not None,
            ]
        )

    excluded_events = sorted(getattr(spec, "excluded_events", ()) or ())
    required_package = getattr(spec, "required_package", None)
    return_type = _enum_name(getattr(spec, "return_type", None))
    arg_types = getattr(spec, "arg_types", {}) or {}
    arg_roles = getattr(spec, "arg_roles", {}) or {}
    body_kind = _enum_name(getattr(spec, "body_kind", None)) or "INLINE"

    # codegen-registry dimensions
    has_lowering = getattr(spec, "lowering", None) is not None
    has_codegen = bool(getattr(spec, "codegens", {}) or {})
    has_const_fold = getattr(spec, "const_fold", None) is not None
    # Python carries ~20 boolean trait flags rather than a bitset; treat the
    # command as "has traits" when any behavioural flag is set or it is pure.
    _trait_flags = [
        "creates_dynamic_barrier", "has_loop_body", "never_inline_body",
        "warn_without_terminator", "evaluates_code", "performs_substitution",
        "opens_channel", "sources_file", "has_switch_body",
        "has_string_list_confusion_risk", "configures_channel", "has_interp_eval",
        "has_destructive_ops", "is_irules_event_handler",
        "is_unnormalized_http_getter", "pure", "cse_candidate", "diagram_action",
    ]
    has_traits = any(bool(getattr(spec, f, False)) for f in _trait_flags)

    return {
        "name": spec.name,
        "dialects_all": dialects_all,
        "dialects": dialect_list,
        "arity_min": arity_min,
        "arity_max": arity_max,
        "hover": has_hover,
        "summary": summary,
        "synopsis": synopsis,
        "source": source,
        "source_is_url": source_is_url,
        "examples": examples,
        "return_value": return_value,
        "n_forms": len(forms),
        "options": sorted(option_names),
        "n_subcommands": len(subcommand_names),
        "subcommands": subcommand_names,
        "n_side_effects": n_side_effects,
        "event_profiles": event_profiles,
        "event_also_in": event_also_in,
        "event_requires_any": event_requires_any,
        "excluded_events": excluded_events,
        "required_package": required_package,
        "return_type": return_type,
        "n_arg_types": len(arg_types),
        "n_arg_roles": len(arg_roles),
        "body_kind": body_kind,
        "has_lowering": has_lowering,
        "has_codegen": has_codegen,
        "has_const_fold": has_const_fold,
        "has_traits": has_traits,
    }


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in GROUPS:
        sys.stderr.write(f"usage: dump_python.py <{'|'.join(GROUPS)}>\n")
        return 2
    group = sys.argv[1]
    mod_name, func_name = GROUPS[group]
    import importlib

    mod = importlib.import_module(f"core.commands.registry.{mod_name}")
    specs = getattr(mod, func_name)()
    for spec in specs:
        sys.stdout.write(json.dumps(normalise(spec), sort_keys=True) + "\n")
    sys.stderr.write(f"[dump_python] group={group} count={len(specs)}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
