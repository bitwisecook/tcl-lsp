"""Global Value Numbering (GVN), CSE, PRE, and LICM-style hints.

Walks SSA/CFG structures to detect:

- **Full redundancies** (classic GVN/CSE): the same pure computation
  appears twice and the first dominates the second.
- **Partial redundancies** (GVN-PRE hybrid): a computation is already
  available on some incoming paths, but not all.

Redundancies are reported as ``O105`` diagnostics suggesting extraction
to a local variable or hoisting before a branch.

Also reports:

- **Loop-invariant computations** (LICM-style): pure computations in a
  loop body whose inputs are not modified by that loop and which run on
  every iteration.  Reported as ``O106``.

The pass handles:

- Standalone pure command calls (``IRCall`` nodes)
- Command substitutions embedded in arguments and values
  (``[HTTP::uri]``, ``[string length $x]``, etc.)
- User-defined pure procs (via ``InterproceduralAnalysis``)
- iRules ``when`` event handler bodies (via token-level flat scan,
  plus optional body-local CFG/SSA fallback for path-sensitive cases)

The iRules ``when`` body scanner also detects standalone expensive
command invocations (e.g. bare ``HTTP::uri``), subsuming the former
``IRULE2102`` check from ``irules_flow.py``.

Kill semantics are effect-aware: barriers and mutators invalidate
tracked value numbers; read-only/output commands do not.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TypeAlias

from ..analysis.semantic_model import Range
from ..commands.registry import REGISTRY
from ..common.dialect import active_dialect
from ..common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ..common.ranges import range_from_token
from ..parsing.lexer import TclLexer
from ..parsing.tokens import SourcePosition, Token, TokenType
from .cfg import CFGBranch, CFGFunction, CFGGoto
from .compilation_unit import CompilationUnit, ensure_compilation_unit
from .core_analyses import FunctionAnalysis
from .interprocedural import InterproceduralAnalysis, resolve_call_target
from .ir import (
    CommandTokens,
    IRBarrier,
    IRCall,
)
from .irules_flow import _find_when_bodies, _walk_body_commands
from .side_effects import EffectRegion, classify_side_effects
from .ssa import BlockName, SSAFunction, SSAVersion
from .var_refs import VarReferenceScanner, VarScanOptions

log = logging.getLogger(__name__)
_CONTROL_FLOW_COMMANDS = REGISTRY.control_flow_commands()
_VAR_REF_SCANNER = VarReferenceScanner(
    VarScanOptions(
        include_var_read_roles=False,
        recurse_cmd_substitutions=True,
    )
)

# Public types

ExprKey: TypeAlias = tuple[str, ...]
"""Hashable canonical identity for a computed expression."""


@dataclass(frozen=True, slots=True)
class RedundantComputation:
    """A computation that re-evaluates an already-available expression."""

    range: Range
    first_range: Range
    expression_text: str
    code: str = "O105"
    message: str = ""


# Purity model — backed by the command registry


def _is_pure_command(
    command: str,
    args: tuple[str, ...],
    interproc: InterproceduralAnalysis | None = None,
    caller_name: str = "::top",
) -> bool:
    """Return True if the command invocation is pure for GVN purposes."""
    callee_summary = None
    if interproc is not None:
        known = set(interproc.procedures)
        target = resolve_call_target(command, args, caller_name, known)
        if target is None:
            target = _normalise_qualified_name(command)
        callee_summary = interproc.procedures.get(target)
    effect = classify_side_effects(command, args, callee_summary=callee_summary)
    return effect.pure


def _is_worth_reporting(
    command: str,
    interproc: InterproceduralAnalysis | None = None,
    caller_name: str = "::top",
    args: tuple[str, ...] = (),
) -> bool:
    """Return True if redundant use of this command is worth flagging."""
    if REGISTRY.is_cse_candidate(command):
        return True
    # User-defined pure procs are always worth reporting.
    if interproc is not None:
        known = set(interproc.procedures)
        qname = resolve_call_target(command, args, caller_name, known)
        if qname is None:
            qname = _normalise_qualified_name(command)
        summary = interproc.procedures.get(qname)
        if summary is not None and summary.pure:
            return True
    return False


# Scoped value table


@dataclass(frozen=True, slots=True)
class _ValueEntry:
    key: ExprKey
    block: BlockName
    statement_index: int
    range: Range
    expression_text: str


@dataclass(frozen=True, slots=True)
class _ExprOccurrence:
    """One pure expression occurrence observed in a statement stream."""

    key: ExprKey
    range: Range
    expression_text: str
    block: BlockName
    statement_index: int
    variable_uses: frozenset[str]


class _ScopedValueTable:
    """Dominator-tree-scoped hash table for value numbering.

    Each ``push_scope`` / ``pop_scope`` pair brackets the processing of
    a dominator-tree subtree.  Lookups search the current scope and all
    ancestor scopes.  ``kill_all`` discards everything (barrier semantics).
    """

    def __init__(self) -> None:
        self._scopes: list[dict[ExprKey, _ValueEntry]] = [{}]

    def push_scope(self) -> None:
        self._scopes.append({})

    def pop_scope(self) -> None:
        if len(self._scopes) > 1:
            self._scopes.pop()

    def lookup(self, key: ExprKey) -> _ValueEntry | None:
        for scope in reversed(self._scopes):
            entry = scope.get(key)
            if entry is not None:
                return entry
        return None

    def insert(self, key: ExprKey, entry: _ValueEntry) -> None:
        self._scopes[-1][key] = entry

    def kill_all(self) -> None:
        """Invalidate all tracked values (barrier/impure call)."""
        self._scopes = [{}]


# Key construction helpers


def _canonicalise_word(text: str, uses: dict[str, SSAVersion]) -> str:
    """Replace variable references with SSA-versioned canonical forms.

    ``$x`` at SSA version 3 becomes ``$x@3`` so the same variable at
    the same version in different blocks maps to the same key.
    """
    if not uses:
        return text
    result = text
    # Sort by name length descending so $longname isn't partially
    # matched before $long.
    for name, ver in sorted(uses.items(), key=lambda x: -len(x[0])):
        versioned = f"${name}@{ver}"
        result = result.replace(f"${{{name}}}", versioned)
        result = result.replace(f"${name}", versioned)
    return result


def _build_call_key(
    command: str,
    args: tuple[str, ...],
    uses: dict[str, SSAVersion],
) -> ExprKey:
    """Build a canonical key for a pure command invocation."""
    parts: list[str] = ["call", command]
    for arg in args:
        parts.append(_canonicalise_word(arg, uses))
    return tuple(parts)


def _format_expression_text(command: str, args: tuple[str, ...]) -> str:
    if not args:
        return command
    return command + " " + " ".join(args)


def _full_redundancy_message(expression_text: str) -> str:
    return (
        f"'{expression_text}' computed again with the same arguments. "
        "Consider storing the result in a local variable."
    )


def _partial_redundancy_message(expression_text: str) -> str:
    return (
        f"'{expression_text}' is partially redundant across control-flow "
        "paths. Consider hoisting it before the branch."
    )


def _loop_invariant_message(expression_text: str) -> str:
    return (
        f"'{expression_text}' is loop-invariant and re-computed on each "
        "iteration. Consider hoisting it before the loop."
    )


def _vars_in_word(text: str) -> set[str]:
    """Return normalized variable names referenced in one Tcl word."""
    return _VAR_REF_SCANNER.scan_word(text)


def _vars_in_args(args: tuple[str, ...]) -> frozenset[str]:
    vars_found: set[str] = set()
    for arg in args:
        vars_found |= _vars_in_word(arg)
    return frozenset(vars_found)


def _statement_occurrences(
    ir_stmt,
    uses: dict[str, SSAVersion],
    source: str,
    block_name: BlockName,
    statement_index: int,
    *,
    interproc: InterproceduralAnalysis | None = None,
    caller_name: str = "::top",
) -> list[_ExprOccurrence]:
    """Collect pure expression occurrences from one statement."""
    occurrences: list[_ExprOccurrence] = []

    if isinstance(ir_stmt, IRCall) and _is_pure_command(
        ir_stmt.command,
        ir_stmt.args,
        interproc,
        caller_name,
    ):
        if _is_worth_reporting(ir_stmt.command, interproc, caller_name, args=ir_stmt.args):
            stmt_range = getattr(ir_stmt, "range", None)
            if stmt_range is not None:
                occurrences.append(
                    _ExprOccurrence(
                        key=_build_call_key(ir_stmt.command, ir_stmt.args, uses),
                        range=stmt_range,
                        expression_text=_format_expression_text(ir_stmt.command, ir_stmt.args),
                        block=block_name,
                        statement_index=statement_index,
                        variable_uses=_vars_in_args(ir_stmt.args),
                    )
                )

    for tok in _cmd_tokens_from_statement(ir_stmt, source):
        parsed = _parse_cmd_token(tok.text)
        if parsed is None:
            continue
        cmd_name, cmd_args = parsed
        if not _is_pure_command(cmd_name, cmd_args, interproc, caller_name):
            continue
        occurrences.append(
            _ExprOccurrence(
                key=_build_call_key(cmd_name, cmd_args, uses),
                range=range_from_token(tok),
                expression_text=_format_expression_text(cmd_name, cmd_args),
                block=block_name,
                statement_index=statement_index,
                variable_uses=_vars_in_args(cmd_args),
            )
        )

    return occurrences


def _statement_writes_state(
    ir_stmt,
    source: str,
    *,
    interproc: InterproceduralAnalysis | None = None,
    caller_name: str = "::top",
) -> bool:
    """Return True when statement effects must invalidate value numbering."""
    if isinstance(ir_stmt, IRBarrier):
        return True

    if isinstance(ir_stmt, IRCall):
        callee_summary = None
        if interproc is not None:
            known = set(interproc.procedures)
            target = resolve_call_target(ir_stmt.command, ir_stmt.args, caller_name, known)
            if target is None:
                target = _normalise_qualified_name(ir_stmt.command)
            callee_summary = interproc.procedures.get(target)
        effect = classify_side_effects(
            ir_stmt.command,
            ir_stmt.args,
            callee_summary=callee_summary,
        )
        _reads, writes = effect.to_effect_regions()
        if writes != EffectRegion.NONE:
            return True

    for tok in _cmd_tokens_from_statement(ir_stmt, source):
        parsed = _parse_cmd_token(tok.text)
        if parsed is None:
            continue
        cmd_name, cmd_args = parsed
        callee_summary = None
        if interproc is not None:
            known = set(interproc.procedures)
            target = resolve_call_target(cmd_name, cmd_args, caller_name, known)
            if target is None:
                target = _normalise_qualified_name(cmd_name)
            callee_summary = interproc.procedures.get(target)
        effect = classify_side_effects(cmd_name, cmd_args, callee_summary=callee_summary)
        _reads, writes = effect.to_effect_regions()
        if writes != EffectRegion.NONE:
            return True

    return False


# Token scanning for embedded command substitutions


def _parse_cmd_token(text: str) -> tuple[str, tuple[str, ...]] | None:
    """Parse a CMD token's text into (command_name, args).

    CMD token text is the content *inside* the brackets, e.g.
    ``"HTTP::uri"`` or ``"string length $x"``.

    Variable references are preserved with their ``$`` prefix so that
    downstream canonicalisation can locate and replace them with
    SSA-versioned forms.
    """
    lexer = TclLexer(text)
    argv: list[str] = []
    prev_sep = True

    while True:
        tok = lexer.get_token()
        if tok is None or tok.type in (TokenType.EOL, TokenType.EOF):
            break
        if tok.type in (TokenType.SEP, TokenType.COMMENT):
            prev_sep = True
            continue
        # Preserve $ prefix for variable tokens so canonicalisation works.
        piece = f"${tok.text}" if tok.type is TokenType.VAR else tok.text
        if prev_sep:
            argv.append(piece)
        else:
            if argv:
                argv[-1] += piece
            else:
                argv.append(piece)
        prev_sep = False

    if not argv:
        return None
    return argv[0], tuple(argv[1:])


def _find_cmd_tokens_in_text(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
) -> list[Token]:
    """Find all CMD tokens in *text* (a word or value string)."""
    lexer = TclLexer(
        text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )
    result: list[Token] = []
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.CMD:
            result.append(tok)
    return result


def _cmd_tokens_from_statement(
    ir_stmt,
    source: str,
) -> list[Token]:
    """Extract all CMD tokens from an IR statement's source text."""
    ct: CommandTokens | None = getattr(ir_stmt, "tokens", None)
    if ct is not None and ct.all_tokens:
        return [t for t in ct.all_tokens if t.type is TokenType.CMD]

    # Synthetic CFG-lowering defs (e.g. foreach/catch/try header nodes)
    # carry a broad statement range but no real token stream; scanning
    # their range would incorrectly attribute nested body CMD tokens.
    if isinstance(ir_stmt, IRCall) and ir_stmt.defs and not ir_stmt.args and ct is None:
        return []

    # Fall back to lexing the source range.
    stmt_range: Range | None = getattr(ir_stmt, "range", None)
    if stmt_range is None:
        return []

    start = stmt_range.start.offset
    end = stmt_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return []

    return _find_cmd_tokens_in_text(
        source[start : end + 1],
        base_offset=start,
        base_line=stmt_range.start.line,
        base_col=stmt_range.start.character,
    )


def _cmd_tokens_from_assign_value(
    value: str,
    stmt_range: Range,
    source: str,
) -> list[Token]:
    """Extract CMD tokens from an IRAssignValue's value field.

    The ``value`` field is the raw RHS text, e.g. ``[HTTP::uri]`` or
    ``[IP::client_addr]:[TCP::client_port]``.  We need source positions,
    so we locate the value within the source and lex from there.
    """
    start = stmt_range.start.offset
    end = stmt_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return []

    stmt_text = source[start : end + 1]
    # The value appears somewhere in the statement text.
    # For `set var [cmd]`, the value is the last word.
    # Re-lex the full statement to find CMD tokens.
    return _find_cmd_tokens_in_text(
        stmt_text,
        base_offset=start,
        base_line=stmt_range.start.line,
        base_col=stmt_range.start.character,
    )


def _cfg_successors(cfg: CFGFunction, block_name: BlockName) -> tuple[BlockName, ...]:
    block = cfg.blocks[block_name]
    term = block.terminator
    match term:
        case None:
            return ()
        case CFGGoto(target=target):
            return (target,)
        case CFGBranch(true_target=true_target, false_target=false_target):
            return (true_target, false_target)
        case _:
            return ()


def _cfg_predecessors(
    cfg: CFGFunction, executable: set[BlockName]
) -> dict[BlockName, set[BlockName]]:
    preds: dict[BlockName, set[BlockName]] = {bn: set() for bn in executable}
    for bn in executable:
        for succ in _cfg_successors(cfg, bn):
            if succ in preds:
                preds[succ].add(bn)
    return preds


def _cfg_order(cfg: CFGFunction, executable: set[BlockName]) -> list[BlockName]:
    """Return a stable forward traversal order for executable blocks."""
    seen: set[BlockName] = set()
    order: list[BlockName] = []
    stack: list[BlockName] = [cfg.entry]

    while stack:
        bn = stack.pop()
        if bn in seen or bn not in executable:
            continue
        seen.add(bn)
        order.append(bn)
        succs = [s for s in _cfg_successors(cfg, bn) if s in executable]
        for succ in reversed(succs):
            stack.append(succ)

    for bn in cfg.blocks:
        if bn in executable and bn not in seen:
            order.append(bn)
    return order


def _collect_function_occurrence_events(
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
    source: str,
    *,
    interproc: InterproceduralAnalysis | None = None,
) -> tuple[dict[BlockName, tuple[_ExprOccurrence | None, ...]], list[_ExprOccurrence]]:
    """Collect per-block occurrence streams; ``None`` marks a barrier kill."""
    executable = set(cfg.blocks) - analysis.unreachable_blocks
    events_by_block: dict[BlockName, tuple[_ExprOccurrence | None, ...]] = {
        bn: () for bn in executable
    }
    all_occurrences: list[_ExprOccurrence] = []

    for block_name in executable:
        cfg_block = cfg.blocks[block_name]
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            continue

        events: list[_ExprOccurrence | None] = []
        stmt_count = min(len(cfg_block.statements), len(ssa_block.statements))

        for idx in range(stmt_count):
            ir_stmt = cfg_block.statements[idx]
            ssa_stmt = ssa_block.statements[idx]
            if _statement_writes_state(
                ir_stmt,
                source,
                interproc=interproc,
                caller_name=cfg.name,
            ):
                events.append(None)
                continue

            occurrences = _statement_occurrences(
                ir_stmt,
                ssa_stmt.uses,
                source,
                block_name,
                idx,
                interproc=interproc,
                caller_name=cfg.name,
            )
            events.extend(occurrences)
            all_occurrences.extend(occurrences)

        events_by_block[block_name] = tuple(events)

    return events_by_block, all_occurrences


def _transfer_occurrence_keys(
    events: tuple[_ExprOccurrence | None, ...],
    in_set: set[ExprKey],
) -> set[ExprKey]:
    state = set(in_set)
    for event in events:
        if event is None:
            state.clear()
            continue
        state.add(event.key)
    return state


def _find_partial_redundancies(
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
    source: str,
    *,
    interproc: InterproceduralAnalysis | None = None,
) -> list[RedundantComputation]:
    """Detect path-partial redundancies via may/must-availability."""
    executable = set(cfg.blocks) - analysis.unreachable_blocks
    if ssa.entry not in executable:
        return []

    events_by_block, all_occurrences = _collect_function_occurrence_events(
        cfg,
        ssa,
        analysis,
        source,
        interproc=interproc,
    )
    if not all_occurrences:
        return []

    universe: set[ExprKey] = {occ.key for occ in all_occurrences}
    preds = _cfg_predecessors(cfg, executable)
    order = _cfg_order(cfg, executable)

    may_in: dict[BlockName, set[ExprKey]] = {bn: set() for bn in executable}
    may_out: dict[BlockName, set[ExprKey]] = {bn: set() for bn in executable}
    must_in: dict[BlockName, set[ExprKey]] = {
        bn: (set() if bn == ssa.entry else set(universe)) for bn in executable
    }
    must_out: dict[BlockName, set[ExprKey]] = {
        bn: _transfer_occurrence_keys(events_by_block.get(bn, ()), must_in[bn]) for bn in executable
    }

    changed = True
    while changed:
        changed = False
        for bn in order:
            pred_list = [p for p in preds.get(bn, set()) if p in executable]
            if bn == ssa.entry or not pred_list:
                new_must_in: set[ExprKey] = set()
                new_may_in: set[ExprKey] = set()
            else:
                new_must_in = set(must_out[pred_list[0]])
                for p in pred_list[1:]:
                    new_must_in &= must_out[p]

                new_may_in = set()
                for p in pred_list:
                    new_may_in |= may_out[p]

            new_must_out = _transfer_occurrence_keys(
                events_by_block.get(bn, ()),
                new_must_in,
            )
            new_may_out = _transfer_occurrence_keys(
                events_by_block.get(bn, ()),
                new_may_in,
            )

            if new_must_in != must_in[bn]:
                must_in[bn] = new_must_in
                changed = True
            if new_may_in != may_in[bn]:
                may_in[bn] = new_may_in
                changed = True
            if new_must_out != must_out[bn]:
                must_out[bn] = new_must_out
                changed = True
            if new_may_out != may_out[bn]:
                may_out[bn] = new_may_out
                changed = True

    key_offsets: dict[ExprKey, set[int]] = {}
    for occ in all_occurrences:
        key_offsets.setdefault(occ.key, set()).add(occ.range.start.offset)

    first_by_key: dict[ExprKey, _ExprOccurrence] = {}
    for occ in sorted(all_occurrences, key=lambda o: o.range.start.offset):
        first_by_key.setdefault(occ.key, occ)

    results: list[RedundantComputation] = []
    for bn in order:
        state_may = set(may_in.get(bn, set()))
        state_must = set(must_in.get(bn, set()))

        for event in events_by_block.get(bn, ()):
            if event is None:
                state_may.clear()
                state_must.clear()
                continue

            occ = event
            if len(key_offsets.get(occ.key, ())) >= 2:
                if occ.key in state_may and occ.key not in state_must:
                    first_occ = first_by_key.get(occ.key, occ)
                    if first_occ.range.start.offset != occ.range.start.offset:
                        results.append(
                            RedundantComputation(
                                range=occ.range,
                                first_range=first_occ.range,
                                expression_text=first_occ.expression_text,
                                message=_partial_redundancy_message(first_occ.expression_text),
                            )
                        )

            state_may.add(occ.key)
            state_must.add(occ.key)

    return results


def _dominates(
    ssa: SSAFunction,
    dominator: BlockName,
    node: BlockName,
) -> bool:
    """Return True if *dominator* dominates *node* in the SSA dominator tree."""
    current: BlockName | None = node
    while current is not None:
        if current == dominator:
            return True
        current = ssa.idom.get(current)
    return False


def _natural_loop_blocks(
    header: BlockName,
    latch: BlockName,
    preds: dict[BlockName, set[BlockName]],
    executable: set[BlockName],
) -> set[BlockName]:
    """Return blocks in the natural loop for one back-edge latch -> header."""
    blocks: set[BlockName] = {header, latch}
    work: list[BlockName] = [latch]

    while work:
        node = work.pop()
        for pred in preds.get(node, set()):
            if pred not in executable or pred in blocks:
                continue
            blocks.add(pred)
            if pred != header:
                work.append(pred)

    return blocks


def _loop_defined_variables(
    ssa: SSAFunction,
    loop_blocks: set[BlockName],
) -> frozenset[str]:
    """Return variable names defined anywhere inside a loop."""
    defs: set[str] = set()
    for bn in loop_blocks:
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        for phi in ssa_block.phis:
            defs.add(phi.name)
        for stmt in ssa_block.statements:
            defs |= set(stmt.defs)
    return frozenset(defs)


def _find_loop_invariants(
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
    source: str,
    *,
    interproc: InterproceduralAnalysis | None = None,
) -> list[RedundantComputation]:
    """Detect loop-invariant pure computations (LICM-style hints)."""
    executable = set(cfg.blocks) - analysis.unreachable_blocks
    if ssa.entry not in executable:
        return []

    events_by_block, _ = _collect_function_occurrence_events(
        cfg,
        ssa,
        analysis,
        source,
        interproc=interproc,
    )
    if not events_by_block:
        return []

    preds = _cfg_predecessors(cfg, executable)

    loop_blocks_by_header: dict[BlockName, set[BlockName]] = {}
    latches_by_header: dict[BlockName, set[BlockName]] = {}

    # A back edge is tail -> header where header dominates tail.
    for tail in executable:
        for succ in _cfg_successors(cfg, tail):
            if succ not in executable:
                continue
            if not _dominates(ssa, succ, tail):
                continue

            loop_blocks = _natural_loop_blocks(succ, tail, preds, executable)
            if succ in loop_blocks_by_header:
                loop_blocks_by_header[succ].update(loop_blocks)
            else:
                loop_blocks_by_header[succ] = set(loop_blocks)
            latches_by_header.setdefault(succ, set()).add(tail)

    if not loop_blocks_by_header:
        return []

    results: list[RedundantComputation] = []

    for header, loop_blocks in loop_blocks_by_header.items():
        latches = latches_by_header.get(header, set())
        if not latches:
            continue

        loop_defs = _loop_defined_variables(ssa, loop_blocks)

        for bn in loop_blocks:
            for event in events_by_block.get(bn, ()):
                if event is None:
                    continue
                occ = event

                # Skip expressions whose inputs are modified by the loop.
                if occ.variable_uses & loop_defs:
                    continue

                # Only suggest hoisting expressions executed every iteration.
                if any(not _dominates(ssa, occ.block, latch) for latch in latches):
                    continue

                command = occ.key[1] if len(occ.key) > 1 else ""
                if not _is_worth_reporting(command, interproc, cfg.name):
                    continue

                results.append(
                    RedundantComputation(
                        range=occ.range,
                        first_range=occ.range,
                        expression_text=occ.expression_text,
                        code="O106",
                        message=_loop_invariant_message(occ.expression_text),
                    )
                )

    return results


# GVN walk


def _gvn_walk_function(
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
    source: str,
    *,
    interproc: InterproceduralAnalysis | None = None,
) -> list[RedundantComputation]:
    """Walk one function in dominator-tree preorder, detecting CSE."""
    table = _ScopedValueTable()
    results: list[RedundantComputation] = []
    executable = set(cfg.blocks) - analysis.unreachable_blocks

    def visit(block_name: BlockName) -> None:
        if block_name not in executable:
            return

        table.push_scope()

        cfg_block = cfg.blocks[block_name]
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            table.pop_scope()
            return

        stmt_count = min(len(cfg_block.statements), len(ssa_block.statements))

        for idx in range(stmt_count):
            ir_stmt = cfg_block.statements[idx]
            ssa_stmt = ssa_block.statements[idx]

            # Mutators/barriers invalidate cached values.
            if _statement_writes_state(
                ir_stmt,
                source,
                interproc=interproc,
                caller_name=cfg.name,
            ):
                table.kill_all()
                continue

            occurrences = _statement_occurrences(
                ir_stmt,
                ssa_stmt.uses,
                source,
                block_name,
                idx,
                interproc=interproc,
                caller_name=cfg.name,
            )
            for occ in occurrences:
                existing = table.lookup(occ.key)
                if existing is not None:
                    results.append(
                        RedundantComputation(
                            range=occ.range,
                            first_range=existing.range,
                            expression_text=existing.expression_text,
                            message=_full_redundancy_message(existing.expression_text),
                        )
                    )
                    continue

                table.insert(
                    occ.key,
                    _ValueEntry(
                        key=occ.key,
                        block=occ.block,
                        statement_index=occ.statement_index,
                        range=occ.range,
                        expression_text=occ.expression_text,
                    ),
                )

        # Visit dominated children.
        for child in ssa.dominator_tree.get(block_name, ()):
            visit(child)

        table.pop_scope()

    if ssa.entry in cfg.blocks:
        visit(ssa.entry)

    return results


# Deduplication


def _deduplicate(warnings: list[RedundantComputation]) -> list[RedundantComputation]:
    """Remove duplicate warnings at the same source offset."""
    seen: set[int] = set()
    result: list[RedundantComputation] = []
    for w in warnings:
        offset = w.range.start.offset
        if offset in seen:
            continue
        seen.add(offset)
        result.append(w)
    return result


# iRules ``when`` body scanning (token-level flat scan)


def _collect_cmd_tokens_recursive(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
) -> list[Token]:
    """Collect all CMD tokens at all nesting levels.

    Braced strings (``{ ... }``) in Tcl prevent substitution at the
    lexer level, but commands like ``if``, ``while``, and ``foreach``
    evaluate their braced arguments at runtime.  We recursively enter
    STR tokens to find CMD tokens inside conditions and bodies.
    """
    result: list[Token] = []
    lexer = TclLexer(text, base_offset=base_offset, base_line=base_line, base_col=base_col)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.CMD:
            result.append(tok)
        elif tok.type is TokenType.STR:
            # STR token text has braces stripped.  Recurse into the
            # content to find CMD tokens inside braced conditions/bodies.
            result.extend(
                _collect_cmd_tokens_recursive(
                    tok.text,
                    base_offset=tok.start.offset + 1,
                    base_line=tok.start.line,
                    base_col=tok.start.character + 1,
                )
            )
    return result


def _record_hit(
    key: tuple[str, ...],
    tok_range: Range,
    cmd_name: str,
    cmd_args: tuple[str, ...],
    seen: dict[tuple[str, ...], tuple[Range, str]],
    results: list[RedundantComputation],
) -> None:
    """Record a pure call; append a warning if it's already been seen."""
    if key in seen:
        first_range, expr_text = seen[key]
        results.append(
            RedundantComputation(
                range=tok_range,
                first_range=first_range,
                expression_text=expr_text,
                message=_full_redundancy_message(expr_text),
            )
        )
    else:
        expr_text = _format_expression_text(cmd_name, cmd_args)
        seen[key] = (tok_range, expr_text)


def _shift_position(
    pos: SourcePosition,
    *,
    line_delta: int,
    col_delta: int,
    offset_delta: int,
) -> SourcePosition:
    return SourcePosition(
        line=pos.line + line_delta,
        character=(pos.character + col_delta) if pos.line == 0 else pos.character,
        offset=pos.offset + offset_delta,
    )


def _shift_range(
    rng: Range,
    *,
    line_delta: int,
    col_delta: int,
    offset_delta: int,
) -> Range:
    return Range(
        start=_shift_position(
            rng.start,
            line_delta=line_delta,
            col_delta=col_delta,
            offset_delta=offset_delta,
        ),
        end=_shift_position(
            rng.end,
            line_delta=line_delta,
            col_delta=col_delta,
            offset_delta=offset_delta,
        ),
    )


def _analyse_when_body_with_cfg(
    body_text: str,
    body_tok: Token,
) -> list[RedundantComputation]:
    """Analyse one ``when`` body as standalone Tcl for GVN/PRE/LICM."""
    from .compilation_unit import compile_source

    try:
        body_cu = compile_source(body_text)
    except Exception:
        log.debug("gvn: compilation failed for when-body analysis", exc_info=True)
        return []

    local_results: list[RedundantComputation] = []
    local_results.extend(
        _gvn_walk_function(
            body_cu.top_level.cfg,
            body_cu.top_level.ssa,
            body_cu.top_level.analysis,
            body_text,
            interproc=body_cu.interproc,
        )
    )
    local_results.extend(
        _find_partial_redundancies(
            body_cu.top_level.cfg,
            body_cu.top_level.ssa,
            body_cu.top_level.analysis,
            body_text,
            interproc=body_cu.interproc,
        )
    )
    local_results.extend(
        _find_loop_invariants(
            body_cu.top_level.cfg,
            body_cu.top_level.ssa,
            body_cu.top_level.analysis,
            body_text,
            interproc=body_cu.interproc,
        )
    )
    for fu in body_cu.procedures.values():
        local_results.extend(
            _gvn_walk_function(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                body_text,
                interproc=body_cu.interproc,
            )
        )
        local_results.extend(
            _find_partial_redundancies(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                body_text,
                interproc=body_cu.interproc,
            )
        )
        local_results.extend(
            _find_loop_invariants(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                body_text,
                interproc=body_cu.interproc,
            )
        )

    if not local_results:
        return []

    line_delta = body_tok.start.line
    col_delta = body_tok.start.character + 1
    offset_delta = body_tok.start.offset + 1

    return [
        RedundantComputation(
            range=_shift_range(
                w.range,
                line_delta=line_delta,
                col_delta=col_delta,
                offset_delta=offset_delta,
            ),
            first_range=_shift_range(
                w.first_range,
                line_delta=line_delta,
                col_delta=col_delta,
                offset_delta=offset_delta,
            ),
            expression_text=w.expression_text,
            code=w.code,
            message=w.message,
        )
        for w in local_results
    ]


def _scan_when_bodies(source: str) -> list[RedundantComputation]:
    """Analyse iRules ``when`` bodies for repeated pure command calls.

    ``when`` bodies bypass the parent document CFG/SSA pipeline (they are
    opaque string arguments to ``IRCall(command="when", ...)``), so we run:

    1. A body-local CFG/SSA GVN-PRE analysis (when parsing succeeds).
    2. A token-level flat scan as fallback/supplement.

    Detects two patterns:

    1. **Standalone commands** — the command name itself is an expensive
       getter (``HTTP::uri``, ``IP::client_addr``, …).  These appear as
       ESC/WORD tokens at the top-level of the body.  (Subsumes the
       former IRULE2102 check.)
    2. **Embedded command substitutions** — ``[HTTP::uri]`` or
       ``[string length $x]`` inside arguments.  These appear as CMD
       tokens, found recursively inside braced strings.

    Within each event body we track ``(cmd_name, args_text)`` keys.
    Because there is no SSA, variable-versioned canonicalisation is not
    possible; we simply use raw argument text.
    """
    results: list[RedundantComputation] = []

    def _writes_tracked_state(command: str, args: tuple[str, ...]) -> bool:
        _reads, writes = classify_side_effects(command, args).to_effect_regions()
        return writes != EffectRegion.NONE

    for _event, _priority, body_text, body_tok, _event_tok in _find_when_bodies(source):
        # Path-sensitive pass first so it wins dedup ties over flat-scan hits.
        results.extend(_analyse_when_body_with_cfg(body_text, body_tok))

        # Track command-call keys seen so far within this body.
        seen: dict[tuple[str, ...], tuple[Range, str]] = {}

        base_offset = body_tok.start.offset + 1
        base_line = body_tok.start.line
        base_col = body_tok.start.character + 1

        for cmd_name, cmd_tok, all_tokens in _walk_body_commands(
            body_text,
            base_offset=base_offset,
            base_line=base_line,
            base_col=base_col,
        ):
            top_args = tuple(t.text for t in all_tokens[1:])
            if cmd_name not in _CONTROL_FLOW_COMMANDS and _writes_tracked_state(cmd_name, top_args):
                seen.clear()

            # Pattern 1: standalone pure command at top level
            # e.g. bare `HTTP::uri` or `HTTP::header value Host`
            if _is_pure_command(cmd_name, top_args):
                args = top_args
                if _is_worth_reporting(cmd_name):
                    key = ("call", cmd_name, *args)
                    _record_hit(
                        key,
                        range_from_token(cmd_tok),
                        cmd_name,
                        args,
                        seen,
                        results,
                    )

            # Pattern 2: embedded CMD tokens in arguments
            # Recursively enter braced strings to find [cmd ...] at
            # any nesting level.
            for tok in all_tokens:
                if tok.type is TokenType.CMD:
                    parsed = _parse_cmd_token(tok.text)
                    if parsed is None:
                        continue
                    inner_cmd, inner_args = parsed
                    if _writes_tracked_state(inner_cmd, inner_args):
                        seen.clear()
                        continue
                    if not _is_pure_command(inner_cmd, inner_args):
                        continue
                    if not _is_worth_reporting(inner_cmd):
                        continue
                    key = ("call", inner_cmd, *inner_args)
                    _record_hit(
                        key,
                        range_from_token(tok),
                        inner_cmd,
                        inner_args,
                        seen,
                        results,
                    )
                elif tok.type is TokenType.STR:
                    # Braced string — recurse to find CMD tokens
                    # inside conditions, loop bodies, etc.
                    for inner_tok in _collect_cmd_tokens_recursive(
                        tok.text,
                        base_offset=tok.start.offset + 1,
                        base_line=tok.start.line,
                        base_col=tok.start.character + 1,
                    ):
                        parsed = _parse_cmd_token(inner_tok.text)
                        if parsed is None:
                            continue
                        inner_cmd, inner_args = parsed
                        if _writes_tracked_state(inner_cmd, inner_args):
                            seen.clear()
                            continue
                        if not _is_pure_command(inner_cmd, inner_args):
                            continue
                        if not _is_worth_reporting(inner_cmd):
                            continue
                        key = ("call", inner_cmd, *inner_args)
                        _record_hit(
                            key,
                            range_from_token(inner_tok),
                            inner_cmd,
                            inner_args,
                            seen,
                            results,
                        )

    return results


# Public API


def find_redundant_computations(
    source: str,
    *,
    cu: CompilationUnit | None = None,
) -> list[RedundantComputation]:
    """Find optimisation hints via GVN/CSE, PRE, and LICM-style checks.

    Follows the same pattern as ``find_shimmer_warnings`` and
    ``find_taint_warnings``.
    """
    cu = ensure_compilation_unit(source, cu, logger=log, context="gvn")
    if cu is None:
        return []

    results: list[RedundantComputation] = []
    results.extend(
        _gvn_walk_function(
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis,
            source,
            interproc=cu.interproc,
        )
    )
    results.extend(
        _find_partial_redundancies(
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis,
            source,
            interproc=cu.interproc,
        )
    )
    results.extend(
        _find_loop_invariants(
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis,
            source,
            interproc=cu.interproc,
        )
    )
    for fu in cu.procedures.values():
        results.extend(
            _gvn_walk_function(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                source,
                interproc=cu.interproc,
            )
        )
        results.extend(
            _find_partial_redundancies(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                source,
                interproc=cu.interproc,
            )
        )
        results.extend(
            _find_loop_invariants(
                fu.cfg,
                fu.ssa,
                fu.analysis,
                source,
                interproc=cu.interproc,
            )
        )

    # iRules ``when`` bodies bypass CFG/SSA -- scan them with a
    # flat token-level pass.
    if active_dialect() == "f5-irules":
        results.extend(_scan_when_bodies(source))

    return _deduplicate(results)
