"""Lower Tcl source to a structured analysis IR.

"Lowering" translates a flat token stream produced by ``TclLexer``
into the tree of IR nodes defined in ``ir.py``.  Each Tcl command is
pattern-matched by name (``set``, ``if``, ``for``, ``proc``, etc.)
and converted to a typed IR statement that preserves just enough
semantics for later analysis, discarding syntactic noise.

Commands that are not specifically handled fall through to ``IRCall``
(a generic call node) or ``IRBarrier`` (a node that tells downstream
passes "stop — this command may have arbitrary side effects").
"""

# canonicalisation: audited #246

from __future__ import annotations

import contextlib
from dataclasses import dataclass, field
from typing import cast

from shared.alias import (
    detect_interp_alias,
)
from shared.alias import (
    expr_alias_names as _expr_alias_names,
)
from shared.alias import (
    resolve_alias as _resolve_alias_shared,
)
from shared.alias import (
    to_canonical_command as _to_canonical_command,
)
from shared.dialect import active_dialect as _active_dialect
from shared.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from shared.naming import (
    normalise_var_name as _normalise_var_name,
)
from shared.ranges import range_from_token

from ..analysis.semantic_model import Range
from ..commands.registry import REGISTRY
from ..commands.registry.runtime import ArgRole, arg_indices_for_role
from ..parsing.command_segmenter import SegmentedCommand, TopLevelChunk, segment_commands
from ..parsing.command_shapes import extract_single_expr_argument
from ..parsing.expr_parser import parse_expr as _std_parse_expr
from ..parsing.lexer import TclLexer, TclParseError
from ..parsing.tokens import Token, TokenType
from .ir import (
    CommandTokens,
    CommandTrace,
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRIncr,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
    IRSwitch,
    IRSwitchArm,
    IRTry,
    IRTryHandler,
    IRUpFrame,
    IRWhile,
)
from .lowering_hooks import register_all as _register_lowering_hooks
from .token_helpers import word_piece as _word_piece

_DYNAMIC_BARRIER_COMMANDS = REGISTRY.dynamic_barrier_commands()

# Register per-command lowering hooks on the REGISTRY.
_register_lowering_hooks()


def _parse_expr(source: str):  # noqa: F811
    return _std_parse_expr(source, dialect=_active_dialect())


# Literal condition words Tcl's ``expr`` treats as unambiguous booleans
# (see ``Tcl_GetBoolean`` in ``tmp/tcl9.0.3/generic/tclGet.c``).  We only
# match condition *expressions* that are exactly one of these tokens —
# anything else (``1 + 0``, a variable reference, a command substitution)
# needs full constant folding that lowering deliberately doesn't do.
_STATIC_FALSE_LITERALS: frozenset[str] = frozenset({"0", "false", "no", "off"})
_STATIC_TRUE_LITERALS: frozenset[str] = frozenset({"1", "true", "yes", "on"})


def _static_bool(expr_text: str) -> bool | None:
    """Return ``True`` / ``False`` if *expr_text* is a literal Tcl
    boolean.  Returns ``None`` when the condition is not a simple
    literal — more elaborate folding stays out of scope for the
    reachability gate in :meth:`Lowerer._lower_if`.

    Tolerates surrounding whitespace (``if { 0 } { ... }``) and is
    case-insensitive (matches ``Tcl_GetBoolean``).  Expressions with
    a prefix / suffix beyond the literal token (``0 + 0``,
    ``!true``, variable references, command substitutions) return
    ``None`` because the simple string match can't prove them
    constant without duplicating the full expr evaluator.
    """
    if not expr_text:
        return None
    stripped = expr_text.strip().lower()
    if stripped in _STATIC_FALSE_LITERALS:
        return False
    if stripped in _STATIC_TRUE_LITERALS:
        return True
    return None


def _join_namespace(parent_ns: str, ns_name: str) -> str:
    if ns_name.startswith("::"):
        return _normalise_qualified_name(ns_name)
    if parent_ns == "::":
        return _normalise_qualified_name(f"::{ns_name}")
    return _normalise_qualified_name(f"{parent_ns}::{ns_name}")


def _qualify_proc_name(namespace: str, proc_name: str) -> str:
    if proc_name.startswith("::"):
        return _normalise_qualified_name(proc_name)
    if namespace == "::":
        return _normalise_qualified_name(f"::{proc_name}")
    return _normalise_qualified_name(f"{namespace}::{proc_name}")


def _parse_params_with_defaults(param_str: str) -> tuple[tuple[str, str | None], ...]:
    """Parse a Tcl proc ``param_str`` into ``(name, default_or_None)`` pairs.

    ``{name default}`` → ``(name, default)``; bare ``name`` →
    ``(name, None)``.  The default string is returned verbatim
    (braces already stripped, interior whitespace preserved).
    Used by the WASM codegen to pad missing call-site args with the
    declared default rather than a boxed zero.
    """
    params: list[tuple[str, str | None]] = []
    i = 0
    text = param_str.strip()
    while i < len(text):
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break
        if text[i] == "{":
            level = 1
            i += 1
            start = i
            while i < len(text) and level > 0:
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            inner = text[start : i - 1]
            inner_stripped = inner.strip()
            if not inner_stripped:
                continue
            # Split into name + optional default.  The default may
            # contain whitespace (``{x {a b c}}``) so we take
            # everything after the first whitespace as the raw
            # default text, stripping outer whitespace only.
            split_at = 0
            while split_at < len(inner) and inner[split_at] not in " \t\n\r":
                split_at += 1
            name = inner[:split_at]
            default: str | None
            if split_at < len(inner):
                raw_default = inner[split_at:].strip()
                # Strip one level of Tcl grouping so ``""`` becomes the
                # empty string and ``{a b c}`` becomes ``a b c``.  The
                # caller gets back the plain value ready to be emitted
                # as an obj_literal.
                if len(raw_default) >= 2 and raw_default[0] == '"' and raw_default[-1] == '"':
                    default = raw_default[1:-1]
                elif len(raw_default) >= 2 and raw_default[0] == "{" and raw_default[-1] == "}":
                    default = raw_default[1:-1]
                else:
                    default = raw_default
            else:
                default = None
            if name:
                params.append((name, default))
            continue
        start = i
        while i < len(text) and text[i] not in " \t\n\r":
            i += 1
        word = text[start:i]
        if word:
            params.append((word, None))
    return tuple(params)


def _parse_param_names(param_str: str) -> tuple[str, ...]:
    params: list[str] = []
    i = 0
    text = param_str.strip()
    while i < len(text):
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break

        if text[i] == "{":
            level = 1
            i += 1
            start = i
            while i < len(text) and level > 0:
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            inner = text[start : i - 1].strip()
            if inner:
                params.append(inner.split(None, 1)[0])
            continue

        start = i
        while i < len(text) and text[i] not in " \t\n\r":
            i += 1
        word = text[start:i]
        if word:
            params.append(word)

    return tuple(params)


def _expr_arg_from_expr_command(cmd_text: str) -> str | None:
    """Return expr argument if command text is exactly: expr <one-arg>."""
    return extract_single_expr_argument(cmd_text)


def _switch_body_elements(body_text: str, outer_tok: Token | None) -> tuple[list[str], list[Token]]:
    if outer_tok is not None:
        lexer = TclLexer(
            body_text,
            base_offset=outer_tok.start.offset + 1,
            base_line=outer_tok.start.line,
            base_col=outer_tok.start.character + 1,
        )
    else:
        lexer = TclLexer(body_text)

    elements: list[str] = []
    element_tokens: list[Token] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
            prev_type = tok.type
            continue

        piece = _word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            elements.append(piece)
            element_tokens.append(tok)
        else:
            if elements:
                elements[-1] += piece
            else:
                elements.append(piece)
                element_tokens.append(tok)
        prev_type = tok.type

    return elements, element_tokens


@dataclass(slots=True)
class _Command:
    range: Range
    argv: list[Token]
    texts: list[str]
    single_token_word: list[bool]
    all_tokens: list[Token] = field(default_factory=list)
    expand_word: list[bool] | None = None

    @property
    def name(self) -> str:
        return self.texts[0] if self.texts else ""

    @property
    def args(self) -> list[str]:
        return self.texts[1:]

    @property
    def arg_tokens(self) -> list[Token]:
        return self.argv[1:]

    @property
    def arg_single_token(self) -> list[bool]:
        return self.single_token_word[1:]

    @property
    def cmd_tokens(self) -> CommandTokens:
        """Snapshot of parsed tokens for downstream passes."""
        return CommandTokens(
            argv=tuple(self.argv),
            argv_texts=tuple(self.texts),
            single_token_word=tuple(self.single_token_word),
            all_tokens=tuple(self.all_tokens),
            expand_word=tuple(self.expand_word) if self.expand_word else None,
        )


class _Lowerer:
    def __init__(self) -> None:
        self.module = IRModule()
        self._in_namespace_eval = False
        self._when_counts: dict[str, int] = {}  # event -> occurrence count
        # Command aliases: alias_name -> (target_cmd, prepended_args).
        # Populated from ``interp alias {} name {} target ?arg ...?``.
        self._command_aliases: dict[str, tuple[str, tuple[str, ...]]] = {}
        # ``namespace import`` directives captured at lowering time,
        # as ``(context_namespace, absolute_pattern)`` pairs.  Codegen
        # pattern-matches each against the final ``module.procedures``
        # table to decide which unqualified calls can dispatch
        # directly.  Order preserved because later imports shadow
        # earlier ones in Tcl's resolution model.
        self._namespace_imports: list[tuple[str, str]] = []
        # Captured ``namespace export pattern …`` directives,
        # keyed by the source namespace that ran the ``export``.
        # Codegen uses these to filter
        # ``namespace_imports``-derived compile-time dispatch so
        # only commands the source ns explicitly exported are
        # eligible for the shortcut (matches C Tcl's
        # ``Tcl_Import`` rules).  Shares the
        # "any syntactic occurrence counts" caveat with
        # ``_namespace_imports`` — dead code still registers, but
        # erring toward "more exports than reality" only loosens
        # the import filter, so the worst case is the pre-fix
        # behaviour (runtime would correctly reject the import).
        self._namespace_exports: list[tuple[str, str]] = []
        # ``trace add execution`` / ``trace remove execution`` directives
        # captured at lowering time.  See ``IRModule.command_traces``.
        # Both adds and removes are recorded in source order so the net
        # active set can be derived later (also reused by future
        # interprocedural passes that need to know *where* a trace is
        # installed).
        self._command_traces: list[CommandTrace] = []
        # Per-script const-map stack: each scope tracks proc-local
        # variables assigned a braced-literal value.  Populated at
        # ``set var {literal}`` sites and consulted by the
        # ``uplevel`` / ``eval`` barrier-relaxation gates so a later
        # ``uplevel 1 $var`` can fold in the literal body.  Cleared on
        # any re-assignment to the tracked variable, on any
        # :class:`IRBarrier`, or on entry to structured IR whose body
        # could write unknown variables.  Only populated when the
        # variable name is a plain proc-local bareword — parameters,
        # globals, namespace-qualified names, and array refs are
        # excluded so upvar-aliased names never pollute the map.
        #
        # Lookups and updates are gated on ``self._proc_depth > 0``
        # so that top-level and ``namespace eval`` scopes stay out of
        # the map.  Names written at those scopes are globals /
        # namespace vars which other code can read and mutate, so
        # const-propagating them would be unsound.
        self._const_map_stack: list[dict[str, str]] = []
        # Depth of ``proc`` bodies we are currently lowering.  A
        # positive depth enables the const-map.  Incremented at the
        # entry of a proc body lowering, decremented on exit.
        self._proc_depth = 0
        # Depth of syntactically-dead branches currently being lowered.
        # ``if {0} { … }`` / ``if {1} { … } else { … }`` / ``while {0}
        # { … }`` bump this counter around the body lowering so any
        # ``namespace import`` / ``namespace export`` directive found
        # inside dead code is NOT collected for the codegen's
        # compile-time import shortcut.  Non-zero depth suppresses
        # collection (but does not otherwise alter lowering — the IR
        # for dead code is still produced so consumers that walk the
        # tree by syntactic offset see the original structure).  See
        # https://github.com/bitwisecook/tcl-lsp/issues/189 for the
        # bug this gates.
        self._dead_code_depth: int = 0
        # Namespace context of the command currently being lowered.
        # Tracked separately from the per-method ``namespace`` parameter
        # so that lowering hooks (which receive only ``(lowerer, cmd)``
        # and can't see the dispatcher's parameter) can still resolve
        # canonical command names against the correct enclosing scope
        # via :meth:`canonicalise_command`.  Set at the top of
        # :meth:`_lower_command` and restored on exit.  Always defaults
        # to global (``::``) outside an active lowering call.  See
        # issue #246 for the canonicalisation contract.
        self._current_namespace: str = "::"
        # ``rename old new`` directives captured at lowering time.
        # Records the *static* renames whose ``old`` and ``new`` arguments
        # are literal token text so per-call-site canonicalisation can
        # follow the chain.  Each entry maps ``new_name`` (the spelling
        # the user calls) to ``old_name`` (the original command the
        # rename redirects to).  ``rename foo {}`` (deletion) records
        # ``new_name → ""`` so canonicalisation can flag a deleted
        # command if it appears later — for now we only consult this
        # for the redirect case.  Renames with non-literal target
        # (``rename $foo bar``) do not register: the canonical form is
        # the user's source spelling for those, since we cannot follow
        # them statically.  See issue #246.
        self._command_renames: dict[str, str] = {}

    @contextlib.contextmanager
    def _dead_code(self):
        """Context manager that marks the enclosed lowering calls as
        running inside statically-dead code.  Used by
        :meth:`_lower_if` (and analogous structured commands) when a
        clause's condition folds to a static ``false`` — or when an
        earlier clause's condition folded to a static ``true``,
        making the remaining clauses unreachable.  See
        ``_dead_code_depth`` for the collection gate this drives.
        """
        self._dead_code_depth += 1
        try:
            yield
        finally:
            self._dead_code_depth -= 1

    def expr_alias_names(self) -> frozenset[str]:
        """Return names that are aliases for ``expr`` (no prepended args)."""
        return _expr_alias_names(self._command_aliases)

    def _resolve_alias(self, cmd_name: str, namespace: str) -> tuple[str, tuple[str, ...]] | None:
        """Look up a command alias, namespace-aware."""
        return _resolve_alias_shared(cmd_name, self._command_aliases, namespace=namespace)

    def _canonicalise_command(self, cmd_name: str, namespace: str) -> str:
        """Resolve a written command name to its canonical fully-qualified form.

        Single source of truth for the lowering-time canonicalisation
        stamped onto ``IRCall.canonical_command`` /
        ``IRBarrier.canonical_command``.  Resolves four layers of
        per-call-site state captured during lowering, in order:

        1. **``rename old new``** — if *cmd_name* matches a recorded
           ``new_name``, follow the chain back to the original
           ``old_name``.  ``rename`` chains are walked with a
           cycle/self-loop guard so a future ``rename a a`` is safe.
        2. **``interp alias``** — :func:`shared.alias.to_canonical_command`
           consumes the captured alias map and follows aliases to their
           terminal target.
        3. **``namespace import``** — if the resolved name is still bare
           and a captured import pattern exposes it under a source
           namespace (``::other::*`` or an exact ``::other::name``),
           rewrite to the imported source name.
        4. **Qualified normalisation** — bare names become global
           (``::cmd``) via :func:`shared.naming.normalise_qualified_name`.

        See issue #246.
        """
        # Iterate until both the rename chain AND the alias chain reach
        # a fixed point — interp alias and rename can compose in either
        # order (``rename foo bar`` then ``interp alias {} baz {} bar``
        # vs ``interp alias {} baz {} foo`` then ``rename foo bar``)
        # and we have to follow both layers to land on the underlying
        # builtin.
        seen: set[str] = set()
        current = cmd_name
        while current not in seen:
            seen.add(current)
            renamed = self._command_renames.get(current)
            if renamed and renamed != current:
                current = renamed
                continue
            # Try alias-resolved form (also strips a leading ``::`` for
            # the rename-table lookup so ``rename oldName newName``
            # records register ``newName`` and the canonicaliser still
            # finds them when handed an explicitly-qualified call).
            alias_resolved = _to_canonical_command(
                current, namespace=namespace, aliases=self._command_aliases
            )
            if alias_resolved != current:
                # If the alias-resolved canonical form has a recorded
                # rename when stripped to bare, follow it.
                bare_alias = (
                    alias_resolved[2:]
                    if alias_resolved.startswith("::") and "::" not in alias_resolved[2:]
                    else None
                )
                if bare_alias and bare_alias in self._command_renames:
                    current = self._command_renames[bare_alias]
                    if not current:
                        # Deletion sentinel — stop with the alias-resolved
                        # form so the canonical reflects the last live name.
                        current = alias_resolved
                        break
                    continue
                current = alias_resolved
                continue
            break
        canonical = _to_canonical_command(
            current, namespace=namespace, aliases=self._command_aliases
        )
        # If the alias-resolved name is still bare-prefix (only one
        # ``::`` segment) and matches a captured ``namespace import``
        # pattern, rewrite to the imported source name.  The captured
        # patterns are absolute (``::other::*`` or ``::other::name``),
        # so we just check for an exact-name or glob-tail match against
        # the bare tail of the canonical form.
        #
        # Builtins (``namespace``, ``set``, ``if``, …) are never
        # rewritten — Tcl's ``namespace import`` does shadow builtins
        # at runtime, but static analysis keeps the canonical bound
        # to the global builtin so e.g. ``namespace import ::tcltest::*``
        # does not silently rename every plain ``namespace`` call to
        # ``::tcltest::namespace``.  Imports only resolve names that
        # are not registered as global commands.
        if canonical.startswith("::") and "::" not in canonical[2:]:
            bare = canonical[2:]
            if REGISTRY.get_any(bare) is None:
                for context_ns, pattern in self._namespace_imports:
                    if context_ns != namespace:
                        continue
                    if not pattern.startswith("::"):
                        continue
                    # Exact: ``::other::bare``.
                    if pattern.endswith(f"::{bare}"):
                        return _to_canonical_command(pattern, namespace="::", aliases={})
                    # Glob: ``::other::*`` matches any bare name.
                    if pattern.endswith("::*"):
                        source_ns = pattern[:-3]
                        return _to_canonical_command(
                            f"{source_ns}::{bare}", namespace="::", aliases={}
                        )
        return canonical

    def canonicalise_command(self, cmd_name: str) -> str:
        """Public canonicalisation helper for lowering hooks.

        Hooks receive only ``(lowerer, cmd)`` and have no view of the
        dispatcher's ``namespace`` parameter, so they call this method
        to canonicalise a command name against
        :attr:`_current_namespace` (which :meth:`_lower_command` sets
        before invoking each hook).  The hook then stamps
        ``canonical_command=lowerer.canonicalise_command(cmd.name)``
        on every ``IRCall`` / ``IRBarrier`` it constructs.
        """
        return self._canonicalise_command(cmd_name, self._current_namespace)

    def lower(self, source: str) -> IRModule:
        self.module.top_level = self._lower_script(source, namespace="::")
        self.module.namespace_exports = tuple(self._namespace_exports)
        self.module.namespace_imports = tuple(self._namespace_imports)
        self.module.command_traces = tuple(self._command_traces)
        return self.module

    def lower_commands(
        self,
        commands: list[SegmentedCommand],
        *,
        namespace: str = "::",
    ) -> tuple[tuple[IRStatement, ...], dict[str, IRProcedure]]:
        """Lower pre-segmented commands to IR without calling ``segment_commands``.

        Returns the top-level IR statements and any procedure definitions
        discovered.  Used by the incremental pipeline to lower a single
        chunk without re-segmenting the entire source.
        """
        procs_before = set(self.module.procedures)
        self._const_map_stack.append({})
        try:
            stmts = self._lower_segmented(commands, namespace=namespace)
        finally:
            self._const_map_stack.pop()
        new_procs = {k: v for k, v in self.module.procedures.items() if k not in procs_before}
        return stmts, new_procs

    def _lower_script(
        self,
        source: str,
        *,
        namespace: str,
        body_token: Token | None = None,
    ) -> IRScript:
        commands = segment_commands(source, body_token)
        # Inherit the parent scope's const-map so a child body (e.g.
        # the ``catch`` body in ``set body {literal}; catch {uplevel 1
        # $body}``) can still relax.  The child's copy is independent,
        # so mutations inside the child do not leak back to the parent
        # — the parent's own invalidation on structured-IR entry stays
        # in charge there.
        parent: dict[str, str] | None = self._const_map_stack[-1] if self._const_map_stack else None
        self._const_map_stack.append(dict(parent) if parent else {})
        try:
            return IRScript(statements=self._lower_segmented(commands, namespace=namespace))
        finally:
            self._const_map_stack.pop()

    def _lower_segmented(
        self,
        segments: list[SegmentedCommand],
        *,
        namespace: str,
    ) -> tuple[IRStatement, ...]:
        """Lower a list of ``SegmentedCommand`` objects to IR statements."""
        statements: list[IRStatement] = []
        for seg in segments:
            if seg.is_partial:
                statements.append(
                    IRBarrier(
                        range=seg.range,
                        reason="incomplete command",
                    )
                )
                continue
            # Fix braced-scalar array refs: the segmenter normalises
            # both ${a(1)} and $a(1) to ${a(1)}, but in Tcl ${a(1)} is
            # a scalar while $a(1) is an array.  Mark braced forms with
            # $={name} so codegen emits push + loadStk (scalar) instead
            # of loadArray1 (array).
            fixed_texts = seg.texts
            for i, (tok, single) in enumerate(zip(seg.argv, seg.single_token_word)):
                if (
                    single
                    and tok.type is TokenType.VAR
                    and "(" in tok.text
                    and tok.text.endswith(")")
                    and (tok.end.offset - tok.start.offset) > len(tok.text)
                ):
                    if fixed_texts is seg.texts:
                        fixed_texts = list(seg.texts)
                    fixed_texts[i] = f"$={{{tok.text}}}"
            cmd = _Command(
                range=seg.range,
                argv=seg.argv,
                texts=fixed_texts,
                single_token_word=seg.single_token_word,
                all_tokens=seg.all_tokens,
                expand_word=seg.expand_word,
            )
            stmt = self._lower_command(cmd, namespace=namespace)
            if stmt is not None:
                statements.append(stmt)
            self._update_const_map(cmd, stmt)
        return tuple(statements)

    # ---------------------------------------------------------------
    # Const-map bookkeeping for barrier-relaxation gate
    # ---------------------------------------------------------------
    def _update_const_map(self, cmd: _Command, stmt: IRStatement | None) -> None:
        """Fold *cmd*/*stmt* into the current scope's const-map.

        Populates a ``name → braced-literal`` entry when *cmd* is
        ``set var {literal}`` with a plain-bareword LHS; otherwise
        invalidates entries that *stmt* may have written.
        Gated on ``self._proc_depth > 0`` — globals / namespace vars
        at top-level or inside ``namespace eval`` cannot be safely
        const-propagated because arbitrary other code may read or
        write them between the ``set`` and the consuming barrier.
        """
        if self._proc_depth <= 0 or not self._const_map_stack:
            return
        cur = self._const_map_stack[-1]

        literal = self._set_literal_body(cmd)
        if literal is not None:
            name, value = literal
            cur[name] = value
            return

        self._invalidate_const_map_for(stmt, cur)

    @staticmethod
    def _set_literal_body(cmd: _Command) -> tuple[str, str] | None:
        """Return ``(name, literal)`` when *cmd* is ``set name {literal}``.

        The LHS must be a plain bareword ESC token (no substitutions,
        no array index, no namespace qualifier) so we only ever track
        proc-local scalars.  The RHS must be a single braced-literal
        STR token — decimal-int ESC values are excluded because
        scripts that look like bare integers are rarely useful as
        ``eval``/``uplevel`` bodies and admitting them would widen the
        trust surface without payoff.
        """
        if cmd.name != "set" or len(cmd.args) != 2:
            return None
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if len(arg_tokens) < 2 or len(arg_single) < 2:
            return None
        if not (arg_single[0] and arg_single[1]):
            return None
        name_tok = arg_tokens[0]
        value_tok = arg_tokens[1]
        if value_tok.type is not TokenType.STR:
            return None
        if name_tok.type is not TokenType.ESC:
            return None
        name = cmd.args[0]
        if not name or "$" in name or "[" in name or "(" in name or "::" in name:
            return None
        return (_normalise_var_name(name), cmd.args[1])

    @staticmethod
    def _invalidate_const_map_for(stmt: IRStatement | None, cur: dict[str, str]) -> None:
        """Drop const-map entries that *stmt* may have overwritten.

        Straight-line assignments pop just the named variable;
        structured IR and barriers conservatively clear the whole map
        because their child scopes (or runtime side effects) could
        touch any of the tracked names — including via ``upvar`` /
        ``global`` aliasing we cannot enumerate here.
        """
        if stmt is None:
            return
        if isinstance(stmt, IRAssignConst):
            cur.pop(_normalise_var_name(stmt.name), None)
            return
        if isinstance(stmt, (IRAssignExpr, IRAssignValue, IRIncr)):
            cur.pop(_normalise_var_name(stmt.name), None)
            return
        if isinstance(stmt, IRCall):
            for var in stmt.defs:
                cur.pop(_normalise_var_name(var), None)
            return
        if isinstance(
            stmt,
            (
                IRBarrier,
                IRIf,
                IRFor,
                IRWhile,
                IRForeach,
                IRCatch,
                IRTry,
                IRSwitch,
                IRBlock,
                IRUpFrame,
            ),
        ):
            cur.clear()
            return
        # IRReturn / IRExprEval don't write variables.

    def _const_map_lookup(self, var_tok: Token) -> str | None:
        """Return the tracked literal for a ``$name`` VAR token, or ``None``.

        Rejects array references (``$a(idx)``) and namespace-qualified
        names so we only ever hit the plain proc-local scalars
        populated by :meth:`_set_literal_body`.  Returns ``None`` when
        we are not inside a proc body (top-level / ``namespace eval``
        globals are out of scope for this optimisation).
        """
        if self._proc_depth <= 0 or not self._const_map_stack:
            return None
        name = var_tok.text
        if not name or "(" in name or name.startswith(":"):
            return None
        return self._const_map_stack[-1].get(_normalise_var_name(name))

    @staticmethod
    def _collapse_continuations(text: str) -> str:
        """Collapse ``\\<newline><whitespace>`` sequences to a single space.

        Inside braces, Tcl preserves ``\\<newline>`` literally, but when
        the brace-quoted string is later evaluated as a script (e.g. a
        proc body), the evaluator collapses them.  We perform that
        collapsing here, before lowering/codegen.
        """
        if "\\\n" not in text:
            return text
        out: list[str] = []
        i = 0
        n = len(text)
        while i < n:
            c = text[i]
            if c == "\\" and i + 1 < n and text[i + 1] == "\n":
                i += 2
                while i < n and text[i] in " \t":
                    i += 1
                out.append(" ")
                continue
            out.append(c)
            i += 1
        return "".join(out)

    def _lower_body_arg(self, text: str, tok: Token | None, *, namespace: str) -> IRScript:
        if tok is None:
            return IRScript()
        try:
            return self._lower_script(text, namespace=namespace, body_token=tok)
        except TclParseError:
            # Syntax errors in body scripts are deferred to runtime —
            # the body will be re-compiled when actually evaluated.
            return IRScript()

    def _lower_if(self, cmd: _Command, *, namespace: str) -> IRIf | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        if not args:
            return IRBarrier(
                range=cmd.range,
                reason="malformed if",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        clauses: list[IRIfClause] = []
        else_body: IRScript | None = None
        else_range: Range | None = None

        # Reachability tracking.  When we encounter a clause whose
        # condition folds to a static ``false``, its body is lowered
        # in a dead-code context so syntactic ``namespace import`` /
        # ``namespace export`` directives inside don't register with
        # the module-level tables.  Once a clause's condition folds
        # to static ``true``, every later clause (and the ``else``
        # branch) is also dead — the flag below latches for the rest
        # of the if.
        later_clauses_dead = False

        i = 0
        while i < len(args):
            if args[i] == "elseif":
                i += 1
                continue

            if args[i] == "else":
                if i + 1 >= len(args):
                    return IRBarrier(
                        range=cmd.range,
                        reason="malformed if else clause",
                        command=cmd.name,
                        canonical_command=self._canonicalise_command(cmd.name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                if i + 2 < len(args):
                    return IRBarrier(
                        range=cmd.range,
                        reason='extra words after "else" clause',
                        command=cmd.name,
                        canonical_command=self._canonicalise_command(cmd.name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                body_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                if later_clauses_dead:
                    with self._dead_code():
                        else_body = self._lower_body_arg(args[i + 1], body_tok, namespace=namespace)
                else:
                    else_body = self._lower_body_arg(args[i + 1], body_tok, namespace=namespace)
                else_range = range_from_token(body_tok) if body_tok is not None else cmd.range
                break

            cond_idx = i
            i += 1
            if i < len(args) and args[i] == "then":
                i += 1
            if i >= len(args):
                return IRBarrier(
                    range=cmd.range,
                    reason="malformed if clause",
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            body_idx = i
            body_tok = arg_tokens[body_idx] if body_idx < len(arg_tokens) else None
            cond_tok = arg_tokens[cond_idx] if cond_idx < len(arg_tokens) else cmd.argv[0]
            cond_value = _static_bool(args[cond_idx])
            body_is_dead = later_clauses_dead or cond_value is False
            if body_is_dead:
                with self._dead_code():
                    body = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
            else:
                body = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
            clauses.append(
                IRIfClause(
                    condition=_parse_expr(args[cond_idx]),
                    condition_range=range_from_token(cond_tok),
                    body=body,
                    body_range=range_from_token(body_tok) if body_tok is not None else cmd.range,
                )
            )
            # Static-true condition makes every later branch dead.
            if cond_value is True and not later_clauses_dead:
                later_clauses_dead = True
            i += 1

        if not clauses:
            return IRBarrier(
                range=cmd.range,
                reason="malformed if",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        return IRIf(
            range=cmd.range,
            clauses=tuple(clauses),
            else_body=else_body,
            else_range=else_range,
        )

    def _lower_switch(self, cmd: _Command, *, namespace: str) -> IRSwitch | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if len(args) < 2:
            return IRBarrier(
                range=cmd.range,
                reason="malformed switch",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        i = 0
        mode = "exact"  # default switch mode
        nocase = False
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "--":
                i += 1
                break
            if args[i] in ("-glob", "-regexp"):
                mode = args[i][1:]
            elif args[i] == "-nocase":
                nocase = True
            i += 1

        if i >= len(args):
            return IRBarrier(
                range=cmd.range,
                reason="malformed switch options",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        subject = args[i]
        subject_tok = arg_tokens[i] if i < len(arg_tokens) else cmd.argv[0]
        i += 1
        if i >= len(args):
            return IRBarrier(
                range=cmd.range,
                reason="switch missing arms",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        pairs: list[tuple[str, Token | None, str, Token | None]] = []
        if i == len(args) - 1 and i < len(arg_tokens) and i < len(arg_single) and arg_single[i]:
            body_text = args[i]
            body_tok = arg_tokens[i]
            elements, element_tokens = _switch_body_elements(body_text, body_tok)
            if len(elements) & 1:
                # Odd count → "extra switch pattern with no body" at runtime
                return IRBarrier(
                    range=cmd.range,
                    reason="switch odd pattern count",
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )
            j = 0
            while j + 1 < len(elements):
                pairs.append(
                    (
                        elements[j],
                        element_tokens[j] if j < len(element_tokens) else None,
                        elements[j + 1],
                        element_tokens[j + 1] if j + 1 < len(element_tokens) else None,
                    )
                )
                j += 2
        else:
            remaining_count = len(args) - i
            if remaining_count & 1:
                return IRBarrier(
                    range=cmd.range,
                    reason="switch odd pattern count",
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )
            j = i
            while j + 1 < len(args):
                pairs.append(
                    (
                        args[j],
                        arg_tokens[j] if j < len(arg_tokens) else None,
                        args[j + 1],
                        arg_tokens[j + 1] if j + 1 < len(arg_tokens) else None,
                    )
                )
                j += 2

        arms: list[IRSwitchArm] = []
        default_body: IRScript | None = None
        default_range: Range | None = None

        for pair_idx, (pattern, pattern_tok, body_text, body_tok) in enumerate(pairs):
            pattern_range = range_from_token(pattern_tok) if pattern_tok is not None else cmd.range
            if body_text == "-":
                arms.append(
                    IRSwitchArm(
                        pattern=pattern,
                        pattern_range=pattern_range,
                        body=None,
                        body_range=None,
                        fallthrough=True,
                    )
                )
                continue

            body = self._lower_body_arg(body_text, body_tok, namespace=namespace)
            body_range = range_from_token(body_tok) if body_tok is not None else cmd.range
            # "default" is only special as the last pattern pair
            if pattern == "default" and pair_idx == len(pairs) - 1:
                default_body = body
                default_range = body_range
            else:
                arms.append(
                    IRSwitchArm(
                        pattern=pattern,
                        pattern_range=pattern_range,
                        body=body,
                        body_range=body_range,
                        fallthrough=False,
                    )
                )

        return IRSwitch(
            range=cmd.range,
            subject=subject,
            subject_range=range_from_token(subject_tok),
            arms=tuple(arms),
            default_body=default_body,
            default_range=default_range,
            mode=mode,
            nocase=nocase,
            raw_args=tuple(args),
        )

    def _lower_for(self, cmd: _Command, *, namespace: str) -> IRFor | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if len(args) < 4 or len(arg_tokens) < 4:
            return IRBarrier(
                range=cmd.range,
                reason="malformed for",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        init_tok = arg_tokens[0]
        cond_tok = arg_tokens[1]
        next_tok = arg_tokens[2]
        body_tok = arg_tokens[3]

        if not (arg_single[0] and arg_single[1] and arg_single[2] and arg_single[3]):
            return IRBarrier(
                range=cmd.range,
                reason="for with dynamic arguments",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        init_script = self._lower_body_arg(args[0], init_tok, namespace=namespace)
        next_script = self._lower_body_arg(args[2], next_tok, namespace=namespace)
        body_script = self._lower_body_arg(args[3], body_tok, namespace=namespace)

        return IRFor(
            range=cmd.range,
            init=init_script,
            init_range=range_from_token(init_tok),
            condition=_parse_expr(args[1]),
            condition_range=range_from_token(cond_tok),
            next=next_script,
            next_range=range_from_token(next_tok),
            body=body_script,
            body_range=range_from_token(body_tok),
            raw_args=tuple(args),
        )

    def _lower_while(self, cmd: _Command, *, namespace: str) -> IRWhile | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if len(args) < 2 or len(arg_tokens) < 2:
            return IRBarrier(
                range=cmd.range,
                reason="malformed while",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        cond_tok = arg_tokens[0]
        body_tok = arg_tokens[1]

        if not (arg_single[0] and arg_single[1]):
            return IRBarrier(
                range=cmd.range,
                reason="while with dynamic arguments",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_script = self._lower_body_arg(args[1], body_tok, namespace=namespace)

        return IRWhile(
            range=cmd.range,
            condition=_parse_expr(args[0]),
            condition_range=range_from_token(cond_tok),
            body=body_script,
            body_range=range_from_token(body_tok),
            raw_args=tuple(args),
        )

    def _lower_foreach(
        self, cmd: _Command, *, namespace: str, is_lmap: bool = False
    ) -> IRForeach | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        # foreach varList list ?varList list ...? body — need at least 3 args (odd count)
        if len(args) < 3 or not len(args) & 1:
            return IRBarrier(
                range=cmd.range,
                reason="malformed foreach",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_idx = len(args) - 1
        body_tok = arg_tokens[body_idx] if body_idx < len(arg_tokens) else None
        if body_tok is None or not (body_idx < len(arg_single) and arg_single[body_idx]):
            return IRBarrier(
                range=cmd.range,
                reason="foreach with dynamic body",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        iterators: list[tuple[tuple[str, ...], str]] = []
        for i in range(0, body_idx, 2):
            var_names = _parse_param_names(args[i])
            list_arg = args[i + 1]
            iterators.append((var_names, list_arg))

        body_script = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)

        return IRForeach(
            range=cmd.range,
            iterators=tuple(iterators),
            body=body_script,
            body_range=range_from_token(body_tok),
            is_lmap=is_lmap,
            raw_args=tuple(args),
        )

    def _lower_catch(self, cmd: _Command, *, namespace: str) -> IRCatch | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if not args:
            return IRBarrier(
                range=cmd.range,
                reason="malformed catch",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_tok = arg_tokens[0] if arg_tokens else None
        # Body must be a braced literal (STR token) to compile statically.
        # Variable references ($cmd) and bracket commands ([expr ...]) must
        # fall through to the runtime eval_catch, which calls eval_script on
        # the substituted value.  Without this guard, ``catch $cmd res`` was
        # compiled as "call the proc named by $cmd with 0 args" — wrong.
        if (
            body_tok is None
            or not (arg_single and arg_single[0])
            or body_tok.type is not TokenType.STR
        ):
            return IRBarrier(
                range=cmd.range,
                reason="catch with dynamic body",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_script = self._lower_body_arg(args[0], body_tok, namespace=namespace)
        result_var = _normalise_var_name(args[1]) if len(args) > 1 else None
        options_var = _normalise_var_name(args[2]) if len(args) > 2 else None

        return IRCatch(
            range=cmd.range,
            body=body_script,
            body_range=range_from_token(body_tok),
            result_var=result_var,
            options_var=options_var,
            raw_args=tuple(args),
            tokens=cmd.cmd_tokens,
        )

    def _lower_try(self, cmd: _Command, *, namespace: str) -> IRTry | IRBarrier:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if not args:
            return IRBarrier(
                range=cmd.range,
                reason="malformed try",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_tok = arg_tokens[0] if arg_tokens else None
        # ``try`` body must be a braced literal (TokenType.STR) for
        # static lowering; ``try $script`` / ``try [cmd]`` carry a
        # VARSUB / CMD token whose ``text`` is the substitution
        # source (``$script``).  Lowering that as a script body
        # compiled to "call the proc named by ``$script`` with 0
        # args" — wrong.  Defer those forms to the runtime
        # via :class:`IRBarrier` (mirrors the ``catch`` branch above).
        if (
            body_tok is None
            or not (arg_single and arg_single[0])
            or body_tok.type is not TokenType.STR
        ):
            return IRBarrier(
                range=cmd.range,
                reason="try with dynamic body",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_script = self._lower_body_arg(args[0], body_tok, namespace=namespace)

        handlers: list[IRTryHandler] = []
        finally_body: IRScript | None = None
        finally_range: Range | None = None

        # Parse: try body ?on|trap matchArg varList handlerBody ...? ?finally finallyBody?
        i = 1
        while i < len(args):
            keyword = args[i]

            if keyword == "finally" and i + 1 < len(args):
                fin_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                if fin_tok is not None and i + 1 < len(arg_single) and arg_single[i + 1]:
                    finally_body = self._lower_body_arg(args[i + 1], fin_tok, namespace=namespace)
                    finally_range = range_from_token(fin_tok)
                i += 2
                continue

            if keyword in ("on", "trap") and i + 3 < len(args):
                match_arg = args[i + 1]
                var_list = args[i + 2]
                handler_body_text = args[i + 3]
                handler_tok = arg_tokens[i + 3] if i + 3 < len(arg_tokens) else None

                # Parse varList to extract result var and options var.
                var_names = _parse_param_names(var_list)
                result_var = _normalise_var_name(var_names[0]) if var_names else None
                options_var = _normalise_var_name(var_names[1]) if len(var_names) > 1 else None

                handler_body = self._lower_body_arg(
                    handler_body_text,
                    handler_tok,
                    namespace=namespace,
                )
                handler_range = (
                    range_from_token(handler_tok) if handler_tok is not None else cmd.range
                )

                handlers.append(
                    IRTryHandler(
                        kind=keyword,
                        match_arg=match_arg,
                        var_name=result_var,
                        options_var=options_var,
                        body=handler_body,
                        body_range=handler_range,
                    )
                )
                i += 4
                continue

            # Unrecognised keyword — bail to barrier.
            return IRBarrier(
                range=cmd.range,
                reason="malformed try handler",
                command=cmd.name,
                canonical_command=self._canonicalise_command(cmd.name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        return IRTry(
            range=cmd.range,
            body=body_script,
            body_range=range_from_token(body_tok),
            handlers=tuple(handlers),
            finally_body=finally_body,
            finally_range=finally_range,
            raw_args=tuple(args),
        )

    def _lower_dict(self, cmd: _Command, *, namespace: str) -> IRStatement:
        """Lower ``dict`` subcommands with awareness of bodies and variable defs."""
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        sub = args[0] if args else ""
        sub_args = args[1:]  # args after the subcommand

        match sub:
            case "for" | "map" if len(sub_args) >= 3:
                # dict for {k v} dictValue body / dict map {k v} dictValue body
                var_names = _parse_param_names(sub_args[0])
                body_idx = 3  # index in original args (sub=0, varList=1, dictVal=2, body=3)
                body_tok = arg_tokens[body_idx] if body_idx < len(arg_tokens) else None
                if body_tok is None or not (body_idx < len(arg_single) and arg_single[body_idx]):
                    return IRBarrier(
                        range=cmd.range,
                        reason=f"dict {sub} with dynamic body",
                        command=cmd.name,
                        canonical_command=self._canonicalise_command(cmd.name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                body_script = self._lower_body_arg(sub_args[2], body_tok, namespace=namespace)
                return IRForeach(
                    range=cmd.range,
                    iterators=((var_names, sub_args[1]),),
                    body=body_script,
                    body_range=range_from_token(body_tok),
                    is_lmap=(sub == "map"),
                    raw_args=tuple(args),
                    is_dict_iteration=True,
                )

            case "set" | "unset" | "append" | "lappend" | "incr" if sub_args:
                # dict set/unset/append/lappend/incr dictVar key ... — mutates dictVar.
                var_name = _normalise_var_name(sub_args[0])
                return IRCall(
                    range=cmd.range,
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    defs=(var_name,),
                    reads_own_defs=True,
                    safe_on_uninit=REGISTRY.is_safe_on_uninit(
                        cmd.name,
                        sub,
                        _active_dialect(),
                    ),
                    tokens=cmd.cmd_tokens,
                )

            case "update" | "with":
                # Complex variable binding — stay as barrier but lower the body.
                return IRBarrier(
                    range=cmd.range,
                    reason=f"dict {sub}",
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case _:
                # Pure subcommands: get, create, exists, keys, values, etc.
                return IRCall(
                    range=cmd.range,
                    command=cmd.name,
                    canonical_command=self._canonicalise_command(cmd.name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

    # ---------------------------------------------------------------
    # Barrier relaxation — static-body ``eval`` / ``uplevel``
    #
    # Recognise call shapes whose body is a pure braced literal with
    # no nested dynamic-shape barriers, and lower them to inline IR
    # (:class:`IRBlock` for ``eval``, :class:`IRUpFrame` for
    # ``uplevel``) rather than the generic :class:`IRBarrier` →
    # runtime ``tcl_eval`` path.  Keeps the compiled frame visible
    # to the body so caller locals, aliases, and ``variable``
    # declarations resolve via compiled lookups.  Anything that
    # fails the gate returns False / None so the dispatcher falls
    # through to the existing barrier case.
    #
    # ``eval`` shares :class:`IRBlock` with ``namespace eval`` for
    # codegen reuse; the two are distinguished by
    # ``source_tokens.argv_texts[0]`` ("eval" vs "namespace") at
    # consumer sites (VM dispatch, literal processing).
    # ---------------------------------------------------------------

    def _can_relax_eval(self, cmd: _Command) -> bool:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if cmd.expand_word is not None and any(cmd.expand_word):
            return False
        # Two recognised shapes:
        #   (a) single braced-literal argument — covered by the first
        #       branch below.  Dominant tcltest shape.
        #   (b) single command-substitution argument of the form
        #       ``eval [list <literal> <literal> ...]`` — a common
        #       idiom for "safely build and evaluate a command from
        #       known literal parts".  Covered by the second branch.
        if len(args) != 1 or not arg_single or not arg_single[0]:
            return False
        body_tok = arg_tokens[0]
        if body_tok.type is TokenType.STR:
            from .lowering_hooks._barrier_gate import body_has_dynamic_barrier

            return not body_has_dynamic_barrier(args[0], body_tok)
        if body_tok.type is TokenType.CMD:
            return self._eval_list_literal_body(body_tok.text) is not None
        if body_tok.type is TokenType.VAR:
            # ``eval $var`` where *var* is const-mapped to a braced
            # literal — treat as an inline body.  Same gate as the
            # STR case.
            literal = self._const_map_lookup(body_tok)
            if literal is None:
                return False
            from .lowering_hooks._barrier_gate import body_has_dynamic_barrier

            return not body_has_dynamic_barrier(literal, None)
        return False

    def _eval_subst_nocommands_body(self, cmd_text: str) -> str | None:
        """If *cmd_text* is ``subst -nocommands {template}`` (or the
        equivalent ``-nobackslashes -nocommands {template}`` /
        ``-nocommands -nobackslashes {template}`` shapes) AND every
        ``\\$var`` inside *template* is in the current const-map,
        return the substituted string; otherwise ``None`` (so the
        caller keeps the runtime-dispatch path).

        Used by the ``proc`` case in :meth:`_lower_command` to
        materialise the tcltest-style ``Option`` factory body at
        compile time when the surrounding proc has all the template
        vars const-tracked.
        """
        try:
            inner = segment_commands(cmd_text, None)
        except TclParseError:
            return None
        if len(inner) != 1:
            return None
        inner_cmd = inner[0]
        if not inner_cmd.texts or inner_cmd.texts[0] != "subst":
            return None
        # Walk the args looking for ``-nocommands`` and (optionally)
        # ``-nobackslashes`` / ``-novariables`` flags.  ``-novariables``
        # would defeat the whole point; refuse on that one.  The
        # template must be the final positional argument.
        saw_nocommands = False
        template_text: str | None = None
        template_tok = None
        for i, tok in enumerate(inner_cmd.argv[1:], start=1):
            text = inner_cmd.texts[i]
            if text == "-nocommands":
                saw_nocommands = True
                continue
            if text == "-nobackslashes":
                # Our evaluator always processes backslashes (Tcl's
                # default).  If the source explicitly disables them,
                # the semantics differ — refuse.
                return None
            if text == "-novariables":
                return None
            if text.startswith("-"):
                return None  # unknown flag; don't risk it
            # First positional — the template.  Must be a single
            # braced STR token so the template bytes are known
            # literally (no nested substitutions).
            if not inner_cmd.single_token_word[i]:
                return None
            if tok.type is not TokenType.STR:
                return None
            if template_text is not None:
                return None  # more than one positional
            template_text = text
            template_tok = tok
        if not saw_nocommands or template_text is None or template_tok is None:
            return None
        # Build a fresh const-map view for the evaluator.  The
        # const-map stack's top frame holds the proc-local names
        # we've tracked so far.  Pass a copy so the evaluator
        # can't accidentally mutate lowering state.
        if self._proc_depth <= 0 or not self._const_map_stack:
            return None
        from ..parsing.subst_nocommands import subst_nocommands

        const_map = dict(self._const_map_stack[-1])
        return subst_nocommands(template_text, const_map)

    @staticmethod
    def _eval_list_literal_body(cmd_text: str) -> str | None:
        """If *cmd_text* is ``list lit1 lit2 ...`` with all-literal
        arguments, return the body text the list would evaluate to —
        otherwise ``None``.

        Literal means ``TokenType.ESC`` / ``TokenType.STR`` only: no
        ``$var`` substitution, no nested command substitution.

        The synthesised body must be a string whose re-parse yields
        the same argument sequence as the original ``[list …]`` call.
        ``list`` applies canonical Tcl list quoting (brace-wrap for
        whitespace / braces / unbalanced brackets, leading-``#``
        escaping, etc.); we do NOT attempt to reproduce that full
        canonicalisation here.  Instead we only relax when every arg
        is already list-safe as-is: a bareword ESC arg with no
        whitespace / braces / backslashes, or a STR token (which we
        re-brace).  Anything that would need canonical requoting
        (spaces, ``;``, ``\\``, leading ``#``, empty, etc.) falls
        back to the runtime eval path.
        """
        try:
            inner = segment_commands(cmd_text, None)
        except TclParseError:
            return None
        if len(inner) != 1:
            return None
        inner_cmd = inner[0]
        if not inner_cmd.texts or inner_cmd.texts[0] != "list":
            return None
        # Each element after ``list`` must be a single-token literal.
        for i, tok in enumerate(inner_cmd.argv[1:], start=1):
            if not inner_cmd.single_token_word[i]:
                return None
            if tok.type not in (TokenType.ESC, TokenType.STR):
                return None
            if tok.type is TokenType.ESC and ("$" in tok.text or "[" in tok.text):
                return None
            text = inner_cmd.texts[i]
            if tok.type is TokenType.ESC:
                # A bareword ESC is list-safe only when it has no
                # characters that would change its parse when joined
                # with spaces.  Whitespace, backslashes, braces,
                # unbalanced brackets, leading ``#``, and empty all
                # need canonical Tcl list quoting which we refuse to
                # reproduce — bail so the body stays on the eval
                # path.
                if text == "" or text[0] == "#":
                    return None
                for ch in text:
                    if ch in ' \t\n\r\\{}";':
                        return None
        # Body is the args joined with spaces — safe because every
        # arg has passed the bareword gate above.
        parts: list[str] = []
        for i, arg in enumerate(inner_cmd.texts[1:], start=1):
            tok = inner_cmd.argv[i]
            if tok.type is TokenType.STR:
                # STR tokens arrive without their outer braces; re-brace
                # so the re-parse groups the element as one word.  The
                # STR content itself cannot contain unescaped braces at
                # the outer level (the lexer would have rejected that).
                parts.append("{" + arg + "}")
            else:
                parts.append(arg)
        return " ".join(parts)

    def _relax_eval(self, cmd: _Command, *, namespace: str) -> IRBlock:
        args = cmd.args
        body_tok = cmd.arg_tokens[0]
        if body_tok.type is TokenType.STR:
            body_text = args[0]
            body_script = self._lower_body_arg(body_text, body_tok, namespace=namespace)
        elif body_tok.type is TokenType.VAR:
            # Const-mapped ``$var`` body — re-lower the recorded literal.
            literal = self._const_map_lookup(body_tok)
            assert literal is not None  # gated by _can_relax_eval
            body_script = self._lower_script(literal, namespace=namespace)
        else:
            # CMD token — body_text synthesised from ``[list ...]`` args.
            body_text = self._eval_list_literal_body(body_tok.text) or ""
            body_script = self._lower_script(body_text, namespace=namespace)
        return IRBlock(
            range=cmd.range,
            body=body_script,
            namespace=namespace,
            source_args=tuple(args),
            source_tokens=cmd.cmd_tokens,
        )

    def _can_relax_uplevel(self, cmd: _Command) -> bool:
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token
        if cmd.expand_word is not None and any(cmd.expand_word):
            return False
        if not args:
            return False
        # Either ``uplevel body`` (implicit level=1) or
        # ``uplevel LEVEL body`` with LEVEL statically parsable.  The
        # final word is the body.
        body_idx = self._uplevel_body_index(args, arg_tokens)
        if body_idx is None:
            return False
        if body_idx >= len(arg_single) or not arg_single[body_idx]:
            return False
        body_tok = arg_tokens[body_idx]
        from .lowering_hooks._barrier_gate import body_has_dynamic_barrier

        if body_tok.type is TokenType.STR:
            return not body_has_dynamic_barrier(args[body_idx], body_tok)
        if body_tok.type is TokenType.VAR:
            # Body is a single ``$var`` reference — relax when the
            # const-map has recorded a braced literal for *var* and
            # that literal is itself free of nested dynamic barriers.
            literal = self._const_map_lookup(body_tok)
            if literal is None:
                return False
            return not body_has_dynamic_barrier(literal, None)
        return False

    def _relax_uplevel(self, cmd: _Command, *, namespace: str):
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        body_idx = self._uplevel_body_index(args, arg_tokens)
        assert body_idx is not None  # gated by _can_relax_uplevel
        body_tok = arg_tokens[body_idx]
        shift = self._uplevel_shift(args, body_idx)
        if body_tok.type is TokenType.VAR:
            # Fold in the const-mapped literal.  The re-lowered body
            # loses the original source range, so passes that rely on
            # token-accurate diagnostics inside the body will see the
            # uplevel's own range instead — acceptable because the
            # original text lived in a separate ``set`` command whose
            # body-source range is retained on the ``IRAssignConst``.
            literal = self._const_map_lookup(body_tok)
            assert literal is not None  # gated by _can_relax_uplevel
            body_script = self._lower_script(literal, namespace=namespace)
        else:
            body_script = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
        return IRUpFrame(
            range=cmd.range,
            frame_shift=shift,
            body=body_script,
            source_tokens=cmd.cmd_tokens,
        )

    @staticmethod
    def _uplevel_body_index(args: list[str], arg_tokens: list[Token]) -> int | None:
        """Return the index of the body arg in an ``uplevel`` call.

        ``uplevel ?level? body`` — level is absent, a bare integer, or
        ``#N``.  When level is present the body is the trailing arg;
        otherwise the body is ``args[0]``.  Returns ``None`` when the
        call is malformed or the level is dynamic (``$lvl`` / ``[cmd]``).
        """
        if not args:
            return None
        first = args[0]
        first_tok = arg_tokens[0]
        is_level = first.lstrip("-").isdigit() or first.startswith("#")
        if is_level:
            # Level must be a plain (ESC) token — braced levels and
            # substituted levels are uncommon and dynamic.
            if first_tok.type is not TokenType.ESC:
                return None
            # Require exactly one body word after the level.  Tcl
            # concatenates multiple body words into a single script
            # (``uplevel 1 puts hello`` → body is ``puts hello``);
            # synthesising a correct body string from separate words
            # at compile time requires per-token list-quoting that
            # matches Tcl's canonical concat rules.  Rather than
            # reproduce that, bail and let the runtime eval path
            # handle the concat semantics.
            if len(args) != 2:
                return None
            return 1
        # No level — body is the first arg.  Require exactly one arg
        # (multi-word concat deferred, same rationale as above).
        if len(args) != 1:
            return None
        return 0

    @staticmethod
    def _uplevel_shift(args: list[str], body_idx: int) -> int:
        """Decode the ``uplevel`` level into a frame-depth-stash shift.

        Mirrors the encoding ``_emit_cmd_uplevel`` uses today:
        ``#0`` → large sentinel (``0x3FFF_FFFF``) so stash clamps to
        global regardless of current depth; bare integer N → N.
        Absent level defaults to 1.
        """
        if body_idx == 0:
            return 1
        level = args[0]
        if level == "#0":
            return 0x3FFF_FFFF
        if level.startswith("#"):
            try:
                # ``#N`` with N > 0 — approximate as a single-frame
                # shift; codegen already clamps.
                abs_level = int(level[1:])
                return 0 if abs_level == 0 else 1
            except ValueError:
                return 1
        try:
            return int(level)
        except ValueError:
            return 1

    def _lower_command(self, cmd: _Command, *, namespace: str) -> IRStatement | None:
        if not cmd.texts:
            return None

        # Track the enclosing namespace for the duration of this lowering
        # call so registered hooks (which receive only ``(lowerer, cmd)``)
        # can canonicalise command names via :meth:`canonicalise_command`.
        # Restored to the previous value on return so nested lowerings
        # (an outer ``namespace eval ns { … }`` wrapping inner top-level
        # statements that recurse back through this method) see the
        # right scope.
        prev_namespace = self._current_namespace
        self._current_namespace = namespace
        try:
            return self._lower_command_body(cmd, namespace=namespace)
        finally:
            self._current_namespace = prev_namespace

    def _lower_command_body(self, cmd: _Command, *, namespace: str) -> IRStatement | None:
        cmd_name = cmd.name
        args = cmd.args
        arg_tokens = cmd.arg_tokens
        arg_single = cmd.arg_single_token

        # Detect ``interp alias {} name {} target ?args?`` and record
        # the alias for resolving argument semantics of later calls.
        detected = detect_interp_alias(cmd_name, args)
        if detected is not None:
            qualified, target_cmd_name, prepended_args = detected
            self._command_aliases[qualified] = (target_cmd_name, prepended_args)

        # Detect ``rename oldName newName`` with literal arguments and
        # record the redirect for per-call-site canonicalisation.  Only
        # static renames register: a dynamic ``rename $foo bar`` cannot
        # be followed at compile time, so the canonical form for any
        # later call to ``bar`` stays the user spelling — the conservative
        # fallback documented on ``_command_renames``.  ``rename foo {}``
        # (deletion) records the new-name → "" mapping but the
        # canonicaliser only consults the redirect case today.
        # Suppressed inside dead code (matches the ``namespace import``
        # / ``namespace export`` gates above) so a folded-out rename
        # does not silently reroute live calls.
        if (
            cmd_name == "rename"
            and len(args) == 2
            and self._dead_code_depth == 0
            and (cmd.expand_word is None or not any(cmd.expand_word))
        ):
            old_name, new_name = args[0], args[1]
            # Skip non-literal arguments (any ``$`` / ``[`` substitution
            # token) — only rename the static case.
            if (
                old_name
                and not old_name.startswith("$")
                and not old_name.startswith("[")
                and not new_name.startswith("$")
                and not new_name.startswith("[")
            ):
                if new_name:
                    self._command_renames[new_name] = old_name
                else:
                    # ``rename oldName {}`` deletes oldName.  Record the
                    # deletion so a future linter check can flag a call
                    # to a deleted command, but don't redirect.
                    self._command_renames[old_name] = ""

        # Detect ``namespace import ?-force? pattern ...`` and record
        # each pattern against the current namespace.  Codegen resolves
        # the patterns against ``module.procedures`` to build the
        # compile-time import table so unqualified calls dispatch
        # directly instead of falling back to the interpreter.  We
        # only track absolute patterns (``::foo::*`` / ``::foo::bar``)
        # — relative patterns require runtime namespace-path walking
        # that we don't reproduce at compile time.
        if (
            cmd_name == "namespace"
            and len(args) >= 2
            and args[0] == "import"
            and (cmd.expand_word is None or not any(cmd.expand_word))
            # Suppress collection inside ``if {0} { … }`` / ``else``
            # branches that fold to dead code.  Otherwise the codegen's
            # compile-time import shortcut could rewrite a call site
            # based on a ``namespace import`` that never actually ran —
            # see https://github.com/bitwisecook/tcl-lsp/issues/189.
            and self._dead_code_depth == 0
        ):
            i = 1
            # Skip option flags (``-force`` and any future ``-foo``).
            while i < len(args) and args[i].startswith("-"):
                i += 1
            for pat in args[i:]:
                if pat.startswith("::") and "::" in pat[2:]:
                    self._namespace_imports.append((namespace, pat))

        # Capture ``namespace export ?-clear? pattern …`` directives
        # against the surrounding namespace.  Codegen pairs these
        # with ``namespace_imports`` to filter the compile-time
        # import shortcut — only exported names dispatch directly;
        # unexported imports fall back to runtime dispatch (which
        # produces the correct "unknown command" semantics).
        # ``-clear`` would wipe previously-registered patterns but
        # we don't implement that at compile time — any subsequent
        # ``export`` still appends, which only broadens the filter
        # (worst case loses some compile-time wins, never
        # misresolves).
        if (
            cmd_name == "namespace"
            and len(args) >= 2
            and args[0] == "export"
            and (cmd.expand_word is None or not any(cmd.expand_word))
            # Dead-code gate — see the matching comment on the
            # ``namespace import`` collection above.  A dead
            # ``namespace export`` paired with a dead ``namespace
            # import`` bypassed the export-pattern filter introduced
            # in commit 2f5cb00 because both patterns registered at
            # lowering time.  Suppressing export collection from the
            # same dead block closes the hole.
            and self._dead_code_depth == 0
        ):
            for pat in args[1:]:
                if pat == "-clear":
                    continue
                if pat.startswith("-"):
                    continue
                # Export patterns are simple names / globs, never
                # namespace-qualified — record as-is against the
                # current namespace.
                self._namespace_exports.append((namespace, pat))

        # Capture ``trace add execution cmdName ops body`` and the
        # matching ``remove`` form.  Execution traces install a Tcl
        # body that runs around every call to ``cmdName``, so the
        # traced command's effective side-effects are no longer just
        # whatever the registry declares — the trace body's effects
        # compose in.  Side-effect classification consults
        # :meth:`IRModule.traced_commands` to drop purity / CSE for
        # any command with a net active execution trace.
        # Out of scope here: ``trace add command`` (fires only on
        # rename/delete; doesn't compose into the command's per-call
        # effects) and ``trace add variable`` (handled separately —
        # its effect is on the named variable, not on a command).
        # Subscriptions on dialect-aliased ``::trace`` spellings are
        # captured too because the rewrite-alias resolver normalises
        # them to ``trace`` before the lowering hook runs.
        if (
            cmd_name in ("trace", "::trace")
            and len(args) >= 5
            and args[0] in ("add", "remove")
            and args[1] == "execution"
            and (cmd.expand_word is None or not any(cmd.expand_word))
            and self._dead_code_depth == 0
        ):
            # An argument word is "literal" only when it is composed of a
            # single non-substituting token: ``single_token_word`` is True
            # AND the token type is STR (braced) or ESC (plain/escaped).
            # ``foo$cmd`` lexes as ESC + VAR — its first token is ESC, but
            # ``single_token_word`` is False, so it must be treated as
            # dynamic.  Without the multi-token guard a target like
            # ``foo$cmd`` would record as a literal ``foo`` and let
            # purity/CSE checks proceed even though the actual traced
            # command is runtime-dependent (issue #251 review).
            def _arg_is_literal(arg_idx: int) -> bool:
                word_idx = arg_idx + 1
                if word_idx >= len(cmd.single_token_word):
                    return False
                if not cmd.single_token_word[word_idx]:
                    return False
                if word_idx >= len(cmd.argv):
                    return False
                return cmd.argv[word_idx].type in (TokenType.STR, TokenType.ESC)

            body_arg_tok = arg_tokens[4] if len(arg_tokens) > 4 else None
            target_dynamic = not _arg_is_literal(2)
            target_name = "" if target_dynamic else args[2]
            ops_dynamic = not _arg_is_literal(3)
            if ops_dynamic:
                ops_tuple: tuple[str, ...] = ()
            else:
                # The ops list is a Tcl list of {enter, leave, enterstep,
                # leavestep}.  Splitting on whitespace covers the common
                # literal form; non-literal ops fall through as ``()``
                # which the consumer treats as "all ops".
                ops_tuple = tuple(args[3].split())
            # Body must be braced (STR) and single-token to be considered
            # literal — an ESC body or a multi-token body could substitute.
            body_literal = (
                body_arg_tok is not None
                and body_arg_tok.type is TokenType.STR
                and len(cmd.single_token_word) > 5
                and cmd.single_token_word[5]
            )
            body_text: str | None = args[4] if body_literal else None
            body_range: Range | None = None
            if body_arg_tok is not None:
                body_range = Range(start=body_arg_tok.start, end=body_arg_tok.end)
            self._command_traces.append(
                CommandTrace(
                    target=target_name,
                    ops=ops_tuple,
                    body=body_text,
                    body_range=body_range,
                    action=args[0],
                    target_dynamic=target_dynamic,
                )
            )

        # Check for a registered lowering hook first.
        spec = REGISTRY.get_any(cmd_name)
        # If the command itself has no lowering hook, try the alias target.
        alias_entry = (
            self._resolve_alias(cmd_name, namespace)
            if spec is None or spec.lowering is None
            else None
        )
        if alias_entry is not None:
            target, prepended = alias_entry
            target_spec = REGISTRY.get_any(target)
            if target_spec is not None and target_spec.lowering is not None:
                # Build a virtual command with the target name and
                # prepended + real arguments for the lowering hook.
                # Construct synthetic tokens for prepended args so that
                # argv, texts, single_token_word, and expand_word all
                # have matching lengths.
                virtual_texts = [target, *prepended, *cmd.texts[1:]]
                virtual_single = [True] * (1 + len(prepended)) + cmd.single_token_word[1:]
                template_tok = cmd.argv[0]
                synthetic_toks = [
                    Token(
                        type=TokenType.ESC, text=p, start=template_tok.start, end=template_tok.end
                    )
                    for p in prepended
                ]
                virtual_argv = [cmd.argv[0], *synthetic_toks, *cmd.argv[1:]]
                virtual_expand: list[bool] | None = None
                if cmd.expand_word is not None:
                    virtual_expand = (
                        [cmd.expand_word[0]] + [False] * len(prepended) + cmd.expand_word[1:]
                    )
                virtual = _Command(
                    range=cmd.range,
                    argv=virtual_argv,
                    texts=virtual_texts,
                    single_token_word=virtual_single,
                    all_tokens=cmd.all_tokens,
                    expand_word=virtual_expand,
                )
                result = target_spec.lowering(self, virtual)
                if result is not None:
                    return cast(IRStatement, result)
                spec = target_spec
        if spec is not None and spec.lowering is not None:
            result = spec.lowering(self, cmd)
            if result is not None:
                return cast(IRStatement, result)

        # {*} argument expansion hides the runtime arg list from the
        # specialised structured lowerings below (proc, when, namespace
        # eval, if, switch, for, while, foreach) which index args
        # positionally.  Emit an IRBarrier so downstream analysis sees
        # the original tokens (for codegen and arity checks) without
        # assuming a particular arg shape.  Generic commands (e.g.
        # ``list {*}{a b c}``) fall through to the default IRCall path,
        # which preserves ``tokens.expand_word`` and lets the codegen's
        # ``_try_list_expand_call`` / ``_emit_expanded_call`` handle it.
        _structured_with_expansion = cmd_name in (
            "proc",
            "when",
            "namespace",
            "if",
            "switch",
            "for",
            "while",
            "foreach",
            "foreach_in_collection",
        )
        if _structured_with_expansion and cmd.expand_word is not None and any(cmd.expand_word):
            return IRBarrier(
                range=cmd.range,
                reason=f"{cmd_name} with argument expansion",
                command=cmd_name,
                canonical_command=self._canonicalise_command(cmd_name, namespace),
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        match cmd_name:
            case "proc" if len(args) == 3 and len(arg_tokens) >= 3:
                proc_name = args[0]
                # Empty-simple-name procs (``proc ::ns:: {args} {body}``):
                # Tcl 9 lets a trailing ``::`` register a command named
                # ``""`` inside the target namespace
                # (``TclGetNamespaceForQualName`` returns simpleName=""
                # rather than NULL for trailing colons on cmd/var
                # lookups — see tclNamesp.c:2493).  Our static
                # name-normaliser strips the trailing colons (since
                # ``"a::".split("::")`` filters the empty trailing
                # element), so an AOT-lifted empty-name proc would
                # register under the namespace name itself.  Route the
                # whole shape through the runtime ``proc`` builtin —
                # ``proc_register`` accepts ``simple_len == 0`` natively
                # and ``ns_find_command`` matches trailing ``::`` on
                # lookup.  Carries namespace-old-1.27 / 2.1 / 2.2 and
                # namespace-14.11.
                if proc_name.endswith("::"):
                    return IRBarrier(
                        range=cmd.range,
                        reason="empty-simple-name proc",
                        command=cmd_name,
                        canonical_command=self._canonicalise_command(cmd_name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                # Dynamic proc names (containing $ or [) can only be
                # resolved at runtime in general — BUT if the name is a
                # bare ``$var`` and ``var`` is in the lowering
                # const-map (see :meth:`_const_map_lookup`), we can
                # substitute the literal and continue lowering as a
                # normal static-name proc.  This catches the Option
                # factory pattern in tcltest (``proc $varName body``
                # inside a helper that was just called with a literal
                # ``-foo`` name), which was previously bailing straight
                # to the runtime.  Multi-token names (``foo_$x``,
                # ``$a$b``) and command-substitution names stay out of
                # scope — they're genuinely dynamic in the general
                # case.  P6.1.
                if "$" in proc_name or "[" in proc_name:
                    substituted: str | None = None
                    if (
                        "[" not in proc_name
                        and cmd.single_token_word[1]
                        and arg_tokens[0].type is TokenType.VAR
                    ):
                        substituted = self._const_map_lookup(arg_tokens[0])
                    if substituted is None:
                        return IRBarrier(
                            range=cmd.range,
                            reason="dynamic proc name",
                            command=cmd_name,
                            canonical_command=self._canonicalise_command(cmd_name, namespace),
                            args=tuple(args),
                            tokens=cmd.cmd_tokens,
                        )
                    proc_name = substituted
                # Dynamic body (``proc inner {} $body``): the body
                # SOURCE TOKEN is a ``$var`` / ``[cmd]`` reference,
                # not braced literal bytes.  Naively lifting it as a
                # static proc would compile the LITERAL ``${body}``
                # text as the proc body, which then re-substitutes
                # at every call — surfaces as ``unknown command:
                # <substituted-value>``.  Before bailing to a
                # runtime ``proc`` call, try to materialise the body
                # at compile time:
                #   * VAR — look up the const-map for ``$var``
                #     bound to a braced literal.
                #   * CMD — evaluate ``[subst -nocommands {…}]`` if
                #     every template ``$var`` is const-tracked.
                # If materialisation succeeds, fold the resolved
                # script in place of the dynamic token; otherwise
                # bail.  Dynamic params have no such materialisation
                # path so they always bail.
                body_tok = arg_tokens[2]
                body_tok_type = body_tok.type
                params_tok_type = arg_tokens[1].type
                # A multi-token word (e.g. quoted/interpolated like
                # ``"foo$bar"`` or ``" … [cmd] … "``) reports its
                # first sub-token's type via ``arg_tokens[i]`` but
                # ``arg_single_token[i]`` is False — the body or
                # params still contain runtime substitutions and
                # must be treated as dynamic to avoid lifting the
                # literal source text as a static script.  Materialise
                # the body only when it is a single VAR or CMD token
                # we can const-resolve; otherwise treat as dynamic
                # and bail (or register an empty IRProcedure when the
                # name resolved).
                body_is_single = arg_single[2] if len(arg_single) > 2 else False
                params_is_single = arg_single[1] if len(arg_single) > 1 else False
                body_is_dynamic = (
                    body_tok_type in (TokenType.VAR, TokenType.CMD) or not body_is_single
                )
                params_is_dynamic = (
                    params_tok_type in (TokenType.VAR, TokenType.CMD) or not params_is_single
                )
                materialised_body: str | None = None
                if body_is_single and body_tok_type is TokenType.CMD:
                    materialised_body = self._eval_subst_nocommands_body(body_tok.text)
                elif body_is_single and body_tok_type is TokenType.VAR:
                    materialised_body = self._const_map_lookup(body_tok)
                # Bail when params is dynamic — there's no static
                # arg-list to build a real IRProcedure from.  When
                # the body is dynamic but the name resolved through
                # the const-map, we still register the IRProcedure
                # (with an empty body when we couldn't materialise
                # the script) so static analysis sees the proc; the
                # IRCall emitted below carries the correct dynamic
                # body to the runtime ``proc`` command.
                if params_is_dynamic or (
                    body_is_dynamic and materialised_body is None and proc_name == args[0]
                ):
                    return IRBarrier(
                        range=cmd.range,
                        reason="dynamic proc body or params",
                        command=cmd_name,
                        canonical_command=self._canonicalise_command(cmd_name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                try:
                    params = _parse_param_names(args[1])
                except Exception:
                    params = ()
                qualified = _qualify_proc_name(namespace, proc_name)
                # Fresh const-map frame for the nested proc body.
                # Without this, ``_lower_script`` would inherit the
                # enclosing scope's tracked scalars — which is
                # correct for control-flow bodies (``if`` / ``catch``
                # / loops run in the same frame) but wrong for
                # ``proc`` bodies, which have their own runtime
                # frame.  A ``set body {…}`` in the outer proc
                # must not appear to a nested ``proc inner``'s
                # barrier-relaxation gate as a tracked literal.
                self._proc_depth += 1
                self._const_map_stack.append({})
                try:
                    if materialised_body is not None:
                        # Lower the substituted string as a fresh
                        # script — ``_lower_body_arg`` short-circuits
                        # to an empty IRScript when given a ``None``
                        # token, so we bypass it and call
                        # ``_lower_script`` directly.
                        body = self._lower_script(
                            materialised_body, namespace=namespace, body_token=None
                        )
                    elif body_is_dynamic:
                        # Dynamic body that we couldn't materialise
                        # (e.g. ``proc $name {x} $body`` where
                        # ``$body`` is a parameter, not a const-mapped
                        # literal).  The runtime ``proc`` IRCall
                        # below carries the actual body bytes; the
                        # IRProcedure body stays empty so static
                        # analysis doesn't try to compile the literal
                        # ``$body`` text as a script.
                        body = IRScript()
                    else:
                        body = self._lower_body_arg(args[2], body_tok, namespace=namespace)
                finally:
                    self._const_map_stack.pop()
                    self._proc_depth -= 1
                # Register the IR proc for compilation (bytecode
                # generation) but ALWAYS emit a runtime ``proc`` call
                # too.  This ensures ``define_proc`` runs at the
                # correct point during sequential execution, which is
                # critical for: (a) parameter validation inside
                # ``catch``, (b) proc redefinitions, and (c) correct
                # ``rename`` interaction with pre-registered procs.
                if qualified not in self.module.procedures:
                    self.module.procedures[qualified] = IRProcedure(
                        name=proc_name,
                        qualified_name=qualified,
                        params=params,
                        range=cmd.range,
                        body=body,
                        params_raw=args[1],
                        body_source=args[2],
                        namespace_scoped=self._in_namespace_eval,
                    )
                else:
                    self.module.redefined_procedures.add(qualified)
                # If the name was a const-substituted ``$var``, the
                # runtime-side registration also needs the concrete
                # name rather than the literal ``$var`` bytes we'd
                # otherwise pass on — splice the resolved ``proc_name``
                # into the args tuple so the emitted ``proc`` command
                # reaches the runtime with the real name.
                out_args = (proc_name,) + tuple(args[1:]) if proc_name != args[0] else tuple(args)
                return IRCall(
                    range=cmd.range,
                    command=cmd_name,
                    canonical_command=self._canonicalise_command(cmd_name, namespace),
                    args=out_args,
                    tokens=cmd.cmd_tokens,
                )

            case "when" if len(args) >= 2 and len(arg_tokens) >= 2:
                event_name = args[0]
                body_idx = len(args) - 1
                body_tok = arg_tokens[body_idx]
                # Same rationale as the ``proc`` case: ``when``
                # handlers have their own runtime frame, so the
                # outer scope's tracked scalars must not be
                # visible to this body's barrier-relaxation gate.
                self._proc_depth += 1
                self._const_map_stack.append({})
                try:
                    body = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
                finally:
                    self._const_map_stack.pop()
                    self._proc_depth -= 1

                # Extract priority from ``when EVENT priority N { body }``
                base_priority = 500
                if len(args) >= 4 and args[1] == "priority":
                    try:
                        base_priority = int(args[2])
                    except ValueError:
                        pass

                # Number duplicate handlers: first is ::when::EVENT,
                # subsequent are ::when::EVENT#1, ::when::EVENT#2, …
                n = self._when_counts.get(event_name, 0)
                self._when_counts[event_name] = n + 1
                qualified = f"::when::{event_name}" if n == 0 else f"::when::{event_name}#{n}"

                self.module.procedures[qualified] = IRProcedure(
                    name=event_name,
                    qualified_name=qualified,
                    params=(),
                    range=cmd.range,
                    body=body,
                    base_priority=base_priority,
                )
                return IRCall(
                    range=cmd.range,
                    command=cmd_name,
                    canonical_command=self._canonicalise_command(cmd_name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case "namespace" if len(args) >= 3 and args[0] == "eval" and len(arg_tokens) >= 3:
                # Only inline-compile the body when *all* three args
                # are static (braced or bare identifiers).  A dynamic
                # namespace name (``[namespace current]``) or a
                # dynamic body (``[list upvar 0 A B]``) can't be
                # reasoned about statically — the body might
                # evaluate to a script that does anything — so fall
                # back to the tcl_eval barrier path where the
                # interpreter resolves both at runtime.
                #
                # For the canonical shape ``namespace eval ::ns {
                # body }`` the body is a single STR token; anything
                # else (CMD / VAR / ESC with substitutions) means
                # dynamic.  The namespace arg must likewise be a
                # plain identifier.
                from ..parsing.tokens import TokenType as _TT

                body_tok = arg_tokens[2]
                ns_tok = arg_tokens[1]
                body_is_static = body_tok is not None and body_tok.type == _TT.STR
                ns_is_static = (
                    ns_tok is not None
                    and ns_tok.type in (_TT.STR, _TT.ESC)
                    and "$" not in args[1]
                    and "[" not in args[1]
                )
                if body_is_static and ns_is_static:
                    child_ns = _join_namespace(namespace, args[1])
                    prev_ns_eval = self._in_namespace_eval
                    self._in_namespace_eval = True
                    body_script = self._lower_body_arg(args[2], arg_tokens[2], namespace=child_ns)
                    self._in_namespace_eval = prev_ns_eval
                    # Emit the body as an inline block — procs
                    # inside were lifted to module.procedures with
                    # qualified names (child_ns::name); other
                    # statements (variable, trace, Option …) run as
                    # ordinary code, with codegen consulting
                    # IRBlock.namespace for unqualified command
                    # resolution.  source_args/source_tokens let
                    # non-inlining codegen targets dispatch the
                    # original namespace-eval call instead.
                    return IRBlock(
                        range=cmd.range,
                        body=body_script,
                        namespace=child_ns,
                        source_args=tuple(args),
                        source_tokens=cmd.cmd_tokens,
                    )
                # Dynamic namespace eval — fall back to tcl_eval so
                # the runtime evaluates the body script however it
                # shakes out.
                return IRBarrier(
                    range=cmd.range,
                    reason="namespace eval",
                    command=cmd_name,
                    canonical_command=self._canonicalise_command(cmd_name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case "if":
                return self._lower_if(cmd, namespace=namespace)

            case "switch":
                return self._lower_switch(cmd, namespace=namespace)

            case "for":
                return self._lower_for(cmd, namespace=namespace)

            case "while":
                return self._lower_while(cmd, namespace=namespace)

            case "foreach":
                return self._lower_foreach(cmd, namespace=namespace)

            case "foreach_in_collection" if REGISTRY.get(cmd_name, _active_dialect()) is not None:
                # Only lower as a loop when enabled in the active dialect.
                # Enforce exact arity so incorrect arg counts produce
                # IRBarrier and get caught by generic arity checks (E002/E003).
                if len(args) != 3:
                    return IRBarrier(
                        range=cmd.range,
                        reason="invalid foreach_in_collection arity",
                        command=cmd_name,
                        canonical_command=self._canonicalise_command(cmd_name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                return self._lower_foreach(cmd, namespace=namespace)

            case "lmap":
                return self._lower_foreach(cmd, namespace=namespace, is_lmap=True)

            case "catch":
                return self._lower_catch(cmd, namespace=namespace)

            case "try":
                return self._lower_try(cmd, namespace=namespace)

            case "dict" if args:
                return self._lower_dict(cmd, namespace=namespace)

            case "eval" if self._can_relax_eval(cmd):
                return self._relax_eval(cmd, namespace=namespace)

            case "uplevel" if self._can_relax_uplevel(cmd):
                return self._relax_uplevel(cmd, namespace=namespace)

            case _ if cmd_name in _DYNAMIC_BARRIER_COMMANDS:
                return IRBarrier(
                    range=cmd.range,
                    reason="dynamic command",
                    command=cmd_name,
                    canonical_command=self._canonicalise_command(cmd_name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case _:
                # Resolve alias for arg role lookups.
                role_cmd = cmd_name
                role_args: list[str] = list(args)
                prepend_n = 0
                fallback_alias = self._resolve_alias(cmd_name, namespace)
                if fallback_alias is not None:
                    target, prepended = fallback_alias
                    role_cmd = target
                    role_args = list(prepended) + list(args)
                    prepend_n = len(prepended)
                body_indices = arg_indices_for_role(role_cmd, role_args, ArgRole.BODY)
                var_indices = arg_indices_for_role(role_cmd, role_args, ArgRole.VAR_WRITE)
                var_read_indices = arg_indices_for_role(role_cmd, role_args, ArgRole.VAR_READ)
                if body_indices:
                    return IRBarrier(
                        range=cmd.range,
                        reason="unsupported body command",
                        command=cmd_name,
                        canonical_command=self._canonicalise_command(cmd_name, namespace),
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                if var_indices or var_read_indices:
                    # Subtract prepend_n to map virtual indices back to
                    # real arg positions (mirrors the analyser).
                    var_defs = tuple(
                        _normalise_var_name(args[i - prepend_n])
                        for i in sorted(var_indices)
                        if 0 <= i - prepend_n < len(args)
                    )
                    var_reads = tuple(
                        _normalise_var_name(args[i - prepend_n])
                        for i in sorted(var_read_indices)
                        if 0 <= i - prepend_n < len(args)
                    )
                    return IRCall(
                        range=cmd.range,
                        command=cmd_name,
                        canonical_command=self._canonicalise_command(cmd_name, namespace),
                        args=tuple(args),
                        defs=var_defs,
                        reads=var_reads,
                        tokens=cmd.cmd_tokens,
                    )
                return IRCall(
                    range=cmd.range,
                    command=cmd_name,
                    canonical_command=self._canonicalise_command(cmd_name, namespace),
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )


def lower_to_ir(
    source: str,
    *,
    chunk_ir: list[tuple[tuple[IRStatement, ...], dict[str, IRProcedure]] | None] | None = None,
    chunks: list[TopLevelChunk] | None = None,
) -> IRModule:
    """Lower Tcl source to the analysis IR.

    When *chunk_ir* and *chunks* are provided, cached chunk IR entries
    (non-``None``) are reused and only ``None`` entries are lowered
    fresh.  This is the incremental path: unchanged chunks skip
    re-segmentation and re-lowering.

    A post-lowering inlining pass runs after every fresh lower:
    whole-callee ``uplevel``-passthrough procs (whose body is a
    single ``uplevel 1 {…}``) have their bodies spliced into every
    call site as :class:`IRBlock` nodes.  See
    :mod:`core.compiler.inline_uplevel` for the recognition gate.
    """
    from .inline_uplevel import inline_uplevel_passthrough

    if chunk_ir is None or chunks is None:
        mod = _Lowerer().lower(source)
        inline_uplevel_passthrough(mod)
        return mod

    lowerer = _Lowerer()
    all_stmts: list[IRStatement] = []
    for i, chunk in enumerate(chunks):
        cached = chunk_ir[i] if i < len(chunk_ir) else None
        if cached is not None:
            stmts, procs = cached
            all_stmts.extend(stmts)
            # Replay proc-definition semantics: first definition wins,
            # later definitions are recorded as redefinitions.
            for name, proc in procs.items():
                if name in lowerer.module.procedures:
                    lowerer.module.redefined_procedures.add(name)
                else:
                    lowerer.module.procedures[name] = proc
        else:
            # Lower this chunk's commands fresh.
            cmds = list(chunk.commands)
            stmts, procs = lowerer.lower_commands(cmds)
            all_stmts.extend(stmts)
            # procs already on lowerer.module via lower_commands

    lowerer.module.top_level = IRScript(statements=tuple(all_stmts))
    lowerer.module.namespace_imports = tuple(lowerer._namespace_imports)
    lowerer.module.namespace_exports = tuple(lowerer._namespace_exports)
    lowerer.module.command_traces = tuple(lowerer._command_traces)
    inline_uplevel_passthrough(lowerer.module)
    return lowerer.module


def lower_commands_to_ir(
    source: str,
    commands: list[SegmentedCommand],
) -> tuple[tuple[IRStatement, ...], dict[str, IRProcedure]]:
    """Lower pre-segmented commands to IR statements and procedure definitions.

    Convenience wrapper around ``_Lowerer.lower_commands`` for use by
    the per-chunk incremental pipeline.
    """
    return _Lowerer().lower_commands(commands)
