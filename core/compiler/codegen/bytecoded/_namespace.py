"""Bytecoded codegen for ``namespace`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_namespace(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``namespace`` subcommands."""
    if not (len(args) >= 2 and not emitter._is_proc):
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "eval" if len(rest) >= 1:
            # invokeReplace pattern: push original words, args, FQ name.
            emitter._push_lit("namespace")
            emitter._push_lit("eval")
            for a in rest:
                emitter._emit_value(a)
            emitter._push_lit("::tcl::namespace::eval")
            emitter._emit(Op.INVOKE_REPLACE, 2 + len(rest), 2)
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True
        case _:
            return False


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("namespace", codegen_namespace)
