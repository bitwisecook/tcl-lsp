from __future__ import annotations

import logging
import time
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ...compiler.compilation_unit import CompilationUnit

from ...commands.registry import REGISTRY
from ...common.dialect import active_dialect
from ...common.ranges import position_from_relative, range_from_token
from ...parsing.command_segmenter import SegmentedCommand, UnclosedDelimiter
from ...parsing.expr_lexer import (
    BUILTIN_EXPR_OPS,
    BUILTIN_MATH_FUNCTIONS,
    IRULES_EXPR_OPS,
    ExprTokenType,
    tokenise_expr,
)
from ...parsing.recovery import segment_with_recovery
from ...parsing.tokens import Token, TokenType
from ..semantic_model import (
    _FILE_SUPPRESS_KEY,
    _NOQA_ALL,
    AnalysisResult,
    Diagnostic,
    Range,
    Scope,
    Severity,
)
from ..stub_comments import scan_source_for_stubs
from ._snapshot import AnalyserSnapshot
from ._utils import parse_file_suppression, parse_noqa_line_suppressions

log = logging.getLogger(__name__)


class _AnalyserBase:
    """Core analysis loop — init, public API, body/expr traversal."""

    if TYPE_CHECKING:
        # Set in analyse()/analyse_chunked() before any usage; not in __init__.
        _source: str

        # Stubs for methods provided by sibling mixins when composed into Analyser.
        def _emit_unresolved_command_diagnostics(self, *a: Any, **kw: Any) -> None: ...
        def _emit_variable_usage_diagnostics(self, *a: Any, **kw: Any) -> None: ...
        def _emit_cfg_ssa_diagnostics(self, *a: Any, **kw: Any) -> None: ...
        def _detect_stolen_close_brace(self, *a: Any, **kw: Any) -> Any: ...
        def _recover_stray_close_bracket(self, *a: Any, **kw: Any) -> Any: ...
        def _recover_missing_open_brace(self, *a: Any, **kw: Any) -> Any: ...
        def _record_var_read(self, *a: Any, **kw: Any) -> None: ...
        def _process_command(self, *a: Any, **kw: Any) -> None: ...

    def __init__(self, *, disabled_diagnostics: frozenset[str] | None = None) -> None:
        self.result = AnalysisResult()
        self._current_scope = self.result.global_scope
        self._disabled_diagnostics = disabled_diagnostics or frozenset()
        self._last_comment: str = ""
        self._file_path: str | None = None
        # Per-scope constant string tracker: maps scope_id →
        #   { var_name → (value_text, value_range) }
        # Populated by `set var {literal}` assignments where the value is a
        # single constant token (no variable/command interpolation).
        self._const_strings: dict[int, dict[str, tuple[str, Range]]] = {}
        # Variables known to contain regex patterns: (scope_id, var_name).
        # Allows highlighting `$pat` when later used in a non-regex context
        # after being assigned via ``set pat {^\d+}; regexp $pat $str``.
        self._regex_vars: set[tuple[int, str]] = set()
        # iRules: the enclosing ``when EVENT`` name, if any.
        self._current_event: str | None = None
        # Cache of built-in command names for redefined-builtin detection.
        self._builtin_names: frozenset[str] | None = None
        self._builtin_dialect: str | None = None
        # Track conditional nesting depth for package require confidence.
        self._conditional_depth: int = 0
        # Track body nesting depth for top-level-only command detection.
        self._body_depth: int = 0
        # Implicit-args carry from a command-prefix body (e.g.
        # ``fileutil::updateInPlace`` appends one arg).  Set just
        # before processing the first top-level command of such a body
        # and consumed by the arity check.  Always 0 outside that
        # narrow window.
        self._pending_implicit_args: int = 0
        # Command aliases: alias_name -> (target_cmd, prepended_args).
        # Populated from ``interp alias {} name {} target ?arg ...?``.
        self._command_aliases: dict[str, tuple[str, tuple[str, ...]]] = {}
        # Variable-as-command sites: records where a $var is used as a
        # command name.  The post-analysis pass checks the type lattice to
        # determine whether the variable holds a TclOO object (suppress W307)
        # or not (emit W307).
        # Each entry: (var_name, method_name_or_None, cmd_token_range,
        # in_method, is_single_token_word).  ``is_single_token_word`` is
        # True only when the command word consists of just ``$var`` —
        # composite words like ``${cmd}x`` (where the runtime command
        # is the *concatenation*, not the variable's value) are False.
        self._var_command_sites: list[tuple[str, str | None, Range, bool, bool]] = []
        # Command-substitution-as-command sites: records where [cmd] is used
        # as a command name (e.g. ``[Dog new] bark``).  The post-pass checks
        # if the command returns a TclOO object and suppresses W307.
        # Each entry: (cmd_text, method_name_or_None, cmd_token_range,
        # in_method, is_single_token_word).
        self._cmd_command_sites: list[tuple[str, str | None, Range, bool, bool]] = []
        # Cache: id(scope) -> namespace string, for _namespace_from_scope.
        self._ns_cache: dict[int, str] = {}
        # Namespace ensembles: namespaces where ``namespace ensemble create``
        # was detected.  Their tail names become valid commands.
        self._ensemble_namespaces: set[str] = set()
        # Variables that have had oo::objdefine applied — these objects may
        # have additional per-instance methods not in the class definition.
        self._objdefined_vars: set[str] = set()
        # Guard against double W123 emission across analyse_commands/analyse_irule_event.
        self._unresolved_commands_emitted: bool = False
        # Pending trace-callback registrations: each entry is the tuple
        # of candidate qualified proc names (in resolution order) for a
        # ``trace add`` callback whose target proc may not yet have been
        # parsed.  Resolved at finalisation to mark
        # ``ProcDef.is_trace_callback`` and suppress W214.
        self._pending_trace_callbacks: list[tuple[str, ...]] = []

    def snapshot(self) -> AnalyserSnapshot:
        """Capture the analyser state for later restoration.

        Uses ``AnalysisResult.copy_for_snapshot()`` instead of
        ``copy.deepcopy`` for dramatically better performance — frozen
        dataclass items are shared by reference rather than copied.

        ``_const_strings`` and ``_regex_vars`` use ``id(scope)`` as
        keys — the ``scope_id_map`` records the mapping from the *live*
        scope ids to the *copied* scope objects so ``restore`` can
        remap them.
        """
        result_copy = self.result.copy_for_snapshot()
        # Build a mapping from old scope id → new (copied) scope object.
        scope_id_map: dict[int, Scope] = {}
        self._build_scope_id_map(self.result.global_scope, result_copy.global_scope, scope_id_map)

        return AnalyserSnapshot(
            result=result_copy,
            last_comment=self._last_comment,
            const_strings={k: dict(v) for k, v in self._const_strings.items()},
            regex_vars=set(self._regex_vars),
            current_event=self._current_event,
            conditional_depth=self._conditional_depth,
            # Shallow copy — values are immutable (str, tuple) so no deep copy needed.
            command_aliases=dict(self._command_aliases),
            scope_id_map=scope_id_map,
            # Tuples inside the list are immutable — shallow copy suffices.
            var_command_sites=list(self._var_command_sites),
            cmd_command_sites=list(self._cmd_command_sites),
            pending_trace_callbacks=list(self._pending_trace_callbacks),
        )

    def restore(self, snap: AnalyserSnapshot) -> None:
        """Restore analyser state from a snapshot.

        Uses ``AnalysisResult.copy_for_snapshot()`` instead of
        ``copy.deepcopy`` for dramatically better performance.
        ``_const_strings`` and ``_regex_vars`` keys (which are scope ids)
        are remapped to the new scope objects' ids.
        """
        self.result = snap.result.copy_for_snapshot()
        self._current_scope = self.result.global_scope
        self._last_comment = snap.last_comment
        self._current_event = snap.current_event
        self._conditional_depth = snap.conditional_depth
        self._command_aliases = dict(snap.command_aliases)
        self._ns_cache.clear()

        # Remap scope ids: old_id → new scope's id
        # snap.scope_id_map maps id(original_live_scope) → copied_scope_in_snap.result.
        # new_scope_id_map maps id(scope_in_snap.result) → scope_in_self.result.
        # We chain through the snapshot's copied scopes to build the final remap.
        id_remap: dict[int, int] = {}
        new_scope_id_map: dict[int, Scope] = {}
        self._build_scope_id_map(
            snap.result.global_scope, self.result.global_scope, new_scope_id_map
        )
        for old_id, snap_scope in snap.scope_id_map.items():
            new_scope = new_scope_id_map.get(id(snap_scope))
            if new_scope is not None:
                id_remap[old_id] = id(new_scope)

        # Remap _const_strings
        self._const_strings = {}
        for old_sid, entries in snap.const_strings.items():
            new_sid = id_remap.get(old_sid, old_sid)
            self._const_strings[new_sid] = dict(entries)

        # Remap _regex_vars
        self._regex_vars = set()
        for old_sid, var_name in snap.regex_vars:
            new_sid = id_remap.get(old_sid, old_sid)
            self._regex_vars.add((new_sid, var_name))

        # Restore _var_command_sites (no scope-id remapping needed;
        # entries store Range, not scope references).
        self._var_command_sites = list(snap.var_command_sites)
        self._cmd_command_sites = list(snap.cmd_command_sites)
        self._pending_trace_callbacks = list(snap.pending_trace_callbacks)

    @staticmethod
    def _build_scope_id_map(
        old_scope: Scope,
        new_scope: Scope,
        mapping: dict[int, Scope],
    ) -> None:
        """Walk old and new scope trees in parallel, recording id mappings."""
        mapping[id(old_scope)] = new_scope
        for old_child, new_child in zip(old_scope.children, new_scope.children, strict=True):
            _AnalyserBase._build_scope_id_map(old_child, new_child, mapping)

    def analyse_chunked(
        self,
        source: str,
        chunk_commands: list[list[SegmentedCommand]],
        *,
        cu: CompilationUnit | None = None,
        skip_stubs: bool = False,
        file_path: str | None = None,
    ) -> tuple[AnalysisResult, list[AnalyserSnapshot]]:
        """Analyse commands chunk-by-chunk and capture per-chunk snapshots.

        Returns the final ``AnalysisResult`` and a list of
        ``AnalyserSnapshot`` objects (one per chunk, taken after each
        chunk's commands are processed).  This avoids the need for a
        separate snapshot-building pass after analysis.

        When *skip_stubs* is ``True``, the inline-stub scan is skipped.
        This is used by the incremental path where the analyser has been
        restored from a snapshot that already includes earlier stubs.
        """
        self._source = source
        self._file_path = file_path
        self._unresolved_commands_emitted = False
        self._ns_cache.clear()
        if not skip_stubs:
            # Pre-scan for inline stubs (same as analyse()).
            cmd_stubs, expr_stubs = scan_source_for_stubs(source)
            self.result.stub_commands.extend(cmd_stubs)
            self.result.stub_expr_defs.extend(expr_stubs)
            self._check_stub_shadows()
        # Pre-scan for top-of-file ``# tcl-lsp: disable=...`` directives.
        file_codes = parse_file_suppression(source)
        if file_codes:
            self.result.suppressed_lines[_FILE_SUPPRESS_KEY] = file_codes

        snapshots: list[AnalyserSnapshot] = []
        for cmds in chunk_commands:
            self._analyse_commands_inner(cmds, self._current_scope, source)
            snapshots.append(self.snapshot())
            time.sleep(0)  # Yield GIL between chunks
        self._emit_unresolved_command_diagnostics(cu=cu)
        self._emit_variable_usage_diagnostics()
        self._emit_cfg_ssa_diagnostics(source, cu=cu)
        self._dedupe_diagnostics()
        return self.result, snapshots

    def analyse_commands(
        self,
        source: str,
        commands: list[SegmentedCommand],
        *,
        cu: CompilationUnit | None = None,
        finalise: bool = True,
    ) -> AnalysisResult:
        """Analyse pre-segmented commands instead of re-parsing *source*.

        This is the incremental entry point.  The caller provides the
        commands from dirty chunks; the analyser should have been
        ``restore()``-d from a snapshot covering earlier clean chunks.

        When *finalise* is ``True`` (the default), the post-analysis
        passes (CFG/SSA diagnostics, deduplication) are run.  Set to
        ``False`` when only building a partial snapshot.
        """
        self._source = source
        self._unresolved_commands_emitted = False
        self._ns_cache.clear()
        # Pre-scan for top-of-file ``# tcl-lsp: disable=...`` directives.
        file_codes = parse_file_suppression(source)
        if file_codes:
            self.result.suppressed_lines[_FILE_SUPPRESS_KEY] = file_codes
        self._analyse_commands_inner(commands, self._current_scope, source)
        if finalise:
            self._emit_unresolved_command_diagnostics()
            self._emit_variable_usage_diagnostics()
            self._emit_cfg_ssa_diagnostics(source, cu=cu)
            self._dedupe_diagnostics()
        return self.result

    def _analyse_commands_inner(
        self,
        commands: list[SegmentedCommand],
        scope: Scope,
        source: str,
    ) -> None:
        """Process pre-segmented commands — inner loop of ``_analyse_body``.

        Mirrors the command processing in ``_analyse_body`` but skips
        ``segment_with_recovery`` since commands are pre-segmented.
        """
        cmd_idx = 0
        while cmd_idx < len(commands):
            cmd = commands[cmd_idx]
            if cmd.preceding_comment is not None:
                self._last_comment = cmd.preceding_comment
                comment_lower = cmd.preceding_comment.lower()
                noqa_pos = comment_lower.find("noqa")
                if noqa_pos >= 0:
                    rest = cmd.preceding_comment[noqa_pos + 4 :].strip()
                    if rest.startswith(":"):
                        codes = frozenset(c.strip() for c in rest[1:].split(",") if c.strip())
                    else:
                        codes = _NOQA_ALL
                    for ln in range(cmd.range.start.line, cmd.range.end.line + 1):
                        existing = self.result.suppressed_lines.get(ln)
                        if existing:
                            self.result.suppressed_lines[ln] = existing | codes
                        else:
                            self.result.suppressed_lines[ln] = codes
            if cmd.is_partial:
                if (
                    cmd.partial_delimiter is UnclosedDelimiter.BRACE
                    and self._detect_stolen_close_brace(cmd, source, None)
                ):
                    cmd_idx += 1
                    continue
                _DELIMITER_MSG = {
                    UnclosedDelimiter.BRACE: "missing close-brace",
                    UnclosedDelimiter.BRACKET: "missing close-bracket",
                    UnclosedDelimiter.QUOTE: 'missing "',
                }
                msg = (
                    _DELIMITER_MSG.get(cmd.partial_delimiter, "missing close-brace")
                    if cmd.partial_delimiter is not None
                    else "missing close-brace"
                )
                self.result.diagnostics.append(
                    Diagnostic(
                        range=cmd.range,
                        severity=Severity.ERROR,
                        code="E200",
                        message=msg,
                    )
                )
                cmd_idx += 1
                continue
            self._recover_stray_close_bracket(cmd, scope)
            consumed = self._recover_missing_open_brace(
                cmd,
                commands,
                cmd_idx,
                scope,
                source,
                None,
            )
            for tok in cmd.all_tokens:
                if tok.type is TokenType.VAR:
                    self._record_var_read(tok.text, range_from_token(tok), scope)
                elif tok.type is TokenType.CMD:
                    self._analyse_body(tok.text, scope, body_token=tok)
            self._process_command(
                cmd.argv,
                cmd.texts,
                cmd.all_tokens,
                scope,
                source,
                expand_word=cmd.expand_word,
                single_token_word=cmd.single_token_word,
            )
            cmd_idx += 1 + consumed

    def analyse(
        self,
        source: str,
        cu: CompilationUnit | None = None,
        *,
        file_path: str | None = None,
    ) -> AnalysisResult:
        """Analyse a full source string."""
        self._source = source
        self._file_path = file_path
        self._unresolved_commands_emitted = False
        self._ns_cache.clear()
        # Pre-scan for inline stubs blocks (independent of Tcl parsing).
        cmd_stubs, expr_stubs = scan_source_for_stubs(source)
        self.result.stub_commands.extend(cmd_stubs)
        self.result.stub_expr_defs.extend(expr_stubs)
        self._check_stub_shadows()
        # Pre-scan for top-of-file ``# tcl-lsp: disable=...`` directives.
        # Stored in ``suppressed_lines`` under ``_FILE_SUPPRESS_KEY`` (-1) so
        # the same per-code filter logic handles both inline and file scopes.
        file_codes = parse_file_suppression(source)
        if file_codes:
            self.result.suppressed_lines[_FILE_SUPPRESS_KEY] = file_codes
        # Pre-scan for next-line noqa suppressions.  Handles orphaned noqa
        # comments at the tail of a brace body and noqa comments immediately
        # before a comment line that itself generates a diagnostic (e.g. W115).
        for ln, codes in parse_noqa_line_suppressions(source).items():
            existing = self.result.suppressed_lines.get(ln)
            if existing is not None:
                self.result.suppressed_lines[ln] = existing | codes
            else:
                self.result.suppressed_lines[ln] = codes
        self._analyse_body(source, self._current_scope)
        self._emit_unresolved_command_diagnostics(cu=cu)
        self._emit_variable_usage_diagnostics()
        self._emit_cfg_ssa_diagnostics(source, cu=cu)
        self._dedupe_diagnostics()
        return self.result

    def _check_stub_shadows(self) -> None:
        """Emit W116/W117 when stubs shadow built-in commands or expr defs."""
        dialect = active_dialect()
        builtin_commands = set(REGISTRY.command_names(dialect))

        for stub in self.result.stub_commands:
            normalised_stub = stub.name.lstrip(":")
            if normalised_stub in builtin_commands:
                self.result.diagnostics.append(
                    Diagnostic(
                        range=stub.range,
                        message=f"Stub command '{stub.name}' shadows built-in command.",
                        severity=Severity.WARNING,
                        code="W116",
                    )
                )

        builtin_expr = BUILTIN_MATH_FUNCTIONS | BUILTIN_EXPR_OPS
        if dialect == "f5-irules":
            builtin_expr = builtin_expr | IRULES_EXPR_OPS

        for stub_expr in self.result.stub_expr_defs:
            if stub_expr.name in builtin_expr:
                kind_label = "function" if stub_expr.kind == "function" else "operator"
                self.result.diagnostics.append(
                    Diagnostic(
                        range=stub_expr.range,
                        message=f"Stub expression {kind_label} '{stub_expr.name}' shadows built-in {kind_label}.",
                        severity=Severity.WARNING,
                        code="W117",
                    )
                )

    def _analyse_body(
        self,
        source: str,
        scope: Scope,
        body_token: Token | None = None,
        implicit_appended_args: int = 0,
    ) -> None:
        """Analyse a Tcl body (top-level or inside braces).

        When *body_token* is provided, the lexer is created with base offsets
        so that all positions in the sub-parse map back to the original source.

        ``implicit_appended_args`` records how many arguments the runtime
        appends to the body when it is invoked as a command prefix
        (e.g. ``fileutil::updateInPlace`` appends the file contents).
        Arity checks on the immediate top-level command of the body are
        relaxed by this many positional arguments; nested bodies do not
        inherit the offset.
        """
        if body_token is not None:
            self._body_depth += 1
        try:
            self._analyse_body_inner(source, scope, body_token, implicit_appended_args)
        finally:
            if body_token is not None:
                self._body_depth -= 1

    def _analyse_body_inner(
        self,
        source: str,
        scope: Scope,
        body_token: Token | None = None,
        implicit_appended_args: int = 0,
    ) -> None:
        """Inner implementation of _analyse_body, separated for try/finally."""
        commands, recovery_diags = segment_with_recovery(source, body_token)
        self.result.diagnostics.extend(recovery_diags)
        cmd_idx = 0
        while cmd_idx < len(commands):
            cmd = commands[cmd_idx]
            if cmd.preceding_comment is not None:
                self._last_comment = cmd.preceding_comment
                # Inline suppression: bare or code-specific form.
                comment_lower = cmd.preceding_comment.lower()
                noqa_pos = comment_lower.find("noqa")
                if noqa_pos >= 0:
                    rest = cmd.preceding_comment[noqa_pos + 4 :].strip()
                    if rest.startswith(":"):
                        codes = frozenset(c.strip() for c in rest[1:].split(",") if c.strip())
                    else:
                        codes = _NOQA_ALL
                    for ln in range(cmd.range.start.line, cmd.range.end.line + 1):
                        existing = self.result.suppressed_lines.get(ln)
                        if existing:
                            self.result.suppressed_lines[ln] = existing | codes
                        else:
                            self.result.suppressed_lines[ln] = codes
            if cmd.is_partial:
                # Try to detect which inner '{' stole the closing '}'.
                # If found, emit E103 instead of the generic E200.
                if (
                    cmd.partial_delimiter is UnclosedDelimiter.BRACE
                    and self._detect_stolen_close_brace(cmd, source, body_token)
                ):
                    cmd_idx += 1
                    continue
                _DELIMITER_MSG = {
                    UnclosedDelimiter.BRACE: "missing close-brace",
                    UnclosedDelimiter.BRACKET: "missing close-bracket",
                    UnclosedDelimiter.QUOTE: 'missing "',
                }
                msg = (
                    _DELIMITER_MSG.get(cmd.partial_delimiter, "missing close-brace")
                    if cmd.partial_delimiter is not None
                    else "missing close-brace"
                )
                self.result.diagnostics.append(
                    Diagnostic(
                        range=cmd.range,
                        severity=Severity.ERROR,
                        code="E200",
                        message=msg,
                    )
                )
                cmd_idx += 1
                continue
            # Recover from stray ']' (missing '[') — merge tokens into
            # a virtual CMD so downstream handlers see correct arg structure.
            self._recover_stray_close_bracket(cmd, scope)
            # Recover from missing '{' on switch — merge subsequent
            # pattern/body pair commands into a synthetic body argument.
            consumed = self._recover_missing_open_brace(
                cmd,
                commands,
                cmd_idx,
                scope,
                source,
                body_token,
            )
            # Process VAR reads and CMD substitutions inline.
            for tok in cmd.all_tokens:
                if tok.type is TokenType.VAR:
                    self._record_var_read(tok.text, range_from_token(tok), scope)
                elif tok.type is TokenType.CMD:
                    self._analyse_body(tok.text, scope, body_token=tok)
            # Apply command-prefix implicit-args offset to the *first*
            # top-level command of this body only (subsequent commands
            # would not be reachable under command-prefix semantics, so
            # we don't relax their arity).
            self._pending_implicit_args = implicit_appended_args
            implicit_appended_args = 0
            try:
                self._process_command(
                    cmd.argv,
                    cmd.texts,
                    cmd.all_tokens,
                    scope,
                    source,
                    expand_word=cmd.expand_word,
                    single_token_word=cmd.single_token_word,
                )
            finally:
                self._pending_implicit_args = 0
            cmd_idx += 1 + consumed

    def _analyse_expr(
        self,
        expr: str,
        scope: Scope,
        expr_token: Token | None = None,
    ) -> None:
        """Analyse a Tcl expression argument."""
        if not expr:
            return

        if expr_token is not None and expr_token.type in (TokenType.STR, TokenType.CMD):
            base_offset = expr_token.start.offset + 1
            base_line = expr_token.start.line
            base_col = expr_token.start.character + 1
        elif expr_token is not None:
            base_offset = expr_token.start.offset
            base_line = expr_token.start.line
            base_col = expr_token.start.character
        else:
            base_offset = 0
            base_line = 0
            base_col = 0

        for tok in tokenise_expr(expr, dialect=active_dialect()):
            if tok.type is ExprTokenType.VARIABLE:
                start = position_from_relative(
                    expr,
                    tok.start,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                end = position_from_relative(
                    expr,
                    tok.end,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                self._record_var_read(tok.text, Range(start=start, end=end), scope)
                continue

            if tok.type is ExprTokenType.COMMAND and len(tok.text) >= 2:
                cmd_text = tok.text[1:-1]
                if expr_token is None:
                    self._analyse_body(cmd_text, scope)
                else:
                    cmd_start = position_from_relative(
                        expr,
                        tok.start,
                        base_line=base_line,
                        base_col=base_col,
                        base_offset=base_offset,
                    )
                    cmd_end = position_from_relative(
                        expr,
                        tok.end,
                        base_line=base_line,
                        base_col=base_col,
                        base_offset=base_offset,
                    )
                    synthetic = Token(
                        type=TokenType.CMD,
                        text=cmd_text,
                        start=cmd_start,
                        end=cmd_end,
                    )
                    self._analyse_body(cmd_text, scope, body_token=synthetic)

    def _dedupe_diagnostics(self) -> None:
        """Drop exact duplicate diagnostics emitted by multiple passes."""
        seen: set[tuple[str, int, int, str, Severity]] = set()
        deduped: list[Diagnostic] = []
        # Collect lines where E101 fires — E002 on the same line is redundant
        # because E101 already explains the missing '{' and recovers the switch.
        e101_lines: set[int] = set()
        # Collect lines where W124 (SSA IP check) fires — W122 (regex IP
        # check) on the same line is redundant.
        w124_lines: set[int] = set()
        for d in self.result.diagnostics:
            if d.code == "E101":
                e101_lines.add(d.range.start.line)
            elif d.code == "W124":
                w124_lines.add(d.range.start.line)
        for d in self.result.diagnostics:
            key = (
                d.code,
                d.range.start.offset,
                d.range.end.offset,
                d.message,
                d.severity,
            )
            if key in seen:
                continue
            # E002 (too few args) on a switch line where E101 already fired
            # is a false positive — the analyser recovered the switch args.
            if d.code == "E002" and d.range.start.line in e101_lines:
                continue
            # W122 (regex IP check) on a line where W124 (SSA IP check) fired
            # is redundant — the SSA check is more precise.
            if d.code == "W122" and d.range.start.line in w124_lines:
                continue
            seen.add(key)
            deduped.append(d)
        self.result.diagnostics = deduped
