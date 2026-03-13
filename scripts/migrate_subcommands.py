#!/usr/bin/env python3
"""Generate SubCommand dict entries from existing command registry data.

This script introspects the existing registry at runtime, merges data from
ValidationSpec, FormSpec, role_hints, type_hints, pure_subcommands, and
mutator_subcommands, and prints Python code for the `subcommands` dict
that can be added to each command's spec() method.

Usage:
    python scripts/migrate_subcommands.py [command_name]
    python scripts/migrate_subcommands.py          # all commands with subcommands
    python scripts/migrate_subcommands.py string   # just "string"
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from core.commands.registry.command_registry import REGISTRY
from core.commands.registry.models import OptionTerminatorSpec


def _find_command_def_classes():
    """Find all CommandDef subclasses across all dialect registries."""
    from core.commands.registry.iapps import iapps_command_defs
    from core.commands.registry.irules import irules_command_defs
    from core.commands.registry.stdlib import stdlib_command_defs
    from core.commands.registry.tcl import tcl_command_defs
    from core.commands.registry.tcllib import tcllib_command_defs
    from core.commands.registry.tk import tk_command_defs

    all_defs = {}
    for source_fn in [
        tcl_command_defs,
        irules_command_defs,
        iapps_command_defs,
        tcllib_command_defs,
        stdlib_command_defs,
        tk_command_defs,
    ]:
        try:
            for cmd_def_cls in source_fn():
                all_defs[cmd_def_cls.name] = cmd_def_cls
        except Exception:
            pass
    return all_defs


def generate_subcommands_for(cmd_name: str) -> str | None:
    """Generate SubCommand dict code for a command."""
    spec = REGISTRY.get_any(cmd_name)
    if spec is None:
        return None

    if not spec.validation or not spec.validation.subcommands:
        return None

    # Collect data from all sources
    sub_arities = spec.validation.subcommands
    sub_names = sorted(sub_arities.keys())

    # Get FormSpec arg_values[0] for detail/synopsis/hover
    sub_details: dict[str, tuple[str, str, object]] = {}  # name -> (detail, synopsis, hover)
    for av in spec.argument_values(0):
        sub_details[av.value] = (av.detail, "", av.hover)

    # Get subcommand_arg_values
    sub_arg_values: dict[str, dict[int, list]] = {}
    for form in spec.forms:
        for (sub, idx), values in form.subcommand_arg_values.items():
            sub_arg_values.setdefault(sub, {})[idx] = values

    # Get pure/mutator sets
    pure_subs = spec.pure_subcommands
    mutator_subs = spec.mutator_subcommands

    # Get option terminator profiles (subcommand-scoped)
    sub_opt_terms: dict[str, OptionTerminatorSpec] = {}
    for profile in spec.option_terminator_profiles:
        if profile.subcommand:
            sub_opt_terms[profile.subcommand] = profile

    # Try to get role_hints and type_hints from CommandDef
    role_hints_data: dict[str, object] = {}
    type_hints_data: dict[str, object] = {}

    # We can't easily call role_hints/type_hints without the class,
    # but we can get them from runtime SIGNATURES and TYPE_HINTS
    from core.commands.registry.runtime import SIGNATURES, TYPE_HINTS
    from core.commands.registry.signatures import SubcommandSig

    sig = SIGNATURES.get(cmd_name)
    if isinstance(sig, SubcommandSig):
        role_hints_data = {name: cs for name, cs in sig.subcommands.items()}

    type_hint = TYPE_HINTS.get(cmd_name)
    if type_hint is not None:
        from core.commands.registry.type_hints import SubcommandTypeHint

        if isinstance(type_hint, SubcommandTypeHint):
            type_hints_data = {name: th for name, th in type_hint.subcommands.items()}

    # Build SubCommand entries
    lines = []
    lines.append("            subcommands={")

    for sub_name in sub_names:
        arity = sub_arities[sub_name]
        detail_info = sub_details.get(sub_name, ("", "", None))
        detail = detail_info[0]
        hover = detail_info[2]

        # Build synopsis from hover
        synopsis = ""
        if hover and hover.synopsis:
            synopsis = hover.synopsis[0]

        # Arity string
        if arity.max == arity.min:
            arity_str = f"Arity({arity.min}, {arity.max})"
        elif arity.max >= 2**31:
            arity_str = f"Arity({arity.min})"
        else:
            arity_str = f"Arity({arity.min}, {arity.max})"

        # Pure/mutator
        is_pure = sub_name in pure_subs
        is_mutator = sub_name in mutator_subs

        # Return type
        return_type = None
        arg_types_dict = {}
        th = type_hints_data.get(sub_name)
        if th is not None:
            return_type = th.return_type
            arg_types_dict = th.arg_types

        # Arg roles
        arg_roles = {}
        rh = role_hints_data.get(sub_name)
        if rh is not None and hasattr(rh, "arg_roles"):
            arg_roles = rh.arg_roles

        # Option terminator
        opt_term = sub_opt_terms.get(sub_name)

        # Arg values
        arg_vals = sub_arg_values.get(sub_name, {})

        # Build SubCommand constructor
        parts = [f'                "{sub_name}": SubCommand(']
        parts.append(f'                    name="{sub_name}",')
        parts.append(f"                    arity={arity_str},")

        if detail:
            escaped_detail = detail.replace('"', '\\"')
            parts.append(f'                    detail="{escaped_detail}",')

        if synopsis:
            escaped_synopsis = synopsis.replace('"', '\\"')
            parts.append(f'                    synopsis="{escaped_synopsis}",')

        if is_pure:
            parts.append("                    pure=True,")

        if is_mutator:
            parts.append("                    mutator=True,")

        if return_type is not None:
            parts.append(f"                    return_type=TclType.{return_type.name},")

        if arg_roles:
            roles_str = ", ".join(f"{k}: ArgRole.{v.name}" for k, v in sorted(arg_roles.items()))
            parts.append(f"                    arg_roles={{{roles_str}}},")

        if arg_types_dict:
            types_parts = []
            for k, v in sorted(arg_types_dict.items()):
                if v.expected:
                    types_parts.append(f"{k}: ArgTypeHint(expected=TclType.{v.expected.name})")
            if types_parts:
                types_str = ", ".join(types_parts)
                parts.append(f"                    arg_types={{{types_str}}},")

        if opt_term:
            ot_parts = [f"scan_start={opt_term.scan_start}"]
            if opt_term.options_with_values:
                vals = ", ".join(f'"{v}"' for v in sorted(opt_term.options_with_values))
                ot_parts.append(f"options_with_values=frozenset({{{vals}}})")
            if opt_term.warn_without_terminator:
                ot_parts.append("warn_without_terminator=True")
            ot_str = ", ".join(ot_parts)
            parts.append(f"                    option_terminator=OptionTerminatorSpec({ot_str}),")

        if arg_vals:
            # This is complex - just note it
            parts.append(
                f"                    # arg_values: {len(arg_vals)} entries (migrate manually)"
            )

        parts.append("                ),")
        lines.append("\n".join(parts))

    lines.append("            },")
    return "\n".join(lines)


def main():
    # Ensure runtime is configured
    from core.commands.registry.runtime import configure_signatures

    configure_signatures()

    target = sys.argv[1] if len(sys.argv) > 1 else None

    for name, specs in sorted(REGISTRY.specs_by_name.items()):
        if target and name != target:
            continue
        spec = REGISTRY.get_any(name)
        if spec and spec.validation and spec.validation.subcommands:
            code = generate_subcommands_for(name)
            if code:
                print(f"\n# === {name} ===")
                print(code)


if __name__ == "__main__":
    main()
