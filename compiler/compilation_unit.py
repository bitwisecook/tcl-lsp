"""Shared compilation artefacts for a single source document.

Built once per diagnostics cycle, consumed by the analyser, optimiser,
shimmer analysis, and compiler checks.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from shared.naming import normalise_qualified_name

from .cfg import (
    CFGFunction,
    CFGModule,
    build_cfg_function,
    cfg_context_fingerprint,
    prepare_cfg_context,
)
from .command_trust import command_trust, module_command_trust
from .connection_scope import ConnectionScope, build_connection_scope
from .execution_intent import FunctionExecutionIntent, build_function_execution_intent
from .ir import IRBarrier, IRBlock, IRCall, IRModule, IRStatement
from .lowering import lower_to_ir
from .ssa import DEEP_ANALYSIS_BODY_BYTES, SSAFunction, build_ssa, is_complexity_guarded

if TYPE_CHECKING:
    from compiler.interprocedural import InterproceduralAnalysis, ProcLocalSummary
    from .core_analyses import FunctionAnalysis

_oo_metaclass_cache: frozenset[str] | None = None


def _oo_metaclasses() -> frozenset[str]:
    global _oo_metaclass_cache
    if _oo_metaclass_cache is None:
        from compiler.registry import REGISTRY

        _oo_metaclass_cache = REGISTRY.check_trait_commands("is_oo_metaclass")
    return _oo_metaclass_cache


# snit type-definers (tcllib).  ``snit::type Name body`` makes ``Name`` a
# class whose instances are created via ``Name create x`` / ``Name %AUTO%`` /
# (widgets) ``Name .path`` — recognised in ``_return_type_for_command`` so the
# created object's variable is typed ``OBJECT`` (suppresses W307 dispatch FPs).
_SNIT_DEFINERS: frozenset[str] = frozenset(
    {
        "snit::type",
        "snit::widget",
        "snit::widgetadaptor",
        "::snit::type",
        "::snit::widget",
        "::snit::widgetadaptor",
    }
)


def _extract_class_names(ir_module: IRModule) -> frozenset[str]:
    """Extract user-defined TclOO / snit class names from IR statements.

    Scans ``oo::class create ClassName`` (and similar metaclass) patterns plus
    ``snit::type Name`` / ``snit::widget`` / ``snit::widgetadaptor`` in the
    top-level script and procedure bodies.
    """
    names: set[str] = set()

    def _qualify(name: str, namespace: str) -> str:
        # Absolute names live at the global root; relative names resolve against
        # the enclosing namespace (mirrors tclsh ``create`` and the analyser's
        # ``_qualify_oo_name``).  normalise collapses any doubled ``::``.
        if name.startswith("::"):
            return normalise_qualified_name(name)
        return normalise_qualified_name(f"{namespace}::{name}")

    def _scan(stmts: tuple[IRStatement, ...], namespace: str = "::") -> None:
        for stmt in stmts:
            if isinstance(stmt, IRBlock):
                # ``namespace eval ns {…}`` — recurse with the block's namespace
                # so a relative ``oo::class create Foo`` inside it is recorded as
                # ``::ns::Foo`` (was ``::Foo``, so a relative ``[Foo new]`` in the
                # namespace never matched → W307 instead of object typing).
                _scan(stmt.body.statements, stmt.namespace or namespace)
                continue
            cmd: str = ""
            args: tuple[str, ...] = ()
            if isinstance(stmt, (IRCall, IRBarrier)):
                cmd, args = stmt.command, stmt.args
            if (
                cmd in _oo_metaclasses()
                and len(args) >= 2
                and args[0] in ("create", "createWithNamespace")
            ):
                names.add(_qualify(args[1], namespace))
            elif cmd in _SNIT_DEFINERS and args:
                names.add(_qualify(args[0], namespace))

    _scan(ir_module.top_level.statements)
    for qname, proc in ir_module.procedures.items():
        # A class created in a proc body is named relative to the proc's own
        # namespace (``proc ::ns::p {} { oo::class create C }`` ⇒ ``::ns::C``).
        proc_ns = normalise_qualified_name(qname).rsplit("::", 1)[0] or "::"
        _scan(proc.body.statements, proc_ns)
    return frozenset(names)


@dataclass(frozen=True, slots=True)
class FunctionUnit:
    """Pre-computed artefacts for a single function.

    ``complexity_guarded`` is the single source of truth for the deep-analysis
    complexity guard: when True (CFG block count OR proc body bytes over the
    ceiling), ``ssa``/``analysis`` are trivial and **every** per-proc diagnostic
    pass must skip this function (consult the flag, not the cfg, so byte-large-
    but-block-light generated bodies are guarded consistently).
    """

    cfg: CFGFunction
    ssa: SSAFunction
    analysis: FunctionAnalysis
    execution_intent: FunctionExecutionIntent
    complexity_guarded: bool = False


@dataclass(frozen=True, slots=True)
class CompilationUnit:
    """Shared compilation artefacts for a single source document."""

    source: str
    ir_module: IRModule
    cfg_module: CFGModule
    top_level: FunctionUnit
    procedures: dict[str, FunctionUnit]
    interproc: InterproceduralAnalysis
    # TclOO method bodies lowered to per-method FunctionUnits (SF-2).  Keyed
    # by ``{class_qname}::{method_name}``; empty for non-OO sources.  Kept
    # separate from ``procedures`` so per-proc diagnostic passes are
    # unaffected — only the optimiser's O126 gate iterates these.
    methods: dict[str, FunctionUnit] = field(default_factory=dict)
    connection_scope: ConnectionScope | None = None
    # The TclOO/snit class-name set the per-proc analyses were built under;
    # cache builders fingerprint it so a class change elsewhere invalidates
    # unchanged procs (see ``known_classes_fingerprint``).
    known_classes: frozenset[str] = frozenset()


def ensure_compilation_unit(
    source: str,
    cu: CompilationUnit | None = None,
    *,
    logger: logging.Logger | None = None,
    context: str = "compiler",
    failure_detail: str = "compilation failed; continuing without CompilationUnit",
    known_classes: frozenset[str] = frozenset(),
    deep_param_traits: bool = False,
) -> CompilationUnit | None:
    """Return a usable ``CompilationUnit`` by reusing or compiling.

    This is the canonical adapter for pass entry points that accept
    ``source`` and an optional pre-built ``CompilationUnit``.

    Set *deep_param_traits* for offline analytics paths (``tcl
    callgraph``, the compiler explorer, the MCP server) to opt into
    the deeper ``infer_param_traits_deep`` pass that descends into
    nested script bodies.  The LSP synchronous path leaves this off
    so per-keystroke analysis stays bounded.
    """
    if cu is not None:
        return cu
    try:
        return compile_source(
            source, known_classes=known_classes, deep_param_traits=deep_param_traits
        )
    except Exception:
        if logger is not None:
            logger.debug(
                "%s: %s",
                context,
                failure_detail,
                exc_info=True,
            )
        return None


def _proc_cache_key(
    source: str,
    qname: str,
    start_offset: int,
    end_offset: int,
    stub_fingerprint: int = 0,
    context_fingerprint: int = 0,
    start_line: int = 0,
    start_char: int = 0,
    known_classes_fp: int = 0,
    call_site_constants_fp: int = 0,
) -> tuple[str, int] | None:
    """Build a procedure cache key from source offsets.

    The fingerprint covers the active stub overlay because cached
    summaries depend on how role-aware lookups resolve ``ArgRole.BODY``
    / ``ArgRole.EXPR`` for the commands the proc invokes — adding,
    removing, or changing a stub must invalidate cached summaries
    even when the proc body text is unchanged.

    ``context_fingerprint`` covers the module-global CFG-construction context
    (``cfg_context_fingerprint``): a caller's CFG/analysis depends on which
    callees write back into the caller frame via ``upvar``, so a callee edit
    that changes that must invalidate the caller even though its text is
    unchanged.

    ``known_classes_fp`` covers the TclOO/snit class-name set fed to
    ``analyse_function``: ``[ClassName new]`` propagates an OBJECT type only
    when ``ClassName`` is known, so adding/renaming a class elsewhere must
    invalidate an otherwise-unchanged proc's cached object-type facts.

    ``call_site_constants_fp`` covers the literal call-site args seeded into
    ``analyse_function`` as ``param_constants``: editing a caller's literal arg
    (``foo 1`` → ``foo 0``) changes the proc's SCCP / unreachable-branch facts
    even though the proc's own text and position are unchanged, so it must
    invalidate the cached unit (see :func:`call_site_constants_fingerprint`).

    The proc's **start line/char AND byte offset** are all part of the key. The
    cached ``FunctionUnit``'s CFG/SSA carry *absolute* source positions — both
    the LSP-visible line/char *and* byte offsets that flow into optimiser /
    code-action edit data (``startOffset``/``endOffset``). A same-line length
    change *before* a proc leaves its line/char stable but shifts every absolute
    offset inside it, so keying on line/char alone would wrongly reuse a unit
    whose offsets are now stale; including ``start_offset`` invalidates it.
    """
    if start_offset < 0 or end_offset < start_offset or end_offset > len(source):
        return None
    # end_offset is the proc's last character (inclusive), so slice
    # end-exclusive at end_offset+1 — otherwise a same-length edit to the
    # proc's final character leaves the hash unchanged and the cached unit is
    # wrongly reused.  Must stay in lockstep with _build_proc_cache's slice.
    return (
        qname,
        hash(
            (
                source[start_offset : end_offset + 1],
                stub_fingerprint,
                context_fingerprint,
                start_line,
                start_char,
                start_offset,
                known_classes_fp,
                call_site_constants_fp,
            )
        ),
    )


def _proc_reposition_key(
    source: str,
    qname: str,
    start_offset: int,
    end_offset: int,
    stub_fingerprint: int = 0,
    context_fingerprint: int = 0,
    known_classes_fp: int = 0,
    call_site_constants_fp: int = 0,
) -> tuple[str, int] | None:
    """Build a *position-independent* procedure key for the reposition cache.

    This is :func:`_proc_cache_key` with the proc's start line/char/offset
    deliberately **excluded**.  Two compilations whose only difference is that
    the proc moved (e.g. a blank line inserted above it) produce the same
    reposition key even though their primary cache keys differ.

    A hit means the body text and every non-positional validity input (stub
    overlay, CFG-construction context, known-class set, call-site constants) are
    unchanged, so the proc's *dataflow* is identical — only its absolute source
    ranges shifted.  The caller may therefore reuse the cached unit's
    position-independent ``analysis`` / ``execution_intent`` (keyed by block
    name + statement index + SSA value key, never by absolute position) and
    rebuild just the cheap CFG + SSA to recover correct ranges.  The validity
    contract matches the primary cache exactly minus position, so this is no
    less sound than the existing whole-unit reuse — including
    ``call_site_constants_fp`` so a moved proc whose caller also changed a
    literal arg is not served stale SCCP / unreachable-branch facts.
    """
    if start_offset < 0 or end_offset < start_offset or end_offset > len(source):
        return None
    # Slice end-exclusive at end_offset+1 to match _proc_cache_key (end_offset
    # is the proc's last character, inclusive).
    return (
        qname,
        hash(
            (
                source[start_offset : end_offset + 1],
                stub_fingerprint,
                context_fingerprint,
                known_classes_fp,
                call_site_constants_fp,
            )
        ),
    )


def known_classes_fingerprint(known_classes: frozenset[str]) -> int:
    """Stable per-process fingerprint of the TclOO/snit class-name set.

    Folded into proc-cache keys (and the incremental shimmer body hash) so that
    adding, removing, or renaming a class definition invalidates an unchanged
    proc whose object-type facts depend on the class set.  Exposed so cache
    builders outside :func:`compile_source` produce matching keys.
    """
    return hash(tuple(sorted(known_classes)))


def call_site_constants_fingerprint(
    call_site_constants: "dict[str, dict[int, set[object]]]",
) -> dict[str, int]:
    """Per-proc fingerprint of the literal values flowing into each proc's
    params from its call sites.

    ``analyse_function`` is seeded with ``param_constants`` derived (via
    ``_params_constants_from_call_sites``) from the literal args every call site
    passes a proc.  Those facts drive SCCP / unreachable-branch results, so a
    proc whose *body text and position are unchanged* can still need
    re-analysis when a caller edits a literal arg (``foo 1`` → ``foo 0``).  The
    proc-cache / reposition keys therefore fold this fingerprint in, or an
    unchanged-but-restamped proc would serve stale branch facts (the
    ``analyse_function`` ``param_constants`` seed is invisible to a body-text
    key).

    Returns ``{qname -> fingerprint}``; a proc absent from *call_site_constants*
    (no user-proc call sites) maps to ``0`` via the callers' ``.get(qname, 0)``.
    Arg indices are sorted and each value set is rendered to a sorted tuple of
    ``(typename, repr)`` pairs so a ``1`` (int) and ``"1"`` (str) never collide.
    The ``_UNKNOWN_ARG`` sentinel (a bare ``object()`` whose ``repr`` embeds its
    address) is rendered to a fixed token so the fingerprint is reproducible.
    """
    from compiler.core_analyses import _UNKNOWN_ARG

    def _render(v: object) -> tuple[str, str]:
        if v is _UNKNOWN_ARG:
            return ("", "<unknown-arg>")
        return (type(v).__name__, repr(v))

    out: dict[str, int] = {}
    for qname, by_idx in call_site_constants.items():
        items = tuple(
            (idx, tuple(sorted(_render(v) for v in by_idx[idx]))) for idx in sorted(by_idx)
        )
        out[qname] = hash(items)
    return out


def compute_stub_fingerprint(source: str) -> int:
    """Return the stub-overlay fingerprint for *source*.

    Exposed so cache builders outside :func:`compile_source` (e.g. the
    LSP workspace's ``_build_proc_cache``) can produce keys that match
    those :func:`compile_source` will look up.  Returns ``0`` when the
    source declares no stubs — the empty-overlay case stays compatible
    with callers that omit the fingerprint argument.
    """
    from compiler.registry.runtime import signatures_from_stubs
    from compiler.registry.stub_comments import ambient_cmd_stubs, scan_source_for_stubs

    cmd_stubs, _ = scan_source_for_stubs(source)
    cmd_stubs = [*cmd_stubs, *ambient_cmd_stubs()]
    if not cmd_stubs:
        return 0
    overlay = signatures_from_stubs(cmd_stubs)
    return hash(tuple(sorted(overlay.keys()))) ^ hash(
        tuple(sorted(repr(v) for v in overlay.values()))
    )


def compile_source(
    source: str,
    *,
    ir_module: IRModule | None = None,
    proc_cache: dict[tuple[str, int], FunctionUnit] | None = None,
    reposition_cache: dict[tuple[str, int], FunctionUnit] | None = None,
    interproc_cache: dict[tuple[str, int], ProcLocalSummary] | None = None,
    prune_interproc_cache: bool = True,
    known_classes: frozenset[str] = frozenset(),
    deep_param_traits: bool = False,
) -> CompilationUnit:
    """Run the full pipeline once and return cached artefacts.

    When *ir_module* is provided, the lowering step is skipped and the
    pre-built IR is used directly.  This is the incremental path: the
    caller has already assembled the ``IRModule`` from cached and
    freshly-lowered chunk IR.

    When *proc_cache* is provided, procedure ``FunctionUnit`` values
    whose source text has not changed are reused instead of rebuilding
    SSA and dataflow analysis from scratch.  The cache key is
    ``(qualified_name, hash(procedure_source_text))``.

    When *reposition_cache* is provided, a procedure whose body text is
    unchanged but which has *moved* (a primary-cache miss because its absolute
    positions shifted) reuses the cached unit's position-independent ``analysis``
    / ``execution_intent`` and rebuilds only the cheap CFG + SSA, instead of
    re-running the expensive dataflow analysis.  See :func:`_proc_reposition_key`.

    When *interproc_cache* is provided, local interprocedural summaries
    are also reused for unchanged procedures using the same key shape.
    """
    from compiler.parsing.green_tree import green_tree_scope
    from compiler.registry.runtime import stub_signature_scope
    from compiler.registry.stub_comments import ambient_cmd_stubs, scan_source_for_stubs

    cmd_stubs, _ = scan_source_for_stubs(source)
    cmd_stubs = [*cmd_stubs, *ambient_cmd_stubs()]
    # Fingerprint the stub overlay so cached proc / interproc summaries
    # are invalidated whenever a stub is added, removed, or changed —
    # the proc body text alone is not enough because summaries depend
    # on the role-aware command lookups the overlay drives.
    stub_fingerprint = compute_stub_fingerprint(source)
    # Share one green-tree intern index across the whole compile so repeated
    # lexing of the same body/command text (lowering + the var_refs scanners
    # that _read_before_set runs several passes of) is paid once.  The
    # foreground analyser path already wraps its work in green_tree_scope; the
    # deep-diagnostics / optimiser / subprocess path reaches compile_source
    # without one, so it re-lexed from scratch on every per-edit call.
    # Reentrant: a caller that already opened a scope just reuses it.
    with green_tree_scope(), stub_signature_scope(cmd_stubs):
        return _compile_source_inner(
            source,
            ir_module=ir_module,
            proc_cache=proc_cache,
            reposition_cache=reposition_cache,
            interproc_cache=interproc_cache,
            prune_interproc_cache=prune_interproc_cache,
            known_classes=known_classes,
            stub_fingerprint=stub_fingerprint,
            deep_param_traits=deep_param_traits,
        )


def _compile_source_inner(
    source: str,
    *,
    ir_module: IRModule | None,
    proc_cache: dict[tuple[str, int], FunctionUnit] | None,
    reposition_cache: dict[tuple[str, int], FunctionUnit] | None = None,
    interproc_cache: dict[tuple[str, int], ProcLocalSummary] | None,
    prune_interproc_cache: bool,
    known_classes: frozenset[str],
    stub_fingerprint: int = 0,
    deep_param_traits: bool = False,
) -> CompilationUnit:
    from compiler.interprocedural import analyse_interprocedural_ir  # deferred (cycle)
    from .core_analyses import analyse_function  # deferred (cycle)

    if ir_module is None:
        ir_module = lower_to_ir(source)

    # P8.2: specialise Option-shape factory calls at every literal-
    # args call site.  Runs before CFG / SSA so the synthesised
    # child procs participate in the rest of the pipeline just like
    # any other IR-lowered proc.  Pass is a no-op when no factories
    # are detected — amortised cost is a single proc walk.
    from .passes.specialise_factories import specialise_factories

    specialise_factories(ir_module)

    # Insert IRInterpBoundary markers before each IRBarrier so the
    # frame-sync at every interpreter-crossing statement lives in IR
    # rather than as scattered ``_emit_frame_sync()`` calls in
    # codegen.  Pure-additive transformation; codegen short-circuits
    # on the new node and dispatches to ``_emit_interp_boundary``.
    # Idempotent — the pass skips insertion when an
    # ``IRInterpBoundary`` already precedes the ``IRBarrier``, so
    # ``compile_source(..., ir_module=cu.ir_module)`` re-runs
    # don't accumulate duplicate boundaries.  Run once here.
    from .passes.interp_boundaries import insert_interp_boundaries

    ir_module = insert_interp_boundaries(ir_module)
    with command_trust(module_command_trust(ir_module)):
        # Extract TclOO class names from the IR so type propagation can
        # recognise ``[ClassName new]`` as returning an OBJECT instance.
        if not known_classes:
            known_classes = _extract_class_names(ir_module)
        known_classes_fp = known_classes_fingerprint(known_classes)

        upvar_procs, all_proc_params = prepare_cfg_context(ir_module)
        context_fingerprint = cfg_context_fingerprint(upvar_procs, all_proc_params)
        top_cfg = build_cfg_function(
            "::top",
            ir_module.top_level,
            upvar_procs=upvar_procs,
            proc_params=all_proc_params,
            faithful_exceptions=True,
        )
        top_ssa = build_ssa(top_cfg)
        top_analysis = analyse_function(top_cfg, top_ssa, known_classes=known_classes)
        top_unit = FunctionUnit(
            cfg=top_cfg,
            ssa=top_ssa,
            analysis=top_analysis,
            execution_intent=build_function_execution_intent(top_cfg),
        )

        proc_cfgs: dict[str, CFGFunction] = {}
        proc_units: dict[str, FunctionUnit] = {}
        # D3-P2 / D3-P8 closure: collect call-site literal arg values per
        # user proc so the SCCP analysis can fold them.  Shared helper
        # also used by ``analyse_ir_module``; see its docstring for the
        # full reasoning.  The per-proc fingerprint is folded into the
        # proc-cache / reposition keys below so a proc whose body text and
        # position are unchanged is still re-analysed when a caller edits a
        # literal arg (``foo 1`` → ``foo 0``) that seeds its ``param_constants``.
        from compiler.core_analyses import (
            _collect_call_site_constants,
            _params_constants_from_call_sites,
        )

        call_site_constants = _collect_call_site_constants(ir_module)
        call_site_constants_fps = call_site_constants_fingerprint(call_site_constants)

        for qname, ir_proc in ir_module.procedures.items():
            cache_key = _proc_cache_key(
                source,
                qname,
                ir_proc.range.start.offset,
                ir_proc.range.end.offset,
                stub_fingerprint=stub_fingerprint,
                context_fingerprint=context_fingerprint,
                start_line=ir_proc.range.start.line,
                start_char=ir_proc.range.start.character,
                known_classes_fp=known_classes_fp,
                call_site_constants_fp=call_site_constants_fps.get(qname, 0),
            )

            # Try the proc cache before rebuilding CFG + SSA + analysis.
            if proc_cache and cache_key is not None:
                cached = proc_cache.get(cache_key)
                if cached is not None:
                    proc_units[qname] = cached
                    proc_cfgs[qname] = cached.cfg
                    continue

            # Decide the byte-size half of the complexity guard *before* SSA /
            # dataflow: a flat generated command list is block-light (so the
            # block-count guard inside build_ssa/analyse_function never fires) yet
            # byte-huge, and running the full O(blocks·vars) SSA + SCCP/taint/
            # liveness walk on it costs seconds for near-zero findings.  Thread the
            # decision in as a skip flag so those passes return trivial results,
            # rather than computing them and discarding via the guard afterwards.
            body_bytes = ir_proc.range.end.offset - ir_proc.range.start.offset
            byte_guarded = body_bytes > DEEP_ANALYSIS_BODY_BYTES

            cfg = build_cfg_function(
                qname,
                ir_proc.body,
                upvar_procs=upvar_procs,
                proc_params=all_proc_params,
                faithful_exceptions=True,
            )
            proc_cfgs[qname] = cfg

            # Reposition fast-path: the proc body text (and every non-positional
            # validity input) is unchanged but the proc moved, so the primary cache
            # missed only because its absolute positions shifted.  The freshly-built
            # CFG above already carries correct ranges; the cached unit's SSA +
            # dataflow ``analysis`` / ``execution_intent`` are position-independent
            # (keyed by block name + statement index + SSA value key), so reuse them
            # instead of paying for SSA + the ~85%-of-cost dataflow analysis.  We
            # still rebuild SSA (cheap, ~11%) because the cached SSA wraps the *old*
            # IR statements with stale ranges and is consumed for ranges downstream.
            if reposition_cache is not None and not byte_guarded:
                repos_key = _proc_reposition_key(
                    source,
                    qname,
                    ir_proc.range.start.offset,
                    ir_proc.range.end.offset,
                    stub_fingerprint=stub_fingerprint,
                    context_fingerprint=context_fingerprint,
                    known_classes_fp=known_classes_fp,
                    call_site_constants_fp=call_site_constants_fps.get(qname, 0),
                )
                repos = reposition_cache.get(repos_key) if repos_key is not None else None
                if repos is not None and not repos.complexity_guarded:
                    ssa = build_ssa(cfg)
                    proc_units[qname] = FunctionUnit(
                        cfg=cfg,
                        ssa=ssa,
                        analysis=repos.analysis,
                        execution_intent=repos.execution_intent,
                        complexity_guarded=repos.complexity_guarded,
                    )
                    time.sleep(0)  # Yield GIL between procedures
                    continue

            ssa = build_ssa(cfg, force_guard=byte_guarded)
            proc_params = frozenset(ir_proc.params)
            param_constants = _params_constants_from_call_sites(ir_proc, call_site_constants, qname)
            analysis = analyse_function(
                cfg,
                ssa,
                params=proc_params,
                known_classes=known_classes,
                force_guard=byte_guarded,
                param_constants=param_constants,
            )
            # Single complexity-guard decision consulted by every per-proc
            # diagnostic pass: byte-heavy (decided above) OR block-heavy.
            guarded = byte_guarded or is_complexity_guarded(cfg)
            proc_units[qname] = FunctionUnit(
                cfg=cfg,
                ssa=ssa,
                analysis=analysis,
                execution_intent=build_function_execution_intent(cfg),
                complexity_guarded=guarded,
            )
            time.sleep(0)  # Yield GIL between procedures

        # SF-2: lower TclOO method bodies (populated in ir_module.methods by
        # lowering) to per-method FunctionUnits, using the same CFG → SSA →
        # analysis pipeline as procs.  Kept in a separate dict so the per-proc
        # diagnostic passes (which iterate ``procedures``) are unaffected — only
        # the interprocedural purity summary and the O126 optimiser gate consume
        # methods.  Methods are analysis-only artefacts; codegen never reads them.
        method_units: dict[str, FunctionUnit] = {}
        for mqname, ir_method in ir_module.methods.items():
            m_range = ir_method.range
            m_byte_guarded = bool(
                m_range is not None
                and (m_range.end.offset - m_range.start.offset) > DEEP_ANALYSIS_BODY_BYTES
            )
            m_cfg = build_cfg_function(
                mqname,
                ir_method.body,
                upvar_procs=upvar_procs,
                proc_params=all_proc_params,
                faithful_exceptions=True,
            )
            m_ssa = build_ssa(m_cfg, force_guard=m_byte_guarded)
            m_analysis = analyse_function(
                m_cfg,
                m_ssa,
                params=frozenset(ir_method.params),
                known_classes=known_classes,
                force_guard=m_byte_guarded,
            )
            method_units[mqname] = FunctionUnit(
                cfg=m_cfg,
                ssa=m_ssa,
                analysis=m_analysis,
                execution_intent=build_function_execution_intent(m_cfg),
                complexity_guarded=m_byte_guarded or is_complexity_guarded(m_cfg),
            )
            time.sleep(0)  # Yield GIL between methods

        cfg_module = CFGModule(top_level=top_cfg, procedures=proc_cfgs)

        interproc = analyse_interprocedural_ir(
            ir_module,
            source=source,
            proc_local_cache=interproc_cache,
            prune_local_cache=prune_interproc_cache,
            proc_units={qname: (fu.cfg, fu.ssa, fu.analysis) for qname, fu in proc_units.items()},
            method_units={
                qname: (fu.cfg, fu.ssa, fu.analysis) for qname, fu in method_units.items()
            },
            stub_fingerprint=stub_fingerprint,
            context_fingerprint=context_fingerprint,
            deep_param_traits=deep_param_traits,
        )

        when_procs = {qn: fu for qn, fu in proc_units.items() if qn.startswith("::when::")}
        conn_scope = build_connection_scope(when_procs, ir_module) if when_procs else None

        return CompilationUnit(
            source=source,
            ir_module=ir_module,
            cfg_module=cfg_module,
            top_level=top_unit,
            procedures=proc_units,
            interproc=interproc,
            methods=method_units,
            connection_scope=conn_scope,
            known_classes=known_classes,
        )
