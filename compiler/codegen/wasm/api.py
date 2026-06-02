"""Public entry points for the WASM code generator.

``wasm_codegen_function`` and ``wasm_codegen_module`` are the two
functions external callers use; the package ``__init__`` re-exports
them.  Everything they lean on (the emitter, the IR/encoding/scan
helpers) lives in the underscore-prefixed sibling modules.
"""

from __future__ import annotations

from compiler.var_escape import ProcEscapeSummary, analyse_var_escape
from shared.diagnostic import Range

from ...cfg import CFGFunction, CFGModule
from ...command_trust import command_trust, module_command_trust
from ...ir import IRModule
from ._emitter import _WasmEmitter
from ._imports import import_signature
from ._ir import DiagMap, WasmData, WasmFunction, WasmImport, WasmModule
from ._scan import _scan_needed_imports
from .proc_scan import _collect_dynamically_modified_procs, _proc_name_variants


def wasm_codegen_function(
    cfg: CFGFunction,
    params: tuple[str, ...] = (),
    *,
    optimise: bool = False,
    is_proc: bool = False,
    shared_imports: dict[str, int] | None = None,
    proc_index: dict[str, tuple[int, int]] | None = None,
) -> WasmFunction:
    """Generate a WASM function from a CFG function.

    When *optimise* is ``True``, constant folding, peephole
    optimisation, and dead-code elimination are applied.
    """
    emitter = _WasmEmitter(
        cfg,
        params=params,
        optimise=optimise,
        is_proc=is_proc,
        shared_imports=shared_imports,
        proc_index=proc_index,
    )
    return emitter.generate()


def wasm_codegen_module(
    cfg_module: CFGModule,
    ir_module: IRModule,
    *,
    optimise: bool = False,
    filename: str | None = None,
    diag_map: DiagMap | None = None,
    escape_summaries: dict[str, ProcEscapeSummary] | None = None,
    frame_elision: bool = True,
    inline: bool = True,
    licm: bool = True,
    dce: bool = True,
    gvn: bool = True,
) -> WasmModule:
    """Generate a complete WASM module from a CFG module.

    Args:
        cfg_module: The control-flow-graph module to compile.
        ir_module: The IR module (for procedure metadata).
        optimise: Toggles the *codegen-side* optimisations only —
            constant folding, peephole, SSA-emit-time dead-code
            elimination.  This flag does **not** affect the IR-
            level passes (S4 inlining, S5 LICM / GVN / DCE);
            those are sound semantics-preserving transforms and
            run by default for every callsite that doesn't pass
            ``inline=False`` / ``licm=False`` / ``dce=False`` /
            ``gvn=False`` explicitly.  Pass ``optimise=False``
            (the default) when emitting straightforward unoptimised
            output for diff-friendly inspection; pair with
            ``inline=False, licm=False, dce=False, gvn=False`` to
            additionally suppress the IR passes.
        filename: Source filename recorded on every diag site.
            Passing this without a ``diag_map`` allocates a fresh
            map; passing neither disables diag instrumentation
            entirely (no ``tcl_diag_set`` calls, no ``site=<id>``
            prefix on traps).
        diag_map: Optional pre-allocated :class:`DiagMap` to populate.
            Pass one when the caller wants to serialise the sidecar
            alongside the WASM bytes.  Leave as ``None`` (and omit
            *filename*) when diagnostics aren't needed — callers
            that can't resolve ``site=<id>`` prefixes shouldn't see
            them at all.

    Returns:
        A ``WasmModule`` that can be serialised to ``.wasm`` binary
        or inspected as WAT text.  Diag data (when enabled) is
        recorded on the supplied *diag_map* (or on the fresh one
        allocated when a *filename* was passed).
    """
    with command_trust(module_command_trust(ir_module)):
        # Only instrument with diag sites when the caller explicitly
        # asks for them — either by passing a ``DiagMap`` to populate,
        # or by providing a ``filename`` (which is useless without a
        # map to record it in, so we interpret it as an implicit
        # request).  Without a map the emitter skips ``tcl_diag_set``
        # calls entirely — avoids emitting ``site=<id>`` prefixes the
        # caller has no way to resolve.
        if diag_map is None and filename is not None:
            diag_map = DiagMap(filename=filename)
        elif diag_map is not None and filename is not None:
            diag_map.filename = filename

        # Phase 0: Run per-proc var-escape analysis so the emitter can narrow
        # frame-sync work to vars the analysis couldn't prove LOCAL-safe.
        # Runs unconditionally — the analysis is a cheap tree walk, and callers
        # that pre-computed it can pass ``escape_summaries`` to skip the work.
        if escape_summaries is None:
            try:
                escape_summaries = analyse_var_escape(ir_module=ir_module)
            except Exception:  # noqa: BLE001 — analysis failure falls back
                escape_summaries = None

        # Phase 0.5: S4.2 — inline eligible calls before any other analysis
        # consumes the IR.  ``inline_module`` (v0) only drops statement-
        # position calls to ALWAYS-tagged empty-body procs, so the
        # rewrite is purely subtractive: it removes ``IRCall`` statements
        # without introducing new shapes.  We rebuild the CFG from the
        # inlined IR so downstream passes (SSA, dataflow) see the
        # same module the codegen will emit.  Re-running var-escape is
        # cheap and ensures the per-proc summaries reflect the post-
        # inline call graph (procs whose only callees were inlined-away
        # may newly qualify as pure_leaf).
        #
        # **Failure policy.**  S4/S5 passes are *not* opportunistic any
        # more — they're load-bearing once the call sites enable them
        # by default.  Silently swallowing exceptions here masked real
        # bugs (PR #237 review).  The pass functions are pure /
        # structural; any exception they raise is a bug we want to fix,
        # not paper over.  Same policy applies to LICM, DCE, and GVN
        # below.
        if inline and escape_summaries is not None:
            from ...cfg import build_cfg as _build_cfg
            from ...inlining import apply_inline_catalogue
            from ...inlining import inline_module as _inline

            tagged = apply_inline_catalogue(ir_module, escape_summaries)
            inlined = _inline(tagged, escape_summaries)
            if inlined is not tagged:
                ir_module = inlined
                cfg_module = _build_cfg(ir_module)
                escape_summaries = analyse_var_escape(ir_module=ir_module)
            else:
                # Catalogue still tagged the procs even if no splices
                # fired this pass — keep the tagged module so any
                # later inliner generation sees the metadata.
                ir_module = tagged

        # Phase 0.6: S5.3 — hoist loop-invariant literal assignments out
        # of loop bodies.  The pass operates on the (possibly inlined)
        # IR and is purely subtractive at the body level (it moves
        # statements OUT of loops; the parent script grows by the same
        # count).  We rebuild the CFG and re-run var-escape after
        # changes so the downstream emitter sees the hoisted shape.
        if licm:
            from ...cfg import build_cfg as _build_cfg2
            from ...passes.licm import licm_module as _licm

            hoisted = _licm(ir_module)
            if hoisted is not ir_module:
                ir_module = hoisted
                cfg_module = _build_cfg2(ir_module)
                escape_summaries = analyse_var_escape(ir_module=ir_module)

        # Phase 0.7: S5.4 — dead-store elimination.  Removes
        # ``IRAssignConst`` / ``IRAssignValue`` / ``IRAssignExpr`` /
        # ``IRIncr`` whose target is only written once and never read.
        # Only fires on ``ALWAYS``-tagged (pure_leaf) procs since
        # other procs may read vars dynamically through eval / upvar /
        # info.  Subtractive at the script level — rebuilds the CFG
        # and var-escape after changes.
        if dce:
            from ...cfg import build_cfg as _build_cfg3
            from ...passes.dce import dce_module as _dce

            # Pass the post-fixpoint summaries so DCE can use the
            # precise ``safe_to_dce`` predicate (PR #237 review)
            # rather than piggybacking on the inline catalogue tag.
            cleaned = _dce(ir_module, escape_summaries)
            if cleaned is not ir_module:
                ir_module = cleaned
                cfg_module = _build_cfg3(ir_module)
                escape_summaries = analyse_var_escape(ir_module=ir_module)

        # Phase 0.8: S5.4 GVN — replace redundant ``IRAssignExpr``
        # writes with copies from prior equivalent results when the
        # source variables haven't been modified between them.
        # ``set y [expr {$x + 1}]; set z [expr {$x + 1}]`` becomes
        # ``set y [expr {$x + 1}]; set z $y``.  Subtractive — never
        # adds work, just elides recomputation.
        if gvn:
            from ...cfg import build_cfg as _build_cfg4
            from ...passes.gvn import gvn_module as _gvn

            valued = _gvn(ir_module)
            if valued is not ir_module:
                ir_module = valued
                cfg_module = _build_cfg4(ir_module)
                escape_summaries = analyse_var_escape(ir_module=ir_module)

        module = WasmModule()

        # Phase 1: Pre-scan IR to find which runtime imports are needed
        needed_imports = _scan_needed_imports(cfg_module, ir_module)

        # Phase 2: Register needed imports (these occupy the first function indices).
        # Uses :func:`import_signature` which reads from the spec-side
        # ``WasmRuntimeImport`` first and falls back to the infrastructure
        # dict for non-command-owned imports (obj lifecycle, arith, frame
        # ops, math funcs).  Unknown keys are skipped — scan should only
        # collect known imports, but this defensive check keeps a stray
        # entry from crashing codegen.
        shared_imports: dict[str, int] = {}
        for import_key in sorted(needed_imports):
            sig = import_signature(import_key)
            if sig is None:
                continue
            mod_name, func_name, params, results = sig
            type_idx = module._intern_type(params, results)
            func_idx = len(module.imports)
            module.imports.append(WasmImport(module=mod_name, name=func_name, type_idx=type_idx))
            shared_imports[import_key] = func_idx

        num_imports = len(module.imports)

        # Phase 3: Build a single ordered list of callables (procs + methods)
        # and the proc name → (func_idx, n_params) map.  This avoids
        # double-indexing methods that appear in cfg_module.procedures but
        # are defined in ir_module.methods rather than ir_module.procedures.
        proc_index: dict[str, tuple[int, int]] = {}
        # Parallel map of proc qname → tuple of default-value strings
        # (or ``None`` per slot without a default).  Used by the codegen
        # to pad missing call-site args with the declared default instead
        # of a boxed zero.
        from ...lowering import _parse_params_with_defaults

        proc_defaults: dict[str, tuple[str | None, ...]] = {}
        callables: list[tuple[str, CFGFunction, tuple[str, ...]]] = []

        for qname, cfg_func in cfg_module.procedures.items():
            ir_proc = ir_module.procedures.get(qname)
            if ir_proc is not None:
                callables.append((qname, cfg_func, ir_proc.params))
                if ir_proc.params_raw:
                    defs = _parse_params_with_defaults(ir_proc.params_raw)
                    proc_defaults[qname] = tuple(d for _, d in defs)
            elif ir_module.methods and qname in ir_module.methods:
                ir_method = ir_module.methods[qname]
                callables.append((qname, cfg_func, ir_method.params))
            else:
                callables.append((qname, cfg_func, ()))

        # Assign function indices: ::top is at num_imports, callables follow
        func_idx = num_imports + 1  # +1 for ::top
        # Also track which procs have a variadic ``args`` tail parameter.
        proc_args_tail: set[str] = set()
        for qname, _, params in callables:
            proc_index[qname] = (func_idx, len(params))
            if params and params[-1] == "args":
                proc_args_tail.add(qname)
            func_idx += 1

        # Invalidate proc_index entries for any proc that's the target of
        # a ``rename`` / ``interp hide`` / ``interp expose`` anywhere in
        # the module.  The compile-time direct-call specialisation
        # (``_resolve_proc`` → ``call $::foo``) bypasses the runtime's
        # hidden-commands table and rename-moved cmd_table, so it's
        # unsound for procs whose runtime dispatch state may change.
        # Removing them from ``proc_index`` downgrades calls to the
        # eval fallback (``tcl_eval`` → ``proc_lookup``), which observes
        # the runtime state correctly.  See
        # ``docs/design/runtime/command-introspection.md`` §8.1 for the
        # background.
        #
        # Names are paired with their enclosing namespace context so
        # ``namespace eval ::ns { rename foo bar }`` correctly
        # invalidates ``::ns::foo`` / ``::ns::bar`` rather than only
        # ``::foo`` / ``::bar``.
        # The callable map is a mutable copy of ``proc_index``.  The
        # emitter's ``_resolve_proc`` reads from it when deciding whether
        # to emit a direct ``call $<idx>`` or fall back to ``tcl_eval``,
        # so invalidation (``rename`` / ``interp hide`` / the full flush
        # for ``interp create``/``eval``/``delete``) only removes
        # entries from the callable map.  The original ``proc_index`` is
        # preserved because it's consumed a second time below to map
        # ``<wasm function N>`` → proc name for the diagnostic sidecar —
        # that mapping must not lose entries, even when the
        # corresponding calls route through eval.
        callable_proc_index: dict[str, tuple[int, int]] = dict(proc_index)
        dynamically_modified, full_flush, uses_dispatch_traces = (
            _collect_dynamically_modified_procs(ir_module)
        )
        # The distrust rename-guard takes a cheap direct ``call`` for a
        # proc command-substitution after verifying (at run time) the name
        # still maps to the proc and tracing is quiescent.  Disable it
        # entirely for units that use command / execution traces or a child
        # interp: those exercise command-table mutation during dispatch in
        # ways the per-call guard can't fully reason about (trace.test's
        # command rename/delete traces), and they never carry the deep
        # ``expr``-bodied recursion the guard exists to rescue.
        distrust_proc_guard = not (uses_dispatch_traces or full_flush)
        if full_flush:
            # Child-interp mutation: any direct call emitted from this
            # module can see a modified proc registry after the fact.
            # Flush the callable map so every subsequent call routes
            # through ``tcl_eval`` — see
            # ``docs/design/runtime/child-interp.md`` §7.
            callable_proc_index.clear()
        else:
            for context_ns, name in dynamically_modified:
                for variant in _proc_name_variants(name, context_ns):
                    callable_proc_index.pop(variant, None)

        # Shared string table so data segments from different functions
        # don't collide at offset 0. All emitters share a single list,
        # index dict, and offset counter.
        shared_strings: list[tuple[str, int]] = []
        shared_string_index: dict[str, int] = {}
        shared_string_offset: list[int] = [0]

        # Resolve ``namespace import`` patterns against the now-complete
        # proc index so the emitter can dispatch unqualified calls
        # (``test name desc body``) directly to the compiled proc
        # (``::tcltest::test``) instead of falling back to ``tcl_eval``.
        # ``namespace_imports`` is ordered; later imports shadow earlier
        # ones (matches Tcl resolution).  Pattern matching supports only
        # the two real-world shapes: absolute glob (``::ns::*``) and
        # absolute single name (``::ns::proc``).  More elaborate globs
        # would need ``string match`` semantics — deferred until a test
        # in the wild trips it.
        #
        # ``namespace export`` filter: C Tcl's ``Tcl_Import`` only
        # redirects commands whose simple name matches the source
        # namespace's export patterns.  We gather those patterns by
        # source ns and reject import candidates whose simple name
        # doesn't match any — the importing caller then falls back to
        # runtime dispatch (which produces the correct "unknown
        # command" diagnostic when the name really wasn't exported).
        # An empty export list for a source ns means "no exports
        # visible", so every import from that ns skips the shortcut.
        from fnmatch import fnmatchcase

        exports_by_ns: dict[str, list[str]] = {}
        for src_ns, pat in ir_module.namespace_exports:
            exports_by_ns.setdefault(src_ns, []).append(pat)

        def _source_exports(simple_name: str, source_ns: str) -> bool:
            patterns = exports_by_ns.get(source_ns, [])
            if not patterns:
                return False
            for p in patterns:
                if fnmatchcase(simple_name, p):
                    return True
            return False

        proc_imports: dict[str, dict[str, str]] = {}
        for context_ns, pattern in ir_module.namespace_imports:
            table = proc_imports.setdefault(context_ns, {})
            ns_part, _, name_part = pattern.rpartition("::")
            if not ns_part:
                continue
            prefix = f"{ns_part}::"
            if name_part == "*":
                for qname in proc_index:
                    if qname.startswith(prefix):
                        short = qname[len(prefix) :]
                        if "::" not in short and _source_exports(short, ns_part):
                            table[short] = qname
            else:
                if pattern in proc_index and _source_exports(name_part, ns_part):
                    table[name_part] = pattern

        # Phase 4: Compile top-level with shared state
        top_escape = escape_summaries.get("::top") if escape_summaries is not None else None
        # Cover the whole file for the ``::top`` pseudo-proc so a click on
        # any top-level instruction resolves to somewhere inside the script.
        top_range: Range | None
        top_stmts = ir_module.top_level.statements
        if top_stmts:
            first = getattr(top_stmts[0], "range", None)
            last = getattr(top_stmts[-1], "range", None)
            top_range = Range(start=first.start, end=last.end) if first and last else None
        else:
            top_range = None
        emitter = _WasmEmitter(
            cfg_module.top_level,
            optimise=optimise,
            is_proc=False,
            shared_imports=shared_imports,
            proc_index=callable_proc_index,
            full_proc_index=proc_index,
            distrust_proc_guard=distrust_proc_guard,
            proc_defaults=proc_defaults,
            proc_args_tail=proc_args_tail,
            shared_strings=shared_strings,
            shared_string_index=shared_string_index,
            shared_string_offset=shared_string_offset,
            diag_map=diag_map,
            escape_summary=top_escape,
            proc_imports=proc_imports,
            frame_elision=frame_elision,
        )
        # Register every compiled proc in the runtime proc table with a
        # non-zero func_idx marker so the interpreter knows to dispatch
        # via the host bridge (``call_compiled_proc`` → wasmtime
        # lookup).  We don't ship the Tcl source body — the compiled
        # WASM function IS the body, and the host bridge invokes it by
        # name.  This lets a dynamically-registered proc (``proc
        # $varName {body}``) whose body references a compiled helper
        # like ``Configure`` actually execute instead of trapping with
        # "unknown command".
        emitter.set_compiled_proc_registrations(ir_module)
        top_func = emitter.generate()
        top_func.name = "::top"
        top_func.kind = "top"
        top_func.source_range = top_range
        module.functions.append(top_func)
        # ::top occupies the first function index after imports.
        if diag_map is not None:
            diag_map.procs.append((num_imports, "::top"))

        # Phase 5: Compile callables (procedures and methods)
        for qname, cfg_func, params in callables:
            proc_escape = escape_summaries.get(qname) if escape_summaries is not None else None
            # Phase 8 follow-up: pull the proc's source body so the
            # compiled-proc prologue can stamp ``frame_set_script`` for
            # ``info frame -script``.  Synthetic procs (``when`` handlers)
            # have ``body_source = None``; the prologue skips the stamp
            # in that case.
            ir_proc_for_body = ir_module.procedures.get(qname)
            proc_body_source = (
                ir_proc_for_body.body_source if ir_proc_for_body is not None else None
            )
            proc_first_line = (
                ir_proc_for_body.range.start.line if ir_proc_for_body is not None else 0
            )
            callable_emitter = _WasmEmitter(
                cfg_func,
                params=params,
                optimise=optimise,
                is_proc=True,
                shared_imports=shared_imports,
                proc_index=callable_proc_index,
                full_proc_index=proc_index,
                distrust_proc_guard=distrust_proc_guard,
                proc_defaults=proc_defaults,
                proc_args_tail=proc_args_tail,
                shared_strings=shared_strings,
                shared_string_index=shared_string_index,
                shared_string_offset=shared_string_offset,
                proc_qname=qname,
                proc_body_source=proc_body_source,
                proc_first_line=proc_first_line,
                diag_map=diag_map,
                escape_summary=proc_escape,
                proc_imports=proc_imports,
                frame_elision=frame_elision,
            )
            callable_func = callable_emitter.generate()
            callable_func.name = qname
            # Stamp the callable's original source range so clicking a
            # ``call N`` in the explorer jumps to the proc's definition.
            ir_proc = ir_module.procedures.get(qname)
            if ir_proc is not None:
                callable_func.source_range = ir_proc.range
                callable_func.kind = "proc"
            elif ir_module.methods and qname in ir_module.methods:
                ir_method = ir_module.methods[qname]
                callable_func.source_range = ir_method.range
                callable_func.kind = "method"
            module.functions.append(callable_func)
            # Record the proc's WASM function index so the sidecar can
            # resolve ``<wasm function N>`` backtraces to proc names.
            if diag_map is not None:
                f_idx, _ = proc_index[qname]
                diag_map.procs.append((f_idx, qname))

        # Emit a single set of data segments from the shared string table.
        # ``surrogatepass`` mirrors ``_intern_string`` — the runtime reads
        # strings as opaque byte sequences; preserving WTF-8 lets test
        # bundles with lone-surrogate test-result literals compile.
        for value, offset in shared_strings:
            encoded = value.encode("utf-8", errors="surrogatepass")
            data = len(encoded).to_bytes(4, "little") + encoded
            module.data_segments.append(WasmData(offset=offset, data=data))

        # Ensure types are registered
        for func in module.functions:
            module._intern_type(func.params, func.results)

        return module
