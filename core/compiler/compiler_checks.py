"""Compiler-driven wrapper for command-level checks.

This module runs existing command checks using command spans discovered from
the lowered IR, then recursively follows nested command substitutions/bodies.

It also implements IR-based arity checking (E001–E003, W001, W002) and
proc-call arity validation.
"""

# canonicalisation: audited #246

from __future__ import annotations

import logging
from collections.abc import Mapping

from shared.codes import diag
from shared.dialect import active_dialect
from shared.naming import normalise_qualified_name
from shared.ranges import position_from_relative, range_from_token
from shared.text import suggest_similar as _suggest_similar_impl

from ..analysis.checks import run_all_checks
from ..analysis.semantic_model import Diagnostic, Range, Severity
from ..commands.registry import REGISTRY
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    Arity,
    CommandSig,
    SubcommandSig,
    arg_indices_for_role,
    iter_body_arguments,
)
from ..parsing.argv import widen_argv_tokens_to_word_spans
from ..parsing.expr_lexer import ExprTokenType, tokenise_expr
from ..parsing.lexer import TclLexer
from ..parsing.tokens import Token, TokenType
from .ir import (
    CommandTokens,
    IRBarrier,
    IRBlock,
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

# Module-level registrations for codes emitted from nested/inline code.
diag("E004", "Invalid argument count.", section="error", internal=True)
diag(
    code="W302",
    description="`catch` without result variable — errors are silently swallowed.",
    section="security",
    ai_category="style",
)


def iter_ir_statements(script: IRScript):
    """Yield every IR statement, recursing into structured bodies."""
    for stmt in script.statements:
        yield stmt

        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from iter_ir_statements(clause.body)
            if stmt.else_body is not None:
                yield from iter_ir_statements(stmt.else_body)
            continue

        if isinstance(stmt, IRFor):
            yield from iter_ir_statements(stmt.init)
            yield from iter_ir_statements(stmt.body)
            yield from iter_ir_statements(stmt.next)
            continue

        if isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    yield from iter_ir_statements(arm.body)
            if stmt.default_body is not None:
                yield from iter_ir_statements(stmt.default_body)
            continue

        if isinstance(stmt, IRWhile):
            yield from iter_ir_statements(stmt.body)
            continue

        if isinstance(stmt, IRForeach):
            yield from iter_ir_statements(stmt.body)
            continue

        if isinstance(stmt, IRCatch):
            yield from iter_ir_statements(stmt.body)
            continue

        if isinstance(stmt, IRTry):
            yield from iter_ir_statements(stmt.body)
            for handler in stmt.handlers:
                yield from iter_ir_statements(handler.body)
            if stmt.finally_body is not None:
                yield from iter_ir_statements(stmt.finally_body)


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
        user_procs: Mapping[str, int] | None = None,
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
        self._user_procs: Mapping[str, int] = user_procs if user_procs is not None else {}

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
                user_procs=self._user_procs,
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
                    user_procs=self._user_procs,
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
    """Look up the command signature from the registry.

    Prefers the pre-built ``SIGNATURES`` dict which carries the full
    ``CommandSig`` (including ``leading_options`` for option-aware arity).
    Falls back to raw ``REGISTRY.validation`` only when SIGNATURES has no
    entry.
    """
    # Pre-built signature has the richest metadata (leading_options, etc.).
    cached = SIGNATURES.get(cmd_name)
    if cached is not None:
        return cached

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
    return None


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
        if isinstance(stmt, IRBarrier) and stmt.canonical_command == "::if":
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

        # Extract {*} expansion markers for the argument words (position 0
        # in ``expand_word`` is the command name, 1..n are the arguments).
        # When expansion is present, arity checks must treat the expanded
        # word as an unknown count rather than a single positional arg.
        arg_expand: list[bool] | None = None
        arg_tokens: list[Token] | None = None
        arg_single: list[bool] | None = None
        if ct is not None and ct.expand_word is not None:
            # If the command name itself is expanded ({*}$dispatch), we
            # cannot resolve the actual command — skip arity checks.
            if ct.expand_word and ct.expand_word[0]:
                return
            arg_expand = list(ct.expand_word[1:])
            if ct.argv:
                arg_tokens = list(ct.argv[1:])
            if ct.single_token_word:
                arg_single = list(ct.single_token_word[1:])

        # Built-in command arity checking
        sig = _resolve_signature(cmd_name)
        if sig is not None:
            _check_arity(
                cmd_name,
                args,
                sig,
                cmd_token_range,
                diagnostics,
                arg_expand,
                arg_tokens,
                arg_single,
            )

    def _walk_ir(script: IRScript) -> None:
        for stmt in iter_ir_statements(script):
            _check_statement(stmt)

    _walk_ir(ir_module.top_level)
    for proc in ir_module.procedures.values():
        _walk_ir(proc.body)

    return diagnostics


@diag("E001", "Missing subcommand — e.g. bare `string` without a subcommand.", section="error")
@diag("W001", "Unknown subcommand.", section="warning")
def _check_arity(
    cmd_name: str,
    args: list[str],
    sig: CommandSig | SubcommandSig,
    diag_range: Range,
    diagnostics: list[Diagnostic],
    arg_expand: list[bool] | None = None,
    arg_tokens: list[Token] | None = None,
    arg_single: list[bool] | None = None,
) -> None:
    """Check argument count against a command signature.

    ``arg_expand`` is a list parallel to ``args`` whose entries are
    ``True`` for arguments preceded by the Tcl 8.5+ ``{*}`` expansion
    prefix.  Expanded words contribute an unknown number of runtime
    arguments and must not be counted as a single positional arg.

    ``arg_tokens`` and ``arg_single`` (when provided) carry the original
    token and the single-token-word marker for each argument so that
    literal-list expansions can be statically resolved to an exact
    element count.
    """
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
        # If the subcommand position itself is {*}-expanded, the actual
        # subcommand is unknown — skip subcommand resolution and arity.
        if arg_expand and arg_expand[0]:
            return
        # In the IR, $var and ${var} forms represent real substitutions
        # (braced literals are resolved during lowering).  A $ or [ in
        # an IR arg always indicates a dynamic value that cannot be
        # resolved statically — skip the unknown-subcommand check.
        if "$" in sub_name or "[" in sub_name:
            return
        sub_sig = sig.subcommands.get(sub_name)
        if sub_sig is None:
            if sig.allow_unknown:
                return
            msg = f"Unknown subcommand '{sub_name}' for '{cmd_name}'"
            suggestions = _suggest_similar_impl(
                sub_name, sig.subcommands, max_suggestions=3, max_distance=3
            )
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
        sub_expand = arg_expand[1:] if arg_expand else None
        sub_tokens = arg_tokens[1:] if arg_tokens else None
        sub_single = arg_single[1:] if arg_single else None
        _check_simple_arity(
            f"{cmd_name} {sub_name}",
            sub_args,
            sub_sig,
            diag_range,
            diagnostics,
            sub_expand,
            sub_tokens,
            sub_single,
        )
        return

    _check_simple_arity(
        cmd_name, args, sig, diag_range, diagnostics, arg_expand, arg_tokens, arg_single
    )


def _resolve_expansion_elements(
    arg_text: str,
    tok: Token | None,
    single_token: bool,
) -> list[str] | None:
    """Return the static element list of a ``{*}``-expanded IR argument word.

    The IR text alone is ambiguous — for the literal expansion
    ``{*}{$x}`` the segmenter strips the braces, leaving the IR arg
    text ``$x``, which is indistinguishable from a variable
    substitution.  Disambiguation requires the original token type and
    the single-token-word marker:

    - **STR token, single-token word** — the word came from a braced
      literal ``{...}``.  The IR text is the brace contents (which may
      legitimately contain ``$`` or ``[``); split it directly with
      :func:`_split_tcl_list`.  An empty word resolves to zero elements.
    - **CMD token, single-token word, ``[list ...]`` form** — the
      command substitution is a literal ``list`` call; reuse
      :func:`_extract_foreach_elements` to fold it.
    - **Concatenated word, variable substitution, command substitution
      with non-list head, etc.** — return ``None``: the count is not
      statically known and the caller treats the expansion as
      contributing 0..∞ runtime arguments, matching Tcl's runtime
      semantics where ``{*}`` calls ``Tcl_ListObjGetElements`` to
      shimmer the value to a list at call time.
    """
    from ..parsing.tokens import TokenType
    from .core_analyses import _extract_foreach_elements
    from .tcl_expr_eval import _split_tcl_list

    if not single_token or tok is None:
        return None
    if tok.type is TokenType.STR:
        # Empty braced literal ``{*}{}`` → zero elements (the segmenter
        # strips the braces, so the IR text is empty).
        if not arg_text:
            return []
        try:
            return _split_tcl_list(arg_text)
        except Exception:
            return None
    if tok.type is TokenType.CMD:
        # Only refine when the command substitution is a literal list,
        # i.e. ``[list a b c]``.  ``_extract_foreach_elements`` handles
        # the bracketed form for us.
        elements = _extract_foreach_elements(arg_text)
        if elements is not None:
            return elements
    return None


def _resolve_expansion_count(
    arg_text: str,
    tok: Token | None,
    single_token: bool,
) -> int | None:
    """Convenience wrapper around :func:`_resolve_expansion_elements`."""
    elements = _resolve_expansion_elements(arg_text, tok, single_token)
    return None if elements is None else len(elements)


@diag("E002", "Too few arguments for command.", section="error")
@diag("E003", "Too many arguments for command.", section="error")
def _check_simple_arity(
    display_name: str,
    args: list[str],
    sig: CommandSig,
    diag_range: Range,
    diagnostics: list[Diagnostic],
    arg_expand: list[bool] | None = None,
    arg_tokens: list[Token] | None = None,
    arg_single: list[bool] | None = None,
) -> None:
    """Check argument count for a simple (non-subcommand) signature.

    When the signature declares ``leading_options``, leading arguments
    that match a declared option are skipped before counting positional
    arguments.  ``--`` is only recognised as an option terminator when the
    command explicitly declares it.  This lets ``puts -nonewline channel
    string`` (3 raw args) pass arity ``(1, 2)`` because only 2 are
    positional.

    ``arg_expand`` marks arguments preceded by ``{*}``.  Each expanded
    word may contribute zero or more runtime arguments; when the word is
    a statically-resolvable literal list the exact count is used, and
    otherwise the word contributes 0..∞ (so the upper bound becomes
    unbounded and the lower bound remains the count of non-expanded
    positional words).
    """
    # First, expand statically-resolvable ``{*}`` words inline so the
    # leading-option scan and the positional count both see the real
    # element list.  Words whose expansion count is unknown are kept as
    # a single placeholder entry whose ``flat_expand`` flag stays True.
    flat_args: list[str] = []
    flat_expand: list[bool] = []
    flat_tokens: list[Token | None] = []
    flat_single: list[bool] = []
    for i, word in enumerate(args):
        expanded = bool(arg_expand and i < len(arg_expand) and arg_expand[i])
        tok = arg_tokens[i] if arg_tokens and i < len(arg_tokens) else None
        single = bool(arg_single and i < len(arg_single) and arg_single[i])
        if expanded:
            elements = _resolve_expansion_elements(word, tok, single)
            if elements is not None:
                # Resolved literal expansion — inline each element so
                # leading_options scanning and positional counting both
                # see real string values, not the brace text.
                for element in elements:
                    flat_args.append(element)
                    flat_expand.append(False)
                    flat_tokens.append(None)
                    flat_single.append(True)
                continue
        flat_args.append(word)
        flat_expand.append(expanded)
        flat_tokens.append(tok)
        flat_single.append(single)

    # Count positional args by skipping leading declared options.
    # An unresolved expanded word can't be reliably classified as an
    # option, so option skipping stops at the first such word.
    positional_start = 0
    if sig.leading_options:
        for i, arg in enumerate(flat_args):
            if flat_expand[i]:
                break
            if arg in sig.leading_options:
                positional_start = i + 1
                if arg == "--":
                    break  # -- terminates option parsing
            else:
                break
    positional = flat_args[positional_start:]
    positional_expand = flat_expand[positional_start:]
    if any(positional_expand):
        # Lower bound: non-expanded positional args.  Upper bound:
        # unbounded once any unresolved expansion appears.
        nargs_min = sum(1 for exp in positional_expand if not exp)
        nargs_max = Arity.ANY
    else:
        nargs_min = len(positional)
        nargs_max = len(positional)
    if nargs_max < sig.arity.min:
        diagnostics.append(
            Diagnostic(
                range=diag_range,
                message=f"Too few arguments for '{display_name}': expected at least {sig.arity.min}, got {nargs_max}",
                severity=Severity.ERROR,
                code="E002",
            )
        )
    elif nargs_min > sig.arity.max:
        diagnostics.append(
            Diagnostic(
                range=diag_range,
                message=f"Too many arguments for '{display_name}': expected at most {sig.arity.max}, got {nargs_min}",
                severity=Severity.ERROR,
                code="E003",
            )
        )


def _qualify_proc_name(proc_name: str, namespace: str) -> str:
    """Qualify a ``proc`` name against its enclosing namespace.

    Mirrors the lowerer's ``_qualify_proc_name`` — an absolute name
    (``::foo``) is returned as-is; a relative name is joined onto the
    enclosing namespace (``::`` at top level, ``::ns`` inside
    ``namespace eval ::ns { ... }``).
    """
    if proc_name.startswith("::"):
        return normalise_qualified_name(proc_name)
    if namespace == "::":
        return normalise_qualified_name(f"::{proc_name}")
    return normalise_qualified_name(f"{namespace}::{proc_name}")


def _collect_unconditional_top_level_procs(ir_module: IRModule) -> dict[str, int]:
    """Map qualified proc name → earliest unconditional top-level offset.

    Walks ``ir_module.top_level`` and recurses into ``namespace eval``
    bodies (``IRBlock``) which run unconditionally. Skips conditional and
    looping constructs (``IRIf``/``IRWhile``/``IRFor``/``IRForeach``/
    ``IRSwitch``/``IRCatch``/``IRTry``) and does not recurse into proc
    bodies — a proc defined inside any of those is not guaranteed to
    exist at an arbitrary call site, so W002 keeps its warning.

    Each entry records the source offset of the defining ``proc``
    command; W002 compares the call-site offset against this value to
    decide whether the definition is statically known to precede the
    call.
    """
    offsets: dict[str, int] = {}

    def _walk(script: IRScript, namespace: str) -> None:
        for stmt in script.statements:
            if isinstance(stmt, IRBlock):
                _walk(stmt.body, stmt.namespace)
                continue
            if (
                isinstance(stmt, (IRCall, IRBarrier))
                and stmt.canonical_command == "::proc"
                and stmt.args
            ):
                qualified = _qualify_proc_name(stmt.args[0], namespace)
                if qualified and qualified not in offsets:
                    offsets[qualified] = stmt.range.start.offset

    _walk(ir_module.top_level, "::")
    return offsets


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
    stmts.extend(iter_ir_statements(ir_module.top_level))
    for proc in ir_module.procedures.values():
        stmts.extend(iter_ir_statements(proc.body))
    stmts.sort(key=lambda s: (s.range.start.offset, s.range.end.offset))

    # Map qualified name → earliest unconditional top-level definition
    # offset for W002. Restricting to unconditional definitions preserves
    # the warning for call sites that would precede a (possibly
    # conditional) proc at runtime.
    user_procs = _collect_unconditional_top_level_procs(ir_module)

    runner = _CompilerCheckRunner(source, user_procs=user_procs)
    for stmt in stmts:
        runner.process_statement(stmt)

    diagnostics = runner.diagnostics
    diagnostics.extend(_arity_checks(ir_module))
    return diagnostics
