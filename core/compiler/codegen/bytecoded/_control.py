"""Bytecoded codegen for control-flow commands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_return(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``return -code CODE`` to specialised opcodes."""
    if not (emitter._is_proc and len(args) >= 2):
        return False
    # return -code CODE ?-level N? [VALUE]
    # Parse -code and -level options and emit returnImm.
    code_names = {
        "ok": 0,
        "error": 1,
        "return": 2,
        "break": 3,
        "continue": 4,
    }
    i = 0
    code: int | None = None
    level: int | None = None
    while i < len(args):
        if args[i] == "-code" and i + 1 < len(args):
            code_str = args[i + 1]
            code = code_names.get(code_str)
            if code is None:
                return False
            i += 2
        elif args[i] == "-level" and i + 1 < len(args):
            try:
                level = int(args[i + 1])
            except ValueError:
                return False
            i += 2
        elif args[i] == "--":
            i += 1
            break
        elif args[i].startswith("-"):
            return False  # unsupported option
        else:
            break
    if code is None:
        return False
    remaining = args[i:]
    if len(remaining) > 1:
        return False
    value = remaining[0] if remaining else ""
    # Compute returnImm operands to match tclsh 9.0:
    #   -code error → returnImm(1, 1)
    #   -code error -level 0 → returnImm(1, 0)
    #   -code return → returnImm(0, 2)
    if code == 1:  # error
        ret_code = 1
        ret_level = level if level is not None else 1
    elif code >= 2:  # return/break/continue
        ret_code = 0
        ret_level = level if level is not None else code
    else:
        return False  # -code ok falls through to plain return
    emitter._emit_value(value, interpolate=True)
    emitter._push_lit_no_dedup("")
    emitter._emit(Op.RETURN_IMM, ret_code, ret_level)
    emitter._emit(Op.POP)
    return True


def codegen_tailcall(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``tailcall`` to specialised opcodes."""
    if not (emitter._is_proc and args):
        return False
    emitter._push_lit("tailcall")
    for a in args:
        emitter._emit_value(a)
    argc = 1 + len(args)
    emitter._emit(Op.TAILCALL, argc)
    emitter._emit(Op.POP)
    return True


def codegen_error(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``error`` to specialised opcodes."""
    if not (emitter._is_proc and 1 <= len(args) <= 3):
        return False
    # error msg ?info? ?code?
    # Tcl 9.0: push msg; {-errorinfo info -errorcode code} list; returnImm 1 0
    msg = args[0]
    info = args[1] if len(args) >= 2 else ""
    code = args[2] if len(args) >= 3 else ""
    emitter._emit_value(msg, interpolate=True)
    opts: list[str] = []
    if info or code:
        opts.extend(["-errorinfo", info])
    if code:
        opts.extend(["-errorcode", code])
    if opts:
        for opt in opts:
            emitter._push_lit(opt)
        emitter._emit(Op.LIST, len(opts))
    else:
        emitter._push_lit("")
    emitter._emit(Op.RETURN_IMM, 1, 0)
    emitter._emit(Op.POP)
    # Replaces a generic invoke — keep startCommand markers.
    emitter._seen_generic_invoke = True
    return True


def codegen_for(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile frozen ``for`` loop to specialised opcodes."""
    if not (len(args) == 4 and args[1].startswith("[") and args[1].endswith("]")):
        return False
    # Frozen for loop: condition is a command substitution
    # like [expr {$i < 3}].  tclsh compiles the expr inline
    # but invokes the for command generically.
    emitter._push_lit("for")
    emitter._push_lit(args[0])  # init
    end_label = emitter._fresh_label("cmd_end")
    emitter._emit(Op.START_CMD, end_label, 1)
    emitter._emit_inline_cmd_subst(args[1])
    emitter._place_label(end_label)
    emitter._push_lit(args[2])  # step
    emitter._push_lit(args[3])  # body
    emitter._emit(Op.INVOKE_STK1, 5)
    emitter._emit(Op.POP)
    emitter._used_generic_invoke = True
    return True


def codegen_while(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile frozen ``while`` loop to specialised opcodes."""
    if not (len(args) == 2 and args[0].startswith("[") and args[0].endswith("]")):
        return False
    # Frozen while loop: same pattern as frozen for.
    emitter._push_lit("while")
    end_label = emitter._fresh_label("cmd_end")
    emitter._emit(Op.START_CMD, end_label, 1)
    emitter._emit_inline_cmd_subst(args[0])
    emitter._place_label(end_label)
    emitter._push_lit(args[1])  # body
    emitter._emit(Op.INVOKE_STK1, 3)
    emitter._emit(Op.POP)
    emitter._used_generic_invoke = True
    return True


def codegen_upvar(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``upvar`` to specialised opcodes."""
    if not (emitter._is_proc and len(args) >= 3):
        return False
    # upvar level remote local [remote local ...]
    # Tcl 9.0: push level; load/push remote; upvar %vN; pop; nop; nop; nop
    level = args[0]
    pairs = args[1:]
    if len(pairs) % 2 != 0:
        return False
    for i in range(0, len(pairs), 2):
        remote, local = pairs[i], pairs[i + 1]
        emitter._push_lit(level)
        emitter._emit_value(remote)
        slot = emitter._lvt.intern(local)
        emitter._emit(Op.UPVAR, slot, comment=f'var "{local}"')
        emitter._emit(Op.POP)
        emitter._emit(Op.NOP)
        emitter._emit(Op.NOP)
        emitter._emit(Op.NOP)
    # Replaces a generic invoke — keep startCommand markers.
    emitter._seen_generic_invoke = True
    return True


def codegen_global(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``global`` to specialised opcodes."""
    if not (emitter._is_proc and args):
        return False
    # global var1 [var2 ...]
    # Tcl 9.0: push "::"; push varname; nsupvar %vN; pop; nop; nop; nop
    # The 3 nops replace tclsh's "push ""; pop" result sequence (3 bytes).
    # tclsh also interns "" as the command result literal; we do the same
    # so that our literal pool indices match tclsh's exactly.
    for varname in args:
        emitter._push_lit("::")
        emitter._push_lit(varname)
        slot = emitter._lvt.intern(varname)
        emitter._emit(Op.NSUPVAR, slot, comment=f'var "{varname}"')
        emitter._emit(Op.POP)
        emitter._lit.intern("")
        emitter._emit(Op.NOP)
        emitter._emit(Op.NOP)
        emitter._emit(Op.NOP)
    # Replaces a generic invoke — keep startCommand markers.
    emitter._seen_generic_invoke = True
    return True


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("return", codegen_return)
    REGISTRY.register_codegen("tailcall", codegen_tailcall)
    REGISTRY.register_codegen("error", codegen_error)
    REGISTRY.register_codegen("for", codegen_for)
    REGISTRY.register_codegen("while", codegen_while)
    REGISTRY.register_codegen("upvar", codegen_upvar)
    REGISTRY.register_codegen("global", codegen_global)
