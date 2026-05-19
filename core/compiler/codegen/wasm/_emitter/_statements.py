"""_WasmEmitterStmtMixin: statement dispatch and proc calls."""

# canonicalisation: audited #246

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....commands.registry import REGISTRY as _REGISTRY
from .....commands.registry import EmitContext
from .....parsing.substitution import backslash_subst as _tcl_backslash_subst
from .....parsing.tokens import TokenType
from ....ir import (
    CommandTokens,
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
    IRInterpBoundary,
    IRReturn,
    IRStatement,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)
from .._encoding import (
    _tcl_list_quote,
)
from .._imports import (
    command_emits_nothing,
    runtime_import_for,
)
from .._ir import (
    DiagSite,
    ValType,
    WasmOp,
)
from .._ownership import Ownership


def _escape_dquote(text: str) -> str:
    """Escape a multi-token IR word for embedding inside a ``"…"`` literal.

    The IR mirrors the source byte stream — backslash sequences such as
    ``\\n`` or ``\\$`` are stored as their two-character source form,
    not as the substituted byte.  Tcl's in-quote backslash substitution
    will re-evaluate them to the original byte values when the wrapped
    script reaches the interpreter, so we leave the backslashes alone.

    Only the closing ``"`` needs escaping so the dquoted span ends at
    the trailing quote we add — not at the first embedded one.  An
    embedded ``"`` is preceded by a backslash unless it is already
    escaped (``\\"`` → leave alone).
    """
    out: list[str] = []
    i = 0
    while i < len(text):
        c = text[i]
        if c == "\\" and i + 1 < len(text):
            # Pass through the backslash + next char as-is so Tcl's
            # in-quote substitution resolves them at eval time.
            out.append(c)
            out.append(text[i + 1])
            i += 2
            continue
        if c == '"':
            out.append("\\")
            out.append('"')
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def _static_parse_error_message(
    barrier_cmd: str | None,
    reason: str | None,
    args: tuple[str, ...],
) -> str | None:
    """Map an :class:`IRBarrier` *reason* to a canonical Tcl runtime error
    message, or return ``None`` when the barrier is not a static parse
    error the codegen can replay verbatim.

    The lowering pass already detects malformed ``if`` shapes and emits an
    ``IRBarrier`` whose ``reason`` describes the static problem (see
    ``_lower_if`` in ``core/compiler/lowering.py``).  The Python VM and C
    Tcl both raise specific ``wrong # args …`` messages for these shapes
    *before* evaluating any branch; the WASM backend used to send the
    script back through ``tcl_eval``, where the runtime parser silently
    accepted the bad input (e.g. ``if {…} {…} else {…} elseif {…} {…}``
    discarded the trailing ``elseif`` clause instead of erroring).  We
    convert the barrier into a direct ``tcl_cmd_error`` call so the WASM
    backend reports the same diagnostic.

    The mapping is deliberately narrow.  Several lowering reasons (e.g.
    ``"malformed if clause"`` for ``if cond body extra``, or
    ``"malformed if"`` for ``if elseif``) cover *multiple* underlying
    shapes whose canonical Tcl messages cannot be reconstructed from the
    barrier alone — the runtime ``eval_if`` already errors on those via
    the expression parser, so the eval-fallback path produces a
    comparable error_status without our help.  We only special-case the
    reasons whose runtime fallback would silently accept (the bug from
    issue #259) AND whose canonical message is unambiguous from the
    barrier metadata.
    """
    if barrier_cmd != "::if" or reason is None:
        return None
    if reason == 'extra words after "else" clause':
        # ``if cond body else other extra…`` — eval_if otherwise discards
        # the trailing words.
        return 'wrong # args: extra words after "else" clause in "if" command'
    if reason == "malformed if else clause":
        # ``if … else`` with no body — eval_if otherwise drops into the
        # else branch with no script and silently returns success.
        return 'wrong # args: no script following "else" argument'
    if reason == "malformed if" and not args:
        # ``if`` with no arguments at all — eval_if's main loop never
        # executes (i=1 ≥ words.len=1) and silently returns 0.  Other
        # ``"malformed if"`` shapes — e.g. ``if elseif``, ``if then``,
        # ``if elseif elseif`` — are degenerate keyword-only shapes
        # whose canonical Tcl message depends on arglist length and
        # which keyword the VM's pre-validate hits first.  We leave
        # them on the eval-fallback path (which silently accepts) for
        # now; reporting a wrong but specific message would be worse
        # than the existing silent-accept divergence.  Issue #259 only
        # requires fixing the ``extra words after "else"`` shape.
        return 'wrong # args: no expression after "if" argument'
    return None


class _WasmEmitterStmtMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_args_list(self, *a: Any, **kw: Any) -> Any: ...
        def _intern_string(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_read_obj_lenient(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_sync(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_interp_boundary(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_readback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_namespace_eval_bridge(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterExprMixin
        def _emit_expr(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_expr_obj(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCtrlMixin
        def _emit_if(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_for(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_while(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_foreach(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_switch(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_catch(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_try(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_catch_from_args(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCmdMixin
        def _emit_cmd_return(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_proc_call(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_runtime(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_uplevel(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_upvar(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_variable(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_lassign(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_info_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_clock_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_array_subcmd_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_signal_check_and_return(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_error_flag_check_and_return(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_compiled_call_with_bridge(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_frame_set_line(self, stmt: IRStatement) -> None:
        """Phase 8 follow-up: stamp ``frame_set_line`` for *stmt* so
        ``info frame -line`` inside a callee body reports the line of
        the current statement.  Centralised so every emit entry point
        (``_emit_stmt``, the implicit-return tail in ``_emit_block``)
        stamps consistently.
        """
        if not self._is_proc:
            return
        set_line_idx = self._shared_imports.get("tcl_frame_set_line")
        if set_line_idx is None:
            return
        rng = getattr(stmt, "range", None)
        if rng is None:
            return
        line_one_based = rng.start.line + 1
        if line_one_based <= 0:
            return
        self._emit_i32_const(line_one_based)
        self._emit_call(set_line_idx)

    def _emit_stmt(self, stmt: IRStatement) -> None:
        """Emit WASM for a single IR statement."""
        # Record this statement's source location so any nested trap
        # emission (eval fallback, unsupported-command trap) can stamp
        # a sidecar diag site with a meaningful file:line:col.
        self._record_stmt_context(stmt)
        # Phase 8 follow-up: stamp ``frame_set_line`` per-statement
        # so ``info frame -line`` returns the source line of the
        # currently-executing command — covers IRAssignValue /
        # IRAssignExpr / IRCall alike.  Compile-time constant; the
        # runtime hit is one i32 immediate plus one call per
        # statement.  Skipped at top level (no proc frame to stamp)
        # and when the setter isn't imported.
        self._emit_frame_set_line(stmt)
        # Centralised interp-boundary dispatch: an IRInterpBoundary
        # node is a structural sync point — codegen translates it to
        # the same ``_emit_interp_boundary`` helper the eval/dispatch
        # call sites already go through.  Lowering inserts these
        # nodes immediately before any statement that hands control
        # to the interpreter, so the sync becomes part of the IR
        # rather than a method call codegen has to remember.
        if isinstance(stmt, IRInterpBoundary):
            vars_observed = set(stmt.vars_observed) if stmt.vars_observed is not None else None
            self._emit_interp_boundary(stmt.kind, vars_observed=vars_observed)
            return
        match stmt:
            case IRAssignConst(name=name, value=value):
                self._emit_obj_literal(value)
                # ``_emit_obj_literal`` always allocates a fresh
                # TclObj → OWNED.  S2.3 migration.
                self._emit_var_write_obj(name, source=Ownership.OWNED)
                if self._optimise and name not in self._aliases:
                    # Aliased writes go to a global under a possibly-dynamic
                    # name — constant-tracking the local Tcl name would
                    # short-circuit reads that ought to see cross-proc
                    # updates to the global.
                    try:
                        self._const_map[name] = int(value)
                    except ValueError:
                        self._const_map.pop(name, None)

            case IRAssignExpr(name=name, expr=expr):
                from ....expr_ast import ExprVar as _ExprVar

                self._emit_expr_obj(expr)
                # Tcl 9 ``set r [expr $v]`` semantics: substitute ``$v``
                # then re-parse its string repr as an expression.
                # Mirror the same canonicalisation the control-flow
                # emitter applies to the var-only RHS shape.
                if isinstance(expr, _ExprVar):
                    canon_idx = self._shared_imports.get("tcl_expr_canonicalise")
                    if canon_idx is not None:
                        self._emit_call(canon_idx)
                # ``_emit_expr_obj`` allocates a fresh boxed result
                # in every code path → OWNED.  S2.3 migration.
                self._emit_var_write_obj(name, source=Ownership.OWNED)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRAssignValue(name=name, value=value):
                # value_needs_backsubst is already handled by _emit_value's
                # general backslash substitution path for non-braced literals.
                source = self._emit_value(value)
                # ``_emit_value`` returns Ownership describing the
                # stack value; threads it through so the wrap picks
                # the right retain-vs-transfer behaviour.
                self._emit_var_write_obj(name, source=source)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRIncr(name=name, amount=amount):
                # Issue #262: route through the runtime helper so the
                # strict-integer guard fires on the current value
                # (a float string like ``"52.60"`` raises ``expected
                # integer but got "52.60"`` rather than silently
                # truncating to ``52`` and advancing the counter).
                # Falls back to the inline integer add path when the
                # runtime helper isn't registered (older compile
                # configs without the import).
                incr_idx = self._shared_imports.get("tcl_incr")
                if incr_idx is not None:
                    # Lenient read so an unset scalar initialises to 0
                    # — Tcl 8.5+ ``incr x`` (with x unset) returns 1,
                    # doesn't raise ``no such variable``.  ``tcl_incr``
                    # treats the null TclObj on a 0 initialisation.
                    self._emit_var_read_obj_lenient(name)  # current value (TclObj)
                    if amount is None:
                        self._emit_i64_const(1)
                        self._emit_box_int()
                    else:
                        try:
                            self._emit_i64_const(int(amount))
                            self._emit_box_int()
                        except ValueError:
                            self._emit_value(amount)  # variable amount
                    self._emit_call(incr_idx)  # → new TclObj
                    # ``tcl_incr`` returns OWNED.
                    self._emit_var_write_obj(name, source=Ownership.OWNED)
                    if self._optimise:
                        self._const_map.pop(name, None)
                    return
                self._emit_var_read_obj(name)
                self._emit_unbox_int()
                amt = 1
                if amount is not None:
                    try:
                        amt = int(amount)
                    except ValueError:
                        # Variable increment — unbox the amount variable
                        self._emit_value(amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        # ``_emit_box_int`` allocates a fresh int
                        # obj → OWNED.
                        self._emit_var_write_obj(name, source=Ownership.OWNED)
                        if self._optimise:
                            self._const_map.pop(name, None)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                # ``_emit_box_int`` returns OWNED.
                self._emit_var_write_obj(name, source=Ownership.OWNED)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit(WasmOp.DROP)

            case IRCall(args=args, defs=defs, tokens=tokens) as ir_call:
                # Bind both forms: ``command`` (source spelling, what the
                # user wrote) flows into ``_emit_eval_fallback`` and
                # proc resolution so Tcl's normal namespace-walk lookup
                # finds the same target it would at runtime — passing
                # the eagerly-globalised canonical form would force
                # ``::name`` resolution and miss namespace-local procs.
                # ``canonical_command`` (post alias / namespace-import
                # / rename resolution) is the dispatch key for
                # literal-string matches against builtin spellings.
                # See issue #246.
                command = ir_call.command
                canonical = ir_call.canonical_command
                if (
                    tokens is not None
                    and tokens.expand_word is not None
                    and any(tokens.expand_word)
                    and tokens.argv_texts
                ):
                    # ``{*}`` argument expansion — reconstruct the
                    # original command text with ``{*}`` prefixes in
                    # front of each expanded word so the runtime can
                    # handle the expansion.  ``argv_texts`` carries
                    # the BRACE-STRIPPED contents for ``{a b c}``-
                    # style words (``_word_piece`` returns
                    # ``tok.text`` which excludes the outer braces),
                    # so the reconstruction has to re-wrap any STR-
                    # type expanded word — without the wrap, a
                    # multi-line ``{*}{ a b c }`` table flattens to
                    # ``{*} a b c`` and the parser sees ``{*}`` as a
                    # literal three-byte word, dispatching the
                    # callee with ``{*}`` and unexpanded contents.
                    ew = tokens.expand_word
                    argv = tokens.argv if tokens.argv is not None else ()
                    single_word = (
                        tokens.single_token_word if tokens.single_token_word is not None else ()
                    )
                    parts = []
                    for i, t in enumerate(tokens.argv_texts):
                        expand = i < len(ew) and ew[i]
                        prefix = "{*}" if expand else ""
                        # Source-faithful wrap for braced words —
                        # ``_word_piece`` already wraps VAR / CMD
                        # tokens in ``${...}`` / ``[...]``, but STR
                        # tokens (brace-quoted literals) are returned
                        # bare.  Add the braces back so the
                        # reconstructed script re-parses to the same
                        # word shape the user wrote.
                        from .....parsing.tokens import TokenType

                        if i < len(argv) and argv[i] is not None and argv[i].type is TokenType.STR:
                            t = "{" + t + "}"
                        elif (
                            i > 0
                            and t
                            and i < len(argv)
                            and argv[i] is not None
                            and argv[i].type is TokenType.ESC
                            and (i >= len(single_word) or single_word[i])
                            and any(
                                c in t
                                for c in (" ", "\t", "\n", "\r", ";", "$", "[", "{", "}", '"', "\\")
                            )
                        ):
                            # ESC single-token arg from a quoted /
                            # backslash-bearing word.  See the
                            # IRBarrier branch above for the
                            # ``namespace {*}"…\n…"`` repro.
                            t = _tcl_list_quote(t, first=False)
                        elif i < len(single_word) and not single_word[i] and t:
                            # Multi-token concatenation (``"$body (suffix)"``,
                            # ``foo$bar``, …).  ``_word_piece`` joined the
                            # piece texts with no separators, so the result
                            # is a substitution-bearing string that would
                            # split into multiple words at re-parse time
                            # if left bare.  Re-wrap in ``"…"`` so it stays
                            # one word; embedded ``$`` / ``[`` / ``]`` keep
                            # their substitution semantics inside ``"…"``,
                            # while ``\`` / ``"`` need escaping so the
                            # quoted span terminates only at the trailing
                            # quote we add here.
                            # Only escape unescaped ``"`` — backslash
                            # sequences in the IR mirror the source
                            # (``\n`` is two literal chars), and Tcl's
                            # backslash substitution inside ``"…"`` will
                            # re-evaluate them to the same byte values
                            # the original word produced (``\n`` →
                            # newline).  Doubling the backslashes here
                            # would change ``\n`` to a literal ``\n``
                            # pair instead of a newline.
                            t = '"' + _escape_dquote(t) + '"'
                        parts.append(prefix + t)
                    script = " ".join(parts)
                    self._emit_eval_fallback(command, args, script_override=script)
                    self._emit(WasmOp.DROP)
                    if self._optimise:
                        self._const_map.clear()
                else:
                    self._emit_call_stmt(
                        command, args, defs, tokens=tokens, canonical_command=canonical
                    )

            case IRReturn(value=value, expr=expr):
                # Inside a compile-time ``catch`` body, ``return`` must
                # not emit a WASM ``return`` — that would unwind past
                # ``catch_leave`` and exit the surrounding proc with
                # the body's value instead of letting ``catch`` absorb
                # it as TCL_RETURN (return code 2).  Route through the
                # runtime ``tcl_return_set`` helper instead, which sets
                # ``return_flag`` + ``return_val``; ``catch_has_error``
                # then short-circuits the body and ``catch_leave``
                # reads the flag and reports TCL_RETURN.
                return_set = self._shared_imports.get("tcl_return_set")
                if self._catch_depth > 0 and return_set is not None:
                    if expr is not None:
                        self._emit_expr_obj(expr)
                    elif value is not None:
                        self._emit_value(value)
                    else:
                        self._emit_i32_const(0)
                    self._emit_call(return_set)
                    return
                if expr is not None:
                    self._emit_expr_obj(expr)
                elif value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i32_const(0)
                self._emit(WasmOp.RETURN)

            case IRBarrier(
                canonical_command=barrier_cmd,
                args=barrier_args,
                reason=reason,
                tokens=barrier_tokens,
            ):
                # Barriers are dynamic commands (eval, uplevel, trace,
                # etc.) that defeat static *analysis* but may still
                # have concrete runtime implementations we can call
                # directly.  Dispatch in priority order:
                #   0. Static parse errors detected by the lowering (e.g.
                #      ``if {…} else {…} elseif {…}``) are replayed as a
                #      direct ``tcl_cmd_error`` so the WASM backend
                #      surfaces the same ``wrong # args …`` message as
                #      the VM and C Tcl.  Without this the eval-fallback
                #      path would send the malformed script back through
                #      the runtime parser, which silently accepts shapes
                #      Tcl is supposed to reject (see #259).
                #   1. ``uplevel`` has a dedicated emitter that shifts
                #      the frame-depth around a tcl_eval call so the
                #      body runs at a caller's scope.
                #   2. Commands in ``_CMD_RUNTIME`` have real (or stub-
                #      trapping) runtime fns — dispatch through that
                #      table so ``trace add variable …`` becomes a
                #      direct ``tcl_trace`` call with precise diag
                #      attribution instead of a generic tcl_eval
                #      fallback that would lose arity info.
                #   3. Anything else really is a black-box barrier:
                #      fall back to the interpreter.
                parse_error_msg = _static_parse_error_message(
                    barrier_cmd, reason, tuple(barrier_args)
                )
                if parse_error_msg is not None:
                    self._emit_static_parse_error_trap(barrier_cmd or "", parse_error_msg)
                    if self._optimise:
                        self._const_map.clear()
                elif barrier_cmd == "::uplevel" and barrier_args:
                    self._emit_cmd_uplevel(barrier_args)
                    self._emit(WasmOp.DROP)
                elif (
                    barrier_cmd == "::return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # ``return -code error <msg>`` — evaluate the
                    # message value inline so embedded ``$var`` /
                    # ``[cmd]`` substitutions resolve against the
                    # current frame, then call ``tcl_cmd_error``.
                    # Going through the eval fallback would
                    # brace-wrap the message and block the
                    # substitutions the error text needs (tcltest's
                    # error strings embed ``$option``/``$values``
                    # everywhere).  Other ``return -code …`` forms
                    # (dynamic code, break/continue, numeric) reach
                    # the fallback below where quoting is safe.
                    self._emit_cmd_return(barrier_args)
                elif barrier_cmd and runtime_import_for(barrier_cmd) is not None:
                    # Prefer the specialised registry hook (append,
                    # lappend, puts, format, …) so the barrier path
                    # picks up the same specialisations as the main
                    # dispatch.  Fall through to the generic runtime
                    # call when no hook is registered.
                    hook = _REGISTRY.get_wasm_hook(barrier_cmd)
                    if hook is None or not hook(self, barrier_args, (), EmitContext.STATEMENT):
                        self._emit_cmd_runtime(barrier_cmd, barrier_args, ())
                elif barrier_cmd:
                    # ``catch $dyn_body result ?opts?`` — result and
                    # options variables are written by the runtime
                    # eval_catch but are not visible to the lowering
                    # pass (no static IRCatch.result_var).  Pre-intern
                    # them so _emit_frame_readback reloads them after
                    # the eval_script call; without this the compiled
                    # proc reads a zero-initialised WASM local and
                    # raises ``can't read "res": no such variable``.
                    _barrier_needs_drop = True
                    if barrier_cmd in ("catch", "::catch"):
                        for vname in barrier_args[1:3]:
                            if vname and not vname.startswith("$") and not vname.startswith("["):
                                self._intern_local(vname)
                        # Try the AOT catch hook for static bodies (bare
                        # words and braced scripts).  Dynamic bodies
                        # ($var, [cmd]) are filtered inside the hook and
                        # fall through to eval_fallback.
                        hook = _REGISTRY.get_wasm_hook("::catch")
                        if hook is not None and hook(self, barrier_args, (), EmitContext.STATEMENT):
                            # AOT path — catch_enter/leave handle everything;
                            # no i32 return value left on the WASM stack.
                            _barrier_needs_drop = False
                        else:
                            self._emit_eval_fallback(
                                barrier_cmd, barrier_args, tokens=barrier_tokens
                            )
                    # ``while`` / ``for`` / ``if`` barriers carry their
                    # condition / scripts as plain ``args`` strings
                    # rather than tokens.  The default eval-fallback
                    # quoting heuristic treats a leading ``[`` as a
                    # command-substitution word, which gets evaluated
                    # ONCE by the runtime parser instead of staying as
                    # the cond expression — turning
                    # ``while [string length $s] { ... }`` into
                    # ``while CONST_LEN { ... }`` that loops forever.
                    # Build a script with the cond/scripts always
                    # brace-wrapped so the runtime sees the literal
                    # expression / body.
                    elif barrier_cmd == "::while" and len(barrier_args) >= 2:
                        cond, body = barrier_args[0], barrier_args[1]
                        script = (
                            "while "
                            + (
                                "{" + cond + "}"
                                if not (cond.startswith("{") and cond.endswith("}"))
                                else cond
                            )
                            + " "
                            + (
                                "{" + body + "}"
                                if not (body.startswith("{") and body.endswith("}"))
                                else body
                            )
                        )
                        self._emit_eval_fallback(barrier_cmd, barrier_args, script_override=script)
                    elif barrier_cmd == "::for" and len(barrier_args) >= 4:
                        init, cond, nxt, body = (
                            barrier_args[0],
                            barrier_args[1],
                            barrier_args[2],
                            barrier_args[3],
                        )

                        def _br(s):
                            return s if (s.startswith("{") and s.endswith("}")) else "{" + s + "}"

                        script = (
                            "for " + _br(init) + " " + _br(cond) + " " + _br(nxt) + " " + _br(body)
                        )
                        self._emit_eval_fallback(barrier_cmd, barrier_args, script_override=script)
                    else:
                        # Pass the barrier's tokens through so the
                        # eval-fallback's quoting heuristics can detect
                        # multi-token / braced words.  Without this an
                        # ``"$body (suffix)"`` arg lowered through a
                        # barrier (e.g. namespace-imported user proc
                        # with ``{*}`` expansion) was passed verbatim
                        # to the interpreter, splitting at the embedded
                        # space and breaking the receiver's arg count.
                        # Threading ``tokens=`` through the simple-case
                        # branch also preserves source-level brace /
                        # quote framing on each arg — without it a
                        # braced description such as ``{[Bug NNN]: x}``
                        # reassembles unbraced and the runtime parser
                        # substitutes the bracketed inner as a command,
                        # which was the original ``compExpr-7.2`` trap.
                        if (
                            barrier_tokens is not None
                            and barrier_tokens.expand_word is not None
                            and any(barrier_tokens.expand_word)
                            and barrier_tokens.argv_texts
                        ):
                            ew = barrier_tokens.expand_word
                            single_word = (
                                barrier_tokens.single_token_word
                                if barrier_tokens.single_token_word is not None
                                else ()
                            )
                            argv = barrier_tokens.argv if barrier_tokens.argv is not None else ()
                            from .....parsing.tokens import TokenType

                            parts: list[str] = []
                            for i, t in enumerate(barrier_tokens.argv_texts):
                                prefix = "{*}" if (i < len(ew) and ew[i]) else ""
                                if (
                                    i < len(argv)
                                    and argv[i] is not None
                                    and argv[i].type is TokenType.STR
                                ):
                                    t = "{" + t + "}"
                                elif i < len(single_word) and not single_word[i] and t:
                                    t = '"' + _escape_dquote(t) + '"'
                                elif (
                                    i > 0
                                    and t
                                    and i < len(argv)
                                    and argv[i] is not None
                                    and argv[i].type is TokenType.ESC
                                    and any(
                                        c in t
                                        for c in (
                                            " ",
                                            "\t",
                                            "\n",
                                            "\r",
                                            ";",
                                            "$",
                                            "[",
                                            "{",
                                            "}",
                                            '"',
                                            "\\",
                                        )
                                    )
                                ):
                                    # ESC arg from a single-token quoted /
                                    # backslash-bearing word.  Source-level
                                    # ``{*}"..."`` lowered to the IR
                                    # without the outer quotes; if we
                                    # re-emit it bare here, embedded
                                    # whitespace / newlines split it into
                                    # multiple words at re-parse time and
                                    # ``namespace {*}"…\n…\n…"`` becomes
                                    # several stray commands.  Round-trip
                                    # through Tcl's list-quote so the
                                    # interpreter sees one word and the
                                    # ``{*}`` expansion list-parses the
                                    # content.  Skipped for argv[0] (the
                                    # command name itself, which list-quote
                                    # would force-brace despite never
                                    # needing the wrap).
                                    t = _tcl_list_quote(t, first=False)
                                parts.append(prefix + t)
                            script = " ".join(parts)
                            self._emit_eval_fallback(
                                barrier_cmd, barrier_args, script_override=script
                            )
                        else:
                            self._emit_eval_fallback(
                                barrier_cmd, barrier_args, tokens=barrier_tokens
                            )
                    if _barrier_needs_drop:
                        self._emit(WasmOp.DROP)
                else:
                    self._emit_eval_fallback(reason)
                    self._emit(WasmOp.DROP)
                if self._optimise:
                    self._const_map.clear()

            case IRBlock(body=body, namespace=block_ns):
                # ``namespace eval`` body inlined into the enclosing
                # script.  Procs inside were already lifted to
                # ``module.procedures`` with qualified names; the
                # remaining statements run as plain code.  We stash
                # the block's namespace so ``_emit_call_stmt`` can
                # resolve unqualified command names (``Option …`` →
                # ``::tcltest::Option``) for the duration of the body.
                #
                # We also push the block namespace onto the runtime
                # ``ns_current`` via ``tcl_ns_set`` so that any
                # eval-fallback inside the body (``tcl_eval("bar")``)
                # performs its ``proc_lookup`` with the correct
                # context namespace.  Without this, under
                # ``full_flush`` (``interp create``/``eval``/``delete``)
                # every unqualified call inside ``namespace eval ::foo
                # { … }`` routes through ``tcl_eval`` with
                # ``ns_current() == ::`` and fails to find
                # ``::foo::bar`` — the concrete symptom is the
                # ``ArrayDefault`` trap when sourcing tcltest.tcl.
                prev_ns = self._block_namespace
                self._block_namespace = block_ns
                ns_saved_local: int | None = None
                ns_set_idx = self._shared_imports.get("tcl_ns_set")
                ns_restore_idx = self._shared_imports.get("tcl_ns_restore")
                should_wrap_runtime_ns = (
                    ns_set_idx is not None
                    and ns_restore_idx is not None
                    and block_ns is not None
                    and block_ns != "::"
                )
                if should_wrap_runtime_ns:
                    ns_saved_local = self._add_extra_local(
                        prefix="_block_ns_saved", val_type=ValType.I64
                    )
                    ns_literal = block_ns
                    offset = self._intern_string(ns_literal)
                    encoded = ns_literal.encode("utf-8", errors="surrogatepass")
                    # _intern_string stores a 4-byte length prefix
                    # before the bytes at ``offset`` — the
                    # pointer the runtime wants is ``offset + 4``.
                    assert ns_set_idx is not None  # for type-checker
                    self._emit_i32_const(offset + 4)
                    self._emit_i32_const(len(encoded))
                    self._emit_call(ns_set_idx)
                    self._emit_local_set(ns_saved_local)
                try:
                    for stmt in body.statements:
                        self._emit_stmt(stmt)
                finally:
                    self._block_namespace = prev_ns
                if should_wrap_runtime_ns and ns_saved_local is not None:
                    assert ns_restore_idx is not None  # for type-checker
                    self._emit_local_get(ns_saved_local)
                    self._emit_call(ns_restore_idx)
                if self._optimise:
                    self._const_map.clear()

            case IRUpFrame(frame_shift=shift, body=_body, source_tokens=src_toks):
                # ``uplevel ?level? {static-body}`` — barrier-relaxed
                # form.  At runtime we still route the body through
                # ``tcl_eval`` with the frame-depth stash/restore
                # wrapper, matching ``_emit_cmd_uplevel``'s semantics.
                # Inline-IR execution in the compiled callee's frame
                # is not yet correct because the WASM compile-to-
                # compile call chain does not push runtime frames —
                # caller WASM-locals are invisible to the runtime
                # frame table regardless of what shift we apply.  A
                # follow-up pass (whole-callee ``uplevel``-passthrough
                # inlining) eliminates the frame boundary entirely by
                # splicing the callee body into the caller's IR, at
                # which point this codegen case can emit ``body.statements``
                # inline.  Until then, IRUpFrame is a semantically-
                # richer IRBarrier: optimiser passes can inspect the
                # parsed body even though codegen still defers to the
                # interpreter.
                stash_idx = self._shared_imports.get("tcl_frame_depth_stash")
                restore_idx = self._shared_imports.get("tcl_frame_depth_restore")
                eval_idx = self._shared_imports.get("tcl_eval")
                body_text = ""
                if src_toks is not None and src_toks.argv_texts:
                    # argv[0] is ``uplevel``; the trailing word is the
                    # braced body.  Use the raw source text so the
                    # interpreter parses it exactly as the user wrote.
                    body_text = src_toks.argv_texts[-1]
                if eval_idx is None:
                    # No interpreter linked (standalone test compile
                    # without the runtime) — emit nothing.  Callers
                    # that check for this case handle the absence.
                    pass
                elif stash_idx is None or restore_idx is None:
                    # Frame helpers missing — run the body without a
                    # shift.  Same degraded behaviour as the existing
                    # ``_emit_cmd_uplevel`` fallback.
                    self._emit_obj_literal(body_text)
                    self._emit_call(eval_idx)
                    self._emit(WasmOp.DROP)
                else:
                    saved_local = self._add_extra_local(
                        prefix="_upframe_saved", val_type=ValType.I32
                    )
                    self._emit_i32_const(shift)
                    self._emit_call(stash_idx)
                    self._emit_local_set(saved_local)
                    self._emit_obj_literal(body_text)
                    self._emit_call(eval_idx)
                    self._emit(WasmOp.DROP)
                    self._emit_local_get(saved_local)
                    self._emit_call(restore_idx)
                if self._optimise:
                    self._const_map.clear()

            case IRIf(clauses=clauses, else_body=else_body):
                self._emit_if(clauses, else_body)

            case IRFor(init=init, condition=condition, next=next_script, body=body):
                self._emit_for(init, condition, next_script, body)

            case IRWhile(condition=condition, body=body):
                self._emit_while(condition, body)

            case IRForeach(iterators=iterators, body=body):
                self._emit_foreach(iterators, body)

            case IRSwitch(subject=subject, arms=arms, default_body=default_body, mode=mode):
                self._emit_switch(subject, arms, default_body, mode=mode)

            case IRCatch(body=body, result_var=result_var, options_var=options_var):
                self._emit_catch(body, result_var, options_var=options_var)

            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                self._emit_try(body, handlers, finally_body)

    def _resolve_import(self, command: str) -> str | None:
        """Resolve an unqualified ``command`` via ``namespace import``.

        Consults the resolved import table built by the codegen
        driver: for each candidate context namespace (the active
        ``namespace eval`` block, then the enclosing proc's namespace,
        then global), look up the short name and return the
        fully-qualified target if it points at a known proc.

        Imports recorded in a specific namespace only apply when
        we're compiling inside that namespace, matching Tcl's
        lexical resolution.  The global import table is always
        consulted last so top-level ``namespace import ::tcltest::*``
        still lets a bare ``test`` call resolve when compiling a
        top-level statement.
        """
        if not self._proc_imports:
            return None
        # Probe the most-specific context first, then widen.
        contexts: list[str] = []
        if self._block_namespace and self._block_namespace != "::":
            contexts.append(self._block_namespace)
        if self._proc_namespace and self._proc_namespace != "::":
            contexts.append(self._proc_namespace)
        contexts.append("::")
        seen: set[str] = set()
        for ctx in contexts:
            if ctx in seen:
                continue
            seen.add(ctx)
            table = self._proc_imports.get(ctx)
            if table is None:
                continue
            qn = table.get(command)
            if qn is not None and qn in self._proc_index:
                return qn
        return None

    @staticmethod
    def _proc_param_name(spec: str) -> str:
        """Extract the formal name from a ``proc`` param spec.

        ``foo`` → ``foo`` (required).
        ``{foo bar}`` → ``foo`` (optional with default ``bar``).
        Used to detect the trailing ``args`` formal so the runtime
        knows to pack surplus call-site args into a list.
        """
        s = spec.strip()
        if s.startswith("{") and s.endswith("}"):
            inner = s[1:-1].strip()
            sp = inner.split(None, 1)
            return sp[0] if sp else ""
        return s

    def _is_static_proc_args(self, args: tuple[str, ...]) -> bool:
        """Return True if ``proc NAME PARAMS BODY`` is fully static.

        Static means none of NAME/PARAMS/BODY contain ``$`` / ``[``
        substitutions — the dynamic paths are handled earlier by the
        eval-fallback branch.
        """
        if len(args) < 3:
            return False
        return all(not a.startswith("$") and not a.startswith("[") for a in args[:3])

    def _emit_re_register_proc(self, name: str, params: str, body: str | None = None) -> None:
        """Emit ``tcl_proc_register_compiled`` for a previously-lifted proc.

        Called from the runtime ``proc NAME PARAMS BODY`` site (in
        addition to the prologue) so a re-define after ``rename
        NAME {}`` (or any other path that clears the cmd_table)
        re-installs the bucket.  Without this hook the prologue
        registration is one-shot: post-rename re-defines silently
        no-op and a later call traps with ``unknown command``.

        Derives the (n_params, has_args_tail, n_required) shape from
        the source-level ``params`` text rather than the proc-index,
        so the hook works at top level (``::top``) where the index is
        empty — same as ``_compute_n_required``'s logic.

        ``body`` — when supplied, the literal source-text body is
        re-stashed via ``proc_set_body_source`` so ``info body`` keeps
        returning the original string after the re-register clears
        the body slot.
        """
        reg_idx = self._shared_imports.get("tcl_proc_register_compiled")
        if reg_idx is None:
            return
        # Parse PARAMS as a Tcl list.  Each element is either a bare
        # name (required positional) or a ``{name default}`` 2-elem
        # list (optional with default).  The trailing ``args`` formal
        # is variadic.
        from ....codegen._helpers import _split_list_simple

        try:
            param_list = _split_list_simple(params)
        except Exception:
            param_list = []
        n_params = len(param_list)
        has_args_tail = bool(param_list and self._proc_param_name(param_list[-1]) == "args")
        n_required = n_params - (1 if has_args_tail else 0)
        for p in param_list[: n_params - (1 if has_args_tail else 0)]:
            # ``{name default}`` form — has whitespace and balanced
            # braces.  ``_split_list_simple`` returns the raw element
            # so we re-split it.
            if " " in p.strip() or p.strip().startswith("{"):
                n_required -= 1
        # S2.7 — qualify against the active block namespace so a
        # static ``proc bar { … }`` inside ``namespace eval ::foo
        # { … }`` re-registers as ``::foo::bar`` rather than
        # ``::bar`` (the prior global-only path orphaned the bucket
        # and host-bridge dispatch then went looking for a wasm
        # export under the wrong qualified name).
        if name.startswith("::"):
            qname = name
        elif self._block_namespace and self._block_namespace != "::":
            qname = f"{self._block_namespace}::{name}"
        else:
            qname = f"::{name}"
        self._emit_obj_literal(qname)
        self._emit_i32_const(n_params)
        self._emit_i32_const(1)  # func_idx marker
        self._emit_i32_const(1 if has_args_tail else 0)
        self._emit_i32_const(n_required)
        self._emit_call(reg_idx)
        self._emit(WasmOp.DROP)
        if body is not None:
            body_idx = self._shared_imports.get("tcl_proc_set_body_source")
            if body_idx is not None:
                self._emit_obj_literal(qname)
                self._emit_obj_literal(body)
                self._emit_call(body_idx)
                self._emit(WasmOp.DROP)
        # Re-stash the params spec so ``info args`` returns the
        # original list after the proc_register_compiled call above
        # cleared it.  Uses the raw (ptr, len) shape — the data-
        # segment bytes are immutable so no TclObj wrapper / retain
        # is needed.  ``params`` here is the raw Tcl list source from
        # the surrounding ``proc`` statement (already parsed into a
        # single literal string), interned in the data segment.
        params_idx = self._shared_imports.get("tcl_proc_set_params_source_raw")
        if params_idx is not None and params:
            params_encoded = params.encode("utf-8", errors="surrogatepass")
            params_offset = self._intern_string(params)
            qname_encoded = qname.encode("utf-8", errors="surrogatepass")
            qname_offset = self._intern_string(qname)
            self._emit_i32_const(qname_offset + 4)
            self._emit_i32_const(len(qname_encoded))
            self._emit_i32_const(params_offset + 4)
            self._emit_i32_const(len(params_encoded))
            self._emit_call(params_idx)
            self._emit(WasmOp.DROP)

    def _resolve_proc_qname(self, command: str) -> str | None:
        """Resolve ``command`` to the qualified proc name if it matches."""
        if command.startswith("::"):
            return command if command in self._proc_index else None
        if self._block_namespace and self._block_namespace != "::":
            qn = f"{self._block_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if self._proc_namespace and self._proc_namespace != "::":
            qn = f"{self._proc_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if f"::{command}" in self._proc_index:
            return f"::{command}"
        if command in self._proc_index:
            return command
        # Final step: consult ``namespace import`` mappings so a bare
        # call like ``test …`` resolves to ``::tcltest::test`` when
        # the caller executed ``namespace import ::tcltest::*``.
        return self._resolve_import(command)

    def _resolve_proc(self, command: str) -> tuple[int, int] | None:
        """Look up a user-defined proc by name, returning (func_idx, n_params) or None.

        Resolution order for an *unqualified* name (Tcl namespace-path
        semantics, simplified):
          1. ``<active block namespace>::<name>`` — inside a
             ``namespace eval ::ns { … }`` body.
          2. ``<enclosing proc namespace>::<name>`` — when compiling
             ``::tcltest::workingDirectory``'s body, a bare call to
             ``AcceptAbsolutePath`` must first try
             ``::tcltest::AcceptAbsolutePath``.
          3. ``::<name>`` — the global namespace.
          4. Bare ``<name>`` — defensive fallback for non-standard
             proc-index entries.
        """
        if command.startswith("::"):
            return self._proc_index.get(command)
        # Inside a namespace-eval block, try the block's namespace first.
        if self._block_namespace and self._block_namespace != "::":
            ns_qname = f"{self._block_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        # Try the enclosing proc's own namespace — captured in
        # ``_proc_namespace`` at emitter construction time.
        if self._proc_namespace and self._proc_namespace != "::":
            ns_qname = f"{self._proc_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        direct = self._proc_index.get(f"::{command}") or self._proc_index.get(command)
        if direct is not None:
            return direct
        # Final step: consult ``namespace import`` mappings so bare
        # calls after ``namespace import ::tcltest::*`` (etc.)
        # dispatch directly instead of falling back to ``tcl_eval``.
        imported = self._resolve_import(command)
        if imported is not None:
            return self._proc_index.get(imported)
        return None

    def _emit_call_stmt(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
        tokens: "CommandTokens | None" = None,
        *,
        canonical_command: str | None = None,
    ) -> None:
        """Emit a command invocation.

        Dispatches known commands to inline WASM or imported runtime
        functions.  Unknown commands emit NOP.

        Proc calls are resolved first so that a user-defined proc
        named ``puts`` shadows the built-in runtime import, matching
        Tcl's command resolution semantics.

        *command* is the source-spelling word the user wrote — it is
        what flows into ``_emit_eval_fallback`` so the runtime sees the
        same name Tcl would resolve through its normal scope walk
        (a bare ``leaf`` from inside ``::ns`` resolves via Tcl's
        namespace lookup to ``::ns::leaf`` if defined; passing the
        already-globalised ``::leaf`` would force a different
        resolution).  *canonical_command* is the lowering-time
        canonical form (``::ns::cmd``) used for the literal-string
        dispatches below — qualified spellings, ``interp alias``
        aliases, and namespace-imported builtins all hit the same
        branch when matched on the canonical form.  When unset (legacy
        callers) we derive canonical from ``command`` via
        :func:`shared.naming.normalise_qualified_name`.  See
        issue #246.

        *tokens* — original parsed tokens for the command invocation.
        Threaded through to user-proc calls so they can distinguish
        braced (``{…}``) args from unbraced ones (braced args must
        skip backslash / interpolation substitution, per Tcl
        semantics).
        """
        if canonical_command is None:
            from shared.naming import normalise_qualified_name

            canonical_command = normalise_qualified_name(command) if command else command
        # <upvar-invalidate> — synthetic IRCall emitted by the CFG builder
        # to invalidate caller-side SSA defs around a call that modifies
        # variables via upvar.  No code to emit at the WASM level — but
        # we pre-intern the ``defs`` so the next statement's frame-bridge
        # readback covers them.  Without this, an embedded call like
        # ``set lg [::tcl::Lassign $item varname arg1 ...]`` (which the
        # CFG builder represents as a synthetic invalidate followed by
        # the IRAssignValue) silently drops the uplevel-back writes:
        # the IRAssignValue's value-context dispatch syncs only the
        # locals already interned, and ``varname`` / ``arg1`` aren't
        # interned until the post-call ``$varname`` reads — by which
        # time the readback has already run with an empty sync set.
        if command == "<upvar-invalidate>":
            for _def_var in defs:
                self._intern_local(_def_var)
            return

        # <cond> — synthetic IRCall emitted by the CFG builder in front
        # of an ``if`` dispatch whose condition contains a command
        # substitution that defines variables (``[catch { ... } result]``
        # is the canonical example).  The actual condition evaluation
        # happens via the block's branch terminator — this placeholder
        # only exists to carry the ``defs`` list for SSA reasoning, and
        # emits no code.
        if command == "<cond>":
            return
        # <empty_clause> — synthetic IRCall emitted by the CFG builder
        # for ``for {} {cond} {} { body }``-style empty init / next
        # clauses.  Bytecode codegen turns this into 3 NOPs to mirror
        # tclsh 9.0's ``push ""; pop`` literal-pool ordering; the WASM
        # backend emits nothing.  Without this dispatch the placeholder
        # fell through to the eval-fallback and the runtime trapped
        # with ``invalid command name "<empty_clause>"``.
        if command == "<empty_clause>":
            return
        # ``frame_set_line`` stamping happens once per IRStatement at
        # ``_emit_stmt``'s top — no per-call duplicate needed.

        # Scope declarations are NOPs in the WASM model — EXCEPT when
        # the proc / namespace command has a dynamic name that must be
        # resolved at runtime (tcltest's ``Option`` does ``proc
        # $varName body`` to install accessor procs, and the compile-
        # time registry never learns about them).  Route those through
        # the eval fallback so the interpreter's ``proc`` handler
        # registers under the current namespace.
        if command_emits_nothing(command):
            if (
                canonical_command == "::proc"
                and args
                and (
                    # Dynamic name (``proc $varName body``)
                    args[0].startswith("$")
                    or args[0].startswith("[")
                    # OR dynamic body (``proc inner {} $body``).  Without
                    # this branch the prologue would register ``inner``
                    # with the literal ``${body}`` source text as the body
                    # — when invoked, eval_script then parses ``${body}``
                    # → one word → ``unknown command: return 7`` (or
                    # whatever ``$body`` substitutes to at the moment of
                    # the proc call).  Routing through the eval-fallback
                    # makes the interpreter perform the substitution
                    # before storing.
                    or (len(args) >= 3 and (args[2].startswith("$") or args[2].startswith("[")))
                    # OR dynamic params (rare but possible: ``proc f $params body``)
                    or (len(args) >= 2 and (args[1].startswith("$") or args[1].startswith("[")))
                )
            ):
                self._emit_eval_fallback(command, args)
                self._emit(WasmOp.DROP)
                return
            # ``namespace eval ns arg1 arg2 ...`` in statement context
            # with dynamic script args: build the script at WASM level
            # (so compiled-frame aliases like $arr($key) are resolved
            # correctly) and call tcl_eval for side effects.
            if canonical_command == "::namespace" and args and args[0] == "eval" and len(args) > 2:
                # Bridge drops the result since we're in statement context.
                # If imports are missing the bridge returns False and we
                # silently skip (statement context has no stack commitments).
                self._emit_namespace_eval_bridge(args[2:], drop_result=True, ns_name=args[1])
                return
            # ``namespace import`` / ``namespace export`` / ``namespace
            # forget`` — record the side effect at runtime so the
            # interpreter's ``ns_import`` / ``ns_export`` / ``ns_forget``
            # creates real redirects / export patterns.  The compile-time
            # resolver (``module.namespace_imports`` / ``namespace_exports``)
            # still shortens specialised calls when the proc index is live,
            # but under full flush (``interp create`` / ``eval`` /
            # ``delete``) that table is cleared and every call routes
            # through ``tcl_eval`` — the runtime then needs the import
            # redirect to resolve ``testConstraint`` → ``::tcltest::testConstraint``.
            #
            # Only emit for subcommands with real runtime side effects.
            # ``namespace eval`` is handled above; ``namespace current``,
            # ``namespace which`` etc. are lookups with no side effect
            # and remain NOPs at codegen.
            if (
                canonical_command == "::namespace"
                and args
                and args[0]
                in (
                    "import",
                    "export",
                    "forget",
                    "path",
                    "unknown",
                    "delete",
                    "ensemble",
                )
            ):
                # ``namespace path LIST`` mutates the current ns's
                # command-resolution path; tcltest's mathop.test does
                # ``namespace path ::tcl::mathop`` then calls bare
                # ``+``.  Without this fallback the call was a no-op
                # and the path stayed empty.  ``namespace unknown`` /
                # ``namespace delete`` are the other ones with real
                # side effects beyond the visible-import / export set.
                self._emit_eval_fallback(command, args)
                self._emit(WasmOp.DROP)
                return
            # Static ``proc NAME PARAMS BODY`` at runtime: re-emit
            # ``tcl_proc_register_compiled`` so a previously-renamed
            # away or unset proc gets re-registered.  Without this,
            # the prologue-only registration is one-shot — after
            # ``rename foo {}`` clears the cmd_table, a later
            # ``proc foo {} {body}`` becomes a no-op and the proc
            # stays absent.  (dict.test 23.X+ uses
            # define→rename-delete→re-define→rename-delete on
            # ``linenumber``.)
            if canonical_command == "::proc" and self._is_static_proc_args(args):
                # First static def with this name at top level: re-emit
                # ``proc_register_compiled`` so a previous
                # ``rename NAME {}`` followed by ``proc NAME`` re-
                # installs the bucket (the prologue's one-shot
                # registration won't fire twice).
                #
                # Second-or-later static def with the same name:
                # the lifter only kept the FIRST def's body in
                # ``ir_module.procedures``, so the compile-time
                # direct-call resolution path would always invoke
                # the stale body.  Route through eval-fallback (the
                # interpreter's ``proc`` registers a fresh
                # interpreted body and clears ``func_idx``), and
                # remove the entry from ``_proc_index`` so any
                # SUBSEQUENT compile-time call-site resolution
                # misses and falls through to the bucket dispatch
                # — which then reads the new body.
                # Same block-namespace qualification as
                # ``_emit_re_register_proc`` so the seen-set / proc-
                # index pop keys agree with the registered name.
                if args[0].startswith("::"):
                    qname = args[0]
                elif self._block_namespace and self._block_namespace != "::":
                    qname = f"{self._block_namespace}::{args[0]}"
                else:
                    qname = f"::{args[0]}"
                seen = getattr(self, "_seen_top_level_procs", None)
                if seen is None:
                    seen = set()
                    self._seen_top_level_procs = seen
                if qname in seen:
                    self._emit_eval_fallback(command, args)
                    self._emit(WasmOp.DROP)
                    # Drop both the qualified and bare entries so
                    # ``_resolve_proc`` (which probes both) misses.
                    self._proc_index.pop(qname, None)
                    self._proc_index.pop(args[0], None)
                else:
                    seen.add(qname)
                    body_arg = args[2] if len(args) >= 3 else None
                    self._emit_re_register_proc(args[0], args[1], body_arg)
            return

        # break/continue — emit WASM br to exit/restart the enclosing loop.
        # Loop structure: block{ loop{ block{ <body> }; <next>; br 0 } }
        # _loop_ctrl_depths records ctrl_depth at the inner (continue) block.
        # From inside the body at ctrl_depth D, with loop_ctrl C:
        #   continue: br(D - C) exits the continue block → runs <next>
        #   break:    br(D - C + 2) exits continue block + loop + outer block
        #
        # When an enclosing ``catch`` sits between us and the innermost
        # loop (``_loop_catch_depths[-1] < _catch_depth``), a structured
        # ``br`` would jump past ``catch_leave`` and prevent the catch
        # from absorbing the flow control as a TCL_BREAK / TCL_CONTINUE
        # return code.  Fall through to the runtime ``break`` /
        # ``continue`` handlers in that case — they set the matching
        # flow-control flag, ``catch_has_error`` short-circuits the rest
        # of the catch body, and ``catch_leave`` reads the flag and
        # returns the right code.  See cmdAH-0.2 / 3.2 in
        # ``cmdAH.test``.
        loop_inside_catch = bool(self._loop_ctrl_depths) and (
            not self._loop_catch_depths or self._loop_catch_depths[-1] >= self._catch_depth
        )
        if canonical_command == "::break" and loop_inside_catch:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl + 2
            self._emit_br(br_depth)
            return
        if canonical_command == "::continue" and loop_inside_catch:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl
            self._emit_br(br_depth)
            return

        # User-defined proc call takes priority over built-ins
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            qname = self._resolve_proc_qname(command)
            self._emit_cmd_proc_call(
                proc_info[0],
                proc_info[1],
                args,
                defs,
                qname=qname,
                tokens=tokens,
                invoked_name=command,
            )
            return

        # Registry-driven dispatch — covers set, incr, return, string,
        # dict, info, lassign, lset, clock, uplevel, array, unset, list,
        # and all runtime-import commands.  Each hook returns True when
        # handled and False to fall through (e.g. unset with no array elems).
        # Uses get_wasm_hook (not get_any) to scan all specs: dialect packs
        # loaded after the emitter was first imported add new specs without
        # hooks, but the hook is still on an earlier spec.
        hook = _REGISTRY.get_wasm_hook(command)
        if hook is not None:
            # Stash the current call's tokens on self so a hook that
            # routes to ``_emit_eval_fallback`` can recover the
            # original brace / substitution information without us
            # having to widen the public ``CodegenHook`` signature.
            # Critical for regex / regsub patterns: a braced word
            # ``{[abc]+}`` lowers to IR value ``[abc]+`` which the
            # fallback's ``a.startswith("[")`` heuristic would
            # otherwise mistake for a Tcl ``[cmd]`` substitution and
            # pass through unquoted — making the interpreter try to
            # execute ``abc`` as a command.
            prev_tokens = getattr(self, "_current_call_tokens", None)
            self._current_call_tokens = tokens
            try:
                if hook(self, args, defs, EmitContext.STATEMENT):
                    return
            finally:
                self._current_call_tokens = prev_tokens

        # Unknown command — fall back to interpreter.
        # Pre-intern defs so _emit_frame_readback (inside the fallback)
        # can reload them.  Without this, a first-time defs variable like
        # ``scan ... v`` is not yet in _tcl_var_locals and the readback
        # silently skips it, leaving the WASM local at 0.
        for _def_var in defs:
            self._intern_local(_def_var)
        self._emit_eval_fallback(command, args, tokens=tokens)
        self._emit(WasmOp.DROP)  # statement context — discard result

    def _emit_diag_site(
        self,
        command: str,
        *,
        args: tuple[str, ...] = (),
        kind: str = "fallback",
    ) -> None:
        """Register a diagnostic site and emit a ``tcl_diag_set`` call.

        The site ID is a monotonic counter scoped to the module's
        :class:`DiagMap`; ID 0 is reserved for "unset", so the first
        emitted site is ID 1.  The site stores the current statement's
        source range (populated by ``_emit_stmt``) and the supplied
        command + kind so a sidecar resolver can print a useful
        location on trap.

        When no ``diag_map`` is attached to this emitter (standalone
        tests that don't care about source mapping) the call is a
        no-op — the ``tcl_diag_set`` import is still registered, but
        we simply don't emit a call for it.
        """
        if self._diag_map is None:
            return
        diag_idx = self._shared_imports.get("tcl_diag_set")
        if diag_idx is None:
            return
        rng = self._current_range
        if rng is None:
            # Happens when a trap is emitted outside a statement context
            # (rare — e.g. the upvar preamble pre-scan).  Skip rather
            # than pointing at an unrelated site.
            return
        site_id = len(self._diag_map.sites) + 1
        site = DiagSite(
            id=site_id,
            file=self._diag_map.filename,
            line=rng.start.line + 1,  # IR is 0-based; sidecar is 1-based
            col=rng.start.character + 1,
            end_line=rng.end.line + 1,
            end_col=rng.end.character + 1,
            command=command,
            args=args,
            kind=kind,
            proc=self._proc_qname,
        )
        self._diag_map.add_site(site)
        self._emit_i32_const(site_id)
        self._emit_call(diag_idx)

    def _emit_unsupported_trap(self, command: str) -> None:
        """Emit hard trap for commands that cannot work in WASM at all.

        Used for exec, socket, coroutine, etc. — these can't be
        handled by the interpreter either.  Inside catch, error()
        sets the flag without trapping.
        """
        self._emit_diag_site(command, kind="unsupported")
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        if self._catch_depth == 0:
            self._emit(WasmOp.UNREACHABLE)

    def _emit_static_parse_error_trap(self, command: str, message: str) -> None:
        """Emit a ``tcl_cmd_error`` call carrying *message* — used by
        :class:`IRBarrier` arms whose ``reason`` describes a static parse
        error the lowering already classified (e.g. ``if`` with extra
        words after ``else``).  Mirrors :meth:`_emit_unsupported_trap`
        but takes the full Tcl-formatted message verbatim so the WASM
        backend reports the same string the VM / C Tcl produce.
        """
        self._emit_diag_site(command, kind="parse_error")
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            self._emit_obj_literal(message)
            self._emit_call(fidx)
        if self._catch_depth == 0:
            self._emit(WasmOp.UNREACHABLE)

    def _emit_eval_fallback(
        self,
        command: str,
        args: tuple[str, ...] | list[str] = (),
        *,
        script_override: str | None = None,
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Fall back to the Zig interpreter for an uncompiled command.

        Builds a command string from *command* and *args* and calls
        ``tcl_eval(script)``.  The result (i32 TclObj) is left on
        the WASM stack — caller must drop or use it as needed.

        When *script_override* is given it is used verbatim as the
        eval script instead of reconstructing it from *command* / *args*.
        Use this when the original source text must be preserved (e.g.
        for ``{*}`` argument expansion whose syntax cannot be round-tripped
        through the normal quoting path).

        *tokens* carries the original parsed word tokens when available.
        Used to distinguish braced (``{…}``, STR) arguments — whose IR
        value is the literal content — from plain / double-quoted
        (ESC) arguments whose IR value still contains raw backslash
        sequences that must be substituted before list-quoting.
        Without this distinction ``"a\\\\{b"`` (source → IR value
        ``a\\{b``) would round-trip through list-quote as though the
        two backslashes were the *final* value, producing ``a\\{b``
        instead of ``a\\{b`` after the interpreter re-parses the word.
        """
        self._emit_diag_site(command, args=tuple(args), kind="fallback")
        eval_idx = self._shared_imports.get("tcl_eval")
        if eval_idx is not None:
            if script_override is not None:
                script = script_override
            else:
                # Hook-driven callers (regsub / regexp / apply / …)
                # don't get a *tokens* parameter through the
                # ``CodegenHook`` signature — pull them from the
                # ``_current_call_tokens`` stash that
                # ``_emit_call_stmt`` set up before invoking the hook.
                # Without this, a braced regex pattern lowered to
                # IR ``[abc]+`` would be reassembled unquoted and
                # the interpreter would try to execute ``abc``.
                if tokens is None:
                    tokens = getattr(self, "_current_call_tokens", None)

                # Build command string: "command arg1 arg2 ..."
                # For literal args, concatenate them. For $var refs,
                # include the dollar sign so the interpreter can resolve them.
                def _arg_was_braced(i: int) -> bool:
                    """Return True if call-site arg *i* came from a ``{…}`` token.

                    *i* is 0-based into *args*, so ``tokens.argv[i + 1]``
                    is the corresponding parsed word (argv[0] is the
                    command name).  Requires a single-token word —
                    a concatenated word like ``{a}b`` is not purely
                    braced and needs normal processing.
                    """
                    if tokens is None or tokens.argv is None:
                        return False
                    tok_idx = i + 1
                    if tok_idx >= len(tokens.argv):
                        return False
                    if tokens.single_token_word is not None:
                        if tok_idx >= len(tokens.single_token_word):
                            return False
                        if not tokens.single_token_word[tok_idx]:
                            return False
                    return tokens.argv[tok_idx].type == TokenType.STR

                def _arg_is_multi_token(i: int) -> bool:
                    """Return True if call-site arg *i* is a concatenation of
                    multiple lexer tokens (e.g. ``"$body (suffix)"`` —
                    substitution + literal text).

                    Multi-token words must be re-wrapped in double quotes
                    when fed to ``tcl_eval`` so the interpreter sees them
                    as a single word; passing the raw IR through verbatim
                    would let the embedded space split the value into
                    multiple words at eval time.
                    """
                    if tokens is None or tokens.single_token_word is None:
                        return False
                    tok_idx = i + 1
                    if tok_idx >= len(tokens.single_token_word):
                        return False
                    return not tokens.single_token_word[tok_idx]

                # ``command`` arrives in canonical-var form when the
                # user wrote a dollar substitution at the command
                # position (``$Verify($option)`` → IRCall.command =
                # ``${Verify($option)}``).  Reference Tcl's ``${name}``
                # syntax does *not* support array element references
                # — ``${arr(key)}`` looks up a SCALAR variable named
                # literally ``arr(key)``.  When ``command`` looks
                # like ``${X(Y)}`` we know the user intended the
                # array form ``$X(Y)``; emit that so the interpreter
                # parses it as an array element substitution and
                # recursively substitutes the index span.  Without
                # this the runtime raises ``can't read "X(Y)": no
                # such variable`` against the literal scalar.
                cmd_emit = command
                if command.startswith("${") and command.endswith("}") and "(" in command:
                    inner = command[2:-1]
                    if inner.endswith(")") and "(" in inner:
                        cmd_emit = "$" + inner
                parts = [cmd_emit]
                for i, a in enumerate(args):
                    if _arg_was_braced(i):
                        # Braced token — IR holds the literal content
                        # with outer ``{}`` stripped.  Re-wrap in braces
                        # so the interpreter sees the exact same word
                        # without applying any substitution.  Fall back
                        # to list-quote when the value contains an
                        # unbalanced brace (rare — ``{a{b}`` style).
                        if "{" in a or "}" in a:
                            # Use list-quote to get balanced/escaped form.
                            # Args are never at command-start so a leading
                            # ``#`` does not need quoting.
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append("{" + a + "}")
                    elif _arg_is_multi_token(i):
                        # Concatenation of multiple lexer tokens — one of
                        # ``"$body (suffix)"`` (subst + literal text),
                        # ``foo$bar`` (literal + subst, no space),
                        # ``\n[list ...]`` (escape + cmd subst), ...
                        # ``_word_piece`` already joined the source
                        # fragments with no separators, so the IR holds
                        # the substitution markers verbatim alongside
                        # any literal text.  Re-wrap in ``"…"`` so the
                        # interpreter sees one word at eval time;
                        # embedded ``$`` / ``[`` keep their substitution
                        # semantics inside ``"…"``.  Backslash sequences
                        # are passed through unmodified — Tcl's in-quote
                        # substitution will re-evaluate them to the same
                        # bytes the original word produced (``\n`` →
                        # newline).
                        parts.append('"' + _escape_dquote(a) + '"')
                    elif a.startswith("$") or a.startswith("["):
                        # Pure substitution word — pass through unquoted
                        # so the interpreter resolves it at eval time.
                        parts.append(a)
                    else:
                        # Literal IR value from an ESC token (plain or
                        # double-quoted word).  The IR stores the RAW
                        # text: source-level ``\\{`` is still two bytes
                        # ``\`` + ``{``.  Apply backslash substitution
                        # so the value we embed in the script reflects
                        # what the original word would have evaluated
                        # to.  _tcl_list_quote then encodes it as a
                        # safe Tcl word that round-trips through the
                        # interpreter's word parser.
                        #
                        # Guard: if compile-time substitution would
                        # produce a Python string with isolated
                        # surrogates (``\uD83D\uDE02`` in a Tcl 9 test)
                        # we can't safely UTF-8-encode it into the WASM
                        # data segment.  Fall back to embedding the raw
                        # value verbatim so the interpreter itself
                        # handles the escape sequences at eval time.
                        prepped: str | None = None
                        if "\\" in a:
                            candidate = _tcl_backslash_subst(a)
                            try:
                                # ``surrogatepass`` lets lone-surrogate
                                # literals (``😂`` in Tcl 9
                                # corpus) round-trip through the data
                                # segment as 3-byte WTF-8 — the runtime
                                # reads them back via tcl_bs.zig and
                                # produces the same codepoints.
                                candidate.encode("utf-8", errors="surrogatepass")
                            except UnicodeEncodeError:
                                prepped = None  # signal: use raw path
                            else:
                                prepped = candidate
                        else:
                            prepped = a
                        # Script args never sit at command-start — pass
                        # ``first=False`` so a leading ``#`` is left alone.
                        if prepped is None:
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append(_tcl_list_quote(prepped, first=False))
                script = " ".join(parts)
            # Sync all live proc-locals into the frame so the
            # interpreter can see them via var_resolve.  We sync
            # conservatively (every local, not just ``$name`` refs
            # in the script) because Tcl commands like ``info exists
            # x`` / ``unset x`` / ``upvar 1 x y`` take BARE identifiers
            # that we can't reliably distinguish from literal strings
            # by a simple scan.  Narrowing is a future optimisation;
            # requires a per-command argument-kind analysis.
            self._emit_interp_boundary("dispatch")
            # Stamp the current namespace into the interpreter's ns
            # register so dynamic ``proc $name body`` / ``variable
            # $name`` inside the fallback qualify into the enclosing
            # namespace instead of falling through to ``::``.  Only
            # applies inside compiled procs that have a namespace
            # other than global.
            ns_saved_idx: int | None = None
            if self._is_proc and self._proc_namespace and self._proc_namespace != "::":
                ns_set_idx = self._shared_imports.get("tcl_ns_set")
                if ns_set_idx is not None:
                    ns_saved_idx = self._add_extra_local(prefix="_ns_saved", val_type=ValType.I64)
                    # ns name without the leading ``::`` — the
                    # interpreter's qualify_name prepends ``::`` if
                    # needed; keep the full ``::ns`` form for
                    # consistency with how the compiler emits
                    # qualified names elsewhere.
                    ns_literal = self._proc_namespace
                    # Stash namespace bytes in the data section and
                    # push (ptr, len).  Reuse the string-constant
                    # pool so multiple fallbacks in the same proc
                    # share the bytes.
                    offset = self._intern_string(ns_literal)
                    encoded = ns_literal.encode("utf-8", errors="surrogatepass")
                    self._emit_i32_const(offset + 4)
                    self._emit_i32_const(len(encoded))
                    self._emit_call(ns_set_idx)
                    self._emit_local_set(ns_saved_idx)
            self._emit_obj_literal(script)
            self._emit_call(eval_idx)
            # Reload locals from frame — eval may have modified them (e.g.
            # ``set x 99``, writes through ``upvar`` aliases, ``unset``).
            self._emit_frame_readback()
            # Restore the caller's namespace context.
            if ns_saved_idx is not None:
                ns_restore_idx = self._shared_imports.get("tcl_ns_restore")
                if ns_restore_idx is not None:
                    # Stash eval result before calling ns_restore
                    # (which returns void) and put it back after.
                    result_tmp = self._add_extra_local(prefix="_eval_result", val_type=ValType.I32)
                    self._emit_local_set(result_tmp)
                    self._emit_local_get(ns_saved_idx)
                    self._emit_call(ns_restore_idx)
                    self._emit_local_get(result_tmp)
            # Signal-flag propagation.  When the eval body raised
            # ``error`` or ``return``, ``eval_script`` exits early
            # leaving the corresponding flag set; without an
            # explicit unwind here the compiled body would happily
            # run the next statement and (for ``return``) try to
            # read variables that the body's early exit never
            # initialised — opt-10.5's "can't read 'arg'" came
            # from exactly this path: ``OptDoOne`` ran the if-chain
            # via fallback whose ``return -code return`` set the
            # flag, then the compiled switch immediately read
            # ``$arg`` with strict-read.  The unwind hands the
            # eval result back to the caller (matches Tcl's
            # ``proc {} { return $x }`` semantics — the body's
            # final value becomes the proc's result) while
            # leaving the flag set so the surrounding catch /
            # outer compiled call can see it.
            self._emit_signal_check_and_return()
            return
        # No interpreter available — hard trap
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        self._emit(WasmOp.UNREACHABLE)

    def _emit_call_stmt_tail(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
        *,
        canonical_command: str | None = None,
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Emit a command invocation in tail position, keeping its i32 result on the stack.

        Used for implicit return: the last command's result becomes the
        proc's return value.  Dispatch order matches ``_emit_call_stmt``
        (proc calls first, then built-ins) to ensure consistent behaviour.

        See :meth:`_emit_call_stmt` for the *command* / *canonical_command*
        contract — *command* is the source spelling that flows into
        ``_emit_eval_fallback``; *canonical_command* is the lowering-time
        canonical form used for the literal-string dispatches.
        """
        if canonical_command is None:
            from shared.naming import normalise_qualified_name

            canonical_command = normalise_qualified_name(command) if command else command
        # Scope declarations produce no value — return null TclObj
        if command_emits_nothing(command):
            # ``namespace eval ns arg1 arg2 ...`` in tail position with dynamic
            # script args: assemble the script at WASM level and call tcl_eval
            # so the result becomes the proc's return value.
            if canonical_command == "::namespace" and args and args[0] == "eval" and len(args) > 2:
                if self._emit_namespace_eval_bridge(args[2:], drop_result=False, ns_name=args[1]):
                    return
                # Runtime imports missing — push null TclObj as fallback.
                self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # <upvar-invalidate> — synthetic CFG-builder invalidation node.
        # In tail position (rare, but possible if placed last by optimisation)
        # we still need to push a null TclObj.
        if command == "<upvar-invalidate>":
            self._emit_i32_const(0)
            return
        # <cond> / <empty_clause> — synthetic CFG-builder placeholders
        # that emit no real instructions.  In tail position we still
        # need a TclObj to leave on the stack so the caller's return
        # has a value; push null.
        if command == "<cond>" or command == "<empty_clause>":
            self._emit_i32_const(0)
            return

        # User-defined proc call — keep i32 result
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            func_idx, n_params = proc_info
            qname = self._resolve_proc_qname(command)
            has_args_tail = qname is not None and qname in self._proc_args_tail
            # Stash the invoked word for the callee's
            # ``info level 0`` argv0 — see
            # :meth:`_emit_prepare_pending_argv0`.
            argv0_local = self._emit_prepare_pending_argv0(command)
            if has_args_tail and n_params > 0:
                # Variadic proc — pack call-site args past the fixed
                # positionals into a single list TclObj for the
                # trailing ``args`` slot.  Without this branch, a
                # tail-position call to ``proc f {x args}`` from
                # inside another proc body silently truncates to
                # ``f a`` (dropping ``b c d``) — same fix-shape as
                # ``_emit_cmd_proc_call`` (statement context) and
                # ``_emit_command_subst_value`` (value context).
                fixed = n_params - 1
                for i in range(min(fixed, len(args))):
                    self._emit_value(args[i])
                for _slot in range(len(args), fixed):
                    self._emit_i32_const(0)
                self._emit_args_list(tuple(args[fixed:]))
            else:
                # Push exactly n_params args (truncate surplus, pad missing)
                for i in range(min(n_params, len(args))):
                    self._emit_value(args[i])
                for _ in range(n_params - len(args)):
                    self._emit_i32_const(0)
            self._emit_push_pending_argv0(argv0_local)
            # Pessimistic procs need the frame-bridge sync/readback
            # around the call so the callee's ``upvar`` /
            # ``uplevel`` writes propagate (and so the callee can
            # observe caller-side WASM-local state via the runtime
            # frame).  The tail position is no exception — without
            # this, a proc whose only body is ``set X v; Callee X``
            # never syncs ``X`` to the frame and ``Callee``'s
            # ``upvar 1 $aN x`` pulls 0 from the empty frame slot,
            # raising ``can't read "x": no such variable``.  See the
            # statement-context path in ``_emit_cmd_proc_call`` for
            # the matching wrap.
            self._emit_compiled_call_with_bridge(func_idx, defs=defs)
            # i32 result stays on the stack
            return

        # Registry hooks handle every command that has either a
        # ``register_wasm_emitter`` hook (set/incr/list/info/…) or a
        # ``CommandSpec.wasm_runtime_import`` (puts/append/lindex/…).
        # Hooks now fully support EmitContext.VALUE — the runtime hook
        # emits the import call and leaves the result on the stack via
        # ``_emit_cmd_runtime(context=VALUE)`` / ``_runtime_call_end``.
        # Stash ``_current_call_tokens`` around the hook so per-arg
        # brace info survives — without this, a tail-position
        # ``string match {\?*} $name`` loses the BRACED bit on the
        # pattern and ``_emit_value`` runs Tcl backslash subst on
        # the IR ``\?*``, producing ``?*`` and matching everything.
        prev_tokens = getattr(self, "_current_call_tokens", None)
        self._current_call_tokens = tokens
        try:
            hook = _REGISTRY.get_wasm_hook(command)
            if hook is not None and hook(self, args, defs, EmitContext.VALUE):
                return
        finally:
            self._current_call_tokens = prev_tokens

        # Unknown command in tail position — fall back to interpreter,
        # leaving the result on the stack for implicit return.
        self._emit_eval_fallback(command, args, tokens=tokens)

    # -- Global variable write-through --
