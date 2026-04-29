"""_WasmEmitterVarMixin: variable reads/writes and frame sync."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .._ir import (
    _BLOCK_VOID,
    ValType,
    WasmOp,
)
from .._ownership import Ownership
from .._parsing import (
    _parse_array_ref,
)


class _WasmEmitterVarMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_var_read_obj(self, name: str) -> None:
        """Push the current TclObj value of local Tcl variable *name* on the stack.

        For aliased variables (``upvar``/``variable``), this reads through
        ``tcl_global_get`` using the target name stashed at alias-setup time.
        For ``::``-qualified names (e.g. ``::ns::var``), this reads the
        global directly.  Array references (``arr(key)``) dispatch to
        ``tcl_array_get`` with the element's stored value.  Otherwise
        this is a plain WASM ``local.get`` of the interned local.
        """
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            self._emit_array_element_read(*array_ref)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                gget_idx = self._shared_imports.get("tcl_global_get")
                if gget_idx is not None:
                    self._emit_local_get(target_idx)
                    self._emit_call(gget_idx)
                    return
        if name.startswith("::"):
            gget_idx = self._shared_imports.get("tcl_global_get")
            if gget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_idx)
                return
        # Mirror of the write-side fix: inside a ``namespace eval
        # ::ns { … }`` body, unqualified reads must look up the
        # ``::ns::<name>`` global rather than a bare local — because
        # we wrote it through the global table with the qualified
        # name, and a bare-local read would find 0.
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            gget_idx = self._shared_imports.get("tcl_global_get")
            if gget_idx is not None:
                qname = f"{self._block_namespace}::{name}"
                self._emit_obj_literal(qname)
                self._emit_call(gget_idx)
                return
        # Var-escape FRAME branch: the escape analysis proved this
        # name is observed by the interpreter or a callee upvar, so
        # its authoritative value lives in the runtime frame.  Read
        # through ``tcl_local_get`` (frame-scoped; does NOT fall through
        # to globals — that would alias with top-level vars of the same
        # name).
        if self._is_frame_only_var(name):
            lget_idx = self._shared_imports.get("tcl_local_get")
            if lget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(lget_idx)
                return
        # Top-level reads must consult the global table directly —
        # not the WASM-local mirror — because eval-fallback paths
        # (interp-side ``set``, ``regexp pat str whole a b`` capture
        # var assignments, etc.) write straight to globals without
        # invalidating the compiled-side mirror.  Without this branch,
        # ``set v 1; eval {set v 2}; puts $v`` printed ``1`` because
        # the compiled read used the stale WASM-local seeded by the
        # original ``set v 1``.  Phase 4.5 finalisation.
        if not self._is_proc:
            gget_idx = self._shared_imports.get("tcl_global_get")
            if gget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_idx)
                return
        idx = self._intern_local(name)
        self._emit_local_get(idx)

    def _emit_var_write_obj(
        self,
        name: str,
        *,
        source: Ownership | None = None,
    ) -> None:
        """Consume a TclObj value on the stack and write it to local Tcl variable *name*.

        For aliased variables, the value is routed to ``tcl_global_set``
        using the stashed target name.  For plain variables, the value is
        stored in the interned WASM local; if the name was declared
        ``global`` it's also written back to the global table.

        *source* (S2.3): when set, the plain WASM-local path uses
        :meth:`_emit_owned_local_write` so the slot's refcount stays
        correct for frame-elided procs.  When ``None`` the old plain
        ``local.set`` / ``local.tee`` path is used — the caller hasn't
        been migrated yet.
        """
        self._emit_var_write_obj_impl(name, keep_on_stack=False, source=source)

    def _emit_var_write_obj_keep(
        self,
        name: str,
        *,
        source: Ownership | None = None,
    ) -> None:
        """Like _emit_var_write_obj but leaves the written value on the stack.

        Used in tail-position ``set``/``incr`` emissions where the command's
        return value is the proc's return value.
        """
        self._emit_var_write_obj_impl(name, keep_on_stack=True, source=source)

    def _emit_var_write_obj_impl(
        self,
        name: str,
        *,
        keep_on_stack: bool,
        source: Ownership | None = None,
    ) -> None:
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            self._emit_array_element_write(array_ref[0], array_ref[1], keep_on_stack=keep_on_stack)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                self._emit_global_set_via_local(target_idx, keep_on_stack=keep_on_stack)
                return
        if name.startswith("::"):
            # ``::``-qualified names always refer to globals.  Route the
            # write directly without creating an unused WASM local.
            self._emit_global_set_via_literal(name, keep_on_stack=keep_on_stack)
            return
        # Inside a ``namespace eval ::ns { … }`` body, unqualified
        # writes must land in the ``::ns::`` table, not in a local
        # or in ``::``.  Without this, ``namespace eval ns { set x
        # 5 }`` wrote ``::x`` (overwriting/creating the wrong
        # global) and reads of ``$::ns::x`` came back empty.  We
        # qualify and route through ``tcl_global_set`` with the
        # full name; no local mirror, because the block may be
        # re-entered in a different frame (tcltest's stage-2 body
        # is a sequence of ``namespace eval ::tcltest { … }``
        # blocks).
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            qname = f"{self._block_namespace}::{name}"
            self._emit_global_set_via_literal(qname, keep_on_stack=keep_on_stack)
            return
        # Var-escape FRAME branch: the escape analysis proved this
        # name is observed by the interpreter or a callee upvar, so
        # its authoritative value must live in the runtime frame.
        # Route the write through ``tcl_local_set``.  Also mirror to a
        # WASM local so subsequent reads of this name in the same
        # basic block could fast-path — but we conservatively skip the
        # mirror today because ``_emit_var_read_obj`` already goes
        # through the frame for FRAME-tagged names.
        if self._is_frame_only_var(name):
            lset_idx = self._shared_imports.get("tcl_local_set")
            if lset_idx is not None:
                # Stack holds [value].  Need [name_obj, value] for
                # tcl_local_set.  Stash value in a scratch local so we
                # can push name_obj first.
                tmp = self._add_extra_local(prefix="_frame_set_tmp", val_type=ValType.I32)
                self._emit_local_set(tmp)
                self._emit_obj_literal(name)
                self._emit_local_get(tmp)
                self._emit_call(lset_idx)
                if keep_on_stack:
                    self._emit(WasmOp.DROP)  # drop tcl_local_set return
                    self._emit_local_get(tmp)  # leave value on stack
                else:
                    self._emit(WasmOp.DROP)
                return

        # At the top level (``::top``) there's no ``proc`` scope — every
        # variable is in the global namespace.  Route writes through
        # ``tcl_global_set`` AND keep a WASM local mirror so later reads
        # at the same level can use the fast path.  Without the global
        # mirror, an interpreter fallback at top level (e.g. a
        # dynamically-registered ``proc $varName`` call) can't see the
        # variable — eval-fallback always resolves via the global
        # table when no frame is active.
        if not self._is_proc:
            gset_idx = self._shared_imports.get("tcl_global_set")
            if gset_idx is not None:
                idx = self._intern_local(name)
                # Stack holds [value].  Need [name_obj, value] for gset.
                tmp = self._add_extra_local(prefix="_gset_tmp", val_type=ValType.I32)
                self._emit_local_set(tmp)
                # Also mirror to the WASM local for fast in-top reads.
                self._emit_local_get(tmp)
                self._emit_local_set(idx)
                self._emit_obj_literal(name)
                self._emit_local_get(tmp)
                self._emit_call(gset_idx)
                if keep_on_stack:
                    self._emit(WasmOp.DROP)  # drop gset return
                    self._emit_local_get(tmp)  # leave value on stack
                else:
                    self._emit(WasmOp.DROP)  # drop gset return
                return
        idx = self._intern_local(name)
        if source is not None:
            # S2.3: caller passed an explicit ownership tag; route
            # through the retain/release wrap.  When the proc is
            # framed or top-level, the wrap is a no-op fallback to
            # plain local.set / local.tee, so this is safe to call
            # unconditionally once the caller migrates.
            self._emit_owned_local_write(idx, source, keep_on_stack=keep_on_stack)
        elif keep_on_stack:
            self._emit_local_tee(idx)
        else:
            self._emit_local_set(idx)
        self._emit_global_writeback(name, idx)

    def _emit_array_name_obj(self, arr: str) -> None:
        """Push the TclObj containing the runtime name of array *arr*.

        Honours ``upvar`` / ``variable`` aliases so ``upvar #0
        counter::T-$tag counter; set counter(N) 0`` hits the correct
        per-tag array.  ``::``-qualified names pass through as
        literals.  Inside a ``namespace eval ::ns { ... }`` body
        unqualified names are prefixed with the block's namespace
        so ``set Option(-match) *`` lands in ``::ns::Option(-match)``
        rather than the bare global ``Option(-match)``.
        """
        binding = self._aliases.get(arr)
        if binding is not None and binding[0] == "global":
            self._emit_local_get(binding[1])
            return
        if arr.startswith("::"):
            self._emit_obj_literal(arr)
            return
        if (
            not self._is_proc
            and self._block_namespace is not None
            and self._block_namespace != "::"
        ):
            self._emit_obj_literal(f"{self._block_namespace}::{arr}")
            return
        self._emit_obj_literal(arr)

    def _emit_array_element_read(self, arr: str, key: str) -> None:
        """``$arr(key)`` — emit tcl_array_get(arr_name, key) and leave the
        i32 TclObj result on the stack.
        """
        func_idx = self._shared_imports.get("tcl_array_get")
        if func_idx is None:
            self._emit_i32_const(0)
            return
        self._emit_array_name_obj(arr)
        self._emit_value(key)
        self._emit_call(func_idx)

    def _emit_array_element_write(self, arr: str, key: str, *, keep_on_stack: bool) -> None:
        """``set arr(key) value`` — value is already on the stack.

        Stashes the value, pushes (arr_name, key, value) and calls
        tcl_array_set.
        """
        func_idx = self._shared_imports.get("tcl_array_set")
        if func_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        tmp = self._add_extra_local(prefix="_arr_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_array_name_obj(arr)
        self._emit_value(key)
        self._emit_local_get(tmp)
        self._emit_call(func_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_set_via_local(self, target_local_idx: int, *, keep_on_stack: bool) -> None:
        """Consume a value on the stack, write it to the global whose name
        is held in WASM local *target_local_idx*.  Leaves the value on the
        stack if *keep_on_stack*.
        """
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        # Stack: [value].  We need [target_name, value] for tcl_global_set.
        tmp = self._add_extra_local(prefix="_gset_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_local_get(target_local_idx)
        self._emit_local_get(tmp)
        self._emit_call(gset_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_set_via_literal(self, target_name: str, *, keep_on_stack: bool) -> None:
        """Consume a value on the stack, write it to the global named *target_name*.

        Leaves the value on the stack if *keep_on_stack*.
        """
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        tmp = self._add_extra_local(prefix="_gset_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_obj_literal(target_name)
        self._emit_local_get(tmp)
        self._emit_call(gset_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_writeback(self, name: str, local_idx: int) -> None:
        """If *name* is a declared global, write the local value to the global table."""
        if name not in self._globals:
            return
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            return
        self._emit_obj_literal(name)
        self._emit_local_get(local_idx)
        self._emit_call(gset_idx)
        self._emit(WasmOp.DROP)

    def _emit_namespace_eval_bridge(
        self,
        script_parts: tuple[str, ...] | list[str],
        *,
        drop_result: bool,
        ns_name: str | None = None,
    ) -> bool:
        """Emit a ``namespace eval <ns> <script-parts...>`` bridge.

        Assembles the script at WASM level (so compiled-frame aliases
        like ``$arr($key)`` resolve correctly) then calls ``tcl_eval``
        with a ``frame_sync`` / ``frame_readback`` pair around it so
        the interpreter sees the caller's locals.

        When *ns_name* is provided the bridge wraps the eval in
        ``tcl_ns_set`` / ``tcl_ns_restore`` so script-level dispatches
        (e.g. ``proc $varName ...`` going through the eval-fallback
        path) see the right ``current_ns`` and qualify their proc
        names against the target namespace.  Without this every
        dynamic-name proc inside ``namespace eval ::ns { ... }``
        registered at root, breaking later ``[name]`` lookups from
        within the namespace.

        *drop_result* controls what happens to the eval result:
          - ``True`` (statement context): drop — stack returns to empty.
          - ``False`` (value / tail context): keep the i32 TclObj on
            the stack for the caller.

        Returns ``True`` when the bridge was emitted in full, ``False``
        when the required runtime imports aren't available — the
        caller should fall through to its normal fallback path
        (usually ``_emit_eval_fallback``) in that case.
        """
        eval_idx = self._shared_imports.get("tcl_eval")
        if eval_idx is None:
            return False
        if not script_parts:
            # No script → empty string is a valid no-op.
            if not drop_result:
                self._emit_obj_literal("")
            return True
        if len(script_parts) == 1:
            self._emit_value(script_parts[0])
        else:
            append_idx = self._shared_imports.get("tcl_append")
            if append_idx is None:
                return False
            self._emit_value(script_parts[0])
            for sa in script_parts[1:]:
                self._emit_obj_literal(" ")
                self._emit_call(append_idx)
                self._emit_value(sa)
                self._emit_call(append_idx)
        # Push namespace context (if requested + the imports exist) so
        # the body's eval-fallback dispatches resolve names against
        # the target namespace.  ``tcl_ns_set`` returns the previously
        # active handle as i64; we stash it in a fresh local and pass
        # it back to ``tcl_ns_restore`` after the eval returns.
        ns_saved_local: int | None = None
        if (
            ns_name
            and not ns_name.startswith("$")
            and not ns_name.startswith("[")
            and "tcl_ns_set" in self._shared_imports
            and "tcl_ns_restore" in self._shared_imports
        ):
            ns_set_idx = self._shared_imports["tcl_ns_set"]
            ns_saved_local = self._add_extra_local(prefix="_ns_eval_saved", val_type=ValType.I64)
            offset = self._intern_string(ns_name)
            encoded = ns_name.encode("utf-8", errors="surrogatepass")
            self._emit_i32_const(offset + 4)
            self._emit_i32_const(len(encoded))
            self._emit_call(ns_set_idx)
            self._emit_local_set(ns_saved_local)
        self._emit_frame_sync()
        self._emit_call(eval_idx)
        if drop_result:
            self._emit(WasmOp.DROP)
        if ns_saved_local is not None:
            ns_restore_idx = self._shared_imports["tcl_ns_restore"]
            self._emit_local_get(ns_saved_local)
            self._emit_call(ns_restore_idx)
        self._emit_frame_readback()
        return True

    def _is_frame_only_var(self, name: str) -> bool:
        """True when the escape analysis says ``name`` must live in the runtime frame.

        Only returns True for non-pessimistic procs where analysis
        specifically proved the var escapes.  Pessimistic procs
        (``dynamic_barrier=True``) keep today's WASM-local-with-sync
        behaviour — routing every var through the frame primitives
        would be a much larger change and the sync path is already
        correct for that case.

        Vars already handled by other routing mechanisms are excluded:
        ``::``-qualified globals go through ``tcl_global_*``;
        ``global`` / ``variable`` declared names go through the globals
        table too; upvar aliases have their own binding.  Those paths
        own the interpreter-visible storage and must not be short-
        circuited by the frame-only branch.
        """
        summary = self._escape_summary
        if summary is None or summary.dynamic_barrier:
            return False
        if not self._is_proc:
            # At top level there's no frame; FRAME tags are meaningless
            # and the writer already routes ``::``-prefixed and
            # block-namespace writes through ``tcl_global_set``.
            return False
        if name in self._globals:
            return False
        if name in self._aliases:
            return False
        if name.startswith("::"):
            return False
        return summary.is_frame(name)

    def _iter_sync_locals(self, vars_used: set[str] | None = None) -> "list[tuple[str, int]]":
        """Return the ``(name, local_idx)`` pairs that should take part
        in a frame sync / readback at the current emit site, sorted by
        name for deterministic WASM output across Python-dict iteration
        orders.

        *vars_used* is the conservative set of local names the upcoming
        interpreter call could observe.  ``None`` means "unknown — sync
        every Tcl local".  An empty set also means the caller
        explicitly knows nothing is referenced (no sync needed).

        When a per-proc var-escape summary is available, the caller
        passes ``vars_used`` (the fallback's statically known reference
        set), and the proc is not in the dynamic-barrier pessimistic
        state, vars the analysis proved the interpreter cannot observe
        by name (``LOCAL`` tag) are excluded from the sync set.

        The ``vars_used is not None`` guard is a soundness gate: an
        unknown fallback body could reference any var by name, so we
        still sync every tcl-visible local in that case.  The
        interprocedural pass only bounds *static* ``upvar`` sources;
        it can't constrain what a runtime eval body touches.
        """
        summary = self._escape_summary
        narrow_to_frame = (
            summary is not None and not summary.dynamic_barrier and vars_used is not None
        )

        pairs: list[tuple[str, int]] = []
        for name, idx in self._tcl_var_locals.items():
            if name in self._aliases:
                continue
            if name in self._globals:
                continue
            if name.startswith("::"):
                continue
            if vars_used is not None and name not in vars_used:
                continue
            if narrow_to_frame and summary is not None and not summary.is_frame(name):
                # Analysis proved this var cannot be observed by name
                # from the interpreter — no sync needed.
                continue
            pairs.append((name, idx))
        pairs.sort(key=lambda p: p[0])
        return pairs

    def _emit_frame_sync(self, vars_used: set[str] | None = None) -> None:
        """Mirror proc-locals into the call frame before the Zig
        interpreter takes control (e.g. at a ``tcl_eval`` call site).

        Only called at interpreter-entry boundary points — not after every
        variable write — so procs that never reach the interpreter pay zero
        frame-sync overhead.

        *vars_used* narrows the sync to locals the upcoming script is
        statically known to reference (``$name`` / ``${name}``).  When
        ``None`` (or script scanning failed) we fall back to syncing
        every Tcl local, matching the original conservative behaviour.
        An empty set skips the sync entirely.

        A null-guard (``if (local != 0) { … }``) around each
        ``tcl_local_set`` call ensures that WASM locals that were never
        assigned (still zero-initialised) do not create phantom frame entries
        that would make ``info exists`` return true for unset variables.

        Skipped when:
        - not inside a compiled proc (``_is_proc`` is False)
        - ``tcl_local_set`` is not imported (frame not active)
        """
        if not self._is_proc:
            return
        lset_idx = self._shared_imports.get("tcl_local_set")
        if lset_idx is None:
            return
        for name, idx in self._iter_sync_locals(vars_used):
            # Emit: if (local != 0) { tcl_local_set(name_obj, local) }
            # Skips zero-initialised locals that were never assigned so they
            # don't create spurious frame entries.
            self._emit_local_get(idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_obj_literal(name)
            self._emit_local_get(idx)
            self._emit_call(lset_idx)
            self._emit(WasmOp.DROP)
            self._emit(WasmOp.END)

    def _emit_frame_readback(self, vars_used: set[str] | None = None) -> None:
        """Reload proc-locals from the call frame after the interpreter
        returns.  See :func:`_emit_frame_sync` for the *vars_used*
        contract — the same set must be used here so writes by the
        interpreter (``set x 99`` through an aliased name) surface back
        into the compiled proc's WASM locals.
        """
        if not self._is_proc:
            return
        lget_idx = self._shared_imports.get("tcl_local_get")
        if lget_idx is None:
            return
        for name, idx in self._iter_sync_locals(vars_used):
            self._emit_obj_literal(name)
            self._emit_call(lget_idx)
            self._emit_local_set(idx)

    def _register_global_alias(self, local_name: str, target_value: str) -> None:
        """Register *local_name* as a global alias and stash the target name.

        Emits code that evaluates *target_value* (which may be an
        interpolated string like ``counter::T-$tag``) to a TclObj, stores
        it in a hidden WASM local, and records the binding so subsequent
        reads/writes of *local_name* route through tcl_global_{get,set}.
        """
        target_idx = self._add_extra_local(prefix=f"_alias_{local_name}_tgt", val_type=ValType.I32)
        self._emit_value(target_value)
        self._emit_local_set(target_idx)
        self._aliases[local_name] = ("global", target_idx)

    def _emit_cmd_upvar(self, args: tuple[str, ...]) -> None:
        """``upvar ?level? otherVar myVar ?otherVar myVar ...?``

        Only the ``#0`` (global alias) form is currently compiled.  For
        any other level the module traps at runtime with a descriptive
        message — a future enhancement will add caller-frame alias
        support via a new runtime hook.
        """
        if len(args) < 2:
            self._emit_unsupported_trap("upvar (too few args)")
            return

        # Determine whether args[0] is a level specifier.  A leading '#'
        # always marks one; a bare digit sequence does too.  Anything
        # else means the level defaults to 1 and args[0] is otherVar.
        first = args[0]
        if first.startswith("#"):
            level_spec = first
            pairs_start = 1
        elif first.lstrip("-").isdigit():
            level_spec = first
            pairs_start = 1
        else:
            level_spec = "1"
            pairs_start = 0

        pair_args = args[pairs_start:]
        if len(pair_args) & 1:
            self._emit_unsupported_trap("upvar (uneven var pairs)")
            return

        if level_spec != "#0":
            # Caller-frame aliasing not yet implemented — trap with a
            # clear message rather than silently mis-compiling.
            self._emit_unsupported_trap(
                f"upvar level {level_spec} — compiled procs only support #0 "
                "(global alias).  Caller-frame aliasing (upvar N) needs "
                "the frame-pushing variant of compiled procs; use uplevel "
                "to run code in the caller instead."
            )
            return

        for i in range(0, len(pair_args), 2):
            target = pair_args[i]
            local_name = pair_args[i + 1]
            self._register_global_alias(local_name, target)

    def _emit_cmd_variable(self, args: tuple[str, ...]) -> None:
        """``variable name ?value? ?name value ...?``

        Inside a namespace proc, aliases ``name`` to ``::ns::name`` in
        the enclosing namespace.  If a value is provided and the
        namespace variable does not already exist, it is initialised.

        At namespace-eval top-level (``namespace eval ::ns {
        variable debug 0 }``), the ``?value?`` form still needs to do
        the write — ``debug 0`` must land in ``::ns::debug``.  The
        previous implementation treated ns-eval variables as a no-op
        claiming assignments "already hit globals", but bare ``set x
        5`` in the body was writing to ``::x`` (the wrong global);
        reads of ``::ns::x`` came back empty, and tcltest's ``$debug
        >= 1`` compared ``""`` against ``1`` (empty-string comparisons
        pass as truthy here), forcing every ``DebugPuts`` to fire.
        """
        if not args:
            return
        if not self._is_proc:
            # Namespace-eval top-level — no frame to alias into,
            # but the ``?value?`` pairs still need their writes.
            # Route through ``_emit_var_write_obj`` so the
            # namespace-qualification logic in that path picks
            # ``_block_namespace`` (set by the enclosing ``IRBlock``)
            # and writes ``::ns::<name>`` to the global table.
            # Bare ``variable name`` with no initializer is a
            # declaration that does nothing in our compiled model
            # (reads are already auto-qualified by block-ns).
            i = 0
            while i < len(args):
                name = args[i]
                has_value = i + 1 < len(args)
                if has_value:
                    self._emit_value(args[i + 1])
                    self._emit_var_write_obj(name)
                    i += 2
                else:
                    i += 1
            return

        ns = self._proc_namespace or "::"
        i = 0
        while i < len(args):
            name = args[i]
            has_value = i + 1 < len(args)
            # Dynamic name — ``variable $varName ?value?`` as used by
            # tcltest's ``Default`` / ``Option``.  Build the qualified
            # target at runtime (``<ns>::<$name-value>``) and — when a
            # value is supplied — write it to the global table via
            # ``tcl_global_set``.  The compile-time alias registration
            # (used for later ``$local`` reads in the proc body) is
            # skipped because the name only exists at runtime; users
            # who want a local alias can follow up with an explicit
            # ``upvar``.
            is_dynamic = (name.startswith("$") or name.startswith("[")) and _parse_array_ref(
                name
            ) is None
            if is_dynamic:
                gset_idx = self._shared_imports.get("tcl_global_set")
                append_idx = self._shared_imports.get("tcl_append")
                gexist_idx = self._shared_imports.get("tcl_global_exists")
                if has_value and gset_idx is not None and append_idx is not None:
                    # Construct ``<ns>::<name>`` at runtime by
                    # concatenating the namespace prefix with the
                    # dynamic name value via ``tcl_append``.  Stash
                    # the result in a hidden local so the existence
                    # check and the write can reuse it without
                    # recomputing the concat.
                    ns_prefix = f"{ns}::" if ns != "::" else "::"
                    qname_idx = self._add_extra_local(prefix="_var_dyn_qname", val_type=ValType.I32)
                    self._emit_obj_literal(ns_prefix)
                    self._emit_value(name)
                    self._emit_call(append_idx)
                    self._emit_local_set(qname_idx)
                    # ``variable`` only initialises when the variable
                    # does not already exist — check via
                    # ``tcl_global_exists`` and skip the write on hit.
                    if gexist_idx is not None:
                        self._emit_local_get(qname_idx)
                        self._emit_call(gexist_idx)
                        self._emit_call(self._shared_imports["tcl_obj_get_int"])
                        self._emit(WasmOp.I64_EQZ)
                        self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                        self._emit_local_get(qname_idx)
                        self._emit_value(args[i + 1])
                        self._emit_call(gset_idx)
                        self._emit(WasmOp.DROP)
                        self._emit(WasmOp.END)
                    else:
                        self._emit_local_get(qname_idx)
                        self._emit_value(args[i + 1])
                        self._emit_call(gset_idx)
                        self._emit(WasmOp.DROP)
                    i += 2
                else:
                    # Bare ``variable $name`` declaration or missing
                    # runtime helpers — nothing to emit at compile time;
                    # reads of the local ``name`` won't route through a
                    # namespace alias, but that's the same degraded
                    # mode we had before dynamic support landed.
                    i += 2 if has_value else 1
                continue

            # Resolve the target name: ``::ns::name`` for unqualified
            # names, or the literal name if already qualified with ``::``.
            if name.startswith("::"):
                target = name
                # The local alias uses the final segment as its Tcl name.
                local_name = name.rsplit("::", 1)[-1]
            else:
                if ns == "::":
                    target = f"::{name}"
                else:
                    target = f"{ns}::{name}"
                local_name = name

            # Stash the target literal in a hidden local and register.
            target_idx = self._add_extra_local(
                prefix=f"_var_{local_name}_tgt", val_type=ValType.I32
            )
            self._emit_obj_literal(target)
            self._emit_local_set(target_idx)
            self._aliases[local_name] = ("global", target_idx)

            if has_value:
                # ``variable name value`` — initialise the namespace var
                # unconditionally (matches Tcl 8.x behaviour in practice
                # for scripted modules; upstream tcltest uses it idempotently).
                self._emit_value(args[i + 1])
                self._emit_var_write_obj(local_name)
                i += 2
            else:
                i += 1

    # -- Individual command emitters --
