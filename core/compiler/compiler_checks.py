"""Compiler-driven wrapper for command-level checks.

This module runs existing command checks using command spans discovered from
the lowered IR, then recursively follows nested command substitutions/bodies.

It also implements IR-based arity checking (E001–E003, W001, W002) and
proc-call arity validation.
"""

from __future__ import annotations

import logging

from ..analysis.checks import run_all_checks
from ..analysis.semantic_model import Diagnostic, Range, Severity
from ..commands.registry import REGISTRY
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    arg_indices_for_role,
    iter_body_arguments,
)
from ..common.dialect import active_dialect
from ..common.ranges import position_from_relative, range_from_token
from ..parsing.argv import widen_argv_tokens_to_word_spans
from ..parsing.expr_lexer import ExprTokenType, tokenise_expr
from ..parsing.lexer import TclLexer
from ..parsing.tokens import Token, TokenType
from .ir import (
    CommandTokens,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRModule,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .lowering import lower_to_ir

log = logging.getLogger(__name__)


def _edit_distance(a: str, b: str) -> int:
    """Compute Levenshtein edit distance between two strings."""
    if len(a) < len(b):
        return _edit_distance(b, a)
    if not b:
        return len(a)
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a):
        curr = [i + 1]
        for j, cb in enumerate(b):
            cost = 0 if ca == cb else 1
            curr.append(min(prev[j + 1] + 1, curr[j] + 1, prev[j] + cost))
        prev = curr
    return prev[-1]


def _suggest_subcommands(
    attempted: str,
    available: dict[str, CommandSig],
    max_suggestions: int = 3,
    max_distance: int = 3,
) -> list[str]:
    """Suggest similar subcommands ranked by edit distance."""
    if not available:
        return []
    candidates = sorted(
        ((name, _edit_distance(attempted, name)) for name in available),
        key=lambda x: x[1],
    )
    return [name for name, dist in candidates[:max_suggestions] if dist <= max_distance]


def _iter_statements(script: IRScript):
    """Yield every IR statement, recursing into structured bodies."""
    for stmt in script.statements:
        yield stmt

        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from _iter_statements(clause.body)
            if stmt.else_body is not None:
                yield from _iter_statements(stmt.else_body)
            continue

        if isinstance(stmt, IRFor):
            yield from _iter_statements(stmt.init)
            yield from _iter_statements(stmt.body)
            yield from _iter_statements(stmt.next)
            continue

        if isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    yield from _iter_statements(arm.body)
            if stmt.default_body is not None:
                yield from _iter_statements(stmt.default_body)
            continue

        if isinstance(stmt, IRWhile):
            yield from _iter_statements(stmt.body)
            continue

        if isinstance(stmt, IRForeach):
            yield from _iter_statements(stmt.body)
            continue

        if isinstance(stmt, IRCatch):
            yield from _iter_statements(stmt.body)
            continue

        if isinstance(stmt, IRTry):
            yield from _iter_statements(stmt.body)
            for handler in stmt.handlers:
                yield from _iter_statements(handler.body)
            if stmt.finally_body is not None:
                yield from _iter_statements(stmt.finally_body)


def _switch_list_body_index(args: list[str]) -> int | None:
    """Return the BODY index for ``switch string {pattern body ...}`` form."""
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "--":
            i += 1
            break
        i += 1
    if i >= len(args):
        return None
    i += 1  # switch string argument
    if i == len(args) - 1:
        return i
    return None


def _argv_with_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each word's full token span.

    ``argv`` carries one representative token per word (typically the first
    token).  For diagnostics we want each argument token range to cover the
    entire Tcl word, including variable/cmd substitutions and trailing pieces.
    """
    return widen_argv_tokens_to_word_spans(argv, all_tokens)


class _CompilerCheckRunner:
    def __init__(
        self,
        source: str,
        *,
        file_profiles: frozenset[str] | None = None,
    ) -> None:
        self._source = source
        self._seen_commands: set[tuple[int, int]] = set()
        self.diagnostics: list[Diagnostic] = []
        self._current_event: str | None = None
        self._file_profiles = (
            file_profiles
            if file_profiles is not None
            else EVENT_REGISTRY.compute_file_profiles(source)
        )

    def process_statement(self, stmt: IRStatement) -> None:
        """Process an IR statement, using carried tokens when available."""
        ct: CommandTokens | None = getattr(stmt, "tokens", None)
        if ct is not None and ct.all_tokens:
            self._process_tokens(
                list(ct.argv),
                list(ct.argv_texts),
                list(ct.all_tokens),
            )
        else:
            r = stmt.range
            start = r.start.offset
            end = r.end.offset
            if start < 0 or end < start or end >= len(self._source):
                return
            self._process_text(
                self._source[start : end + 1],
                base_offset=start,
                base_line=r.start.line,
                base_col=r.start.character,
            )

    def _process_tokens(
        self,
        argv: list[Token],
        argv_texts: list[str],
        all_tokens: list[Token],
    ) -> None:
        """Run checks using pre-parsed tokens from the IR."""
        if not argv or not all_tokens:
            return

        span = (all_tokens[0].start.offset, all_tokens[-1].end.offset)
        if span in self._seen_commands:
            return
        self._seen_commands.add(span)

        argv_spanned = _argv_with_word_spans(argv, all_tokens)
        cmd_name = argv_texts[0]
        args = argv_texts[1:]
        arg_tokens = argv_spanned[1:]

        self.diagnostics.extend(
            run_all_checks(
                cmd_name,
                args,
                arg_tokens,
                all_tokens,
                self._source,
                event=self._current_event,
                file_profiles=self._file_profiles,
            )
        )

        self._recurse_nested_commands(all_tokens)
        self._recurse_expression_subcommands(cmd_name, args, arg_tokens)
        self._recurse_body_arguments(cmd_name, args, arg_tokens)

    def _process_text(
        self,
        text: str,
        *,
        base_offset: int,
        base_line: int,
        base_col: int,
    ) -> None:
        lexer = TclLexer(
            text,
            base_offset=base_offset,
            base_line=base_line,
            base_col=base_col,
        )

        argv: list[Token] = []
        argv_texts: list[str] = []
        all_tokens: list[Token] = []
        prev_type = TokenType.EOL

        def flush_command() -> None:
            if not argv or not all_tokens:
                return

            span = (all_tokens[0].start.offset, all_tokens[-1].end.offset)
            if span in self._seen_commands:
                return
            self._seen_commands.add(span)

            argv_spanned = _argv_with_word_spans(argv, all_tokens)
            cmd_name = argv_texts[0]
            args = argv_texts[1:]
            arg_tokens = argv_spanned[1:]

            self.diagnostics.extend(
                run_all_checks(
                    cmd_name,
                    args,
                    arg_tokens,
                    all_tokens,
                    self._source,
                    event=self._current_event,
                    file_profiles=self._file_profiles,
                )
            )

            self._recurse_nested_commands(all_tokens)
            self._recurse_expression_subcommands(cmd_name, args, arg_tokens)
            self._recurse_body_arguments(cmd_name, args, arg_tokens)

        while True:
            tok = lexer.get_token()
            if tok is None:
                break

            match tok.type:
                case TokenType.COMMENT:
                    continue
                case TokenType.SEP:
                    prev_type = tok.type
                    continue
                case TokenType.EOL:
                    flush_command()
                    argv = []
                    argv_texts = []
                    all_tokens = []
                    prev_type = tok.type
                    continue
                case _:
                    text_piece = tok.text

            all_tokens.append(tok)

            if prev_type in (TokenType.SEP, TokenType.EOL):
                argv.append(tok)
                argv_texts.append(text_piece)
            else:
                if argv_texts:
                    argv_texts[-1] += text_piece
                else:
                    argv.append(tok)
                    argv_texts.append(text_piece)

            prev_type = tok.type

        flush_command()

    def _recurse_nested_commands(self, tokens: list[Token]) -> None:
        for tok in tokens:
            if tok.type is not TokenType.CMD or not tok.text:
                continue
            self._process_text(
                tok.text,
                base_offset=tok.start.offset + 1,
                base_line=tok.start.line,
                base_col=tok.start.character + 1,
            )

    def _recurse_body_arguments(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
    ) -> None:
        # For ``when EVENT { body }``, set event context while recursing.
        prev_event = self._current_event
        if cmd_name == "when" and args:
            self._current_event = args[0]
        for body in iter_body_arguments(cmd_name, args, arg_tokens):
            if body.token.type is not TokenType.STR:
                continue
            if not body.text.strip():
                continue
            # switch list-form body (`switch x {pattern body ...}`) is a Tcl
            # list, not a script. Parse pairs and recurse into each body arm.
            if cmd_name == "switch" and _switch_list_body_index(args) == body.index:
                self._recurse_switch_list_body(body.text, body.token)
                continue
            self._process_text(
                body.text,
                base_offset=body.token.start.offset + 1,
                base_line=body.token.start.line,
                base_col=body.token.start.character + 1,
            )
        if cmd_name == "when":
            self._current_event = prev_event

    def _recurse_expression_subcommands(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
    ) -> None:
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.EXPR)):
            if idx >= len(args) or idx >= len(arg_tokens):
                continue
            expr_text = args[idx]
            owner = arg_tokens[idx]

            if owner.type in (TokenType.STR, TokenType.CMD):
                base_offset = owner.start.offset + 1
                base_line = owner.start.line
                base_col = owner.start.character + 1
            else:
                base_offset = owner.start.offset
                base_line = owner.start.line
                base_col = owner.start.character

            for expr_tok in tokenise_expr(expr_text, dialect=active_dialect()):
                if expr_tok.type is not ExprTokenType.COMMAND or len(expr_tok.text) < 2:
                    continue

                cmd_text = expr_tok.text[1:-1]
                cmd_start = position_from_relative(
                    expr_text,
                    expr_tok.start,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                self._process_text(
                    cmd_text,
                    base_offset=cmd_start.offset + 1,
                    base_line=cmd_start.line,
                    base_col=cmd_start.character + 1,
                )

    def _recurse_switch_list_body(self, body_text: str, body_tok: Token) -> None:
        """Recurse into switch list-form arm bodies."""
        elements, element_tokens = self._lex_switch_elements(body_text, body_tok)
        i = 0
        while i + 1 < len(elements):
            body = elements[i + 1]
            tok = element_tokens[i + 1]
            i += 2
            if body == "-" or not body.strip():
                continue
            if tok.type in (TokenType.STR, TokenType.CMD):
                base_offset = tok.start.offset + 1
                base_line = tok.start.line
                base_col = tok.start.character + 1
            else:
                base_offset = tok.start.offset
                base_line = tok.start.line
                base_col = tok.start.character
            self._process_text(
                body,
                base_offset=base_offset,
                base_line=base_line,
                base_col=base_col,
            )

    def _lex_switch_elements(
        self,
        body_text: str,
        body_tok: Token,
    ) -> tuple[list[str], list[Token]]:
        """Lex switch list-form body into alternating pattern/body elements."""
        lexer = TclLexer(
            body_text,
            base_offset=body_tok.start.offset + 1,
            base_line=body_tok.start.line,
            base_col=body_tok.start.character + 1,
        )
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

        return elements, element_tokens


def _resolve_signature(cmd_name: str) -> CommandSig | SubcommandSig | None:
    """Look up the command signature from the registry."""
    dialect = active_dialect()
    spec = REGISTRY.get(cmd_name, dialect)
    if spec is not None and spec.subcommands:
        return SubcommandSig(
            subcommands={
                name: CommandSig(
                    arity=sub.arity,
                    arg_roles=dict(sub.arg_roles) if sub.arg_roles else {},
                )
                for name, sub in spec.subcommands.items()
            },
            allow_unknown=spec.allow_unknown_subcommands,
        )
    validation = REGISTRY.validation(cmd_name, dialect)
    if validation is not None:
        return CommandSig(arity=validation.arity)
    return SIGNATURES.get(cmd_name)


def _arity_checks(ir_module: IRModule) -> list[Diagnostic]:
    """IR-native checks: arity (E001–E003), unknown subcommands (W001), W302.

    W002/W003/W004/W307 are now in checks.py ALL_CHECKS so they fire for
    all command spans including nested commands in ``[...]``.

    Only checks built-in commands against registry signatures.  User-defined
    proc-call arity is checked by the analyser which has access to full
    parameter default information via ``ProcDef``.
    """
    diagnostics: list[Diagnostic] = []

    def _check_statement(stmt: IRStatement) -> None:
        # W302: catch without result variable (IR-native)
        if isinstance(stmt, IRCatch) and stmt.result_var is None:
            diagnostics.append(
                Diagnostic(
                    range=stmt.range,
                    message=(
                        "catch without a result variable silently swallows errors. "
                        "Consider capturing the result: catch {…} result"
                    ),
                    severity=Severity.HINT,
                    code="W302",
                )
            )

        # E004: malformed control-flow structures detected by lowering
        if isinstance(stmt, IRBarrier) and stmt.command == "if":
            if "extra words" in stmt.reason:
                diagnostics.append(
                    Diagnostic(
                        range=stmt.range,
                        message='Extra words after "else" clause in "if" command',
                        severity=Severity.ERROR,
                        code="E004",
                    )
                )
            elif "malformed" in stmt.reason:
                diagnostics.append(
                    Diagnostic(
                        range=stmt.range,
                        message="Malformed 'if' command",
                        severity=Severity.ERROR,
                        code="E004",
                    )
                )

        if not isinstance(stmt, (IRCall, IRBarrier)):
            return

        cmd_name = stmt.command
        ct: CommandTokens | None = getattr(stmt, "tokens", None)

        cmd_token_range = range_from_token(ct.argv[0]) if ct and ct.argv else stmt.range

        args = list(stmt.args) if isinstance(stmt, IRCall) else list(getattr(stmt, "args", ()))

        # Built-in command arity checking
        sig = _resolve_signature(cmd_name)
        if sig is not None:
            _check_arity(cmd_name, args, sig, cmd_token_range, diagnostics)

    def _walk_ir(script: IRScript) -> None:
        for stmt in _iter_statements(script):
            _check_statement(stmt)

    _walk_ir(ir_module.top_level)
    for proc in ir_module.procedures.values():
        _walk_ir(proc.body)

    return diagnostics


def _check_arity(
    cmd_name: str,
    args: list[str],
    sig: CommandSig | SubcommandSig,
    diag_range: Range,
    diagnostics: list[Diagnostic],
) -> None:
    """Check argument count against a command signature."""
    if isinstance(sig, SubcommandSig):
        if not args:
            diagnostics.append(
                Diagnostic(
                    range=diag_range,
                    message=f"'{cmd_name}' requires a subcommand",
                    severity=Severity.ERROR,
                    code="E001",
                )
            )
            return
        sub_name = args[0]
        sub_sig = sig.subcommands.get(sub_name)
        if sub_sig is None:
            if sig.allow_unknown:
                return
            msg = f"Unknown subcommand '{sub_name}' for '{cmd_name}'"
            suggestions = _suggest_subcommands(sub_name, sig.subcommands)
            if suggestions:
                msg += f"; did you mean '{suggestions[0]}'?"
            diagnostics.append(
                Diagnostic(
                    range=diag_range,
                    message=msg,
                    severity=Severity.WARNING,
                    code="W001",
                )
            )
            return
        # Check arity of subcommand (args after subcommand name)
        sub_args = args[1:]
        _check_simple_arity(f"{cmd_name} {sub_name}", sub_args, sub_sig, diag_range, diagnostics)
        return

    _check_simple_arity(cmd_name, args, sig, diag_range, diagnostics)


def _check_simple_arity(
    display_name: str,
    args: list[str],
    sig: CommandSig,
    diag_range: Range,
    diagnostics: list[Diagnostic],
) -> None:
    """Check argument count for a simple (non-subcommand) signature."""
    nargs = len(args)
    if nargs < sig.arity.min:
        diagnostics.append(
            Diagnostic(
                range=diag_range,
                message=f"Too few arguments for '{display_name}': expected at least {sig.arity.min}, got {nargs}",
                severity=Severity.ERROR,
                code="E002",
            )
        )
    elif nargs > sig.arity.max:
        diagnostics.append(
            Diagnostic(
                range=diag_range,
                message=f"Too many arguments for '{display_name}': expected at most {sig.arity.max}, got {nargs}",
                severity=Severity.ERROR,
                code="E003",
            )
        )


def run_compiler_checks(
    source: str,
    *,
    ir_module: IRModule | None = None,
) -> list[Diagnostic]:
    """Run command checks through compiler-discovered command ranges."""
    if ir_module is None:
        try:
            ir_module = lower_to_ir(source)
        except Exception:
            log.debug(
                "compiler_checks: IR lowering failed, skipping compiler checks", exc_info=True
            )
            return []

    stmts: list[IRStatement] = []
    stmts.extend(_iter_statements(ir_module.top_level))
    for proc in ir_module.procedures.values():
        stmts.extend(_iter_statements(proc.body))
    stmts.sort(key=lambda s: (s.range.start.offset, s.range.end.offset))

    runner = _CompilerCheckRunner(source)
    for stmt in stmts:
        runner.process_statement(stmt)

    diagnostics = runner.diagnostics
    diagnostics.extend(_arity_checks(ir_module))
    return diagnostics
