"""Single-pass Tcl analyser.

Walks the token stream from TclLexer, builds a semantic model of the source:
scopes, proc definitions, variable definitions, and emits diagnostics for
detectable errors.
"""

from __future__ import annotations

import copy
import logging
import re
from dataclasses import dataclass, field

from ..commands.registry import REGISTRY
from ..commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    arg_indices_for_role,
    iter_body_arguments,
)
from ..commands.registry.signatures import Arity
from ..common.dialect import active_dialect
from ..common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ..common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ..common.ranges import position_from_relative, range_from_token
from ..compiler.cfg import CFGBranch, CFGFunction
from ..compiler.compilation_unit import CompilationUnit, ensure_compilation_unit
from ..compiler.compiler_checks import run_compiler_checks
from ..compiler.core_analyses import FunctionAnalysis
from ..compiler.ir import IRAssignConst, IRAssignValue, IRCall, IRProcedure, IRStatement
from ..parsing.argv import widen_argv_tokens_to_word_spans
from ..parsing.command_segmenter import SegmentedCommand, UnclosedDelimiter
from ..parsing.expr_lexer import ExprTokenType, tokenise_expr
from ..parsing.known_commands import known_command_names
from ..parsing.lexer import TclLexer
from ..parsing.recovery import segment_with_recovery
from ..parsing.tokens import SourcePosition, Token, TokenType
from .semantic_model import (
    AnalysisResult,
    CodeFix,
    CommandInvocation,
    Diagnostic,
    PackageProvide,
    PackageRequire,
    ParamDef,
    ProcDef,
    Range,
    RegexPattern,
    Scope,
    Severity,
    VarDef,
)

log = logging.getLogger(__name__)

# Short names: d = Diagnostic, m = regex Match, r = Range,
# s = Scope, t = Token, p = ParamDef.


_UNUSED_VAR_RE = re.compile(r"Variable '([^']+)' is set but never used\Z")

# Inline suppression via Tcl comments: bare form suppresses all diagnostics,
# ``# <noqa>: W100,W101`` suppresses specific codes on the following command.
_NOQA_ALL = frozenset({"*"})  # sentinel for "suppress everything"


def _possible_paste_fingerprint(stmt: IRStatement) -> tuple[str, str] | None:
    """Return (variable, static_value) for static assignments worth heuristic checks."""
    if isinstance(stmt, IRAssignConst):
        return (stmt.name, stmt.value.strip())

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        if not value:
            return None
        if "$" in value or "[" in value or "]" in value:
            return None
        return (stmt.name, value)

    return None


def _format_literal_for_message(value: str) -> str:
    """Keep heuristic diagnostic literals short and single-line."""
    display = value.replace("\n", "\\n")
    if len(display) > 40:
        return display[:37] + "..."
    return display


def _argv_with_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each Tcl word's full token span."""
    return widen_argv_tokens_to_word_spans(argv, all_tokens)


def _parse_param_list(param_str: str) -> list[ParamDef]:
    """Parse a Tcl proc argument list string into ParamDef objects.

    Handles:  "a b c"  and  "a {b default} c"
    """
    params: list[ParamDef] = []
    # Simple word-level parse -- braced items contain defaults
    i = 0
    text = param_str.strip()
    while i < len(text):
        # skip whitespace
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break

        if text[i] == "{":
            # Braced parameter with possible default
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
            parts = inner.split(None, 1)
            if len(parts) == 2:
                params.append(ParamDef(name=parts[0], has_default=True, default_value=parts[1]))
            elif len(parts) == 1:
                params.append(ParamDef(name=parts[0]))
        else:
            # Bare word
            start = i
            while i < len(text) and text[i] not in " \t\n\r":
                i += 1
            word = text[start:i]
            if word:
                params.append(ParamDef(name=word))

    return params


@dataclass
class AnalyserSnapshot:
    """Checkpoint of ``Analyser`` state at a chunk boundary.

    Captures the cumulative ``AnalysisResult`` and internal tracking
    state so that the analyser can resume from a checkpoint when only
    later chunks have changed.  The scope tree is deep-copied so that
    the snapshot is fully independent of live analyser state.
    """

    result: AnalysisResult
    last_comment: str
    const_strings: dict[int, dict[str, tuple[str, Range]]]
    regex_vars: set[tuple[int, str]]
    current_event: str | None
    conditional_depth: int = 0
    # Map from old scope id → scope object for scope identity reconstruction.
    scope_id_map: dict[int, Scope] = field(default_factory=dict)


class Analyser:
    """Analyses Tcl source and produces an AnalysisResult."""

    def __init__(self) -> None:
        self.result = AnalysisResult()
        self._current_scope = self.result.global_scope
        self._last_comment: str = ""
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
        # Track conditional nesting depth for package require confidence.
        self._conditional_depth: int = 0

    def snapshot(self) -> AnalyserSnapshot:
        """Capture the analyser state for later restoration.

        The ``AnalysisResult`` and scope tree are deep-copied so that
        the snapshot is fully independent.  ``_const_strings`` and
        ``_regex_vars`` use ``id(scope)`` as keys — the ``scope_id_map``
        records the mapping from the *live* scope ids to the *copied*
        scope objects so ``restore`` can remap them.
        """
        result_copy = copy.deepcopy(self.result)
        # Build a mapping from old scope id → new (copied) scope object.
        # The deep copy preserves structure; we walk both trees in parallel.
        scope_id_map: dict[int, Scope] = {}
        self._build_scope_id_map(self.result.global_scope, result_copy.global_scope, scope_id_map)

        return AnalyserSnapshot(
            result=result_copy,
            last_comment=self._last_comment,
            const_strings=copy.deepcopy(self._const_strings),
            regex_vars=set(self._regex_vars),
            current_event=self._current_event,
            conditional_depth=self._conditional_depth,
            scope_id_map=scope_id_map,
        )

    def restore(self, snap: AnalyserSnapshot) -> None:
        """Restore analyser state from a snapshot.

        The ``AnalysisResult`` is deep-copied from the snapshot so that
        the snapshot can be reused across multiple incremental passes.
        ``_const_strings`` and ``_regex_vars`` keys (which are scope ids)
        are remapped to the new scope objects' ids.
        """
        self.result = copy.deepcopy(snap.result)
        self._current_scope = self.result.global_scope
        self._last_comment = snap.last_comment
        self._current_event = snap.current_event
        self._conditional_depth = snap.conditional_depth

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
            self._const_strings[new_sid] = copy.deepcopy(entries)

        # Remap _regex_vars
        self._regex_vars = set()
        for old_sid, var_name in snap.regex_vars:
            new_sid = id_remap.get(old_sid, old_sid)
            self._regex_vars.add((new_sid, var_name))

    @staticmethod
    def _build_scope_id_map(
        old_scope: Scope,
        new_scope: Scope,
        mapping: dict[int, Scope],
    ) -> None:
        """Walk old and new scope trees in parallel, recording id mappings."""
        mapping[id(old_scope)] = new_scope
        for old_child, new_child in zip(old_scope.children, new_scope.children):
            Analyser._build_scope_id_map(old_child, new_child, mapping)

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
        self._analyse_commands_inner(commands, self._current_scope, source)
        if finalise:
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
            self._process_command(cmd.argv, cmd.texts, cmd.all_tokens, scope, source)
            cmd_idx += 1 + consumed

    def _set_const_string(
        self, var_name: str, value: str, value_range: Range, scope: Scope
    ) -> None:
        """Record a constant string assignment for *var_name* in *scope*."""
        sid = id(scope)
        if sid not in self._const_strings:
            self._const_strings[sid] = {}
        self._const_strings[sid][var_name] = (value, value_range)

    def _clear_const_string(self, var_name: str, scope: Scope) -> None:
        """Remove constant value knowledge for *var_name* (re-assigned dynamically)."""
        sid = id(scope)
        scope_dict = self._const_strings.get(sid)
        if scope_dict is not None:
            scope_dict.pop(var_name, None)

    def _lookup_const_string(self, var_name: str, scope: Scope) -> str | None:
        """Look up a constant string value for *var_name*, searching up the scope chain."""
        s: Scope | None = scope
        while s is not None:
            scope_dict = self._const_strings.get(id(s))
            if scope_dict is not None and var_name in scope_dict:
                return scope_dict[var_name][0]
            s = s.parent
        return None

    def _record_defining_set_as_regex(self, var_name: str, scope: Scope, command: str) -> None:
        """Mark the definition site of *var_name* as a regex pattern.

        Searches up the scope chain for the constant-string entry and, if
        found, adds a :class:`RegexPattern` pointing at the *value* range
        (not the variable name) so the semantic token provider can highlight
        the literal value as a regex.
        """
        s: Scope | None = scope
        while s is not None:
            scope_dict = self._const_strings.get(id(s))
            if scope_dict is not None and var_name in scope_dict:
                const_val, val_range = scope_dict[var_name]
                self.result.regex_patterns.append(
                    RegexPattern(
                        range=val_range,
                        pattern=const_val,
                        command=command,
                    )
                )
                return
            s = s.parent

    def analyse(
        self,
        source: str,
        cu: CompilationUnit | None = None,
    ) -> AnalysisResult:
        """Analyse a full source string."""
        self._source = source
        self._analyse_body(source, self._current_scope)
        self._emit_variable_usage_diagnostics()
        self._emit_cfg_ssa_diagnostics(source, cu=cu)
        self._dedupe_diagnostics()
        return self.result

    def _analyse_body(
        self,
        source: str,
        scope: Scope,
        body_token: Token | None = None,
    ) -> None:
        """Analyse a Tcl body (top-level or inside braces).

        When *body_token* is provided, the lexer is created with base offsets
        so that all positions in the sub-parse map back to the original source.
        """
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
            self._process_command(cmd.argv, cmd.texts, cmd.all_tokens, scope, source)
            cmd_idx += 1 + consumed

    def _recover_stray_close_bracket(
        self,
        cmd: SegmentedCommand,
        scope: Scope,
    ) -> None:
        """Merge tokens around a stray ``]`` into a virtual CMD for recovery.

        When ``[`` is missing but ``]`` is present (E100 case, e.g.
        ``switch ACCESS::policy agent_id] {...}``), the tokens from the
        first known-command-name argument through the ``]`` are merged
        into a virtual CMD token.  This lets ``_handle_switch`` detect
        the compact form and properly parse pattern/body pairs.
        """
        # Step 1: Find an ESC token containing ']' at its end.
        bracket_tok: Token | None = None
        bracket_tok_idx = -1
        bracket_char_idx = -1
        for ti, tok in enumerate(cmd.all_tokens):
            if tok.type is not TokenType.ESC:
                continue
            idx = tok.text.find("]")
            if idx >= 0 and idx == len(tok.text) - 1:
                bracket_tok = tok
                bracket_tok_idx = ti
                bracket_char_idx = idx
                break
        if bracket_tok is None:
            return

        # Step 2: Find the corresponding argv index.
        bracket_argv_idx = -1
        for ai, av in enumerate(cmd.argv):
            if av.start.offset == bracket_tok.start.offset:
                bracket_argv_idx = ai
                break
        if bracket_argv_idx <= 0:
            return  # not found, or is the command name itself

        # Step 3: Scan backward through all_tokens for a known command
        # name — that's where the missing '[' should have been.
        known = known_command_names()

        prefix = bracket_tok.text[:bracket_char_idx] if bracket_char_idx > 0 else ""
        cmd_start_all_idx: int | None = None
        cmd_start_argv_idx: int | None = None

        if prefix in known:
            cmd_start_all_idx = bracket_tok_idx
            cmd_start_argv_idx = bracket_argv_idx
        else:
            for i in range(bracket_tok_idx - 1, 0, -1):
                t = cmd.all_tokens[i]
                if t.type is TokenType.ESC and t.text in known:
                    cmd_start_all_idx = i
                    for ai, av in enumerate(cmd.argv):
                        if av.start.offset == t.start.offset:
                            cmd_start_argv_idx = ai
                            break
                    break

        # Arity-based fallback: if the enclosing command has bounded max
        # arity and the argument count exceeds it, the missing [ should
        # go before the last expected argument position.
        if cmd_start_all_idx is None or cmd_start_argv_idx is None:
            cmd_name = cmd.texts[0] if cmd.texts else ""
            validation = REGISTRY.validation(cmd_name)
            if validation is not None and not validation.arity.is_unlimited:
                max_args = validation.arity.max
                # argv includes cmd name, so excess starts at index max_args
                nargs = len(cmd.argv) - 1  # exclude command name
                if nargs > max_args >= 1:
                    target_argv_idx = max_args  # argv[max_args] = last expected arg
                    if target_argv_idx < len(cmd.argv):
                        target_tok = cmd.argv[target_argv_idx]
                        if target_tok.start.offset < bracket_tok.start.offset:
                            cmd_start_argv_idx = target_argv_idx
                            for j, t in enumerate(cmd.all_tokens):
                                if t.start.offset == target_tok.start.offset:
                                    cmd_start_all_idx = j
                                    break

        if cmd_start_all_idx is None or cmd_start_argv_idx is None:
            return
        if cmd_start_argv_idx <= 0:
            return  # don't merge the enclosing command name

        # Step 4: Extract virtual CMD text from the full source.
        src_start = cmd.argv[cmd_start_argv_idx].start.offset
        src_end = bracket_tok.start.offset + bracket_char_idx  # up to but not including ']'
        virtual_cmd_text = self._source[src_start:src_end]

        # Step 5: Build virtual CMD token.
        start_tok = cmd.argv[cmd_start_argv_idx]
        virtual_start = SourcePosition(
            line=start_tok.start.line,
            character=max(start_tok.start.character - 1, 0),  # virtual '['
            offset=src_start - 1,
        )
        virtual_end = SourcePosition(
            line=bracket_tok.start.line,
            character=bracket_tok.start.character + bracket_char_idx - 1,
            offset=bracket_tok.start.offset + bracket_char_idx - 1,
        )
        virtual_cmd = Token(
            type=TokenType.CMD,
            text=virtual_cmd_text,
            start=virtual_start,
            end=virtual_end,
        )

        # Step 6: Splice all_tokens.
        cmd.all_tokens[cmd_start_all_idx : bracket_tok_idx + 1] = [virtual_cmd]

        # Step 7: Splice argv / texts / single_token_word.
        cmd.argv[cmd_start_argv_idx : bracket_argv_idx + 1] = [virtual_cmd]
        cmd.texts[cmd_start_argv_idx : bracket_argv_idx + 1] = [f"[{virtual_cmd_text}]"]
        cmd.single_token_word[cmd_start_argv_idx : bracket_argv_idx + 1] = [True]

    # E101: Missing '{' on switch — merge subsequent case commands

    @staticmethod
    def _looks_like_switch_case(cmd: SegmentedCommand) -> bool:
        """Return True when *cmd* looks like a switch pattern/body pair.

        A switch case looks like ``pattern { body }`` or ``pattern -`` (fall-through).
        """
        if len(cmd.texts) != 2:
            return False
        known = known_command_names()
        if cmd.texts[0] in known:
            return False
        # Body must be brace-quoted (STR) or fall-through dash.
        last_tok = cmd.argv[-1] if cmd.argv else None
        if last_tok is not None and last_tok.type is TokenType.STR:
            return True
        if cmd.texts[-1] == "-":
            return True
        return False

    def _recover_missing_open_brace(
        self,
        cmd: SegmentedCommand,
        commands: list[SegmentedCommand],
        cmd_idx: int,
        scope: Scope,
        source: str,
        body_token: Token | None,
    ) -> int:
        """Detect and recover from a missing ``{`` on a switch command.

        Two cases arise depending on trailing whitespace after the switch
        string argument:

        **Case A** – no trailing space: the newline is an EOL so the switch
        gets only the string argument (too few args).  ALL pattern/body
        pairs become separate commands.

        **Case B** – trailing space: the lexer treats the space + newline
        as a SEP (continuation), so the first pattern/body pair merges
        into the switch as Form 1 args.  Only *subsequent* pairs are
        orphaned as separate commands.

        In either case, this method:

        1. Collects consecutive case-like commands after the switch.
        2. Extends ``cmd.argv``/``cmd.texts`` with the orphaned pairs so
           ``_handle_switch`` sees all pattern/body pairs.
        3. Emits E101 with a CodeFix to insert ``{``.

        Returns the number of commands consumed (0 if no recovery happened).
        """
        if cmd.name != "switch":
            return 0

        # Parse options to find where non-option args start.
        # Options: -exact, -glob, -regexp, -nocase, -matchvar, -indexvar, --
        arg_start = 0
        args = cmd.texts[1:]  # skip command name
        while arg_start < len(args) and args[arg_start].startswith("-"):
            if args[arg_start] == "--":
                arg_start += 1
                break
            # -matchvar / -indexvar consume the next arg as a value
            if args[arg_start] in ("-matchvar", "-indexvar"):
                arg_start += 2
                continue
            arg_start += 1

        non_option_args = args[arg_start:]

        # Determine which case we're in.
        # Case A: 0-1 non-option args → body completely missing
        # Case B: 2+ non-option args → Form 1 but possibly orphaned cases
        #
        # In Case B, we only fire if the switch's last arg is a STR
        # (brace-quoted body) — meaning Form 1 has pattern/body pairs —
        # AND the next command is an orphaned case.
        # For Form 2 (single brace body as last arg), the switch is fine.
        if len(non_option_args) >= 2:
            # Check if this is already Form 2 (compact form).
            # Form 2: the last non-option arg IS the entire body.
            # We detect this the same way _handle_switch does: last arg
            # is position len(args)-1 in the args list.
            last_arg_idx = len(args) - 1
            if last_arg_idx < len(cmd.argv) - 1:
                last_tok = cmd.argv[last_arg_idx + 1]  # +1 for cmd name
            else:
                last_tok = cmd.argv[-1]

            # If Form 2 (last arg is STR and it's the only non-option
            # arg after the string), no recovery needed.
            if (
                len(non_option_args) == 2
                and last_tok.type is TokenType.STR
                and last_arg_idx == arg_start + 1
            ):
                return 0  # Already Form 2 — valid switch

        # Check if the next command(s) look like switch cases.
        case_count = 0
        for j in range(cmd_idx + 1, len(commands)):
            if self._looks_like_switch_case(commands[j]):
                case_count += 1
            else:
                break

        if case_count == 0:
            return 0

        # Recovery: extend switch args with orphaned case pairs

        for k in range(case_count):
            orphan = commands[cmd_idx + 1 + k]
            # Each orphaned command has 2 texts: pattern and body.
            # Add them as additional argv entries to the switch.
            for ai, (text, tok, single) in enumerate(
                zip(
                    orphan.texts,
                    orphan.argv,
                    orphan.single_token_word,
                )
            ):
                cmd.argv.append(tok)
                cmd.texts.append(text)
                cmd.single_token_word.append(single)
                cmd.all_tokens.append(tok)

        # Determine where to point the diagnostic and CodeFix.
        # The '{' should go after the switch string argument.
        # arg_start is the index in args (0-based, after cmd name) of
        # the string arg.  In cmd.argv, that's index arg_start + 1.
        string_arg_idx = arg_start + 1  # index in cmd.argv
        if string_arg_idx < len(cmd.argv):
            string_tok = cmd.argv[string_arg_idx]
            diag_pos = string_tok.end
        else:
            diag_pos = cmd.range.end

        insert_pos = SourcePosition(
            line=diag_pos.line,
            character=diag_pos.character + 1,
            offset=diag_pos.offset + 1,
        )
        insert_end = SourcePosition(
            line=insert_pos.line,
            character=insert_pos.character - 1,
            offset=insert_pos.offset - 1,
        )
        self.result.diagnostics.append(
            Diagnostic(
                range=Range(start=diag_pos, end=diag_pos),
                severity=Severity.ERROR,
                code="E101",
                message="Missing '{' after switch — body cases follow without braces",
                fixes=(
                    CodeFix(
                        range=Range(start=insert_pos, end=insert_end),
                        new_text=" {",
                        description="Insert missing '{'",
                    ),
                ),
            )
        )

        return case_count

    def _detect_stolen_close_brace(
        self,
        cmd: SegmentedCommand,
        source: str,
        body_token: Token | None,
    ) -> bool:
        """Detect when an inner ``{`` stole the enclosing scope's closing ``}``.

        When a ``}`` is missing from a nested body (e.g. the switch body inside
        a ``when`` body), brace counting causes the enclosing scope's ``}`` to
        close the inner scope instead, leaving the outer scope unclosed.

        A stack-based scan of the body text identifies the inner ``{`` that
        "stole" the final ``}``.  If detected, emits **E103** with a CodeFix
        to insert the missing ``}`` and returns *True* so the caller skips
        the generic E200.
        """
        # Find the unclosed body STR token — the last STR in argv.
        body_tok: Token | None = None
        for tok in reversed(cmd.argv):
            if tok.type is TokenType.STR:
                body_tok = tok
                break
        if body_tok is None:
            return False

        text = body_tok.text
        if not text:
            return False

        # Stack-based brace scan.
        # Skip backslash-escaped characters (including \{ and \}).
        stack: list[int] = []  # offsets of unmatched '{' in body text
        last_pop: tuple[int, int] | None = None  # (open_offset, close_offset)
        i = 0
        while i < len(text):
            ch = text[i]
            if ch == "\\" and i + 1 < len(text):
                i += 2  # skip escaped-char pair
                continue
            if ch == "{":
                stack.append(i)
            elif ch == "}":
                if not stack:
                    # More closes than opens — can't determine stolen brace.
                    return False
                open_off = stack.pop()
                last_pop = (open_off, i)
            i += 1

        # Body text braces must be balanced for the "stolen brace" pattern.
        if stack or last_pop is None:
            return False

        open_offset, close_offset = last_pop

        # The stolen '}' must be the last significant content in the body.
        # If there's non-whitespace after it, the '}' legitimately closed
        # an inner scope and the enclosing '}' is genuinely missing.
        if text[close_offset + 1 :].strip():
            return False

        # Compute indentation from the line containing the inner '{'.
        line_start = text.rfind("\n", 0, open_offset)
        line_start = line_start + 1 if line_start >= 0 else 0
        indent = ""
        for c in text[line_start:]:
            if c in (" ", "\t"):
                indent += c
            else:
                break

        # Map body-text offsets to absolute source positions.
        # body_tok.start already points to the first char *after* the '{'.
        base_line = body_tok.start.line
        base_col = body_tok.start.character
        base_offset = body_tok.start.offset

        stolen_pos = position_from_relative(
            text,
            close_offset,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        stolen_end = position_from_relative(
            text,
            close_offset + 1,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )

        # Insertion point: start of the line containing the stolen '}'.
        stolen_line_start = text.rfind("\n", 0, close_offset)
        if stolen_line_start >= 0:
            insert_off = stolen_line_start + 1
        else:
            insert_off = close_offset

        insert_pos = position_from_relative(
            text,
            insert_off,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )

        self.result.diagnostics.append(
            Diagnostic(
                range=Range(start=stolen_pos, end=stolen_end),
                severity=Severity.ERROR,
                code="E103",
                message=("Missing '}' — a nested body consumed this closing brace"),
                fixes=(
                    CodeFix(
                        range=Range(start=insert_pos, end=insert_pos),
                        new_text=f"{indent}}}\n",
                        description="Insert missing '}'",
                    ),
                ),
            )
        )
        return True

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

    def _signature_for_command(self, cmd_name: str) -> CommandSig | SubcommandSig | None:
        """Resolve signature metadata from the command registry."""
        dialect = active_dialect()
        spec = REGISTRY.get(cmd_name, dialect)
        if spec is not None and spec.subcommands:
            dialect_subs = spec.subcommands_for_dialect(dialect)
            return SubcommandSig(
                subcommands={
                    name: CommandSig(
                        arity=sub.arity,
                        arg_roles=dict(sub.arg_roles) if sub.arg_roles else {},
                    )
                    for name, sub in dialect_subs.items()
                },
                allow_unknown=spec.allow_unknown_subcommands,
            )
        validation = REGISTRY.validation(cmd_name, dialect)
        if validation is not None:
            return CommandSig(arity=validation.arity)
        return SIGNATURES.get(cmd_name)

    def _process_command(
        self,
        argv: list[Token],
        argv_texts: list[str],
        all_tokens: list[Token],
        scope: Scope,
        source: str,
    ) -> None:
        """Process a single command (argv[0] is the command name)."""
        if not argv:
            return

        argv = _argv_with_word_spans(argv, all_tokens)
        cmd_name = argv_texts[0]
        args = argv_texts[1:]
        arg_tokens = argv[1:]
        resolved_proc = self._resolve_proc_call(cmd_name, scope)
        self.result.command_invocations.append(
            CommandInvocation(
                name=cmd_name,
                range=range_from_token(argv[0]),
                resolved_qualified_name=resolved_proc.qualified_name if resolved_proc else None,
            )
        )

        # IRULE5005: direct proc invocation without ``call`` in iRules.
        # In iRules, procs must be invoked via ``call proc_name``, not
        # directly as ``proc_name arg...``.
        if (
            resolved_proc is not None
            and cmd_name != "call"
            and self._current_event is not None
            and active_dialect() == "f5-irules"
        ):
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(argv[0]),
                    message=(
                        f"iRules procs must be invoked with 'call': "
                        f"call {cmd_name}" + ((" " + " ".join(args)) if args else "")
                    ),
                    severity=Severity.ERROR,
                    code="IRULE5005",
                    fixes=(
                        CodeFix(
                            range=range_from_token(argv[0]),
                            new_text=f"call {cmd_name}",
                            description=f"Use 'call {cmd_name}'",
                        ),
                    ),
                )
            )

        # iRules ``call proc_name arg...`` — record an additional
        # CommandInvocation for the target proc so that references,
        # rename, and call-hierarchy see through the indirection.
        if cmd_name == "call" and args and arg_tokens and active_dialect() == "f5-irules":
            call_target_name = args[0]
            call_target_proc = self._resolve_proc_call(call_target_name, scope)
            self.result.command_invocations.append(
                CommandInvocation(
                    name=call_target_name,
                    range=range_from_token(arg_tokens[0]),
                    resolved_qualified_name=(
                        call_target_proc.qualified_name if call_target_proc else None
                    ),
                )
            )

        # Record package require/provide statements for cross-file resolution.
        if cmd_name == "package" and args:
            if args[0] == "require":
                pkg_name = args[1] if len(args) >= 2 else None
                if pkg_name:
                    # Strip leading -exact flag if present
                    pkg_version: str | None = None
                    if pkg_name == "-exact" and len(args) >= 3:
                        pkg_name = args[2]
                        pkg_version = args[3] if len(args) >= 4 else None
                    else:
                        pkg_version = args[2] if len(args) >= 3 else None
                    self.result.package_requires.append(
                        PackageRequire(
                            name=pkg_name,
                            version=pkg_version,
                            range=range_from_token(argv[0]),
                            conditional=self._conditional_depth > 0,
                        )
                    )
            elif args[0] == "provide":
                pkg_name = args[1] if len(args) >= 2 else None
                if pkg_name:
                    pkg_version = args[2] if len(args) >= 3 else None
                    self.result.package_provides.append(
                        PackageProvide(
                            name=pkg_name,
                            version=pkg_version,
                            range=range_from_token(argv[0]),
                        )
                    )

        # Detect dynamic providers: load command or auto_path manipulation.
        if cmd_name == "load":
            self.result.has_dynamic_providers = True
        elif cmd_name == "set" and args and args[0] == "auto_path":
            self.result.has_dynamic_providers = True

        # W002 and built-in arity (E001-E003, W001) are now emitted by
        # the IR-based _arity_checks in compiler_checks.py.

        if self._run_command_special_cases(cmd_name, args, arg_tokens, argv[0], scope):
            return

        resolved_sig = self._signature_for_command(cmd_name)

        # Arity checking for user-defined procs (needs ProcDef.params
        # with has_default info, which the IR doesn't carry).
        if resolved_sig is None:
            proc_def = self._resolve_proc_call(cmd_name, scope)
            if proc_def is not None:
                self._check_proc_call_arity(proc_def, args, argv[0])

        # iRules ``call``: arity-check the target proc against the
        # arguments *after* the proc name (args[1:], not args).
        if cmd_name == "call" and args and arg_tokens and active_dialect() == "f5-irules":
            call_target_proc = self._resolve_proc_call(args[0], scope)
            if call_target_proc is not None:
                self._check_proc_call_arity(
                    call_target_proc,
                    args[1:],
                    arg_tokens[0],
                )

        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.EXPR)):
            expr_tok = arg_tokens[idx] if idx < len(arg_tokens) else None
            self._analyse_expr(args[idx], scope, expr_token=expr_tok)

        # Recurse into body arguments and capture var-name args.
        # For ``when EVENT { body }``, set event context while analysing.
        # For ``if``, bodies are conditional so increment depth for package
        # confidence tracking.
        prev_event = self._current_event
        if cmd_name == "when" and args:
            self._current_event = args[0]
        is_conditional = cmd_name in ("if", "try")
        if is_conditional:
            self._conditional_depth += 1
        for body in iter_body_arguments(cmd_name, args, arg_tokens):
            self._analyse_body(body.text, scope, body_token=body.token)
        if is_conditional:
            self._conditional_depth -= 1
        if cmd_name == "when":
            self._current_event = prev_event

        if cmd_name not in ("set", "variable", "global", "incr"):
            for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.VAR_NAME)):
                tok = arg_tokens[idx] if idx < len(arg_tokens) else argv[0]
                self._define_var(args[idx], tok, scope, warn_if_unused=False)

        # Record regex pattern arguments (regexp, regsub).
        # When the pattern position is a variable reference, look up its
        # constant value and also mark the defining `set` as a regex pattern.
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.PATTERN)):
            if idx >= len(arg_tokens):
                continue
            pat_tok = arg_tokens[idx]
            if pat_tok.type is TokenType.VAR:
                # Pattern is a variable — resolve to constant value if possible
                var_name = pat_tok.text
                const_val = self._lookup_const_string(var_name, scope)
                if const_val is not None:
                    self.result.regex_patterns.append(
                        RegexPattern(
                            range=range_from_token(pat_tok),
                            pattern=const_val,
                            command=cmd_name,
                        )
                    )
                    self._regex_vars.add((id(scope), var_name))
                    # Also mark the original definition site as a regex pattern
                    self._record_defining_set_as_regex(var_name, scope, cmd_name)
            else:
                self.result.regex_patterns.append(
                    RegexPattern(
                        range=range_from_token(pat_tok),
                        pattern=args[idx],
                        command=cmd_name,
                    )
                )

    def _run_command_special_cases(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        """Apply command-specific analyser behaviour.

        Returns True when the command was fully handled and generic role-based
        processing should stop.
        """
        if self._handle_proc_command(cmd_name, args, arg_tokens, scope):
            return True

        self._handle_set_command(cmd_name, args, arg_tokens, scope)
        self._handle_var_declaration_command(cmd_name, args, arg_tokens, scope)

        if self._handle_namespace_eval_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_foreach_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_for_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_switch_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True
        if self._handle_catch_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True
        if self._handle_try_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True

        self._handle_incr_command(cmd_name, args, arg_tokens, scope)
        return False

    def _handle_proc_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "proc" or len(args) < 3:
            return False
        self._handle_proc(args, arg_tokens, scope)
        return True

    def _handle_set_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name != "set" or not args:
            return

        self._handle_set(args, arg_tokens, scope)

        # Track constant string values for regex variable propagation.
        if len(args) < 2 or len(arg_tokens) < 2:
            return
        value_token = arg_tokens[1]
        if value_token.type in (TokenType.ESC, TokenType.STR) and value_token.text == args[1]:
            self._set_const_string(args[0], args[1], range_from_token(value_token), scope)
            return
        self._clear_const_string(args[0], scope)

    def _handle_var_declaration_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name not in ("variable", "global") or not args:
            return

        if cmd_name == "global":
            for i, arg_text in enumerate(args):
                if i < len(arg_tokens):
                    self._define_var(arg_text, arg_tokens[i], scope, warn_if_unused=False)
            return

        i = 0
        while i < len(args):
            if i < len(arg_tokens):
                self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
            if i + 1 < len(args):
                i += 2
            else:
                i += 1

    def _handle_namespace_eval_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "namespace" or len(args) < 2 or args[0] != "eval":
            return False

        ns_name = args[1]
        ns_scope = Scope(
            kind="namespace",
            name=ns_name,
            parent=scope,
            body_range=range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else None,
        )
        scope.children.append(ns_scope)
        if len(args) >= 3:
            body_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
            self._analyse_body(args[2], ns_scope, body_token=body_tok)
        return True

    def _handle_foreach_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "foreach" or len(args) < 3:
            return False

        tok = arg_tokens[0] if arg_tokens else None
        if tok:
            self._define_vars_from_list(args[0], tok, scope)

        body_tok = arg_tokens[-1] if arg_tokens else None
        self._analyse_body(args[-1], scope, body_token=body_tok)
        return True

    def _define_vars_from_list(
        self,
        var_list_text: str,
        tok: Token,
        scope: Scope,
    ) -> None:
        """Define variables from a varList token, giving each its own range."""
        text = tok.text
        span = tok.end.offset - tok.start.offset + 1
        content_offset = 1 if span > len(text) else 0
        base_line = tok.start.line
        base_col = tok.start.character + content_offset
        base_off = tok.start.offset + content_offset

        search_start = 0
        for var_name in var_list_text.split():
            if not var_name:
                continue
            idx = text.find(var_name, search_start)
            if idx >= 0:
                start_pos = position_from_relative(
                    text,
                    idx,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_off,
                )
                end_pos = position_from_relative(
                    text,
                    idx + len(var_name) - 1,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_off,
                )
                self._define_var(
                    var_name,
                    tok,
                    scope,
                    warn_if_unused=True,
                    definition_range=Range(start=start_pos, end=end_pos),
                )
                search_start = idx + len(var_name)
            else:
                self._define_var(var_name, tok, scope, warn_if_unused=True)

    def _handle_for_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "for" or len(args) < 4:
            return False

        init_tok = arg_tokens[0] if len(arg_tokens) > 0 else None
        test_tok = arg_tokens[1] if len(arg_tokens) > 1 else None
        next_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
        body_tok = arg_tokens[3] if len(arg_tokens) > 3 else None
        self._analyse_body(args[0], scope, body_token=init_tok)
        self._analyse_expr(args[1], scope, expr_token=test_tok)
        self._analyse_body(args[2], scope, body_token=next_tok)
        self._analyse_body(args[3], scope, body_token=body_tok)
        return True

    def _handle_switch_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "switch" or len(args) < 2:
            return False

        self._handle_switch(args, arg_tokens, scope)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_catch_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "catch" or not args:
            return False

        catch_body_tok = arg_tokens[0] if arg_tokens else None
        self._conditional_depth += 1
        self._analyse_body(args[0], scope, body_token=catch_body_tok)
        self._conditional_depth -= 1
        for i in range(1, min(len(args), 3)):
            if i < len(arg_tokens):
                self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_try_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "try" or not args:
            return False

        self._handle_try(args, arg_tokens, scope)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_incr_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name == "incr" and args and arg_tokens:
            self._define_var(args[0], arg_tokens[0], scope, warn_if_unused=True)

    def _handle_proc(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle a proc definition."""
        proc_name = args[0]
        param_str = args[1] if len(args) > 1 else ""
        body = args[2] if len(args) > 2 else ""

        params = _parse_param_list(param_str)

        # Determine qualified name
        if scope.kind == "namespace":
            qualified = f"::{scope.name}::{proc_name}"
        else:
            qualified = f"::{proc_name}"

        if not arg_tokens:
            return
        name_range = range_from_token(arg_tokens[0])
        body_range = range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else name_range

        # W113: warn when proc name shadows a built-in command.
        if self._builtin_names is None:
            self._builtin_names = frozenset(REGISTRY.command_names(active_dialect()))
        if proc_name in self._builtin_names:
            self.result.diagnostics.append(
                Diagnostic(
                    range=name_range,
                    message=f"Procedure '{proc_name}' shadows built-in command",
                    severity=Severity.WARNING,
                    code="W113",
                )
            )

        proc_def = ProcDef(
            name=proc_name,
            qualified_name=qualified,
            params=params,
            name_range=name_range,
            body_range=body_range,
            doc=self._last_comment,
        )
        self._last_comment = ""

        scope.procs[proc_name] = proc_def
        self.result.all_procs[qualified] = proc_def

        # Analyse the body in a new proc scope
        proc_scope = Scope(kind="proc", name=proc_name, parent=scope, body_range=body_range)
        scope.children.append(proc_scope)

        # Define parameters as variables in the proc scope
        for p in params:
            proc_scope.variables[p.name] = VarDef(
                name=p.name,
                definition_range=name_range,
                warn_if_unused=False,
            )

        body_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
        self._analyse_body(body, proc_scope, body_token=body_tok)

    def _handle_set(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle a set command -- defines/references a variable."""
        var_name = args[0]
        if len(args) >= 2:
            # This is a definition/assignment
            self._define_var(var_name, arg_tokens[0], scope, warn_if_unused=True)
        else:
            # set with one arg is a read -- record reference
            self._record_var_read(var_name, range_from_token(arg_tokens[0]), scope)

    def _handle_switch(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle switch command -- analyse pattern/body pairs.

        Switch has two forms:
          1. switch string pattern body ?pattern body ...?
          2. switch string { pattern body ?pattern body ...? }
        In form 2, the entire pattern/body block is a single braced string.

        When ``-regexp`` is among the option switches, pattern arguments are
        recorded as :class:`RegexPattern` instances in the analysis result.
        """
        # Scan options, detecting -regexp
        is_regexp = False
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "-regexp":
                is_regexp = True
            if args[i] == "--":
                i += 1
                break
            i += 1
        # Skip the string argument
        i += 1

        if i < len(args) and i == len(args) - 1:
            # Form 2: single braced body containing all pattern/body pairs
            # Re-lex the braced body to extract pairs
            body_text = args[i]
            body_tok = arg_tokens[i] if i < len(arg_tokens) else None
            self._parse_switch_body(body_text, body_tok, scope, is_regexp=is_regexp)
        else:
            # Form 1: remaining args are pattern/body pairs
            while i + 1 < len(args):
                # Record regex patterns from switch -regexp
                if is_regexp and args[i] != "default" and i < len(arg_tokens):
                    pat_tok = arg_tokens[i]
                    if pat_tok.type is TokenType.VAR:
                        const_val = self._lookup_const_string(pat_tok.text, scope)
                        if const_val is not None:
                            self.result.regex_patterns.append(
                                RegexPattern(
                                    range=range_from_token(pat_tok),
                                    pattern=const_val,
                                    command="switch",
                                )
                            )
                            self._regex_vars.add((id(scope), pat_tok.text))
                            self._record_defining_set_as_regex(pat_tok.text, scope, "switch")
                    else:
                        self.result.regex_patterns.append(
                            RegexPattern(
                                range=range_from_token(pat_tok),
                                pattern=args[i],
                                command="switch",
                            )
                        )
                body = args[i + 1]
                if body != "-":  # '-' means fall through
                    body_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                    self._analyse_body(body, scope, body_token=body_tok)
                i += 2

    def _parse_switch_body(
        self,
        body_text: str,
        outer_body_token: Token | None,
        scope: Scope,
        *,
        is_regexp: bool = False,
    ) -> None:
        """Parse the braced body of a switch command to extract pattern/body pairs."""
        # Create lexer with base offsets if we have the outer body token
        if outer_body_token is not None:
            lexer = TclLexer(
                body_text,
                base_offset=outer_body_token.start.offset + 1,
                base_line=outer_body_token.start.line,
                base_col=outer_body_token.start.character + 1,
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
            if tok.type in (TokenType.SEP, TokenType.EOL):
                prev_type = tok.type
                continue

            if prev_type in (TokenType.SEP, TokenType.EOL):
                elements.append(tok.text)
                element_tokens.append(tok)
            else:
                if elements:
                    elements[-1] += tok.text
                else:
                    elements.append(tok.text)
                    element_tokens.append(tok)
            prev_type = tok.type

        # elements should be alternating pattern/body pairs
        i = 0
        while i + 1 < len(elements):
            # Record regex patterns from switch -regexp
            if is_regexp and elements[i] != "default" and i < len(element_tokens):
                pat_tok = element_tokens[i]
                if pat_tok.type is TokenType.VAR:
                    const_val = self._lookup_const_string(pat_tok.text, scope)
                    if const_val is not None:
                        self.result.regex_patterns.append(
                            RegexPattern(
                                range=range_from_token(pat_tok),
                                pattern=const_val,
                                command="switch",
                            )
                        )
                        self._regex_vars.add((id(scope), pat_tok.text))
                        self._record_defining_set_as_regex(pat_tok.text, scope, "switch")
                else:
                    self.result.regex_patterns.append(
                        RegexPattern(
                            range=range_from_token(pat_tok),
                            pattern=elements[i],
                            command="switch",
                        )
                    )
            body = elements[i + 1]
            if body != "-":  # '-' means fall through
                body_tok = element_tokens[i + 1] if i + 1 < len(element_tokens) else None
                self._analyse_body(body, scope, body_token=body_tok)
            i += 2

    def _handle_try(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle try command -- analyse body and handler bodies."""
        if not args:
            return
        # First arg is the try body
        body_tok = arg_tokens[0] if arg_tokens else None
        self._analyse_body(args[0], scope, body_token=body_tok)
        # Scan for 'on', 'trap', 'finally' keywords
        i = 1
        while i < len(args):
            kw = args[i]
            if kw == "finally" and i + 1 < len(args):
                finally_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                self._analyse_body(args[i + 1], scope, body_token=finally_tok)
                i += 2
            elif kw in ("on", "trap") and i + 3 < len(args):
                # on code varList body  /  trap pattern varList body
                handler_tok = arg_tokens[i + 3] if i + 3 < len(arg_tokens) else None
                self._analyse_body(args[i + 3], scope, body_token=handler_tok)
                i += 4
            else:
                i += 1

    def _resolve_proc_call(self, cmd_name: str, scope: Scope) -> ProcDef | None:
        """Resolve a command name to a known proc definition, if any."""
        if not cmd_name:
            return None

        candidates: list[str] = []
        seen: set[str] = set()

        def add_candidate(name: str) -> None:
            qname = _normalise_qualified_name(name)
            if not qname or qname in seen:
                return
            seen.add(qname)
            candidates.append(qname)

        if cmd_name.startswith("::"):
            add_candidate(cmd_name)
        elif "::" in cmd_name:
            add_candidate(f"::{cmd_name}")
        else:
            current: Scope | None = scope
            while current is not None:
                if current.kind == "namespace":
                    add_candidate(f"{current.name}::{cmd_name}")
                current = current.parent
            add_candidate(f"::{cmd_name}")

        for qname in candidates:
            proc = self.result.all_procs.get(qname)
            if proc is not None:
                return proc
        return None

    def _check_proc_call_arity(
        self,
        proc_def: ProcDef,
        args: list[str],
        cmd_token: Token,
    ) -> None:
        """Check a proc call against the proc's parameter list."""
        required = 0
        variadic = False
        for i, param in enumerate(proc_def.params):
            if i == len(proc_def.params) - 1 and param.name == "args" and not param.has_default:
                variadic = True
                continue
            if not param.has_default:
                required += 1

        arity = Arity(required, Arity.ANY if variadic else len(proc_def.params))
        nargs = len(args)
        display_name = proc_def.qualified_name
        if nargs < arity.min:
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(cmd_token),
                    message=f"Too few arguments for '{display_name}': expected at least {arity.min}, got {nargs}",
                    severity=Severity.ERROR,
                    code="E002",
                )
            )
        elif nargs > arity.max:
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(cmd_token),
                    message=f"Too many arguments for '{display_name}': expected at most {arity.max}, got {nargs}",
                    severity=Severity.ERROR,
                    code="E003",
                )
            )

    def _record_var_read(self, name: str, read_range: Range, scope: Scope) -> None:
        """Record a variable read for go-to-definition / find-references."""
        base_name = _normalise_var_name(name)
        if not base_name:
            return

        if base_name in scope.variables:
            scope.variables[base_name].references.append(read_range)
            return

        # Cross-rule variables (::var and static::var) live in global scope.
        if (
            base_name.startswith("::") or base_name.startswith("static::")
        ) and base_name in self.result.global_scope.variables:
            self.result.global_scope.variables[base_name].references.append(read_range)
            return
        # W210 is now emitted by the SSA-based analysis in
        # _emit_cfg_ssa_diagnostics_for_function.

    def _walk_scopes(self, scope: Scope):
        yield scope
        for child in scope.children:
            yield from self._walk_scopes(child)

    def _emit_variable_usage_diagnostics(self) -> None:
        """Kept for potential future scope-tree consumers.

        W211 is now emitted by the SSA-based analysis in
        _emit_cfg_ssa_diagnostics_for_function.
        """

    def _emit_cfg_ssa_diagnostics(
        self,
        source: str,
        *,
        cu: CompilationUnit | None = None,
    ) -> None:
        """Emit diagnostics backed by CFG/SSA core analyses."""
        cu = ensure_compilation_unit(source, cu, logger=log, context="analyser")
        if cu is None:
            return

        ir_module = cu.ir_module
        self.result.diagnostics.extend(
            run_compiler_checks(source, ir_module=ir_module),
        )
        self._emit_cfg_ssa_diagnostics_for_function(
            cu.top_level.cfg,
            cu.top_level.analysis,
        )
        conn = cu.connection_scope
        for qname, fu in cu.procedures.items():
            cross_vars: frozenset[str] = frozenset()
            if conn is not None and qname.startswith("::when::"):
                cross_vars = conn.cross_event_defs | conn.cross_event_imports
            self._emit_cfg_ssa_diagnostics_for_function(
                fu.cfg,
                fu.analysis,
                cross_event_vars=cross_vars,
            )
            ir_proc = ir_module.procedures.get(qname)
            if ir_proc is not None:
                self._emit_unused_param_diagnostics(ir_proc, fu.analysis)

    def _emit_cfg_ssa_diagnostics_for_function(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
    ) -> None:
        self._emit_constant_branch_diagnostics(cfg, analysis)
        self._emit_dead_store_diagnostics(cfg, analysis, cross_event_vars=cross_event_vars)
        self._emit_possible_paste_error_diagnostics(cfg, analysis)
        self._emit_read_before_set_diagnostics(cfg, analysis, cross_event_vars=cross_event_vars)
        self._emit_unused_variable_diagnostics(cfg, analysis, cross_event_vars=cross_event_vars)

    def _emit_constant_branch_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        for branch in analysis.constant_branches:
            if branch.not_taken_target not in analysis.unreachable_blocks:
                continue
            block = cfg.blocks.get(branch.block)
            if block is None or not isinstance(block.terminator, CFGBranch):
                continue
            r = block.terminator.range
            if r is None:
                continue

            names = (branch.block, branch.taken_target, branch.not_taken_target)
            is_switch = any(name.startswith("switch_") for name in names)
            is_if = any(name.startswith("if_") for name in names)

            if is_switch:
                code = "I231"
                if branch.value:
                    msg = (
                        f"Switch condition '{branch.condition}' is always true here; "
                        "subsequent switch arms are unreachable"
                    )
                else:
                    msg = (
                        f"Switch arm condition '{branch.condition}' is always false; "
                        "this arm is unreachable"
                    )
            else:
                code = "I230"
                if branch.value:
                    msg = (
                        f"Condition '{branch.condition}' is always true; "
                        "the alternate branch is unreachable"
                    )
                else:
                    msg = (
                        f"Condition '{branch.condition}' is always false; "
                        "the alternate branch is unreachable"
                    )
                if not is_if:
                    msg = f"Branch condition '{branch.condition}' is constant; one branch is unreachable"

            self.result.diagnostics.append(
                Diagnostic(
                    range=r,
                    message=msg,
                    severity=Severity.INFO,
                    code=code,
                )
            )

    def _emit_dead_store_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
    ) -> None:
        existing_unused: set[tuple[str, int]] = set()
        for d in self.result.diagnostics:
            if d.code != "W211":
                continue
            m = _UNUSED_VAR_RE.match(d.message)
            if m is None:
                continue
            existing_unused.add((m.group(1), d.range.start.offset))

        for dead in analysis.dead_stores:
            if dead.variable in cross_event_vars:
                continue
            block = cfg.blocks.get(dead.block)
            if block is None:
                continue
            if dead.statement_index < 0 or dead.statement_index >= len(block.statements):
                continue
            stmt = block.statements[dead.statement_index]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            if (dead.variable, stmt_range.start.offset) in existing_unused:
                continue
            self.result.diagnostics.append(
                Diagnostic(
                    range=stmt_range,
                    message=f"Assignment to '{dead.variable}' is never read",
                    severity=Severity.HINT,
                    code="W220",
                )
            )

    def _emit_possible_paste_error_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        """Emit low-confidence duplicate-assignment paste-error heuristics."""
        dead_store_indices: dict[str, set[int]] = {}
        for dead in analysis.dead_stores:
            dead_store_indices.setdefault(dead.block, set()).add(dead.statement_index)

        for block_name, block in cfg.blocks.items():
            dead_indices = dead_store_indices.get(block_name)
            if not dead_indices:
                continue

            statements = block.statements
            for idx in range(len(statements) - 1):
                if idx not in dead_indices:
                    continue

                first = _possible_paste_fingerprint(statements[idx])
                if first is None:
                    continue

                second = _possible_paste_fingerprint(statements[idx + 1])
                if second is None or first != second:
                    continue

                var_name, literal = first
                if var_name.startswith("_"):
                    continue

                stmt_range = getattr(statements[idx + 1], "range", None)
                if stmt_range is None:
                    continue

                self.result.diagnostics.append(
                    Diagnostic(
                        range=stmt_range,
                        message=(
                            f"Possible paste error: repeated assignment to '{var_name}' "
                            f"with static value '{_format_literal_for_message(literal)}'; "
                            "did you mean to assign a different variable?"
                        ),
                        severity=Severity.HINT,
                        code="H300",
                    )
                )

    def _emit_read_before_set_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
    ) -> None:
        for rbs in analysis.read_before_set:
            block = cfg.blocks.get(rbs.block)
            if block is None:
                continue
            stmt = None
            if rbs.statement_index == -1:
                # Version-0 use in a branch condition — use terminator range.
                r = getattr(block.terminator, "range", None)
            elif 0 <= rbs.statement_index < len(block.statements):
                stmt = block.statements[rbs.statement_index]
                r = getattr(stmt, "range", None)
            else:
                continue
            if r is None:
                continue
            if isinstance(stmt, IRCall) and stmt.command == "unset":
                # unset without -nocomplain on a possibly-undefined variable.
                # Still warn even for cross-event vars — unset is explicit.
                self.result.diagnostics.append(
                    Diagnostic(
                        range=r,
                        message=(
                            f"Variable '{rbs.variable}' may not exist; "
                            "use 'unset -nocomplain' to suppress the error"
                        ),
                        severity=Severity.WARNING,
                        code="W213",
                    )
                )
            elif rbs.variable in cross_event_vars:
                # Variable is set in another event — not a real read-before-set.
                continue
            else:
                self.result.diagnostics.append(
                    Diagnostic(
                        range=r,
                        message=f"Variable '{rbs.variable}' is read before it is set",
                        severity=Severity.WARNING,
                        code="W210",
                    )
                )

    def _emit_unused_variable_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
    ) -> None:
        for unused in analysis.unused_variables:
            if unused.variable in cross_event_vars:
                continue
            block = cfg.blocks.get(unused.block)
            if block is None:
                continue
            if unused.statement_index < 0 or unused.statement_index >= len(block.statements):
                continue
            stmt = block.statements[unused.statement_index]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            self.result.diagnostics.append(
                Diagnostic(
                    range=stmt_range,
                    message=f"Variable '{unused.variable}' is set but never used",
                    severity=Severity.HINT,
                    code="W211",
                )
            )

    def _emit_unused_param_diagnostics(
        self,
        ir_proc: IRProcedure,
        analysis: FunctionAnalysis,
    ) -> None:
        """W214: flag proc parameters that are never read in the body."""
        for param_name in analysis.unused_params:
            self.result.diagnostics.append(
                Diagnostic(
                    range=ir_proc.range,
                    message=(f"Parameter '{param_name}' of proc '{ir_proc.name}' is unused"),
                    severity=Severity.HINT,
                    code="W214",
                )
            )

    def _dedupe_diagnostics(self) -> None:
        """Drop exact duplicate diagnostics emitted by multiple passes."""
        seen: set[tuple[str, int, int, str, Severity]] = set()
        deduped: list[Diagnostic] = []
        # Collect lines where E101 fires — E002 on the same line is redundant
        # because E101 already explains the missing '{' and recovers the switch.
        e101_lines: set[int] = set()
        for d in self.result.diagnostics:
            if d.code == "E101":
                e101_lines.add(d.range.start.line)
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
            seen.add(key)
            deduped.append(d)
        self.result.diagnostics = deduped

    def _define_var(
        self,
        name: str,
        tok: Token,
        scope: Scope,
        *,
        warn_if_unused: bool = False,
        definition_range: Range | None = None,
    ) -> None:
        """Record a variable definition in the current scope."""
        base_name = _normalise_var_name(name)
        if not base_name:
            return

        if base_name not in scope.variables:
            scope.variables[base_name] = VarDef(
                name=base_name,
                definition_range=definition_range or range_from_token(tok),
                warn_if_unused=warn_if_unused,
            )
            self.result.all_variables[f"{scope.name}::{base_name}"] = scope.variables[base_name]
            return

        if warn_if_unused:
            scope.variables[base_name].warn_if_unused = True


def analyse(
    source: str,
    cu: CompilationUnit | None = None,
) -> AnalysisResult:
    """Convenience function: analyse a Tcl source string."""
    return Analyser().analyse(source, cu=cu)
