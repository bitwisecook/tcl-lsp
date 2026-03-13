"""Tk-specific diagnostics.

Checks for common Tk programming mistakes that the LSP can detect
statically from command invocations:

- TK1001: Geometry manager conflict (pack and grid in same parent)
- TK1002: Widget path references non-existent parent
- TK1003: Unknown option for widget type
"""

from __future__ import annotations

from ..analysis.semantic_model import (
    AnalysisResult,
    CommandInvocation,
    Diagnostic,
    Range,
    Severity,
)
from ..commands.registry import REGISTRY
from .common import (
    GEOMETRY_COMMANDS as _GEOMETRY_COMMANDS,
)
from .common import (
    WIDGET_COMMANDS as _WIDGET_COMMANDS,
)
from .common import (
    is_widget_path as _is_widget_path,
)
from .common import (
    parent_widget_path as _parent_path,
)


def check_tk_diagnostics(
    source: str,
    analysis: AnalysisResult,
) -> list[Diagnostic]:
    """Run Tk-specific diagnostic checks on analysed source.

    Only call this when ``has_tk_require(analysis)`` is True.
    """
    diagnostics: list[Diagnostic] = []

    # Track created widgets and their geometry managers.
    created_widgets: dict[str, Range] = {".": Range.zero()}  # root always exists
    geometry_by_parent: dict[str, set[str]] = {}  # parent -> {pack, grid, place}
    geometry_ranges: dict[str, list[tuple[str, Range]]] = {}  # parent -> [(manager, range)]

    for inv in analysis.command_invocations:
        cmd_name = inv.resolved_qualified_name or inv.name
        args = _extract_args(source, inv)

        # Track widget creation.
        if cmd_name in _WIDGET_COMMANDS and args:
            path = args[0]
            if _is_widget_path(path):
                created_widgets[path] = inv.range

                # TK1002: Check parent exists.
                parent = _parent_path(path)
                if parent and parent not in created_widgets:
                    diagnostics.append(
                        Diagnostic(
                            code="TK1002",
                            message=f"Widget path '{path}' references non-existent parent '{parent}'.",
                            range=inv.range,
                            severity=Severity.WARNING,
                        )
                    )

                # TK1003: Check for unknown options.
                spec = REGISTRY.get(cmd_name)
                if spec is not None:
                    known_options = set(spec.switch_names())
                    for arg in args[1:]:
                        if arg.startswith("-") and not arg.startswith("--") and len(arg) > 1:
                            if arg not in known_options:
                                diagnostics.append(
                                    Diagnostic(
                                        code="TK1003",
                                        message=f"Unknown option '{arg}' for {cmd_name}.",
                                        range=inv.range,
                                        severity=Severity.HINT,
                                    )
                                )

        # Track geometry manager usage.
        if cmd_name in _GEOMETRY_COMMANDS and args:
            # First arg is typically the widget, parent is derived from widget path.
            widget_path = args[0]
            if _is_widget_path(widget_path):
                parent = _parent_path(widget_path)
                if parent not in geometry_by_parent:
                    geometry_by_parent[parent] = set()
                    geometry_ranges[parent] = []
                geometry_by_parent[parent].add(cmd_name)
                geometry_ranges[parent].append((cmd_name, inv.range))

    # TK1001: Geometry manager conflicts.
    for parent, managers in geometry_by_parent.items():
        # Mixing pack and grid in the same parent is a Tk error at runtime.
        if "pack" in managers and "grid" in managers:
            # Report on the second manager usage.
            for manager, rng in geometry_ranges[parent]:
                diagnostics.append(
                    Diagnostic(
                        code="TK1001",
                        message=(
                            f"Geometry manager conflict: cannot mix 'pack' and 'grid' "
                            f"in the same parent '{parent}'."
                        ),
                        range=rng,
                        severity=Severity.WARNING,
                    )
                )

    return diagnostics


def _extract_args(source: str, inv: CommandInvocation) -> list[str]:
    """Extract string arguments from a command invocation.

    Uses the source text and invocation range to get the raw words.
    This is a best-effort extraction for literal arguments.
    """
    # CommandInvocation stores name and range; we parse from invocation data.
    # For now, use a simple approach: split the source line at the invocation.
    lines = source.splitlines()
    start_line = inv.range.start.line
    if start_line >= len(lines):
        return []

    line_text = lines[start_line]
    # Simple word splitting for the invocation line.
    words = line_text.split()
    # Skip the command name (first word) and return the rest.
    if len(words) > 1:
        return words[1:]
    return []
