from __future__ import annotations

import logging
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...commands.registry import REGISTRY
from ...common.ranges import position_from_relative
from ...parsing.command_segmenter import SegmentedCommand
from ...parsing.known_commands import known_command_names
from ...parsing.tokens import SourcePosition, Token, TokenType
from ..semantic_model import (
    CodeFix,
    Diagnostic,
    Range,
    Scope,
    Severity,
)

log = logging.getLogger(__name__)


class _AnalyserRecoveryMixin(_Base):
    """Parser-error recovery heuristics."""

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

        A ``]`` inside a double-quoted string is a literal character, not
        a stray bracket — quoted-context ESC tokens must be skipped.
        """
        from ...parsing.token_positions import classify_quoted_contexts

        # Step 1: Find an ESC token containing ']' at its end.
        # Skip ESC tokens whose ']' is inside a double-quoted string
        # (e.g. `foo "bar]"`).
        in_quoted = classify_quoted_contexts(list(cmd.all_tokens))
        bracket_tok: Token | None = None
        bracket_tok_idx = -1
        bracket_char_idx = -1
        for ti, tok in enumerate(cmd.all_tokens):
            if tok.type is not TokenType.ESC:
                continue
            if in_quoted[ti]:
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
