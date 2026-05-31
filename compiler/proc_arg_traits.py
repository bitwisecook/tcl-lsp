"""Proc argument trait inference.

Walks a proc body to determine how each parameter is used:
- EVAL: passed to eval/uplevel/subst (treated as code)
- BODY: used as a loop/control body (foreach body, while body, etc.)
- VAR_WRITE: names a variable the proc writes via upvar + set/incr/append
- VAR_READ: names a variable the proc reads via upvar (read-only alias)
- EXPR: evaluated as an expression
- LOOP_LIST: used as the list argument in foreach/lmap

Two analysis tiers:

1. **Shallow** (``infer_param_traits``) — single-pass, top-level command
   scan.  Fast enough for synchronous analysis during typing.  Detects
   direct patterns like ``eval $param`` and ``upvar 1 $param local``.

2. **Deep** (``infer_param_traits_deep``) — recursive descent into nested
   script bodies.  Catches traits hidden inside braced bodies, e.g.
   ``foreach item $items { uplevel 1 $body }``.  Intended to be called
   asynchronously and its results merged into the proc's trait map.
"""

from __future__ import annotations

from collections.abc import Mapping

from compiler.parsing.lexer import TclLexer, TclParseError
from compiler.registry import REGISTRY
from compiler.registry.runtime import resolve_arg_role_map as _resolve_arg_roles
from compiler.registry.signatures import ArgRole
from shared.proc_traits import ProcArgTrait
from shared.tokens import TokenType

# Simple $varName reference pattern.

# Variable-writing commands derived from CommandSpec.assigns_variable_at.
# "dict set" etc. are handled via the registry's ArgRole.VAR_WRITE on
# subcommand specs, not here (cmd_name from the segmenter is just "dict").
_var_write_cache: dict[str, int] | None = None


def _var_write_commands() -> dict[str, int]:
    global _var_write_cache
    if _var_write_cache is None:
        from compiler.registry.runtime import variable_writing_commands

        _var_write_cache = variable_writing_commands()
    return _var_write_cache


def _extract_var_name(text: str) -> str | None:
    """Extract a bare scalar variable name from a ``$var`` / ``${var}`` word.

    Uses the lexer rather than a regex: the whole word must be exactly one
    variable substitution naming a scalar (no array index), and the name must
    have the conventional shape (letter/underscore start, word chars and
    ``::`` namespace separators).
    """
    lexer = TclLexer(text)
    tok = lexer.get_token()
    if tok is None or tok.type is not TokenType.VAR or tok.start.offset != 0:
        return None
    name = tok.text
    # The word must be solely this variable reference.
    if text not in (f"${name}", f"${{{name}}}"):
        return None
    if not name or not (name[0].isalpha() or name[0] == "_"):
        return None
    if not all(ch.isalnum() or ch in "_:" for ch in name):
        return None
    return name


def _extract_commands(source: str) -> list[tuple[str, list[str]]]:
    """Extract (command_name, args) pairs from source via command segmenter."""
    from compiler.parsing.command_segmenter import segment_commands

    commands: list[tuple[str, list[str]]] = []
    try:
        segments = segment_commands(source)
    except TclParseError:
        return commands

    for seg in segments:
        if not seg.texts:
            continue
        cmd_name = seg.texts[0]
        cmd_args = seg.texts[1:]
        commands.append((cmd_name, cmd_args))

    return commands


def _scan_commands(
    commands: list[tuple[str, list[str]]],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
) -> None:
    """Core trait detection loop shared by both shallow and deep passes."""
    for cmd_name, cmd_args in commands:
        arg_roles = _resolve_arg_roles(cmd_name, cmd_args)

        for idx, arg in enumerate(cmd_args):
            var_name = _extract_var_name(arg)
            if var_name is None:
                continue

            source_param = var_name if var_name in param_set else upvar_aliases.get(var_name)
            if source_param is None:
                continue

            roles = arg_roles.get(idx, frozenset())

            if ArgRole.BODY in roles:
                traits[source_param].add(ProcArgTrait.BODY)
            if ArgRole.EXPR in roles:
                traits[source_param].add(ProcArgTrait.EXPR)
            if ArgRole.VAR_WRITE in roles:
                # This branch only ever sees the ``$param`` *substitution* form
                # (``_extract_var_name`` requires it), so the command writes a
                # variable in the CURRENT scope *named by the param's value*
                # (``set $p …``, ``variable $p``, ``incr $p``) — it does NOT
                # write back to the caller's passed variable (verified: in Tcl
                # only ``upvar`` to a caller frame writes back).  The trait is
                # DYNAMIC_NAME_LOCAL (callee-local dynamic-name use), NOT
                # VAR_READ/VAR_WRITE (which would imply caller-frame aliasing).
                # See PR #498 deep-review finding 10: conflating these caused
                # ``proc f {p} { set \$p 1 }`` to wrongly suppress the caller's
                # ``set x 1; f x`` as if ``x`` were consumed by the callee.
                #
                # Also emit VAR_READ: the param's string value IS read
                # (used as a variable name).  VAR_READ is the broader
                # "param value is consumed" trait; DYNAMIC_NAME_LOCAL
                # is the refinement that says "consumed as a callee-
                # local var name, not as a caller-frame alias".
                # Consumers querying VAR_READ-alone for "is the param
                # used at all" must still see this case.
                traits[source_param].add(ProcArgTrait.DYNAMIC_NAME_LOCAL)
                traits[source_param].add(ProcArgTrait.VAR_READ)
            if ArgRole.VAR_READ in roles:
                traits[source_param].add(ProcArgTrait.DYNAMIC_NAME_LOCAL)
                traits[source_param].add(ProcArgTrait.VAR_READ)

        # Commands that evaluate code (eval, uplevel, subst, etc.)
        spec = REGISTRY.get_any(cmd_name)
        if spec is not None and (spec.evaluates_code or spec.performs_substitution):
            if spec.evaluates_code and cmd_name == "uplevel":
                # uplevel: last arg is the script
                if cmd_args:
                    vn = _extract_var_name(cmd_args[-1])
                    if vn and vn in param_set:
                        traits[vn].add(ProcArgTrait.EVAL)
            else:
                for arg in cmd_args:
                    vn = _extract_var_name(arg)
                    if vn and vn in param_set:
                        traits[vn].add(ProcArgTrait.EVAL)

        # upvar creates aliases
        if spec is not None and spec.creates_scope_alias and cmd_name == "upvar":
            _handle_upvar(cmd_args, param_set, traits, upvar_aliases)

        # foreach / lmap
        if cmd_name in ("foreach", "lmap"):
            _handle_foreach(cmd_args, param_set, traits)

        # while {cond} body
        if cmd_name == "while":
            _handle_while(cmd_args, param_set, traits)

        # for {init} {cond} {next} body
        if cmd_name == "for":
            _handle_for(cmd_args, param_set, traits)

        # after ms ?script ...? / after idle ?script ...?
        if cmd_name == "after":
            _handle_after(cmd_args, param_set, traits)

        # scan string format ?varName ...? — args 2+ are written
        if cmd_name == "scan":
            _handle_variadic_var_write(cmd_args, param_set, traits, start=2)

        # lassign list ?varName ...? — args 1+ are written
        if cmd_name == "lassign":
            _handle_variadic_var_write(cmd_args, param_set, traits, start=1)

        # regexp ?switches? exp string ?matchVar ?subMatchVar ...??
        if cmd_name == "regexp":
            _handle_regexp_vars(cmd_args, param_set, traits)

        # regsub ?switches? exp string subSpec ?varName?
        if cmd_name == "regsub":
            _handle_regsub_var(cmd_args, param_set, traits)

        # switch — body args handled via registry, but dynamic
        # pattern/body pairs also detected via _resolve_arg_roles.

        # Variable-writing commands where param is used as var name.
        # Skip commands with subcommands (e.g. array) — those are handled
        # via _resolve_arg_roles above which checks subcommand-level roles.
        vwc = _var_write_commands()
        if cmd_name in vwc:
            spec = REGISTRY.get_any(cmd_name)
            if spec is None or not spec.subcommands:
                var_idx = vwc[cmd_name]
                if var_idx < len(cmd_args):
                    vn = _extract_var_name(cmd_args[var_idx])
                    if vn and vn in param_set:
                        # The name-arg is the ``$param`` substitution form
                        # (``_extract_var_name`` only matches that), so the
                        # param's VALUE names a variable in the CURRENT scope
                        # (``set $p …``, ``variable $p``) — not a write-back to
                        # the caller's passed variable.  Emit BOTH the
                        # DYNAMIC_NAME_LOCAL refinement (so caller-arg
                        # suppression honours the callee-local rule)
                        # AND VAR_READ (the param string IS consumed),
                        # so broader "is the param used" queries see it.
                        # VAR_READ is OK to add since the param string
                        # IS being read; only VAR_WRITE (genuine
                        # upvar-aliased caller-frame write) is held back.
                        traits[vn].add(ProcArgTrait.DYNAMIC_NAME_LOCAL)
                        traits[vn].add(ProcArgTrait.VAR_READ)

        # Track writes through upvar aliases
        if cmd_name in ("set", "incr", "append", "lappend") and cmd_args:
            alias_target = upvar_aliases.get(cmd_args[0])
            if alias_target is not None:
                traits[alias_target].add(ProcArgTrait.VAR_WRITE)

        # foreach/lmap loop variables are written to — upgrade aliases.
        if cmd_name in ("foreach", "lmap") and len(cmd_args) >= 3:
            remaining = cmd_args[:-1]
            i = 0
            while i < len(remaining):
                alias_target = upvar_aliases.get(remaining[i])
                if alias_target is not None:
                    traits[alias_target].add(ProcArgTrait.VAR_WRITE)
                i += 2


def infer_param_traits(
    params: tuple[str, ...],
    body_source: str,
) -> dict[str, frozenset[ProcArgTrait]]:
    """Shallow trait inference — top-level commands only.

    Fast enough for synchronous analysis.  Detects direct patterns
    like ``eval $param``, ``upvar 1 $param local``, ``foreach x $list body``.
    Does not recurse into braced body arguments.
    """
    if not params or not body_source.strip():
        return {}

    param_set = set(params)
    traits: dict[str, set[ProcArgTrait]] = {p: set() for p in params}
    upvar_aliases: dict[str, str] = {}

    commands = _extract_commands(body_source)
    _scan_commands(commands, param_set, traits, upvar_aliases)

    return {p: frozenset(t) for p, t in traits.items() if t}


def infer_param_traits_deep(
    params: tuple[str, ...],
    body_source: str,
) -> dict[str, frozenset[ProcArgTrait]]:
    """Deep trait inference — recursively descends into nested script bodies.

    Catches traits hidden inside braced bodies, e.g.::

        foreach item $items {
            uplevel 1 $body    ;# $body EVAL trait detected here
        }

    This is more expensive than ``infer_param_traits`` and should be
    called asynchronously.  Its results are merged (union) with the
    shallow pass results.
    """
    if not params or not body_source.strip():
        return {}

    param_set = set(params)
    traits: dict[str, set[ProcArgTrait]] = {p: set() for p in params}
    upvar_aliases: dict[str, str] = {}

    _scan_deep(body_source, param_set, traits, upvar_aliases, depth=0)

    return {p: frozenset(t) for p, t in traits.items() if t}


_MAX_DEPTH = 8  # prevent runaway recursion on pathological input


def _scan_deep(
    source: str,
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
    depth: int,
) -> None:
    """Recursively scan commands, descending into body arguments."""
    if depth > _MAX_DEPTH:
        return

    from compiler.registry.runtime import body_arg_indices

    commands = _extract_commands(source)
    _scan_commands(commands, param_set, traits, upvar_aliases)

    # Now recurse into body arguments to find deeper usage.
    from compiler.parsing.command_segmenter import segment_commands

    try:
        segments = segment_commands(source)
    except TclParseError:
        return

    for seg in segments:
        if not seg.texts:
            continue
        cmd_name = seg.texts[0]
        args = seg.texts[1:]
        body_indices = body_arg_indices(cmd_name, args)

        for idx in body_indices:
            if idx >= len(args):
                continue
            body_text = args[idx]
            if not body_text.strip():
                continue
            # Only recurse into braced bodies (not $var references which
            # are already handled at the top level).
            if "$" in body_text[:2] or "[" in body_text[:2]:
                continue
            _scan_deep(body_text, param_set, traits, upvar_aliases, depth + 1)


def merge_traits(
    shallow: dict[str, frozenset[ProcArgTrait]],
    deep: dict[str, frozenset[ProcArgTrait]],
) -> dict[str, frozenset[ProcArgTrait]]:
    """Merge shallow and deep trait results (union per parameter)."""
    merged: dict[str, frozenset[ProcArgTrait]] = dict(shallow)
    for param, deep_traits in deep.items():
        if param in merged:
            merged[param] = merged[param] | deep_traits
        else:
            merged[param] = deep_traits
    return merged


def collect_call_by_name_reads(
    cfg,
    proc_index: dict[str, list[tuple[tuple[str, ...], dict[str, frozenset[ProcArgTrait]]]]],
) -> set[str]:
    """Caller-local var names passed *by name* to a user proc that consumes
    them via ``upvar`` (call-by-name).

    Mirrors the analyser-side helper in ``_diag_var_lifecycle`` but is keyed
    only by a pre-built ``proc_index`` so both the analyser (W211/W220) and
    the optimiser (O109/O126) can share the suppression without crossing
    layer boundaries.  ``proc_index`` maps bare/qualified command names to
    ``(params, param_traits)`` pairs; callers populate it from whichever
    proc registry they have access to (``ProcDef``-backed in the analyser,
    ``ProcSummary``-backed in the optimiser).

    Returns the set of caller-local variable names that should NOT be
    flagged as dead/unused.  Substituted args (``$name``) and array elems
    are deliberately excluded — they name a runtime variable we cannot
    statically identify, so they preserve genuine FP cases where the user
    passed a literal string instead of a name.

    The scan covers (a) top-level ``IRCall``/``IRBarrier`` statements and
    (b) command substitutions inside ``IRAssignValue``/``IRAssignExpr``/
    ``IRExprEval``/``IRReturn`` values — a call like ``[asnPeekTag data
    tag type dummy]`` embedded in ``set len [..]`` is the dominant
    call-by-name shape in tcllib (asn, tar, ncgi, …) and must be caught
    or W211/W220/O109/O126 fire on the initialiser ``set tag ""``.
    """
    from compiler.cfg import IRBarrier
    from compiler.ir import IRAssignExpr, IRAssignValue, IRCall, IRExprEval, IRReturn
    from shared.naming import normalise_qualified_name as _norm

    if not proc_index:
        return set()

    by_name: set[str] = set()

    def _add_for_call(cmd: str, args: tuple[str, ...]) -> None:
        if not cmd or "$" in cmd or "[" in cmd:
            return
        cand = proc_index.get(cmd) or proc_index.get(_norm(cmd)) or proc_index.get(cmd.lstrip(":"))
        if not cand:
            return
        for params, traits_map in cand:
            for i, arg in enumerate(args):
                if i >= len(params):
                    break
                pname = params[i]
                traits = traits_map.get(pname, frozenset())
                if ProcArgTrait.VAR_READ not in traits and ProcArgTrait.VAR_WRITE not in traits:
                    continue
                # DYNAMIC_NAME_LOCAL refines VAR_READ: the param's
                # VALUE is used as a name for a CALLEE-LOCAL variable,
                # NOT as a caller-frame alias.  The caller's literal
                # arg ``x`` is NOT consumed by the callee's reference
                # (``$param`` inside the callee references whatever
                # the param's value names in the callee's own frame).
                # See ``test_FP_callbyname_scan_target_not_caller_alias``
                # -- D4-F4 / proc-arg-traits VAR_READ alongside fix.
                if ProcArgTrait.DYNAMIC_NAME_LOCAL in traits and (
                    ProcArgTrait.VAR_WRITE not in traits
                ):
                    continue
                if not arg or "$" in arg or "[" in arg:
                    continue
                if arg and all(ch.isalnum() or ch in "_:" for ch in arg):
                    by_name.add(arg)

    def _parse_subst(text: str) -> tuple[str, tuple[str, ...]] | None:
        """Parse a ``[cmd ...]`` body into (cmd, args)."""
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
            if tok.type is TokenType.VAR:
                piece = f"${tok.text}"
            elif tok.type is TokenType.CMD:
                piece = f"[{tok.text}]"
            else:
                piece = tok.text
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

    def _scan_value_text(text: str) -> None:
        """Find every ``[cmd ...]`` substitution in *text* and dispatch."""
        if not text or "[" not in text:
            return
        try:
            lexer = TclLexer(text)
        except TclParseError:
            return
        while True:
            try:
                tok = lexer.get_token()
            except TclParseError:
                break
            if tok is None:
                break
            if tok.type is TokenType.CMD:
                parsed = _parse_subst(tok.text)
                if parsed is not None:
                    cmd, args = parsed
                    _add_for_call(cmd, args)
                    for a in args:
                        _scan_value_text(a)

    for block in cfg.blocks.values():
        for stmt in block.statements:
            if isinstance(stmt, (IRCall, IRBarrier)):
                _add_for_call(stmt.command, stmt.args)
                for a in stmt.args:
                    _scan_value_text(a)
            elif isinstance(stmt, IRAssignValue):
                _scan_value_text(stmt.value)
            elif isinstance(stmt, IRAssignExpr):
                _scan_expr_node(stmt.expr, _scan_value_text)
            elif isinstance(stmt, (IRExprEval, IRReturn)):
                expr = getattr(stmt, "expr", None)
                if expr is not None:
                    _scan_expr_node(expr, _scan_value_text)
    return by_name


def _scan_expr_node(node, scan_text) -> None:
    """Walk an ``ExprNode`` and feed embedded command substitution text to
    ``scan_text`` so call-by-name args inside ``expr {[..]}`` are caught.
    """
    from compiler.expr_ast import (
        ExprBinary,
        ExprCall,
        ExprCommand,
        ExprTernary,
        ExprUnary,
    )

    if node is None:
        return
    if isinstance(node, ExprCommand):
        scan_text(node.text)
        return
    if isinstance(node, ExprBinary):
        _scan_expr_node(node.left, scan_text)
        _scan_expr_node(node.right, scan_text)
        return
    if isinstance(node, ExprUnary):
        _scan_expr_node(node.operand, scan_text)
        return
    if isinstance(node, ExprTernary):
        _scan_expr_node(node.condition, scan_text)
        _scan_expr_node(node.true_branch, scan_text)
        _scan_expr_node(node.false_branch, scan_text)
        return
    if isinstance(node, ExprCall):
        for arg in node.args:
            _scan_expr_node(arg, scan_text)


def build_proc_index_from_summaries(
    summaries: Mapping[str, object],
) -> dict[str, list[tuple[tuple[str, ...], dict[str, frozenset[ProcArgTrait]]]]]:
    """Build the ``proc_index`` for ``collect_call_by_name_reads`` from
    ``InterproceduralAnalysis.procedures`` (``ProcSummary``-valued).

    Keyed by bare name, qualified name, and lstripped qualified name so a
    bare call ``foo`` resolves to ``::foo``.
    """
    index: dict[str, list[tuple[tuple[str, ...], dict[str, frozenset[ProcArgTrait]]]]] = {}
    for qname, summary in summaries.items():
        traits = getattr(summary, "param_traits", None)
        params = getattr(summary, "params", None)
        if not traits or not params:
            continue
        entry = (tuple(params), dict(traits))
        bare = qname.rsplit("::", 1)[-1]
        for key in (bare, qname, qname.lstrip(":")):
            index.setdefault(key, []).append(entry)
    return index


def _handle_upvar(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
) -> None:
    """Process upvar to detect variable aliasing and mutation patterns.

    Marks the source param as VAR_READ initially.  If a write through
    the alias is later observed (``set local ...``), the alias tracking
    in ``_scan_commands`` upgrades it to VAR_WRITE.
    """
    start = 0
    level = "1"  # `upvar` default level is the immediate caller's frame
    if args and (args[0].isdigit() or args[0].startswith("#")):
        level = args[0]
        start = 1

    # A write through the alias is a write-back to the CALLER's variable named
    # by the param ONLY for level 1 (the default).  For level 0 (the proc's own
    # frame), ``#N`` (absolute/global), or 2+ (further up the stack), the
    # param's VALUE is used as a variable name in a NON-caller frame — verified
    # in tclsh: `proc w {p} {upvar 0 $p x; set x 1}` leaves the caller's `p`
    # untouched.  So such a param is READ (a name source), not a write-back of
    # the caller's passed variable; registering the alias would wrongly upgrade
    # it to VAR_WRITE and make the call site treat `$p` as a def, not a read
    # (false "unused variable"/dead-store).
    caller_frame = level == "1"

    i = start
    while i + 1 < len(args):
        other_var = args[i]
        my_var = args[i + 1]
        i += 2

        other_vn = _extract_var_name(other_var)
        my_vn = _extract_var_name(my_var)

        if other_vn and other_vn in param_set:
            # Start as VAR_READ; upgraded to VAR_WRITE if alias is written
            # (only when the alias targets the caller's frame).
            traits[other_vn].add(ProcArgTrait.VAR_READ)
            if caller_frame:
                upvar_aliases[my_var] = other_vn

        if my_vn and my_vn in param_set:
            traits[my_vn].add(ProcArgTrait.VAR_WRITE)


def _handle_foreach(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process foreach/lmap to detect loop list and body usage."""
    if len(args) < 3:
        return

    body_vn = _extract_var_name(args[-1])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)

    i = 0
    remaining = args[:-1]
    while i + 1 < len(remaining):
        list_vn = _extract_var_name(remaining[i + 1])
        if list_vn and list_vn in param_set:
            traits[list_vn].add(ProcArgTrait.LOOP_LIST)
        i += 2


def _handle_while(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``while {cond} body`` to detect EXPR and BODY traits."""
    if len(args) < 2:
        return

    # while $condParam $bodyParam
    cond_vn = _extract_var_name(args[0])
    if cond_vn and cond_vn in param_set:
        traits[cond_vn].add(ProcArgTrait.EXPR)

    body_vn = _extract_var_name(args[1])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)


def _handle_for(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``for {init} {cond} {next} body`` to detect EXPR and BODY traits."""
    if len(args) < 4:
        return

    # for $initParam $condParam $nextParam $bodyParam
    init_vn = _extract_var_name(args[0])
    if init_vn and init_vn in param_set:
        traits[init_vn].add(ProcArgTrait.BODY)

    cond_vn = _extract_var_name(args[1])
    if cond_vn and cond_vn in param_set:
        traits[cond_vn].add(ProcArgTrait.EXPR)

    next_vn = _extract_var_name(args[2])
    if next_vn and next_vn in param_set:
        traits[next_vn].add(ProcArgTrait.BODY)

    body_vn = _extract_var_name(args[3])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)


def _handle_after(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``after ms ?script?`` and ``after idle ?script?``.

    ``after cancel`` and ``after info`` do not take bodies.
    """
    if len(args) < 2:
        return
    # after cancel / after info — no body
    if args[0] in ("cancel", "info"):
        return
    # after ms script... / after idle script...
    # Script args are everything after the first arg (ms or "idle")
    # and optional -periodic flag (iRules extension to after).
    start = 1
    if start < len(args) and args[start] == "-periodic":
        start += 1
    for arg in args[start:]:
        vn = _extract_var_name(arg)
        if vn and vn in param_set:
            traits[vn].add(ProcArgTrait.EVAL)


def _handle_variadic_var_write(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    start: int,
) -> None:
    """Mark all args from *start* onwards as DYNAMIC_NAME_LOCAL if they
    reference params.

    Used for commands like ``scan`` (start=2) and ``lassign`` (start=1)
    where trailing arguments name CALLEE-LOCAL variables the command
    writes.  These writes happen in the callee's own frame -- they do
    NOT consume / alias the caller's variable unless the callee has
    set up an explicit ``upvar`` (which the ``_scan_commands`` path
    detects separately and emits as VAR_WRITE on the upvar-alias
    target, not the dynamic-name param).

    PR #498 / PR #499 deep-review follow-up finding 6: previously this
    handler emitted ``VAR_WRITE`` which the call-site call-by-name
    suppression treated as caller-frame consumption, falsely silencing
    O109/W211/W220 on the caller's literal arg.  Emitting
    ``DYNAMIC_NAME_LOCAL`` (the same trait the generic registry path
    emits for the same shape) makes the suppression honour the
    callee-local distinction.
    """
    for arg in args[start:]:
        vn = _extract_var_name(arg)
        if vn and vn in param_set:
            # See the corresponding note at the generic handler above:
            # DYNAMIC_NAME_LOCAL refines VAR_READ for callee-local
            # name use.  Both are emitted so consumers querying
            # VAR_READ-alone (broader "param is consumed at all" check)
            # still see this case.
            traits[vn].add(ProcArgTrait.DYNAMIC_NAME_LOCAL)
            traits[vn].add(ProcArgTrait.VAR_READ)


# Options that regexp/regsub accept before the positional arguments.
_REGEXP_SWITCHES = frozenset(
    {
        "-nocase",
        "-expanded",
        "-line",
        "-linestop",
        "-lineanchor",
        "-all",
        "-inline",
        "-indices",
        "--",
    }
)
_REGEXP_VALUE_SWITCHES = frozenset({"-start"})


def _skip_regexp_switches(args: list[str]) -> int:
    """Return the index of the first positional argument after switches."""
    i = 0
    while i < len(args):
        if args[i] == "--":
            return i + 1
        if args[i] in _REGEXP_SWITCHES:
            i += 1
        elif args[i] in _REGEXP_VALUE_SWITCHES:
            i += 2
        else:
            break
    return i


def _handle_regexp_vars(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``regexp ?switches? exp string ?matchVar ?subMatchVar ...??``."""
    pos = _skip_regexp_switches(args)
    # positional: exp(pos), string(pos+1), matchVar(pos+2), subMatchVar(pos+3)...
    var_start = pos + 2
    _handle_variadic_var_write(args, param_set, traits, start=var_start)


def _handle_regsub_var(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``regsub ?switches? exp string subSpec ?varName?``.

    The output ``varName`` is a CALLEE-LOCAL variable (the substitution
    result is written into the caller's frame only when ``upvar`` is
    in play, which the upvar-alias path tracks separately).  Emit
    ``DYNAMIC_NAME_LOCAL`` -- see ``_handle_variadic_var_write`` for
    the full rationale (PR #498 / PR #499 deep-review finding 6).
    """
    pos = _skip_regexp_switches(args)
    # positional: exp(pos), string(pos+1), subSpec(pos+2), varName(pos+3)
    var_idx = pos + 3
    if var_idx < len(args):
        vn = _extract_var_name(args[var_idx])
        if vn and vn in param_set:
            traits[vn].add(ProcArgTrait.DYNAMIC_NAME_LOCAL)
            traits[vn].add(ProcArgTrait.VAR_READ)
