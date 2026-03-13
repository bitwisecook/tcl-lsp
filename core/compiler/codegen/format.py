"""Formatting helpers for bytecode disassembly output."""

from __future__ import annotations

from .opcodes import _INDEX_END, _JUMP_OPS, _LVT_OPS, _STR_CLASS_NAMES, Op


def _esc(text: str, limit: int = 40) -> str:
    """Escape a literal value for disassembly comments.

    Matches Tcl's disassembler: backslashes are NOT doubled; control
    characters and non-ASCII codepoints are escaped.
    """
    parts: list[str] = []
    for ch in text:
        cp = ord(ch)
        if ch == '"':
            parts.append('\\"')
        elif ch == "\n":
            parts.append("\\n")
        elif ch == "\t":
            parts.append("\\t")
        elif ch == "\r":
            parts.append("\\r")
        elif ch == "\v":
            parts.append("\\v")
        elif ch == "\f":
            parts.append("\\f")
        elif cp == 0:
            parts.append("\\u0000")
        elif cp > 0x7E:
            parts.append(f"\\u{cp:04x}")
        else:
            parts.append(ch)
    s = "".join(parts)
    if len(s) > limit:
        return s[: limit - 3] + "..."
    return s


def format_function_asm(asm) -> str:
    """Render FunctionAsm to disassembly text."""
    lines: list[str] = []

    total_bytes = sum(i.size for i in asm.instructions)
    lines.append(
        f"ByteCode {asm.name}, {len(asm.instructions)} instructions, "
        f"{total_bytes} bytes, {len(asm.literals)} literals, "
        f"{len(asm.lvt)} variables"
    )

    if asm.literals.entries():
        lines.append("  Literals:")
        for i, lit in enumerate(asm.literals.entries()):
            lines.append(f'    {i}: "{_esc(lit)}"')

    if asm.lvt.entries():
        lines.append("  Local variables:")
        for i, var in enumerate(asm.lvt.entries()):
            lines.append(f'    %v{i}: "{var}"')

    lines.append("  Instructions:")

    off2labels: dict[int, list[str]] = {}
    for label, off in asm.labels.items():
        off2labels.setdefault(off, []).append(label)

    for instr in asm.instructions:
        if instr.offset in off2labels:
            for lbl in off2labels[instr.offset]:
                lines.append(f"  # {lbl}:")

        parts: list[str] = []
        jump_comment = ""
        for j, operand in enumerate(instr.operands):
            if isinstance(operand, str) and instr.op in _JUMP_OPS:
                target = asm.labels.get(operand, 0)
                relative = target - instr.offset
                parts.append(f"+{relative}" if relative >= 0 else str(relative))
                jump_comment = f"\t# pc {target}"
            elif isinstance(operand, str) and instr.op == Op.START_CMD and j == 0:
                target = asm.labels.get(operand, 0)
                relative = target - instr.offset
                parts.append(f"+{relative}" if relative >= 0 else str(relative))
                count = instr.operands[1] if len(instr.operands) > 1 else 1
                jump_comment = f"\t# next cmd at pc {target}, {count} cmds start here"
            elif isinstance(operand, str):
                target = asm.labels.get(operand, 0)
                parts.append(f"pc {target}")
            elif instr.op in _LVT_OPS and j == 0:
                parts.append(f"%v{operand}")
            elif instr.op in (Op.DICT_SET, Op.DICT_UNSET, Op.DICT_INCR_IMM) and j == 1:
                parts.append(f"%v{operand}")
            elif instr.op in (
                Op.INCR_SCALAR1_IMM,
                Op.INCR_STK_IMM,
                Op.INCR_ARRAY_STK_IMM,
                Op.DICT_INCR_IMM,
                Op.STR_MATCH,
                Op.REGEXP,
            ):
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op in (Op.RETURN_IMM, Op.SYNTAX) and j == 0:
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op == Op.STR_CLASS and isinstance(operand, int):
                parts.append(_STR_CLASS_NAMES.get(operand, str(operand)))
            elif (
                instr.op in (Op.LIST_INDEX_IMM, Op.LIST_RANGE_IMM, Op.STR_RANGE_IMM)
                and isinstance(operand, int)
                and operand <= _INDEX_END
            ):
                if operand == _INDEX_END:
                    parts.append("end")
                else:
                    parts.append(f"end{operand - _INDEX_END}")
            else:
                parts.append(str(operand))

        ops_str = " ".join(parts)
        spacer = " " if ops_str else ""
        comment = (
            jump_comment if jump_comment else (f"\t# {instr.comment}" if instr.comment else "")
        )
        lines.append(f"    ({instr.offset}) {instr.mnemonic}{spacer}{ops_str}{comment}")

        if instr.op == Op.JUMP_TABLE and instr.jump_table:
            entries = []
            for pattern, label in instr.jump_table.items():
                target_pc = asm.labels.get(label, 0)
                entries.append(f'"{_esc(pattern)}"->pc {target_pc}')
            lines.append(f"\t\t[{', '.join(entries)}]")

    return "\n".join(lines)


def format_module_asm(module) -> str:
    """Format entire module assembly."""
    parts = [format_function_asm(module.top_level)]
    for name in sorted(module.procedures):
        parts.append("")
        parts.append(format_function_asm(module.procedures[name]))
    return "\n".join(parts)
