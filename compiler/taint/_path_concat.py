r"""W201 -- manual path concatenation instead of ``file join``.

Detects ``set`` assignments where the rendered value contains both a
literal path separator (``/`` or ``\``) and a variable or path-returning
command result.

Detection uses the **Rendered Value Properties** pass
(``core/compiler/rendered_properties.py``) which computes string content
properties (``HAS_FORWARD_SLASH``, ``HAS_INTERPOLATION``, etc.) for
every SSA value after Tcl escape rendering via ``backslash_subst()``.
This means escape sequences like ``\n``, ``\t``, ``\xNN`` are correctly
resolved *before* checking for path separators.

Suppression uses the **taint lattice** (``PATH_NORMALISED`` / ``PATH_JOINED``
colours) and a forward-scan for ``[file normalize $var]`` in the same block.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from analyser.checks._helpers import _build_file_join_fix
from compiler.registry.taint_hints import TaintColour
from shared.codes import diag
from shared.diagnostic import CodeFix
from shared.ranges import range_from_token

from ..cfg import CFGFunction
from ..ir import IRAssignValue
from ..ssa import SSAFunction, SSAValueKey
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._lattice import TaintLattice
from ._types import TaintWarning

if TYPE_CHECKING:
    from ..rendered_properties import RenderedValueProps

diag("W201", "Manual path concatenation \u2014 use `file join` instead.", section="warning")


def _is_file_normalize_of(value: str, var_name: str) -> bool:
    """Return True if *value* is ``[file normalize $var_name]``."""
    parsed = parse_command_substitution(value)
    if parsed is None:
        return False
    cmd, args = parsed
    if cmd != "file" or not args or args[0] != "normalize":
        return False
    for arg in args[1:]:
        stripped = arg.strip().strip('"')
        if stripped == f"${var_name}" or stripped == f"${{{var_name}}}":
            return True
    return False


def _find_path_concat_warnings(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice] | None,
    executable_blocks: set[str],
    rendered_props: dict[SSAValueKey, "RenderedValueProps"] | None = None,
) -> list[TaintWarning]:
    """Find ``set`` assignments that use manual path concatenation.

    Detection (via rendered-properties pass):
      1. The SSA value has ``HAS_FORWARD_SLASH`` or ``HAS_BACKSLASH``
         in its ``may`` properties (path separators in rendered literals).
      2. The SSA value has ``HAS_INTERPOLATION`` in its ``may``
         properties (contains variable refs or command substitutions).

    Suppression via taint lattice:
      - ``PATH_NORMALISED`` on the assigned SSA value suppresses.

    Suppression via forward-scan:
      - The next assignment to the same variable in the same block
        is ``[file normalize $var]``.
    """
    from ..rendered_properties import RenderedProperties

    warnings: list[TaintWarning] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        for idx, stmt in enumerate(block.statements):
            if not isinstance(stmt, IRAssignValue):
                continue
            if idx >= len(ssa_block.statements):
                continue

            # Skip pure variable refs and pure command substitutions --
            # these are structural forms, not manual path concatenation.
            value = stmt.value.strip()
            if is_pure_var_ref(value):
                continue
            if parse_command_substitution(value) is not None:
                continue

            ssa_stmt = ssa_block.statements[idx]
            var_name = stmt.name

            # Look up rendered properties for the defined SSA value.
            has_path_sep = False
            has_interpolation = False
            path_normalised = False

            for def_name, def_ver in ssa_stmt.defs.items():
                key: SSAValueKey = (def_name, def_ver)

                # Check rendered properties.
                if rendered_props is not None:
                    rp = rendered_props.get(key)
                    if rp is not None:
                        path_bits = (
                            RenderedProperties.HAS_FORWARD_SLASH | RenderedProperties.HAS_BACKSLASH
                        )
                        if rp.may & path_bits:
                            has_path_sep = True
                        if rp.may & RenderedProperties.HAS_INTERPOLATION:
                            has_interpolation = True

                # Check taint lattice for PATH_NORMALISED / PATH_JOINED suppression.
                if taints is not None:
                    taint_val = taints.get(key)
                    if taint_val is not None and bool(
                        taint_val.colour & (TaintColour.PATH_NORMALISED | TaintColour.PATH_JOINED)
                    ):
                        path_normalised = True

            if not has_path_sep or not has_interpolation:
                continue
            if path_normalised:
                continue

            # Suppression: forward-scan -- next assignment to same var
            # is [file normalize $var].
            suppressed = False
            for later_idx in range(idx + 1, len(block.statements)):
                later_stmt = block.statements[later_idx]
                if isinstance(later_stmt, IRAssignValue) and later_stmt.name == var_name:
                    if _is_file_normalize_of(later_stmt.value, var_name):
                        suppressed = True
                    break
            if suppressed:
                continue

            # Derive the range from the value token when available,
            # falling back to the full statement range.
            value_range = stmt.range
            if stmt.tokens is not None and len(stmt.tokens.argv) >= 3:
                value_range = range_from_token(stmt.tokens.argv[2])

            # Build optional code fix.
            fixes: tuple[CodeFix, ...] = ()
            replacement = _build_file_join_fix(stmt.value)
            if replacement is not None:
                fixes = (
                    CodeFix(
                        range=value_range,
                        new_text=replacement,
                        description="Rewrite as [file join ...]",
                    ),
                )

            warnings.append(
                TaintWarning(
                    range=value_range,
                    variable=var_name,
                    sink_command="set",
                    code="W201",
                    message=(
                        "Possible manual path concatenation. Use [file join] "
                        "for portable path construction."
                    ),
                    fixes=fixes,
                )
            )

    return warnings
