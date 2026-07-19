# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import logging

from compiler.parsing.expr_lexer import ExprTokenType, tokenise_expr
from compiler.parsing.known_commands import known_command_names
from compiler.parsing.recovering_lexer import tokenise_recovering
from compiler.parsing.syntax.red import build_line_starts
from compiler.parsing.token_positions import token_content_base, token_content_shift
from compiler.registry.command_registry import REGISTRY
from compiler.registry.dialect import active_dialect
from compiler.registry.runtime import (
    ArgRole,
    arg_indices_for_roles,
    iter_switch_case_list,
)
from shared.ranges import position_from_offset
from shared.tokens import SourcePosition, Token, TokenType

from ._constants import (
    _BUILTIN_COMMANDS,
    _EVENT_RE,
    _MOD_INDEX,
    _TYPE_INDEX,
)
from ._format_args import (
    _binary_format_arg_index,
    _clock_format_arg_index,
    _collect_binary_format_spec_tokens,
    _collect_clock_format_spec_tokens,
    _collect_glob_pattern_tokens,
    _collect_param_list_tokens,
    _collect_regsub_subspec_tokens,
    _collect_sprintf_format_spec_tokens,
    _collect_string_map_pairs_tokens,
    _emit_regex_token,
    _glob_pattern_arg_indices,
    _is_known_subcommand,
    _option_arg_indices,
    _proc_param_list_arg_index,
    _procedure_name_arg_index,
    _regsub_subspec_arg_index,
    _split_words,
    _sprintf_format_arg_index,
    _string_map_mapping_arg_index,
    _subcommand_arg_index,
)
from ._primitives import (
    _append_text_token,
    _classify_expr_token,
    _classify_token,
    _emit_namespace_qualified,
    _emit_string_with_escapes,
)

log = logging.getLogger(__name__)


def _collect_expression_tokens(
    out: list[tuple[int, int, int, int, int]],
    expr_text: str,
    owner_token: Token,
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> None:
    """Collect semantic tokens from an expression argument."""
    if owner_token.type is TokenType.STR:
        base_offset = owner_token.start.offset + 1
    else:
        base_offset = owner_token.start.offset

    prev_op_text = ""
    for expr_tok in tokenise_expr(expr_text, dialect=active_dialect()):
        if expr_tok.type is ExprTokenType.WHITESPACE:
            continue

        if expr_tok.type is ExprTokenType.COMMAND and len(expr_tok.text) >= 2:
            cmd_text = expr_tok.text[1:-1]
            cmd_start = position_from_offset(
                base_offset + expr_tok.start,
                line_starts,
                source_len,
            )
            cmd_end = position_from_offset(
                base_offset + expr_tok.end,
                line_starts,
                source_len,
            )
            synthetic = Token(type=TokenType.CMD, text=cmd_text, start=cmd_start, end=cmd_end)
            _collect_tokens(
                out,
                cmd_text,
                body_token=synthetic,
                regex_positions=regex_positions,
                _line_starts=line_starts,
                _source_len=source_len or None,
            )
            prev_op_text = ""
            continue

        # iRules matches_glob / matches_regex: highlight the RHS string
        if expr_tok.type is ExprTokenType.STRING and prev_op_text in (
            "matches_glob",
            "matches_regex",
        ):
            str_text = expr_tok.text
            # Strip surrounding delimiters (braces or quotes)
            if len(str_text) >= 2 and str_text[0] == "{" and str_text[-1] == "}":
                inner = str_text[1:-1]
                inner_offset = expr_tok.start + 1
            elif len(str_text) >= 2 and str_text[0] == '"' and str_text[-1] == '"':
                inner = str_text[1:-1]
                inner_offset = expr_tok.start + 1
            else:
                inner = str_text
                inner_offset = expr_tok.start

            inner_start = position_from_offset(
                base_offset + inner_offset,
                line_starts,
                source_len,
            )
            inner_end = position_from_offset(
                base_offset + inner_offset + len(inner) - 1,
                line_starts,
                source_len,
            )
            synthetic = Token(
                type=TokenType.STR,
                text=inner,
                start=inner_start,
                end=inner_end,
            )
            if prev_op_text == "matches_glob":
                if not _collect_glob_pattern_tokens(
                    out, synthetic, line_starts=line_starts, source_len=source_len
                ):
                    _append_text_token(
                        out,
                        start=inner_start,
                        text=inner,
                        type_idx=_TYPE_INDEX["string"],
                    )
            else:
                _emit_regex_token(out, synthetic, line_starts=line_starts, source_len=source_len)
            # Emit surrounding delimiters (quotes only; braces are syntax)
            if len(str_text) >= 2 and str_text[0] == '"':
                quote_start = position_from_offset(
                    base_offset + expr_tok.start,
                    line_starts,
                    source_len,
                )
                _append_text_token(
                    out,
                    start=quote_start,
                    text='"',
                    type_idx=_TYPE_INDEX["string"],
                )
                end_quote_start = position_from_offset(
                    base_offset + expr_tok.start + len(str_text) - 1,
                    line_starts,
                    source_len,
                )
                _append_text_token(
                    out,
                    start=end_quote_start,
                    text='"',
                    type_idx=_TYPE_INDEX["string"],
                )
            prev_op_text = ""
            continue

        if expr_tok.type is ExprTokenType.OPERATOR:
            prev_op_text = expr_tok.text
        else:
            prev_op_text = ""

        type_idx = _classify_expr_token(expr_tok.type, expr_tok.text)
        if type_idx is None:
            continue
        modifiers = 0
        if expr_tok.type is ExprTokenType.FUNCTION:
            modifiers = 1 << _MOD_INDEX["defaultLibrary"]
        start = position_from_offset(
            base_offset + expr_tok.start,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=start,
            text=expr_tok.text,
            type_idx=type_idx,
            modifiers=modifiers,
        )


def _collect_switch_case_bodies(
    out: list[tuple[int, int, int, int, int]],
    args: list[str],
    arg_tokens: list[Token],
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> set[int]:
    """Collect body tokens for switch braced case-list form.

    Returns argument indices (0-based after command name) whose generic BODY
    recursion should be skipped because they are handled here.

    When ``-regexp`` is among the option switches, pattern elements are
    emitted as ``regexp`` semantic tokens.
    """
    is_regexp = False
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-regexp":
            is_regexp = True
        if args[i] == "--":
            i += 1
            break
        i += 1

    if i >= len(args):
        return set()
    i += 1  # switch value/pattern source
    if i >= len(args):
        return set()

    # Non-braced form: pattern/body pairs as separate arguments.
    if i != len(args) - 1:
        # Emit regex tokens for patterns in inline form.
        # VAR tokens are skipped here — the analysis-driven regex_positions
        # override in the main token loop handles variable patterns.
        if is_regexp:
            j = i
            while j + 1 < len(args):
                if args[j] != "default" and j < len(arg_tokens):
                    tok = arg_tokens[j]
                    if tok.type is not TokenType.VAR:
                        _emit_regex_token(out, tok, line_starts=line_starts, source_len=source_len)
                j += 2
        return set()

    if i >= len(arg_tokens):
        return {i}

    case_list_tok = arg_tokens[i]
    if case_list_tok.type is not TokenType.STR:
        return set()

    case_offset, case_line, case_col = token_content_base(case_list_tok)

    for case in iter_switch_case_list(
        args[i],
        base_offset=case_offset,
        base_line=case_line,
        base_col=case_col,
    ):
        # Emit regex token for pattern in braced case list
        if is_regexp and case.pattern != "default":
            _emit_regex_token(
                out, case.pattern_token, line_starts=line_starts, source_len=source_len
            )
        if (
            case.body is not None
            and case.body_token is not None
            and case.body_token.type is TokenType.STR
            and case.body.strip()
        ):
            _collect_tokens(
                out,
                case.body,
                body_token=case.body_token,
                regex_positions=regex_positions,
                _line_starts=line_starts,
                _source_len=source_len or None,
            )

    return {i}


def _collect_apply_lambda(
    out: list[tuple[int, int, int, int, int]],
    args: list[str],
    arg_tokens: list[Token],
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> set[int]:
    """Collect tokens for the lambda argument of ``apply``.

    ``apply {argList body ?namespace?} ?arg ...?`` takes a lambda as its first
    argument — a braced list whose first element is a proc-style parameter list
    and whose second element is a script body.  Neither element is a whole
    argument, so the generic ``ArgRole.BODY`` recursion (which treats a whole
    braced argument as a script) cannot reach the body.  This mirrors
    :func:`_collect_switch_case_bodies`: parse the braced lambda, emit its
    parameter list, and recurse into its body as a nested script so commands
    inside it are highlighted (#954).

    Returns ``{0}`` (the lambda argument index) so the caller skips the generic
    handling of that argument, or an empty set when the lambda is not a braced
    literal (e.g. ``apply $lambdaVar``), in which case it is left untouched.
    """
    if not arg_tokens:
        return set()
    lambda_tok = arg_tokens[0]
    if lambda_tok.type is not TokenType.STR:
        # ``apply $var`` / ``apply [cmd]`` — the lambda is not a literal, so it
        # cannot be split for highlighting.
        return set()

    base_offset, base_line, base_col = token_content_base(lambda_tok)
    words, word_tokens = _split_words(
        lambda_tok.text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )
    if not word_tokens:
        return {0}

    # Element 0 — the formal parameter list (``dir`` or ``{a {b 5} args}``).
    _collect_param_list_tokens(out, word_tokens[0])

    # Element 1 — the body script.  Recurse only when it is a non-empty braced
    # word; a bare or substituted body word is left for the generic handling.
    if len(word_tokens) >= 2:
        body_tok = word_tokens[1]
        if body_tok.type is TokenType.STR and body_tok.text.strip():
            _collect_tokens(
                out,
                body_tok.text,
                body_token=body_tok,
                regex_positions=regex_positions,
                _line_starts=line_starts,
                _source_len=source_len or None,
            )

    return {0}


def _recover_stray_close_bracket_in_flush(
    argv: list[Token],
    argv_texts: list[str],
    all_tokens_buf: list[Token],
    source: str,
    body_token: Token | None,
) -> None:
    """Merge tokens around a stray ``]`` into a virtual CMD for recovery.

    Mirrors the analyser's ``_recover_stray_close_bracket`` so that the
    semantic token provider sees the same argument structure.  A ``]``
    inside a double-quoted string (e.g. ``foo "bar]"``) is a literal
    character, not a stray bracket — quoted-context ESCs are skipped.
    """
    from compiler.parsing.token_positions import classify_quoted_contexts

    base_off = (body_token.start.offset + 1) if body_token else 0

    # Compute quoted-context flags on the full token stream, then look up
    # each argv token by object identity.  We must NOT key the dict by
    # ``tok.start.offset`` because multiple tokens can legitimately share
    # the same start offset — notably the zero-width synthetic SEP
    # injected at the iRules ``}{`` word boundary, which sits at the same
    # offset as the following STR/ESC token.  Object identity is safe
    # because ``argv`` and ``all_tokens_buf`` hold the same ``Token``
    # instances (see the flush loop below).
    in_quoted_by_id = {
        id(tok): flag for tok, flag in zip(all_tokens_buf, classify_quoted_contexts(all_tokens_buf))
    }

    # Step 1: Find an ESC token in argv containing ']' at its end.
    bracket_argv_idx = -1
    bracket_char_idx = -1
    for i, tok in enumerate(argv):
        if tok.type is not TokenType.ESC:
            continue
        if in_quoted_by_id.get(id(tok), False):
            continue
        idx = tok.text.find("]")
        if idx >= 0 and idx == len(tok.text) - 1:
            bracket_argv_idx = i
            bracket_char_idx = idx
            break
    if bracket_argv_idx <= 0:
        return  # not found, or is command name

    bracket_tok = argv[bracket_argv_idx]

    # Step 2: Scan backward through argv for a known command name.
    known = _known_commands_set()
    prefix = bracket_tok.text[:bracket_char_idx] if bracket_char_idx > 0 else ""

    cmd_start_argv_idx: int | None = None
    if prefix in known:
        cmd_start_argv_idx = bracket_argv_idx
    else:
        for i in range(bracket_argv_idx - 1, 0, -1):
            if argv[i].type is TokenType.ESC and argv[i].text in known:
                cmd_start_argv_idx = i
                break

    # Arity-based fallback: if the enclosing command has bounded max
    # arity and the argument count exceeds it, the missing [ should
    # go before the last expected argument position.
    if cmd_start_argv_idx is None or cmd_start_argv_idx <= 0:
        cmd_name = argv_texts[0] if argv_texts else ""
        validation = REGISTRY.validation(cmd_name)
        if validation is not None and not validation.arity.is_unlimited:
            max_args = validation.arity.max
            nargs = len(argv) - 1  # exclude command name
            if nargs > max_args >= 1:
                target_argv_idx = max_args  # argv[max_args] = last expected arg
                if target_argv_idx < len(argv):
                    target_tok = argv[target_argv_idx]
                    if target_tok.start.offset < bracket_tok.start.offset:
                        cmd_start_argv_idx = target_argv_idx

    if cmd_start_argv_idx is None or cmd_start_argv_idx <= 0:
        return

    # Step 3: Extract virtual CMD text from body source.
    start_tok = argv[cmd_start_argv_idx]
    local_src_start = start_tok.start.offset - base_off
    local_src_end = bracket_tok.start.offset + bracket_char_idx - base_off
    if local_src_start < 0 or local_src_end > len(source):
        return
    virtual_cmd_text = source[local_src_start:local_src_end]

    # Step 4: Build virtual CMD token.
    src_start = start_tok.start.offset
    virtual_cmd = Token(
        type=TokenType.CMD,
        text=virtual_cmd_text,
        start=SourcePosition(
            line=start_tok.start.line,
            character=max(start_tok.start.character - 1, 0),
            offset=src_start - 1,
        ),
        end=SourcePosition(
            line=bracket_tok.start.line,
            character=bracket_tok.start.character + bracket_char_idx - 1,
            offset=bracket_tok.start.offset + bracket_char_idx - 1,
        ),
    )

    # Step 5: Splice argv / argv_texts.
    argv[cmd_start_argv_idx : bracket_argv_idx + 1] = [virtual_cmd]
    argv_texts[cmd_start_argv_idx : bracket_argv_idx + 1] = [f"[{virtual_cmd_text}]"]

    # Step 6: Splice all_tokens_buf — find the matching range,
    # accounting for SEP tokens between the merged argv entries.
    start_all_idx: int | None = None
    end_all_idx: int | None = None
    for j, t in enumerate(all_tokens_buf):
        if start_all_idx is None and t.start.offset == start_tok.start.offset:
            start_all_idx = j
        if t.start.offset == bracket_tok.start.offset:
            end_all_idx = j

    if start_all_idx is not None and end_all_idx is not None:
        all_tokens_buf[start_all_idx : end_all_idx + 1] = [virtual_cmd]


# E101 recovery helpers for orphaned switch cases

_KNOWN_COMMANDS: frozenset[str] | None = None


def _known_commands_set() -> frozenset[str]:
    """Lazily build the set of known command names."""
    global _KNOWN_COMMANDS
    if _KNOWN_COMMANDS is None:
        _KNOWN_COMMANDS = known_command_names()
    return _KNOWN_COMMANDS


def _switch_is_form1_incomplete(
    argv_texts: list[str],
    argv: list[Token],
) -> bool:
    """Return True if the switch command is Form 1 (not compact Form 2).

    Form 2 is when the last non-option arg is a single STR token
    containing all pattern/body pairs.  Everything else is Form 1.
    """
    args = argv_texts[1:]
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "--":
            i += 1
            break
        if args[i] in ("-matchvar", "-indexvar"):
            i += 2
            continue
        i += 1
    # i is now the index of the string arg in args (0-based after cmd)
    i += 1  # skip string arg
    remaining = len(args) - i
    if remaining <= 0:
        # Switch has no body at all — definitely incomplete
        return True
    if remaining == 1:
        # Check if it's a STR (Form 2 compact body)
        tok_idx = i + 1  # +1 for cmd name in argv
        if tok_idx < len(argv) and argv[tok_idx].type is TokenType.STR:
            return False  # Form 2 — complete
    # Form 1 with explicit pairs — may have orphaned cases
    return True


def _looks_like_orphaned_switch_case(
    argv_texts: list[str],
    argv: list[Token],
) -> bool:
    """Return True if a command looks like an orphaned switch case.

    An orphaned case has 2 words: pattern + brace-body (or ``-`` for
    fall-through), and its "name" is not a known Tcl command.
    """
    if len(argv_texts) != 2:
        return False
    known = _known_commands_set()
    if argv_texts[0] in known:
        return False
    if len(argv) >= 2 and argv[-1].type is TokenType.STR:
        return True
    if argv_texts[-1] == "-":
        return True
    return False


def _emit_orphaned_switch_case(
    out: list[tuple[int, int, int, int, int]],
    argv: list[Token],
    all_tokens_buf: list[Token],
    regex_positions: frozenset[tuple[int, int]],
) -> None:
    """Emit semantic tokens for an orphaned switch case command.

    The pattern is emitted as a string token and the body is recursed
    into (instead of being emitted as a single string token).
    """
    pattern_emitted = False
    for tok in all_tokens_buf:
        if tok.type is TokenType.SEP:
            continue
        if not pattern_emitted:
            # First non-SEP token is the pattern — emit as string.
            if tok.type is TokenType.ESC and token_content_shift(tok) > 0:
                rendered = '"' + tok.text + '"'
            elif tok.type is TokenType.VAR:
                rendered = "$" + tok.text
            else:
                rendered = tok.text
            _append_text_token(
                out,
                start=tok.start,
                text=rendered,
                type_idx=_TYPE_INDEX["string"],
            )
            pattern_emitted = True
            continue
        # Subsequent tokens: body (STR → recurse) or other types.
        if tok.type is TokenType.STR and tok.text.strip():
            _collect_tokens(out, tok.text, body_token=tok, regex_positions=regex_positions)
        elif tok.type is TokenType.CMD:
            _collect_tokens(out, tok.text, body_token=tok, regex_positions=regex_positions)
        elif tok.type is TokenType.VAR:
            span = tok.end.offset - tok.start.offset + 1
            if span > len(tok.text) + 1:
                rendered = "${" + tok.text + "}"
            else:
                rendered = "$" + tok.text
            _append_text_token(
                out,
                start=tok.start,
                text=rendered,
                type_idx=_TYPE_INDEX["variable"],
            )


def _collect_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
    body_token: Token | None = None,
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
    _line_starts: list[int] | tuple[int, ...] | None = None,
    _source_len: int | None = None,
) -> None:
    """Collect semantic tokens from *source* into *tokens*.

    Each entry is (line, char, length, type_idx, modifiers) -- absolute positions.
    These are sorted and delta-encoded by the caller.

    *regex_positions* is a frozenset of ``(line, character)`` tuples where the
    analyser has determined a token holds a regex pattern (e.g. a variable whose
    constant value flows into a ``regexp`` call).

    Recurses into CMD tokens (``[...]``) and into BODY arguments (braced
    bodies of proc, if, while, for, foreach, namespace eval, etc.).
    """
    # Share line_starts across recursive calls to avoid O(n) newline scanning
    # per call, and capture the full document length so position_from_offset
    # clamps correctly even when ``source`` is a body substring.  build_line_starts
    # is byte-identical to the lexer's own index.
    if _line_starts is None and body_token is None:
        _line_starts = build_line_starts(source)
        _source_len = len(source)
    if body_token is not None:
        # Token start points to the delimiter ({ or [); the content
        # starts one character later, so add 1 to offset and col.
        base_offset = body_token.start.offset + 1
        base_line = body_token.start.line
        base_col = body_token.start.character + 1
    else:
        base_offset = base_line = base_col = 0
    # Single-pass recovering tokenise: recovers unterminated delimiters (e.g. a
    # missing ]) so downstream classification sees the correct argument
    # structure, while well-formed top-level sources skip the recovery pass
    # entirely.  Shares the green-tree memo and the line index across recursion.
    lex_tokens, _ = tokenise_recovering(
        source,
        base_offset,
        base_line,
        base_col,
        body_token=body_token,
        line_starts=list(_line_starts) if _line_starts is not None else None,
    )

    # We need to track commands so we can identify BODY arguments.
    # Collect tokens per command, then emit them.
    argv: list[Token] = []  # first token per argument
    argv_texts: list[str] = []  # concatenated text per argument
    all_tokens_buf: list[Token] = []  # all tokens in current command
    prev_type = TokenType.EOL

    # E101 recovery: when a switch is flushed in Form 1 (explicit
    # pattern/body pairs), subsequent commands that look like orphaned
    # cases should be emitted as switch-case tokens, not standalone
    # commands.
    switch_recovery_active = [False]

    def _flush_command() -> None:
        """Process a complete command's tokens with BODY awareness."""
        if not argv:
            return
        # Recovery: merge tokens around stray ']' (missing '[') into
        # a virtual CMD so downstream handlers see correct arg structure.
        _recover_stray_close_bracket_in_flush(
            argv,
            argv_texts,
            all_tokens_buf,
            source,
            body_token,
        )
        cmd_name = argv_texts[0]

        # E101 recovery: if a switch was just flushed (Form 1) and
        # this command looks like an orphaned case, emit as switch
        # case tokens (pattern = string, body = recurse) and stay in
        # recovery mode for further cases.
        if switch_recovery_active[0]:
            if _looks_like_orphaned_switch_case(argv_texts, argv):
                _emit_orphaned_switch_case(
                    tokens,
                    argv,
                    all_tokens_buf,
                    regex_positions,
                )
                return  # stay in switch recovery for next command
            switch_recovery_active[0] = False
        (
            body_indices,
            expr_indices,
            _varname_indices,
            _varread_indices,
            pattern_indices,
            keyword_indices,
        ) = arg_indices_for_roles(
            cmd_name,
            argv_texts[1:],
            (
                ArgRole.BODY,
                ArgRole.EXPR,
                ArgRole.VAR_WRITE,
                ArgRole.VAR_READ,
                ArgRole.PATTERN,
                ArgRole.KEYWORD,
            ),
        )
        varname_indices = _varname_indices | _varread_indices
        param_arg_idx = _proc_param_list_arg_index(cmd_name, argv_texts)
        proc_name_arg_idx = _procedure_name_arg_index(cmd_name, argv_texts)
        binary_format_arg_idx = _binary_format_arg_index(cmd_name, argv_texts)
        subcommand_arg_idx = _subcommand_arg_index(cmd_name, argv_texts)
        sprintf_format_arg_idx = _sprintf_format_arg_index(cmd_name, argv_texts)
        clock_format_arg_idx = _clock_format_arg_index(cmd_name, argv_texts)
        regsub_subspec_arg_idx = _regsub_subspec_arg_index(cmd_name, argv_texts)
        string_map_arg_idx = _string_map_mapping_arg_index(cmd_name, argv_texts)
        glob_pattern_indices = _glob_pattern_arg_indices(cmd_name, argv_texts)
        option_indices = _option_arg_indices(cmd_name, argv_texts[1:])
        skip_body_indices: set[int] = set()
        if cmd_name == "apply":
            skip_body_indices = _collect_apply_lambda(
                tokens,
                argv_texts[1:],
                argv[1:],
                regex_positions=regex_positions,
                line_starts=_line_starts or (),
                source_len=_source_len or len(source),
            )
        elif cmd_name == "switch":
            skip_body_indices = _collect_switch_case_bodies(
                tokens,
                argv_texts[1:],
                argv[1:],
                regex_positions=regex_positions,
                line_starts=_line_starts or (),
                source_len=_source_len or len(source),
            )
            # Activate E101 recovery if the switch parsed as Form 1
            # (explicit pattern/body pairs, not a single braced body).
            # Subsequent case-like commands are likely orphaned.
            switch_recovery_active[0] = _switch_is_form1_incomplete(
                argv_texts,
                argv,
            )

        # Walk all tokens in this command and emit/recurse
        arg_idx = -1  # -1 = command name position, then 0, 1, 2...
        prev = TokenType.EOL
        for tok in all_tokens_buf:
            if tok.type is TokenType.SEP:
                prev = tok.type
                continue

            # Determine argument index
            if prev in (TokenType.SEP, TokenType.EOL):
                arg_idx += 1
            prev = tok.type

            is_cmd_name = arg_idx == 0

            # Check if this token is part of a BODY argument
            is_body = (arg_idx - 1) in body_indices and arg_idx > 0
            is_expr = (arg_idx - 1) in expr_indices and arg_idx > 0
            is_varname = (arg_idx - 1) in varname_indices and arg_idx > 0
            is_pattern = (arg_idx - 1) in pattern_indices and arg_idx > 0
            is_option = (arg_idx - 1) in option_indices and arg_idx > 0
            is_keyword_arg = (arg_idx - 1) in keyword_indices and arg_idx > 0

            if tok.type is TokenType.CMD:
                # Always recurse into command substitutions
                _collect_tokens(
                    tokens,
                    tok.text,
                    body_token=tok,
                    regex_positions=regex_positions,
                    _line_starts=_line_starts,
                    _source_len=_source_len,
                )
                continue

            # Skip tokens already processed by _collect_switch_case_bodies.
            if tok.type is TokenType.STR and arg_idx > 0 and (arg_idx - 1) in skip_body_indices:
                continue

            if tok.type is TokenType.STR and is_body and tok.text.strip():
                # This is a body argument -- recurse instead of emitting as string
                _collect_tokens(
                    tokens,
                    tok.text,
                    body_token=tok,
                    regex_positions=regex_positions,
                    _line_starts=_line_starts,
                    _source_len=_source_len,
                )
                continue

            if tok.type is TokenType.STR and is_expr and tok.text.strip():
                _collect_expression_tokens(
                    tokens,
                    tok.text,
                    tok,
                    regex_positions=regex_positions,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                )
                continue

            if (
                proc_name_arg_idx is not None
                and arg_idx == proc_name_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
            ):
                rendered_name = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=rendered_name,
                    type_idx=_TYPE_INDEX["function"],
                    modifiers=1 << _MOD_INDEX["definition"],
                )
                continue

            if (
                subcommand_arg_idx is not None
                and arg_idx == subcommand_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
                and _is_known_subcommand(cmd_name, argv_texts[arg_idx])
            ):
                rendered_sub = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=rendered_sub,
                    type_idx=_TYPE_INDEX["keyword"],
                    modifiers=1 << _MOD_INDEX["defaultLibrary"],
                )
                continue

            if (
                param_arg_idx is not None
                and arg_idx == param_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_param_list_tokens(tokens, tok):
                    continue

            if (
                binary_format_arg_idx is not None
                and arg_idx == binary_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_binary_format_spec_tokens(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            if (
                sprintf_format_arg_idx is not None
                and arg_idx == sprintf_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_sprintf_format_spec_tokens(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            if (
                clock_format_arg_idx is not None
                and arg_idx == clock_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_clock_format_spec_tokens(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            if (
                regsub_subspec_arg_idx is not None
                and arg_idx == regsub_subspec_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_regsub_subspec_tokens(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            # string map mapping list: alternate pair colours.
            if (
                string_map_arg_idx is not None
                and arg_idx == string_map_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type is TokenType.STR
            ):
                if _collect_string_map_pairs_tokens(tokens, tok):
                    continue

            # Glob pattern arguments (string match, glob, lsearch)
            # Must be checked before the generic PATTERN handler.
            if (
                arg_idx in glob_pattern_indices
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
            ):
                if _collect_glob_pattern_tokens(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            # Variable name arguments (e.g. the "a" in "set a foo") get
            # highlighted as variables with the declaration modifier.
            if is_varname and tok.type is TokenType.ESC:
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["variable"],
                    modifiers=1 << _MOD_INDEX["declaration"],
                )
                continue

            # Regex pattern arguments (e.g. the pattern in "regexp {pat} str")
            # get highlighted as the "regexp" semantic token type.
            if is_pattern and tok.type in (TokenType.ESC, TokenType.STR):
                _emit_regex_token(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                )
                continue

            # Analysis-driven regex override: when the analyser has
            # determined that a token (typically a variable reference or
            # string literal) holds a regex pattern, highlight it as
            # ``regexp``.  The analyser records positions for both the
            # ``set`` value and the ``$var`` use-site.
            if regex_positions and (tok.start.line, tok.start.character) in regex_positions:
                # But don't override positional variables inside a format string!
                if not (sprintf_format_arg_idx is not None and arg_idx == sprintf_format_arg_idx):
                    rendered = "$" + tok.text if tok.type is TokenType.VAR else tok.text
                    if tok.type is TokenType.STR:
                        rendered = f"{{{tok.text}}}"
                    _append_text_token(
                        tokens,
                        start=tok.start,
                        text=rendered,
                        type_idx=_TYPE_INDEX["regexp"],
                    )
                    continue

            # iRule event name in ``when EVENT { body }``
            if (
                cmd_name == "when"
                and arg_idx == 1
                and tok.type is TokenType.ESC
                and _EVENT_RE.match(tok.text)
            ):
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["event"],
                )
                continue

            # Structural keyword words (if's else/elseif/then, try's
            # on/trap/finally) sit at argument positions, not the command-name
            # slot, so the default classifier would render them as strings.
            # The registry's KEYWORD role marks them; highlight as keywords.
            # Use the token content base so a quoted keyword (``"else"``, whose
            # ``start`` sits on the opening quote) is offset past the quote
            # rather than marking ``"els``.
            if is_keyword_arg and tok.type is TokenType.ESC:
                kw_offset, kw_line, kw_col = token_content_base(tok)
                _append_text_token(
                    tokens,
                    start=SourcePosition(line=kw_line, character=kw_col, offset=kw_offset),
                    text=tok.text,
                    type_idx=_TYPE_INDEX["keyword"],
                )
                continue

            # Command options/flags known to the registry
            if is_option and tok.type is TokenType.ESC and tok.text.startswith("-"):
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["decorator"],
                )
                continue

            type_idx = _classify_token(tok.type, tok.text, is_command_name=is_cmd_name)
            if type_idx is None:
                continue

            # defaultLibrary modifier for built-in commands (registry functions)
            modifiers = 0
            if (
                is_cmd_name
                and type_idx == _TYPE_INDEX["function"]
                and tok.text in _BUILTIN_COMMANDS
            ):
                modifiers = 1 << _MOD_INDEX["defaultLibrary"]

            # Braced literal classified as a string: colour only the inner
            # content and leave the ``{`` / ``}`` delimiters as grouping
            # syntax.  ``tok.text`` already holds just the inner content, but
            # the token starts on the opening brace, so shift one character
            # right (via ``token_content_base``) and emit from there; the
            # closing brace is simply not covered.  Emitting ``tok.text`` from
            # ``tok.start`` instead would start on the brace and fall one
            # character short, dropping the last inner character (issue #579).
            if tok.type is TokenType.STR and type_idx == _TYPE_INDEX["string"]:
                content_offset, content_line, content_col = token_content_base(tok)
                _append_text_token(
                    tokens,
                    start=SourcePosition(
                        line=content_line,
                        character=content_col,
                        offset=content_offset,
                    ),
                    text=tok.text,
                    type_idx=type_idx,
                    modifiers=modifiers,
                )
                continue

            # Reconstruct the full source representation so
            # len(rendered) matches the source span.
            if tok.type is TokenType.VAR:
                span = tok.end.offset - tok.start.offset + 1
                if span > len(tok.text) + 1:  # +1 for '$'
                    rendered = "${" + tok.text + "}"  # brace-delimited
                else:
                    rendered = "$" + tok.text
            elif tok.type is TokenType.ESC and token_content_shift(tok) > 0:
                # Leading quote is always present.  A trailing quote only
                # exists when the token is NOT still inside a quoted string
                # (i.e. the token completed the quoted word).
                if tok.in_quote:
                    rendered = '"' + tok.text
                elif tok.text == "":
                    # A completing segment with no text is *just* the closing
                    # quote — the string ended immediately after an
                    # interpolation (``"$name"`` / ``"a $b"``).  The token's
                    # start already sits on that closing quote, so emitting
                    # ``'"' + '' + '"'`` would double-count it: a 2-wide token
                    # over the 1-char quote, over-running the line and tripping
                    # the client's "overlapping semantic tokens" guard.
                    rendered = '"'
                else:
                    rendered = '"' + tok.text + '"'
            else:
                rendered = tok.text

            # Namespace-qualified command names: split into namespace + command
            # (but never split comment tokens — they should remain atomic)
            if is_cmd_name and "::" in tok.text and tok.type is not TokenType.COMMENT:
                _emit_namespace_qualified(
                    tokens,
                    tok,
                    type_idx,
                    modifiers,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                )
                continue

            # Escape sequences in string-classified ESC tokens
            if type_idx == _TYPE_INDEX["string"] and tok.type is TokenType.ESC and "\\" in tok.text:
                if _emit_string_with_escapes(
                    tokens,
                    tok,
                    line_starts=_line_starts or (),
                    source_len=_source_len or len(source),
                ):
                    continue

            _append_text_token(
                tokens,
                start=tok.start,
                text=rendered,
                type_idx=type_idx,
                modifiers=modifiers,
            )

    for tok in lex_tokens:
        match tok.type:
            case TokenType.SEP:
                all_tokens_buf.append(tok)
                prev_type = tok.type
                continue
            case TokenType.EOL:
                _flush_command()
                argv = []
                argv_texts = []
                all_tokens_buf = []
                prev_type = tok.type
                continue
            case _:
                pass

        all_tokens_buf.append(tok)

        # Build argv for command identification
        text = tok.text
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
            argv_texts.append(text)
        else:
            if argv_texts:
                argv_texts[-1] += text
            else:
                argv.append(tok)
                argv_texts.append(text)

        prev_type = tok.type

    # Handle trailing command without final EOL
    _flush_command()
