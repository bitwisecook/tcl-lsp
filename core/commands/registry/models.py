"""Data models for registry-backed command metadata."""

from __future__ import annotations

import enum
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from .namespace_models import EventRequires
from .signatures import ArgRole, Arity

if TYPE_CHECKING:
    from ...compiler.side_effects import SideEffect, StorageType
    from ...compiler.types import TclType
    from ._base import CommandDef
    from .taint_hints import TaintColour
    from .type_hints import ArgTypeHint


# Dialect status


class DialectStatus(enum.Enum):
    """Tri-state result of a dialect lookup for a command, subcommand, or option."""

    EXISTS = "exists"  # available in this dialect
    DEPRECATED = "deprecated"  # available but deprecated; has a replacement
    DISALLOWED = "disallowed"  # exists in some dialect, but not this one
    NOT_EXISTS = "not_exists"  # not known anywhere


# Format and pattern type enums


class FormatType(str, enum.Enum):
    """Kind of format string for inlay-hint parsing and semantic tokens."""

    SPRINTF = "sprintf"  # format, scan
    CLOCK = "clock"  # clock format, clock scan
    BINARY = "binary"  # binary format, binary scan
    REGSUB = "regsub"  # regsub replacement string (\& backrefs)


class PatternType(str, enum.Enum):
    """Kind of pattern language for semantic tokens and validation."""

    GLOB = "glob"  # string match, glob, lsearch (default), switch (default)
    REGEX = "regex"  # regexp, regsub, lsearch -regexp, switch -regexp


# Hook type aliases
#
# These are Callable signatures for VM execution, codegen, lowering,
# and other optional per-command/subcommand hooks.  ``None`` means
# "use the generic path".
#
# Using string-quoted forward references for types that live in other
# modules (vm/, compiler/) to avoid circular imports.

SubcommandHandler = Callable[..., object]
"""VM execution handler for a subcommand: (interp, args) -> TclResult."""

CommandHandler = Callable[..., object]
"""VM execution handler for a command without subcommands."""

CodegenHook = Callable[..., bool]
"""Bytecode specialisation hook: (emitter, args, ...) -> emitted?"""

LoweringHook = Callable[..., object]
"""IR lowering specialisation hook: (lowerer, ir_call) -> IRStatement | None."""

WasmEmitHook = Callable[..., bool]
"""WASM emit hook: ``(emitter, args, defs, context) -> emitted?``

*context* is an :class:`EmitContext` indicating whether the result is
being dropped / stored (``STATEMENT``) or left on the WASM operand
stack (``VALUE``).  Hooks that don't care about result placement can
ignore the argument.

Keyed by target name (e.g. ``"wasm"``) in the ``codegens`` dict on
``CommandSpec`` and ``SubCommand``.  Unimplemented commands should emit
a trap instruction so execution fails loudly rather than silently.
"""


class EmitContext(enum.Enum):
    """Result placement expected by the WASM emit-hook caller.

    ``STATEMENT`` — the caller will either store the value into a
    designated local (``defs[0]``) or drop it.  Hooks in this context
    must leave *nothing* on the operand stack.

    ``VALUE`` — the caller needs the value on the operand stack
    (e.g. because the command is in implicit-return / tail position or
    appears as the RHS of an assignment).  Hooks in this context must
    leave *exactly one* ``i32`` TclObj pointer on the stack.
    """

    STATEMENT = "statement"
    VALUE = "value"


@dataclass(frozen=True, slots=True)
class WasmRuntimeImport:
    """Describes how a command is dispatched to a Zig runtime import.

    Attached to :class:`CommandSpec` (and :class:`SubCommand` in B.8) so
    the WASM codegen can derive dispatch metadata from the registry
    instead of a shadow table in ``core/compiler/codegen/wasm/_imports.py``.

    Attributes:
        import_key: Key into ``_RUNTIME_IMPORTS`` — the declaration of
            the Zig export name, module, and WASM param/result types.
        argc: Fixed argument count the compiler can assume, or ``None``
            for variadic commands.
        nontrapping: When ``True`` the Zig implementation is total for
            every argument shape the compiler emits — the codegen may
            then elide the per-call diag-site preamble.
    """

    import_key: str
    argc: int | None = None
    nontrapping: bool = False

ArgRoleResolver = Callable[[list[str]], dict[int, ArgRole]]
"""Maps actual argument values to {index: ArgRole} for variable-layout commands."""

ArgTypeResolver = Callable[[tuple[str, ...]], dict[int, "ArgTypeHint"]]
"""Maps actual argument values to {index: ArgTypeHint} for variable-layout commands."""

ConstFoldFunc = Callable[[tuple[str, ...]], str | None]
"""Compile-time constant folder: (resolved_args) -> result string, or None."""

# Note: TaintTransformHook was previously ``Callable[..., object]``.
# It is now just ``TaintColour`` — a static colour-bit value that a
# sanitiser command adds to its tainted output.  The type is written
# inline on the field declarations as ``TaintColour | None`` (resolved
# via TYPE_CHECKING + ``from __future__ import annotations``).

DeprecationFixer = Callable[..., object]
"""Code action for deprecated usage: (cmd, args, arg_tokens, all_tokens) -> CodeFix | None."""

ValidationHook = Callable[..., list[object]]
"""Command-specific validation: (cmd, args, arg_toks, all_toks, source) -> [Diagnostic]."""


# Keyword completion for variable-layout commands


@dataclass(frozen=True, slots=True)
class KeywordCompletion:
    """A completable keyword inside a variable-layout command (if/try/switch).

    Attributes:
        keyword: The keyword text (e.g. "elseif", "else", "on", "finally").
        detail: Description shown in the completion list.
        snippet: Inserted text in LSP snippet format, or ``None`` for plain keyword.
    """

    keyword: str
    detail: str = ""
    snippet: str | None = None


KeywordCompletionProvider = Callable[[list[str]], list[KeywordCompletion]]
"""Given args so far, returns valid keyword completions with optional snippets."""


@dataclass(frozen=True, slots=True)
class EventCommandSet:
    """Pre-computed valid commands for a specific (dialect, event) pair."""

    # Commands that are EXISTS for this dialect AND event-valid.
    valid_commands: frozenset[str]
    # Commands that are EXISTS but NOT event-valid (for demotion, not hiding).
    out_of_event_commands: frozenset[str]
    # Per-command: valid subcommands for this dialect.
    valid_subcommands: dict[str, frozenset[str]]
    # Subset of valid_commands that have event_requires/excluded_events
    # metadata and passed validation — i.e. commands that are *specifically*
    # valid in this event, not merely event-neutral.
    event_scoped_commands: frozenset[str] = frozenset()


@dataclass(frozen=True, slots=True)
class CommandLegality:
    """Pre-computed (event, command) legality matrix for a dialect.

    Allows O(1) lookup of whether a command is legal in a given event
    without re-iterating the registry each time.
    """

    # event_name -> frozenset of valid command names.
    _by_event: dict[str, frozenset[str]]
    # event_name -> frozenset of out-of-event command names.
    _out_of_event: dict[str, frozenset[str]]

    def valid_commands(self, event: str) -> frozenset[str]:
        """Return commands valid in *event*."""
        return self._by_event.get(event, frozenset())

    def out_of_event_commands(self, event: str) -> frozenset[str]:
        """Return commands that exist but aren't valid in *event*."""
        return self._out_of_event.get(event, frozenset())

    def is_legal(self, event: str, command: str) -> bool:
        """Return ``True`` if *command* is legal in *event*."""
        return command in self._by_event.get(event, frozenset())

    def events_for_command(self, command: str) -> list[str]:
        """Return events where *command* is legal."""
        return [ev for ev, cmds in self._by_event.items() if command in cmds]


@dataclass(frozen=True, slots=True)
class RegistryCacheKey:
    """Cache key for filtered registry snapshots.

    Registry snapshots are keyed by the combination of dialect, active
    packages, and Tk mode.  The key space is small (handful of dialects ×
    small package sets), so caching is effective.  Context changes (package
    edits, shebang changes) produce a new key that misses the cache.
    """

    dialect: str | None = None
    active_packages: frozenset[str] | None = None
    tk_mode: str | None = None


@dataclass(frozen=True, slots=True)
class HoverSnippet:
    """Short hover content derived from man page or vendor docs."""

    summary: str
    synopsis: tuple[str, ...] = ()
    snippet: str = ""
    source: str = ""
    examples: str = ""
    return_value: str = ""

    def render_markdown(self, symbol: str) -> str:
        """Render full markdown content (used for signature help documentation)."""
        parts: list[str] = []
        if self.synopsis:
            sig = "\n".join(self.synopsis)
            parts.append(f"```tcl\n{sig}\n```")
        elif symbol:
            parts.append(f"```tcl\n{symbol}\n```")
        if self.summary:
            parts.append(self.summary)
        if self.snippet:
            parts.append(self.snippet)
        if self.return_value:
            parts.append(f"**Returns**: {self.return_value}")
        if self.examples:
            parts.append(f"**Example**:\n```tcl\n{self.examples}\n```")
        if self.source:
            parts.append(f"_Source: {self.source}_")
        return "\n\n".join(parts)

    def render_hover_lean(self, symbol: str) -> str:
        """Render lean hover: synopsis + summary only.

        Rich details (snippet, examples, return_value) are left for
        signature help, keeping the hover tooltip short so it doesn't
        bury diagnostics.
        """
        parts: list[str] = []
        if self.synopsis:
            sig = "\n".join(self.synopsis)
            parts.append(f"```tcl\n{sig}\n```")
        elif symbol:
            parts.append(f"```tcl\n{symbol}\n```")
        if self.summary:
            parts.append(self.summary)
        if self.source:
            parts.append(f"_Source: {self.source}_")
        return "\n\n".join(parts)


@dataclass(frozen=True, slots=True)
class ArgumentValueSpec:
    """Completion and hover metadata for a positional argument value."""

    value: str
    detail: str = ""
    hover: HoverSnippet | None = None


@dataclass(frozen=True, slots=True)
class OptionSpec:
    """Metadata for a switch-like option (for completion and hover)."""

    name: str
    takes_value: bool = False
    value_hint: str = ""
    detail: str = ""
    hover: HoverSnippet | None = None


class FormKind(enum.Enum):
    """Classification of a command invocation form."""

    DEFAULT = "default"
    GETTER = "getter"
    SETTER = "setter"


@dataclass(frozen=True, slots=True)
class FormSpec:
    """A concrete invocation form of a command.

    Each form describes one way to invoke the command, with its own
    synopsis, arity, purity, and side-effect hints.  For commands with
    getter/setter semantics (e.g. ``HTTP::uri``), the getter and setter
    are separate forms with distinct arities and effect classifications.

    Attributes:
        kind: Classifies this form (DEFAULT, GETTER, SETTER).
        synopsis: Human-readable invocation signature.
        arity: Per-form arity constraint.  ``None`` means inherit from
            the parent ``ValidationSpec``.
        pure: Whether this form is side-effect-free.
        mutator: Whether this form mutates state.
        side_effect_hints: Structured side-effect metadata for this form.
        arg_values: Maps argument index (0-based after command name) to
            completable values.  Index 0 typically holds subcommand names.
        subcommand_arg_values: Maps ``(subcommand, sub_arg_index)`` to
            completable values *within* a subcommand.  ``sub_arg_index``
            is 0-based after the subcommand word itself.
            Example: ``("is", 0)`` in ``string`` holds character-class
            names for ``string is <class>``.
    """

    kind: FormKind
    synopsis: str
    arity: Arity | None = None
    options: tuple[OptionSpec, ...] = ()
    pure: bool = False
    mutator: bool = False
    side_effect_hints: tuple[SideEffect, ...] = ()
    arg_values: dict[int, tuple[ArgumentValueSpec, ...]] = field(default_factory=dict)
    subcommand_arg_values: dict[tuple[str, int], tuple[ArgumentValueSpec, ...]] = field(
        default_factory=dict,
    )

    def option_names(self) -> tuple[str, ...]:
        return tuple(opt.name for opt in self.options)


@dataclass(frozen=True, slots=True)
class ValidationSpec:
    """Validation metadata used for arity and syntax checks."""

    arity: Arity = field(default_factory=Arity)


@dataclass(slots=True)
class SubCommand:
    """Complete metadata for a single subcommand.

    All metadata for one subcommand lives in one place: arity, completion,
    hover, arg roles, types, effect classification, options, and hooks.

    Not frozen: carries ``Callable`` hooks (``handler``, ``codegen``,
    ``lowering``, etc.) which may be set after initial construction by
    subsystem init code.  Remains effectively immutable by convention —
    no code mutates specs after registration.
    """

    name: str
    arity: Arity

    # Completion & hover (replaces FormSpec.arg_values[0] entries).
    detail: str = ""
    synopsis: str = ""
    hover: HoverSnippet | None = None

    # Arg roles (replaces role_hints() -> SubcommandSig).
    arg_roles: dict[int, ArgRole] = field(default_factory=dict)
    arg_role_resolver: ArgRoleResolver | None = None

    # Type info (replaces type_hints() -> SubcommandTypeHint).
    return_type: TclType | None = None
    arg_types: dict[int, ArgTypeHint] = field(default_factory=dict)
    arg_type_resolver: ArgTypeResolver | None = None

    # Effect classification (replaces pure_subcommands / mutator_subcommands).
    pure: bool = False
    mutator: bool = False
    destructive: bool = False  # Irreversible operations (file delete, etc.)

    # Deprecation (subcommand-level).
    deprecated_replacement: type[CommandDef] | str | None = None
    deprecation_fixer: DeprecationFixer | None = None

    # Per-subcommand options (replaces command-wide FormSpec.options).
    options: tuple[OptionSpec, ...] = ()

    # Per-subcommand completable arg values (replaces FormSpec.subcommand_arg_values).
    # Maps arg index (0-based after subcommand word) to completable values.
    arg_values: dict[int, tuple[ArgumentValueSpec, ...]] = field(default_factory=dict)

    # Dialect membership — which dialects this subcommand is available in.
    # ``None`` means "inherit from parent CommandSpec.dialects" (the common case).
    dialects: frozenset[str] | None = None

    # Execution and compilation hooks — all optional.
    handler: SubcommandHandler | None = None
    codegen: CodegenHook | None = None
    lowering: LoweringHook | None = None
    codegens: dict[str, WasmEmitHook] = field(default_factory=dict)

    # Taint transform — colour bits added to tainted output by this subcommand.
    taint_transform: TaintColour | None = None
    taint_double_encode_colour: TaintColour | None = None

    # Taint output sink — for subcommands that are XSS/header-injection sinks.
    taint_output_sink: str | None = None  # e.g. "IRULE3002"

    # Format string type — overrides parent CommandSpec.format_string_type.
    format_string_type: FormatType | None = None

    # Pattern type — overrides parent CommandSpec.pattern_type.
    pattern_type: PatternType | None = None

    # XC translation — what XC operation this subcommand maps to.
    xc_operation: str | None = None

    # Validation hook — command-specific diagnostics beyond arity.
    validation_hook: ValidationHook | None = None

    # Static side-effect hints for structured effect analysis.
    # When set, these override the heuristic classification in
    # ``core.compiler.side_effects.classify_side_effects()``.
    side_effect_hints: tuple[SideEffect, ...] | None = None

    # Whether this subcommand is destructive (e.g. file delete, file rename).
    destructive: bool = False

    # Credential arg index for security checks.
    credential_arg: int | None = None

    # Header names whose values are secrets (e.g. "authorization").
    sensitive_headers: frozenset[str] | None = None

    # Compile-time constant folding callback.  When set and all arguments
    # resolve to constants, SCCP calls this to compute the return value.
    const_fold: ConstFoldFunc | None = None

    # Per-subcommand invocation forms (for arity-dependent getter/setter).
    # When populated, ``resolve_form()`` matches actual arguments to a form.
    # When empty (the common case), the subcommand's top-level ``pure``/
    # ``mutator`` flags apply directly.
    forms: tuple[FormSpec, ...] = ()

    # Inferred storage type for the target variable (DICT, LIST, ARRAY).
    inferred_storage_type: StorageType | None = None

    # Whether this subcommand's CFG header carries list-expression args
    # that are evaluated once before the loop body.
    loop_list_header: bool = False

    # Whether this subcommand creates a scope alias (upvar-like binding).
    creates_scope_alias: bool = False

    # Whether this subcommand safely initialises an uninitialised variable.
    # Semantics match CommandSpec.safe_on_uninit: ``None`` = not safe,
    # empty frozenset = safe in all dialects, non-empty = safe in listed dialects.
    safe_on_uninit: frozenset[str] | None = None

    # Whether this subcommand returns a filesystem path.
    returns_path: bool = False

    # Whether this subcommand performs unescaping / decoding.
    is_unescape_command: bool = False

    def resolve_form(self, args: tuple[str, ...] | list[str]) -> FormSpec | None:
        """Given actual arguments (after the subcommand word), return the matching form.

        Returns ``None`` when this subcommand has no forms (the common case).
        """
        if not self.forms:
            return None
        if len(self.forms) == 1:
            return self.forms[0]
        n = len(args)
        for form in self.forms:
            if form.arity is not None and form.arity.accepts(n):
                return form
        # Fall back to first DEFAULT form.
        for form in self.forms:
            if form.kind is FormKind.DEFAULT:
                return form
        return self.forms[0] if self.forms else None

    @property
    def deprecated_replacement_name(self) -> str | None:
        """Return the human-readable name of the replacement command/subcommand."""
        if self.deprecated_replacement is None:
            return None
        if isinstance(self.deprecated_replacement, str):
            return self.deprecated_replacement
        return self.deprecated_replacement.name

    def supports_dialect(
        self,
        dialect: str | None,
        parent_dialects: frozenset[str] | None = None,
    ) -> bool:
        """Check if this subcommand is available in *dialect*.

        If this subcommand has its own ``dialects`` set, use that.
        Otherwise, inherit from the parent command's dialects.
        """
        if dialect is None:
            return True
        if self.dialects is not None:
            return dialect in self.dialects
        # Inherit from parent.
        if parent_dialects is None:
            return True
        return dialect in parent_dialects


@dataclass(slots=True)
class CommandSpec:
    """Unified command metadata for completion/hover/registry lookup."""

    name: str
    dialects: frozenset[str] | None = None
    hover: HoverSnippet | None = None
    forms: tuple[FormSpec, ...] = ()
    validation: ValidationSpec | None = None
    # iRules event validity.
    excluded_events: tuple[str, ...] = ()

    # Layer-based event requirements for IRULE1001 validation.
    event_requires: EventRequires | None = None

    # Behavioral traits for compiler, analyser, formatter, and optimizer.
    creates_dynamic_barrier: bool = False
    has_loop_body: bool = False
    never_inline_body: bool = False
    # When True, W304 fires even for non-dynamic positional values
    # (not just variables/command substitutions).  Only needed for commands
    # where option injection is especially dangerous (e.g. regexp).
    warn_without_terminator: bool = False

    # Analysis check dispatch traits.  These drive the targeted-check
    # routing in ``core/analysis/checks/_orchestrator.py`` so that
    # command-specific checks are declared on the spec, not in consumer
    # frozensets.
    evaluates_code: bool = False  # eval, uplevel
    performs_substitution: bool = False  # subst
    opens_channel: bool = False  # open
    sources_file: bool = False  # source
    has_switch_body: bool = False  # switch
    has_string_list_confusion_risk: bool = False  # append
    configures_channel: bool = False  # fconfigure, chan (configure)
    has_interp_eval: bool = False  # interp
    has_destructive_ops: bool = False  # file, namespace, chan
    is_irules_event_handler: bool = False  # when
    is_unnormalized_http_getter: bool = False  # HTTP::path, HTTP::uri, HTTP::query

    # Purity and CSE traits for compiler/gvn.py.
    pure: bool = False
    cse_candidate: bool = False

    # Compile-time constant folding callback (for commands without subcommands).
    const_fold: ConstFoldFunc | None = None

    # Diagram extraction trait for diagram/extract.py.
    diagram_action: bool = False

    # XC translatability for xc/mapping.py.
    # None = follow default rules, False = never translatable,
    # True = translatable despite namespace prefix.
    xc_translatable: bool | None = None

    # Package-conditional activation.  When set, this command only appears
    # in completions when the named package has been required via
    # ``package require``.  Hover and validation are unconditional.
    required_package: str | None = None

    # Tcllib package that provides this command, if any.
    # Used for per-document activation via ``package require``.
    tcllib_package: str | None = None

    # Whether W120 should fire when this package-gated command is used
    # without a corresponding ``package require``.  Default True; set
    # False for Tk commands because ``wish`` auto-loads Tk.
    warn_missing_import: bool = True

    # Unified subcommand registry.  Each SubCommand carries its own
    # arity, arg_roles, return_type, pure/mutator flags, arg_values, etc.
    subcommands: dict[str, SubCommand] = field(default_factory=dict)
    allow_unknown_subcommands: bool = False

    # Execution and compilation hooks (for commands WITHOUT subcommands).
    handler: CommandHandler | None = None
    codegen: CodegenHook | None = None
    lowering: LoweringHook | None = None
    codegens: dict[str, WasmEmitHook] = field(default_factory=dict)

    # WASM runtime dispatch metadata — when set, the WASM codegen
    # auto-registers a hook that emits a call to the named runtime
    # import.  Replaces the ``_CMD_RUNTIME`` shadow table in
    # ``core/compiler/codegen/wasm/_imports.py``.
    wasm_runtime_import: WasmRuntimeImport | None = None

    # Static arg roles and type info for commands WITHOUT subcommands.
    # Replaces role_hints() -> CommandSig and type_hints() -> CommandTypeHint.
    arg_roles: dict[int, ArgRole] = field(default_factory=dict)
    return_type: TclType | None = None
    arg_types: dict[int, ArgTypeHint] = field(default_factory=dict)

    # Arg-type resolution for variable-layout commands (lsort/foreach/concat).
    arg_type_resolver: ArgTypeResolver | None = None

    # Arg-role resolution for variable-layout commands (if/try/switch/foreach).
    arg_role_resolver: ArgRoleResolver | None = None

    # Keyword+scaffold completions inside variable-layout commands.
    keyword_completions: KeywordCompletionProvider | None = None

    # New flags (replacing hardcoded lists in consumer modules).
    deprecated_replacement: type[CommandDef] | str | None = None
    unsafe: bool = False
    password_option_command: bool = False

    # Credential detection — option flags that carry secrets (e.g. -password).
    credential_options: frozenset[str] | None = None

    # Sensitive header names for HTTP header/cookie commands.
    sensitive_headers: frozenset[str] | None = None

    # Taint sink classification.
    taint_sink: bool = False
    taint_output_sink: str | None = None
    taint_output_sink_subcommands: frozenset[str] | None = None
    taint_log_sink: str | None = None
    taint_network_sink_args: tuple[int, ...] | None = None
    taint_interp_eval_subcommands: frozenset[str] | None = None
    # Taint transform — colour bits added to tainted output by this command.
    taint_transform: TaintColour | None = None
    taint_double_encode_colour: TaintColour | None = None
    # Colour that suppresses T100 for this taint sink (e.g. SHELL_ATOM for exec).
    taint_sink_safe_colour: TaintColour | None = None

    # Deprecation fixer — code action for deprecated usage.
    deprecation_fixer: DeprecationFixer | None = None

    # Validation hook — command-specific diagnostics beyond arity.
    validation_hook: ValidationHook | None = None

    # Bytecode control flow.
    needs_start_cmd: bool = False

    # Static side-effect hints for structured effect analysis.
    # When set, these override the heuristic classification in
    # ``core.compiler.side_effects.classify_side_effects()``.
    side_effect_hints: tuple[SideEffect, ...] | None = None

    # Variable assignment semantics.
    assigns_variable_at: int | None = None

    # Whether the command safely initialises an uninitialised variable
    # (e.g. ``lappend`` creates an empty list, ``append`` an empty string).
    # ``None`` means *not* safe.  A frozenset of dialect strings means safe
    # only in those dialects (e.g. ``incr`` is safe in 8.5+ but not 8.4).
    # An empty frozenset is treated as "safe in all dialects".
    safe_on_uninit: frozenset[str] | None = None

    # Procedure definition semantics.
    defines_procedure: bool = False

    # Format string metadata.
    format_string_type: FormatType | None = None

    # Pattern metadata.
    pattern_type: PatternType | None = None

    # Control flow classification.
    is_control_flow: bool = False

    # Inferred storage type for the target variable (DICT, LIST, ARRAY).
    inferred_storage_type: StorageType | None = None

    # Whether this command's CFG header carries list-expression args
    # that are evaluated once before the loop body (foreach, lmap).
    loop_list_header: bool = False

    # Whether this command creates a scope alias (upvar-like binding).
    creates_scope_alias: bool = False

    # Whether this command returns a filesystem path (pwd, file join, etc.).
    returns_path: bool = False

    # Whether this command performs unescaping / decoding (subst, URI::decode).
    is_unescape_command: bool = False

    # Whether this command is pure evaluation (expr — side-effect-free
    # when braced, which is the recommended usage).
    pure_evaluation: bool = False

    # Whether this command destroys/removes a variable (unset).
    destroys_variable: bool = False

    # Whether this command is a language keyword for semantic token
    # classification (control-flow, definition, TclOO framework words).
    is_language_keyword: bool = False

    # Whether this command reads the target variable before writing it
    # (incr, append, lappend — read-modify-write semantics).
    reads_variable_before_write: bool = False

    # Whether this command's first expression arg is in boolean context
    # (if, while, for — for expression optimisation).
    has_boolean_condition: bool = False

    # Whether this command produces a canonical Tcl list representation
    # (list, concat — for taint colour propagation).
    produces_canonical_list: bool = False

    # Whether this command is a side-switching command (iRules
    # clientside/serverside — for connection-side analysis).
    is_side_switch: bool = False

    # Whether this command must appear at top level in iRules
    # (proc, when, timing, priority).
    irules_top_level_only: bool = False

    # Whether this command is a TclOO metaclass (oo::class, oo::abstract,
    # oo::configurable, oo::singleton).
    is_oo_metaclass: bool = False

    # Whether this command unconditionally terminates the current block
    # (error, return, throw, exit — control never passes to the next statement).
    terminates_block: bool = False

    def supports_dialect(self, dialect: str | None) -> bool:
        if dialect is None:
            return True
        if self.dialects is None:
            return True
        return dialect in self.dialects

    def supports_packages(self, active_packages: frozenset[str] | None) -> bool:
        """Check if this command is available given the active packages.

        When *active_packages* is ``None`` (the default), package-gated
        commands are visible — this keeps backwards compatibility for
        call-sites that don't track packages.
        """
        if self.required_package is None:
            return True
        if active_packages is None:
            return True
        return self.required_package in active_packages

    def resolve_form(self, args: tuple[str, ...] | list[str]) -> FormSpec | None:
        """Given actual arguments, return the matching form.

        Resolution priority:

        1. If no forms, return ``None``.
        2. If only one form, return it.
        3. Match by per-form arity: return the first form whose arity
           accepts ``len(args)``.
        4. Fall back to the first ``DEFAULT`` form.
        """
        if not self.forms:
            return None
        if len(self.forms) == 1:
            return self.forms[0]
        n = len(args)
        for form in self.forms:
            if form.arity is not None and form.arity.accepts(n):
                return form
        # Fall back to first DEFAULT form.
        for form in self.forms:
            if form.kind is FormKind.DEFAULT:
                return form
        return self.forms[0] if self.forms else None

    @property
    def deprecated_replacement_name(self) -> str | None:
        """Return the human-readable name of the replacement command."""
        if self.deprecated_replacement is None:
            return None
        if isinstance(self.deprecated_replacement, str):
            return self.deprecated_replacement
        return self.deprecated_replacement.name

    def switch_names(self) -> tuple[str, ...]:
        names: list[str] = []
        seen: set[str] = set()
        for form in self.forms:
            for name in form.option_names():
                if name not in seen:
                    seen.add(name)
                    names.append(name)
        return tuple(names)

    @property
    def supports_normalized_flag(self) -> bool:
        """Whether this command accepts ``-normalized`` (derived from options)."""
        return self.option("-normalized") is not None

    def option(self, option_name: str) -> OptionSpec | None:
        for form in self.forms:
            for option in form.options:
                if option.name == option_name:
                    return option
        return None

    def argument_values(self, arg_index: int) -> tuple[ArgumentValueSpec, ...]:
        values: list[ArgumentValueSpec] = []
        seen: set[str] = set()
        for form in self.forms:
            for value_spec in form.arg_values.get(arg_index, ()):  # pragma: no branch
                if value_spec.value not in seen:
                    seen.add(value_spec.value)
                    values.append(value_spec)
        return tuple(values)

    def argument_value(self, arg_index: int, value: str) -> ArgumentValueSpec | None:
        for value_spec in self.argument_values(arg_index):
            if value_spec.value == value:
                return value_spec
        return None

    def subcommand_argument_values(
        self,
        subcommand: str,
        arg_index: int,
    ) -> tuple[ArgumentValueSpec, ...]:
        """Return completable values for a subcommand argument position."""
        values: list[ArgumentValueSpec] = []
        seen: set[str] = set()
        # Prefer SubCommand.arg_values (new unified registry).
        sub = self.subcommands.get(subcommand)
        if sub is not None:
            for value_spec in sub.arg_values.get(arg_index, ()):
                if value_spec.value not in seen:
                    seen.add(value_spec.value)
                    values.append(value_spec)
        # Fall back to legacy FormSpec.subcommand_arg_values.
        key = (subcommand, arg_index)
        for form in self.forms:
            for value_spec in form.subcommand_arg_values.get(key, ()):
                if value_spec.value not in seen:
                    seen.add(value_spec.value)
                    values.append(value_spec)
        return tuple(values)

    def subcommand_argument_value(
        self,
        subcommand: str,
        arg_index: int,
        value: str,
    ) -> ArgumentValueSpec | None:
        """Look up a specific subcommand argument value."""
        for value_spec in self.subcommand_argument_values(subcommand, arg_index):
            if value_spec.value == value:
                return value_spec
        return None

    # Derived subcommand helpers

    @property
    def pure_subcommand_names(self) -> frozenset[str]:
        """Subcommand names that are side-effect-free (derived from subcommands dict)."""
        return frozenset(n for n, s in self.subcommands.items() if s.pure)

    @property
    def mutator_subcommand_names(self) -> frozenset[str]:
        """Subcommand names that mutate state (derived from subcommands dict)."""
        return frozenset(n for n, s in self.subcommands.items() if s.mutator)

    @property
    def path_returning_subcommand_names(self) -> frozenset[str]:
        """Subcommand names that return filesystem paths."""
        return frozenset(n for n, s in self.subcommands.items() if s.returns_path)

    @property
    def unescape_subcommand_names(self) -> frozenset[str]:
        """Subcommand names that perform unescaping / decoding."""
        return frozenset(n for n, s in self.subcommands.items() if s.is_unescape_command)

    def subcommand_completions(self) -> tuple[ArgumentValueSpec, ...]:
        """Derive subcommand completion items from the subcommands dict."""
        return tuple(
            ArgumentValueSpec(value=s.name, detail=s.detail, hover=s.hover)
            for s in self.subcommands.values()
        )

    def switches_for_subcommand(self, sub: str) -> tuple[str, ...]:
        """Return option names for a specific subcommand."""
        s = self.subcommands.get(sub)
        if s is None:
            return ()
        return tuple(opt.name for opt in s.options)

    def subcommands_for_dialect(self, dialect: str | None) -> dict[str, SubCommand]:
        """Return subcommands available in *dialect*."""
        return {
            name: sub
            for name, sub in self.subcommands.items()
            if sub.supports_dialect(dialect, self.dialects)
        }
