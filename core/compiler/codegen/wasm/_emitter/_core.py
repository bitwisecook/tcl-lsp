"""Core _WasmEmitterBase: __init__, local helpers, low-level emit ops, generate."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from .....analysis.semantic_model import Range
from ....cfg import (
    CFGFunction,
)
from ....ir import (
    IRModule,
    IRStatement,
)
from ....var_escape import ProcEscapeSummary
from .._encoding import (
    _leb128_signed,
    _leb128_unsigned,
)
from .._ir import (
    _BLOCK_VOID,
    DiagMap,
    ValType,
    WasmData,
    WasmFunction,
    WasmInstruction,
    WasmOp,
)
from .._ownership import Ownership
from .._parsing import (
    _derive_proc_namespace,
)

# Signed integer ranges for ``i32.const`` / ``i64.const`` LEB128
# saturation in :meth:`_WasmEmitterBase._emit_i32_const` and
# :meth:`_WasmEmitterBase._emit_i64_const`.  Tcl 9 supports BigInt
# literals; the wasm spec rejects encodings outside these ranges.
_I32_MIN = -(1 << 31)
_I32_MAX = (1 << 31) - 1
_I64_MIN = -(1 << 63)
_I64_MAX = (1 << 63) - 1


class _WasmEmitterBase:
    if TYPE_CHECKING:
        # From sibling mixins — declared here so generate() and helpers in _core.py
        # can resolve them when type-checked in isolation.
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _intern_string(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_block(self, *a: Any, **kw: Any) -> Any: ...
        def _run_optimisations(self, *a: Any, **kw: Any) -> Any: ...
        def _body_references_info_level(self, *a: Any, **kw: Any) -> Any: ...
        def _is_frame_only_var(self, *a: Any, **kw: Any) -> Any: ...

    def __init__(
        self,
        cfg: CFGFunction,
        params: tuple[str, ...] = (),
        *,
        optimise: bool = False,
        is_proc: bool = False,
        func_index_base: int = 0,
        shared_imports: dict[str, int] | None = None,
        proc_index: dict[str, tuple[int, int]] | None = None,
        proc_defaults: dict[str, tuple[str | None, ...]] | None = None,
        proc_args_tail: set[str] | None = None,
        shared_strings: list[tuple[str, int]] | None = None,
        shared_string_index: dict[str, int] | None = None,
        shared_string_offset: list[int] | None = None,
        proc_qname: str | None = None,
        diag_map: DiagMap | None = None,
        escape_summary: "ProcEscapeSummary | None" = None,
        proc_imports: dict[str, dict[str, str]] | None = None,
        frame_elision: bool = True,
    ) -> None:
        self._cfg = cfg
        self._params = params
        self._optimise = optimise
        self._is_proc = is_proc
        self._func_index_base = func_index_base
        # When False, every proc forces ``wants_frame=True`` regardless
        # of escape analysis.  S1.1: gives the codegen a kill-switch
        # so we can A/B the elided vs framed paths on the same source
        # and prove the framed path is the correctness floor S2 rolls
        # back to.
        self._frame_elision = frame_elision
        # Fully qualified proc name (e.g. ``::counter::init``) so the
        # ``variable`` command can resolve ``name`` → ``::counter::name``.
        # None for ``::top`` and other non-proc emitters.
        self._proc_qname = proc_qname
        self._proc_namespace = _derive_proc_namespace(proc_qname) if proc_qname else "::"

        # Shared state from module compilation
        self._shared_imports: dict[str, int] = shared_imports or {}
        # proc_index maps qualified name → (func_idx, n_params)
        self._proc_index: dict[str, tuple[int, int]] = proc_index or {}
        # Per-proc default-value strings, indexed by slot.  Consulted
        # by ``_emit_cmd_proc_call`` to pad missing call-site args
        # with the declared default rather than a boxed zero.
        self._proc_defaults: dict[str, tuple[str | None, ...]] = proc_defaults or {}
        # Set of proc qnames whose last formal parameter is ``args``
        # (Tcl's variadic catch-all).  When calling these procs,
        # surplus call-site args are packed into a single list TclObj
        # and passed as the last positional argument.
        self._proc_args_tail: set[str] = proc_args_tail or set()

        # Resolved ``namespace import`` table: ``{context_ns: {short:
        # qualified}}``.  Consulted by ``_resolve_proc`` /
        # ``_resolve_proc_qname`` so unqualified calls like ``test
        # name desc body`` dispatch directly to ``::tcltest::test``
        # (when ``namespace import ::tcltest::*`` was seen at lowering
        # time) rather than falling back to ``tcl_eval``.  Empty when
        # no namespace-import directives were present.
        self._proc_imports: dict[str, dict[str, str]] = proc_imports or {}

        # Local variable management
        self._local_names: list[str] = []
        self._local_types: list[ValType] = []
        self._local_index: dict[str, int] = {}
        # Subset of _local_index: only Tcl user-visible variables (not
        # internal scratch locals created by _add_extra_local).  Used
        # by _emit_frame_sync to mirror proc locals into the call frame
        # before handing control to the Zig interpreter.
        self._tcl_var_locals: dict[str, int] = {}
        # Tcl-variable WASM-local indices, in interning order.
        # Populated by ``_intern_local`` regardless of whether the
        # frame is elided — the actual retain/release wiring at the
        # write site only fires when ``self._wants_frame`` is False
        # (S2.2's primitive).  Also indexed in a set for O(1)
        # "is this slot owned?" lookups during emission.
        self._owned_locals: list[int] = []
        self._owned_locals_set: set[int] = set()
        # Set in ``generate()`` once the elision decision has been
        # made so the body emitters can decide whether to wrap each
        # write with retain/release.
        self._wants_frame: bool = False
        # Lazy scratch slot for the retain/release wrap.  Allocated on
        # first use of ``_emit_owned_local_write`` and reused for
        # every wrapped write in this proc.
        self._rc_set_scratch: int | None = None
        # Issue #263 — single scratch slot for the unset-variable
        # ``i32.eqz`` peek emitted by ``_emit_unset_check_with_alias``.
        # Lazily allocated on first use and reused for every read in
        # this function; without this, a non-trivial proc with N
        # variable reads grew the locals section by N redundant scratch
        # i32s, bloating the WASM binary and pushing more
        # ``_var_check_<n>`` slots through the linker.
        self._var_unset_check_scratch: int | None = None
        # S5.1 — set of slot indices that have been written at least
        # once during this emitter's body emission.  At the first
        # write (slot not yet in the set) the prior value is provably
        # 0 (WASM locals are zero-initialised at function entry), so
        # the wrap can skip the ``local.get idx; call release`` pair.
        # Conservative — the set is approximate across control flow
        # (a write inside an ``if`` adds the slot to the set even
        # though the join may not execute it), so we only use it for
        # the FIRST-WRITE optimisation, not for later writes.
        self._first_writes_seen: set[int] = set()
        # S5.2 — counter of fully-elided wraps (``set x $x`` / no-op
        # incr).  Bumped each time ``_emit_owned_local_write``
        # detects that the value about to be stored aliases the
        # slot's current contents and therefore the retain + release
        # pair cancels.  Surfaced via a getter for diagnostics.
        self._s52_alias_skipped: int = 0

        # Register parameters as locals
        for p in params:
            self._intern_local(p)

        # Instructions
        self._body: list[WasmInstruction] = []

        # String data accumulation — shared across all functions in the
        # module so data segments don't overlap in linear memory.
        # shared_string_offset is a single-element list so all emitters
        # mutate the same counter.
        self._strings: list[tuple[str, int]] = shared_strings if shared_strings is not None else []
        self._string_index: dict[str, int] = (
            shared_string_index if shared_string_index is not None else {}
        )
        self._string_offset_ref: list[int] = (
            shared_string_offset if shared_string_offset is not None else [0]
        )

        # Global variable tracking
        self._globals: set[str] = set()

        # Variable aliases — populated by ``upvar`` and ``variable``.
        # Maps a local Tcl variable name to an (kind, target_local_idx)
        # tuple where target_local_idx is a hidden WASM local holding the
        # resolved target name as an i32 TclObj pointer.
        #   kind == "global": reads/writes go through tcl_global_get/set
        #     using the TclObj name stashed at the alias site.
        # Caller-frame (``upvar 1 var local``) is not yet supported —
        # _emit_cmd_upvar traps at compile time for non-``#0`` levels.
        self._aliases: dict[str, tuple[str, int]] = {}

        # Catch depth — when > 0, unsupported commands call error()
        # without unreachable (error returns normally in catch context).
        self._catch_depth = 0

        # Block ordering for structured control flow
        self._visited: set[str] = set()

        # Active loop headers — used by _is_loop_header to distinguish
        # a nested-loop back-edge from an outer-loop back-edge. When we
        # walk from a body-start node looking for a cycle back to the
        # current branch block, reaching an active OUTER loop header
        # means we've followed the outer loop's back-edge, not found
        # a new nested loop; stop the walk in that case.
        self._active_loop_headers: set[str] = set()

        # Loop control — break/continue use WASM structured control flow.
        # The `_loop_depth` increments each time we enter a block{loop{}}
        # pair.  An `if` inside a loop body adds nesting that break/continue
        # must skip over.  We track the nesting depth in `_ctrl_depth` and
        # record the ctrl_depth at each loop entry in `_loop_ctrl_depths`.
        self._loop_depth = 0
        self._ctrl_depth = 0
        self._loop_ctrl_depths: list[int] = []
        # Snapshot of ``_catch_depth`` at the time each loop entry was
        # pushed onto ``_loop_ctrl_depths``.  Used by the ``break`` /
        # ``continue`` codegen to detect loops that sit OUTSIDE an
        # enclosing ``catch`` — those flow-control words must route
        # through the runtime so ``catch`` absorbs them as TCL_BREAK /
        # TCL_CONTINUE return codes rather than ``br`` past
        # ``catch_leave``.  Loops opened INSIDE the current catch keep
        # using the structured ``br`` (no catch boundary in the way).
        self._loop_catch_depths: list[int] = []

        # Constant propagation state (for optimised mode)
        self._const_map: dict[str, int] = {}

        # Source-location sidecar.  Shared across all emitters in a
        # module so a single monotonic ID space covers every trap-able
        # call site.  When ``None`` the emitter runs without diag
        # instrumentation (useful for tests that don't care about
        # source mapping); fallback / unsupported sites still work,
        # they just don't emit ``tcl_diag_set`` before the call.
        self._diag_map: DiagMap | None = diag_map

        # Active ``namespace eval`` block (if any).  When non-None,
        # unqualified command names inside the block are resolved to
        # ``<block_namespace>::<name>`` before proc-registry lookup,
        # matching Tcl's namespace path semantics for the common case
        # where a namespace body calls its own helper procs by bare
        # name (``Option -verbose …`` inside ``namespace eval
        # ::tcltest { … }`` really means ``::tcltest::Option``).
        self._block_namespace: str | None = None

        # Current statement's source range + command, updated at the
        # top of ``_emit_stmt`` and consulted by the fallback / trap
        # emitters to stamp a diag site.  Zero/empty until the first
        # statement is walked.
        self._current_range: Range | None = None
        self._current_command: str = ""
        self._current_args: tuple[str, ...] = ()

        # Per-proc var-escape summary.  When provided, _iter_sync_locals
        # uses it to skip variables the analysis proved cannot be seen by
        # name from the interpreter — trimming the frame-sync set below
        # what ``vars_used`` alone would allow.  ``None`` means "no
        # analysis run" and falls back to conservative behaviour.
        self._escape_summary: ProcEscapeSummary | None = escape_summary

    def _intern_local(self, name: str) -> int:
        """Register a Tcl variable as an i32 local (TclObj pointer).

        Parameters always get a WASM local slot and stay in the sync
        map — they come in as function args and the prologue mirrors
        them into the frame when ``wants_frame`` is set.

        Non-parameter FRAME-tagged vars skip ``_tcl_var_locals``: the
        runtime frame is their authoritative storage, so keeping them
        out of the sync map means the narrow-sync and readback paths
        ignore them entirely.  The slot itself is still allocated in
        case a stale index gets emitted on a fallback path that
        bypasses the escape-aware routing.
        """
        idx = self._local_index.get(name)
        if idx is not None:
            return idx
        idx = len(self._local_names)
        self._local_names.append(name)
        self._local_types.append(ValType.I32)
        self._local_index[name] = idx
        if name in self._params or not self._is_frame_only_var(name):
            self._tcl_var_locals[name] = idx
        # Every Tcl-variable slot is a candidate owned slot.  The
        # actual retain/release wrapping at the write site is gated
        # on ``self._wants_frame`` so the per-frame mode (where
        # ``frame_pop`` already manages the slot's refcount) does
        # not double-track.
        self._owned_locals.append(idx)
        self._owned_locals_set.add(idx)
        return idx

    def _add_extra_local(self, prefix: str = "_tmp", val_type: ValType = ValType.I64) -> int:
        """Create an internal temporary local with the given type.

        Internal locals default to i64 for arithmetic scratch space.
        """
        name = f"{prefix}_{len(self._local_names)}"
        idx = len(self._local_names)
        self._local_names.append(name)
        self._local_types.append(val_type)
        self._local_index[name] = idx
        return idx

    def _resolve_var_name(self, value: str) -> str | None:
        """Extract a variable name from a Tcl variable reference.

        Returns the bare variable name for ``$x`` or ``${x}`` patterns
        when *value* is the entire reference (nothing after it), or
        ``None`` otherwise.  Rejecting mixed forms like ``$x*`` or
        ``$x$y`` is essential — those are interpolated strings where
        the codegen has to emit a concat chain, not a single
        ``local.get`` of a variable named ``x*``.
        """
        if value.startswith("${") and value.endswith("}"):
            # ``${name}`` must not have anything past the closing
            # brace.  Check by scanning: the first ``}`` that
            # terminates the braced form must be at position
            # ``len - 1``.
            inner = value[2:-1]
            # ``${name}`` braces don't nest, so any inner ``}`` is
            # invalid Tcl — but we degrade gracefully by treating
            # it as non-simple (fall through to interpolation).
            if "}" in inner:
                return None
            return inner
        if value.startswith("$") and not value.startswith("$["):
            # Bare ``$name`` accepts only identifier characters in
            # *name* (letters, digits, underscore).  Anything else
            # — including ``*``, ``/``, ``-`` (used by tcltest
            # option names like ``-tmpdir``) — ends the variable
            # reference and starts literal-text territory, which
            # means the value is an interpolated string.
            #
            # Exception: ``$arr(key)`` is a single variable reference
            # (array element read) — once we see ``(`` after a valid
            # identifier head, consume through the matching ``)`` and
            # treat the whole thing as the variable name so that the
            # downstream ``_emit_var_read_obj`` path can dispatch it
            # to the array-element reader via ``_parse_array_ref``.
            i = 1
            while i < len(value):
                c = value[i]
                if c.isalnum() or c == "_":
                    i += 1
                    continue
                break
            if i < len(value) and value[i] == "(" and value.endswith(")"):
                # ``$arr(key)`` — the key may itself contain
                # substitutions; let downstream codegen handle it.
                return value[1:]
            if i != len(value):
                return None
            return value[1:]
        return None

    def _emit(self, op: int, operands: bytes = b"", *, label: str | None = None) -> None:
        if op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
            self._ctrl_depth += 1
        elif op == WasmOp.END:
            self._ctrl_depth -= 1
        self._body.append(
            WasmInstruction(
                op=op,
                operands=operands,
                range=self._current_range,
                label=label,
            )
        )

    def _emit_i64_const(self, value: int) -> None:
        # wasmtime rejects ``i64.const`` values outside signed 64-bit
        # range with ``invalid var_i64: integer too large``.  Tcl 9
        # supports BigInt literals (e.g. ``expr {0x8000000000000000}``
        # or ``-9223372036854775808`` parsed as unary-minus of a 2^63
        # literal), so a naive ``_leb128_signed(2**63)`` here produces
        # an LEB128 wasmtime refuses to validate and the WHOLE module
        # fails to instantiate.  Saturate to the i64 boundary; loads
        # of arbitrary-precision integers should reach this through
        # the string-literal path (see ``_emit_obj_literal_for_expr``)
        # which preserves the source text for the runtime to parse.
        if value > _I64_MAX:
            value = _I64_MAX
        elif value < _I64_MIN:
            value = _I64_MIN
        self._emit(WasmOp.I64_CONST, _leb128_signed(value))

    def _emit_i32_const(self, value: int) -> None:
        if value > _I32_MAX:
            value = _I32_MAX
        elif value < _I32_MIN:
            value = _I32_MIN
        self._emit(WasmOp.I32_CONST, _leb128_signed(value))

    def _emit_f64_const(self, value: float) -> None:
        """Emit an f64.const instruction (8-byte little-endian IEEE 754)."""
        import struct

        self._emit(WasmOp.F64_CONST, struct.pack("<d", value))

    def _emit_local_get(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(idx))

    def _emit_local_set(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))

    def _emit_local_tee(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))

    def _emit_call(self, func_idx: int) -> None:
        self._emit(WasmOp.CALL, _leb128_unsigned(func_idx))

    # -- S2.2: refcount-disciplined owned-slot write --------------------

    def _owned_local_wrap_active(self, idx: int) -> bool:
        """``True`` when a write to slot *idx* should be wrapped with
        ``tcl_obj_retain`` / ``tcl_obj_release``.

        The wrap fires only inside frame-elided procs whose escape
        analysis proved no runtime frame is needed.  In that mode
        the WASM-local mirror is the sole authoritative storage for
        the Tcl variable and must own a +1 reference to its current
        value.  When a frame is present ``frame_pop`` already
        balances the slot's refcount so the wrap stays off.
        Top-level (``::top``, ``self._is_proc=False``) and scratch
        slots (created via ``_add_extra_local`` so not in
        ``_owned_locals_set``) are excluded by virtue of the membership
        check.
        """
        if not self._is_proc:
            return False
        if self._wants_frame:
            return False
        if idx not in self._owned_locals_set:
            return False
        return (
            "tcl_obj_retain" in self._shared_imports and "tcl_obj_release" in self._shared_imports
        )

    def _peek_last_local_get_idx(self) -> int | None:
        """Return the immediate of the last instruction iff it is a
        ``local.get`` with no attached label, else ``None``.

        Used by the S5.2 alias-skip peephole.  We deliberately treat a
        labelled instruction as opaque — labels carry diagnostic
        intent the optimiser must not erase — and we only inspect the
        single most-recent entry, so any structural op (block / loop /
        end / br / br_if / call) intervening between a ``local.get``
        and the write helper disables the optimisation.
        """
        if not self._body:
            return None
        last = self._body[-1]
        if last.op != WasmOp.LOCAL_GET or last.label is not None:
            return None
        # Decode the unsigned LEB128 idx from the operands.
        idx = 0
        shift = 0
        for byte in last.operands:
            idx |= (byte & 0x7F) << shift
            if not (byte & 0x80):
                return idx
            shift += 7
        return None

    def _emit_owned_local_write(
        self,
        idx: int,
        source: "Ownership",
        *,
        keep_on_stack: bool,
    ) -> None:
        """Wrap a write to an owned WASM-local slot with the retain /
        release dance the S2 discipline requires.

        Stack on entry: ``[new_val]``.  Stack on exit: ``[new_val]``
        when *keep_on_stack* is True, otherwise empty.

        The emission depends on the value's :class:`Ownership` tag:

        - ``OWNED``: the value already brings a +1.  Just release the
          slot's prior value (if any) and store; the +1 transfers
          cleanly.
        - ``BORROWED``: the value is a borrow from another owning
          slot.  Retain to claim a separate +1, release the slot's
          prior, then store.

        Both ``tcl_obj_retain`` and ``tcl_obj_release`` are null-safe,
        so a self-set (``new_val == current``) and the very first
        write to a slot (still 0 from function entry) work without
        special-casing.

        When the wrap is inactive (framed proc, top-level, or scratch
        slot) the call falls back to a plain ``local.set`` /
        ``local.tee`` — semantically identical to today's path.  This
        lets call sites adopt ``_emit_local_set_owned`` without a
        sweep regression while S2.3 migrates writers one at a time.
        """
        if not self._owned_local_wrap_active(idx):
            if keep_on_stack:
                self._emit_local_tee(idx)
            else:
                self._emit_local_set(idx)
            return

        retain_idx = self._shared_imports["tcl_obj_retain"]
        release_idx = self._shared_imports["tcl_obj_release"]

        # S5.2 — alias-skip peephole.  If the value about to be stored
        # was just produced by ``local.get idx`` for the same slot,
        # the slot already holds the value and a BORROWED write is a
        # no-op: the retain (claim a +1) and the release (drop the
        # prior +1) cancel because they touch the same handle.  Drop
        # the trailing ``local.get`` for ``keep_on_stack=False``;
        # leave it in place when the caller wants the value back on
        # the stack.  OWNED is intentionally not optimised here — an
        # OWNED value carrying a fresh +1 must still release that
        # extra reference, so the wrap is not a pure cancellation.
        if source is Ownership.BORROWED:
            last_get = self._peek_last_local_get_idx()
            if last_get == idx:
                self._s52_alias_skipped += 1
                # Mark the slot as written even though we emitted
                # nothing — subsequent writes need to see this slot
                # as no-longer-pristine for the S5.1 first-write
                # gate.  (If this WAS the first write the slot held
                # 0, we wrote 0, still 0 — equivalent to no-op.)
                self._first_writes_seen.add(idx)
                if not keep_on_stack:
                    # Pop the just-emitted ``local.get idx`` so the
                    # runtime stack stays balanced.  Safe because
                    # ``_peek_last_local_get_idx`` already verified
                    # the instruction has no label.
                    self._body.pop()
                return

        # S5.1 — first-write optimisation.  The very first write to
        # a slot during emission has provably-0 prior value (WASM
        # locals init to 0 at function entry).  Skip the
        # ``local.get idx; call release`` pair — release(0) is a
        # null-safe no-op.  Note: param slots are NOT first-writes
        # at function entry because the WASM ABI seeded them from
        # caller args; the prologue's param-retain (S2.4) covers
        # those slots so they enter this emitter pre-set.  Treat
        # params as "already written" too via a seed at the start
        # of the body.
        #
        # **PR #237 review**: the "first emit-time write" check is
        # unsound when the emit site is inside a loop body — the
        # same wasm code runs once per iteration, so the
        # *runtime*-first iteration has prior=0 (correct) but
        # subsequent iterations carry the previous iteration's
        # value (must release).  When ``_loop_depth > 0`` we
        # force the release-prior path even for the first
        # compile-time emission, leaving the optimisation only on
        # straight-line "this slot truly hasn't been touched yet"
        # writes.
        first_write = idx not in self._first_writes_seen and self._loop_depth == 0
        self._first_writes_seen.add(idx)

        if source is Ownership.OWNED:
            # Stack: [v]; v owns its +1 we transfer to the slot.
            if first_write:
                # No prior to release — direct store transfers +1.
                if keep_on_stack:
                    self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))
                else:
                    self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))
                return
            if self._rc_set_scratch is None:
                self._rc_set_scratch = self._add_extra_local(
                    prefix="_rc_set_tmp",
                    val_type=ValType.I32,
                )
            tmp = self._rc_set_scratch
            self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(tmp))
            self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(idx))
            self._emit_call(release_idx)
            self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(tmp))
            if keep_on_stack:
                self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))
            else:
                self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))
            return

        # BORROWED: slot needs to claim its own +1.
        if first_write:
            # No prior to release.  Just retain the borrowed value
            # and store.  Stack: [v] -> retain pops v -> empty;
            # restore v from a scratch slot.
            if self._rc_set_scratch is None:
                self._rc_set_scratch = self._add_extra_local(
                    prefix="_rc_set_tmp",
                    val_type=ValType.I32,
                )
            tmp = self._rc_set_scratch
            self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(tmp))
            self._emit_call(retain_idx)
            self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(tmp))
            if keep_on_stack:
                self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))
            else:
                self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))
            return
        if self._rc_set_scratch is None:
            self._rc_set_scratch = self._add_extra_local(
                prefix="_rc_set_tmp",
                val_type=ValType.I32,
            )
        tmp = self._rc_set_scratch
        # Sequence: tee tmp; call retain; local.get idx; release; local.get tmp; (set | tee).
        self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(tmp))
        self._emit_call(retain_idx)
        self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(idx))
        self._emit_call(release_idx)
        self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(tmp))
        if keep_on_stack:
            self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))
        else:
            self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))

    def _emit_local_set_owned(self, idx: int, source: "Ownership") -> None:
        """Owned-slot write that consumes the value off the stack.

        Convenience wrapper over :meth:`_emit_owned_local_write` with
        ``keep_on_stack=False``.  Called from S2.3's per-site
        migrations.
        """
        self._emit_owned_local_write(idx, source, keep_on_stack=False)

    def _emit_local_tee_owned(self, idx: int, source: "Ownership") -> None:
        """Owned-slot write that leaves the value on the stack.

        Convenience wrapper over :meth:`_emit_owned_local_write` with
        ``keep_on_stack=True``.
        """
        self._emit_owned_local_write(idx, source, keep_on_stack=True)

    def _emit_br(self, depth: int) -> None:
        self._emit(WasmOp.BR, _leb128_unsigned(depth))

    def _emit_br_if(self, depth: int) -> None:
        self._emit(WasmOp.BR_IF, _leb128_unsigned(depth))

    # -- Expression emission --

    def set_compiled_proc_registrations(self, ir_module: IRModule) -> None:
        """Queue a ``tcl_proc_register_compiled`` call per compiled proc.

        Emitted at the top of ``generate()`` (before normal ::top
        code runs) so the runtime's proc table knows every compiled
        proc by name, marked with ``func_idx=1`` so
        ``eval_proc_call`` takes the host-bridge path instead of
        looking for a Tcl source body.  The bridge
        (``call_compiled_proc`` in the host embedder) then
        dispatches by proc name to the compiled WASM function.

        ``func_idx`` is used as a marker here, not an actual index
        — the host looks procs up by *name*.  Any non-zero value
        works; we pick 1 for clarity.

        ``has_args_tail`` — 1 when the last declared parameter is
        named ``args`` (Tcl's variadic marker).  The runtime's
        host-bridge dispatcher reads this flag to pack excess
        call arguments into a list TclObj so the compiled
        function's fixed-arity signature always sees the right
        number of parameters.  Without this bit, a
        ``proc foo {args} {}`` compiled to a single-i32-param
        function traps with ``too many parameters provided: given
        N, expected 1`` on any N ≥ 2 invocation (e.g. tcltest's
        ``useLocal counter.tcl counter`` bundle-runner stub).
        """
        self._compiled_proc_queue = []
        reg_idx = self._shared_imports.get("tcl_proc_register_compiled")
        if reg_idx is None:
            return
        for qname, proc in ir_module.procedures.items():
            if proc.body_source is None:
                continue
            has_args_tail = bool(proc.params and proc.params[-1] == "args")
            n_required = self._compute_n_required(qname, proc.params)
            self._compiled_proc_queue.append(
                (qname, len(proc.params), has_args_tail, n_required, proc.body_source)
            )

    def _compute_n_required(self, qname: str, params: tuple[str, ...]) -> int:
        """Count required (no-default) positional params, excluding ``args`` tail."""
        if not params:
            return 0
        has_tail = bool(params and params[-1] == "args")
        fixed_count = len(params) - 1 if has_tail else len(params)
        defaults = self._proc_defaults.get(qname, ())
        count = 0
        for i in range(fixed_count):
            # None → required; any other value → has a default → optional.
            default_val = defaults[i] if i < len(defaults) else None
            if default_val is None:
                count += 1
        return count

    def _record_stmt_context(self, stmt: IRStatement) -> None:
        """Update the 'current statement' tracking the diag machinery reads.

        Called at the top of ``_emit_stmt`` and — importantly — from
        ``_emit_block``'s implicit-return tail path which invokes
        ``_emit_call_stmt_tail`` directly without going through
        ``_emit_stmt``.  Every IR statement carries a ``range`` (see
        core/compiler/ir.py); IRCall additionally carries ``command``
        and ``args``.
        """
        rng = getattr(stmt, "range", None)
        if rng is not None:
            self._current_range = rng
        cmd = getattr(stmt, "command", None)
        self._current_command = cmd if isinstance(cmd, str) else ""
        raw_args = getattr(stmt, "args", ())
        self._current_args = tuple(raw_args) if isinstance(raw_args, tuple | list) else ()

    def generate(self) -> WasmFunction:
        """Generate a complete WASM function.

        All functions take i32 TclObj params and return a single i32
        TclObj result (null pointer 0 for void/empty-string returns).
        """
        # Emit the compiled-proc registration prologue so the
        # runtime's proc table records every compiled proc by name.
        # The func_idx=1 marker tells ``eval_proc_call`` to dispatch
        # through the host bridge (``call_compiled_proc``) rather
        # than try to eval a Tcl source body.
        queue = getattr(self, "_compiled_proc_queue", None)
        if queue:
            reg_idx = self._shared_imports.get("tcl_proc_register_compiled")
            body_idx = self._shared_imports.get("tcl_proc_set_body_source")
            if reg_idx is not None:
                for qname, n_params, has_args_tail, n_required, body_source in queue:
                    self._emit_obj_literal(qname)
                    self._emit_i32_const(n_params)
                    self._emit_i32_const(1)  # func_idx marker (any non-zero)
                    self._emit_i32_const(1 if has_args_tail else 0)
                    self._emit_i32_const(n_required)
                    self._emit_call(reg_idx)
                    self._emit(WasmOp.DROP)
                    # Stash the source-text body so ``info body`` can
                    # return the original ``proc`` body rather than the
                    # empty string the AOT path otherwise leaves
                    # behind.  Skipped when the import wasn't pulled
                    # in (older bundles) or the proc lacks a
                    # body_source (synthetic ``when`` handlers).
                    if body_idx is not None and body_source is not None:
                        self._emit_obj_literal(qname)
                        self._emit_obj_literal(body_source)
                        self._emit_call(body_idx)
                        self._emit(WasmOp.DROP)

        # Compiled-proc entry prologue: push a frame and bind each
        # param into the frame's local-variable table.  This lets
        # eval-fallbacks *inside* the proc body (e.g. ``proc
        # $varName {body}`` where the interpreter needs to resolve
        # ``$varName`` against the compiled proc's locals) see the
        # param values via ``var_resolve``.  ``::top`` isn't a
        # proc — it runs in global scope and doesn't get a frame.
        wants_frame = (
            self._is_proc
            and "tcl_frame_push" in self._shared_imports
            and "tcl_local_set" in self._shared_imports
            and "tcl_frame_pop" in self._shared_imports
        )
        # Escape-aware frame elision: a proc the analysis proved has
        # no FRAME vars and no interpreter fallback path doesn't need
        # a runtime frame at all.  Skip frame_push / param mirrors /
        # frame_pop entirely in that case.  Falls back to "push the
        # frame" whenever the analysis is missing or indefinite.
        #
        # Elision is also disabled when the proc's body references
        # ``info level`` — the runtime's ``info_level`` reads the
        # frame stack, so without a push the caller's frame would
        # answer the callee's query.  A more aggressive ("does any
        # proc reachable from here use info level?") transitive
        # check would let more procs elide, but computing it
        # inside the emitter requires a whole-module pass; the
        # per-proc check is a reasonable trade-off until a
        # profiler justifies more work.
        summary = self._escape_summary
        if (
            wants_frame
            and self._frame_elision
            and summary is not None
            and not summary.frame_needed
            and not summary.has_fallback
            and not self._body_references_info_level()
        ):
            wants_frame = False
        # Publish the elision decision so the body emitters
        # (``_emit_local_set_owned``, S2.3 onwards) know whether to
        # wrap owned-slot writes with retain/release.  When a frame
        # is present ``frame_pop`` already balances the slot's
        # refcount; the wrap stays inactive there.
        self._wants_frame = wants_frame
        if wants_frame:
            push_idx = self._shared_imports["tcl_frame_push"]
            self._emit_call(push_idx)
            self._emit(WasmOp.DROP)  # discard frame idx

        # Stamp the proc's namespace onto ``ns_current`` for the
        # duration of the body.  Without this, an unqualified
        # call from inside a ``::ns::foo`` proc (e.g. tcltest's
        # ``::tcltest::cleanupTests`` calling ``preserveCore``
        # — an accessor proc created by ``Option``) routes
        # through ``tcl_eval`` with ``ns_current() == ::`` and
        # fails to find ``::ns::preserveCore``.  This mirrors
        # the IRBlock wrapper that Stage 1 put around inlined
        # ``namespace eval`` bodies, extended to cover procs
        # whose qualified name already implies a non-root ns.
        #
        # Only fires when the proc lives outside root — root
        # procs keep ``ns_current == ::`` and need no push.
        ns_saved_proc_local: int | None = None
        if (
            self._is_proc
            and self._proc_namespace
            and self._proc_namespace != "::"
            and "tcl_ns_set" in self._shared_imports
            and "tcl_ns_restore" in self._shared_imports
        ):
            ns_set_idx = self._shared_imports["tcl_ns_set"]
            ns_saved_proc_local = self._add_extra_local(
                prefix="_proc_ns_saved", val_type=ValType.I64
            )
            ns_literal = self._proc_namespace
            offset = self._intern_string(ns_literal)
            encoded = ns_literal.encode("utf-8", errors="surrogatepass")
            self._emit_i32_const(offset + 4)
            self._emit_i32_const(len(encoded))
            self._emit_call(ns_set_idx)
            self._emit_local_set(ns_saved_proc_local)

        # Stash the invocation argv for ``info level 0`` / ``info
        # level -N``.  Build a list TclObj of the form
        # ``<proc-tail> <param-1> <param-2> ...`` — but only
        # include params that were actually *supplied* (non-null)
        # by the caller.  Defaulted-or-missing slots are skipped
        # so ``[llength [info level 0]]`` accurately reports the
        # call's argument count — the key invariant tcltest's
        # accessor wrappers depend on
        # (``[llength [info level 0]] == 1`` ⇒ "called without
        # args").
        #
        # This runs *before* the default-substitution block so we
        # can still distinguish null slots (missing args) from
        # slots that contain the declared default value.  The
        # argv list lives in a reserved WASM local through the
        # rest of the prologue; ``frame_set_argv`` is called once
        # the default-sub + param-bind loop is done.
        argv_local: int | None = None
        if (
            wants_frame
            and self._is_proc
            and self._proc_qname is not None
            and "tcl_list_create" in self._shared_imports
            and "tcl_frame_set_argv" in self._shared_imports
        ):
            list_idx = self._shared_imports["tcl_list_create"]
            tail = self._proc_qname.rsplit("::", 1)[-1] or self._proc_qname
            argv_local = self._add_extra_local(prefix="_argv", val_type=ValType.I32)
            # argv0 comes from the caller's pending-argv0 slot —
            # the *exact word* the caller wrote for the invocation
            # (imported name, qualified form, or rename target).  If
            # no caller recorded one (e.g. entry reached via the
            # host bridge or as ``::top``), ``take_pending_argv0``
            # returns 0 and we fall back to ``obj_new_string(tail)``
            # so the argv list still carries a sensible proc name.
            #
            # Emit structure:
            #   take_pending_argv0 -> argv0_local
            #   if argv0_local == 0 { argv0_local = obj_new_string(tail) }
            #   argv_local = tcl_list("", argv0_local)
            take_argv0_idx = self._shared_imports.get("tcl_frame_take_pending_argv0")
            argv0_local: int | None = None
            if take_argv0_idx is not None:
                argv0_local = self._add_extra_local(prefix="_argv0", val_type=ValType.I32)
                self._emit_call(take_argv0_idx)
                self._emit_local_set(argv0_local)
                # if argv0_local == 0: argv0_local = obj_new_string(tail)
                self._emit_local_get(argv0_local)
                self._emit(WasmOp.I32_EQZ)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]), label="argv0_fallback")
                self._emit_obj_literal(tail)
                self._emit_local_set(argv0_local)
                self._emit(WasmOp.END)
                # Seed argv_local with tcl_list("", argv0_local).
                self._emit_obj_literal("")
                self._emit_local_get(argv0_local)
                self._emit_call(list_idx)
                self._emit_local_set(argv_local)
            else:
                # Legacy fallback: no pending-argv0 import available.
                # Seed with the qname tail as before.
                self._emit_obj_literal("")
                self._emit_obj_literal(tail)
                self._emit_call(list_idx)
                self._emit_local_set(argv_local)
            # Determine which param (if any) is the variadic ``args``
            # tail — last declared param, and only when the proc's
            # registered signature recorded it as such.  Variadic
            # dispatch packs surplus caller words into a list before
            # the call, so appending the tail slot as a single
            # element would collapse ``p 1 2 3`` (args = ``{2 3}``)
            # into ``{p 1 {2 3}}`` and break ``[llength [info level
            # 0]]``.  Expand the list contents instead so the argv
            # matches the caller's actual word count.
            has_args_tail = (
                self._proc_qname in self._proc_args_tail
                and bool(self._params)
                and self._params[-1] == "args"
            )
            args_tail_slot = len(self._params) - 1 if has_args_tail else -1
            llength_idx = self._shared_imports.get("tcl_list_length") if has_args_tail else None
            lindex_idx = self._shared_imports.get("tcl_list_index") if has_args_tail else None
            expand_tail = llength_idx is not None and lindex_idx is not None
            # For each real param: if the WASM local is non-null
            # (caller supplied a value), append it; otherwise
            # leave the list alone.  The ``args`` tail slot gets
            # its list contents expanded element-by-element.
            for i, pname in enumerate(self._params):
                if not pname:
                    continue
                if i == args_tail_slot and expand_tail:
                    assert llength_idx is not None
                    assert lindex_idx is not None
                    # Loop: for n = llength(args_local); i < n; i++
                    #   argv_local = tcl_list(argv_local,
                    #                         tcl_list_index(args_local, i))
                    counter_local = self._add_extra_local(prefix="_argv_i", val_type=ValType.I64)
                    limit_local = self._add_extra_local(prefix="_argv_n", val_type=ValType.I64)
                    # limit = unbox(tcl_list_length(args_local))
                    self._emit_local_get(i)
                    self._emit_call(llength_idx)
                    self._emit_unbox_int()
                    self._emit_local_set(limit_local)
                    # counter = 0
                    self._emit_i64_const(0)
                    self._emit_local_set(counter_local)
                    self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label="args_expand_end")
                    self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]), label="args_expand_body")
                    # if counter >= limit, break
                    self._emit_local_get(counter_local)
                    self._emit_local_get(limit_local)
                    self._emit(WasmOp.I64_GE_S)
                    self._emit_br_if(1)
                    # argv_local = tcl_list(argv_local,
                    #                       tcl_list_index(args, box(counter)))
                    self._emit_local_get(argv_local)
                    self._emit_local_get(i)
                    self._emit_local_get(counter_local)
                    self._emit_box_int()
                    self._emit_call(lindex_idx)
                    self._emit_call(list_idx)
                    self._emit_local_set(argv_local)
                    # counter++
                    self._emit_local_get(counter_local)
                    self._emit_i64_const(1)
                    self._emit(WasmOp.I64_ADD)
                    self._emit_local_set(counter_local)
                    # br to loop header
                    self._emit_br(0)
                    self._emit(WasmOp.END)  # loop
                    self._emit(WasmOp.END)  # block
                    continue
                self._emit_local_get(i)
                self._emit(WasmOp.I32_EQZ)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]), label="argv_append_skip")
                self._emit(WasmOp.ELSE)
                self._emit_local_get(argv_local)
                self._emit_local_get(i)
                self._emit_call(list_idx)
                self._emit_local_set(argv_local)
                self._emit(WasmOp.END)

        # Default-value substitution for null (0) param slots.  Applies
        # whether or not we push a runtime frame — body code reads
        # params via ``local.get i`` directly, so the WASM local must
        # already hold the default if the caller omitted the slot.
        #
        # Needed because the host-bridge dispatch path
        # (``env.call_compiled_proc`` in ``tcl_dispatch.zig``) pads
        # missing argv slots with literal 0 (null TclObj): the bridge
        # doesn't know the callee's declared defaults.  The
        # compile-time specialised call-site path (``_emit_cmd_proc_call``)
        # pads with a real TclObj (boxed int 0 or the declared default),
        # so this substitution is a no-op for that path — the
        # ``i32.eqz`` check fails on a valid TclObj pointer.
        #
        # Without this, ``proc Opt {v {verify ident}} { ... }`` called
        # via the eval fallback (``Opt hello``) ends up with
        # ``verify`` bound to null — ``$verify`` resolves to the empty
        # string and dynamic dispatch patterns like
        # ``catch {$verify $v}`` silently produce an empty result.
        # tcltest's ``Option`` proc relies on this shape.
        # S2.4: claim ownership of caller-passed param values BEFORE
        # default-substitution so the wrapped default-sub store
        # below can release the prior slot value safely.  Without
        # this, the wrap would release the caller's handle on the
        # first overwrite — sending its rc to 0 → freed → use-
        # after-free in the caller's slot.
        #
        # ``tcl_obj_retain`` is null-safe so absent params (slot
        # still 0 because no caller arg and no default yet) are a
        # silent no-op here.
        if self._is_proc and not wants_frame:
            retain_idx = self._shared_imports.get("tcl_obj_retain")
            if retain_idx is not None:
                # S5.1: param slots enter the body pre-populated by
                # the WASM ABI and already-retained by the prologue
                # below, so subsequent writes to a param slot have
                # a non-null prior value that must be released.
                # Seed the first-write tracker with each param idx
                # so the first-write fast path doesn't fire on them.
                for i, pname in enumerate(self._params):
                    if pname:
                        self._first_writes_seen.add(i)
                for i, pname in enumerate(self._params):
                    if not pname:
                        continue
                    self._emit_local_get(i)
                    self._emit_call(retain_idx)

        own_defaults = self._proc_defaults.get(self._proc_qname, ()) if self._proc_qname else ()
        if self._is_proc and own_defaults:
            for i, pname in enumerate(self._params):
                if not pname:
                    continue
                if i >= len(own_defaults):
                    break
                default_val = own_defaults[i]
                if default_val is None:
                    continue
                # if local.get(i) == 0 { local.set(i, obj_literal(default_val)) }
                # ``_emit_obj_literal`` allocates fresh → OWNED.  In
                # frame-elided procs the wrap releases the prior slot
                # value (which is 0 because of the I32_EQZ guard, so
                # null-safe) and transfers the literal's +1 cleanly.
                self._emit_local_get(i)
                self._emit(WasmOp.I32_EQZ)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]), label="default_sub")
                self._emit_obj_literal(default_val)
                self._emit_owned_local_write(i, Ownership.OWNED, keep_on_stack=False)
                self._emit(WasmOp.END)

        if wants_frame:
            local_set_idx = self._shared_imports["tcl_local_set"]
            for i, pname in enumerate(self._params):
                if not pname:
                    continue
                self._emit_obj_literal(pname)
                self._emit_local_get(i)
                self._emit_call(local_set_idx)
                self._emit(WasmOp.DROP)

        # Publish the argv list built before default-substitution.
        # The list is the real "only supplied args" invocation so
        # ``[llength [info level 0]]`` reports the caller's true
        # word count.
        if argv_local is not None and "tcl_frame_set_argv" in self._shared_imports:
            set_argv_idx = self._shared_imports["tcl_frame_set_argv"]
            self._emit_local_get(argv_local)
            self._emit_call(set_argv_idx)

        # Record where the body begins — we need to walk only the
        # body instructions in the frame_pop post-process, not the
        # prologue we just emitted.
        prologue_end = len(self._body)

        self._emit_block(self._cfg.entry)

        # Ensure function has a return value on all paths (i32 null TclObj)
        if not self._body or self._body[-1].op != WasmOp.RETURN:
            self._emit_i32_const(0)
            self._emit(WasmOp.END)
        else:
            self._emit(WasmOp.END)

        # Compiled-proc exit epilogue: pop the frame we pushed in the
        # prologue, and restore the namespace context if we stamped
        # one.  Inject the CALLs immediately before every
        # ``WasmOp.RETURN`` and at the natural implicit fall-through
        # (between the last body instruction and the final END).
        # Confining the walk to instructions *after* ``prologue_end``
        # means we don't accidentally treat the prologue's own
        # call/drop as a RETURN site.
        #
        # The ns-restore needs the saved i64 token from the
        # prologue's ``ns_set``; we emit it after any frame_pop and
        # before the END so the stack is ordered
        # ``[return_val] -> (frame_pop drop) -> (local.get saved ->
        # ns_restore) -> END``.
        cleanup_instrs: list[WasmInstruction] = []
        # S2.5: in frame-elided procs, every owned slot still holds
        # its +1.  Release each before the proc returns so the
        # discipline closes.  Wrap around the return value: tee it
        # into a save slot, retain it (so the upcoming releases
        # cannot queue it for free), do the releases, then restore
        # the saved value to the stack so RETURN / END consumes it.
        # ``tcl_obj_retain`` and ``tcl_obj_release`` are null-safe
        # so a return of ``0`` and any slot still at 0 are no-ops.
        if (
            self._is_proc
            and not wants_frame
            and self._owned_locals
            and "tcl_obj_retain" in self._shared_imports
            and "tcl_obj_release" in self._shared_imports
        ):
            retain_idx = self._shared_imports["tcl_obj_retain"]
            release_idx = self._shared_imports["tcl_obj_release"]
            save_local = self._add_extra_local(
                prefix="_rc_ret_save",
                val_type=ValType.I32,
            )
            cleanup_instrs.append(
                WasmInstruction(op=WasmOp.LOCAL_TEE, operands=_leb128_unsigned(save_local))
            )
            cleanup_instrs.append(
                WasmInstruction(op=WasmOp.CALL, operands=_leb128_unsigned(retain_idx))
            )
            for slot in self._owned_locals:
                cleanup_instrs.append(
                    WasmInstruction(op=WasmOp.LOCAL_GET, operands=_leb128_unsigned(slot))
                )
                cleanup_instrs.append(
                    WasmInstruction(op=WasmOp.CALL, operands=_leb128_unsigned(release_idx))
                )
            cleanup_instrs.append(
                WasmInstruction(op=WasmOp.LOCAL_GET, operands=_leb128_unsigned(save_local))
            )
        if wants_frame:
            pop_idx = self._shared_imports.get("tcl_frame_pop")
            if pop_idx is not None:
                cleanup_instrs.append(
                    WasmInstruction(op=WasmOp.CALL, operands=_leb128_unsigned(pop_idx))
                )
        if ns_saved_proc_local is not None and "tcl_ns_restore" in self._shared_imports:
            ns_restore_idx = self._shared_imports["tcl_ns_restore"]
            cleanup_instrs.extend(
                [
                    WasmInstruction(
                        op=WasmOp.LOCAL_GET,
                        operands=_leb128_unsigned(ns_saved_proc_local),
                    ),
                    WasmInstruction(op=WasmOp.CALL, operands=_leb128_unsigned(ns_restore_idx)),
                ]
            )
        if cleanup_instrs:
            new_body = list(self._body[:prologue_end])
            body_tail = list(self._body[prologue_end:])
            for instr in body_tail:
                if instr.op == WasmOp.RETURN:
                    new_body.extend(cleanup_instrs)
                new_body.append(instr)
            # Insert cleanup between the last non-END instruction and
            # the trailing END(s) for the natural fallthrough.
            i = len(new_body) - 1
            while i >= 0 and new_body[i].op == WasmOp.END:
                i -= 1
            if i >= 0 and new_body[i].op != WasmOp.RETURN:
                new_body = new_body[: i + 1] + cleanup_instrs + new_body[i + 1 :]
            self._body = new_body

        # Apply optimisations
        self._run_optimisations()

        # Separate param types from extra locals
        param_types = [ValType.I32] * len(self._params)
        extra_locals = self._local_types[len(self._params) :]

        return WasmFunction(
            name=self._cfg.name,
            params=param_types,
            results=[ValType.I32],
            locals=extra_locals,
            body=self._body,
            local_names=[f"${n}" for n in self._local_names],
            exported=True,
            kind="proc" if self._is_proc else "top",
        )

    @property
    def data_segments(self) -> list[WasmData]:
        """Return accumulated string data segments."""
        segments: list[WasmData] = []
        for value, offset in self._strings:
            encoded = value.encode("utf-8", errors="surrogatepass")
            # Store: 4-byte length prefix + utf-8 data
            data = len(encoded).to_bytes(4, "little") + encoded
            segments.append(WasmData(offset=offset, data=data))
        return segments
