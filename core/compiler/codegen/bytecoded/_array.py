"""Bytecoded codegen for ``array`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_array(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``array`` subcommands using resolved FQ names."""
    if not (len(args) >= 2 and not emitter._is_proc):
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "exists" if (
            len(rest) == 1
            and emitter._is_proc
            and not emitter._is_qualified(rest[0])
            and not emitter._is_array_ref(rest[0])
        ):
            slot = emitter._lvt.intern(rest[0])
            emitter._emit(Op.ARRAY_EXISTS_IMM, slot, comment=f'var "{rest[0]}"')
            emitter._emit(Op.POP)
            return True
        case "names" | "size" if len(rest) >= 1:
            fq_name = f"::tcl::array::{sub}"
            emitter._push_lit(fq_name)
            for a in rest:
                emitter._emit_value(a)
            emitter._emit(Op.INVOKE_STK1, 1 + len(rest))
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True
        case _:
            return False


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("array", codegen_array)
