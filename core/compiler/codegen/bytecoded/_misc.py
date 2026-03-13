"""Bytecoded codegen for ``subst`` and ``unset``."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .._helpers import _parse_subst_template
from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def _try_subst_inline(emitter: _Emitter, template: str) -> bool:
    """Inline ``subst`` by decomposing into literals and variable loads."""
    from vm.compiler import _BRACE_CLOSE, _BRACE_OPEN

    # Strip sentinel markers or surrounding braces added by _process_literals.
    if template.startswith(_BRACE_OPEN) and template.endswith(_BRACE_CLOSE):
        template = template[len(_BRACE_OPEN) : -len(_BRACE_CLOSE)]
    elif len(template) >= 2 and template[0] == "{" and template[-1] == "}":
        template = template[1:-1]
    parts = _parse_subst_template(template)
    if parts is None:
        return False
    # If the template contains command substitutions, fall back to
    # the runtime handler which properly handles break/continue/return
    # exceptions inside [cmd] (Tcl_SubstObj semantics).
    if any(kind == "cmd" for kind, _ in parts):
        return False
    for kind, value in parts:
        if kind == "lit":
            emitter._push_lit(value)
        else:
            emitter._load_var(value)
    if len(parts) > 1:
        emitter._emit(Op.STR_CONCAT1, len(parts))
    emitter._emit(Op.POP)
    return True


def codegen_subst(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``subst`` to specialised opcodes."""
    if len(args) != 1:
        return False
    return _try_subst_inline(emitter, args[0])


def codegen_unset(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``unset`` to specialised opcodes."""
    if not (args and not emitter._is_proc):
        return False
    # Parse -nocomplain / -- flags.
    i = 0
    nocomplain = False
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-nocomplain":
            nocomplain = True
        if args[i] == "--":
            i += 1
            break
        i += 1
    flags = 0 if nocomplain else 1
    var_names = args[i:]
    if not var_names:
        return False
    for name in var_names:
        emitter._push_lit(name)
        emitter._emit(Op.UNSET_STK, flags)
    # unsetStk leaves no value on the stack.  Push an empty
    # string as a placeholder; _fold_const_push_pop_nops will
    # convert the push+pop to 3 nops for intermediate commands,
    # and _remove_trailing_pop will strip the pop for the last
    # command (leaving push "" as the result for done).
    emitter._push_lit("")
    emitter._emit(Op.POP)
    return True


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("subst", codegen_subst)
    REGISTRY.register_codegen("unset", codegen_unset)
