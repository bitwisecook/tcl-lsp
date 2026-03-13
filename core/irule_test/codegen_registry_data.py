"""Generate _registry_data.tcl from the authoritative Python registries.

This script reads command, operator, and dialect data from the Python
command registry and emits a Tcl file that tmm_shim.tcl, expr_ops.tcl,
and command_mocks.tcl source.  This ensures there is a single source of
truth for TMM environment configuration.

Generated data:
  - ``::tmm::_gen_disabled_commands``   -- standard Tcl commands TMM removes
  - ``::tmm::_gen_post84_commands``     -- Tcl 8.5+ commands absent from 8.4
  - ``::tmm::expr_ops::_gen_operators`` -- TMM custom expression operators
  - ``::itest::cmd::_gen_irule_commands`` -- all iRule command names

Usage::

    python -m core.irule_test.codegen_registry_data

Or via make::

    make irule-registry-data
"""

from __future__ import annotations

from pathlib import Path

from core.commands.registry.command_registry import REGISTRY
from core.commands.registry.operators import IRULES_OPERATOR_HOVER

_OUTPUT = Path(__file__).parent / "tcl" / "_registry_data.tcl"


def _generate() -> str:
    lines: list[str] = []

    lines.append(
        "# _registry_data.tcl -- AUTO-GENERATED from Python command registry\n"
        "#\n"
        "# DO NOT EDIT.  Regenerate with:\n"
        "#   python -m core.irule_test.codegen_registry_data\n"
        "#\n"
        "# Source: core/commands/registry/\n"
        "#\n"
        "# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.\n"
    )

    # Disabled commands
    # Standard Tcl 8.4 commands that TMM removes (not in f5-irules dialect).

    tcl84 = set(REGISTRY.command_names(dialect="tcl8.4"))
    irules = set(REGISTRY.command_names(dialect="f5-irules"))
    disabled = sorted(tcl84 - irules)

    lines.append("namespace eval ::tmm {\n")
    lines.append("    # Standard Tcl 8.4 commands that TMM removes.")
    lines.append("    # Derived from: tcl8.4 dialect minus f5-irules dialect.\n")
    lines.append("    variable _gen_disabled_commands {")
    for cmd in disabled:
        lines.append(f"        {cmd}")
    lines.append("    }\n")

    # Post-8.4 commands
    # Commands in 8.5/8.6/9.0 that don't exist in 8.4, also not in iRules.

    tcl85 = set(REGISTRY.command_names(dialect="tcl8.5"))
    tcl86 = set(REGISTRY.command_names(dialect="tcl8.6"))
    tcl90 = set(REGISTRY.command_names(dialect="tcl9.0"))
    post84 = sorted((tcl85 | tcl86 | tcl90) - tcl84 - irules)

    lines.append("    # Commands from Tcl 8.5+ that do not exist in 8.4 or iRules.")
    lines.append("    # Derived from: (tcl8.5 | tcl8.6 | tcl9.0) - tcl8.4 - f5-irules.\n")
    lines.append("    variable _gen_post84_commands {")
    for cmd in post84:
        lines.append(f"        {cmd}")
    lines.append("    }\n")

    lines.append("}")  # end namespace eval ::tmm
    lines.append("")

    # TMM expression operators

    # Filter to only the infix binary operators used in expr rewriting.
    # Exclude 'and', 'or', 'not' which are boolean operator aliases,
    # not infix string operators.
    infix_ops = sorted(k for k in IRULES_OPERATOR_HOVER if k not in ("and", "or", "not"))
    all_ops = sorted(IRULES_OPERATOR_HOVER.keys())

    lines.append("namespace eval ::tmm::expr_ops {\n")
    lines.append("    # TMM custom infix expression operators for expr rewriting.")
    lines.append("    # Derived from: core.commands.registry.operators.IRULES_OPERATOR_HOVER\n")
    lines.append("    variable _gen_operators {")
    for op in infix_ops:
        lines.append(f"        {op}")
    lines.append("    }\n")
    lines.append("    # All TMM expression operators (including boolean aliases).\n")
    lines.append("    variable _gen_all_operators {")
    for op in all_ops:
        lines.append(f"        {op}")
    lines.append("    }\n")
    lines.append("}")  # end namespace eval ::tmm::expr_ops
    lines.append("")

    # iRule command names
    # All commands in the f5-irules dialect, for mock registration.

    # Separate namespaced (NS::cmd) from top-level
    namespaced = sorted(c for c in irules if "::" in c)
    toplevel = sorted(c for c in irules if "::" not in c)

    lines.append("namespace eval ::itest::cmd {\n")
    lines.append("    # All f5-irules namespaced commands (NS::subcommand).")
    lines.append(f"    # Count: {len(namespaced)}\n")
    lines.append("    variable _gen_namespaced_commands {")
    # Group by namespace for readability
    current_ns = ""
    for cmd in namespaced:
        ns = cmd.split("::")[0]
        if ns != current_ns:
            if current_ns:
                lines.append("")
            lines.append(f"        # {ns}::")
            current_ns = ns
        lines.append(f"        {{{cmd}}}")
    lines.append("    }\n")

    lines.append("    # All f5-irules top-level commands.")
    lines.append(f"    # Count: {len(toplevel)}\n")
    lines.append("    variable _gen_toplevel_commands {")
    for cmd in toplevel:
        lines.append(f"        {cmd}")
    lines.append("    }\n")

    lines.append("}")  # end namespace eval ::itest::cmd
    lines.append("")

    return "\n".join(lines)


def main() -> None:
    content = _generate()
    _OUTPUT.write_text(content)
    print(f"Generated {_OUTPUT}")


if __name__ == "__main__":
    main()
