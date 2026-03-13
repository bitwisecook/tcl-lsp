"""Bytecoded codegen for ``dict`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .._helpers import _tcl_list_element
from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_dict(emitter: _Emitter, args: tuple[str, ...]) -> bool:  # noqa: C901
    """Compile ``dict`` subcommands to specialised opcodes."""
    if len(args) < 2:
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "get" if len(rest) >= 2 and not emitter._is_proc:
            # ``dict get $d key`` → load var, push key, dictGet 1
            var_ref = rest[0]
            keys = rest[1:]
            emitter._emit_value(var_ref)
            for k in keys:
                emitter._push_lit(k)
            emitter._emit(Op.DICT_GET, len(keys))
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True

        case "set" if len(rest) >= 3 and emitter._is_proc and not emitter._is_qualified(rest[0]):
            # ``dict set d key val`` → push key(s), push val, dictSet N %vK
            var_name = rest[0]
            keys = rest[1:-1]
            value = rest[-1]
            slot = emitter._lvt.intern(var_name)
            for k in keys:
                emitter._push_lit(k)
            emitter._push_lit(value)
            emitter._emit(Op.DICT_SET, len(keys), slot, comment=f'var "{var_name}"')
            emitter._emit(Op.POP)
            return True

        case "unset" if len(rest) >= 2 and emitter._is_proc and not emitter._is_qualified(rest[0]):
            # ``dict unset d key`` → push key(s), dictUnset N %vK
            var_name = rest[0]
            keys = rest[1:]
            slot = emitter._lvt.intern(var_name)
            for k in keys:
                emitter._push_lit(k)
            emitter._emit(Op.DICT_UNSET, len(keys), slot, comment=f'var "{var_name}"')
            emitter._emit(Op.POP)
            return True

        case "incr" if (
            len(rest) in (2, 3) and emitter._is_proc and not emitter._is_qualified(rest[0])
        ):
            # ``dict incr d key ?amount?`` → push key, dictIncrImm +amt %vK
            var_name = rest[0]
            key = rest[1]
            amount = int(rest[2]) if len(rest) == 3 else 1
            slot = emitter._lvt.intern(var_name)
            emitter._push_lit(key)
            emitter._emit(Op.DICT_INCR_IMM, amount, slot, comment=f'var "{var_name}"')
            emitter._emit(Op.POP)
            return True

        case "append" if len(rest) == 3 and emitter._is_proc and not emitter._is_qualified(rest[0]):
            # ``dict append d key value`` → push key, push value, dictAppend %vK
            var_name = rest[0]
            key = rest[1]
            value = rest[2]
            slot = emitter._lvt.intern(var_name)
            emitter._push_lit(key)
            emitter._push_lit(value)
            emitter._emit(Op.DICT_APPEND, slot, comment=f'var "{var_name}"')
            emitter._emit(Op.POP)
            return True

        case "lappend" if (
            len(rest) == 3 and emitter._is_proc and not emitter._is_qualified(rest[0])
        ):
            # ``dict lappend d key value`` → push key, push value, dictLappend %vK
            var_name = rest[0]
            key = rest[1]
            value = rest[2]
            slot = emitter._lvt.intern(var_name)
            emitter._push_lit(key)
            emitter._push_lit(value)
            emitter._emit(Op.DICT_LAPPEND, slot, comment=f'var "{var_name}"')
            emitter._emit(Op.POP)
            return True

        case "set" if len(rest) >= 3 and not emitter._is_proc:
            # ``dict set d key val`` → push "dict", push "set",
            # push var, push key, push val, push "::tcl::dict::set",
            # invokeReplace 5 2
            var_name = rest[0]
            keys = rest[1:-1]
            value = rest[-1]
            emitter._push_lit("dict")
            emitter._push_lit("set")
            emitter._push_lit(var_name)
            for k in keys:
                emitter._push_lit(k)
            emitter._push_lit(value)
            ns_cmd = "::tcl::dict::set"
            emitter._push_lit(ns_cmd)
            # invokeReplace argc replace_count
            # argc = dict + set + var + keys + value = 2 + 1 + len(keys) + 1
            argc = 2 + 1 + len(keys) + 1
            emitter._emit(Op.INVOKE_REPLACE, argc, 2)
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True

        case "exists" if len(rest) >= 2 and not emitter._is_proc:
            # ``dict exists $d key`` → load var, push key, dictExists 1
            var_ref = rest[0]
            keys = rest[1:]
            emitter._emit_value(var_ref)
            for k in keys:
                emitter._push_lit(k)
            emitter._emit(Op.DICT_EXISTS, len(keys))
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True

        case "create" if not emitter._is_proc:
            # Constant args: fold to literal + dup + verifyDict.
            # Only fold when even number of args (valid key/value pairs).
            all_const = all("$" not in a and "[" not in a for a in rest)
            if all_const and len(rest) % 2 == 0:
                clean_rest = [
                    a[3:-3] if a.startswith("\x00\x01{") and a.endswith("}\x01\x00") else a
                    for a in rest
                ]
                parts = [_tcl_list_element(a) for a in clean_rest]
                result = " ".join(parts) if parts else ""
                emitter._push_lit(result)
                emitter._emit(Op.DUP)
                emitter._emit(Op.VERIFY_DICT)
                emitter._emit(Op.POP)
                emitter._seen_generic_invoke = True
                return True
            # Non-constant: use resolved FQ name.
            emitter._push_lit("::tcl::dict::create")
            for a in rest:
                emitter._emit_value(a)
            emitter._emit(Op.INVOKE_STK1, 1 + len(rest))
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True

        case "lappend" if len(rest) >= 1 and not emitter._is_proc:
            # ``dict lappend d key val ...`` → invokeReplace
            emitter._push_lit("dict")
            emitter._push_lit("lappend")
            for a in rest:
                emitter._emit_value(a)
            emitter._push_lit("::tcl::dict::lappend")
            emitter._emit(Op.INVOKE_REPLACE, 2 + len(rest), 2)
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True

        case (
            "append" | "incr" | "keys" | "values" | "size" | "merge" | "unset" | "update" | "with"
        ) if not emitter._is_proc:
            # Resolved FQ name: push ``::tcl::dict::<sub>``, args, invokeStk1.
            fq_name = f"::tcl::dict::{sub}"
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

    REGISTRY.register_codegen("dict", codegen_dict)
