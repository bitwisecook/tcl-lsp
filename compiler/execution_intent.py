# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Pre-codegen execution intent facts derived from IR/CFG statements.

Phase 2 introduces a lightweight intent layer that can be shared by
analysis passes as an optional fast-path without changing behaviour.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from compiler.parsing.green_tree import tokenise
from compiler.registry import REGISTRY
from shared.tokens import TokenType

from .cfg import CFGFunction
from .ir import IRAssignValue


class InvocationShape(str, Enum):
    """How a value is executed/evaluated by Tcl at runtime."""

    COMMAND_SUBSTITUTION = "command_substitution"


class SubstitutionCategory(str, Enum):
    """High-level argument substitution category for command invocations."""

    LITERAL = "literal"
    SCALAR_VAR = "scalar_var"
    ARRAY_VAR = "array_var"
    NESTED_COMMAND = "nested_command"
    MIXED = "mixed"


class SideEffectClass(str, Enum):
    """Conservative side-effect classification for command substitutions."""

    PURE = "pure"
    MAY_SIDE_EFFECT = "may_side_effect"


class EscapeClass(str, Enum):
    """Conservative escape classification for command substitutions."""

    NO_ESCAPE = "no_escape"
    MAY_ESCAPE = "may_escape"


@dataclass(frozen=True, slots=True)
class CommandSubstitutionIntent:
    """Intent for a bracketed value like ``[llength $x]``."""

    command: str
    args: tuple[str, ...]
    arg_categories: tuple[SubstitutionCategory, ...]
    side_effect: SideEffectClass
    escape: EscapeClass
    shimmer_pressure: int
    invocation_shape: InvocationShape = InvocationShape.COMMAND_SUBSTITUTION
    # True when any argument uses ``{*}`` expansion: the lexer drops the EXPAND
    # token and the expanded word collapses to one arg, so ``args`` no longer
    # reflects the runtime arity (``[list {*}{a b}]`` is one arg but two
    # elements).  Consumers that count args (e.g. interval-bounds list lengths)
    # must not infer arity when this is set.
    has_expansion: bool = False


@dataclass(frozen=True, slots=True)
class FunctionExecutionIntent:
    """Per-function intent facts keyed by CFG statement coordinates."""

    command_substitutions: dict[tuple[str, int], CommandSubstitutionIntent]


def _token_text_for_intent(tok) -> str:
    """Return token text that preserves substitution sigils where relevant."""
    if tok.type is TokenType.VAR:
        return "$" + tok.text
    if tok.type is TokenType.CMD:
        return "[" + tok.text + "]"
    return tok.text


def _categorise_arg(text: str) -> SubstitutionCategory:
    stripped = text.strip()
    if "[" in stripped and "]" in stripped:
        return SubstitutionCategory.NESTED_COMMAND
    if stripped.startswith("$"):
        if "(" in stripped and ")" in stripped:
            return SubstitutionCategory.ARRAY_VAR
        return SubstitutionCategory.SCALAR_VAR
    if any(ch in stripped for ch in ("$", "[", "\\")):
        return SubstitutionCategory.MIXED
    return SubstitutionCategory.LITERAL


def _shimmer_pressure(categories: tuple[SubstitutionCategory, ...]) -> int:
    """Estimate type-conversion pressure from substitution categories."""
    pressure = 0
    for cat in categories:
        if cat in (SubstitutionCategory.SCALAR_VAR, SubstitutionCategory.ARRAY_VAR):
            pressure += 1
        elif cat in (SubstitutionCategory.NESTED_COMMAND, SubstitutionCategory.MIXED):
            pressure += 2
    return pressure


def _classify_side_effect(command: str, args: tuple[str, ...]) -> SideEffectClass:
    from .side_effects import classify_side_effects

    effect = classify_side_effects(command, args)
    return SideEffectClass.PURE if effect.pure else SideEffectClass.MAY_SIDE_EFFECT


def _classify_escape(command: str, categories: tuple[SubstitutionCategory, ...]) -> EscapeClass:
    if REGISTRY.is_dynamic_barrier(command):
        return EscapeClass.MAY_ESCAPE
    if any(
        cat in (SubstitutionCategory.NESTED_COMMAND, SubstitutionCategory.MIXED)
        for cat in categories
    ):
        return EscapeClass.MAY_ESCAPE
    return EscapeClass.NO_ESCAPE


def _parse_command_substitution(value: str) -> CommandSubstitutionIntent | None:
    stripped = value.strip()
    if not (stripped.startswith("[") and stripped.endswith("]")):
        return None

    inner = stripped[1:-1]
    argv: list[str] = []
    prev_type = TokenType.EOL

    lex_tokens, _ = tokenise(inner, 0, 0, 0)
    saw_command_terminator = False
    has_expansion = False
    for tok in lex_tokens:
        if tok.type is TokenType.EXPAND:
            # ``{*}`` prefix: the following word expands at runtime, so the arg
            # count no longer reflects the element count.  Only flag it — fall
            # through so the empty EXPAND token still starts a new word exactly
            # as before (the next token merges onto it).
            has_expansion = True
        if tok.type in (TokenType.COMMENT, TokenType.SEP):
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            # Single-command only: remember terminator and reject further words.
            saw_command_terminator = bool(argv)
            prev_type = tok.type
            continue
        if saw_command_terminator:
            return None
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(_token_text_for_intent(tok))
        elif argv:
            argv[-1] += _token_text_for_intent(tok)
        else:
            argv.append(_token_text_for_intent(tok))
        prev_type = tok.type

    if not argv:
        return None

    args = tuple(argv[1:])
    categories = tuple(_categorise_arg(arg) for arg in args)
    command = argv[0]
    return CommandSubstitutionIntent(
        command=command,
        args=args,
        arg_categories=categories,
        side_effect=_classify_side_effect(command, args),
        escape=_classify_escape(command, categories),
        shimmer_pressure=_shimmer_pressure(categories),
        has_expansion=has_expansion,
    )


def build_function_execution_intent(cfg: CFGFunction) -> FunctionExecutionIntent:
    """Build lightweight intent facts from CFG statements."""
    substitutions: dict[tuple[str, int], CommandSubstitutionIntent] = {}

    for block_name, block in cfg.blocks.items():
        for idx, stmt in enumerate(block.statements):
            if not isinstance(stmt, IRAssignValue):
                continue
            parsed = _parse_command_substitution(stmt.value)
            if parsed is not None:
                substitutions[(block_name, idx)] = parsed

    return FunctionExecutionIntent(command_substitutions=substitutions)
