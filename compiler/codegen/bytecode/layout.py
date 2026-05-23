"""Instruction layout and jump-size optimisation helpers."""

from __future__ import annotations


def optimise_jumps(
    instrs, labels: dict[str, int], jump4_to_jump1: dict, *, max_iters: int = 10
) -> None:
    """Replace 4-byte jumps with 1-byte jumps when relative offset fits."""
    for _ in range(max_iters):
        offset = 0
        for instr in instrs:
            instr.offset = offset
            offset += instr.size

        label_offsets: dict[str, int] = {}
        for label, instr_idx in labels.items():
            if instr_idx < len(instrs):
                label_offsets[label] = instrs[instr_idx].offset
            else:
                label_offsets[label] = offset

        changed = False
        for instr in instrs:
            short_op = jump4_to_jump1.get(instr.op)
            if short_op is None:
                continue
            target = instr.operands[0]
            if isinstance(target, str):
                if target.startswith(("switch_", "proc_exit_")):
                    continue
                if instr.comment in ("break", "continue", "try_on"):
                    continue
                target_off = label_offsets.get(target)
                if target_off is None:
                    continue
            else:
                target_off = target

            rel = target_off - instr.offset
            if -128 <= rel <= 127:
                instr.op = short_op
                changed = True

        if not changed:
            break


def resolve_layout(instrs, labels: dict[str, int]) -> dict[str, int]:
    """Assign final byte offsets and return label->offset mapping."""
    offset = 0
    for instr in instrs:
        instr.offset = offset
        offset += instr.size

    label_offsets: dict[str, int] = {}
    for label, instr_idx in labels.items():
        if instr_idx < len(instrs):
            label_offsets[label] = instrs[instr_idx].offset
        else:
            label_offsets[label] = offset
    return label_offsets
