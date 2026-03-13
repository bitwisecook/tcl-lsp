#!/usr/bin/env python3
"""Console test client for Tcl syntax and diagnostics inspection.

Features:
- Accept Tcl script from file, stdin, or --source.
- Print a colourised syntax tree (commands -> words -> token fragments).
- Include diagnostics in the tree.
- Render source with Rust-style ^---- markers for tokens and diagnostics.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass, field
from pathlib import Path

try:
    from core.analysis.analyser import analyse
    from core.analysis.semantic_model import Diagnostic, Severity
    from core.common.source_map import offset_to_line_col as _offset_to_line_col
    from core.parsing.lexer import TclLexer
    from core.parsing.tokens import TokenType
except ModuleNotFoundError:
    # Allow running this file directly from anywhere.
    ROOT = Path(__file__).resolve().parent
    if str(ROOT) not in sys.path:
        sys.path.insert(0, str(ROOT))
    from core.analysis.analyser import analyse
    from core.analysis.semantic_model import Diagnostic, Severity
    from core.common.source_map import offset_to_line_col as _offset_to_line_col
    from core.parsing.lexer import TclLexer
    from core.parsing.tokens import TokenType


class Ansi:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    GREEN = "\033[32m"
    GRAY = "\033[90m"


SEVERITY_COLOURS = {
    Severity.ERROR: Ansi.RED,
    Severity.WARNING: Ansi.YELLOW,
    Severity.INFO: Ansi.BLUE,
    Severity.HINT: Ansi.MAGENTA,
}

TOKEN_COLOURS = {
    TokenType.ESC: Ansi.GREEN,
    TokenType.STR: Ansi.GREEN,
    TokenType.CMD: Ansi.CYAN,
    TokenType.VAR: Ansi.BLUE,
    TokenType.COMMENT: Ansi.GRAY,
    TokenType.SEP: Ansi.GRAY,
    TokenType.EOL: Ansi.GRAY,
}


@dataclass(slots=True, frozen=True)
class TokenView:
    type: TokenType
    text: str
    start_offset: int
    end_offset: int


@dataclass(slots=True)
class WordNode:
    text: str
    start_offset: int
    end_offset: int
    fragments: list[TokenView] = field(default_factory=list)
    nested_commands: list["CommandNode"] = field(default_factory=list)


@dataclass(slots=True)
class CommandNode:
    index: int
    start_offset: int
    end_offset: int
    words: list[WordNode] = field(default_factory=list)
    comments: list[TokenView] = field(default_factory=list)


@dataclass(slots=True, frozen=True)
class Annotation:
    start_offset: int
    end_offset: int
    label: str
    colour: str


class LineIndex:
    def __init__(self, source: str) -> None:
        self.source = source
        self.lines = source.splitlines()
        if source.endswith("\n"):
            self.lines.append("")
        self.line_starts = [0]
        for i, ch in enumerate(source):
            if ch == "\n":
                self.line_starts.append(i + 1)
        if not self.lines:
            self.lines = [""]

    def line_count(self) -> int:
        return len(self.lines)

    def line_text(self, line: int) -> str:
        if 0 <= line < len(self.lines):
            return self.lines[line]
        return ""

    def line_start(self, line: int) -> int:
        if line < 0:
            return 0
        if line >= len(self.line_starts):
            return len(self.source)
        return self.line_starts[line]

    def line_end_exclusive(self, line: int) -> int:
        if line + 1 < len(self.line_starts):
            return self.line_starts[line + 1]
        return len(self.source)

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        return _offset_to_line_col(self.source, self.line_starts, offset)

    def format_range(self, start_offset: int, end_offset: int) -> str:
        s_line, s_col = self.offset_to_line_col(start_offset)
        e_line, e_col = self.offset_to_line_col(end_offset)
        return f"{s_line + 1}:{s_col + 1}-{e_line + 1}:{e_col + 1}"


def style(text: str, colour: str, enabled: bool) -> str:
    if not enabled:
        return text
    return f"{colour}{text}{Ansi.RESET}"


def preview(text: str, limit: int = 48) -> str:
    escaped = text.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
    if len(escaped) > limit:
        return escaped[: limit - 3] + "..."
    return escaped


def token_views(source: str, base_offset: int = 0) -> list[TokenView]:
    views: list[TokenView] = []
    for tok in TclLexer(source).tokenise_all():
        views.append(
            TokenView(
                type=tok.type,
                text=tok.text,
                start_offset=base_offset + tok.start.offset,
                end_offset=base_offset + tok.end.offset,
            )
        )
    return views


def build_commands(source: str, base_offset: int = 0) -> list[CommandNode]:
    return _build_commands_from_tokens(token_views(source, base_offset))


def _build_commands_from_tokens(tokens: list[TokenView]) -> list[CommandNode]:
    commands: list[CommandNode] = []
    current: list[TokenView] = []
    next_index = 1

    for tok in tokens:
        if tok.type is TokenType.EOL:
            cmd = _build_command(current, next_index)
            if cmd is not None:
                _resolve_body_args(cmd)
                _resolve_expression_args(cmd)
                commands.append(cmd)
                next_index += 1
            current = []
            continue
        current.append(tok)

    cmd = _build_command(current, next_index)
    if cmd is not None:
        _resolve_body_args(cmd)
        _resolve_expression_args(cmd)
        commands.append(cmd)
    return commands


def _build_command(tokens: list[TokenView], index: int) -> CommandNode | None:
    meaningful = [t for t in tokens if t.type is not TokenType.SEP]
    if not meaningful:
        return None

    cmd = CommandNode(
        index=index,
        start_offset=meaningful[0].start_offset,
        end_offset=meaningful[-1].end_offset,
    )

    current_fragments: list[TokenView] = []
    prev_type = TokenType.EOL

    for tok in tokens:
        if tok.type is TokenType.SEP:
            if current_fragments:
                cmd.words.append(_build_word(current_fragments))
                current_fragments = []
            prev_type = tok.type
            continue

        if tok.type is TokenType.COMMENT and not current_fragments and not cmd.words:
            cmd.comments.append(tok)
            prev_type = tok.type
            continue

        if prev_type in (TokenType.EOL, TokenType.SEP):
            if current_fragments:
                cmd.words.append(_build_word(current_fragments))
            current_fragments = [tok]
        else:
            current_fragments.append(tok)

        prev_type = tok.type

    if current_fragments:
        cmd.words.append(_build_word(current_fragments))

    return cmd


def _build_word(fragments: list[TokenView]) -> WordNode:
    start = fragments[0].start_offset
    end = fragments[-1].end_offset
    text = "".join(f.text for f in fragments)
    nested: list[CommandNode] = []

    for fragment in fragments:
        if fragment.type is TokenType.CMD and fragment.text:
            # start_offset points to '['; content starts one char later
            nested.extend(build_commands(fragment.text, base_offset=fragment.start_offset + 1))

    return WordNode(
        text=text,
        start_offset=start,
        end_offset=end,
        fragments=fragments,
        nested_commands=nested,
    )


def _resolve_body_args(cmd: CommandNode) -> None:
    """If words in *cmd* are BODY arguments per SIGNATURES, recursively expand them."""
    from core.commands.registry.runtime import body_arg_indices

    if not cmd.words:
        return

    cmd_name = cmd.words[0].text
    args = [word.text for word in cmd.words[1:]]
    for arg_idx in sorted(body_arg_indices(cmd_name, args)):
        word_idx = arg_idx + 1  # skip command name
        if word_idx < len(cmd.words):
            _expand_body_word(cmd.words[word_idx])


def _expand_body_word(word: WordNode) -> None:
    """If a word consists of a single STR fragment (braced body), recursively expand it."""
    # Only expand if word has at least one STR fragment with content
    for fragment in word.fragments:
        if fragment.type is TokenType.STR and fragment.text.strip():
            # start_offset points to '{'; content starts one char later
            nested = build_commands(fragment.text, base_offset=fragment.start_offset + 1)
            word.nested_commands.extend(nested)
            break  # Only expand the first (primary) STR fragment


def _resolve_expression_args(cmd: CommandNode) -> None:
    """If words in *cmd* are EXPR arguments, parse expression sub-language."""
    from core.commands.registry.runtime import ArgRole, arg_indices_for_role

    if not cmd.words:
        return

    cmd_name = cmd.words[0].text
    args = [word.text for word in cmd.words[1:]]
    for arg_idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.EXPR)):
        word_idx = arg_idx + 1  # skip command name
        if word_idx < len(cmd.words):
            _expand_expression_word(cmd.words[word_idx])


def _expand_expression_word(word: WordNode) -> None:
    """Expand command substitutions that appear inside expression arguments."""
    from core.parsing.expr_lexer import ExprTokenType, tokenise_expr

    for fragment in word.fragments:
        if fragment.type is not TokenType.STR or not fragment.text.strip():
            continue

        base_offset = fragment.start_offset + 1  # skip '{'
        for expr_tok in tokenise_expr(fragment.text):
            if expr_tok.type is ExprTokenType.COMMAND and len(expr_tok.text) >= 2:
                cmd_text = expr_tok.text[1:-1]
                cmd_start = base_offset + expr_tok.start  # points at '['
                nested = build_commands(cmd_text, base_offset=cmd_start + 1)
                word.nested_commands.extend(nested)


def diagnostics_for_command(cmd: CommandNode, diagnostics: list[Diagnostic]) -> list[Diagnostic]:
    matched: list[Diagnostic] = []
    for diagnostic in diagnostics:
        start = diagnostic.range.start.offset
        if cmd.start_offset <= start <= cmd.end_offset:
            matched.append(diagnostic)
    return matched


def print_tree(
    commands: list[CommandNode],
    diagnostics: list[Diagnostic],
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("syntax-tree", Ansi.BOLD, use_colour))
    top_level = len(commands)

    for i, cmd in enumerate(commands):
        _print_command(
            cmd,
            prefix="",
            is_last=i == (top_level - 1),
            diagnostics=diagnostics,
            line_index=line_index,
            use_colour=use_colour,
            is_nested=False,
        )


def _print_command(
    cmd: CommandNode,
    *,
    prefix: str,
    is_last: bool,
    diagnostics: list[Diagnostic],
    line_index: LineIndex,
    use_colour: bool,
    is_nested: bool,
) -> None:
    connector = "└── " if is_last else "├── "
    kind = "NestedCommand" if is_nested else f"Command {cmd.index}"
    name = style(kind, Ansi.BOLD, use_colour)
    span = style(
        f"[{line_index.format_range(cmd.start_offset, cmd.end_offset)}]", Ansi.DIM, use_colour
    )
    print(f"{prefix}{connector}{name} {span}")

    child_prefix = prefix + ("    " if is_last else "│   ")
    children: list[tuple[str, object]] = []

    for comment in cmd.comments:
        children.append(("comment", comment))
    for word in cmd.words:
        children.append(("word", word))

    local_diags = diagnostics_for_command(cmd, diagnostics)
    for diagnostic in local_diags:
        children.append(("diagnostic", diagnostic))

    for i, (kind, item) in enumerate(children):
        child_last = i == (len(children) - 1)
        child_connector = "└── " if child_last else "├── "
        leaf_prefix = child_prefix + child_connector
        nested_prefix = child_prefix + ("    " if child_last else "│   ")

        if kind == "comment":
            comment = item
            assert isinstance(comment, TokenView)
            label = f"COMMENT '{preview(comment.text)}'"
            span = line_index.format_range(comment.start_offset, comment.end_offset)
            text = style(label, TOKEN_COLOURS[TokenType.COMMENT], use_colour)
            print(f"{leaf_prefix}{text} {style(f'[{span}]', Ansi.DIM, use_colour)}")
            continue

        if kind == "diagnostic":
            diagnostic = item
            assert isinstance(diagnostic, Diagnostic)
            sev = diagnostic.severity.name
            code = f" {diagnostic.code}" if diagnostic.code else ""
            span = line_index.format_range(
                diagnostic.range.start.offset, diagnostic.range.end.offset
            )
            label = f"{sev}{code}: {preview(diagnostic.message, limit=80)}"
            colour = SEVERITY_COLOURS.get(diagnostic.severity, Ansi.RED)
            print(
                f"{leaf_prefix}{style(label, colour, use_colour)} {style(f'[{span}]', Ansi.DIM, use_colour)}"
            )
            continue

        word = item
        assert isinstance(word, WordNode)
        word_span = line_index.format_range(word.start_offset, word.end_offset)
        word_label = f"word '{preview(word.text)}'"
        print(
            f"{leaf_prefix}{style(word_label, Ansi.CYAN, use_colour)} {style(f'[{word_span}]', Ansi.DIM, use_colour)}"
        )

        word_children_count = len(word.fragments) + len(word.nested_commands)
        child_idx = 0

        for fragment in word.fragments:
            frag_last = child_idx == (word_children_count - 1)
            child_idx += 1
            frag_connector = "└── " if frag_last else "├── "
            frag_prefix = nested_prefix + frag_connector

            frag_span = line_index.format_range(fragment.start_offset, fragment.end_offset)
            colour = TOKEN_COLOURS.get(fragment.type, Ansi.GREEN)
            text = f"{fragment.type.name} '{preview(fragment.text)}'"
            print(
                f"{frag_prefix}{style(text, colour, use_colour)} {style(f'[{frag_span}]', Ansi.DIM, use_colour)}"
            )

        for j, nested_cmd in enumerate(word.nested_commands):
            nested_is_last = (child_idx + j) == (word_children_count - 1)
            _print_command(
                nested_cmd,
                prefix=nested_prefix,
                is_last=nested_is_last,
                diagnostics=diagnostics,
                line_index=line_index,
                use_colour=use_colour,
                is_nested=True,
            )


def make_annotations(
    source: str,
    diagnostics: list[Diagnostic],
    *,
    include_separators: bool,
) -> list[Annotation]:
    anns: list[Annotation] = []
    top_tokens = token_views(source)

    for tok in top_tokens:
        if tok.type in (TokenType.EOL,):
            continue
        if not include_separators and tok.type in (TokenType.SEP,):
            continue
        if tok.end_offset < tok.start_offset:
            continue
        label = f"token {tok.type.name}: {preview(tok.text, limit=32)}"
        anns.append(
            Annotation(
                start_offset=tok.start_offset,
                end_offset=tok.end_offset,
                label=label,
                colour=TOKEN_COLOURS.get(tok.type, Ansi.GREEN),
            )
        )

    for d in diagnostics:
        sev = d.severity.name
        code = f" {d.code}" if d.code else ""
        label = f"diag {sev}{code}: {preview(d.message, limit=64)}"
        anns.append(
            Annotation(
                start_offset=d.range.start.offset,
                end_offset=d.range.end.offset,
                label=label,
                colour=SEVERITY_COLOURS.get(d.severity, Ansi.RED),
            )
        )

    anns.sort(key=lambda a: (a.start_offset, a.end_offset - a.start_offset, a.label))
    return anns


def print_marked_source(
    source: str, diagnostics: list[Diagnostic], use_colour: bool, include_separators: bool
) -> None:
    print()
    print(style("source-markup", Ansi.BOLD, use_colour))

    line_index = LineIndex(source)
    annotations = make_annotations(source, diagnostics, include_separators=include_separators)
    line_no_width = max(2, len(str(line_index.line_count())))

    for line_number in range(line_index.line_count()):
        text = line_index.line_text(line_number)
        gutter = str(line_number + 1).rjust(line_no_width)
        print(f"{style(gutter, Ansi.DIM, use_colour)} | {text}")

        line_start = line_index.line_start(line_number)
        line_end_exclusive = line_index.line_end_exclusive(line_number)
        line_end_inclusive = max(line_start, line_end_exclusive - 1)

        line_annotations = [
            a
            for a in annotations
            if not (a.end_offset < line_start or a.start_offset > line_end_inclusive)
        ]
        for ann in line_annotations:
            seg_start = max(ann.start_offset, line_start)
            seg_end = min(ann.end_offset, line_end_inclusive)
            start_col = seg_start - line_start
            end_col = max(start_col, seg_end - line_start)
            marker = (" " * start_col) + "^" + ("-" * max(0, end_col - start_col))
            marker_line = f"{' ' * line_no_width} | {marker} {ann.label}"
            print(style(marker_line, ann.colour, use_colour))


def load_source(path: str | None, source_arg: str | None) -> str:
    if source_arg is not None:
        return source_arg

    if path and path != "-":
        return Path(path).read_text(encoding="utf-8")

    if path == "-" or not sys.stdin.isatty():
        return sys.stdin.read()

    raise ValueError("No Tcl input provided. Use a file path, --source, or pipe stdin.")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Inspect Tcl syntax and diagnostics as a tree and a marker-annotated source view."
        )
    )
    parser.add_argument(
        "path",
        nargs="?",
        help="Path to Tcl file to inspect, or '-' to read from stdin.",
    )
    parser.add_argument(
        "--source",
        help="Inline Tcl source to inspect.",
    )
    parser.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable ANSI colours.",
    )
    parser.add_argument(
        "--show-separators",
        action="store_true",
        help="Include SEP tokens in marker output.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    use_colour = (not args.no_colour) and sys.stdout.isatty()

    try:
        source = load_source(args.path, args.source)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    commands = build_commands(source)
    analysis = analyse(source)
    line_index = LineIndex(source)

    print_tree(commands, analysis.diagnostics, line_index, use_colour=use_colour)
    print_marked_source(
        source,
        analysis.diagnostics,
        use_colour=use_colour,
        include_separators=args.show_separators,
    )

    print()
    diag_count = len(analysis.diagnostics)
    summary = f"commands={len(commands)} diagnostics={diag_count}"
    print(style(summary, Ansi.BOLD, use_colour))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
