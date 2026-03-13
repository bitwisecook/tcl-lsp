"""Pre-loop pattern recognition passes for the optimiser."""

from __future__ import annotations

from ...common.dialect import active_dialect
from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...parsing.tokens import Token
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRBarrier,
    IRCall,
)
from ._helpers import (
    _DYNAMIC_BARRIER_COMMANDS,
    _SAFE_WORD_RE,
    _STATIC_VAR_WORD_RE,
    _full_command_range,
    _is_static_var_word,
    _parse_static_string_arg,
    _render_static_string_word,
    _tokens_for_statement,
    _try_incr_idiom,
)
from ._types import Optimisation, PassContext, _StringWriteChain


def _set_read_var(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> str | None:
    if len(argv_texts) != 2 or argv_texts[0] != "set":
        return None
    if not _is_static_var_word(argv_texts[1], argv_tokens[1], single_token=argv_single[1]):
        return None
    return _normalise_var_name(argv_texts[1])


def _append_read_var(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> str | None:
    if len(argv_texts) != 2 or argv_texts[0] != "append":
        return None
    if not _is_static_var_word(argv_texts[1], argv_tokens[1], single_token=argv_single[1]):
        return None
    return _normalise_var_name(argv_texts[1])


def _static_set_write(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> tuple[str, str, str] | None:
    if len(argv_texts) != 3 or argv_texts[0] != "set":
        return None
    if not _is_static_var_word(argv_texts[1], argv_tokens[1], single_token=argv_single[1]):
        return None
    value = _parse_static_string_arg(argv_texts[2], argv_tokens[2], single_token=argv_single[2])
    if value is None:
        return None
    return _normalise_var_name(argv_texts[1]), argv_texts[1], value


def _static_append_write(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> tuple[str, str, str] | None:
    if len(argv_texts) < 3 or argv_texts[0] != "append":
        return None
    if not _is_static_var_word(argv_texts[1], argv_tokens[1], single_token=argv_single[1]):
        return None
    pieces: list[str] = []
    for idx in range(2, len(argv_texts)):
        piece = _parse_static_string_arg(
            argv_texts[idx],
            argv_tokens[idx],
            single_token=argv_single[idx],
        )
        if piece is None:
            return None
        pieces.append(piece)
    return _normalise_var_name(argv_texts[1]), argv_texts[1], "".join(pieces)


def _written_var_keys(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> set[str]:
    """Return normalised variable names written by this command."""
    if not argv_texts:
        return set()

    cmd_name = argv_texts[0]
    written: set[str] = set()

    match cmd_name:
        case "set" | "append" | "lappend" | "incr":
            if len(argv_texts) < 2:
                return written
            if cmd_name in ("set", "append") and len(argv_texts) == 2:
                return written
            if _is_static_var_word(argv_texts[1], argv_tokens[1], single_token=argv_single[1]):
                written.add(_normalise_var_name(argv_texts[1]))

        case "unset":
            i = 1
            while i < len(argv_texts) and argv_texts[i].startswith("-"):
                if argv_texts[i] == "--":
                    i += 1
                    break
                i += 1
            for idx in range(i, len(argv_texts)):
                if idx >= len(argv_tokens) or idx >= len(argv_single):
                    continue
                if _is_static_var_word(
                    argv_texts[idx], argv_tokens[idx], single_token=argv_single[idx]
                ):
                    written.add(_normalise_var_name(argv_texts[idx]))

        case "global" | "variable":
            step = 2 if cmd_name == "variable" else 1
            for idx in range(1, len(argv_texts), step):
                if idx >= len(argv_tokens) or idx >= len(argv_single):
                    continue
                if _is_static_var_word(
                    argv_texts[idx], argv_tokens[idx], single_token=argv_single[idx]
                ):
                    written.add(_normalise_var_name(argv_texts[idx]))

        case "upvar":
            start = 1
            if len(argv_texts) > 1 and argv_texts[1].lstrip("-").isdigit():
                start = 2
            for idx in range(start + 1, len(argv_texts), 2):
                if idx >= len(argv_tokens) or idx >= len(argv_single):
                    continue
                if _is_static_var_word(
                    argv_texts[idx], argv_tokens[idx], single_token=argv_single[idx]
                ):
                    written.add(_normalise_var_name(argv_texts[idx]))

        case "dict" if len(argv_texts) >= 2:
            sub = argv_texts[1]
            if sub in ("set", "unset", "append", "lappend", "incr") and len(argv_texts) >= 3:
                if _is_static_var_word(argv_texts[2], argv_tokens[2], single_token=argv_single[2]):
                    written.add(_normalise_var_name(argv_texts[2]))

    return written


def _statement_delete_rewrite_range(
    source: str,
    command_range,
    next_stmt_start: int | None,
):
    """Compute the range for deleting a statement including trailing whitespace."""
    from ...analysis.semantic_model import Range
    from ._helpers import _advance_position

    if next_stmt_start is None:
        return command_range

    start = command_range.start.offset
    end_offset = command_range.end.offset
    if next_stmt_start <= end_offset + 1 or next_stmt_start > len(source):
        return command_range

    cursor = end_offset + 1
    while cursor < next_stmt_start and source[cursor] in " \t\r":
        cursor += 1
    if cursor < next_stmt_start and source[cursor] in "\n;":
        cursor += 1
        while cursor < next_stmt_start and source[cursor] in " \t\r":
            cursor += 1
        end_offset = cursor - 1

    if end_offset <= command_range.end.offset:
        return command_range
    end_pos = _advance_position(command_range.start, source[start : end_offset + 1])
    return Range(start=command_range.start, end=end_pos)


def _statement_rewrite_context(
    source: str,
    cfg,
):
    """Build rewrite context maps for statement ranges and next-statement offsets."""
    from ...analysis.semantic_model import Range

    entries: list[tuple[str, int, Range]] = []
    for block_name, block in cfg.blocks.items():
        for idx, stmt in enumerate(block.statements):
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            full_range = _full_command_range(source, stmt_range) or stmt_range
            entries.append((block_name, idx, full_range))

    entries.sort(key=lambda item: item[2].start.offset)

    range_by_stmt: dict[tuple[str, int], Range] = {}
    next_start_by_stmt: dict[tuple[str, int], int | None] = {}
    for i, (block_name, idx, stmt_range) in enumerate(entries):
        key = (block_name, idx)
        range_by_stmt[key] = stmt_range
        if i + 1 < len(entries):
            next_start_by_stmt[key] = entries[i + 1][2].start.offset
        else:
            next_start_by_stmt[key] = None
    return range_by_stmt, next_start_by_stmt


def optimise_string_build_chains(ctx: PassContext, cfg, ssa) -> None:
    source = ctx.source
    for block_name, block in cfg.blocks.items():
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            continue

        stmt_count = min(len(block.statements), len(ssa_block.statements))
        parsed_by_stmt: list[tuple[list[str], list[Token], list[bool]] | None] = []
        full_ranges: list = []
        stmt_start_offsets: list[int | None] = []
        for idx in range(stmt_count):
            stmt = block.statements[idx]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                parsed_by_stmt.append(None)
                full_ranges.append(None)
                stmt_start_offsets.append(None)
                continue
            parsed_by_stmt.append(_tokens_for_statement(stmt, source))
            full_ranges.append(_full_command_range(source, stmt_range))
            stmt_start_offsets.append(stmt_range.start.offset)

        active: dict[str, _StringWriteChain] = {}

        def finish_chain(var_key: str) -> None:
            chain = active.pop(var_key, None)
            if chain is None or len(chain.writes) < 2:
                return
            rendered = _render_static_string_word(chain.value)
            if rendered is None:
                return
            last_idx = chain.writes[-1]
            last_range = full_ranges[last_idx]
            if last_range is None:
                return
            ctx.optimisations.append(
                Optimisation(
                    code="O104",
                    message="Fold write-only string build chain",
                    range=last_range,
                    replacement=f"set {chain.var_word} {rendered}",
                )
            )
            for dead_idx in chain.writes[:-1]:
                dead_range = full_ranges[dead_idx]
                if dead_range is None:
                    continue
                next_start = stmt_start_offsets[dead_idx + 1] if dead_idx + 1 < stmt_count else None
                dead_rewrite_range = _statement_delete_rewrite_range(
                    source,
                    dead_range,
                    next_start,
                )
                ctx.optimisations.append(
                    Optimisation(
                        code="O104",
                        message="Remove dead intermediate string write",
                        range=dead_rewrite_range,
                        replacement="",
                    )
                )

        for idx in range(stmt_count):
            stmt = block.statements[idx]
            parsed = parsed_by_stmt[idx]
            if parsed is None:
                for var_key in list(active):
                    finish_chain(var_key)
                continue

            argv_texts, argv_tokens, argv_single = parsed
            if not argv_texts:
                for var_key in list(active):
                    finish_chain(var_key)
                continue

            if isinstance(stmt, IRBarrier):
                for var_key in list(active):
                    finish_chain(var_key)
                continue

            cmd_name = argv_texts[0]
            if cmd_name in _DYNAMIC_BARRIER_COMMANDS:
                for var_key in list(active):
                    finish_chain(var_key)
                continue

            ssa_uses = ssa_block.statements[idx].uses
            own_defs = ssa_block.statements[idx].defs
            read_vars = set(ssa_uses.keys()) - set(own_defs.keys())
            set_read = _set_read_var(argv_texts, argv_tokens, argv_single)
            if set_read is not None:
                read_vars.add(set_read)
            append_read = _append_read_var(argv_texts, argv_tokens, argv_single)
            if append_read is not None:
                read_vars.add(append_read)
            for var_key in list(active):
                if var_key in read_vars:
                    finish_chain(var_key)

            static_set = _static_set_write(argv_texts, argv_tokens, argv_single)
            if static_set is not None:
                var_key, var_word, value = static_set
                chain = active.get(var_key)
                if chain is None:
                    active[var_key] = _StringWriteChain(
                        var_word=var_word,
                        writes=[idx],
                        value=value,
                    )
                else:
                    chain.var_word = var_word
                    chain.writes.append(idx)
                    chain.value = value
                continue

            static_append = _static_append_write(argv_texts, argv_tokens, argv_single)
            if static_append is not None:
                var_key, var_word, append_value = static_append
                chain = active.get(var_key)
                if chain is not None:
                    chain.var_word = var_word
                    chain.writes.append(idx)
                    chain.value += append_value
                continue

            for written in _written_var_keys(argv_texts, argv_tokens, argv_single):
                finish_chain(written)

        for var_key in list(active):
            finish_chain(var_key)


def optimise_incr_idioms(ctx: PassContext, cfg, ssa) -> None:
    """O114: Recognise ``set x [expr {$x + N}]`` -> ``incr x N``."""
    source = ctx.source
    for block_name, block in cfg.blocks.items():
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            continue

        stmt_count = min(len(block.statements), len(ssa_block.statements))
        for idx in range(stmt_count):
            stmt = block.statements[idx]
            if not isinstance(stmt, (IRAssignExpr, IRCall)):
                continue
            if isinstance(stmt, IRCall) and stmt.command != "set":
                continue
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue

            parsed = _tokens_for_statement(stmt, source)
            if parsed is None:
                continue
            argv_texts, argv_tokens, argv_single = parsed
            if not argv_texts:
                continue

            replacement = _try_incr_idiom(argv_texts, argv_tokens, argv_single)
            if replacement is None:
                continue

            full_range = _full_command_range(source, stmt_range)
            if full_range is None:
                continue
            ctx.optimisations.append(
                Optimisation(
                    code="O114",
                    message="Use incr instead of set/expr",
                    range=full_range,
                    replacement=replacement,
                )
            )


# O119: Multi-set packing
_SET_PACK_MIN_GROUP = 3  # minimum candidates for packing


def optimise_multi_set_packing(ctx: PassContext, cfg, ssa) -> None:
    """O119: Pack interspersed ``set var literal`` into ``lassign``/``foreach``."""
    source = ctx.source
    dialect = active_dialect()
    # In Tcl 9.0 individual set commands are faster than lassign/foreach
    if dialect == "tcl9.0":
        return
    use_lassign = dialect in ("tcl8.5", "tcl8.6")
    range_by_stmt, next_start_by_stmt = _statement_rewrite_context(source, cfg)

    for block_name, block in cfg.blocks.items():
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            continue

        stmt_count = min(len(block.statements), len(ssa_block.statements))
        if stmt_count < _SET_PACK_MIN_GROUP:
            continue

        # Phase 1: Identify candidates and barriers.
        candidates: list[tuple[int, str, str, str]] = []
        barrier_indices: set[int] = set()

        _stmt_brace_depth: dict[int, int] = {}
        if block.statements:
            first_range = block.statements[0].range
            if first_range is not None:
                base_offset = first_range.start.offset
                depth = 0
                depth_at: dict[int, int] = {}
                for ci, ch in enumerate(source[base_offset:], start=base_offset):
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                    depth_at[ci] = depth
                for si in range(stmt_count):
                    sr = block.statements[si].range
                    if sr is not None:
                        _stmt_brace_depth[si] = depth_at.get(sr.start.offset, 0)

        for idx in range(stmt_count):
            stmt = block.statements[idx]
            if isinstance(stmt, IRAssignConst):
                var_key = _normalise_var_name(stmt.name)
                if var_key in ctx.cross_event_vars:
                    continue
                if not _STATIC_VAR_WORD_RE.fullmatch(stmt.name):
                    continue
                if not _SAFE_WORD_RE.fullmatch(stmt.value):
                    continue
                candidates.append((idx, var_key, stmt.name, stmt.value))
            elif isinstance(stmt, IRBarrier):
                barrier_indices.add(idx)
            elif isinstance(stmt, IRCall) and stmt.command in _DYNAMIC_BARRIER_COMMANDS:
                barrier_indices.add(idx)

        if len(candidates) < _SET_PACK_MIN_GROUP:
            continue

        # Phase 2: Build read-after-write constraints.
        var_earliest_read: dict[str, int] = {}
        for c_idx, var_key, _vw, _val in candidates:
            if var_key in var_earliest_read and var_earliest_read[var_key] <= c_idx:
                continue
            for scan_idx in range(c_idx + 1, stmt_count):
                if scan_idx >= len(ssa_block.statements):
                    break
                scan_uses = ssa_block.statements[scan_idx].uses
                if var_key in scan_uses:
                    old = var_earliest_read.get(var_key)
                    if old is None or scan_idx < old:
                        var_earliest_read[var_key] = scan_idx
                    break

        # Phase 3: Greedy grouping with reordering.
        groups: list[list[tuple[int, str, str, str]]] = []
        current_group: list[tuple[int, str, str, str]] = []
        seen_vars: dict[str, int] = {}

        def _has_barrier_between(a: int, b: int) -> bool:
            return any(bi for bi in barrier_indices if a < bi < b)

        def _finalise_group() -> None:
            if len(current_group) >= _SET_PACK_MIN_GROUP:
                deduped: dict[str, tuple[int, str, str, str]] = {}
                for entry in current_group:
                    deduped[entry[1]] = entry
                final = sorted(deduped.values(), key=lambda e: e[0])
                if len(final) >= _SET_PACK_MIN_GROUP:
                    groups.append(final)

        for cand in candidates:
            c_idx, c_var, c_word, c_val = cand

            can_extend = True

            if current_group:
                prev_depth = _stmt_brace_depth.get(current_group[0][0], 0)
                curr_depth = _stmt_brace_depth.get(c_idx, 0)
                if curr_depth != prev_depth:
                    can_extend = False

            if can_extend:
                for prev in current_group:
                    prev_idx = prev[0]
                    prev_var = prev[1]
                    if _has_barrier_between(prev_idx, c_idx):
                        can_extend = False
                        break
                    earliest = var_earliest_read.get(prev_var)
                    if earliest is not None and earliest <= c_idx:
                        can_extend = False
                        break

            if not can_extend:
                _finalise_group()
                current_group = []
                seen_vars = {}

            if c_var in seen_vars:
                old_pos = seen_vars[c_var]
                current_group = [e for i, e in enumerate(current_group) if i != old_pos]
                seen_vars = {e[1]: i for i, e in enumerate(current_group)}

            seen_vars[c_var] = len(current_group)
            current_group.append(cand)

        _finalise_group()

        # Phase 4: Emit optimisations.
        for group in groups:
            group_id = ctx.alloc_group()

            vars_list = [e[2] for e in group]
            vals_list = [e[3] for e in group]
            vars_joined = " ".join(vars_list)
            vals_joined = " ".join(vals_list)

            if use_lassign:
                replacement = f"lassign {{{vals_joined}}} {vars_joined}"
            else:
                replacement = f"foreach {{{vars_joined}}} {{{vals_joined}}} {{break}}"

            last_idx = group[-1][0]
            last_key = (block_name, last_idx)
            last_range = range_by_stmt.get(last_key)
            if last_range is None:
                continue

            ctx.optimisations.append(
                Optimisation(
                    code="O119",
                    message="Pack set statements into lassign"
                    if use_lassign
                    else "Pack set statements into foreach",
                    range=last_range,
                    replacement=replacement,
                    group=group_id,
                )
            )

            for entry in group[:-1]:
                entry_idx = entry[0]
                entry_key = (block_name, entry_idx)
                entry_range = range_by_stmt.get(entry_key)
                if entry_range is None:
                    continue
                next_start = next_start_by_stmt.get(entry_key)
                delete_range = _statement_delete_rewrite_range(source, entry_range, next_start)
                ctx.optimisations.append(
                    Optimisation(
                        code="O119",
                        message="Remove packed set (moved to lassign)"
                        if use_lassign
                        else "Remove packed set (moved to foreach)",
                        range=delete_range,
                        replacement="",
                        group=group_id,
                    )
                )
