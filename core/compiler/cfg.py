"""Control-flow graph (CFG) construction from structured IR.

A CFG represents a procedure as a set of *basic blocks* — straight-
line sequences of IR statements with no internal branching.  Each
block ends with a *terminator* that transfers control:

- ``CFGGoto``  – unconditional jump to one successor.
- ``CFGBranch`` – conditional jump to one of two successors.
- ``CFGReturn`` – procedure exit.

Structured IR constructs (``IRIf``, ``IRFor``, ``IRSwitch``) are
flattened into this graph form so that SSA and dataflow analyses can
reason about all possible execution paths.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from ..analysis.semantic_model import Range
from ..commands.registry.runtime import arg_indices_for_role
from ..commands.registry.signatures import ArgRole
from ..common.naming import normalise_var_name as _normalise_var_name
from ..parsing.lexer import TclLexer
from ..parsing.tokens import TokenType
from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprTernary,
    ExprUnary,
)
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRModule,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)

# Short names: r = Range, bn = block name (str).


def _defs_from_body_script(body_text: str) -> list[str]:
    """Extract variable names defined by commands in a body script.

    Scans a body script (e.g. from ``catch {set x 1}``) for commands
    that define variables via ``ArgRole.VAR_NAME`` arguments.  This lets
    ``_defs_from_expr`` recognise that ``[catch {set x [foo]}]`` defines
    ``x`` even though ``x`` is not a direct argument of ``catch``.
    """
    defs: list[str] = []
    lexer = TclLexer(body_text)
    words: list[str] = []
    prev_type = TokenType.EOL

    def _flush() -> None:
        if not words:
            return
        cmd_name = words[0]
        args = words[1:]
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.VAR_NAME)):
            if idx < len(args):
                name = _normalise_var_name(args[idx])
                if name:
                    defs.append(name)

    for tok in lexer.tokenise_all():
        if tok.type in (TokenType.EOL, TokenType.EOF):
            _flush()
            words = []
            prev_type = tok.type
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(tok.text)
        else:
            if words:
                words[-1] += tok.text
            else:
                words.append(tok.text)
        prev_type = tok.type
    _flush()
    return defs


@dataclass(frozen=True, slots=True)
class _UpvarInfo:
    """Information about a proc's upvar usage for caller-side invalidation."""

    # Literal caller-side variable names referenced in upvar commands
    literal_targets: frozenset[str]
    # Parameter names whose runtime values are used as upvar targets
    # (e.g. upvar 1 $param_name local → param_name)
    param_targets: frozenset[str]


def _collect_upvar_targets(script: IRScript) -> _UpvarInfo | None:
    """Scan *script* for ``upvar`` commands and extract target info.

    Returns None if the script contains no upvar commands.
    """
    literal_targets: set[str] = set()
    param_targets: set[str] = set()

    def _scan(s: IRScript) -> None:
        for stmt in s.statements:
            if isinstance(stmt, (IRCall, IRBarrier)) and stmt.command in (
                "upvar",
                "namespace upvar",
            ):
                args = stmt.args
                if not args:
                    continue
                # upvar ?level? otherVar myVar ?otherVar myVar ...?
                start = 0
                if stmt.command == "upvar":
                    if args[0].lstrip("-").isdigit() or args[0].startswith("#"):
                        start = 1
                elif stmt.command == "namespace upvar":
                    start = 1  # skip namespace arg
                # Pairs: otherVar myVar (caller-side, local)
                for i in range(start, len(args) - 1, 2):
                    caller_name = args[i]
                    if caller_name.startswith("$"):
                        # Dynamic: upvar 1 $param local
                        param = _normalise_var_name(caller_name)
                        if param:
                            param_targets.add(param)
                    else:
                        literal_targets.add(caller_name)
            elif isinstance(stmt, IRIf):
                for clause in stmt.clauses:
                    _scan(clause.body)
                if stmt.else_body is not None:
                    _scan(stmt.else_body)
            elif isinstance(stmt, (IRFor, IRWhile)):
                _scan(stmt.body)
            elif isinstance(stmt, IRForeach):
                _scan(stmt.body)
            elif isinstance(stmt, IRCatch):
                _scan(stmt.body)
            elif isinstance(stmt, IRTry):
                _scan(stmt.body)
                for handler in stmt.handlers:
                    _scan(handler.body)
                if stmt.finally_body is not None:
                    _scan(stmt.finally_body)
            elif isinstance(stmt, IRSwitch):
                for arm in stmt.arms:
                    if arm.body is not None:
                        _scan(arm.body)
                if stmt.default_body is not None:
                    _scan(stmt.default_body)

    _scan(script)
    if not literal_targets and not param_targets:
        return None
    return _UpvarInfo(
        literal_targets=frozenset(literal_targets),
        param_targets=frozenset(param_targets),
    )


def _defs_from_ir_script(script: IRScript) -> list[str]:
    """Collect all variable names defined anywhere inside *script* (recursive).

    Used at ``catch``/``try`` merge points to conservatively invalidate
    variables that may have been partially modified before an exception.
    """
    defs: list[str] = []
    for stmt in script.statements:
        if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
            name = _normalise_var_name(stmt.name)
            if name:
                defs.append(name)
        elif isinstance(stmt, IRCall) and stmt.defs:
            defs.extend(stmt.defs)
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                defs.extend(_defs_from_ir_script(clause.body))
            if stmt.else_body is not None:
                defs.extend(_defs_from_ir_script(stmt.else_body))
        elif isinstance(stmt, (IRFor, IRWhile)):
            defs.extend(_defs_from_ir_script(stmt.body))
        elif isinstance(stmt, IRForeach):
            for var_names, _ in stmt.iterators:
                for vn in var_names:
                    name = _normalise_var_name(vn)
                    if name:
                        defs.append(name)
            defs.extend(_defs_from_ir_script(stmt.body))
        elif isinstance(stmt, IRCatch):
            defs.extend(_defs_from_ir_script(stmt.body))
            if stmt.result_var:
                defs.append(stmt.result_var)
            if stmt.options_var:
                defs.append(stmt.options_var)
        elif isinstance(stmt, IRTry):
            defs.extend(_defs_from_ir_script(stmt.body))
            for handler in stmt.handlers:
                if handler.var_name:
                    defs.append(handler.var_name)
                if handler.options_var:
                    defs.append(handler.options_var)
                defs.extend(_defs_from_ir_script(handler.body))
            if stmt.finally_body is not None:
                defs.extend(_defs_from_ir_script(stmt.finally_body))
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    defs.extend(_defs_from_ir_script(arm.body))
            if stmt.default_body is not None:
                defs.extend(_defs_from_ir_script(stmt.default_body))
    return defs


def _defs_from_expr(expr: ExprNode) -> list[str]:
    """Extract variable names defined by command substitutions in *expr*.

    Walks the expression tree and looks for ``[cmd ...]`` substitutions
    where *cmd* has ``ArgRole.VAR_NAME`` arguments (e.g. ``catch``, ``set``,
    ``gets``, ``regexp``, ``scan``, ``binary scan``).  Also scans
    ``ArgRole.BODY`` arguments for nested variable definitions so that
    patterns like ``[catch {set x [foo]}]`` correctly report ``x``.
    """
    commands: list[ExprCommand] = []
    _collect_expr_commands(expr, commands)
    defs: list[str] = []
    for cmd_node in commands:
        text = cmd_node.text
        # Strip outer [ ... ] if present.
        if text.startswith("[") and text.endswith("]"):
            text = text[1:-1]
        # Tokenise the command to get the command name and plain-word args.
        lexer = TclLexer(text)
        words: list[str] = []
        prev_type = TokenType.EOL
        for tok in lexer.tokenise_all():
            if tok.type in (TokenType.SEP, TokenType.EOL, TokenType.EOF):
                prev_type = tok.type
                continue
            if prev_type in (TokenType.SEP, TokenType.EOL):
                words.append(tok.text)
            else:
                # Multi-token word — append to current word.
                if words:
                    words[-1] += tok.text
                else:
                    words.append(tok.text)
            prev_type = tok.type
        if not words:
            continue
        cmd_name = words[0]
        args = words[1:]
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.VAR_NAME)):
            if idx < len(args):
                name = _normalise_var_name(args[idx])
                if name:
                    defs.append(name)
        # Scan BODY arguments for variable definitions so that
        # ``[catch {set x [foo]}]`` in a condition defines ``x``.
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.BODY)):
            if idx < len(args):
                body = args[idx]
                if body.startswith("{") and body.endswith("}"):
                    body = body[1:-1]
                defs.extend(_defs_from_body_script(body))
    return defs


def _is_statically_falsy(expr: ExprNode) -> bool:
    """Return ``True`` when *expr* is a compile-time constant that is falsy.

    Walks the tree so that ``!(1)``, ``!1``, ``0 && …``, etc. are
    recognised.  Returns ``False`` for anything that cannot be resolved
    statically (variables, command substitutions, unknown functions).
    """
    from .tcl_expr_eval import eval_tcl_expr  # local to avoid circular import

    val = eval_tcl_expr(expr)
    return val is not None and not val


def _is_statically_truthy(expr: ExprNode) -> bool:
    """Return ``True`` when *expr* is a compile-time constant that is truthy."""
    from .tcl_expr_eval import eval_tcl_expr

    val = eval_tcl_expr(expr)
    return val is not None and bool(val)


def _collect_expr_commands(expr: ExprNode, out: list[ExprCommand]) -> None:
    """Recursively collect ``ExprCommand`` nodes respecting short-circuit.

    For ``&&`` / ``||`` (and their word variants ``and`` / ``or``), the
    RHS is only evaluated when the LHS does not short-circuit.  If the
    LHS is a compile-time constant that triggers short-circuit, the RHS
    is skipped — any variable definitions it would produce never execute.
    Ternary branches are handled similarly.
    """
    if isinstance(expr, ExprCommand):
        out.append(expr)
    elif isinstance(expr, ExprBinary):
        _collect_expr_commands(expr.left, out)
        if expr.op in (BinOp.AND, BinOp.WORD_AND):
            # RHS only evaluates when LHS is truthy.
            if not _is_statically_falsy(expr.left):
                _collect_expr_commands(expr.right, out)
        elif expr.op in (BinOp.OR, BinOp.WORD_OR):
            # RHS only evaluates when LHS is falsy.
            if not _is_statically_truthy(expr.left):
                _collect_expr_commands(expr.right, out)
        else:
            _collect_expr_commands(expr.right, out)
    elif isinstance(expr, ExprUnary):
        _collect_expr_commands(expr.operand, out)
    elif isinstance(expr, ExprTernary):
        _collect_expr_commands(expr.condition, out)
        if _is_statically_truthy(expr.condition):
            _collect_expr_commands(expr.true_branch, out)
        elif _is_statically_falsy(expr.condition):
            _collect_expr_commands(expr.false_branch, out)
        else:
            _collect_expr_commands(expr.true_branch, out)
            _collect_expr_commands(expr.false_branch, out)
    elif isinstance(expr, ExprCall):
        for arg in expr.args:
            _collect_expr_commands(arg, out)


@dataclass(frozen=True, slots=True)
class CFGGoto:
    target: str
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class CFGBranch:
    condition: ExprNode
    true_target: str
    false_target: str
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class CFGReturn:
    value: str | None = None
    range: Range | None = None


CFGTerminator = CFGGoto | CFGBranch | CFGReturn


@dataclass(frozen=True, slots=True)
class CFGBlock:
    name: str
    statements: tuple[IRStatement, ...] = ()
    terminator: CFGTerminator | None = None


@dataclass(frozen=True, slots=True)
class CFGFunction:
    name: str
    entry: str
    blocks: dict[str, CFGBlock]
    loop_nodes: dict[str, tuple[str, IRFor]] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class CFGModule:
    top_level: CFGFunction
    procedures: dict[str, CFGFunction]


@dataclass(slots=True)
class _MutableBlock:
    name: str
    statements: list[IRStatement] = field(default_factory=list)
    terminator: CFGTerminator | None = None


class _CFGBuilder:
    def __init__(
        self,
        *,
        inline_loops: bool = True,
        upvar_procs: dict[str, _UpvarInfo] | None = None,
        proc_params: dict[str, tuple[str, ...]] | None = None,
    ) -> None:
        self._counter = 0
        self._blocks: dict[str, _MutableBlock] = {}
        self._loop_nodes: dict[str, tuple[str, IRFor]] = {}
        self._inline_loops = inline_loops
        self._upvar_procs = upvar_procs or {}
        self._proc_params = proc_params or {}

    def _new_block(self, prefix: str) -> str:
        self._counter += 1
        name = f"{prefix}_{self._counter}"
        self._blocks[name] = _MutableBlock(name=name)
        return name

    def _block(self, name: str) -> _MutableBlock:
        return self._blocks[name]

    def _ensure_goto(self, block_name: str, target: str, r: Range | None = None) -> None:
        block = self._block(block_name)
        if block.terminator is None:
            block.terminator = CFGGoto(target=target, range=r)

    def _upvar_defs_from_text(self, text: str) -> list[str]:
        """Extract caller-side variable names invalidated by upvar procs in *text*.

        Scans command substitutions like ``[step 1]`` in the text.  If
        ``step`` is a known upvar proc, its literal targets and resolved
        param targets are returned.
        """
        if not self._upvar_procs or "[" not in text:
            return []
        defs: list[str] = []
        import re

        for m in re.finditer(r"\[(\w+)((?:\s+[^\[\]]*)*)\]", text):
            cmd = m.group(1)
            info = self._upvar_procs.get(cmd)
            if info is None:
                continue
            for name in info.literal_targets:
                if name not in defs:
                    defs.append(name)
            if info.param_targets:
                raw_args = m.group(2).split()
                params = self._proc_params.get(cmd, ())
                for pt in info.param_targets:
                    if pt in params:
                        param_idx = params.index(pt)
                        if param_idx < len(raw_args):
                            arg_name = _normalise_var_name(raw_args[param_idx])
                            if arg_name and arg_name not in defs:
                                defs.append(arg_name)
        return defs

    def _resolve_upvar_defs(self, cmd: str, args: tuple[str, ...]) -> list[str]:
        """Return caller-side variable names that *cmd* might modify via upvar."""
        info = self._upvar_procs.get(cmd)
        if info is None:
            return []
        defs: list[str] = []
        for name in info.literal_targets:
            if name not in defs:
                defs.append(name)
        if info.param_targets:
            params = self._proc_params.get(cmd, ())
            for pt in info.param_targets:
                if pt in params:
                    param_idx = params.index(pt)
                    if param_idx < len(args):
                        arg_name = _normalise_var_name(args[param_idx])
                        if arg_name and arg_name not in defs:
                            defs.append(arg_name)
        return defs

    def _apply_upvar_invalidation(self, stmt: IRStatement, block: _MutableBlock) -> IRStatement:
        """Augment *stmt* to invalidate caller variables modified via upvar.

        For direct IRCall to an upvar proc: add targets to defs.
        For IRAssignValue/IRCall with embedded [upvar_proc ...]: emit a
        synthetic IRCall with the extra defs before the statement.
        """
        if not self._upvar_procs:
            return stmt

        # Direct call to an upvar proc
        if isinstance(stmt, IRCall) and stmt.command in self._upvar_procs:
            extra = self._resolve_upvar_defs(stmt.command, stmt.args)
            if extra:
                merged = list(stmt.defs)
                for d in extra:
                    if d not in merged:
                        merged.append(d)
                stmt = IRCall(
                    range=stmt.range,
                    command=stmt.command,
                    args=stmt.args,
                    defs=tuple(merged),
                    tokens=stmt.tokens,
                )

        # Embedded command substitutions in value/args
        texts: list[str] = []
        if isinstance(stmt, IRAssignValue) and "[" in stmt.value:
            texts.append(stmt.value)
        elif isinstance(stmt, IRCall):
            for arg in stmt.args:
                if "[" in arg:
                    texts.append(arg)
        if texts:
            all_extra: list[str] = []
            for t in texts:
                all_extra.extend(self._upvar_defs_from_text(t))
            if all_extra:
                if isinstance(stmt, IRCall):
                    merged = list(stmt.defs)
                    for d in all_extra:
                        if d not in merged:
                            merged.append(d)
                    stmt = IRCall(
                        range=stmt.range,
                        command=stmt.command,
                        args=stmt.args,
                        defs=tuple(merged),
                        tokens=stmt.tokens,
                    )
                else:
                    # Emit synthetic IRCall before the statement to
                    # invalidate the upvar-affected variables.
                    block.statements.append(
                        IRCall(
                            range=stmt.range,
                            command="<upvar-invalidate>",
                            defs=tuple(dict.fromkeys(all_extra)),
                        )
                    )
        return stmt

    def build_function(self, name: str, script: IRScript) -> CFGFunction:
        entry = self._new_block("entry")
        tail = self._lower_script(script, entry)
        if tail is not None:
            self._ensure_goto(tail, self._new_block("exit"))
        frozen = {
            bn: CFGBlock(
                name=b.name,
                statements=tuple(b.statements),
                terminator=b.terminator,
            )
            for bn, b in self._blocks.items()
        }
        return CFGFunction(
            name=name,
            entry=entry,
            blocks=frozen,
            loop_nodes=dict(self._loop_nodes),
        )

    def _lower_script(self, script: IRScript, block_name: str) -> str | None:
        current = block_name
        for stmt in script.statements:
            block = self._block(current)
            if block.terminator is not None:
                return None

            match stmt:
                case IRIf():
                    current = self._lower_if(stmt, current)
                    if current is None:
                        return None
                case IRFor():
                    if isinstance(stmt.condition, ExprCommand) and stmt.raw_args:
                        # Frozen for: condition is a command substitution
                        # like [expr {$i < 3}].  tclsh compiles the expr
                        # inline but invokes the for command generically.
                        block.statements.append(
                            IRBarrier(
                                range=stmt.range,
                                reason="frozen for (cmd-subst condition)",
                                command="for",
                                args=stmt.raw_args,
                            )
                        )
                    else:
                        current = self._lower_for(stmt, current)
                        if current is None:
                            return None
                case IRWhile():
                    if isinstance(stmt.condition, ExprCommand) and stmt.raw_args:
                        # Frozen while: same as frozen for above.
                        block.statements.append(
                            IRBarrier(
                                range=stmt.range,
                                reason="frozen while (cmd-subst condition)",
                                command="while",
                                args=stmt.raw_args,
                            )
                        )
                    else:
                        current = self._lower_while(stmt, current)
                        if current is None:
                            return None
                case IRForeach():
                    if stmt.is_dict_iteration and stmt.raw_args:
                        # dict for/map: Tcl 9.0 compiles as a generic
                        # invoke to the ensemble implementation, not an
                        # inlined loop.  Rewrite to qualified name and
                        # drop the subcommand arg.
                        sub = stmt.raw_args[0]  # "for" or "map"
                        qual_cmd = f"::tcl::dict::{sub}"
                        block.statements.append(
                            IRBarrier(
                                range=stmt.range,
                                reason="dict for/map",
                                command=qual_cmd,
                                args=stmt.raw_args[1:],  # skip subcommand
                                tokens=None,
                            )
                        )
                    elif (not self._inline_loops and stmt.raw_args) or any(
                        v.startswith("::") for var_list, _ in stmt.iterators for v in var_list
                    ):
                        # Top-level or qualified loop vars: emit as opaque
                        # invokeStk call.  tclsh 9.0 falls back to generic
                        # invoke when a loop variable is namespace-qualified.
                        cmd = "lmap" if stmt.is_lmap else "foreach"
                        loop_vars = tuple(v for var_list, _ in stmt.iterators for v in var_list)
                        block.statements.append(
                            IRCall(
                                range=stmt.range,
                                command=cmd,
                                args=stmt.raw_args,
                                defs=loop_vars,
                            )
                        )
                    else:
                        current = self._lower_foreach(stmt, current)
                        if current is None:
                            return None
                case IRCatch():
                    # Always keep catch as opaque IRCall — codegen emits
                    # inline beginCatch4/endCatch bytecodes.
                    # Include defs for all variables modified inside the body
                    # so that SSA/SCCP doesn't propagate stale pre-catch
                    # constants past the catch (an exception can leave
                    # variables partially modified).
                    catch_defs: list[str] = _defs_from_ir_script(stmt.body)
                    if stmt.result_var:
                        catch_defs.append(stmt.result_var)
                    if stmt.options_var:
                        catch_defs.append(stmt.options_var)
                    block.statements.append(
                        IRCall(
                            range=stmt.range,
                            command="catch",
                            args=stmt.raw_args,
                            defs=tuple(dict.fromkeys(catch_defs)),
                        )
                    )
                case IRTry():
                    # try/finally (no handlers) is compiled inline at
                    # all levels by tclsh 9.0.  Other try forms (with
                    # on/trap handlers) are deferred at top level.
                    _defer_try = (
                        not self._inline_loops
                        and stmt.raw_args
                        and (stmt.handlers or not stmt.finally_body)
                    )
                    if _defer_try:
                        try_defs: list[str] = _defs_from_ir_script(stmt.body)
                        for handler in stmt.handlers:
                            if handler.var_name:
                                try_defs.append(handler.var_name)
                            if handler.options_var:
                                try_defs.append(handler.options_var)
                            try_defs.extend(_defs_from_ir_script(handler.body))
                        if stmt.finally_body is not None:
                            try_defs.extend(_defs_from_ir_script(stmt.finally_body))
                        block.statements.append(
                            IRCall(
                                range=stmt.range,
                                command="try",
                                args=stmt.raw_args,
                                defs=tuple(dict.fromkeys(try_defs)),
                            )
                        )
                    else:
                        current = self._lower_try(stmt, current)
                        if current is None:
                            return None
                case IRSwitch():
                    current = self._lower_switch(stmt, current)
                    if current is None:
                        return None
                case IRReturn(value=value, range=r):
                    block.terminator = CFGReturn(value=value, range=r)
                    return None
                case _:
                    stmt = self._apply_upvar_invalidation(stmt, block)
                    block.statements.append(stmt)

        return current

    def _lower_if(self, stmt: IRIf, block_name: str) -> str | None:
        end_block = self._new_block("if_end")
        dispatch_block = block_name

        for clause in stmt.clauses:
            # Emit synthetic defs for variables set by command substitutions
            # in the condition (e.g. [catch { ... } result]).
            cond_defs = _defs_from_expr(clause.condition)
            if cond_defs:
                self._block(dispatch_block).statements.append(
                    IRCall(range=stmt.range, command="<cond>", defs=tuple(cond_defs))
                )

            then_block = self._new_block("if_then")
            next_dispatch = self._new_block("if_next")
            self._block(dispatch_block).terminator = CFGBranch(
                condition=clause.condition,
                true_target=then_block,
                false_target=next_dispatch,
                range=clause.condition_range,
            )
            then_tail = self._lower_script(clause.body, then_block)
            if then_tail is not None:
                self._ensure_goto(then_tail, end_block, clause.body_range)
            dispatch_block = next_dispatch

        if stmt.else_body is not None:
            else_tail = self._lower_script(stmt.else_body, dispatch_block)
            if else_tail is not None:
                self._ensure_goto(else_tail, end_block, stmt.else_range)
        else:
            self._ensure_goto(dispatch_block, end_block, stmt.range)

        return end_block

    def _lower_switch(self, stmt: IRSwitch, block_name: str) -> str | None:
        # Tcl 9.0 compiles -glob/-regexp switches as generic invokeStk1
        # calls, not inline jumpTable dispatch.
        if stmt.mode in ("glob", "regexp"):
            barrier = IRBarrier(
                range=stmt.range,
                reason=f"switch -{stmt.mode}",
                command="switch",
                args=stmt.raw_args,
            )
            self._block(block_name).statements.append(barrier)
            return block_name

        end_block = self._new_block("switch_end")
        default_block = self._new_block("switch_default")
        dispatch_block = block_name

        body_targets: list[str] = []
        for arm in stmt.arms:
            body_targets.append(self._new_block("switch_arm_body"))

        final_targets: list[str] = []
        for i, arm in enumerate(stmt.arms):
            if arm.fallthrough:
                j = i + 1
                target = default_block
                while j < len(stmt.arms):
                    if not stmt.arms[j].fallthrough:
                        target = body_targets[j]
                        break
                    j += 1
                final_targets.append(target)
            else:
                final_targets.append(body_targets[i])

        for i, arm in enumerate(stmt.arms):
            next_dispatch = self._new_block("switch_next")
            cond = ExprBinary(
                op=BinOp.STR_EQ,
                left=ExprRaw(text=stmt.subject),
                right=ExprLiteral(text=arm.pattern, start=0, end=0),
            )
            self._block(dispatch_block).terminator = CFGBranch(
                condition=cond,
                true_target=final_targets[i],
                false_target=next_dispatch,
                range=arm.pattern_range,
            )
            dispatch_block = next_dispatch

        self._ensure_goto(dispatch_block, default_block, stmt.default_range or stmt.range)

        for i, arm in enumerate(stmt.arms):
            if arm.fallthrough or arm.body is None:
                self._ensure_goto(
                    body_targets[i], final_targets[i], arm.body_range or arm.pattern_range
                )
                continue
            arm_tail = self._lower_script(arm.body, body_targets[i])
            if arm_tail is not None:
                self._ensure_goto(arm_tail, end_block, arm.body_range)

        if stmt.default_body is not None:
            d_tail = self._lower_script(stmt.default_body, default_block)
            if d_tail is not None:
                self._ensure_goto(d_tail, end_block, stmt.default_range)
        else:
            self._ensure_goto(default_block, end_block, stmt.range)

        return end_block

    def _lower_for(self, stmt: IRFor, block_name: str) -> str | None:
        # Emit placeholder for empty init clause (tclsh emits push ""; pop → 3 nops).
        if not stmt.init.statements:
            self._block(block_name).statements.append(
                IRCall(range=stmt.init_range, command="<empty_clause>")
            )
        init_tail = self._lower_script(stmt.init, block_name)
        if init_tail is None:
            return None

        header_block = self._new_block("for_header")
        body_block = self._new_block("for_body")
        step_block = self._new_block("for_step")
        end_block = self._new_block("for_end")

        self._ensure_goto(init_tail, header_block, stmt.init_range)

        cond_defs = _defs_from_expr(stmt.condition)
        if cond_defs:
            self._block(header_block).statements.append(
                IRCall(range=stmt.range, command="<cond>", defs=tuple(cond_defs))
            )

        self._block(header_block).terminator = CFGBranch(
            condition=stmt.condition,
            true_target=body_block,
            false_target=end_block,
            range=stmt.condition_range,
        )

        body_tail = self._lower_script(stmt.body, body_block)
        if body_tail is not None:
            self._ensure_goto(body_tail, step_block, stmt.body_range)

        # Emit placeholder for empty next clause (tclsh emits push ""; pop → 3 nops).
        if not stmt.next.statements:
            self._block(step_block).statements.append(
                IRCall(range=stmt.next_range, command="<empty_clause>")
            )
        step_tail = self._lower_script(stmt.next, step_block)
        if step_tail is not None:
            self._ensure_goto(step_tail, header_block, stmt.next_range)

        self._loop_nodes[end_block] = (block_name, stmt)
        return end_block

    def _lower_foreach(self, stmt: IRForeach, block_name: str) -> str | None:
        """Lower foreach/lmap to a simple loop CFG pattern.

        A synthetic ``IRCall`` with ``defs`` is placed at the loop header
        to mark the iteration variables as defined on each iteration.
        """
        header_block = self._new_block("foreach_header")
        body_block = self._new_block("foreach_body")
        end_block = self._new_block("foreach_end")

        self._ensure_goto(block_name, header_block, stmt.range)

        # Collect all iteration variable names across all iterator groups.
        all_vars: list[str] = []
        for var_names, _list_arg in stmt.iterators:
            all_vars.extend(var_names)

        # Synthetic def node for iteration variables — SSA sees these as
        # definitions produced by the foreach header on each iteration.
        # Carry list expressions in args so codegen can emit foreach_start.
        list_args = tuple(list_arg for _, list_arg in stmt.iterators)
        if stmt.is_dict_iteration:
            fe_cmd = "dict for" if not stmt.is_lmap else "dict map"
        else:
            fe_cmd = "foreach" if not stmt.is_lmap else "lmap"
        var_def = IRCall(
            range=stmt.range,
            command=fe_cmd,
            args=list_args,
            defs=tuple(all_vars),
        )
        self._block(header_block).statements.append(var_def)

        # Opaque condition: the loop runs "for each element" — we model
        # this as a non-deterministic branch so both paths are explored.
        opaque_cond = ExprRaw(text="<foreach_has_next>")
        self._block(header_block).terminator = CFGBranch(
            condition=opaque_cond,
            true_target=body_block,
            false_target=end_block,
            range=stmt.range,
        )

        body_tail = self._lower_script(stmt.body, body_block)
        if body_tail is not None:
            self._ensure_goto(body_tail, header_block, stmt.body_range)

        return end_block

    def _lower_while(self, stmt: IRWhile, block_name: str) -> str | None:
        header_block = self._new_block("while_header")
        body_block = self._new_block("while_body")
        end_block = self._new_block("while_end")

        self._ensure_goto(block_name, header_block, stmt.condition_range)

        cond_defs = _defs_from_expr(stmt.condition)
        if cond_defs:
            self._block(header_block).statements.append(
                IRCall(range=stmt.range, command="<cond>", defs=tuple(cond_defs))
            )

        self._block(header_block).terminator = CFGBranch(
            condition=stmt.condition,
            true_target=body_block,
            false_target=end_block,
            range=stmt.condition_range,
        )

        body_tail = self._lower_script(stmt.body, body_block)
        if body_tail is not None:
            self._ensure_goto(body_tail, header_block, stmt.body_range)

        return end_block

    def _lower_catch(self, stmt: IRCatch, block_name: str) -> str | None:
        """Lower ``catch`` to CFG: body may throw, then merge with result vars."""
        body_block = self._new_block("catch_body")
        merge_block = self._new_block("catch_merge")

        self._ensure_goto(block_name, body_block, stmt.range)

        body_tail = self._lower_script(stmt.body, body_block)
        if body_tail is not None:
            self._ensure_goto(body_tail, merge_block, stmt.body_range)

        # If the body throws, control still reaches the merge block.
        # Model this as a second edge from the entry to the merge.
        self._ensure_goto(block_name, merge_block, stmt.range)

        # Emit synthetic defs for result/options variables at the merge point.
        var_defs: list[str] = []
        if stmt.result_var:
            var_defs.append(stmt.result_var)
        if stmt.options_var:
            var_defs.append(stmt.options_var)
        if var_defs:
            self._block(merge_block).statements.append(
                IRCall(range=stmt.range, command="catch", defs=tuple(var_defs))
            )

        return merge_block

    def _lower_try(self, stmt: IRTry, block_name: str) -> str | None:
        """Lower ``try`` to CFG: body → handlers → finally → end."""
        body_block = self._new_block("try_body")
        end_block = self._new_block("try_end")

        self._ensure_goto(block_name, body_block, stmt.range)

        # Where does control go after the body succeeds?
        post_body = self._new_block("try_ok") if stmt.handlers else end_block
        body_tail = self._lower_script(stmt.body, body_block)
        if body_tail is not None:
            self._ensure_goto(body_tail, post_body, stmt.body_range)

        # Each handler is reachable from body failure (modelled as a
        # non-deterministic branch from the body entry block).
        for handler in stmt.handlers:
            handler_block = self._new_block("try_handler")
            self._ensure_goto(block_name, handler_block, stmt.range)

            # Emit synthetic defs for handler-bound variables.
            var_defs: list[str] = []
            if handler.var_name:
                var_defs.append(handler.var_name)
            if handler.options_var:
                var_defs.append(handler.options_var)
            if var_defs:
                self._block(handler_block).statements.append(
                    IRCall(range=stmt.range, command="try", defs=tuple(var_defs))
                )

            handler_tail = self._lower_script(handler.body, handler_block)
            target = end_block
            if handler_tail is not None:
                self._ensure_goto(handler_tail, target, handler.body_range)

        # The success path also reaches end_block.
        if stmt.handlers:
            self._ensure_goto(post_body, end_block, stmt.range)

        # Finally block: if present, every path passes through it.
        if stmt.finally_body is not None:
            finally_block = self._new_block("try_finally")
            # Re-target end_block to go through finally first.
            self._ensure_goto(end_block, finally_block, stmt.finally_range or stmt.range)
            finally_tail = self._lower_script(stmt.finally_body, finally_block)
            after_finally = self._new_block("try_after_finally")
            if finally_tail is not None:
                self._ensure_goto(finally_tail, after_finally, stmt.finally_range or stmt.range)
            return after_finally

        return end_block


def _detect_upvar_procs(module: IRModule) -> dict[str, _UpvarInfo]:
    """Scan the IR module for procedures whose bodies contain ``upvar``.

    Returns a mapping from command name → _UpvarInfo.  Both qualified
    (``::foo``) and unqualified (``foo``) names are included.
    """
    result: dict[str, _UpvarInfo] = {}
    for qname, proc in module.procedures.items():
        info = _collect_upvar_targets(proc.body)
        if info is not None:
            result[qname] = info
            short = qname.rsplit("::", 1)[-1]
            if short:
                result[short] = _UpvarInfo(
                    literal_targets=info.literal_targets,
                    # For unqualified calls, map param targets through the
                    # proc's parameter list
                    param_targets=info.param_targets,
                )
    return result


def prepare_cfg_context(
    module: IRModule,
) -> tuple[dict[str, _UpvarInfo], dict[str, tuple[str, ...]]]:
    """Return (upvar_procs, proc_params) for CFG construction.

    Centralises the upvar scan and parameter-map build so that both
    ``build_cfg()`` and ``compile_source()`` share one implementation.
    """
    upvar_procs = _detect_upvar_procs(module)
    proc_params: dict[str, tuple[str, ...]] = {}
    for qname, proc in module.procedures.items():
        proc_params[qname] = proc.params
        short = qname.rsplit("::", 1)[-1]
        if short:
            proc_params[short] = proc.params
    return upvar_procs, proc_params


def build_cfg(module: IRModule, *, defer_top_level: bool = False) -> CFGModule:
    """Build CFG for top-level script and each lowered proc.

    When *defer_top_level* is True, foreach/catch/try at the top level
    are compiled as opaque ``invokeStk`` calls (matching tclsh's
    bytecode output).  Analysis passes should leave this False to get
    full inlining of loop bodies.
    """
    upvar_procs, proc_params = prepare_cfg_context(module)
    top_cfg = build_cfg_function(
        "::top",
        module.top_level,
        inline_loops=not defer_top_level,
        upvar_procs=upvar_procs,
        proc_params=proc_params,
    )

    proc_cfgs: dict[str, CFGFunction] = {}
    for qname, proc in module.procedures.items():
        proc_cfgs[qname] = build_cfg_function(
            qname,
            proc.body,
            upvar_procs=upvar_procs,
            proc_params=proc_params,
        )

    return CFGModule(top_level=top_cfg, procedures=proc_cfgs)


def build_cfg_function(
    name: str,
    script: IRScript,
    *,
    inline_loops: bool = True,
    upvar_procs: dict[str, _UpvarInfo] | None = None,
    proc_params: dict[str, tuple[str, ...]] | None = None,
) -> CFGFunction:
    """Build CFG for a single function/script body."""
    builder = _CFGBuilder(
        inline_loops=inline_loops,
        upvar_procs=upvar_procs,
        proc_params=proc_params,
    )
    return builder.build_function(name, script)
