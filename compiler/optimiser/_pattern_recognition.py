"""Pre-loop pattern recognition passes for the optimiser."""

# canonicalisation: audited #246

from __future__ import annotations

from compiler.parsing.tokens import Token, TokenType
from compiler.registry import REGISTRY
from compiler.registry.runtime import variable_writing_commands
from shared.codes import opt
from shared.dialect import active_dialect
from shared.naming import (
    normalise_var_name as _normalise_var_name,
)

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
    _command_subst_range,
    _expr_arg_from_expr_command,
    _full_command_range,
    _is_static_var_word,
    _parse_static_string_arg,
    _render_static_string_word,
    _tokens_for_statement,
    _try_end_offset_from_length_expr,
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
    """Return normalised variable names written by this command.

    Uses ``assigns_variable_at`` from the registry for single-variable
    commands, ``mutator_subcommands`` for dict, and structural dispatch
    for multi-variable commands (unset, global, variable, upvar).
    """
    if not argv_texts:
        return set()

    cmd_name = argv_texts[0]
    written: set[str] = set()

    # Use the registry to identify variable-writing commands.
    var_commands = variable_writing_commands()
    var_idx = var_commands.get(cmd_name)
    spec = REGISTRY.get_any(cmd_name)

    if var_idx is not None and not REGISTRY.is_destroys_variable(cmd_name):
        # Simple variable-writing command (set, append, lappend, incr, etc.)
        arg_pos = var_idx + 1  # +1 because argv_texts[0] is the command name
        if len(argv_texts) < arg_pos + 1:
            return written
        # `set varName` with only the variable name is a read, not a write.
        # Other variable-writing commands (incr, lappend) always write even
        # at their minimum arity, so this special case applies only to `set`.
        if cmd_name == "set" and len(argv_texts) == 2:
            return written
        if _is_static_var_word(
            argv_texts[arg_pos], argv_tokens[arg_pos], single_token=argv_single[arg_pos]
        ):
            written.add(_normalise_var_name(argv_texts[arg_pos]))
        return written

    if REGISTRY.is_destroys_variable(cmd_name):
        # unset-like: skips option flags, then all remaining args are variables
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

    elif cmd_name == "global" or cmd_name == "variable":
        step = 2 if cmd_name == "variable" else 1
        for idx in range(1, len(argv_texts), step):
            if idx >= len(argv_tokens) or idx >= len(argv_single):
                continue
            if _is_static_var_word(
                argv_texts[idx], argv_tokens[idx], single_token=argv_single[idx]
            ):
                written.add(_normalise_var_name(argv_texts[idx]))

    elif spec is not None and spec.creates_scope_alias:
        # upvar-like: skips level arg, then pairs (otherVar myVar)
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

    elif spec is not None and len(argv_texts) >= 2:
        # Check for subcommand-based variable mutation (dict set/unset/etc.)
        mutator_subs = REGISTRY.mutator_subcommands(cmd_name)
        sub = argv_texts[1]
        if mutator_subs and sub in mutator_subs and len(argv_texts) >= 3:
            sub_spec = spec.subcommands.get(sub)
            if sub_spec is not None and sub_spec.inferred_storage_type is not None:
                if _is_static_var_word(argv_texts[2], argv_tokens[2], single_token=argv_single[2]):
                    written.add(_normalise_var_name(argv_texts[2]))

    return written


def _statement_delete_rewrite_range(
    source: str,
    command_range,
    next_stmt_start: int | None,
):
    """Compute the range for deleting a statement including trailing whitespace."""
    from analyser.semantic_model import Range

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
    from analyser.semantic_model import Range

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


@opt(
    code="O104",
    description="Fold static string build chains into a single assignment.",
    opt_category="pattern",
)
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

            # TODO(kcs-tcl9-barrier-relaxation): IRUpFrame and
            # eval-shape IRBlock are scoped barriers — the body runs
            # inline in the (caller's / current) frame, so static-set
            # chaining could chase through with frame-aware analysis.
            # For now, treat conservatively as hard barriers so the
            # first-wave relaxation does not regress SCCP/DCE.
            from ..ir import IRBlock as _IRBlock
            from ..ir import IRUpFrame

            if isinstance(stmt, IRUpFrame) or (
                isinstance(stmt, _IRBlock)
                and stmt.source_tokens is not None
                and stmt.source_tokens.argv_texts
                and stmt.source_tokens.argv_texts[0] == "eval"
            ):
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


@opt(
    code="O114",
    description="Recognise `incr` idiom (`set x [expr {$x + N}]` → `incr x N`).",
    opt_category="readability",
)
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
            if isinstance(stmt, IRCall) and stmt.canonical_command != "::set":
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


@opt(
    code="O119",
    description="Pack consecutive `set` literals into `lassign`/`foreach`.",
    opt_category="pattern",
)
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
            else:
                # TODO(kcs-tcl9-barrier-relaxation): IRUpFrame and
                # eval-shape IRBlock are scoped barriers; treat them
                # the same here so set-packing cannot pack across
                # the relaxed boundary.
                from ..ir import IRBlock as _IRBlock
                from ..ir import IRUpFrame

                if isinstance(stmt, IRUpFrame) or (
                    isinstance(stmt, _IRBlock)
                    and stmt.source_tokens is not None
                    and stmt.source_tokens.argv_texts
                    and stmt.source_tokens.argv_texts[0] == "eval"
                ):
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


# O128: End-offset index rewrite

# (command-matcher) -> (index_arg_positions, container_arg_position, expected_kind)
# container and index positions are 0-based indices into argv_texts.


def _end_offset_command_shape(
    argv_texts: list[str],
) -> tuple[tuple[int, ...], int, str] | None:
    """Identify list/string commands that accept ``end``/``end-N`` index args.

    Returns ``(index_positions, container_position, expected_kind)`` or
    ``None`` when the command does not match a supported shape.

    ``linsert`` is intentionally excluded: ``linsert $L end x`` appends to
    the list, whereas ``linsert $L [expr {[llength $L] - 1}] x`` inserts
    before the final element. No general ``end``/``end-N`` rewrite
    preserves that semantics for the ``N == 1`` case, so the whole command
    is skipped rather than partially rewritten.

    ``lindex`` with multiple indices resolves each index against the
    *sub-list* produced by the previous step, so only the first index
    position (2) is safe to rewrite.
    """
    if not argv_texts:
        return None
    cmd = argv_texts[0]
    nargs = len(argv_texts)
    if cmd == "lindex" and nargs >= 3:
        # Only the first index is relative to the original list value;
        # later indices resolve against intermediate sub-lists.
        return ((2,), 1, "llength")
    if cmd == "lrange" and nargs == 4:
        # lrange list first last
        return ((2, 3), 1, "llength")
    if cmd == "lreplace" and nargs >= 4:
        # lreplace list first last ?element ...?
        return ((2, 3), 1, "llength")
    if cmd == "string" and nargs >= 2:
        sub = argv_texts[1]
        if sub == "index" and nargs == 4:
            # string index str charIndex
            return ((3,), 2, "strlen")
        if sub == "range" and nargs == 5:
            # string range str first last
            return ((3, 4), 2, "strlen")
        if sub == "replace" and nargs >= 5:
            # string replace str first last ?newString?
            return ((3, 4), 2, "strlen")
    return None


def _apply_end_offset_to_argv(
    ctx: PassContext,
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> None:
    """Emit O128 optimisations for index args in the given command words."""
    shape = _end_offset_command_shape(argv_texts)
    if shape is None:
        return
    index_positions, container_pos, expected_kind = shape

    if container_pos >= len(argv_texts) or container_pos >= len(argv_tokens):
        return
    if not argv_single[container_pos]:
        return
    container_tok = argv_tokens[container_pos]
    if container_tok.type is not TokenType.VAR:
        return
    # Compare full variable references (``${L}``, ``${a(1)}``, ``$={a(1)}``),
    # not normalised base names — otherwise ``$a(1)`` and ``$a(2)`` would be
    # treated as the same container and the rewrite would change semantics.
    container_repr = argv_texts[container_pos].strip()

    for pos in index_positions:
        if pos >= len(argv_texts) or pos >= len(argv_tokens):
            continue
        if not argv_single[pos]:
            continue
        idx_tok = argv_tokens[pos]
        if idx_tok.type is not TokenType.CMD:
            continue
        expr_arg = _expr_arg_from_expr_command(idx_tok.text)
        if expr_arg is None:
            continue
        match = _try_end_offset_from_length_expr(expr_arg)
        if match is None:
            continue
        kind, length_var_ref, offset = match
        if kind != expected_kind:
            continue
        if length_var_ref.strip() != container_repr:
            continue

        replacement = "end" if offset == 0 else f"end-{offset}"
        ctx.optimisations.append(
            Optimisation(
                code="O128",
                message="Use end-offset index instead of length arithmetic",
                range=_command_subst_range(idx_tok),
                replacement=replacement,
            )
        )


def _parse_cmd_token_contents(
    cmd_tok: Token,
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Parse the inside of a CMD substitution token with absolute positions.

    The inner text lives one character past the opening ``[``; passing the
    correct ``base_offset``/``base_line``/``base_col`` to the lexer keeps
    every re-parsed inner token's range pointing into the original source.
    """
    from compiler.parsing.lexer import TclLexer

    lexer = TclLexer(
        cmd_tok.text,
        base_offset=cmd_tok.start.offset + 1,
        base_line=cmd_tok.start.line,
        base_col=cmd_tok.start.character + 1,
    )
    argv_texts: list[str] = []
    argv_tokens: list[Token] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL
    saw_eol = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            if argv_texts:
                saw_eol = True
            prev_type = tok.type
            continue
        if saw_eol:
            return None

        from ..token_helpers import word_piece

        piece = word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += piece
                argv_single[-1] = False
            else:
                argv_texts.append(piece)
                argv_tokens.append(tok)
                argv_single.append(True)
        prev_type = tok.type

    if not argv_texts:
        return None
    return argv_texts, argv_tokens, argv_single


def _walk_nested_cmd_tokens(argv_tokens: list[Token], argv_single: list[bool]):
    """Yield each CMD token's argv appearing in *argv_tokens*, recursing into
    nested command substitutions while preserving absolute source positions.
    """
    for idx, tok in enumerate(argv_tokens):
        if idx >= len(argv_single) or not argv_single[idx]:
            continue
        if tok.type is not TokenType.CMD:
            continue
        inner = _parse_cmd_token_contents(tok)
        if inner is None:
            continue
        yield inner
        _inner_texts, inner_tokens, inner_single = inner
        yield from _walk_nested_cmd_tokens(list(inner_tokens), list(inner_single))


@opt(
    code="O128",
    description=(
        "Rewrite `[expr {[llength $L] - N}]` / `[expr {[string length $s] - N}]` "
        "to `end-(N-1)` when used as an index argument."
    ),
    opt_category="readability",
)
def optimise_end_offset_indexes(ctx: PassContext, cfg, ssa) -> None:
    """O128: Use ``end``/``end-N`` instead of length arithmetic for index args."""
    for _block_name, block in cfg.blocks.items():
        for stmt in block.statements:
            parsed = _tokens_for_statement(stmt, ctx.source)
            if parsed is None:
                continue
            argv_texts, argv_tokens, argv_single = parsed
            _apply_end_offset_to_argv(ctx, argv_texts, list(argv_tokens), list(argv_single))
            for inner in _walk_nested_cmd_tokens(list(argv_tokens), list(argv_single)):
                inner_texts, inner_tokens, inner_single = inner
                _apply_end_offset_to_argv(
                    ctx,
                    list(inner_texts),
                    list(inner_tokens),
                    list(inner_single),
                )
