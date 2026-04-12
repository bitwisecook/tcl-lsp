"""Data types for the bytecode assembly backend."""

from __future__ import annotations

from dataclasses import dataclass, field

from .opcodes import _OP_INFO, Op


@dataclass(slots=True)
class Instruction:
    """A single bytecode instruction (labels unresolved until layout)."""

    op: Op
    operands: tuple[int | str, ...]  # int = literal/imm, str = label ref
    comment: str = ""
    offset: int = -1  # filled by layout pass
    jump_table: dict[str, str] | None = None  # pattern → label (JUMP_TABLE only)
    no_fold: bool = False  # prevent push-pop folding (jump target result)
    source_line: int = 0  # 1-based line within compilation unit (for errorInfo)
    source_cmd_text: str = ""  # original command text (pre-substitution) for errorInfo

    @property
    def size(self) -> int:
        return _OP_INFO[self.op][1]

    @property
    def mnemonic(self) -> str:
        return _OP_INFO[self.op][0]


class LiteralTable:
    """Intern pool mapping literal strings to object-array indices."""

    __slots__ = ("_entries", "_index")

    def __init__(self) -> None:
        self._entries: list[str] = []
        self._index: dict[str, int] = {}

    def intern(self, value: str) -> int:
        idx = self._index.get(value)
        if idx is not None:
            return idx
        idx = len(self._entries)
        self._entries.append(value)
        self._index[value] = idx
        return idx

    def register(self, value: str) -> int:
        """Always append *value*, even if it already exists (no dedup)."""
        idx = len(self._entries)
        self._entries.append(value)
        return idx

    def entries(self) -> list[str]:
        return list(self._entries)

    def __len__(self) -> int:
        return len(self._entries)


class LocalVarTable:
    """Maps variable names to LVT slot indices."""

    __slots__ = ("_slots", "_index")

    def __init__(self, params: tuple[str, ...] = ()) -> None:
        self._slots: list[str] = []
        self._index: dict[str, int] = {}
        for p in params:
            self.intern(p)

    def intern(self, name: str) -> int:
        idx = self._index.get(name)
        if idx is not None:
            return idx
        idx = len(self._slots)
        self._slots.append(name)
        self._index[name] = idx
        return idx

    def entries(self) -> list[str]:
        return list(self._slots)

    def __len__(self) -> int:
        return len(self._slots)


@dataclass(slots=True)
class FunctionAsm:
    """Complete assembly for one CFG function."""

    name: str
    literals: LiteralTable
    lvt: LocalVarTable
    instructions: list[Instruction]
    labels: dict[str, int]  # label → byte offset
    # VM-computed caches (populated lazily by the bytecode VM)
    _cached_offset_map: dict[int, int] | None = field(default=None, repr=False, compare=False)
    _cached_loops: list | None = field(default=None, repr=False, compare=False)


@dataclass(slots=True)
class ModuleAsm:
    """Assembly for an entire module."""

    top_level: FunctionAsm
    procedures: dict[str, FunctionAsm]
