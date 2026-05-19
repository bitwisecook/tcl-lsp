"""Bytecoded codegen for ``string`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .._helpers import _DEFAULT_TRIM_CHARS
from ..opcodes import (
    _STR_CLASS_IDS,
    Op,
    _parse_tcl_index,
)

if TYPE_CHECKING:
    from .._emitter import _Emitter


def _try_string_is_op(emitter: _Emitter, rest: tuple[str, ...]) -> bool:
    """Compile ``string is <class> ?-strict? <value>``."""
    class_name = rest[0]
    # Parse optional -strict flag and value
    if len(rest) == 2:
        strict = False
        value = rest[1]
    elif len(rest) == 3 and rest[1] == "-strict":
        strict = True
        value = rest[2]
    else:
        return False

    # strclass-based types (alpha, digit, alnum, etc.)
    class_id = _STR_CLASS_IDS.get(class_name)
    if class_id is not None:
        emitter._emit_value(value)
        emitter._emit(Op.STR_CLASS, class_id)
        emitter._emit(Op.POP)
        return True

    # numericType-based: integer
    if class_name == "integer":
        emitter._emit_value(value)
        if strict:
            emitter._emit(Op.NUMERIC_TYPE)
            emitter._emit(Op.DUP)
            end_label = emitter._fresh_label("si_end")
            emitter._emit(Op.JUMP_FALSE1, end_label)
            emitter._push_lit("3")  # wide int type threshold
            emitter._emit(Op.LE)
            emitter._place_label(end_label)
        else:
            # Non-strict: dup, numericType, dup, jumpTrue → reverse+pop+push "3"+le
            #             else pop+push ""+streq
            emitter._emit(Op.DUP)
            emitter._emit(Op.NUMERIC_TYPE)
            emitter._emit(Op.DUP)
            has_type = emitter._fresh_label("si_has_type")
            emitter._emit(Op.JUMP_TRUE1, has_type)
            emitter._emit(Op.POP)
            emitter._push_lit("")
            emitter._emit(Op.STR_EQ)
            end_label = emitter._fresh_label("si_end")
            emitter._emit(Op.JUMP1, end_label)
            emitter._place_label(has_type)
            emitter._emit(Op.REVERSE, 2)
            emitter._emit(Op.POP)
            emitter._push_lit("3")
            emitter._emit(Op.LE)
            emitter._place_label(end_label)
        emitter._emit(Op.POP)
        return True

    # numericType-based: double
    if class_name == "double":
        emitter._emit_value(value)
        if strict:
            emitter._emit(Op.NUMERIC_TYPE)
            true_label = emitter._fresh_label("si_true")
            emitter._emit(Op.JUMP_TRUE1, true_label)
            emitter._push_lit("0")
            end_label = emitter._fresh_label("si_end")
            emitter._emit(Op.JUMP1, end_label)
            emitter._place_label(true_label)
            emitter._push_lit("1")
            emitter._instrs[-1].no_fold = True
            emitter._place_label(end_label)
        else:
            # Non-strict: dup, "" streq, jumpTrue → pop; numericType,
            # jumpTrue → push "1"; else push "0"
            emitter._emit(Op.DUP)
            emitter._push_lit("")
            emitter._emit(Op.STR_EQ)
            true_label = emitter._fresh_label("si_true")
            emitter._emit(Op.JUMP_TRUE1, true_label)
            emitter._emit(Op.NUMERIC_TYPE)
            has_type = emitter._fresh_label("si_has_type")
            emitter._emit(Op.JUMP_TRUE1, has_type)
            emitter._push_lit("0")
            end_label = emitter._fresh_label("si_end")
            emitter._emit(Op.JUMP1, end_label)
            emitter._place_label(true_label)
            emitter._emit(Op.POP)
            emitter._place_label(has_type)
            emitter._push_lit("1")
            emitter._instrs[-1].no_fold = True
            emitter._place_label(end_label)
        emitter._emit(Op.POP)
        return True

    # tryCvtToBoolean-based: boolean
    if class_name == "boolean" and not strict:
        emitter._emit_value(value)
        emitter._emit(Op.TRY_CVT_TO_BOOLEAN)
        true_label = emitter._fresh_label("si_true")
        emitter._emit(Op.JUMP_TRUE1, true_label)
        emitter._push_lit("")
        emitter._emit(Op.STR_EQ)
        end_label = emitter._fresh_label("si_end")
        emitter._emit(Op.JUMP1, end_label)
        emitter._place_label(true_label)
        emitter._emit(Op.POP)
        emitter._push_lit("1")
        emitter._instrs[-1].no_fold = True
        emitter._place_label(end_label)
        emitter._emit(Op.POP)
        return True

    return False


def codegen_string(emitter: _Emitter, args: tuple[str, ...]) -> bool:  # noqa: C901
    """Compile ``string`` subcommands to specialised opcodes."""
    if len(args) < 1:
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "length" if len(rest) == 1:
            emitter._emit_value(rest[0])
            emitter._emit(Op.STR_LEN)
            emitter._emit(Op.POP)
            return True
        case "index" if len(rest) == 2:
            emitter._emit_value(rest[0])
            emitter._emit_value(rest[1])
            emitter._emit(Op.STR_INDEX)
            emitter._emit(Op.POP)
            return True
        case "range" if len(rest) == 3:
            emitter._emit_value(rest[0])
            # Use strrangeImm when both indices are constant (int, end, end-N)
            start_idx = _parse_tcl_index(rest[1])
            end_idx = _parse_tcl_index(rest[2])
            if start_idx is not None and end_idx is not None:
                emitter._emit(Op.STR_RANGE_IMM, start_idx, end_idx)
            else:
                emitter._emit_value(rest[1])
                emitter._emit_value(rest[2])
                emitter._emit(Op.STR_RANGE)
            emitter._emit(Op.POP)
            return True
        case "equal":
            if len(rest) == 2:
                emitter._emit_value(rest[0])
                emitter._emit_value(rest[1])
                emitter._emit(Op.STR_EQ)
                emitter._emit(Op.POP)
                return True
            if len(rest) == 3 and rest[0] == "-nocase":
                # invokeReplace: push original words, FQ name.
                emitter._push_lit("string")
                emitter._push_lit("equal")
                emitter._push_lit("-nocase")
                emitter._emit_value(rest[1])
                emitter._emit_value(rest[2])
                emitter._push_lit("::tcl::string::equal")
                emitter._emit(Op.INVOKE_REPLACE, 5, 2)
                emitter._emit(Op.POP)
                emitter._seen_generic_invoke = True
                return True
            return False
        case "compare":
            if len(rest) == 2:
                emitter._emit_value(rest[0])
                emitter._emit_value(rest[1])
                emitter._emit(Op.STR_CMP)
                emitter._emit(Op.POP)
                return True
            if len(rest) == 3 and rest[0] == "-nocase":
                # invokeReplace: push original words, args, FQ name.
                emitter._push_lit("string")
                emitter._push_lit("compare")
                emitter._push_lit("-nocase")
                emitter._emit_value(rest[1])
                emitter._emit_value(rest[2])
                emitter._push_lit("::tcl::string::compare")
                emitter._emit(Op.INVOKE_REPLACE, 5, 2)
                emitter._emit(Op.POP)
                emitter._seen_generic_invoke = True
                return True
            if len(rest) == 4 and rest[0] == "-length":
                # invokeReplace for -length N str1 str2.
                emitter._push_lit("string")
                emitter._push_lit("compare")
                emitter._push_lit("-length")
                emitter._emit_value(rest[1])
                emitter._emit_value(rest[2])
                emitter._emit_value(rest[3])
                emitter._push_lit("::tcl::string::compare")
                emitter._emit(Op.INVOKE_REPLACE, 6, 2)
                emitter._emit(Op.POP)
                emitter._seen_generic_invoke = True
                return True
            return False
        case "match" if 2 <= len(rest) <= 3:
            nocase = 0
            pattern_idx = 0
            string_idx = 1
            if len(rest) == 3 and rest[0] == "-nocase":
                nocase = 1
                pattern_idx = 1
                string_idx = 2
            emitter._emit_value(rest[pattern_idx])
            emitter._emit_value(rest[string_idx])
            emitter._emit(Op.STR_MATCH, nocase)
            emitter._emit(Op.POP)
            return True
        case "toupper" if len(rest) == 1:
            emitter._emit_value(rest[0])
            emitter._emit(Op.STR_UPPER)
            emitter._emit(Op.POP)
            return True
        case "tolower" if len(rest) == 1:
            emitter._emit_value(rest[0])
            emitter._emit(Op.STR_LOWER)
            emitter._emit(Op.POP)
            return True
        case "totitle" if len(rest) == 1:
            emitter._emit_value(rest[0])
            emitter._emit(Op.STR_TITLE)
            emitter._emit(Op.POP)
            return True
        case "trim" if len(rest) >= 1:
            emitter._emit_value(rest[0])
            if len(rest) > 1:
                emitter._emit_value(rest[1])
            else:
                emitter._push_lit(_DEFAULT_TRIM_CHARS)
            emitter._emit(Op.STR_TRIM)
            emitter._emit(Op.POP)
            return True
        case "trimleft" if len(rest) >= 1:
            emitter._emit_value(rest[0])
            if len(rest) > 1:
                emitter._emit_value(rest[1])
            else:
                emitter._push_lit(_DEFAULT_TRIM_CHARS)
            emitter._emit(Op.STR_TRIM_LEFT)
            emitter._emit(Op.POP)
            return True
        case "trimright" if len(rest) >= 1:
            emitter._emit_value(rest[0])
            if len(rest) > 1:
                emitter._emit_value(rest[1])
            else:
                emitter._push_lit(_DEFAULT_TRIM_CHARS)
            emitter._emit(Op.STR_TRIM_RIGHT)
            emitter._emit(Op.POP)
            return True
        case "cat" if len(rest) >= 1:
            # Single arg: identity (no strcat needed).
            # Multiple args: strcat N.
            for a in rest:
                emitter._emit_value(a)
            if len(rest) > 1:
                emitter._emit(Op.STR_CONCAT1, len(rest))
            emitter._emit(Op.POP)
            return True
        case "first" if len(rest) == 2:
            emitter._emit_value(rest[0])
            emitter._emit_value(rest[1])
            emitter._emit(Op.STR_FIND)
            emitter._emit(Op.POP)
            return True
        case "last" if len(rest) == 2:
            emitter._emit_value(rest[0])
            emitter._emit_value(rest[1])
            emitter._emit(Op.STR_RFIND)
            emitter._emit(Op.POP)
            return True
        case "is" if len(rest) >= 2:
            return _try_string_is_op(emitter, rest)
        case _:
            # Unhandled subcommand — emit direct FQN invoke.
            # tclsh 9.0 compiles unrecognised string subcommands as
            # ``push FQN; push args...; invokeStk1 N``.
            fqn = f"::tcl::string::{sub}"
            emitter._push_lit(fqn)
            for a in rest:
                emitter._emit_value(a)
            argc = 1 + len(rest)
            op = Op.INVOKE_STK1 if argc < 256 else Op.INVOKE_STK4
            emitter._emit(op, argc)
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("string", codegen_string)
