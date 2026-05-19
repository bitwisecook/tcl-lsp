from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.core_analyses import FunctionAnalysis
from compiler.ssa import SSAFunction

from ...commands.registry.runtime import ArgRole, arg_indices_for_role
from ..semantic_model import Diagnostic, Severity


class _AnalyserDiagChannelMixin(_Base):
    """W126 diagnostics: channel argument validation."""

    # Standard Tcl channel names that are always valid.
    _STANDARD_CHANNELS = frozenset({"stdout", "stderr", "stdin"})

    def _emit_channel_diagnostics(
        self,
        ssa: "SSAFunction",
        analysis: FunctionAnalysis,
    ) -> None:
        """W126: flag non-channel values passed to channel argument positions.

        Walks SSA statements for commands that declare ``ArgRole.CHANNEL``
        arguments.  For each channel arg, checks the SSA value's type:
        if it's a known constant that isn't a standard channel name and
        the variable type is not ``TclType.CHANNEL``, emit a warning.
        """
        from compiler.ir import IRCall
        from compiler.types import TclType, TypeKind

        for block in ssa.blocks.values():
            for ssa_stmt in block.statements:
                ir_stmt = ssa_stmt.statement
                if not isinstance(ir_stmt, IRCall):
                    continue
                cmd = ir_stmt.command
                args = list(ir_stmt.args)
                channel_indices = arg_indices_for_role(cmd, args, ArgRole.CHANNEL)
                if not channel_indices:
                    continue

                for idx in channel_indices:
                    if idx >= len(args):
                        continue
                    arg_text = args[idx]

                    # Extract variable name from ${var} or $var form
                    var_name: str | None = None
                    if arg_text.startswith("${") and arg_text.endswith("}"):
                        var_name = arg_text[2:-1]
                    elif arg_text.startswith("$"):
                        var_name = arg_text[1:]

                    if var_name and var_name in ssa_stmt.uses:
                        # Variable reference — check its type in the SSA
                        version = ssa_stmt.uses[var_name]
                        key = (var_name, version)
                        var_type = analysis.types.get(key)
                        if var_type is not None and var_type.kind == TypeKind.KNOWN:
                            if var_type.tcl_type == TclType.CHANNEL:
                                continue  # Confirmed channel — ok
                            # Known non-channel type → warn
                            stmt_range = getattr(ir_stmt, "range", None)
                            if stmt_range is not None:
                                self.result.diagnostics.append(
                                    Diagnostic(
                                        range=stmt_range,
                                        message=(
                                            "Variable '$"
                                            + var_name
                                            + f"' passed as channel to '{cmd}'"
                                            f" has type {var_type.tcl_type.name if var_type.tcl_type else 'UNKNOWN'},"
                                            " not CHANNEL."
                                        ),
                                        severity=Severity.WARNING,
                                        code="W126",
                                    )
                                )
                        # UNKNOWN or OVERDEFINED — could be anything, don't warn
                    elif not var_name:
                        # Literal string — check if it's a standard channel
                        literal = arg_text.strip('"').strip("{").strip("}")
                        if literal in self._STANDARD_CHANNELS:
                            continue  # stdout/stderr/stdin — ok
                        # Non-standard literal in channel position
                        # Only warn if it's clearly not a variable ref
                        if "$" not in arg_text and "[" not in arg_text:
                            stmt_range = getattr(ir_stmt, "range", None)
                            if stmt_range is not None:
                                self.result.diagnostics.append(
                                    Diagnostic(
                                        range=stmt_range,
                                        message=(
                                            f"String literal '{literal}' used as channel "
                                            f"argument to '{cmd}' — expected a channel "
                                            f"from open/socket/chan create."
                                        ),
                                        severity=Severity.WARNING,
                                        code="W126",
                                    )
                                )
