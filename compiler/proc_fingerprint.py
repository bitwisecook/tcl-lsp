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

"""Per-proc dependency fingerprint for incremental diagnostic memoization (P6).

Phase 6 memoises a procedure's *diagnostic results* across edits, keyed by the
proc body hash already used for the ``FunctionUnit`` cache **plus** a
*dependency fingerprint*: the set of **external symbols** the proc references —
the commands it invokes and the global / namespace (``::``-qualified) variables
it touches.  A proc's *local* diagnostics (W123 unknown-command, var lifecycle,
style, …) can only change when its body changes or when one of those external
symbols' resolution changes, so recomputing only procs whose body-hash or
fingerprint changed — and merging cached per-proc diagnostics in source order —
skips the bulk of per-edit work.  (Cross-proc-inherent diagnostics — interproc
purity/taint, duplicate-proc, iRules connection scope — recompute over the
cheap cached summaries instead.)

This module is the *foundation* (a pure, testable function); the memoization
wiring in ``server/workspace/document_state.py`` is gated behind the
snapshot/restore golden-equivalence contract (``tests/test_incremental_update.py``
— incremental diagnostics must equal a full rebuild byte-for-byte) and is the
risk-bearing remainder, specified in ``docs/design/compiler/phases-3-5-6-design.md``.

The fingerprint is deliberately an **over-approximation**: collecting every
referenced command and global name is sound (it can only force *more*
recomputation, never stale results), which is the right bias for a cache.
"""

from __future__ import annotations

from shared.tokens import TokenType

from .expr_ast import command_texts_in_expr_node, vars_in_expr_node
from .ir import (
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRProcedure,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .parsing.green_tree import tokenise


def _walk(script: IRScript):
    """Yield every statement in *script*, descending control-flow bodies."""
    for stmt in script.statements:
        yield stmt
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from _walk(clause.body)
            if stmt.else_body:
                yield from _walk(stmt.else_body)
        elif isinstance(stmt, IRFor):
            yield from _walk(stmt.init)
            yield from _walk(stmt.body)
            yield from _walk(stmt.next)
        elif isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRBlock)):
            yield from _walk(stmt.body)
        elif isinstance(stmt, IRTry):
            yield from _walk(stmt.body)
            for handler in stmt.handlers:
                yield from _walk(handler.body)
            if stmt.finally_body:
                yield from _walk(stmt.finally_body)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body:
                    yield from _walk(arm.body)
            if stmt.default_body:
                yield from _walk(stmt.default_body)


def _scan_word(text: str, deps: set[str]) -> None:
    """Collect command names (first word of each ``[...]`` substitution) and
    ``::``-qualified variable refs from a word's text, recursing into nested
    substitutions."""
    if not text or ("[" not in text and "$::" not in text and "${::" not in text):
        return
    tokens, _ = tokenise(text, 0, 0, 0)
    for tok in tokens:
        if tok.type is TokenType.VAR:
            name = tok.text.strip("{}")
            if name.startswith("::"):
                deps.add(f"var:{name}")
        elif tok.type is TokenType.CMD and tok.text:
            inner = tok.text.lstrip()
            # First whitespace-delimited word is the invoked command name.
            head = inner.split(None, 1)[0] if inner else ""
            if head and "$" not in head and "[" not in head:
                deps.add(f"cmd:{head}")
            _scan_word(tok.text, deps)  # nested subs / globals


def _statement_words(stmt: IRStatement) -> list[str]:
    words: list[str] = []
    if isinstance(stmt, (IRCall, IRBarrier)):
        words.append(stmt.command)
        words.extend(stmt.args)
    elif isinstance(stmt, IRAssignValue):
        words.append(stmt.value)
    elif isinstance(stmt, IRIncr):
        if stmt.amount is not None:
            words.append(stmt.amount)
    elif isinstance(stmt, IRReturn):
        if stmt.value is not None:
            words.append(stmt.value)
        if stmt.expr is not None:
            words.extend(command_texts_in_expr_node(stmt.expr))
    elif isinstance(stmt, (IRAssignExpr, IRExprEval)):
        words.extend(command_texts_in_expr_node(stmt.expr))
    # Control-flow heads carry dependencies too: a command/global in an
    # if/while/for condition, a foreach list, or a switch subject/pattern is an
    # external reference just like a body command.  _walk recurses the *bodies*
    # but not these head words, so collect them here (over-approximate: scan
    # every command substitution, including lazy ternary/&&/|| arms).
    elif isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            words.extend(command_texts_in_expr_node(clause.condition))
    elif isinstance(stmt, (IRWhile, IRFor)):
        words.extend(command_texts_in_expr_node(stmt.condition))
    elif isinstance(stmt, IRForeach):
        words.extend(listword for _vars, listword in stmt.iterators)
    elif isinstance(stmt, IRSwitch):
        words.append(stmt.subject)
        words.extend(arm.pattern for arm in stmt.arms)
    return [w for w in words if w]


def dependency_fingerprint(proc: IRProcedure) -> frozenset[str]:
    """External symbols *proc* depends on: invoked command names (top-level and
    inside ``[...]`` substitutions, in every nested control-flow body) plus
    referenced global / namespace (``::``-qualified) variable names.

    Local variables and parameters are excluded — they are internal and cannot
    be changed by an edit elsewhere.  Over-approximate by design (more deps ⇒
    more recomputation, never stale results).  Order-independent, suitable for
    hashing into a per-proc diagnostic cache key.
    """
    deps: set[str] = set()
    for stmt in _walk(proc.body):
        if isinstance(stmt, (IRCall, IRBarrier)):
            if stmt.command:
                deps.add(f"cmd:{stmt.command}")
            for name in (*stmt.defs, *getattr(stmt, "reads", ())):
                if name.startswith("::"):
                    deps.add(f"var:{name}")
        # Globals referenced directly in an expr AST (``$::config`` inside an
        # ``[expr {...}]``) are ExprVars, not command substitutions.
        for expr in _statement_exprs(stmt):
            for name in vars_in_expr_node(expr):
                if name.startswith("::"):
                    deps.add(f"var:{name}")
        for word in _statement_words(stmt):
            _scan_word(word, deps)
    return frozenset(deps)


def _statement_exprs(stmt: IRStatement):
    if isinstance(stmt, (IRAssignExpr, IRExprEval)):
        yield stmt.expr
    elif isinstance(stmt, IRReturn) and stmt.expr is not None:
        yield stmt.expr
    elif isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            yield clause.condition
    elif isinstance(stmt, (IRWhile, IRFor)) and stmt.condition is not None:
        yield stmt.condition
