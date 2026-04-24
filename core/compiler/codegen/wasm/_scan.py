"""Pre-codegen scan of the IR for runtime imports.

``_scan_needed_imports`` walks the CFG module and every IR statement,
classifies each command against the runtime-import tables in
``_imports.py``, and returns the set of runtime functions the emitter
must import. This keeps the emitter itself free of the noisy
command → import dispatch: it can assume every primitive it might call
is available.
"""

from __future__ import annotations

from ...cfg import CFGModule
from ...expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprTernary,
    ExprUnary,
    ExprVar,
)
from ...ir import (
    IRAssignConst,
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
    IRModule,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ._imports import (
    _OBJ_LIFECYCLE_IMPORTS,
    _SCOPE_NOP_COMMANDS,
    _STRING_IS_IMPORT,
    runtime_import_for,
    subcommand_runtime_import_for,
)
from ._parsing import _has_embedded_subst_scan, _parse_array_ref


def _scan_expr_body_imports(expr_text: str, needed: set[str]) -> None:
    """Parse an expression body and add any runtime imports it needs."""
    from ....parsing.expr_parser import parse_expr
    from ...expr_ast import BinOp

    try:
        node = parse_expr(expr_text)
    except Exception:
        return

    _ARITH_OPS = frozenset({BinOp.ADD, BinOp.SUB, BinOp.MUL, BinOp.DIV, BinOp.MOD})
    _ARITH_IMPORT = {
        BinOp.ADD: "tcl_arith_add",
        BinOp.SUB: "tcl_arith_sub",
        BinOp.MUL: "tcl_arith_mul",
        BinOp.DIV: "tcl_arith_div",
        BinOp.MOD: "tcl_arith_mod",
    }
    _MATH_FUNC_IMPORT = {
        "log": "tcl_math_log",
        "sqrt": "tcl_math_sqrt",
        "exp": "tcl_math_exp",
        "log10": "tcl_math_log10",
        "sin": "tcl_math_sin",
        "cos": "tcl_math_cos",
        "fabs": "tcl_math_fabs",
        "abs": "tcl_math_fabs",
        "double": "tcl_math_double",
        "float": "tcl_math_double",
        "round": "tcl_math_round",
        "int": "tcl_math_int",
        "entier": "tcl_math_int",
        "wide": "tcl_math_int",
    }

    def _walk(n: object) -> None:
        match n:
            case ExprBinary(op=op, left=left, right=right):
                if op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.STR_NE):
                    needed.add("tcl_string_equal")
                if op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
                    needed.add("tcl_string_compare")
                if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
                    needed.add("tcl_expr_order_cmp")
                if op in (BinOp.IN, BinOp.NI):
                    needed.add("tcl_list_contains")
                if op in _ARITH_OPS:
                    imp = _ARITH_IMPORT.get(op)
                    if imp:
                        needed.add(imp)
                    needed.add("tcl_obj_get_int")
                    needed.add("tcl_obj_new_int")
                    needed.add("tcl_obj_new_float")
                _walk(left)
                _walk(right)
            case ExprUnary(operand=operand):
                _walk(operand)
            case ExprTernary(cond=cond, true_expr=t, false_expr=f):
                _walk(cond)
                _walk(t)
                _walk(f)
            case ExprCall(function=func, args=args):
                imp = _MATH_FUNC_IMPORT.get(func)
                if imp:
                    needed.add(imp)
                    needed.add("tcl_obj_get_int")
                    needed.add("tcl_obj_new_int")
                    needed.add("tcl_obj_new_float")
                for arg in args:
                    _walk(arg)

    _walk(node)


def _scan_text_for_cmd_subst(text: str, needed: set[str]) -> None:
    """Scan a text value for [cmd ...] command substitutions and add imports."""
    i = 0
    while i < len(text):
        if text[i] == "[":
            depth = 1
            j = i + 1
            while j < len(text) and depth > 0:
                if text[j] == "[":
                    depth += 1
                elif text[j] == "]":
                    depth -= 1
                j += 1
            cmd_text = text[i + 1 : j - 1].strip()
            parts = cmd_text.split(None, 2)
            # Full split to count args when we need to decide whether a
            # multi-value linsert/lreplace shape is in play.
            full_parts = cmd_text.split()
            if parts:
                cmd = parts[0]
                rimp = runtime_import_for(cmd)
                if rimp is not None:
                    needed.add(rimp.import_key)
                    if cmd == "puts":
                        # ``puts -nonewline …`` dispatches to a
                        # second helper; include the import whenever
                        # ``puts`` appears so the dispatch path can
                        # pick it up.  Cheap (one import slot) and
                        # avoids a second scan.
                        needed.add("tcl_puts_nonewline")
                    elif cmd == "lreplace" and len(full_parts) > 5:
                        # ``[lreplace list first last v1 v2 …]`` —
                        # more than 4 args triggers the multi-value
                        # chain in ``_emit_cmd_runtime`` which needs
                        # ``tcl_list_insert`` to thread earlier values
                        # before the replaced slot.
                        needed.add("tcl_list_insert")
                elif cmd == "list":
                    # ``[list $a $b ...]`` with variable/command args uses
                    # tcl_lappend internally in _emit_list_value to properly
                    # quote each element.  Add it so the import is available.
                    needed.add("tcl_lappend")
                elif cmd == "dict" and len(parts) > 1:
                    subcmd = parts[1]
                    sri = subcommand_runtime_import_for("dict", subcmd)
                    if sri is not None:
                        needed.add(sri.import_key)
                    elif subcmd == "create":
                        # ``dict create`` with non-literal k/v args
                        # folds at runtime via ``tcl_lappend`` so each
                        # element is properly list-quoted (concat
                        # trims whitespace — wrong for dict values).
                        needed.add("tcl_lappend")
                    elif subcmd == "merge":
                        # ``dict merge`` is chained at the compiler
                        # level; the runtime helper takes pairs.
                        needed.add("tcl_dict_merge_pair")
                elif cmd == "string" and len(parts) > 1:
                    subcmd = parts[1]
                    sri = subcommand_runtime_import_for("string", subcmd)
                    if sri is not None:
                        needed.add(sri.import_key)
                    elif subcmd == "cat":
                        # ``string cat`` is variadic no-trim concat.
                        # Register ``tcl_append`` so the runtime path
                        # is always reachable.
                        needed.add("tcl_append")
                    elif subcmd == "is":
                        # "string is <class> <value>" — extract class name
                        rest = parts[2] if len(parts) > 2 else ""
                        class_parts = rest.split(None, 1)
                        if class_parts and class_parts[0] in _STRING_IS_IMPORT:
                            needed.add(_STRING_IS_IMPORT[class_parts[0]])
                elif cmd == "info" and len(parts) > 1:
                    subcmd = parts[1]
                    if subcmd == "exists":
                        needed.add("tcl_global_exists")
                        needed.add("tcl_array_exists")
                        needed.add("tcl_obj_get_int")
                        needed.add("tcl_obj_new_int")
                        # FRAME-tagged vars (per var-escape analysis)
                        # route through this runtime helper for
                        # literal-name existence checks.
                        needed.add("tcl_info_exists")
                        # ``info exists arr(key)`` inside a substitution —
                        # parts[2] (if present) is the var text.
                        if len(parts) > 2:
                            raw = parts[2]
                            if raw.startswith("${") and raw.endswith("}"):
                                raw = raw[2:-1]
                            elif raw.startswith("$"):
                                raw = raw[1:]
                            if _parse_array_ref(raw) is not None:
                                needed.add("tcl_array_element_exists")
                    if subcmd in ("body", "args", "exists"):
                        needed.add("tcl_info_dispatch")
                    elif subcmd == "level":
                        # Inline ``info level 0`` in a compiled proc
                        # builds a list by ``tcl_append``-ing the
                        # proc name + each param; pull in the helper
                        # so the codegen's concat chain can reach it.
                        needed.add("tcl_append")
                elif cmd == "lassign":
                    needed.add("tcl_list_index")
                    needed.add("tcl_list_tail")
                elif cmd == "clock" and len(parts) > 1:
                    sri = subcommand_runtime_import_for("clock", parts[1])
                    if sri is not None:
                        needed.add(sri.import_key)
                elif cmd == "uplevel":
                    needed.add("tcl_eval")
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                elif cmd == "array" and len(parts) > 1:
                    sub = parts[1]
                    if sub == "exists":
                        needed.add("tcl_array_exists")
                    elif sub == "size":
                        needed.add("tcl_array_size")
                    elif sub == "unset":
                        needed.add("tcl_array_unset")
                    elif sub == "names":
                        needed.add("tcl_array_names")
                        # ``array names arr $pat*`` interpolates the
                        # pattern; pull in ``tcl_append`` so the
                        # codegen's interpolation emitter can build
                        # the pattern string at runtime instead of
                        # degrading to a raw literal (which would
                        # disable the glob filter).
                        needed.add("tcl_append")
                    elif sub == "set":
                        needed.add("tcl_array_set")
                    elif sub == "get":
                        needed.add("tcl_array_get")
                elif cmd == "namespace" and len(parts) > 1 and parts[1] == "eval":
                    # ``[namespace eval ns arg1 arg2 ...]`` with dynamic
                    # script args: needs tcl_eval to run the assembled
                    # script and tcl_append to join multiple args.
                    needed.add("tcl_eval")
                    needed.add("tcl_append")
                elif cmd == "expr":
                    if len(parts) > 1:
                        # Extract the whole expr body (not using split, which
                        # breaks braced content like {"a" < "b"}).
                        rest = cmd_text[len(cmd) :].lstrip()
                        if rest.startswith("{") and rest.endswith("}"):
                            rest = rest[1:-1]
                        _scan_expr_body_imports(rest, needed)
                elif cmd == "catch":
                    # ``[catch {body} ?var?]`` as a command substitution
                    # compiles via the real catch codegen path (see
                    # ``_emit_command_subst``/``_emit_command_subst_value``);
                    # that path needs the catch runtime imports whose
                    # top-level scan only adds them when ``catch`` is a
                    # statement command.
                    needed.add("tcl_catch_enter")
                    needed.add("tcl_catch_leave")
                    needed.add("tcl_catch_result")
                    needed.add("tcl_catch_has_error")
                    needed.add("tcl_catch_set_ok_result")
                # Recurse into the command text for nested substitutions
                if len(parts) > 1:
                    _scan_text_for_cmd_subst(parts[-1], needed)
            i = j
        else:
            i += 1


def _scan_needed_imports(
    cfg_module: CFGModule,
    ir_module: IRModule,
) -> set[str]:
    """Pre-scan IR to find which runtime imports the compiled module needs."""
    needed: set[str] = set()

    def _scan_script(script: IRScript) -> None:
        for stmt in script.statements:
            _scan_stmt(stmt)

    def _scan_value(value: str, *, is_body_text: bool = False) -> None:
        """Scan a value string for embedded command substitutions.

        ``is_body_text=True`` means the string is source text that the
        lowerer will re-parse (e.g. a proc body) rather than a value
        that the codegen will emit at runtime.  In that case we skip
        the interpolated-string scan — otherwise a proc body containing
        ``$a`` / ``$b`` would spuriously pull in ``tcl_append``, since
        the body is never emitted via the concat-chain path.
        """
        if "[" in value:
            _scan_text_for_cmd_subst(value, needed)
        if is_body_text:
            return
        # Interpolated strings ("hello $x" / "$x[foo]" etc.) need tcl_append
        # at codegen time to concatenate the parts.
        if _has_embedded_subst_scan(value):
            needed.add("tcl_append")
        # ``$arr(key)`` references need tcl_array_get at codegen time.
        _scan_value_for_array_refs(value)
        # Pure ``$::ns::var`` / ``${::ns::var}`` references read through
        # the global table.
        if "::" in value and ("$" in value or value.startswith("::")):
            # Conservative: request global_get whenever the value might
            # resolve to a ``::``-qualified name.  False positives just
            # over-import; they don't break correctness.
            inner = value
            if inner.startswith("${") and inner.endswith("}"):
                inner = inner[2:-1]
            elif inner.startswith("$"):
                inner = inner[1:]
            if inner.startswith("::"):
                needed.add("tcl_global_get")

    def _scan_expr(expr: object) -> None:
        """Walk an expression AST and scan ExprCommand nodes."""

        match expr:
            case ExprCommand(text=text):
                _scan_text_for_cmd_subst(text, needed)
            case ExprVar(name=name, text=text):
                # ``$::ns::var`` in an expression reads from the global
                # table; ``$arr(key)`` reads via tcl_array_get.
                if name.startswith("::"):
                    needed.add("tcl_global_get")
                if "(" in text and text.endswith(")"):
                    needed.add("tcl_array_get")
            case ExprBinary(op=op, left=left, right=right):
                if op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.STR_NE):
                    needed.add("tcl_string_equal")
                if op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
                    needed.add("tcl_string_compare")
                if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
                    needed.add("tcl_expr_order_cmp")
                if op in (BinOp.ADD, BinOp.SUB, BinOp.MUL, BinOp.DIV, BinOp.MOD):
                    _ARITH_IMPORT2 = {
                        BinOp.ADD: "tcl_arith_add",
                        BinOp.SUB: "tcl_arith_sub",
                        BinOp.MUL: "tcl_arith_mul",
                        BinOp.DIV: "tcl_arith_div",
                        BinOp.MOD: "tcl_arith_mod",
                    }
                    imp2 = _ARITH_IMPORT2.get(op)
                    if imp2:
                        needed.add(imp2)
                    needed.add("tcl_obj_get_int")
                    needed.add("tcl_obj_new_int")
                    needed.add("tcl_obj_new_float")
                _scan_expr(left)
                _scan_expr(right)
            case ExprUnary(operand=operand):
                _scan_expr(operand)
            case ExprTernary(cond=cond, true_expr=t, false_expr=f):
                _scan_expr(cond)
                _scan_expr(t)
                _scan_expr(f)
            case ExprCall(function=func, args=args):
                _MATH_FUNC_IMPORT2 = {
                    "log": "tcl_math_log",
                    "sqrt": "tcl_math_sqrt",
                    "exp": "tcl_math_exp",
                    "log10": "tcl_math_log10",
                    "sin": "tcl_math_sin",
                    "cos": "tcl_math_cos",
                    "fabs": "tcl_math_fabs",
                    "abs": "tcl_math_fabs",
                    "double": "tcl_math_double",
                    "float": "tcl_math_double",
                    "round": "tcl_math_round",
                    "int": "tcl_math_int",
                    "entier": "tcl_math_int",
                    "wide": "tcl_math_int",
                }
                imp2 = _MATH_FUNC_IMPORT2.get(func)
                if imp2:
                    needed.add(imp2)
                    needed.add("tcl_obj_get_int")
                    needed.add("tcl_obj_new_int")
                    needed.add("tcl_obj_new_float")
                for arg in args:
                    _scan_expr(arg)

    def _scan_qualified_name(name: str | None) -> None:
        """``::``-prefixed variable names route reads/writes through the
        global table.  Pull in the matching runtime hooks eagerly so the
        code emission paths can rely on them being registered.

        Also handles array-element writes: ``set arr(key) val`` lowers
        with ``name='arr(key)'``, which needs ``tcl_array_set``.
        """
        if not name:
            return
        if _parse_array_ref(name) is not None:
            needed.add("tcl_array_set")
            needed.add("tcl_array_get")
            return
        if name.startswith("::"):
            needed.add("tcl_global_get")
            needed.add("tcl_global_set")

    def _scan_value_for_array_refs(value: str | None) -> None:
        """Pull in array-get for ``$arr(key)`` references inside a value
        string.  The read path routes through tcl_array_get whenever the
        name parses as an array reference.
        """
        if not value or "(" not in value:
            return
        # Conservative: scan for ``$name(`` or ``${name(`` substrings.
        i = 0
        n = len(value)
        while i < n:
            if value[i] == "$" and i + 1 < n:
                # Advance over bare name or ${...} with parens.
                j = i + 1
                if value[j] == "{":
                    # ``${arr(key)}`` — the lowerer emits this form
                    # when the surface syntax was a quoted-string
                    # ``$arr(key)``.  Check for ``(`` *inside* the
                    # braced content, not just after the closing
                    # brace.  Without this, the import-scan misses
                    # ``tcl_array_get`` and ``_emit_array_element_read``
                    # silently emits ``i32.const 0`` at run time —
                    # making every ``puts "v=$arr(key)"`` read come
                    # back as empty string.
                    inner_start = j + 1
                    k = inner_start
                    while k < n and value[k] != "}":
                        k += 1
                    # scan the inner for ``(``
                    ii = inner_start
                    while ii < k:
                        if value[ii] == "(":
                            needed.add("tcl_array_get")
                            break
                        ii += 1
                    else:
                        pass
                    if "tcl_array_get" in needed:
                        break
                    j = k
                else:
                    while j < n and (
                        value[j].isalnum()
                        or value[j] == "_"
                        or (value[j] == ":" and j + 1 < n and value[j + 1] == ":")
                    ):
                        j += 1
                if j < n and value[j] == "(":
                    needed.add("tcl_array_get")
                    break
                i = j
            else:
                i += 1

    def _scan_array_subcmd(args: tuple[str, ...]) -> None:
        if not args:
            return
        sub = args[0]
        if sub == "exists":
            needed.add("tcl_array_exists")
        elif sub == "size":
            needed.add("tcl_array_size")
        elif sub == "unset":
            needed.add("tcl_array_unset")
        elif sub == "names":
            needed.add("tcl_array_names")
            # ``array names arr ?pattern?`` — the pattern may be an
            # interpolated string (``$option*``); scan it so
            # ``tcl_append`` / ``tcl_array_get`` make it into the
            # module imports and the pattern is emitted literally,
            # not as a null TclObj that disables the filter.
            if len(args) >= 3:
                _scan_value(args[2])
        elif sub == "set":
            needed.add("tcl_array_set")

    def _scan_stmt(stmt: IRStatement) -> None:
        match stmt:
            case IRCall(command=command, args=args):
                if command == "lset":
                    # ``lset`` is emitter-resident, not in _CMD_RUNTIME
                    # (see the comment in _imports.py).  Register the
                    # runtime import here so the shared-imports table
                    # has ``tcl_list_set`` ready for ``_emit_cmd_lset``.
                    needed.add("tcl_list_set")
                    if len(args) > 3:
                        # Multi-index ``lset v i j k newval`` builds an
                        # indices TclObj by chaining ``tcl_list`` pairs
                        # in ``_emit_cmd_lset``.  Without this import,
                        # the emitter raises an internal error.
                        needed.add("tcl_list_create")
                rimp = runtime_import_for(command)
                if rimp is not None:
                    needed.add(rimp.import_key)
                    if command == "puts":
                        # Include the -nonewline helper alongside tcl_puts
                        # so the dispatch path can choose between them.
                        needed.add("tcl_puts_nonewline")
                    elif command == "lreplace" and len(args) > 4:
                        # Multi-value ``lreplace list first last v1 v2 …``
                        # chains successive ``tcl_list_insert`` calls at
                        # ``first`` to position earlier values before the
                        # one the base call replaced.  See
                        # ``_emit_cmd_runtime`` for the emit shape.
                        needed.add("tcl_list_insert")
                elif command == "string" and args and (
                    sri := subcommand_runtime_import_for("string", args[0])
                ) is not None:
                    needed.add(sri.import_key)
                elif command == "string" and args and args[0] == "is" and len(args) >= 3:
                    is_key = _STRING_IS_IMPORT.get(args[1])
                    if is_key:
                        needed.add(is_key)
                elif command == "string" and args and args[0] == "cat":
                    # ``string cat`` is variadic no-trim concat; the
                    # runtime path leans on ``tcl_append`` when any arg
                    # isn't a pure literal.
                    needed.add("tcl_append")
                elif command == "list":
                    # ``list $a $b ...`` with variable args uses tcl_lappend
                    # internally in _emit_list_value to quote each element.
                    needed.add("tcl_lappend")
                elif command == "dict" and args and (
                    sri := subcommand_runtime_import_for("dict", args[0])
                ) is not None:
                    needed.add(sri.import_key)
                elif command == "dict" and args and args[0] == "create":
                    # ``dict create`` with non-literal k/v args
                    # folds at runtime via ``tcl_lappend`` so
                    # values with whitespace / braces survive
                    # (concat would trim them).
                    needed.add("tcl_lappend")
                elif command == "dict" and args and args[0] == "merge":
                    needed.add("tcl_dict_merge_pair")
                elif command == "global":
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                elif command == "upvar":
                    # upvar #0 resolves to a global alias — reads/writes go
                    # through the global table. The target name is computed
                    # at runtime (may be an interpolated string).
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                elif command == "variable":
                    # variable inside a proc aliases local → ::ns::local in
                    # the enclosing namespace. Same runtime path as upvar #0.
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                    # Dynamic ``variable $name ?value?`` (tcltest's
                    # ``Default`` / ``Option``) needs runtime string
                    # concatenation + existence probe to build the
                    # qualified target and honour the Tcl semantics
                    # of initialising only when unset.
                    if args and (args[0].startswith("$") or args[0].startswith("[")):
                        needed.add("tcl_append")
                        needed.add("tcl_global_exists")
                        needed.add("tcl_obj_get_int")
                elif command == "info" and args:
                    # info exists — resolves via tcl_global_exists for
                    # aliased / ``::`` names; plain locals are boxed to
                    # TclObj via tcl_obj_new_int.  Other subcommands go
                    # through tcl_info_dispatch.
                    if args[0] == "exists":
                        needed.add("tcl_global_exists")
                        needed.add("tcl_array_exists")
                        needed.add("tcl_obj_get_int")
                        needed.add("tcl_obj_new_int")
                        # Literal-name existence in FRAME-tagged vars
                        # dispatches through the runtime helper.
                        needed.add("tcl_info_exists")
                        # ``info exists arr(key)`` — probe the array table.
                        if len(args) >= 2:
                            raw = args[1]
                            # ``info exists $dynamicName`` — dispatch
                            # to runtime ``info_exists`` which resolves
                            # the name's value at runtime.
                            if raw.startswith("$") or raw.startswith("["):
                                stripped = raw
                                if raw.startswith("${") and raw.endswith("}"):
                                    stripped = raw[2:-1]
                                elif raw.startswith("$"):
                                    stripped = raw[1:]
                                if _parse_array_ref(stripped) is None:
                                    needed.add("tcl_info_exists")
                            # Strip a ``$`` / ``${...}`` sigil if present.
                            if raw.startswith("${") and raw.endswith("}"):
                                raw = raw[2:-1]
                            elif raw.startswith("$"):
                                raw = raw[1:]
                            if _parse_array_ref(raw) is not None:
                                needed.add("tcl_array_element_exists")
                    if args[0] in ("body", "args", "exists"):
                        needed.add("tcl_info_dispatch")
                    elif args[0] == "level":
                        # Inline ``info level 0`` builds a list via
                        # the ``tcl_append`` concat-chain; see
                        # ``_emit_info_value``.
                        needed.add("tcl_append")
                elif command == "lassign":
                    needed.add("tcl_list_index")
                    needed.add("tcl_list_tail")
                elif command == "clock" and args:
                    sri = subcommand_runtime_import_for("clock", args[0])
                    if sri is not None:
                        needed.add(sri.import_key)
                elif command == "array" and args:
                    _scan_array_subcmd(args)
                elif command == "uplevel":
                    needed.add("tcl_eval")
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                    # Multi-word bodies concat with spaces; also each body
                    # part is run through _emit_value which may need
                    # tcl_append for interpolated strings.
                    for arg in args:
                        _scan_value(arg)
                    if len(args) > 2 or (
                        len(args) > 1
                        and not args[0].startswith(("#", "-"))
                        and not args[0].lstrip("-").isdigit()
                    ):
                        needed.add("tcl_append")
                elif command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                    # ``namespace eval ns arg1 arg2 ...`` with dynamic
                    # script args: evaluated through WASM, not fallback.
                    needed.add("tcl_eval")
                    needed.add("tcl_append")
                    # Scan each script arg as a runtime value (not body text)
                    # so array-element refs ($arr($key)) pull in tcl_array_get.
                    for a in args[2:]:
                        _scan_value(a)
                elif command == "unset":
                    for a in args:
                        if _parse_array_ref(a) is not None:
                            needed.add("tcl_array_unset_element")
                        else:
                            # Whole-variable unset: may need array_unset to
                            # clear an array table when the var is an alias.
                            needed.add("tcl_array_unset")
                elif command == "catch":
                    needed.add("tcl_catch_enter")
                    needed.add("tcl_catch_leave")
                    needed.add("tcl_catch_result")
                    needed.add("tcl_catch_has_error")
                    needed.add("tcl_catch_set_ok_result")
                # Scan all arguments for command substitutions.  For
                # body-taking commands (``proc``, ``namespace eval``,
                # ``catch``, etc.) the args are source text re-parsed
                # by the lowerer, not values emitted at runtime — mark
                # them so the scan doesn't over-import tcl_append.
                is_body = command in _SCOPE_NOP_COMMANDS or command == "catch"
                for arg in args:
                    _scan_value(arg, is_body_text=is_body)
            case IRAssignValue(name=name, value=value):
                _scan_qualified_name(name)
                _scan_value(value)
                _scan_value_for_array_refs(value)
            case IRAssignConst(name=name):
                _scan_qualified_name(name)
            case IRIncr(name=name, amount=amount):
                _scan_qualified_name(name)
                if isinstance(amount, str):
                    _scan_value(amount)
                    _scan_value_for_array_refs(amount)
            case IRExprEval(expr=expr):
                _scan_expr(expr)
            case IRAssignExpr(name=name, expr=expr):
                _scan_qualified_name(name)
                _scan_expr(expr)
            case IRReturn(value=value, expr=expr):
                if value is not None:
                    _scan_value(value)
                if expr is not None:
                    _scan_expr(expr)
            case IRBarrier(command=barrier_cmd, args=barrier_args):
                # Eval-fallback path always needs tcl_eval.
                needed.add("tcl_eval")
                if barrier_cmd == "uplevel":
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                    # The body may contain ``$var``/``[cmd]`` references
                    # that have to be resolved BEFORE eval — scan for
                    # interpolation helpers.
                    for arg in barrier_args:
                        _scan_value(arg)
                elif (
                    barrier_cmd == "namespace"
                    and barrier_args
                    and barrier_args[0] == "eval"
                    and len(barrier_args) > 2
                ):
                    # ``namespace eval ns arg1 arg2 ...`` with dynamic script
                    # args: emitted via WASM assembly (not fallback), so each
                    # script arg is evaluated through ``_emit_value`` and may
                    # need ``tcl_append``, ``tcl_array_get``, etc.
                    needed.add("tcl_append")
                    for a in barrier_args[2:]:
                        _scan_value(a)
                elif (
                    barrier_cmd == "return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # ``return -code error <msg>`` emits the message
                    # inline via ``_emit_value``; scan it so the
                    # interpolation helpers (``tcl_append``, maybe
                    # ``tcl_array_get`` for ``$arr(key)`` references)
                    # show up in the module's import list.
                    _scan_value(barrier_args[2])
            case IRBlock(body=body):
                # ``namespace eval`` body inlined; recurse into its
                # statements so any runtime helpers they need (e.g.
                # tcl_append for interpolated strings) show up in
                # the module's import list.
                _scan_script(body)
            case IRIf(clauses=clauses, else_body=else_body):
                for clause in clauses:
                    _scan_expr(clause.condition)
                    _scan_script(clause.body)
                if else_body:
                    _scan_script(else_body)
            case IRFor(init=init, condition=condition, body=body, next=next_s):
                _scan_script(init)
                _scan_expr(condition)
                _scan_script(body)
                _scan_script(next_s)
            case IRWhile(condition=condition, body=body):
                _scan_expr(condition)
                _scan_script(body)
            case IRForeach(body=body):
                needed.add("tcl_list_length")
                needed.add("tcl_list_index")
                _scan_script(body)
            case IRSwitch(subject=subject, arms=arms, default_body=default_body, mode=mode):
                if mode == "glob":
                    needed.add("tcl_string_match")
                else:
                    needed.add("tcl_string_equal")
                # The subject may be a command substitution or
                # interpolated string — scan it so the codegen's
                # ``_emit_value`` call in ``_emit_str_value``
                # (which now routes ``ExprRaw`` through the full
                # value-emitter) can reach ``tcl_append`` /
                # ``tcl_list_length`` / etc.  Without this the
                # interpolation emitter degrades to
                # ``_emit_obj_literal`` and the subject compares
                # against its raw source text rather than its
                # runtime value.
                _scan_value(subject)
                for arm in arms:
                    if arm.body:
                        _scan_script(arm.body)
                if default_body:
                    _scan_script(default_body)
            case IRCatch(body=body):
                needed.add("tcl_catch_enter")
                needed.add("tcl_catch_leave")
                needed.add("tcl_catch_result")
                needed.add("tcl_catch_has_error")
                needed.add("tcl_catch_set_ok_result")
                _scan_script(body)
            case IRTry(body=body, finally_body=finally_body):
                _scan_script(body)
                if finally_body:
                    _scan_script(finally_body)

    # Scan top-level
    _scan_script(ir_module.top_level)

    # Scan procedures
    for proc in ir_module.procedures.values():
        _scan_script(proc.body)

    # Scan methods
    if ir_module.methods:
        for method in ir_module.methods.values():
            _scan_script(method.body)

    # Always include TclObj lifecycle imports — every non-trivial module
    # needs them for value creation, storage, and arithmetic unboxing.
    needed.update(_OBJ_LIFECYCLE_IMPORTS)

    # Always include error and eval — error for traps, eval for
    # interpreter fallback on uncompiled commands.  tcl_diag_set is
    # also always imported so trap messages can be resolved against
    # the sidecar source-location map.  tcl_global_set / get cover
    # the top-level-var-mirror path in ``_emit_var_write_obj_impl``
    # that keeps ``::top`` writes visible to eval fallbacks.
    needed.add("tcl_error")
    needed.add("tcl_eval")
    needed.add("tcl_ns_set")
    needed.add("tcl_ns_restore")
    needed.add("tcl_diag_set")
    needed.add("tcl_global_set")
    needed.add("tcl_global_get")
    # Register each compiled proc by name so the interpreter's
    # host-bridge dispatch can find it when an interpreted caller
    # (a Tcl-source proc body walked by eval_script) invokes a
    # compiled helper.  The frame helpers are also pulled in so
    # compiled procs can push a frame + bind params (necessary for
    # eval-fallbacks inside a proc body to resolve ``$var`` via
    # ``var_resolve``).  Only needed when the module has any procs.
    if ir_module.procedures:
        needed.add("tcl_proc_register_compiled")
        needed.add("tcl_frame_push")
        needed.add("tcl_frame_pop")
        needed.add("tcl_frame_set_argv")
        needed.add("tcl_frame_get_argv")
        # Pending-argv0 ABI: the callee prologue reads this slot
        # (cleared on read) to pick up the invoked word recorded by
        # the caller immediately before the compiled ``call``.  The
        # set side is emitted per-call-site; always pulling in both
        # halves keeps host-bridge entry points working (the take
        # returns 0 and the prologue falls back to the qname tail).
        needed.add("tcl_frame_set_pending_argv0")
        needed.add("tcl_frame_take_pending_argv0")
        needed.add("tcl_local_set")
        needed.add("tcl_local_get")
        # tcl_list is used by the compiled-proc prologue to build
        # the invocation argv list, element by element, before
        # stashing it via frame_set_argv.
        needed.add("tcl_list_create")
        # Variadic (``args`` tail) procs need ``llength`` / ``lindex``
        # in the prologue so the argv-capture loop can expand the
        # packed ``args`` list into individual elements — otherwise
        # ``[llength [info level 0]]`` is off by one for
        # ``proc p {args} {}`` and collapses surplus words into one
        # element for ``proc p {x args} {}``.  Only add when at least
        # one proc in the module has an ``args`` tail; pure-arith
        # fixtures stay minimal.
        for proc in ir_module.procedures.values():
            if proc.params and proc.params[-1] == "args":
                needed.add("tcl_list_length")
                needed.add("tcl_list_index")
                break

    return needed
