"""Intermediate representation (IR) for Tcl analysis.

This is a structured IR front-end that keeps source ranges on all nodes.
Later passes can lower this further to CFG + SSA.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from ..analysis.semantic_model import Range
from .expr_ast import ExprNode

if TYPE_CHECKING:
    from ..parsing.tokens import Token


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
    """

    range: Range
    command: str
    args: tuple[str, ...] = ()
    defs: tuple[str, ...] = ()
    reads: tuple[str, ...] = ()
    reads_own_defs: bool = False
    safe_on_uninit: bool = False
    tokens: CommandTokens | None = None


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
    """

    range: Range
    reason: str
    command: str = ""
    args: tuple[str, ...] = ()
    tokens: CommandTokens | None = None


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

        A trace is "net active" when there are more ``trace add execution``
        directives for the target than ``trace remove execution`` directives
        — modelling the global command-table state at end-of-script.
        Dynamic targets (``target_dynamic=True``) are not folded into the
        named set; callers that need the over-approximation must check
        :meth:`has_dynamic_trace` separately.
        """
        counts: dict[str, int] = {}
        for trace in self.command_traces:
            if trace.target_dynamic:
                continue
            delta = 1 if trace.action == "add" else -1
            counts[trace.target] = counts.get(trace.target, 0) + delta
        return frozenset(name for name, count in counts.items() if count > 0)

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
