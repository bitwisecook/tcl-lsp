"""_WasmEmitterCmdMixin: Tcl command emitters."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....parsing.tokens import TokenType
from ....ir import (
    CommandTokens,
)
from .._encoding import (
    _tcl_list_quote,
    _tcl_token_value,
)
from .._imports import (
    _CLOCK_SUBCMD_IMPORT,
    _CMD_RUNTIME,
    _CMD_RUNTIME_NONTRAPPING,
    _RUNTIME_IMPORTS,
)
from .._ir import (
    ValType,
    WasmOp,
)
from .._parsing import (
    _parse_array_ref,
)
from ._ops import _is_end_relative_index


class _WasmEmitterCmdMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_args_list(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _has_embedded_subst(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_diag_site(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_array_name_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _is_frame_only_var(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_cmd_return(self, args: tuple[str, ...]) -> None:
        """``return ?value?`` or ``return -code code ?value?``.

        The simple single-value form compiles to a WASM return.
        ``return -code error <msg>`` is special-cased inline: evaluate
        *msg* via :meth:`_emit_value` (so embedded ``$var`` /
        ``[cmd]`` substitutions work), then call ``tcl_cmd_error``
        — which sets the catch's ``error_flag``/``error_msg`` when
        inside a ``catch`` or traps otherwise.  Going through
        :meth:`_emit_eval_fallback` would brace-wrap the message to
        preserve list structure, blocking the substitutions the
        error text needs — a real hazard because tcltest's error
        messages embed ``$option`` / ``$values`` everywhere.

        Other ``-code`` forms (``return -code break``, ``return -code
        continue``, numeric codes, ``-level N``, ``-errorinfo``
        ``-errorcode``) are rarer and fall through to the eval
        fallback, whose argument quoting is safe for them because
        their payloads are typically literal keywords or numeric
        values without interpolation.
        """
        if args and len(args) >= 3 and args[0] == "-code" and args[1] == "error" and len(args) == 3:
            # return -code error <msg>
            self._emit_value(args[2])
            # ``_RUNTIME_IMPORTS`` keys the error import as
            # ``tcl_error`` (internal key) → WASM name
            # ``tcl_cmd_error``; use the internal key to look up
            # the shared import slot.
            err_idx = self._shared_imports.get("tcl_error")
            if err_idx is None:
                self._emit_eval_fallback("return", args)
                return
            self._emit_call(err_idx)
            # tcl_cmd_error returns nothing; emit a null TclObj for
            # the WASM return value.  When inside a catch, error_flag
            # is now set and the catch body's has_error check will
            # trip on the next statement.  When outside a catch, the
            # runtime's tcl_cmd_error already traps and this return
            # is unreachable.
            self._emit_i32_const(0)
            self._emit(WasmOp.RETURN)
            return
        if args and args[0].startswith("-"):
            self._emit_eval_fallback("return", args)
            return
        if args:
            self._emit_value(args[0])
        else:
            self._emit_i32_const(0)
        self._emit(WasmOp.RETURN)

    def _emit_array_subcmd_value(self, args: tuple[str, ...]) -> None:
        """``array <subcmd> <arr> ?args?`` — leaves i32 TclObj on the stack.

        Supported subcommands:
          exists, size, unset, names, set, get.  Others fall back to
          the interpreter, which will see the compile-time snapshot
          of the array's state (via globals — interpreter doesn't
          touch per-array tables yet).
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]
        if subcmd == "exists" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_exists")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "size" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_size")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "unset" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_unset")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "names" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_names")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                # Optional glob pattern — ``array names arr`` → no
                # filter (null TclObj), ``array names arr pat`` →
                # use the supplied pattern.  ``-exact`` / ``-glob``
                # / ``-regexp`` modes fall through to the fallback
                # for now; the common case scripts use is positional.
                if len(args) >= 3:
                    self._emit_value(args[2])
                else:
                    self._emit_i32_const(0)
                self._emit_call(fidx)
                return
        elif subcmd == "set" and len(args) >= 3:
            # ``array set arr {key val key val ...}`` — iterate the list
            # literal at compile time when possible, otherwise fall back
            # to the interpreter.  Most real-world usage is literal.
            self._emit_array_set_list(args[1], args[2])
            return
        elif subcmd == "get" and len(args) >= 2:
            # ``array get arr`` — return a flat {key val key val ...}
            # list.  Not implemented in the compiled runtime yet; fall
            # back to tcl_eval so the interpreter handles it (returns
            # empty for now, which degrades rather than mis-computes).
            pass
        self._emit_eval_fallback("array", args)

    def _emit_array_set_list(self, arr: str, kv_text: str) -> None:
        """``array set arr {k v k v ...}`` — compile-time list literal.

        Parses the inline list and emits one ``array_set`` call per
        pair.  Falls back to the interpreter for non-literal inputs.
        Leaves an empty string TclObj on the stack as the command's
        return value.
        """
        from ..._helpers import _split_list_simple

        fidx = self._shared_imports.get("tcl_array_set")
        if fidx is None:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        # Strip an optional outer braces around the literal list.
        text = kv_text
        if text.startswith("{") and text.endswith("}"):
            text = text[1:-1]
        try:
            words = _split_list_simple(text)
        except Exception:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        if len(words) & 1:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        for i in range(0, len(words), 2):
            self._emit_array_name_obj(arr)
            self._emit_value(words[i])
            self._emit_value(words[i + 1])
            self._emit_call(fidx)
            self._emit(WasmOp.DROP)
        # ``array set`` returns empty string.
        self._emit_obj_literal("")

    def _emit_unset_array_elems(self, args: tuple[str, ...]) -> bool:
        """Handle ``unset arr(key)`` forms.  Returns True if at least one
        array-element unset was emitted (and scalar unsets, if mixed in,
        were emitted as NOPs) — caller should ``return`` from the
        dispatch.  Returns False if *args* contains no array-element
        references at all.
        """
        had_array = False
        fidx = self._shared_imports.get("tcl_array_unset_element")
        # Skip ``-nocomplain`` / ``--`` option prefix.
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "--":
                i += 1
                break
            i += 1
        for name in args[i:]:
            ref = _parse_array_ref(name)
            if ref is None:
                continue
            had_array = True
            arr, key = ref
            if fidx is None:
                continue
            self._emit_array_name_obj(arr)
            self._emit_value(key)
            self._emit_call(fidx)
            self._emit(WasmOp.DROP)
        return had_array

    def _emit_list_value(self, args: tuple[str, ...]) -> None:
        """Build a Tcl list from *args* and leave it on the stack as a TclObj.

        Each arg is emitted with proper Tcl list-element encoding —
        empty strings become ``{}``, words containing whitespace /
        braces / brackets / ``"`` / ``\\`` / ``$`` / ``;`` get wrapped
        in braces so re-parsers see one element per arg.  This is
        critical: ``[list "a b" c]`` must produce a two-element list
        ``{a b} c``, not the three-word string ``a b c``.

        When every arg is a literal the result is folded at compile
        time into a single ``obj_new_string``; otherwise we emit a
        ``tcl_concat`` chain with single-space separators.  Variable
        and command-substitution args skip the element-quoting step
        — their runtime value is used as-is, matching the pre-
        existing ``tcl_list`` behaviour for interpolated values
        (quoting them at compile time would brace-wrap whatever
        runtime value they hold, changing semantics).
        """
        if not args:
            self._emit_i32_const(0)
            return
        # Fast path: all literals → compile-time string.
        # _tcl_token_value expands each source token to its VALUE (strips
        # braces, applies backslash subst), then _tcl_list_quote encodes
        # the value as a list element.
        if all(not a.startswith("$") and not a.startswith("[") for a in args):
            self._emit_obj_literal(
                " ".join(
                    _tcl_list_quote(_tcl_token_value(a), first=(i == 0)) for i, a in enumerate(args)
                )
            )
            return
        # Mixed: start with an empty list, lappend each arg so that
        # runtime values containing spaces are properly quoted as
        # single list elements (tcl_cmd_lappend wraps in {} if needed).
        lappend_idx = self._shared_imports.get("tcl_lappend")
        if lappend_idx is None:
            # No lappend helper — fall back to first arg only.
            self._emit_value(args[0])
            return

        def _emit_elem(a: str) -> None:
            # Braced literal — strip outer braces and emit the raw content;
            # tcl_cmd_lappend will re-quote it correctly if needed.
            if a.startswith("{") and a.endswith("}"):
                self._emit_obj_literal(a[1:-1])
                return
            if a.startswith("$") or a.startswith("["):
                self._emit_value(a)
            else:
                self._emit_obj_literal(a)

        self._emit_obj_literal("")  # empty list seed
        for a in args:
            _emit_elem(a)
            self._emit_call(lappend_idx)

    def _emit_cmd_uplevel(self, args: tuple[str, ...]) -> None:
        """``uplevel ?level? body`` — evaluate script in a caller's frame.

        ``level`` defaults to ``1``.  ``#0`` means absolute global
        scope; ``#N`` means N frames above global (Tcl only really
        uses ``#0``); a bare integer N means N frames up relative to
        the current frame.

        We emit:
            saved = frame_depth_stash(up)
            result = tcl_eval(body)
            frame_depth_restore(saved)

        ``frame_depth_stash`` clamps the shift at frame 0 so invalid
        levels degrade to global-scope eval rather than trapping.  If
        the caller chain is entirely compiled (no frames pushed), this
        is effectively a no-op and the eval runs at global scope — the
        ``#0`` behaviour Tcl scripts most commonly use.

        Multiple bodies concatenated as separate words (``uplevel 1 a b c``)
        are joined with spaces to form the script, matching Tcl's
        semantics of "concat all remaining arguments into a single
        script".
        """
        if not args:
            self._emit_i32_const(0)
            return
        # Parse the level specifier.  ``#0`` → absolute 0 which clamps
        # to global; a bare integer N → relative N; anything else means
        # no level, default 1, and args[0] is the first body word.
        level_spec = args[0]
        up: int
        body_start = 1
        if level_spec.startswith("#"):
            try:
                abs_level = int(level_spec[1:])
                # "#N means N frames above global".  We approximate by
                # stashing all the way to that absolute depth; frame_depth_stash
                # subtracts, so ``up`` is (current_depth - abs_level), which
                # we can't know at compile time.  For the common #0 case
                # we stash "all the way" by passing a large relative shift.
                up = 0 if abs_level == 0 else 1
            except ValueError:
                up = 1
        else:
            try:
                up = int(level_spec)
                # Numeric → relative.
            except ValueError:
                # Not a level — arg0 is part of the body.
                up = 1
                body_start = 0
        if body_start >= len(args):
            self._emit_i32_const(0)
            return

        body_parts = list(args[body_start:])

        eval_idx = self._shared_imports.get("tcl_eval")
        stash_idx = self._shared_imports.get("tcl_frame_depth_stash")
        restore_idx = self._shared_imports.get("tcl_frame_depth_restore")
        if eval_idx is None or stash_idx is None or restore_idx is None:
            # Missing runtime helpers — fall back to plain eval, which
            # still works for compiled-to-compiled chains where
            # frame_depth is 0 throughout.
            self._emit_uplevel_body(body_parts)
            if eval_idx is not None:
                self._emit_call(eval_idx)
            else:
                self._emit_i32_const(0)
            return

        # For ``#0`` we pass a large shift (INT32_MAX / 2) so
        # frame_depth_stash clamps to zero regardless of the actual depth.
        shift = 0x3FFF_FFFF if level_spec == "#0" else up
        saved_local = self._add_extra_local(prefix="_uplevel_saved", val_type=ValType.I32)

        self._emit_i32_const(shift)
        self._emit_call(stash_idx)
        self._emit_local_set(saved_local)

        self._emit_uplevel_body(body_parts)
        self._emit_call(eval_idx)
        # Result TclObj is on stack; stash temporarily so we can restore.
        result_local = self._add_extra_local(prefix="_uplevel_result", val_type=ValType.I32)
        self._emit_local_set(result_local)

        self._emit_local_get(saved_local)
        self._emit_call(restore_idx)

        self._emit_local_get(result_local)

    def _emit_uplevel_body(self, parts: list[str]) -> None:
        """Push the uplevel body as a single TclObj, resolving any
        ``$var``/``[cmd]`` substitutions the parts contain before
        handing the final string to ``tcl_eval``.

        Typical cases:
          - ``uplevel #0 {set ::g 42}``  — parts = ["set ::g 42"], a
            literal script.  Emitted as an obj_literal.
          - ``uplevel "set ::$name $val"`` — parts = ["set ::$name $val"],
            an interpolated string.  Emitted via ``_emit_value`` so the
            concat chain resolves ``$name``/``$val`` before eval.
          - ``uplevel 1 a b c`` — parts = ["a","b","c"], Tcl concat-join
            with spaces.  Each part is resolved and joined.
        """
        if not parts:
            self._emit_obj_literal("")
            return
        if len(parts) == 1:
            self._emit_value(parts[0])
            return
        # Multi-part: emit each value obj and concat with spaces.
        # Simplest: use _emit_value for each and chain tcl_append.
        append_idx = self._shared_imports.get("tcl_append")
        if append_idx is None:
            # No concat helper — best-effort: fall back to the first part
            # (typical callers supply a single braced script).
            self._emit_value(parts[0])
            return
        self._emit_value(parts[0])
        for part in parts[1:]:
            self._emit_obj_literal(" ")
            self._emit_call(append_idx)
            self._emit_value(part)
            self._emit_call(append_idx)

    def _emit_clock_value(self, args: tuple[str, ...]) -> None:
        """Emit a ``clock <subcmd>`` expression; leaves i32 TclObj on stack.

        ``seconds``/``clicks``/``milliseconds`` call the WASI-backed
        runtime helpers directly.  Anything else falls through to the
        interpreter (which will likely trap for ``format``/``scan`` in
        the sandbox — that's fine as a clear diagnostic until we ship a
        timezone-aware formatter).
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]
        import_key = _CLOCK_SUBCMD_IMPORT.get(subcmd)
        if import_key is not None:
            func_idx = self._shared_imports.get(import_key)
            if func_idx is not None:
                self._emit_call(func_idx)
                return
        # Fall back to the interpreter for unsupported subcommands.
        self._emit_eval_fallback("clock", args)

    def _emit_cmd_lassign(
        self,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        *,
        keep_on_stack: bool,
    ) -> None:
        """``lassign list ?varName ...?`` — destructure a list into vars.

        For each ``varName`` at position *i*, assigns ``list_index(list, i)``
        to that variable (empty string if out of range).  Returns the
        leftover list (elements beyond the supplied variables) — computed
        at runtime via ``tcl_list_tail``.

        If *keep_on_stack* is True (value/expression context), the
        leftover-list i32 TclObj is left on the stack; otherwise the
        result is stored in ``defs[0]`` if given, or dropped.
        """
        if not args:
            self._emit_unsupported_trap("lassign (no list arg)")
            return
        list_arg = args[0]
        var_names = args[1:]

        # Stash the list value once so we can index into it repeatedly
        # without re-evaluating its expression (which may have side effects
        # via command substitution).
        list_local = self._add_extra_local(prefix="_lassign_list", val_type=ValType.I32)
        self._emit_value(list_arg)
        self._emit_local_set(list_local)

        # Per-variable: write list_index(list, i) into the named variable.
        lindex_idx = self._shared_imports.get("tcl_list_index")
        if lindex_idx is None:
            # Can't emit without the runtime helper — fall back to trap.
            self._emit_unsupported_trap("lassign (missing tcl_list_index)")
            return

        for i, var_name in enumerate(var_names):
            self._emit_local_get(list_local)
            # Index argument is a TclObj holding the integer i.
            self._emit_obj_literal(str(i))
            self._emit_call(lindex_idx)
            self._emit_var_write_obj(var_name)

        # Produce the leftover list (elements from index len(var_names) on).
        ltail_idx = self._shared_imports.get("tcl_list_tail")
        if ltail_idx is None:
            # No tail helper — emit empty list.
            self._emit_i32_const(0)
        else:
            self._emit_local_get(list_local)
            self._emit_obj_literal(str(len(var_names)))
            self._emit_call(ltail_idx)

        if keep_on_stack:
            return
        # Statement context: the leftover list is discarded.  ``defs`` for
        # lassign lists the VAR_WRITE targets (var_names), NOT a place to
        # store the command's return value — ignore it here to avoid
        # overwriting one of the just-assigned locals.
        self._emit(WasmOp.DROP)

    def _emit_info_value(self, args: tuple[str, ...]) -> None:
        """Leave the i32 TclObj result of ``info <args>`` on the stack.

        ``info exists varName`` is inlined so that compile-time scope
        information (alias bindings, ``::``-qualified globals) informs the
        lookup; everything else falls through to the runtime
        ``info_dispatch`` helper for subcommands it understands.
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]

        # ``info level 0`` — return the current proc's invocation
        # list.  The prologue stashed the real argv via
        # ``frame_set_argv`` (a proper list TclObj built with
        # ``tcl_list``), so the inline shortcut just reads it back
        # with ``frame_get_argv(0)``.  This matches tclsh's
        # semantics exactly: the list's length, element contents,
        # and quoting are all consistent with the invocation site.
        #
        # Falls through to the eval fallback when the required
        # imports aren't available (``frame_get_argv`` is pulled
        # in with the other frame primitives whenever the module
        # defines procs, so this is really a belt-and-braces
        # guard).
        if (
            subcmd == "level"
            and len(args) == 2
            and args[1] == "0"
            and self._is_proc
            and self._proc_qname is not None
        ):
            get_argv_idx = self._shared_imports.get("tcl_frame_get_argv")
            if get_argv_idx is not None:
                # offset 0 = current (topmost) frame's argv.
                self._emit_i32_const(0)
                self._emit_call(get_argv_idx)
                return
            # Fall through when the import is absent — the generic
            # ``info`` dispatch below will pick it up and route to
            # ``info_dispatch`` at runtime.

        if subcmd == "exists" and len(args) >= 2:
            # ``info exists var`` — resolve the variable reference with
            # compile-time alias info so upvar'd / variable'd / ``::``-
            # qualified names hit the global table rather than searching
            # for an unrelated local.
            var = args[1]
            # ``info exists $dynamicName`` / ``info exists [cmd]`` —
            # the NAME of the variable to check is produced at runtime
            # from a substitution.  Resolve the name value at runtime
            # and dispatch through the runtime ``info_exists`` helper
            # so the check follows the name's actual string, not the
            # literal text after ``$``.
            is_dynamic = (var.startswith("$") or var.startswith("[")) and _parse_array_ref(
                var
            ) is None
            if is_dynamic:
                info_exists_idx = self._shared_imports.get("tcl_info_exists")
                if info_exists_idx is not None:
                    self._emit_value(var)
                    self._emit_call(info_exists_idx)
                    return
                # Helper missing — fall through to the interpreter so
                # the answer is correct even if less efficient.
                self._emit_eval_fallback("info", args)
                return
            # Normalise ``$name`` / ``${name}`` just in case the lowerer
            # hasn't stripped the sigil.
            resolved = self._resolve_var_name(var) or var
            # ``info exists arr(key)`` — array element lookup.  The array
            # name itself may be alias-bound (upvar/variable) so resolve
            # it through _emit_array_name_obj, then probe the element
            # table via tcl_array_element_exists.
            array_ref = _parse_array_ref(resolved)
            if array_ref is not None:
                elem_idx = self._shared_imports.get("tcl_array_element_exists")
                if elem_idx is not None:
                    arr, key = array_ref
                    self._emit_array_name_obj(arr)
                    self._emit_value(key)
                    self._emit_call(elem_idx)
                    return
                # Runtime helper missing — conservative false.
                self._emit_i64_const(0)
                self._emit_box_int()
                return
            gexist_idx = self._shared_imports.get("tcl_global_exists")
            binding = self._aliases.get(resolved)
            if binding is not None and binding[0] == "global" and gexist_idx is not None:
                _, target_idx = binding
                self._emit_local_get(target_idx)
                self._emit_call(gexist_idx)
                return
            if resolved.startswith("::") and gexist_idx is not None:
                self._emit_obj_literal(resolved)
                self._emit_call(gexist_idx)
                return
            # FRAME-only vars (routed through tcl_local_set by the
            # escape-aware writer) don't land in a WASM local, so the
            # pointer-nullness check below would always say "no".
            # Dispatch through the runtime ``tcl_info_exists`` helper,
            # which probes the frame table by name and returns a
            # boxed TclObj 0/1 (matching the other existence paths).
            if self._is_frame_only_var(resolved):
                info_exists_idx = self._shared_imports.get("tcl_info_exists")
                if info_exists_idx is not None:
                    self._emit_obj_literal(resolved)
                    self._emit_call(info_exists_idx)
                    return
            # Plain proc-local: the compiled proc uses WASM locals
            # that are zero-initialised, so "exists" is approximated
            # as "value pointer is non-null" — matches writes that
            # land through _emit_var_write_obj.
            idx = self._local_index.get(resolved)
            if idx is None:
                # Never referenced — always non-existent.  Emit boxed 0.
                self._emit_i64_const(0)
                self._emit_box_int()
                return
            self._emit_local_get(idx)
            self._emit_i32_const(0)
            self._emit(WasmOp.I32_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit_box_int()
            return

        # Subcommands dispatched via the runtime's info_dispatch helper.
        info_dispatch_idx = self._shared_imports.get("tcl_info_dispatch")
        if info_dispatch_idx is not None and subcmd in ("body", "args", "exists"):
            self._emit_obj_literal(subcmd)
            if len(args) >= 2:
                self._emit_value(args[1])
            else:
                self._emit_i32_const(0)
            self._emit_call(info_dispatch_idx)
            return

        # Unknown subcommand — fall back to the interpreter.
        self._emit_eval_fallback("info", args)

    def _emit_cmd_runtime(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...],
    ) -> None:
        """Emit a call to an imported runtime function for a known command."""
        import_key, expected_argc = _CMD_RUNTIME[command]
        func_idx = self._shared_imports.get(import_key)
        if func_idx is None:
            self._emit_unsupported_trap(command)
            return

        # Record a diag site for runtime-dispatched commands whose
        # stubs can trap (``unsupported command: X`` from I/O / FS /
        # event / coroutine stubs, ``regexp`` on bad patterns, dict /
        # clock error paths, etc.) so stderr's ``tcl trap: site=<id>``
        # line resolves to the right source location.  Commands in
        # ``_CMD_RUNTIME_NONTRAPPING`` (currently just ``puts`` and
        # ``append`` — see the set definition in ``_imports.py``)
        # are total for every arg shape the codegen emits and
        # never raise into ``tcl_diag``, so the per-call
        # ``tcl_diag_set`` preamble (~4 WASM bytes + one DiagSite
        # record) is pure overhead for them.  ``lappend`` /
        # ``lindex`` etc. still emit diag sites because they can
        # trap on malformed list values.
        if command not in _CMD_RUNTIME_NONTRAPPING:
            self._emit_diag_site(command, args=args, kind="runtime")

        spec = _RUNTIME_IMPORTS[import_key]
        param_count = len(spec[2])

        # For commands like append/lappend that mutate a variable,
        # the first arg is the variable name; the runtime receives the
        # current variable value + one new value and returns the
        # updated value.  ``append x a b c`` concatenates a, b, c onto
        # x in order; ``lappend x a b c`` appends each as a separate
        # element.  Both loop per-value and store the running result
        # back between each call.
        mutates_var = command in ("append", "lappend")
        if mutates_var and len(args) >= 2:
            var_name = args[0]
            var_idx = self._intern_local(var_name)
            for value_arg in args[1:]:
                self._emit_local_get(var_idx)  # current value
                self._emit_value(value_arg)  # value to append
                self._emit_call(func_idx)
                self._emit_local_set(var_idx)
            return

        # ``list`` handled outside _emit_cmd_runtime — see
        # ``_emit_list_value`` for the N-arg build.

        # ``format`` takes a format string + up to 3 arg TclObjs —
        # fewer args get zero-padded so the runtime sees empty slots.
        if command == "format":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                for slot in range(1, 4):
                    if slot < len(args):
                        self._emit_value(args[slot])
                    else:
                        self._emit_i32_const(0)
            self._emit_call(func_idx)
            if spec[3]:
                if defs:
                    def_idx = self._intern_local(defs[0])
                    self._emit_local_set(def_idx)
                else:
                    # Keep on stack if result matters (value context)
                    # — here we're in statement context so drop.
                    self._emit(WasmOp.DROP)
            return

        # fconfigure takes a fd + a variable number of ``-option value``
        # pairs; the runtime fn has a 2-arg signature (fd, opts_obj),
        # so pack args[1:] into a single space-joined string literal
        # here and hand the resulting TclObj to the stub.  For
        # arguments that are variable references (``$value``) we
        # can't fold at compile time; in that case we emit a
        # ``tcl_concat`` chain at runtime.  Most fconfigure call
        # sites in the wild use only literal option names and
        # literal values, so the fast path is common.
        if command == "fconfigure":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                rest = args[1:]
                if not rest:
                    self._emit_i32_const(0)
                elif all(not a.startswith("$") and not a.startswith("[") for a in rest):
                    self._emit_obj_literal(" ".join(rest))
                else:
                    # Mixed literals + refs — build via repeated
                    # ``tcl_concat(acc, " word")``.  tcl_concat is
                    # always imported via the lifecycle set.
                    concat_idx = self._shared_imports.get("tcl_concat")
                    if concat_idx is None:
                        self._emit_obj_literal(" ".join(rest))
                    else:
                        self._emit_obj_literal(rest[0])
                        for word in rest[1:]:
                            self._emit_obj_literal(" ")
                            self._emit_call(concat_idx)
                            self._emit_value(word)
                            self._emit_call(concat_idx)
            self._emit_call(func_idx)
            if spec[3]:
                if defs:
                    def_idx = self._intern_local(defs[0])
                    self._emit_local_set(def_idx)
                else:
                    self._emit(WasmOp.DROP)
            return

        # For puts, handle optional channel argument: puts ?-nonewline? ?channelId? string
        if command == "puts":
            # ``puts -nonewline <string>`` dispatches to a newline-
            # suppressing runtime helper.  Channel-id forms (e.g.
            # ``puts stdout foo``) still fall through to the default
            # tcl_cmd_puts call.
            nonewline = len(args) >= 2 and args[0] == "-nonewline"
            if nonewline:
                no_nl_idx = self._shared_imports.get("tcl_puts_nonewline")
                if no_nl_idx is not None:
                    self._emit_value(args[-1])
                    self._emit_call(no_nl_idx)
                    if spec[3]:
                        self._emit(WasmOp.DROP)
                    return
            # Use the last argument as the string value
            if args:
                self._emit_value(args[-1])
            else:
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            if spec[3]:
                # puts returns empty string — drop
                self._emit(WasmOp.DROP)
            return

        # concat: variadic — Tcl concat trims whitespace from each arg and
        # joins non-empty results with a single space.
        if command == "concat":

            def _concat_is_lit(a: str) -> bool:
                return (
                    not a.startswith("$")
                    and not a.startswith("[")
                    and not self._has_embedded_subst(a)
                    and a not in self._aliases
                    and a not in self._local_index
                )

            all_lits = all(_concat_is_lit(a) for a in args)
            if not args:
                self._emit_obj_literal("")
            elif all_lits:
                # Compile-time: trim each IR value (already de-braced and
                # backslash-substituted by the lexer), drop empties, join.
                parts = [a.strip() for a in args]
                self._emit_obj_literal(" ".join(p for p in parts if p))
            else:
                # Mixed literals + runtime values: chain tcl_cmd_concat calls.
                # tcl_cmd_concat trims each arg's whitespace before joining.
                self._emit_value(args[0])
                for a in args[1:]:
                    self._emit_value(a)
                    self._emit_call(func_idx)
            if defs and spec[3]:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                self._emit(WasmOp.DROP)
            return

        # ``linsert list index v1 ?v2 ...?`` with multiple values —
        # ``tcl_cmd_list_insert`` is a single-value export, so we chain
        # per-value inserts at the same index.  The iteration order
        # depends on how the index resolves against the growing list:
        #
        #   * Numeric indices stay pinned to the same position through
        #     each insert, so inserting in *reverse* value order
        #     produces the correct forward layout
        #     (``{before} v1 v2 … vN {after}``).
        #   * ``end`` / ``end-N`` indices re-resolve after every insert
        #     (``end`` moves as the list grows).  Here *forward* value
        #     order is correct because each iteration's index lands at
        #     ``end-N + (i-1)`` — right after the previously inserted
        #     value.
        #
        # The textual shape of the index tells us which strategy to
        # use at compile time; a ``$var`` index whose runtime value is
        # an ``end-N`` string would be mis-ordered, but that is an
        # uncommon shape we accept for now.
        if command == "linsert" and len(args) > param_count:
            list_arg = args[0]
            index_arg = args[1]
            values = args[2:]
            self._emit_value(list_arg)
            if _is_end_relative_index(index_arg):
                iter_values = values
            else:
                iter_values = tuple(reversed(values))
            for v in iter_values:
                self._emit_value(index_arg)
                self._emit_value(v)
                self._emit_call(func_idx)
            if defs and spec[3]:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                self._emit(WasmOp.DROP)
            return

        # ``lreplace list first last v1 ?v2 ...?`` with multiple values —
        # emit one ``lreplace`` for one value (consumes the range
        # [first..last] and drops it in that slot), then chain inserts
        # for the remaining values.  Same index-type-dependent ordering
        # as linsert above: numeric ``first`` uses reverse value order
        # after the base replace; ``end``-family uses forward.
        if command == "lreplace" and len(args) > param_count:
            list_arg = args[0]
            first_arg = args[1]
            last_arg = args[2]
            values = args[3:]
            list_insert_idx = self._shared_imports.get("tcl_list_insert")
            if list_insert_idx is None or not values:
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[0] if values else "")
                self._emit_call(func_idx)
            elif _is_end_relative_index(first_arg):
                # ``end-N`` — replace the range with the *first* value,
                # then forward-insert the rest at ``first+1``.  Because
                # ``end-N`` grows with the list, each subsequent insert
                # lands immediately after the previous.  Note we use
                # ``first`` (not ``first+1`` as compile-time arithmetic)
                # because ``end-N`` has already moved forward by one
                # after the replace grew the list.
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[0])
                self._emit_call(func_idx)
                for v in values[1:]:
                    self._emit_value(first_arg)
                    self._emit_value(v)
                    self._emit_call(list_insert_idx)
            else:
                # Numeric index — replace with the *last* value, then
                # insert the earlier values in reverse at ``first``.
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[-1])
                self._emit_call(func_idx)
                for v in reversed(values[:-1]):
                    self._emit_value(first_arg)
                    self._emit_value(v)
                    self._emit_call(list_insert_idx)
            if defs and spec[3]:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                self._emit(WasmOp.DROP)
            return

        # ``lsort``/``lsearch``/``lindex``-style commands accept a
        # trailing positional ``list`` preceded by optional ``-switches``.
        # The runtime export is the no-switch form, so when extra args
        # are present pick the trailing positional as the list rather
        # than the leading ``-option`` token — otherwise ``lsort
        # -integer {3 1 2}`` would dispatch with the list ``-integer``
        # and produce the single-element result ``-integer``.
        if command in ("lsort",) and len(args) > param_count:
            self._emit_value(args[-1])
            for _ in range(param_count - 1):
                self._emit_i32_const(0)
        else:
            # Generic: push args up to param_count (all i32 TclObj pointers)
            for i in range(min(param_count, len(args))):
                self._emit_value(args[i])
            # Pad missing args with null TclObj
            for _ in range(param_count - len(args)):
                self._emit_i32_const(0)

        self._emit_call(func_idx)

        # Store result in def variable if present
        if defs and spec[3]:
            def_idx = self._intern_local(defs[0])
            self._emit_local_set(def_idx)
        elif spec[3]:
            self._emit(WasmOp.DROP)

    def _emit_cmd_proc_call(
        self,
        func_idx: int,
        n_params: int,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        qname: str | None = None,
        tokens: "CommandTokens | None" = None,
        invoked_name: str | None = None,
    ) -> None:
        """Emit a direct call to a compiled procedure.

        Enforces WASM arity: pushes exactly *n_params* i32 TclObj
        pointers onto the stack — padding with the proc's declared
        defaults (via ``_proc_defaults``) for missing args, or
        boxed zero when a param has no default.  Surplus args are
        dropped (matches Tcl's behaviour for procs without ``args``
        catchall).

        When the callee has an ``args`` variadic tail (its qname is in
        ``_proc_args_tail``), all call-site args beyond the fixed
        positional count are packed into a single list TclObj and
        passed as the last argument.

        *tokens* provides the original parsed tokens for each call-site
        word, letting us distinguish braced (``{…}``) args from
        unbraced ones.  Braced args must skip backslash / interpolation
        substitution at emit time (Tcl semantics: braces suppress all
        substitution).

        *invoked_name* is the exact word the source used for the
        call (``::foo::bar``, ``bar``, ``renamed_bar``, …).  Stashed
        in the runtime's pending-argv0 slot immediately before the
        ``call`` so the callee's prologue can report it via
        ``info level 0``.  When ``None`` — callers that don't have
        the source word handy — the callee's prologue falls back
        to its qname tail.
        """
        # The callee's prologue (see :meth:`generate`) applies
        # declared defaults on null-TclObj slots, so callers just
        # pad with ``i32.const 0`` via :meth:`_emit_default_arg`
        # for missing args — no per-call-site default lookup
        # needed.
        has_args_tail = qname is not None and qname in self._proc_args_tail
        argv0_local = (
            self._emit_prepare_pending_argv0(invoked_name) if invoked_name is not None else None
        )

        def _was_braced(call_arg_idx: int) -> bool:
            """Return True if the call-site word at *call_arg_idx* was braced.

            *call_arg_idx* is 0-based into *args* (the CALLSITE args tuple),
            so ``tokens.argv`` is indexed at ``call_arg_idx + 1`` to skip
            the command-name token.
            """
            if tokens is None or tokens.argv is None:
                return False
            tok_idx = call_arg_idx + 1
            if tok_idx >= len(tokens.argv):
                return False
            return tokens.argv[tok_idx].type == TokenType.STR

        if has_args_tail and n_params > 0:
            # Fixed positional slots: first n_params-1
            fixed = n_params - 1
            for i in range(min(fixed, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing fixed slots with defaults / null
            for _slot in range(len(args), fixed):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)
            # Last slot: list of all remaining call args.  Pass a
            # per-tail-arg ``was_braced`` probe so braced tokens in
            # the args tail keep their exact source bytes (no
            # backslash subst) when packed into the list.
            tail_args = args[fixed:]
            self._emit_args_list(
                tail_args,
                was_braced_fn=lambda i, base=fixed: _was_braced(base + i),
            )
        else:
            # Push arguments up to the callee's parameter count
            for i in range(min(n_params, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing args with the declared default or boxed zero.
            # Defaults are LITERAL values from the param spec — ``{$y}``
            # and ``{[clock seconds]}`` must reach the callee unchanged,
            # *not* be substituted at call time.  Emit as an obj literal
            # so no ``$var`` / ``[cmd]`` interpolation happens.
            for _slot in range(len(args), n_params):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)

        # Publish the caller's invoked word to the pending-argv0
        # slot right before the call — the callee's prologue
        # consumes it on entry.
        self._emit_push_pending_argv0(argv0_local)
        self._emit_call(func_idx)

        # Store result in def variable if present
        if defs:
            def_idx = self._intern_local(defs[0])
            self._emit_local_set(def_idx)
        else:
            # Drop unused return value in statement context
            self._emit(WasmOp.DROP)
