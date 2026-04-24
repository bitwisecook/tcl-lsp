"""_WasmEmitterCmdHelpersMixin: per-command emit helpers (return / array /
list / uplevel / clock / lassign / info / unset).

These are the command-specific implementations that ``cmds/*.py`` hooks
delegate to via ``emitter._emit_cmd_foo()``.  Keeping them on the emitter
(rather than as free functions in the hooks) lets them access the
emitter's shared state — locals, aliases, imports, namespace block —
without threading every field through each call.

``_commands.py`` keeps the generic runtime-dispatch machinery
(``_emit_cmd_runtime``, ``_runtime_call_end``, ``_emit_cmd_proc_call``)
— the logic that runs for every runtime-backed command regardless of
shape.  Per-command branches live here.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .._encoding import (
    _tcl_list_quote,
    _tcl_token_value,
)
from .._imports import (
    subcommand_runtime_import_for,
)
from .._ir import (
    ValType,
    WasmOp,
)
from .._parsing import (
    _parse_array_ref,
)


class _WasmEmitterCmdHelpersMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...
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
        """Handle ``unset`` for array-element and whole-variable forms.

        For ``unset arr(key)``: emits ``tcl_array_unset_element``.
        For ``unset arr`` when ``arr`` is an upvar global alias: emits
        ``array_unset(target)`` + ``global_set(target, 0)`` so that the
        entire array table is cleared in addition to nulling the global.
        For ``unset arr`` where ``arr`` is not an alias: returns False so
        the caller falls through to the eval fallback.

        Returns True if at least one unset was emitted; False otherwise.
        """
        had_any = False
        elem_fidx = self._shared_imports.get("tcl_array_unset_element")
        arr_fidx = self._shared_imports.get("tcl_array_unset")
        gset_fidx = self._shared_imports.get("tcl_global_set")
        # Skip ``-nocomplain`` / ``--`` option prefix.
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "--":
                i += 1
                break
            i += 1
        for name in args[i:]:
            ref = _parse_array_ref(name)
            if ref is not None:
                # Array element: unset arr(key)
                had_any = True
                arr, key = ref
                if elem_fidx is None:
                    continue
                self._emit_array_name_obj(arr)
                self._emit_value(key)
                self._emit_call(elem_fidx)
                self._emit(WasmOp.DROP)
                continue
            # Whole-variable unset: ``unset var``
            # Only handle when var is a global alias (upvar #0) — we know
            # the target name at compile time and can clear both the array
            # table and the global slot.  Scalars and frame locals fall
            # through to the eval fallback.
            binding = self._aliases.get(name)
            if binding is not None and binding[0] == "global":
                had_any = True
                target_idx = binding[1]
                if arr_fidx is not None:
                    self._emit_local_get(target_idx)
                    self._emit_call(arr_fidx)
                    self._emit(WasmOp.DROP)
                if gset_fidx is not None:
                    self._emit_local_get(target_idx)
                    self._emit_i32_const(0)
                    self._emit_call(gset_fidx)
                    self._emit(WasmOp.DROP)
        return had_any

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
        body_local = self._add_extra_local(prefix="_uplevel_body", val_type=ValType.I32)

        # Resolve the body string while still in the current frame so that
        # any $var/$[cmd] substitutions read from the correct (callee) frame,
        # not the stashed-to caller frame.
        self._emit_uplevel_body(body_parts)
        self._emit_local_set(body_local)

        self._emit_i32_const(shift)
        self._emit_call(stash_idx)
        self._emit_local_set(saved_local)

        self._emit_local_get(body_local)
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
        sri = subcommand_runtime_import_for("clock", subcmd)
        if sri is not None:
            import_key = sri.import_key
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
            aexist_idx = self._shared_imports.get("tcl_array_exists")
            binding = self._aliases.get(resolved)
            if binding is not None and binding[0] == "global":
                _, target_idx = binding
                if aexist_idx is not None and gexist_idx is not None:
                    # ``info exists arr`` for an upvar global alias must return 1
                    # when *either* the global scalar is set OR the array table
                    # exists (pure-array variables have no scalar slot).
                    ogi_idx = self._shared_imports["tcl_obj_get_int"]
                    self._emit_local_get(target_idx)
                    self._emit_call(aexist_idx)
                    self._emit_call(ogi_idx)  # i64: 0 or 1
                    self._emit_local_get(target_idx)
                    self._emit_call(gexist_idx)
                    self._emit_call(ogi_idx)  # i64: 0 or 1
                    self._emit(WasmOp.I64_OR)  # i64: 1 if either exists
                    self._emit_box_int()
                    return
                if gexist_idx is not None:
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
