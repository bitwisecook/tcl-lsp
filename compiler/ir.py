"""Intermediate representation (IR) for Tcl analysis.

This is a structured IR front-end that keeps source ranges on all nodes.
Later passes can lower this further to CFG + SSA.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING

from core.analysis.semantic_model import Range

from .expr_ast import ExprNode

if TYPE_CHECKING:
    from compiler.parsing.tokens import Token


class InlineDecision(Enum):
    """S4.1 catalogue of inlining-eligibility decisions.

    Set on ``IRProcedure.inline_decision`` by the
    ``compiler.inlining.decision`` policy after var-escape
    analysis has populated ``pure_leaf``.

    ``NEVER``  — the proc is too large, has dynamic-barrier
    constructs (upvar / uplevel / info / tailcall), or any callee is
    not itself ``pure_leaf``.  The S4.2 inliner skips it.

    ``ALWAYS`` — small (≤ ``SMALL_BODY_THRESHOLD`` IR statements) and
    fully ``pure_leaf``.  Inline at every static call site.

    ``IF_SINGLE_CALL`` — ``pure_leaf`` and statically referenced
    exactly once.  Inlining replaces the only call and the original
    proc becomes garbage.

    ``IF_HOT`` — reserved for S4.3 (profile-guided hot-call
    inlining); the static catalogue never assigns this.
    """

    NEVER = "never"
    ALWAYS = "always"
    IF_SINGLE_CALL = "if_single_call"
    IF_HOT = "if_hot"


@dataclass(frozen=True, slots=True)
class CommandTokens:
    """Original parsed tokens for a command invocation.

    Carried on ``IRCall`` and ``IRBarrier`` so downstream passes
    (optimiser, compiler checks) can inspect tokens without re-lexing.
    """

    argv: tuple["Token", ...]
    argv_texts: tuple[str, ...]
    single_token_word: tuple[bool, ...]
    all_tokens: tuple["Token", ...] = ()
    expand_word: tuple[bool, ...] | None = None  # {*} expansion markers


@dataclass(frozen=True, slots=True)
class IRAssignConst:
    range: Range
    name: str
    value: str


@dataclass(frozen=True, slots=True)
class IRAssignExpr:
    range: Range
    name: str
    expr: ExprNode


@dataclass(frozen=True, slots=True)
class IRAssignValue:
    range: Range
    name: str
    value: str
    value_needs_backsubst: bool = False
    tokens: CommandTokens | None = None  # Reserved for future command-token analysis


@dataclass(frozen=True, slots=True)
class IRIncr:
    range: Range
    name: str
    amount: str | None = None
    safe_on_uninit: bool = False


@dataclass(frozen=True, slots=True)
class IRExprEval:
    """Standalone expression evaluation (``expr`` command).

    Evaluates the expression for its result but does not assign it to
    a variable.  The result becomes the command's return value.
    """

    range: Range
    expr: ExprNode


@dataclass(frozen=True, slots=True)
class IRCall:
    """A generic command invocation, optionally annotated with variables it defines.

    When ``reads_own_defs`` is true, the defined variables are also read
    (read-before-write semantics), like ``append`` which reads and then
    extends the variable's current value.

    ``reads`` lists bare variable names that the command reads without
    modification (e.g. ``info exists varName``, ``array get arrayName``).
    These are not ``$varName`` references in the args so the SSA scanner
    cannot detect them automatically.

    ``command`` carries the source spelling — what the user wrote at the
    call site — for diagnostic rendering and source-fidelity passes.
    ``canonical_command`` carries the namespace-qualified form
    (``::ns::cmd``) that alias resolution and namespace prefixing produce,
    set once at lowering and never re-derived.  Downstream analysis,
    optimiser patterns, and registry lookups should match on
    ``canonical_command`` so qualified spellings, ``interp alias`` aliases,
    and namespace imports all hit the same branch.  See issue #246.
    """

    range: Range
    command: str
    args: tuple[str, ...] = ()
    defs: tuple[str, ...] = ()
    reads: tuple[str, ...] = ()
    reads_own_defs: bool = False
    safe_on_uninit: bool = False
    tokens: CommandTokens | None = None
    canonical_command: str = ""
    foreach_groups: tuple[int, ...] | None = None


@dataclass(frozen=True, slots=True)
class IRReturn:
    range: Range
    value: str | None = None
    expr: ExprNode | None = None
    braced: bool = False


@dataclass(frozen=True, slots=True)
class IRBarrier:
    """A command whose side effects defeat static analysis.

    Commands like ``eval``, ``uplevel``, and ``upvar`` can modify
    arbitrary variables at runtime, so no constant propagation or
    dead-store reasoning can cross a barrier.  The ``reason`` field
    is a human-readable label for diagnostic messages; ``command``
    and ``args`` preserve the original call for passes that inspect
    specific barrier shapes (e.g. ``for`` inside a barrier block).

    ``canonical_command`` mirrors :class:`IRCall.canonical_command` —
    the lowering-time canonical form that downstream passes match against.
    """

    range: Range
    reason: str
    command: str = ""
    args: tuple[str, ...] = ()
    tokens: CommandTokens | None = None
    canonical_command: str = ""


@dataclass(frozen=True, slots=True)
class IRInterpBoundary:
    """Marker statement: control is about to enter the Tcl interpreter.

    Inserted by the lowerer (or a pre-codegen pass) immediately before
    any statement that will hand control to the runtime interpreter
    — eval-fallback dispatch, ``[eval $body]`` execution, ``info``
    introspection, dynamic-dispatch, etc.  Codegen translates this
    node into a frame-sync call (mirror compiled-side WASM-locals
    into the runtime frame so the interpreter can resolve names).

    ``kind`` matches the BarrierKind taxonomy in the var-escape
    analysis (``"call"``, ``"eval"``, ``"dispatch"``, ``"info"``)
    so per-kind narrowing in codegen can sync only the subset of
    locals each boundary type actually needs to observe.

    ``vars_observed`` is the *known* set of names the upcoming
    interpreter activity will touch (e.g. names referenced in a
    statically-scanned eval body).  ``None`` means "unknown — sync
    every Tcl local".  An empty set explicitly skips the sync.

    Centralising the boundary as data (an IR node) rather than as
    scattered ``_emit_frame_sync()`` calls in codegen makes
    "forgot to sync before this interp call" structurally
    impossible — the sync is part of the IR.
    """

    range: Range
    kind: str
    vars_observed: frozenset[str] | None = None
    detail: str = ""


@dataclass(frozen=True, slots=True)
class IRScript:
    statements: tuple["IRStatement", ...] = ()


@dataclass(frozen=True, slots=True)
class IRBlock:
    """Inline group of statements.

    Used by ``namespace eval`` lowering to splice the body's
    statements into the enclosing script without introducing a
    separate scope — the body's ``proc`` definitions are already
    lifted into ``module.procedures`` with qualified names, so what
    remains (variable, trace, Option, if, …) runs as plain top-level
    code.  The WASM codegen flattens :class:`IRBlock` nodes during
    emission.

    ``namespace`` holds the fully-qualified namespace the body was
    lowered in (e.g. ``::tcltest``) so codegen can resolve
    unqualified command names to the right procedure when the body
    calls its own helpers (``Option -verbose …`` → ``::tcltest::Option``).

    ``source_args`` keeps the original ``namespace eval`` args
    (``("eval", ns, body_text)``) so a codegen target that can't
    inline (the stack-VM) can still dispatch the call with full
    namespace semantics at runtime.
    """

    range: Range
    body: IRScript
    namespace: str = "::"
    source_args: tuple[str, ...] = ()
    source_tokens: CommandTokens | None = None


@dataclass(frozen=True, slots=True)
class IRUpFrame:
    """Frame-shifted inline execution of a static ``uplevel`` body.

    Produced by the ``uplevel`` lowering hook when both the level
    (default 1, bare integer, or ``#N``) and body (braced literal
    free of nested dynamic barriers) are statically decidable from
    tokens.  Codegen emits a ``tcl_frame_depth_stash`` around the
    inlined body IR, then ``tcl_frame_depth_restore`` afterwards —
    the same pattern today's ``_emit_cmd_uplevel`` uses around a
    ``tcl_eval`` call, except the body runs as compiled IR so the
    caller's locals remain visible.

    ``frame_shift`` encodes the stash argument:
    ``0`` means inline execution at the current frame (used for
    ``eval {…}`` if ever routed through this node — currently the
    ``eval`` hook emits :class:`IRBlock` instead, but the encoding
    leaves the door open).  ``1`` means one frame up.  The sentinel
    ``0x3FFF_FFFF`` mirrors ``_emit_cmd_uplevel``'s shift for
    ``uplevel #0`` — stash clamps to global regardless of depth.

    ``source_tokens`` preserves the original parsed tokens so
    non-inlining codegen targets can fall back to the string-level
    ``uplevel`` dispatch when they cannot emit inline IR.
    """

    range: Range
    frame_shift: int
    body: IRScript
    source_tokens: CommandTokens | None = None


@dataclass(frozen=True, slots=True)
class IRIfClause:
    condition: ExprNode
    condition_range: Range
    body: IRScript
    body_range: Range


@dataclass(frozen=True, slots=True)
class IRIf:
    range: Range
    clauses: tuple[IRIfClause, ...]
    else_body: IRScript | None = None
    else_range: Range | None = None


@dataclass(frozen=True, slots=True)
class IRFor:
    range: Range
    init: IRScript
    init_range: Range
    condition: ExprNode
    condition_range: Range
    next: IRScript
    next_range: Range
    body: IRScript
    body_range: Range
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRWhile:
    """A while loop: test an expression, execute body while true."""

    range: Range
    condition: ExprNode
    condition_range: Range
    body: IRScript
    body_range: Range
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRForeach:
    """Iterate over one or more lists, assigning elements to loop variables.

    Covers both ``foreach``, ``lmap``, and ``dict for``/``dict map``.
    Each ``(var_list, list_arg)`` pair in ``iterators`` corresponds to
    one varList/list argument group.
    """

    range: Range
    iterators: tuple[tuple[tuple[str, ...], str], ...]
    body: IRScript
    body_range: Range
    is_lmap: bool = False
    raw_args: tuple[str, ...] = ()
    is_dict_iteration: bool = False


@dataclass(frozen=True, slots=True)
class IRCatch:
    """``catch script ?resultVar? ?optionsVar?`` — evaluate body and trap exceptions."""

    range: Range
    body: IRScript
    body_range: Range
    result_var: str | None = None
    options_var: str | None = None
    raw_args: tuple[str, ...] = ()
    # Original parsed tokens (incl. braced/quoted flags).  Threaded
    # through so the cfg's IRCatch → IRCall lowering can preserve the
    # ``{...}`` braces around the body when the codegen falls back
    # to ``tcl_eval``.  Without this, ``catch {$undef} msg`` gets
    # reconstructed as ``catch $undef msg`` and the unset-var read
    # fires before catch can intercept it.
    tokens: CommandTokens | None = None


@dataclass(frozen=True, slots=True)
class IRTryHandler:
    """One ``on``/``trap`` handler clause inside a ``try`` statement."""

    kind: str  # "on" or "trap"
    match_arg: str  # return code or error class pattern
    var_name: str | None  # variable bound to result
    options_var: str | None  # variable bound to options dict
    body: IRScript
    body_range: Range


@dataclass(frozen=True, slots=True)
class IRTry:
    """``try body ?on code varList body ...? ?finally body?`` — structured exception handling."""

    range: Range
    body: IRScript
    body_range: Range
    handlers: tuple[IRTryHandler, ...] = ()
    finally_body: IRScript | None = None
    finally_range: Range | None = None
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRSwitchArm:
    pattern: str
    pattern_range: Range
    body: IRScript | None
    body_range: Range | None
    fallthrough: bool = False


@dataclass(frozen=True, slots=True)
class IRSwitch:
    range: Range
    subject: str
    subject_range: Range
    arms: tuple[IRSwitchArm, ...] = ()
    default_body: IRScript | None = None
    default_range: Range | None = None
    mode: str = "exact"  # "exact", "glob", or "regexp"
    nocase: bool = False
    raw_args: tuple[str, ...] = ()  # original command args for generic fallback


@dataclass(frozen=True, slots=True)
class IRProcedure:
    name: str
    qualified_name: str
    params: tuple[str, ...]
    range: Range
    body: IRScript
    params_raw: str = ""
    body_source: str | None = None  # None for synthetic procs (``when``)
    namespace_scoped: bool = False  # True when defined inside namespace eval
    base_priority: int = 500  # BigIP handler priority (0–2**32-1, default 500)
    # S4.1: inlining-eligibility tag.  Computed by
    # ``compiler.inlining.decision`` after var-escape analysis.
    # Default ``NEVER`` so any proc that hasn't been classified is
    # opaque to the inliner.
    inline_decision: InlineDecision = InlineDecision.NEVER
    # S4.1: number of statically resolved call sites for this proc
    # across the whole module (top-level + every other proc body).
    # Used as the gate for ``IF_SINGLE_CALL``.  ``0`` means the
    # catalogue hasn't run.
    static_call_count: int = 0
    # PR #237 review: True when the compiler synthesised this
    # procedure (e.g. as a helper extracted by an optimisation
    # pass) and is therefore safe to delete after inlining its
    # call sites.  Lowering from user source code never sets this
    # — Tcl ``proc`` definitions register externally observable
    # commands and must survive inlining unconditionally.  The
    # dead-proc-removal pass in :mod:`compiler.inlining`
    # consults this flag before pruning.
    compiler_synthetic: bool = False


@dataclass(frozen=True, slots=True)
class IRMethodDef:
    """A method definition within a class body.

    Compiles like IRProcedure but carries class context for
    interprocedural analysis and devirtualisation.
    """

    class_name: str
    method_name: str
    params: tuple[str, ...]
    body: IRScript
    kind: str = "method"  # "method" | "classmethod" | "constructor" | "destructor"
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class CommandTrace:
    """A static record of a ``trace add/remove execution`` directive.

    Captured at lowering time so downstream side-effect classification
    can union the trace body's effects into every call to ``target``.
    Variable / command traces are not captured here — only execution
    traces, which are the form whose body composes into the traced
    command's effective side-effects (issue #251).

    ``body`` is ``None`` when the body argument is dynamic
    (non-literal — e.g. a ``$variable`` or ``[command]`` substitution);
    callers must treat dynamic bodies as worst-case.
    """

    target: str
    """Command name being traced (e.g. ``"set"``, ``"::ns::proc"``)."""

    ops: tuple[str, ...]
    """Operations the trace fires on (``"enter"``/``"leave"``/``"enterstep"``/``"leavestep"``)."""

    body: str | None
    """Literal trace-body script, or ``None`` when the body is dynamic."""

    body_range: Range | None = None
    """Source range of the body argument (when known)."""

    action: str = "add"
    """``"add"`` or ``"remove"`` — distinguishes registration from removal."""

    target_dynamic: bool = False
    """True when the *target* command name itself is a non-literal."""


@dataclass
class IRModule:
    top_level: IRScript = field(default_factory=IRScript)
    procedures: dict[str, IRProcedure] = field(default_factory=dict)
    methods: dict[str, IRMethodDef] = field(default_factory=dict)
    redefined_procedures: set[str] = field(default_factory=set)
    # Static ``namespace import`` directives captured at lowering time.
    # Each entry is ``(context_namespace, pattern)`` — the namespace
    # that executed the import and the raw pattern argument (either a
    # fully-qualified single name like ``::tcltest::test`` or a glob
    # like ``::tcltest::*``).  Codegen resolves patterns against the
    # final ``procedures`` table to build the compile-time import
    # lookup so unqualified calls (``test name desc body``) dispatch
    # directly to ``::tcltest::test`` instead of falling back to
    # ``tcl_eval``.
    namespace_imports: tuple[tuple[str, str], ...] = ()

    # Captured ``namespace export`` directives.  Each entry is
    # ``(source_namespace, pattern)`` — the namespace whose body ran
    # ``namespace export pattern`` and the raw glob pattern.  Used
    # by codegen to filter the ``namespace_imports``-derived
    # compile-time shortcut so only commands explicitly exported by
    # the source namespace are eligible for direct dispatch (matches
    # C Tcl's ``Tcl_Import`` semantics).  An importing namespace
    # with no matching export falls back to the runtime dispatch
    # path, where the interpreter can apply the correct
    # "unknown command" diagnostic.
    namespace_exports: tuple[tuple[str, str], ...] = ()

    # ``trace add/remove execution`` directives captured at lowering
    # time.  Traces installed on a command compose into that command's
    # effective side-effects: a trace on a registry-pure command (e.g.
    # ``set``) is no longer pure because the trace body runs around
    # every call.  Side-effect classification consults
    # :meth:`traced_commands` to gate purity / CSE on traced names.
    # See ``docs/design/compiler/side-effects-system.md`` for the
    # composition rule and ``docs/design/compiler/lowering-contracts.md``
    # for the capture contract.  Out of scope here:
    # ``trace add command`` and ``trace add variable`` (different
    # semantics — handled, if at all, in their own captures).
    command_traces: tuple["CommandTrace", ...] = ()

    def traced_commands(self) -> frozenset[str]:
        """Return the set of command names with a net active execution trace.

        Walks ``command_traces`` in source order, modelling the global
        command-table state.  Tcl matches ``trace remove execution`` by
        the full registration tuple (target, ops, command prefix); a
        non-matching ``remove`` is a runtime no-op.  We mirror that
        semantics conservatively:

        - An ``add`` with a literal target appends ``(target, ops_set,
          body)`` to the active list.
        - A ``remove`` cancels at most one matching add — same target,
          same ops set, same literal body.  Removes with a dynamic ops
          list, dynamic body, or dynamic target leave the active list
          alone (the safe over-approximation: the trace might still be
          installed, so keep treating the command as traced).
        - Dynamic-target adds are not folded into the named set;
          callers that need the worst-case over-approximation must
          check :meth:`has_dynamic_trace` separately.
        """
        active: list[tuple[str, frozenset[str], str | None]] = []
        for trace in self.command_traces:
            if trace.target_dynamic:
                continue
            if trace.action == "add":
                active.append((trace.target, frozenset(trace.ops), trace.body))
                continue
            # ``remove`` — only cancel a matching add when target, ops,
            # and body are all known.  Skip otherwise (over-pessimise).
            if not trace.ops or trace.body is None:
                continue
            key = (trace.target, frozenset(trace.ops), trace.body)
            try:
                active.remove(key)
            except ValueError:
                continue
        return frozenset(target for target, _ops, _body in active)

    def has_dynamic_trace(self) -> bool:
        """True when any ``trace add execution`` had a non-literal target.

        When this flag is set, downstream effect classification cannot
        rule out *any* command being traced and must pessimise calls
        whose purity matters for correctness.
        """
        return any(t.target_dynamic and t.action == "add" for t in self.command_traces)


def when_event_name(qualified_name: str) -> str:
    """Extract the event name from a ``::when::`` qualified name.

    Handles both ``::when::HTTP_REQUEST`` and indexed forms like
    ``::when::HTTP_REQUEST#1``.
    """
    bare = qualified_name.removeprefix("::when::")
    idx = bare.find("#")
    return bare[:idx] if idx >= 0 else bare


IRStatement = (
    IRAssignConst
    | IRAssignExpr
    | IRAssignValue
    | IRExprEval
    | IRIncr
    | IRCall
    | IRReturn
    | IRBarrier
    | IRInterpBoundary
    | IRIf
    | IRFor
    | IRWhile
    | IRForeach
    | IRCatch
    | IRTry
    | IRSwitch
    | IRBlock
    | IRUpFrame
)
