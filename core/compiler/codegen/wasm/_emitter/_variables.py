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

    def _emit_unset_check_with_alias(self, error_name: str) -> bool:
        """Wrap a value already on the stack with an unset-variable check
        that raises with the supplied *error_name* when the value is null.

        Stack on entry: ``[value]``.  Stack on exit: ``[value]``.

        Emits:
        ::
            local.tee tmp
            i32.eqz
            if
              <error_name TclObj literal>
              call tcl_var_unset_error
            end
            local.get tmp

        Returns True when the wrap was emitted.  Returns False (with the
        original value untouched on the stack) when ``tcl_var_unset_error``
        is not imported — the caller should treat this as a degraded
        mode and emit a plain read.

        *error_name* is the user-facing name to surface in the error
        message.  Aliased reads (``upvar #0 g x``) pass the local alias
        name (``x``) here even though the lookup happens on the target
        (``g``) — Tcl's error wording reports the alias identifier the
        source code referenced, not the resolved target.

        Catch-context limitation (shared by every compiled error path):
        outside a catch, ``tcl_var_unset_error`` writes the diagnostic
        to stderr and traps, so any later WASM ops in the same
        statement never run.  Inside a catch, it sets ``error_flag``
        and returns; the catch boundary's ``tcl_catch_has_error`` /
        ``br_if`` probe only fires after the *current* statement
        finishes emitting, so the (zero) value left on the stack here
        can flow into a downstream operator (``set x [expr {$undef +
        1}]`` would still attempt the ``set x``).  This is the same
        behaviour as the existing ``tcl_arith_div`` divide-by-zero
        and ``stubs.raise`` paths and is consistent across the
        compiled error model — a post-error side-effect within the
        same statement is observable but the catch sees the error
        first and ``catch_result`` returns the diagnostic.  A real
        fix needs a within-statement abort hook that's out of scope
        for this issue; tracked in the wasm-codegen design doc.
        """
        unset_err_idx = self._shared_imports.get("tcl_var_unset_error")
        if unset_err_idx is None:
            return False
        # Reuse a single per-function scratch slot for every unset
        # peek.  Allocated lazily on first call so functions that
        # never read a variable don't grow their locals section, and
        # then reused so a proc with many reads doesn't accumulate one
        # ``_var_check_<n>`` slot per read site.
        if self._var_unset_check_scratch is None:
            self._var_unset_check_scratch = self._add_extra_local(
                prefix="_var_check", val_type=ValType.I32
            )
        tmp = self._var_unset_check_scratch
        self._emit_local_tee(tmp)
        self._emit(WasmOp.I32_EQZ)
        self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
        self._emit_obj_literal(error_name)
        self._emit_call(unset_err_idx)
        self._emit(WasmOp.END)
        self._emit_local_get(tmp)
        return True

    def _emit_var_read_obj(self, name: str) -> None:
        """Push the current TclObj value of local Tcl variable *name* on the stack.

        For aliased variables (``upvar``/``variable``), this reads through
        the lenient ``tcl_global_get`` using the target name stashed at
        alias-setup time, then wraps the result with the inline unset
        check so the error message reports the local alias name (the
        identifier the source code referenced) rather than the resolved
        target — matching reference Tcl's wording for ``upvar #0 g x;
        set x``.  For ``::``-qualified names (e.g. ``::ns::var``), this
        reads the global directly through the strict variant.  Array
        references (``arr(key)``) dispatch to ``tcl_array_get`` with
        the element's stored value.  Otherwise this is a plain WASM
        ``local.get`` of the interned local, followed by a runtime check
        that raises ``can't read "<name>": no such variable`` when the
        slot was never assigned.

        This is a user-visible variable read — every path raises the
        standard Tcl missing-variable error when the lookup resolves to
        the null TclObj handle (slot zero).  The lenient lookups
        (``tcl_global_get`` / ``tcl_local_get``) remain in use by paths
        that legitimately want the missing-is-fine behaviour:
        ``info exists`` / ``unset -nocomplain`` / frame readback after an
        eval-fallback / the ``global`` command's pre-load of a possibly-
        uninitialised slot.
        """
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            self._emit_array_element_read(*array_ref)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                # Aliased read — look up via the target name but
                # surface the *local* alias in any unset error so the
                # message matches reference Tcl (which reports the
                # identifier the source code wrote, not the resolved
                # target).
                gget_lenient = self._shared_imports.get("tcl_global_get")
                if gget_lenient is not None:
                    self._emit_local_get(target_idx)
                    self._emit_call(gget_lenient)
                    if not self._emit_unset_check_with_alias(name):
                        # Runtime helper missing — fall back to the
                        # strict variant which surfaces the target
                        # name; better than swallowing the unset.
                        gget_strict = self._shared_imports.get("tcl_global_get_or_error")
                        if gget_strict is not None:
                            # Drop the lenient-fetched value and redo
                            # via the strict path.
                            self._emit(WasmOp.DROP)
                            self._emit_local_get(target_idx)
                            self._emit_call(gget_strict)
                    return
            if kind == "frame_var":
                # ``upvar N other local`` — read through the runtime
                # frame's alias bucket.  ``tcl_local_get_or_error``
                # follows ALIAS_EXT into the target frame and raises
                # ``can't read "<local>": no such variable`` (with the
                # *local* name) when the target is unset, matching
                # reference Tcl's wording.
                lget_idx = self._shared_imports.get("tcl_local_get_or_error")
                if lget_idx is not None:
                    self._emit_obj_literal(name)
                    self._emit_call(lget_idx)
                    return
        if name.startswith("::"):
            gget_idx = self._shared_imports.get("tcl_global_get_or_error")
            if gget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_idx)
                return
        # Mirror of the write-side fix: inside a ``namespace eval
        # ::ns { … }`` body, unqualified reads must look up the
        # ``::ns::<name>`` global rather than a bare local — because
        # we wrote it through the global table with the qualified
        # name, and a bare-local read would find 0.  Surface the
        # unqualified source-level name in any unset error so the
        # message matches what the user wrote.
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            gget_lenient = self._shared_imports.get("tcl_global_get")
            if gget_lenient is not None:
                qname = f"{self._block_namespace}::{name}"
                self._emit_obj_literal(qname)
                self._emit_call(gget_lenient)
                if not self._emit_unset_check_with_alias(name):
                    gget_strict = self._shared_imports.get("tcl_global_get_or_error")
                    if gget_strict is not None:
                        self._emit(WasmOp.DROP)
                        self._emit_obj_literal(qname)
                        self._emit_call(gget_strict)
                return
        # Var-escape FRAME branch: the escape analysis proved this
        # name is observed by the interpreter or a callee upvar, so
        # its authoritative value lives in the runtime frame.  Read
        # through ``tcl_local_get_or_error`` (frame-scoped; does NOT
        # fall through to globals — that would alias with top-level
        # vars of the same name — and raises ``no such variable`` if
        # the frame slot is unset).
        if self._is_frame_only_var(name):
            # Phase 7: indexed slot fast path — replaces the
            # name-keyed ``tcl_local_get_or_error`` with
            # ``tcl_frame_local_at(idx)`` when the slot-resolution
            # pass assigned a slot for this name.  Falls back to
            # the legacy reader when no slot was assigned.
            slot_idx = self._local_slot_index(name)
            if slot_idx is not None:
                fla_idx = self._shared_imports.get("tcl_frame_local_at")
                if fla_idx is not None:
                    self._emit_i32_const(slot_idx)
                    self._emit_call(fla_idx)
                    if not self._emit_unset_check_with_alias(name):
                        # Indexed slot was 0 — raise the same error
                        # the name-keyed strict reader would have.
                        lget_idx_strict = self._shared_imports.get("tcl_local_get_or_error")
                        if lget_idx_strict is not None:
                            self._emit(WasmOp.DROP)
                            self._emit_obj_literal(name)
                            self._emit_call(lget_idx_strict)
                    return
            lget_idx = self._shared_imports.get("tcl_local_get_or_error")
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
            gget_idx = self._shared_imports.get("tcl_global_get_or_error")
            if gget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_idx)
                return
        # WASM-local-mirror read for proc-locals.  The slot defaults to 0
        # (null TclObj) when never assigned, so emit an inline check that
        # raises ``can't read "<name>": no such variable`` via
        # ``tcl_var_unset_error`` when the slot is still zero.  The check
        # is ``i32.eqz``-cheap on the hot path (taken slot is always
        # non-null) and only allocates the error message TclObj on the
        # cold (missing-variable) path.
        #
        # Eliding the check requires runtime — not just compile-time —
        # certainty that the slot is bound.  The only such case is a
        # proc parameter, which the call prologue stores the caller's
        # arg into before the body runs.  ``self._first_writes_seen``
        # is unsafe to use here: it tracks emission order, so a write
        # inside an ``if`` branch flips the flag at compile time even
        # when the runtime path skipped the branch.  ``if {$flag} {set
        # x 1}; set x`` would then take the elision fast path on the
        # second read and silently return 0 instead of erroring when
        # ``$flag`` was false.  Keeping the check on every non-param
        # read costs one ``i32.eqz``-cheap branch — worth it for the
        # correctness guarantee.
        idx = self._intern_local(name)
        if name in self._params:
            self._emit_local_get(idx)
            return
        # Read the WASM-local mirror; when it's null, probe the
        # runtime frame before raising.  A callee that did
        # ``uplevel N set <name> ...`` writes through the
        # interpreter's ``var_set`` to the *frame* table, not the
        # WASM-local mirror — so a same-name read here would
        # otherwise raise even though the variable now exists at
        # the runtime level.  Falls back to the original null-check
        # path when ``tcl_local_get`` isn't imported.
        lget_lenient = self._shared_imports.get("tcl_local_get")
        unset_err_idx = self._shared_imports.get("tcl_var_unset_error")
        if lget_lenient is not None and unset_err_idx is not None:
            if self._var_unset_check_scratch is None:
                self._var_unset_check_scratch = self._add_extra_local(
                    prefix="_var_check", val_type=ValType.I32
                )
            tmp = self._var_unset_check_scratch
            self._emit_local_get(idx)
            self._emit_local_tee(tmp)
            self._emit(WasmOp.I32_EQZ)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            # WASM-local is null: probe the runtime frame in case
            # an uplevel/upvar populated this slot.
            self._emit_obj_literal(name)
            self._emit_call(lget_lenient)
            self._emit_local_tee(tmp)
            self._emit(WasmOp.I32_EQZ)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_obj_literal(name)
            self._emit_call(unset_err_idx)
            self._emit(WasmOp.END)
            self._emit(WasmOp.END)
            self._emit_local_get(tmp)
            return
        self._emit_local_get(idx)
        if not self._emit_unset_check_with_alias(name):
            # Runtime helper unavailable — leave the read as plain
            # ``local.get``.  Degraded mode, same shape as before the
            # issue #263 fix.
            return

    def _emit_var_read_obj_lenient(self, name: str) -> None:
        """Lenient counterpart to :meth:`_emit_var_read_obj` — pushes the
        TclObj or 0 (null TclObj) if the variable is unset, instead of
        raising ``can't read "<name>": no such variable``.

        Used by ``incr`` (Tcl 8.5+: ``incr x`` on an unset scalar
        initialises it to ``0`` before adding, returning the increment;
        the strict-integer contract from issues #260–#262 only applies
        to non-empty values that fail the integer parse — :func:`tcl_incr`
        treats a null obj as ``0``).  Without this lenient variant,
        ``proc p {} { incr x }`` regresses from "returns 1" to
        "raises".

        The name dispatch (alias / array / namespace-eval / global /
        frame-only / proc-local) mirrors :meth:`_emit_var_read_obj`
        exactly; only the missing-variable branch differs.
        """
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            # Array element reads return 0 (null TclObj) for missing
            # elements via ``tcl_array_get`` — already lenient.
            self._emit_array_element_read(*array_ref)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                gget_lenient = self._shared_imports.get("tcl_global_get")
                if gget_lenient is not None:
                    self._emit_local_get(target_idx)
                    self._emit_call(gget_lenient)
                    return
            if kind == "frame_var":
                lget_lenient = self._shared_imports.get("tcl_local_get")
                if lget_lenient is not None:
                    self._emit_obj_literal(name)
                    self._emit_call(lget_lenient)
                    return
        if name.startswith("::"):
            gget_lenient = self._shared_imports.get("tcl_global_get")
            if gget_lenient is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_lenient)
                return
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            gget_lenient = self._shared_imports.get("tcl_global_get")
            if gget_lenient is not None:
                qname = f"{self._block_namespace}::{name}"
                self._emit_obj_literal(qname)
                self._emit_call(gget_lenient)
                return
        if self._is_frame_only_var(name):
            lget_lenient = self._shared_imports.get("tcl_local_get")
            if lget_lenient is not None:
                self._emit_obj_literal(name)
                self._emit_call(lget_lenient)
                return
        if not self._is_proc:
            gget_lenient = self._shared_imports.get("tcl_global_get")
            if gget_lenient is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_lenient)
                return
        # Plain proc-local mirror — slot defaults to 0 if never written.
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
            if kind == "frame_var":
                # ``upvar N other local`` — runtime frame holds an
                # ALIAS_EXT bucket on *local_name* pointing at the
                # caller's slot.  ``tcl_local_set`` chases the alias
                # and lands the write in the target frame's variable.
                self._emit_frame_var_set(name, keep_on_stack=keep_on_stack)
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
            # Phase 7: indexed slot fast path for FRAME writes.
            # ``tcl_frame_local_set_at(idx, value)`` skips the
            # name-keyed bucket scan entirely; the value-store
            # contract (retain new, release old) is identical.
            slot_idx = self._local_slot_index(name)
            if slot_idx is not None:
                flsa_idx = self._shared_imports.get("tcl_frame_local_set_at")
                if flsa_idx is not None:
                    tmp = self._add_extra_local(prefix="_frame_slot_tmp", val_type=ValType.I32)
                    self._emit_local_set(tmp)
                    self._emit_i32_const(slot_idx)
                    self._emit_local_get(tmp)
                    self._emit_call(flsa_idx)
                    if keep_on_stack:
                        self._emit(WasmOp.DROP)
                        self._emit_local_get(tmp)
                    else:
                        self._emit(WasmOp.DROP)
                    return
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
                self._emit(WasmOp.DROP)  # drop gset return
                # When the caller transferred a +1 (OWNED) and didn't
                # ask to keep the value on the stack, balance the extra
                # retain ``tcl_global_set`` did on store.  Without
                # this, every top-level ``set L X`` accumulates one
                # leaked rc on X — observable in the leak baseline and
                # fatal to the rc==1 fast path of mutators like
                # ``lappend`` (rc reads as 2, slow rebuild fires every
                # iteration).  BORROWED writes leave rc untouched
                # because the caller never owned the +1.  The
                # keep_on_stack branch leaves the value on the stack
                # for the caller to release, balancing the retain via a
                # later release at the consume site.
                if source is Ownership.OWNED and not keep_on_stack:
                    release_idx = self._shared_imports.get("tcl_obj_release")
                    if release_idx is not None:
                        self._emit_local_get(tmp)
                        self._emit_call(release_idx)
                if keep_on_stack:
                    self._emit_local_get(tmp)  # leave value on stack
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
        if binding is not None and binding[0] in ("global", "frame_var"):
            # For "global" aliases (upvar #0 / variable): target_idx holds
            # the name of the global.
            # For "frame_var" aliases (upvar N): target_idx holds the name
            # of the target variable in the caller frame.  Passing it to the
            # array helpers routes to the correct array in the global directory
            # (top-level arrays are stored there regardless of call depth).
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

    def _emit_frame_var_set(self, local_name: str, *, keep_on_stack: bool) -> None:
        """Consume a value on the stack and write it to a local whose
        runtime frame slot carries an ALIAS_EXT bucket (registered by
        ``upvar N`` / ``upvar #N``).  ``tcl_local_set`` follows the
        alias to the target frame's variable; the caller-frame slot is
        the authoritative storage.
        """
        lset_idx = self._shared_imports.get("tcl_local_set")
        if lset_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        tmp = self._add_extra_local(prefix="_uvr_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_obj_literal(local_name)
        self._emit_local_get(tmp)
        self._emit_call(lset_idx)
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

    def _local_slot_index(self, name: str) -> int | None:
        """Phase 7: return the compile-time slot index for ``name``
        if the slot-resolution pass assigned one, otherwise None.

        Consumed by the variable read/write paths to swap the
        name-keyed ``tcl_local_set`` / ``tcl_local_get_or_error``
        runtime calls for indexed accessors
        (``tcl_frame_local_set_at`` / ``tcl_frame_local_at``).
        """
        if not self._is_proc:
            return None
        summary = self._escape_summary
        if summary is None:
            return None
        return summary.local_slot_indices.get(name)

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
        # Frame-sync uses the silent setter so a write trace doesn't
        # observe each phantom assignment on every interpreter
        # boundary.  The user-visible writes still go through
        # ``tcl_local_set`` (with traces); the state-transfer
        # mirroring is invisible to ``trace add variable``.  Falls
        # back to the trace-firing setter only if the silent variant
        # isn't imported (older runtimes).
        lset_idx = self._shared_imports.get("tcl_local_set_silent")
        if lset_idx is None:
            lset_idx = self._shared_imports.get("tcl_local_set")
        if lset_idx is None:
            return
        for name, idx in self._iter_sync_locals(vars_used):
            # Emit: if (local != 0) { tcl_local_set_silent(name_obj, local) }
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
        # Mirror of ``_emit_frame_sync``: use the silent reader so
        # READ traces don't fire on every interpreter-boundary
        # readback.  Only user-visible reads (``$x`` / ``set x``)
        # should trigger a trace.
        lget_idx = self._shared_imports.get("tcl_local_get_silent")
        if lget_idx is None:
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

    def _emit_upvar_abs_depth(self, level_spec: str) -> bool:
        """Push an i32 absolute frame depth onto the stack.

        *level_spec* may be:

        * ``#N`` — literal absolute level; emits ``i32.const N``.
        * bare integer ``N`` (possibly with a leading ``-``) — relative
          level; emits ``frame_get_depth() - |N|``.
        * a dynamic value (``$level``, ``[expr ...]``) whose runtime
          string starts with ``#`` for absolute or is a bare integer
          for relative.  We emit a runtime helper call:
          ``tcl_upvar_resolve_depth(level_obj, frame_get_depth())``
          which mirrors Tcl's parsing.  When that helper is not
          available we default to relative-1 to keep optparse-style
          ``upvar $level $vname var`` working in the common case
          where the caller passes ``#N`` (e.g. OptKeyParse hands
          OptTreeVars ``"#[expr ...]"``); see
          ``tcl::OptKeyParse`` line 540.

        Returns True when the depth was pushed; False if the level
        could not be resolved at compile time and the caller should
        emit an unsupported trap.
        """
        depth_idx = self._shared_imports.get("tcl_frame_get_depth")
        if depth_idx is None:
            return False
        if level_spec.startswith("#"):
            try:
                self._emit_i32_const(int(level_spec[1:]))
                return True
            except ValueError:
                pass  # fall through to dynamic path
        elif not level_spec.startswith("$") and not level_spec.startswith("["):
            # Static relative integer.
            try:
                rel = abs(int(level_spec))
            except ValueError:
                rel = 1
            self._emit_call(depth_idx)
            self._emit_i32_const(rel)
            self._emit(WasmOp.I32_SUB)
            return True
        # Dynamic level value.  Resolve it through a runtime helper
        # that accepts a TclObj and returns the absolute depth.
        upvar_depth_idx = self._shared_imports.get("tcl_upvar_resolve_depth")
        if upvar_depth_idx is not None:
            self._emit_value(level_spec)
            self._emit_call(depth_idx)
            self._emit_call(upvar_depth_idx)
            return True
        # Helper not available — fall back to assuming the dynamic
        # value is a relative ``1`` (reference Tcl's default).  This
        # mis-compiles ``#0`` / ``#N`` callsites that pass a dynamic
        # level, but those are vanishingly rare in scripted code.
        self._emit_call(depth_idx)
        self._emit_i32_const(1)
        self._emit(WasmOp.I32_SUB)
        return True

    def _register_frame_var_alias(
        self, local_name: str, target_value: str, level_spec: str
    ) -> None:
        """Register *local_name* as a caller-frame alias and stash the
        target name + emit the runtime ``frame_alias_frame_var`` call so
        the runtime frame's bucket carries the alias descriptor.

        The local is then driven through the runtime's ``tcl_local_get``
        / ``tcl_local_set`` (which transparently chase ALIAS_EXT to the
        target frame's slot) — both compiled reads/writes and any
        interpreter-side eval inside the body see the same binding.
        """
        alias_idx = self._shared_imports.get("tcl_frame_alias_frame_var")
        depth_idx = self._shared_imports.get("tcl_frame_get_depth")
        if alias_idx is None or depth_idx is None:
            self._emit_unsupported_trap("upvar (frame-var alias runtime helpers missing)")
            return

        target_idx = self._add_extra_local(prefix=f"_uvr_{local_name}_tgt", val_type=ValType.I32)
        self._emit_value(target_value)
        self._emit_local_set(target_idx)

        abs_tmp = self._add_extra_local(prefix="_uvr_abs", val_type=ValType.I32)
        if not self._emit_upvar_abs_depth(level_spec):
            self._emit_unsupported_trap(f"upvar level {level_spec} (could not resolve depth)")
            return
        self._emit_local_set(abs_tmp)

        # frame_alias_frame_var(local_name_obj, abs_depth, target_obj)
        self._emit_obj_literal(local_name)
        self._emit_local_get(abs_tmp)
        self._emit_local_get(target_idx)
        self._emit_call(alias_idx)

        self._aliases[local_name] = ("frame_var", target_idx)

    def _emit_cmd_upvar(self, args: tuple[str, ...]) -> None:
        """``upvar ?level? otherVar myVar ?otherVar myVar ...?``

        ``#0`` aliases the local to a global; non-zero levels (relative
        ``N`` or absolute ``#N``) register a caller-frame alias via
        ``frame_alias_frame_var`` so reads/writes of the local in the
        compiled body resolve through the runtime to the target frame's
        variable.  Both forms register a codegen-side alias entry so
        later ``$local`` / ``set local x`` route through the right
        runtime primitive.

        The first arg is treated as a level specifier when it has the
        ``#N`` / ``-?N`` literal shape OR when the remaining args have
        an odd count (since the var-pair list following a level must
        be even).  ``upvar $level $vname var`` (OptTreeVars) hits the
        odd-count branch even though ``$level`` is dynamic.
        """
        if len(args) < 2:
            self._emit_unsupported_trap("upvar (too few args)")
            return

        first = args[0]
        # Strict-literal level shapes.
        literal_level = first.startswith("#") or first.lstrip("-").isdigit()
        # Dynamic-level disambiguation: ``upvar`` requires an even
        # var-pair count after the optional level.  When the total
        # arg count is ODD the first MUST be a level token (statically
        # ``#N`` / ``N`` or dynamically a ``$x`` resolved at runtime).
        # ``upvar $level $vname var`` in OptTreeVars (3 args) is the
        # canonical dynamic case.
        odd_total = bool(len(args) & 1)
        if literal_level:
            level_spec = first
            pairs_start = 1
        elif odd_total:
            level_spec = first  # dynamic — handled by _emit_upvar_abs_depth
            pairs_start = 1
        else:
            level_spec = "1"
            pairs_start = 0

        pair_args = args[pairs_start:]
        if len(pair_args) & 1:
            self._emit_unsupported_trap("upvar (uneven var pairs)")
            return

        if level_spec == "#0":
            for i in range(0, len(pair_args), 2):
                target = pair_args[i]
                local_name = pair_args[i + 1]
                self._register_global_alias(local_name, target)
            return

        # Caller-frame alias path.
        for i in range(0, len(pair_args), 2):
            target = pair_args[i]
            local_name = pair_args[i + 1]
            self._register_frame_var_alias(local_name, target, level_spec)

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

            # Mirror the alias into the runtime frame as well so any
            # interpreter-side eval inside the proc body (a dynamic
            # ``while`` condition, an ``eval`` / ``catch`` body, the
            # eval-fallback path for an unknown command) sees the
            # same ``X`` -> ``::ns::X`` mapping the compiled code uses.
            # Without this, ``variable n`` declares the alias in the
            # codegen ``_aliases`` dict only -- compiled reads/writes
            # of ``$n`` go to ``::ns::n`` via ``tcl_global_set``,
            # but interpreter-side ``incr n`` (e.g. inside a ``while
            # {[info exists arr($n)]} { incr n }`` body) lands in a
            # fresh frame-local ``n`` and never propagates back.
            # See opt.test ``OptKeyRegister`` auto-allocation loop.
            alias_named_idx = self._shared_imports.get("tcl_frame_alias_named")
            if alias_named_idx is not None and self._is_proc:
                self._emit_obj_literal(local_name)
                self._emit_local_get(target_idx)
                self._emit_call(alias_named_idx)

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
