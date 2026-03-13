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

from __future__ import annotations

from dataclasses import dataclass, field
from typing import cast

from ..analysis.semantic_model import Range
from ..commands.registry import REGISTRY
from ..commands.registry.runtime import ArgRole, arg_indices_for_role
from ..common.dialect import active_dialect as _active_dialect
from ..common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ..common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ..common.ranges import range_from_token
from ..parsing.command_segmenter import SegmentedCommand, TopLevelChunk, segment_commands
from ..parsing.command_shapes import extract_single_expr_argument
from ..parsing.expr_parser import parse_expr as _std_parse_expr
from ..parsing.lexer import TclLexer, TclParseError
from ..parsing.tokens import Token, TokenType
from .ir import (
    CommandTokens,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
    IRSwitch,
    IRSwitchArm,
    IRTry,
    IRTryHandler,
    IRWhile,
)
from .lowering_hooks import register_all as _register_lowering_hooks
from .token_helpers import word_piece as _word_piece

_DYNAMIC_BARRIER_COMMANDS = REGISTRY.dynamic_barrier_commands()

# Register per-command lowering hooks on the REGISTRY.
_register_lowering_hooks()


def _parse_expr(source: str):  # noqa: F811
    return _std_parse_expr(source, dialect=_active_dialect())


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

    def lower(self, source: str) -> IRModule:
        self.module.top_level = self._lower_script(source, namespace="::")
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
        stmts = self._lower_segmented(commands, namespace=namespace)
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
        return IRScript(statements=self._lower_segmented(commands, namespace=namespace))

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
        return tuple(statements)

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
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        clauses: list[IRIfClause] = []
        else_body: IRScript | None = None
        else_range: Range | None = None

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
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                if i + 2 < len(args):
                    return IRBarrier(
                        range=cmd.range,
                        reason='extra words after "else" clause',
                        command=cmd.name,
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                body_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
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
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            body_idx = i
            body_tok = arg_tokens[body_idx] if body_idx < len(arg_tokens) else None
            cond_tok = arg_tokens[cond_idx] if cond_idx < len(arg_tokens) else cmd.argv[0]
            body = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
            clauses.append(
                IRIfClause(
                    condition=_parse_expr(args[cond_idx]),
                    condition_range=range_from_token(cond_tok),
                    body=body,
                    body_range=range_from_token(body_tok) if body_tok is not None else cmd.range,
                )
            )
            i += 1

        if not clauses:
            return IRBarrier(
                range=cmd.range,
                reason="malformed if",
                command=cmd.name,
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
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        i = 0
        mode = "exact"  # default switch mode
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "--":
                i += 1
                break
            if args[i] in ("-glob", "-regexp"):
                mode = args[i][1:]
            i += 1

        if i >= len(args):
            return IRBarrier(
                range=cmd.range,
                reason="malformed switch options",
                command=cmd.name,
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
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        pairs: list[tuple[str, Token | None, str, Token | None]] = []
        if i == len(args) - 1 and i < len(arg_tokens) and i < len(arg_single) and arg_single[i]:
            body_text = args[i]
            body_tok = arg_tokens[i]
            elements, element_tokens = _switch_body_elements(body_text, body_tok)
            if len(elements) % 2 != 0:
                # Odd count → "extra switch pattern with no body" at runtime
                return IRBarrier(
                    range=cmd.range,
                    reason="switch odd pattern count",
                    command=cmd.name,
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
            if remaining_count % 2 != 0:
                return IRBarrier(
                    range=cmd.range,
                    reason="switch odd pattern count",
                    command=cmd.name,
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
        if len(args) < 3 or len(args) % 2 == 0:
            return IRBarrier(
                range=cmd.range,
                reason="malformed foreach",
                command=cmd.name,
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
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_tok = arg_tokens[0] if arg_tokens else None
        if body_tok is None or not (arg_single and arg_single[0]):
            return IRBarrier(
                range=cmd.range,
                reason="catch with dynamic body",
                command=cmd.name,
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
                args=tuple(args),
                tokens=cmd.cmd_tokens,
            )

        body_tok = arg_tokens[0] if arg_tokens else None
        if body_tok is None or not (arg_single and arg_single[0]):
            return IRBarrier(
                range=cmd.range,
                reason="try with dynamic body",
                command=cmd.name,
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
                    args=tuple(args),
                    defs=(var_name,),
                    reads_own_defs=True,
                    tokens=cmd.cmd_tokens,
                )

            case "update" | "with":
                # Complex variable binding — stay as barrier but lower the body.
                return IRBarrier(
                    range=cmd.range,
                    reason=f"dict {sub}",
                    command=cmd.name,
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case _:
                # Pure subcommands: get, create, exists, keys, values, etc.
                return IRCall(
                    range=cmd.range, command=cmd.name, args=tuple(args), tokens=cmd.cmd_tokens
                )

    def _lower_command(self, cmd: _Command, *, namespace: str) -> IRStatement | None:
        if not cmd.texts:
            return None

        cmd_name = cmd.name
        args = cmd.args
        arg_tokens = cmd.arg_tokens

        # Check for a registered lowering hook first.
        spec = REGISTRY.get_any(cmd_name)
        if spec is not None and spec.lowering is not None:
            result = spec.lowering(self, cmd)
            if result is not None:
                return cast(IRStatement, result)

        match cmd_name:
            case "proc" if len(args) == 3 and len(arg_tokens) >= 3:
                proc_name = args[0]
                # Dynamic proc names (containing $ or [) can only be
                # resolved at runtime — emit as a regular command call
                # so that the VM evaluates variable/command substitutions.
                if "$" in proc_name or "[" in proc_name:
                    return IRBarrier(
                        range=cmd.range,
                        reason="dynamic proc name",
                        command=cmd_name,
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                try:
                    params = _parse_param_names(args[1])
                except Exception:
                    params = ()
                qualified = _qualify_proc_name(namespace, proc_name)
                body_tok = arg_tokens[2]
                body = self._lower_body_arg(args[2], body_tok, namespace=namespace)
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
                return IRCall(
                    range=cmd.range,
                    command=cmd_name,
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case "when" if len(args) >= 2 and len(arg_tokens) >= 2:
                event_name = args[0]
                body_idx = len(args) - 1
                body_tok = arg_tokens[body_idx]
                body = self._lower_body_arg(args[body_idx], body_tok, namespace=namespace)
                qualified = f"::when::{event_name}"
                self.module.procedures[qualified] = IRProcedure(
                    name=event_name,
                    qualified_name=qualified,
                    params=(),
                    range=cmd.range,
                    body=body,
                )
                return IRCall(
                    range=cmd.range, command=cmd_name, args=tuple(args), tokens=cmd.cmd_tokens
                )

            case "namespace" if len(args) >= 3 and args[0] == "eval" and len(arg_tokens) >= 3:
                child_ns = _join_namespace(namespace, args[1])
                prev_ns_eval = self._in_namespace_eval
                self._in_namespace_eval = True
                self._lower_body_arg(args[2], arg_tokens[2], namespace=child_ns)
                self._in_namespace_eval = prev_ns_eval
                return IRBarrier(
                    range=cmd.range,
                    reason="namespace eval",
                    command=cmd_name,
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

            case "lmap":
                return self._lower_foreach(cmd, namespace=namespace, is_lmap=True)

            case "catch":
                return self._lower_catch(cmd, namespace=namespace)

            case "try":
                return self._lower_try(cmd, namespace=namespace)

            case "dict" if args:
                return self._lower_dict(cmd, namespace=namespace)

            case _ if cmd_name in _DYNAMIC_BARRIER_COMMANDS:
                return IRBarrier(
                    range=cmd.range,
                    reason="dynamic command",
                    command=cmd_name,
                    args=tuple(args),
                    tokens=cmd.cmd_tokens,
                )

            case _:
                body_indices = arg_indices_for_role(cmd_name, args, ArgRole.BODY)
                if body_indices:
                    return IRBarrier(
                        range=cmd.range,
                        reason="unsupported body command",
                        command=cmd_name,
                        args=tuple(args),
                        tokens=cmd.cmd_tokens,
                    )
                var_indices = arg_indices_for_role(cmd_name, args, ArgRole.VAR_NAME)
                var_read_indices = arg_indices_for_role(cmd_name, args, ArgRole.VAR_READ)
                if var_indices or var_read_indices:
                    var_defs = tuple(
                        _normalise_var_name(args[i]) for i in sorted(var_indices) if i < len(args)
                    )
                    var_reads = tuple(
                        _normalise_var_name(args[i])
                        for i in sorted(var_read_indices)
                        if i < len(args)
                    )
                    return IRCall(
                        range=cmd.range,
                        command=cmd_name,
                        args=tuple(args),
                        defs=var_defs,
                        reads=var_reads,
                        tokens=cmd.cmd_tokens,
                    )
                return IRCall(
                    range=cmd.range, command=cmd_name, args=tuple(args), tokens=cmd.cmd_tokens
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
    """
    if chunk_ir is None or chunks is None:
        return _Lowerer().lower(source)

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
