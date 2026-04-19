"""Generate _mock_stubs.tcl from CommandSpec metadata.

This script reads every iRule command from the registry and generates Tcl
stub procs for commands that do NOT already have hand-written mocks in
command_mocks.tcl.  Each stub:

  1. Logs the call to the decision log (category + action)
  2. Returns an empty string
  3. Does not raise errors

This ensures iRules using specialised commands (DIAMETER, SIP, MQTT, etc.)
don't crash the test framework.  Users can override stubs with
``::itest::register_command`` or by defining the expected mock proc.

Generated data:
  - ``::itest::cmd::_stub_*`` procs for each unmocked command

Usage::

    python -m core.irule_test.codegen_mock_stubs

Or via make::

    make irule-mock-stubs
"""

from __future__ import annotations

import re
from pathlib import Path

from core.commands.registry.command_registry import REGISTRY

_OUTPUT = Path(__file__).parent / "tcl" / "_mock_stubs.tcl"
_COMMAND_MOCKS = Path(__file__).parent / "tcl" / "command_mocks.tcl"


def _existing_mock_procs() -> set[str]:
    """Parse command_mocks.tcl to find already-defined mock proc names."""
    content = _COMMAND_MOCKS.read_text()
    return set(re.findall(r"proc\s+([\w_]+)\s", content))


def _sanitize_proc_name(name: str) -> str:
    """Make a string safe for use as a Tcl proc name."""
    return name.replace("-", "_").replace(".", "_")


def _mock_proc_name(irule_cmd: str) -> str:
    """Derive the mock proc name from an iRule command name.

    Convention matches command_mocks.tcl:
      NS::sub -> ns_sub
      toplevel -> cmd_toplevel

    Hyphens and dots in names are converted to underscores.
    """
    if "::" in irule_cmd:
        parts = irule_cmd.split("::")
        ns = _sanitize_proc_name(parts[0].lower())
        sub = _sanitize_proc_name(parts[-1])
        return f"{ns}_{sub}"
    else:
        return f"cmd_{_sanitize_proc_name(irule_cmd)}"


def _tcl_escape(s: str) -> str:
    """Escape a string for Tcl source."""
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$")


def _generate() -> str:
    lines: list[str] = []

    lines.append(
        "# _mock_stubs.tcl -- AUTO-GENERATED stub mocks for iRule commands\n"
        "#\n"
        "# DO NOT EDIT.  Regenerate with:\n"
        "#   python -m core.irule_test.codegen_mock_stubs\n"
        "#\n"
        "# These stubs provide minimal mock implementations for iRule commands\n"
        "# that do not have hand-written mocks in command_mocks.tcl.  Each stub\n"
        "# logs the call to the decision log and returns an empty string.\n"
        "#\n"
        "# Source: core/commands/registry/\n"
        "#\n"
        "# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.\n"
    )

    existing = _existing_mock_procs()
    irule_cmds = sorted(REGISTRY.command_names(dialect="f5-irules"))

    # Group by namespace for readability
    namespaced: dict[str, list[str]] = {}
    toplevel: list[str] = []

    for cmd in irule_cmds:
        proc_name = _mock_proc_name(cmd)
        # Skip if already has a hand-written mock
        if proc_name in existing:
            continue
        if "::" in cmd:
            ns = cmd.split("::")[0]
            namespaced.setdefault(ns, []).append(cmd)
        else:
            toplevel.append(cmd)

    lines.append("")
    lines.append("namespace eval ::itest::cmd {")
    lines.append("")

    stub_count = 0

    # Namespaced commands grouped by namespace
    for ns in sorted(namespaced):
        cmds = namespaced[ns]
        lines.append(f"    # {ns}:: stubs ({len(cmds)} commands)")
        lines.append("")
        for cmd in sorted(cmds):
            proc_name = _mock_proc_name(cmd)
            # Determine category and action for decision logging
            ns_lower = ns.lower()
            sub = cmd.split("::")[-1]

            # Stubs return empty string by default.
            return_expr = '""'

            lines.append(f"    proc {proc_name} {{args}} {{")
            lines.append(f"        ::itest::log_decision {ns_lower} {sub} $args")
            lines.append(f"        return {return_expr}")
            lines.append("    }")
            lines.append("")
            stub_count += 1
        lines.append("")

    # Top-level commands
    if toplevel:
        lines.append(f"    # Top-level stubs ({len(toplevel)} commands)")
        lines.append("")
        for cmd in sorted(toplevel):
            proc_name = _mock_proc_name(cmd)
            lines.append(f"    proc {proc_name} {{args}} {{")
            lines.append(f"        ::itest::log_decision toplevel {cmd} $args")
            lines.append('        return ""')
            lines.append("    }")
            lines.append("")
        stub_count += len(toplevel)

    lines.append("}")  # end namespace eval
    lines.append("")
    lines.append(f"# Total stub mocks generated: {stub_count}")
    lines.append("")

    return "\n".join(lines)


def main() -> None:
    content = _generate()
    _OUTPUT.write_text(content)

    # Count for reporting
    stub_count = content.count("    proc ")
    print(f"Generated {_OUTPUT} ({stub_count} stub mocks)")


if __name__ == "__main__":
    main()
