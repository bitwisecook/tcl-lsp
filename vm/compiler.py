"""Compilation pipeline wrapper: source -> ModuleAsm.

Orchestrates the full compiler suite from ``core/`` to produce
bytecode that the VM can execute.
"""

from __future__ import annotations

from core.analysis.semantic_model import Range
from core.compiler.cfg import CFGModule, build_cfg_function
from core.compiler.codegen import ModuleAsm, codegen_module
from core.compiler.ir import (
    IRAssignConst,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from core.compiler.lowering import lower_to_ir
from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType

from .substitution import backslash_subst

# Literal processing for VM execution


# Sentinel marker: _process_literals wraps braced string values with this
# prefix/suffix instead of raw ``{…}`` to avoid ambiguity with literal
# strings that happen to contain balanced braces (e.g. ``{*}``).
_BRACE_OPEN = "\x00\x01{"
_BRACE_CLOSE = "}\x01\x00"

# Sentinel marker for literals that must be pushed as-is with no
# brace-stripping or runtime substitution (e.g. interpolation template
# parts that happen to contain braces or ``$``).
_RAW_PREFIX = "\x00\x02"


def _process_literals(source: str, ir_module: IRModule) -> IRModule:
    """Process backslash escapes and preserve brace context in IR args.

    Walks IRCall / IRBarrier nodes that still have ``CommandTokens``:

    * **ESC** single-token words: apply ``backslash_subst()`` so the
      literal pool contains the evaluated escape sequence.
    * **STR** single-token words: wrap in ``{…}`` so the PUSH handler
      knows to strip braces without triggering substitution.

    Also handles ``IRAssignValue`` / ``IRAssignConst`` nodes (which lack
    tokens) by re-lexing the source to determine quoting context.

    Must run *before* ``_desugar_for_vm`` because desugaring replaces
    IRForeach/IRCatch/IRTry/IRSwitch with tokenless IRBarrier nodes.
    """
    new_top = _process_script_literals(source, ir_module.top_level)
    new_procs: dict[str, IRProcedure] = {}
    procs_changed = False
    for qname, proc in ir_module.procedures.items():
        new_body = _process_script_literals(source, proc.body)
        if new_body is not proc.body:
            new_procs[qname] = IRProcedure(
                name=proc.name,
                qualified_name=proc.qualified_name,
                params=proc.params,
                range=proc.range,
                body=new_body,
                params_raw=proc.params_raw,
                body_source=proc.body_source,
            )
            procs_changed = True
        else:
            new_procs[qname] = proc
    if new_top is not ir_module.top_level or procs_changed:
        return IRModule(top_level=new_top, procedures=new_procs)
    return ir_module


def _process_script_literals(source: str, script: IRScript) -> IRScript:
    """Walk an IR script and process literals in call/barrier nodes."""
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        new_stmt = _process_stmt_literals(source, stmt)
        if new_stmt is not stmt:
            changed = True
        new_stmts.append(new_stmt)
    if changed:
        return IRScript(statements=tuple(new_stmts))
    return script


def _value_token_type(source: str, stmt: IRAssignValue | IRAssignConst) -> TokenType | None:
    """Determine the quoting context of the value word in an assignment.

    Re-lexes the source span covered by *stmt* to find the last token
    (the value argument) and returns its ``TokenType``.
    """
    start = stmt.range.start.offset
    end = stmt.range.end.offset
    cmd_text = source[start : end + 1]
    # Disable strict_quoting during re-lexing — this is an internal
    # token-type check, not user-facing syntax validation.
    old_strict = TclLexer.strict_quoting
    TclLexer.strict_quoting = False
    try:
        lexer = TclLexer(cmd_text)
        last_type: TokenType | None = None
        for tok in lexer.tokenise_all():
            if tok.type is TokenType.EOF:
                break
            if tok.type not in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
                last_type = tok.type
        return last_type
    finally:
        TclLexer.strict_quoting = old_strict


def _process_stmt_literals(source: str, stmt: IRStatement) -> IRStatement:
    """Process backslash escapes and brace context in a single statement.

    * **ESC** single-token words with ``\\``: apply ``backslash_subst()``
      so the literal pool contains the evaluated escape sequence.
    * **STR** single-token words: wrap in ``{…}`` so the PUSH handler
      strips braces without triggering substitution.
    """
    # Handle IRAssignValue / IRAssignConst — these lack tokens so we
    # re-lex the source to determine quoting.
    if isinstance(stmt, (IRAssignValue, IRAssignConst)):
        value = stmt.value
        tok_type = _value_token_type(source, stmt)
        new_value = value
        changed = False
        if tok_type is TokenType.ESC and "\\" in value:
            new_value = backslash_subst(value)
            # Mark with _RAW_PREFIX if result has literal $ or [ from escapes
            if new_value != value:
                if "$" in new_value or ("[" in new_value and "]" in new_value):
                    new_value = _RAW_PREFIX + new_value
            changed = new_value != value
        elif tok_type is TokenType.STR:
            # Braced value — wrap with sentinel so PUSH suppresses substitution.
            # Guard against the PUSH handler's raw {…} fallback stripping
            # an extra layer of braces from values that happen to be brace-balanced.
            if (
                ("[" in value and "]" in value)
                or "$" in value
                or (value.startswith("{") and value.endswith("}"))
            ):
                new_value = _BRACE_OPEN + value + _BRACE_CLOSE
                changed = True
        if changed:
            if isinstance(stmt, IRAssignValue):
                return IRAssignValue(range=stmt.range, name=stmt.name, value=new_value)
            return IRAssignConst(range=stmt.range, name=stmt.name, value=new_value)
        return stmt

    # Process IRCall and IRBarrier nodes with tokens
    if isinstance(stmt, (IRCall, IRBarrier)) and stmt.tokens is not None:
        tokens = stmt.tokens
        new_args = list(stmt.args)
        args_changed = False

        for i, arg in enumerate(new_args):
            word_idx = i + 1  # argv[0] is the command name
            if word_idx >= len(tokens.single_token_word):
                break
            if not tokens.single_token_word[word_idx]:
                continue  # compound words — handled by substitute() at PUSH time

            tok = tokens.argv[word_idx]
            if tok.type is TokenType.ESC and "\\" in arg:
                processed = backslash_subst(arg)
                # After backslash substitution, any remaining $ or [ are
                # literal (they came from \$ or \[).  Mark with _RAW_PREFIX
                # so the PUSH handler doesn't re-substitute them.
                if "$" in processed or ("[" in processed and "]" in processed):
                    new_args[i] = _RAW_PREFIX + processed
                else:
                    new_args[i] = processed
                args_changed = True
            elif tok.type is TokenType.STR:
                new_args[i] = _BRACE_OPEN + arg + _BRACE_CLOSE
                args_changed = True

        if args_changed:
            if isinstance(stmt, IRCall):
                return IRCall(
                    range=stmt.range,
                    command=stmt.command,
                    args=tuple(new_args),
                    defs=stmt.defs,
                    reads=stmt.reads,
                    reads_own_defs=stmt.reads_own_defs,
                    tokens=stmt.tokens,
                )
            return IRBarrier(
                range=stmt.range,
                reason=stmt.reason,
                command=stmt.command,
                args=tuple(new_args),
                tokens=stmt.tokens,
            )

    # Recurse into nested scripts
    if isinstance(stmt, IRIf):
        new_clauses: list[IRIfClause] = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _process_script_literals(source, clause.body)
            if new_body is not clause.body:
                new_clauses.append(
                    IRIfClause(
                        condition=clause.condition,
                        condition_range=clause.condition_range,
                        body=new_body,
                        body_range=clause.body_range,
                    )
                )
                clauses_changed = True
            else:
                new_clauses.append(clause)
        new_else = stmt.else_body
        if stmt.else_body is not None:
            new_else = _process_script_literals(source, stmt.else_body)
            if new_else is not stmt.else_body:
                clauses_changed = True
        if clauses_changed:
            return IRIf(
                range=stmt.range,
                clauses=tuple(new_clauses),
                else_body=new_else,
                else_range=stmt.else_range,
            )

    if isinstance(stmt, IRFor):
        new_init = _process_script_literals(source, stmt.init)
        new_next = _process_script_literals(source, stmt.next)
        new_body = _process_script_literals(source, stmt.body)
        if new_init is not stmt.init or new_next is not stmt.next or new_body is not stmt.body:
            return IRFor(
                range=stmt.range,
                init=new_init,
                init_range=stmt.init_range,
                condition=stmt.condition,
                condition_range=stmt.condition_range,
                next=new_next,
                next_range=stmt.next_range,
                body=new_body,
                body_range=stmt.body_range,
            )

    if isinstance(stmt, IRWhile):
        new_body = _process_script_literals(source, stmt.body)
        if new_body is not stmt.body:
            return IRWhile(
                range=stmt.range,
                condition=stmt.condition,
                condition_range=stmt.condition_range,
                body=new_body,
                body_range=stmt.body_range,
            )

    return stmt


def compile_script(
    source: str,
    *,
    optimise: bool = False,
) -> tuple[ModuleAsm, IRModule]:
    """Compile Tcl *source* to bytecode.

    Returns ``(ModuleAsm, IRModule)`` — the bytecode assembly for
    execution and the IR module (for procedure metadata / parameter names).

    When *optimise* is True, SSA construction and optimisation passes
    are applied before codegen.  When False the bytecode matches
    Tcl 9.0's ``tcl::unsupported::disassemble`` output.
    """
    ir_module = lower_to_ir(source)

    # Process literals: evaluate backslash escapes in double-quoted words
    # and preserve brace context for braced words.  This must run before
    # desugaring so that original IRCall/IRBarrier nodes (with tokens)
    # are still present.
    ir_module = _process_literals(source, ir_module)

    # Replace IRForeach with IRBarrier so foreach runs as a command
    # (the CFG builder's foreach lowering produces placeholder bytecode
    # that doesn't work at runtime).
    ir_module = _desugar_for_vm(source, ir_module)

    top_cfg = build_cfg_function("::top", ir_module.top_level)

    proc_cfgs: dict[str, object] = {}
    for qname, proc in ir_module.procedures.items():
        proc_cfgs[qname] = build_cfg_function(qname, proc.body)

    cfg_module = CFGModule(top_level=top_cfg, procedures=proc_cfgs)  # type: ignore[arg-type]

    if optimise:
        # TODO: insert SSA + optimiser passes here
        pass

    module_asm = codegen_module(cfg_module, ir_module)
    return module_asm, ir_module


# IR desugaring for VM execution


def _desugar_for_vm(source: str, ir_module: IRModule) -> IRModule:
    """Convert IR nodes with placeholder bytecode to command-level barriers.

    Also re-inserts procedure definitions as runtime ``proc`` calls so that
    multi-command scripts like ``concat abc; proc foo {} {}`` return the
    correct result (the last command's result, not the first).
    """
    new_top = _reinsert_proc_defs(source, ir_module.top_level, ir_module.procedures)
    new_top = _desugar_script(source, new_top)
    new_procs: dict[str, IRProcedure] = {}
    procs_changed = False
    for qname, proc in ir_module.procedures.items():
        new_body = _desugar_script(source, proc.body)
        if new_body is not proc.body:
            new_procs[qname] = IRProcedure(
                name=proc.name,
                qualified_name=proc.qualified_name,
                params=proc.params,
                range=proc.range,
                body=new_body,
                params_raw=proc.params_raw,
                body_source=proc.body_source,
            )
            procs_changed = True
        else:
            new_procs[qname] = proc
    if new_top is not ir_module.top_level or procs_changed:
        return IRModule(top_level=new_top, procedures=new_procs)
    return ir_module


def _reinsert_proc_defs(
    source: str,
    script: IRScript,
    procedures: dict[str, IRProcedure],
) -> IRScript:
    """Re-insert procedure definitions as runtime ``proc`` barriers.

    The lowering phase extracts ``proc`` definitions into
    ``ir_module.procedures`` and removes them from the statement list.
    This is correct for static analysis, but the VM needs them as actual
    commands so that multi-statement scripts return the last command's
    result and proc definition side-effects happen at the right time.

    Only re-inserts procs that were defined at the script's own level
    (not inside nested bodies like namespace eval, if, etc.).
    """
    if not procedures:
        return script

    # Build ranges of existing top-level statements to determine
    # which procs are "top-level" (not inside another statement's body)
    stmt_ranges: list[tuple[int, int]] = []
    for stmt in script.statements:
        s = stmt.range.start.offset
        e = stmt.range.end.offset
        stmt_ranges.append((s, e))

    # Build barriers for each procedure, keyed by source offset
    proc_barriers: list[tuple[int, IRBarrier]] = []
    for _qname, proc in procedures.items():
        proc_offset = proc.range.start.offset

        # Skip procs that are inside an existing statement's range
        # (e.g. procs inside namespace eval bodies — the runtime
        # namespace command will handle defining them)
        inside = False
        for s, e in stmt_ranges:
            if s < proc_offset <= e:
                inside = True
                break
        if inside:
            continue

        body_text = proc.body_source if proc.body_source else ""
        params_raw = proc.params_raw if proc.params_raw else ""
        barrier = IRBarrier(
            range=proc.range,
            reason="proc runtime",
            command="proc",
            args=(
                proc.name,
                _BRACE_OPEN + params_raw + _BRACE_CLOSE,
                _BRACE_OPEN + body_text + _BRACE_CLOSE,
            ),
        )
        proc_barriers.append((proc_offset, barrier))

    if not proc_barriers:
        return script

    proc_barriers.sort(key=lambda x: x[0])

    # Merge proc barriers into the statement list by source offset
    stmts = list(script.statements)
    merged: list[IRStatement] = []
    proc_idx = 0

    for stmt in stmts:
        stmt_offset = stmt.range.start.offset
        # Insert any proc barriers that come before this statement
        while proc_idx < len(proc_barriers) and proc_barriers[proc_idx][0] < stmt_offset:
            merged.append(proc_barriers[proc_idx][1])
            proc_idx += 1
        merged.append(stmt)

    # Append any remaining proc barriers (procs that come after all statements)
    while proc_idx < len(proc_barriers):
        merged.append(proc_barriers[proc_idx][1])
        proc_idx += 1

    return IRScript(statements=tuple(merged))


def _desugar_script(source: str, script: IRScript) -> IRScript:
    """Walk an IR script and replace foreach with barriers."""
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        new_stmt = _desugar_stmt(source, stmt)
        if new_stmt is not stmt:
            changed = True
        new_stmts.append(new_stmt)
    if changed:
        return IRScript(statements=tuple(new_stmts))
    return script


def _desugar_stmt(source: str, stmt: IRStatement) -> IRStatement:
    """Replace IRForeach/IRCatch with IRBarrier; recurse into nested scripts."""
    if isinstance(stmt, IRForeach):
        return _foreach_to_barrier(source, stmt)

    if isinstance(stmt, IRCatch):
        return _catch_to_barrier(source, stmt)

    if isinstance(stmt, IRIf):
        new_clauses: list[IRIfClause] = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _desugar_script(source, clause.body)
            if new_body is not clause.body:
                new_clauses.append(
                    IRIfClause(
                        condition=clause.condition,
                        condition_range=clause.condition_range,
                        body=new_body,
                        body_range=clause.body_range,
                    )
                )
                clauses_changed = True
            else:
                new_clauses.append(clause)
        new_else = stmt.else_body
        if stmt.else_body is not None:
            new_else = _desugar_script(source, stmt.else_body)
            if new_else is not stmt.else_body:
                clauses_changed = True
        if clauses_changed:
            return IRIf(
                range=stmt.range,
                clauses=tuple(new_clauses),
                else_body=new_else,
                else_range=stmt.else_range,
            )
        return stmt

    if isinstance(stmt, IRFor):
        return _for_to_barrier(source, stmt)

    if isinstance(stmt, IRWhile):
        return _while_to_barrier(source, stmt)

    if isinstance(stmt, IRTry):
        return _try_to_barrier(source, stmt)

    if isinstance(stmt, IRSwitch):
        return _switch_to_barrier(source, stmt)

    return stmt


def _extract_body_text(source: str, range_obj: Range) -> str:
    """Extract body text from source using a Range, stripping the opening brace.

    For STR tokens (braced words), the range's ``end.offset`` points to the
    last content character — one position *before* the closing ``}``.  So
    ``source[start : end + 1]`` already excludes the outer closing brace and
    we must only strip the leading ``{``.  Stripping a trailing ``}`` would
    accidentally remove a nested closing brace (e.g. the inner ``}`` of a
    nested ``for`` body).
    """
    body_start = range_obj.start.offset
    body_end = range_obj.end.offset
    body_text = source[body_start : body_end + 1]
    if body_text.startswith("{"):
        body_text = body_text[1:]
    return body_text.strip()


def _extract_raw_text(source: str, range_obj: Range) -> str:
    """Extract raw source text from a Range, preserving original quoting."""
    start = range_obj.start.offset
    end = range_obj.end.offset
    return source[start : end + 1]


def _catch_to_barrier(source: str, stmt: IRCatch) -> IRBarrier:
    """Convert an IRCatch to an IRBarrier with the original command args."""
    # Determine body text from raw_args (pre-substituted by the lowering).
    # For braced bodies the text is literal; for double-quoted bodies we
    # must resolve backslash escapes (since the lowering stores unresolved
    # escapes in raw_args).
    if stmt.raw_args:
        body_text = stmt.raw_args[0]
        # Check if the original body was double-quoted
        start_offset = stmt.body_range.start.offset
        if start_offset < len(source) and source[start_offset] == '"':
            body_text = backslash_subst(body_text)
    else:
        body_text = _extract_body_text(source, stmt.body_range)

    # Wrap with brace markers so PUSH-time substitution is suppressed;
    # the catch command handler will receive the unwrapped text.
    args: list[str] = [_BRACE_OPEN + body_text + _BRACE_CLOSE]
    if stmt.result_var is not None:
        args.append(stmt.result_var)
    if stmt.options_var is not None:
        args.append(stmt.options_var)

    return IRBarrier(
        range=stmt.range,
        reason="catch runtime",
        command="catch",
        args=tuple(args),
    )


def _try_to_barrier(source: str, stmt: IRTry) -> IRBarrier:
    """Convert an IRTry to an IRBarrier with the original command args.

    Reconstructs: try body ?on code varList body ...? ?finally body?
    """
    body_text = _extract_body_text(source, stmt.body_range)
    args: list[str] = [_BRACE_OPEN + body_text + _BRACE_CLOSE]

    for handler in stmt.handlers:
        args.append(handler.kind)  # "on" or "trap"
        args.append(handler.match_arg)
        # Build the varList from handler's variable bindings
        var_parts: list[str] = []
        if handler.var_name is not None:
            var_parts.append(handler.var_name)
        if handler.options_var is not None:
            var_parts.append(handler.options_var)
        args.append(" ".join(var_parts) if var_parts else "")
        handler_text = _extract_body_text(source, handler.body_range)
        args.append(_BRACE_OPEN + handler_text + _BRACE_CLOSE)

    if stmt.finally_body is not None and stmt.finally_range is not None:
        args.append("finally")
        finally_text = _extract_body_text(source, stmt.finally_range)
        args.append(_BRACE_OPEN + finally_text + _BRACE_CLOSE)

    return IRBarrier(
        range=stmt.range,
        reason="try runtime",
        command="try",
        args=tuple(args),
    )


def _switch_to_barrier(source: str, stmt: IRSwitch) -> IRBarrier:
    """Convert an IRSwitch to an IRBarrier with the original command args.

    The compiled CFG path always uses exact matching (STR_EQ), losing
    -glob/-regexp mode info.  Running via the command handler preserves
    all switch semantics.
    """
    # Extract any options from source text between "switch" keyword and
    # the subject.  The IR doesn't carry mode info, so we recover it
    # from the raw source.
    cmd_start = stmt.range.start.offset
    subj_start = stmt.subject_range.start.offset
    prefix = source[cmd_start:subj_start].strip()
    # prefix is e.g. "switch -glob" or "switch" or "switch -exact --"
    prefix_words = prefix.split()
    options = prefix_words[1:]  # skip the "switch" keyword itself

    args: list[str] = list(options)
    args.append(stmt.subject)

    # Build pattern/body pairs as individual args
    for arm in stmt.arms:
        args.append(arm.pattern)
        if arm.fallthrough:
            args.append("-")
        elif arm.body is not None and arm.body_range is not None:
            body_text = _extract_body_text(source, arm.body_range)
            args.append(_BRACE_OPEN + body_text + _BRACE_CLOSE)
        else:
            args.append(_BRACE_OPEN + _BRACE_CLOSE)
    if stmt.default_body is not None and stmt.default_range is not None:
        args.append("default")
        default_text = _extract_body_text(source, stmt.default_range)
        args.append(_BRACE_OPEN + default_text + _BRACE_CLOSE)

    return IRBarrier(
        range=stmt.range,
        reason="switch runtime",
        command="switch",
        args=tuple(args),
    )


def _for_to_barrier(source: str, stmt: IRFor) -> IRBarrier:
    """Convert an IRFor to an IRBarrier with the original command args."""
    init_text = _extract_body_text(source, stmt.init_range)
    cond_text = _extract_body_text(source, stmt.condition_range)
    next_text = _extract_body_text(source, stmt.next_range)
    body_text = _extract_body_text(source, stmt.body_range)

    return IRBarrier(
        range=stmt.range,
        reason="for runtime",
        command="for",
        args=(
            _BRACE_OPEN + init_text + _BRACE_CLOSE,
            _BRACE_OPEN + cond_text + _BRACE_CLOSE,
            _BRACE_OPEN + next_text + _BRACE_CLOSE,
            _BRACE_OPEN + body_text + _BRACE_CLOSE,
        ),
    )


def _while_to_barrier(source: str, stmt: IRWhile) -> IRBarrier:
    """Convert an IRWhile to an IRBarrier with the original command args."""
    cond_text = _extract_body_text(source, stmt.condition_range)
    body_text = _extract_body_text(source, stmt.body_range)

    return IRBarrier(
        range=stmt.range,
        reason="while runtime",
        command="while",
        args=(
            _BRACE_OPEN + cond_text + _BRACE_CLOSE,
            _BRACE_OPEN + body_text + _BRACE_CLOSE,
        ),
    )


def _foreach_to_barrier(source: str, stmt: IRForeach) -> IRBarrier:
    """Convert an IRForeach to an IRBarrier with the original command args."""
    # Build args: varList list ?varList list ...? body
    cmd = "lmap" if stmt.is_lmap else "foreach"
    args: list[str] = []
    for var_names, list_arg in stmt.iterators:
        args.append(" ".join(var_names))
        args.append(list_arg)
    body_text = _extract_body_text(source, stmt.body_range)
    # Wrap body in braces so PUSH-time substitution is suppressed.
    # However, if the raw source is a command substitution ([...]),
    # preserve it so it's evaluated at runtime.
    raw = _extract_raw_text(source, stmt.body_range)
    stripped_raw = raw.lstrip()
    if stripped_raw.startswith("["):
        # Command-substitution body — use the raw_args if available
        if stmt.raw_args:
            args.append(stmt.raw_args[-1])
        else:
            args.append(raw)
    elif stripped_raw.startswith("$"):
        # Variable reference body — preserve so PUSH substitutes it
        args.append(stripped_raw.strip())
    else:
        args.append(_BRACE_OPEN + body_text + _BRACE_CLOSE)

    return IRBarrier(
        range=stmt.range,
        reason=f"{cmd} runtime",
        command=cmd,
        args=tuple(args),
    )
