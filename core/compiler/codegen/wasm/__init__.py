"""WebAssembly (WASM) code generator for Tcl.

Translates CFG-based IR into valid WASM binary modules.  Supports two
output modes controlled by the ``optimise`` flag:

- **Non-optimised** (``optimise=False``): straightforward translation
  with stack-based variable access, no instruction folding.
- **Optimised** (``optimise=True``): constant folding, dead-code
  elimination, and local variable promotion.

Public API::

    wasm_codegen_module(cfg_module, ir_module, *, optimise=False) -> WasmModule
    wasm_codegen_function(cfg, params=(), *, optimise=False) -> WasmFunction

The resulting ``WasmModule`` can be serialised to a ``.wasm`` binary
via :meth:`WasmModule.to_bytes` or inspected via :meth:`WasmModule.to_wat`.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

from ....analysis.semantic_model import Range
from ....parsing.substitution import backslash_subst as _tcl_backslash_subst
from ....parsing.tokens import TokenType
from ...cfg import (
    CFGBranch,
    CFGFunction,
    CFGGoto,
    CFGModule,
    CFGReturn,
)
from ...expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from ...ir import (
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
    IRIfClause,
    IRIncr,
    IRModule,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ...var_escape import ProcEscapeSummary, analyse_var_escape
from ._emitter import (  # noqa: E402
    _BINOP_WASM,
    _UNARYOP_WASM,
    _WasmEmitter,
)

# Encoding helpers extracted to _encoding.py.  Re-exported here for
# back-compat with callers that import these symbols from the
# top-level ``wasm`` package (and for internal use by this module).
from ._encoding import (  # noqa: E402
    _FLAG_CONVERT_BRACE,
    _FLAG_CONVERT_ESCAPE,
    _FLAG_CONVERT_MASK,
    _FLAG_CONVERT_NONE,
    _FLAG_DONT_QUOTE_HASH,
    _FLAG_DONT_USE_BRACES,
    _encode_string,
    _encode_vector,
    _leb128_signed,
    _leb128_unsigned,
    _tcl_convert_element,
    _tcl_list_quote,
    _tcl_scan_element,
    _tcl_token_value,
)

# WASM Emitter
from ._imports import (  # noqa: E402
    _CLOCK_SUBCMD_IMPORT,
    _CMD_RUNTIME,
    _DICT_SUBCMD_IMPORT,
    _OBJ_LIFECYCLE_IMPORTS,
    _RUNTIME_IMPORTS,
    _SCOPE_NOP_COMMANDS,
    _STRING_IS_IMPORT,
    _STRING_SUBCMD_IMPORT,
    _UNSUPPORTED_COMMANDS,
)
from ._ir import (  # noqa: E402
    _BLOCK_I64,
    _BLOCK_VOID,
    _WASM_MAGIC,
    _WASM_VERSION,
    DiagMap,
    DiagSite,
    SectionId,
    ValType,
    WasmData,
    WasmFunction,
    WasmImport,
    WasmInstruction,
    WasmModule,
    WasmOp,
    _decode_leb128_signed,
    _decode_leb128_unsigned,
    _format_wat_instr,
    _valtype_name,
)

# Parsing helpers extracted to _parsing.py — re-exported here for
# back-compat and internal use.
from ._parsing import (  # noqa: E402
    _derive_proc_namespace,
    _has_embedded_subst_scan,
    _parse_array_ref,
)
from ._scan import (  # noqa: E402
    _scan_expr_body_imports,
    _scan_needed_imports,
    _scan_text_for_cmd_subst,
)

# Public API


def _proc_name_variants(name: str, context_ns: str = "::") -> tuple[str, ...]:
    """Candidate ``proc_index`` keys for a user-supplied name in a
    given namespace context.

    The user's spelling of a command name on a ``rename`` / ``interp
    hide`` line can be unqualified (``foo``) or fully qualified
    (``::foo``, ``::ns::foo``).  The compile-time ``proc_index`` keys
    are always fully qualified, but resolution of an unqualified
    token depends on the enclosing namespace: inside
    ``namespace eval ::ns { rename foo bar }`` the target is
    ``::ns::foo``, not ``::foo``.

    We produce:

    * ``name`` verbatim (for users who wrote the qualified form
      directly, and as a defensive pass-through).
    * ``::<name>`` when the name is unqualified (root-scope fallback).
    * ``<context_ns>::<name>`` when the name is unqualified and the
      context is not root (``namespace eval`` body, or the
      enclosing proc's home ns).

    Brace-wrapping (``{foo}``) survives as-is because the IR lowers
    brace-quoted words to their inner bytes.
    """
    name = name.strip()
    if not name:
        return ()
    if name.startswith("::"):
        return (name,)
    variants: list[str] = [name, f"::{name}"]
    if context_ns and context_ns != "::":
        # ``context_ns`` is expected to be FQN-shaped (``::ns`` /
        # ``::ns::sub``).  Normalise just in case a caller passed a
        # trailing ``::``.
        base = context_ns.rstrip(":")
        variants.append(f"{base}::{name}")
    return tuple(variants)


def _collect_dynamically_modified_procs(
    ir_module: IRModule,
) -> tuple[set[tuple[str, str]], bool]:
    """Return the set of ``(context_ns, name)`` pairs for procs that
    ``rename`` / ``interp hide`` / ``interp expose`` touches
    anywhere in ``ir_module``, plus a ``full_flush`` flag that's
    ``True`` when the module uses a command whose side-effects can't
    be tracked surgically (``interp create`` / ``interp eval`` /
    ``interp delete``).

    Used by :func:`wasm_codegen_module` to invalidate
    ``proc_index`` entries for those names, downgrading calls from
    a compile-time specialised ``call $::foo`` to the runtime
    ``tcl_eval`` / ``proc_lookup`` path so runtime hide / rename
    state is visible.  When ``full_flush`` is set, the caller drops
    *every* entry — the conservative shortcut documented in
    ``docs/design/runtime/child-interp.md`` §7 for the child-interp
    wave.

    The walker threads a ``context_ns`` through every script body:

    * ``IRBlock`` — from ``namespace eval`` — pushes its own
      ``namespace`` field.
    * Procedure / method bodies start in the proc's enclosing
      namespace (``::ns::foo`` runs with context ``::ns``).
    * All other compound nodes inherit the current context.

    The context is paired with every invalidation target so the
    caller can qualify unqualified names against the right
    namespace — unqualified ``rename foo bar`` inside
    ``namespace eval ::ns { … }`` must invalidate
    ``::ns::foo`` / ``::ns::bar``, not just ``::foo`` / ``::bar``.
    """
    affected: set[tuple[str, str]] = set()
    full_flush = [False]

    def _scan_statement(stmt: IRStatement, context_ns: str) -> None:
        match stmt:
            case IRCall(command=cmd, args=args):
                if cmd == "rename" and len(args) >= 1:
                    affected.add((context_ns, args[0]))
                    if len(args) >= 2:
                        affected.add((context_ns, args[1]))
                elif cmd == "interp" and args and args[0] in ("hide", "expose"):
                    # ``interp hide path cmd ?hiddenName?`` →
                    # args = ("hide", path, cmd, [hiddenName]).
                    # Both ``cmd`` and ``hiddenName`` / ``newName``
                    # name procs that may disappear or re-appear at
                    # runtime, so both are invalidated.
                    if len(args) >= 3:
                        affected.add((context_ns, args[2]))
                    if len(args) >= 4:
                        affected.add((context_ns, args[3]))
                elif (
                    cmd == "interp"
                    and args
                    and args[0]
                    in (
                        "create",
                        "eval",
                        "delete",
                    )
                ):
                    # Child-interp primitives: a child can define
                    # arbitrary procs whose names we can't enumerate
                    # statically, and ``interp delete`` can unlink
                    # procs we might be specialising.  Full flush is
                    # the conservative-but-correct shortcut — calls
                    # route through ``tcl_eval`` and observe the live
                    # registry.
                    full_flush[0] = True
            case IRIf(clauses=clauses, else_body=else_body):
                for clause in clauses:
                    _scan_script(clause.body, context_ns)
                if else_body is not None:
                    _scan_script(else_body, context_ns)
            case IRFor(init=init, body=body, next=next_s):
                _scan_script(init, context_ns)
                _scan_script(body, context_ns)
                _scan_script(next_s, context_ns)
            case IRWhile(body=body):
                _scan_script(body, context_ns)
            case IRForeach(body=body):
                _scan_script(body, context_ns)
            case IRSwitch(arms=arms, default_body=default_body):
                for arm in arms:
                    if arm.body is not None:
                        _scan_script(arm.body, context_ns)
                if default_body is not None:
                    _scan_script(default_body, context_ns)
            case IRCatch(body=body):
                _scan_script(body, context_ns)
            case IRTry(body=body, finally_body=finally_body):
                _scan_script(body, context_ns)
                if finally_body is not None:
                    _scan_script(finally_body, context_ns)
            case IRBlock(body=body, namespace=ns):
                # ``namespace eval ::ns { … }`` — the body runs with
                # the block's own namespace, not the enclosing one.
                _scan_script(body, ns if ns else context_ns)
            case _:
                pass

    def _scan_script(script: IRScript, context_ns: str) -> None:
        for stmt in script.statements:
            _scan_statement(stmt, context_ns)

    _scan_script(ir_module.top_level, "::")
    for qname, proc in ir_module.procedures.items():
        # Derive the enclosing ns from the proc's qualified name:
        # ``::ns::foo`` runs in ``::ns``; ``::foo`` runs in ``::``.
        parent_ns = qname.rsplit("::", 1)[0] or "::"
        _scan_script(proc.body, parent_ns)
    if ir_module.methods:
        for method in ir_module.methods.values():
            # Methods run in their class's namespace; use root as a
            # conservative default — the class name isn't a
            # ``namespace eval`` target, but qualified ``rename``
            # inside a method is so rare that the extra root probe
            # is harmless.
            _scan_script(method.body, "::")
    return affected, full_flush[0]


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
) -> WasmModule:
    """Generate a complete WASM module from a CFG module.

    Args:
        cfg_module: The control-flow-graph module to compile.
        ir_module: The IR module (for procedure metadata).
        optimise: When ``True``, enable optimisation passes
            (constant folding, peephole, dead-code elimination).
            When ``False``, emit straightforward unoptimised code.
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

    module = WasmModule()

    # Phase 1: Pre-scan IR to find which runtime imports are needed
    needed_imports = _scan_needed_imports(cfg_module, ir_module)

    # Phase 2: Register needed imports (these occupy the first function indices)
    shared_imports: dict[str, int] = {}
    for import_key in sorted(needed_imports):
        spec = _RUNTIME_IMPORTS.get(import_key)
        if spec is None:
            continue
        mod_name, func_name, params, results = spec
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
    dynamically_modified, full_flush = _collect_dynamically_modified_procs(ir_module)
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
        proc_defaults=proc_defaults,
        proc_args_tail=proc_args_tail,
        shared_strings=shared_strings,
        shared_string_index=shared_string_index,
        shared_string_offset=shared_string_offset,
        diag_map=diag_map,
        escape_summary=top_escape,
        proc_imports=proc_imports,
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
        callable_emitter = _WasmEmitter(
            cfg_func,
            params=params,
            optimise=optimise,
            is_proc=True,
            shared_imports=shared_imports,
            proc_index=callable_proc_index,
            proc_defaults=proc_defaults,
            proc_args_tail=proc_args_tail,
            shared_strings=shared_strings,
            shared_string_index=shared_string_index,
            shared_string_offset=shared_string_offset,
            proc_qname=qname,
            diag_map=diag_map,
            escape_summary=proc_escape,
            proc_imports=proc_imports,
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
