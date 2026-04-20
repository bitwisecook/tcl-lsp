"""Parser-backed iRules object reference extraction."""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache

from core.analysis.semantic_model import Range
from core.bigip.registry.data import OBJECT_KIND_SPECS
from core.commands.registry.runtime import ArgRole, arg_indices_for_role
from core.common.ranges import position_from_relative
from core.parsing.command_segmenter import SegmentedCommand, segment_commands
from core.parsing.lexer import TclLexer
from core.parsing.token_positions import token_content_base
from core.parsing.tokens import Token, TokenType

_CLASS_OPERATORS = frozenset(
    {"equals", "starts_with", "ends_with", "contains", "matches_glob", "matches_regex"}
)

_PERSIST_NON_REFERENCE_KEYWORDS = frozenset(
    {
        "add",
        "delete",
        "lookup",
        "replace",
        "none",
        "simple",
        "sticky",
        "source_addr",
        "dest_addr",
        "hash",
        "cookie",
        "ssl",
        "msrdp",
        "sip",
        "uie",
        "carp",
        "universal",
    }
)

_POOL_NON_REFERENCE_KEYWORDS = frozenset({"member", "members"})


@dataclass(frozen=True, slots=True)
class IrulesObjectReference:
    """One iRules object reference resolved from a literal command argument."""

    name: str
    kinds: tuple[str, ...]
    command: str
    argument_index: int  # 0-based index after command name
    range: Range


@lru_cache(maxsize=1)
def _gtm_pool_kinds() -> tuple[str, ...]:
    """Return all GTM pool object kinds from the object registry."""
    kinds: list[str] = []
    for kind, spec in OBJECT_KIND_SPECS.items():
        if spec.module != "gtm":
            continue
        if any(obj_type.startswith("pool ") for obj_type in spec.object_types):
            kinds.append(kind)
    return tuple(sorted(kinds))


def _pool_kinds_for_module(rule_module: str | None) -> tuple[str, ...]:
    if rule_module == "gtm":
        return _gtm_pool_kinds()
    return ("ltm_pool",)


def _normalise_literal_name(text: str) -> str | None:
    """Normalise a literal word into a potential BIG-IP object name."""
    clean = text.strip()
    if not clean:
        return None
    if clean.startswith("$") or clean.startswith("["):
        return None
    if any(ch.isspace() for ch in clean):
        return None
    return clean


def _literal_arg_value(cmd: SegmentedCommand, arg_index: int) -> tuple[str, Range] | None:
    """Return normalised literal argument text and source range for one arg."""
    word_index = arg_index + 1
    if word_index >= len(cmd.argv):
        return None
    if word_index >= len(cmd.single_token_word) or not cmd.single_token_word[word_index]:
        return None
    tok = cmd.argv[word_index]
    if tok.type not in (TokenType.ESC, TokenType.STR):
        return None

    raw = tok.text
    left = 0
    right = len(raw) - 1
    while left <= right and raw[left].isspace():
        left += 1
    while right >= left and raw[right].isspace():
        right -= 1
    if left > right:
        return None

    base_offset, base_line, base_col = token_content_base(tok)
    start = position_from_relative(
        raw,
        left,
        base_line=base_line,
        base_col=base_col,
        base_offset=base_offset,
    )
    end = position_from_relative(
        raw,
        right,
        base_line=base_line,
        base_col=base_col,
        base_offset=base_offset,
    )
    name = _normalise_literal_name(raw[left : right + 1])
    if name is None:
        return None
    return (name, Range(start=start, end=end))


def _append_reference(
    out: list[IrulesObjectReference],
    cmd: SegmentedCommand,
    *,
    arg_index: int,
    kinds: tuple[str, ...],
) -> None:
    literal = _literal_arg_value(cmd, arg_index)
    if literal is None:
        return
    name, rng = literal
    out.append(
        IrulesObjectReference(
            name=name,
            kinds=kinds,
            command=cmd.name,
            argument_index=arg_index,
            range=rng,
        )
    )


def _collect_class_references(cmd: SegmentedCommand, out: list[IrulesObjectReference]) -> None:
    args = cmd.args
    if not args:
        return
    sub = args[0].lower()
    dg_kinds = ("ltm_data_group_internal", "ltm_data_group_external")

    if sub in {"match", "search"}:
        idx = 1
        while idx < len(args) and args[idx].startswith("-"):
            idx += 1
        if idx < len(args) and args[idx] == "--":
            idx += 1
        idx += 1  # item
        if idx >= len(args) or args[idx].lower() not in _CLASS_OPERATORS:
            return
        idx += 1
        if idx < len(args):
            _append_reference(out, cmd, arg_index=idx, kinds=dg_kinds)
        return

    if sub == "lookup":
        idx = 1
        if idx < len(args) and args[idx] == "--":
            idx += 1
        idx += 1  # key argument
        if idx < len(args):
            _append_reference(out, cmd, arg_index=idx, kinds=dg_kinds)
        return

    if sub in {"exists", "size", "type", "get", "startsearch"} and len(args) >= 2:
        _append_reference(out, cmd, arg_index=1, kinds=dg_kinds)


def _collect_command_references(
    cmd: SegmentedCommand,
    out: list[IrulesObjectReference],
    *,
    rule_module: str | None,
) -> None:
    name = cmd.name
    lower_name = name.lower()
    args = cmd.args
    if not args:
        return
    pool_kinds = _pool_kinds_for_module(rule_module)

    match lower_name:
        case "pool":
            if args[0].lower() not in _POOL_NON_REFERENCE_KEYWORDS:
                _append_reference(out, cmd, arg_index=0, kinds=pool_kinds)
        case "active_members" | "members":
            _append_reference(out, cmd, arg_index=0, kinds=pool_kinds)
        case "snatpool":
            if rule_module != "gtm":
                _append_reference(out, cmd, arg_index=0, kinds=("ltm_snatpool",))
        case "virtual":
            if rule_module != "gtm":
                _append_reference(out, cmd, arg_index=0, kinds=("ltm_virtual",))
        case "node":
            if rule_module != "gtm":
                _append_reference(out, cmd, arg_index=0, kinds=("ltm_node",))
        case "clone":
            _append_reference(out, cmd, arg_index=0, kinds=pool_kinds)
            if len(args) >= 2:
                _append_reference(out, cmd, arg_index=1, kinds=pool_kinds)
        case "snat":
            if rule_module != "gtm" and len(args) >= 2 and args[0].lower() == "pool":
                _append_reference(out, cmd, arg_index=1, kinds=("ltm_snatpool",))
        case "persist":
            if rule_module != "gtm":
                head = args[0].lower()
                if head not in _PERSIST_NON_REFERENCE_KEYWORDS or args[0].startswith("/"):
                    _append_reference(
                        out,
                        cmd,
                        arg_index=0,
                        kinds=(
                            "ltm_persistence_cookie",
                            "ltm_persistence_dest_addr",
                            "ltm_persistence_global_settings",
                            "ltm_persistence_hash",
                            "ltm_persistence_host",
                            "ltm_persistence_msrdp",
                            "ltm_persistence_persist_records",
                            "ltm_persistence_sip",
                            "ltm_persistence_source_addr",
                            "ltm_persistence_ssl",
                            "ltm_persistence_universal",
                        ),
                    )
        case "class":
            _collect_class_references(cmd, out)
        case "matchclass":
            if len(args) >= 2:
                _append_reference(
                    out,
                    cmd,
                    arg_index=len(args) - 1,
                    kinds=("ltm_data_group_internal", "ltm_data_group_external"),
                )
        case "lb::reselect":
            if len(args) >= 2:
                sub = args[0].lower()
                if sub == "pool":
                    _append_reference(out, cmd, arg_index=1, kinds=pool_kinds)
                elif sub == "snatpool" and rule_module != "gtm":
                    _append_reference(out, cmd, arg_index=1, kinds=("ltm_snatpool",))
                elif sub == "virtual" and rule_module != "gtm":
                    _append_reference(out, cmd, arg_index=1, kinds=("ltm_virtual",))
        case "lb::status":
            if len(args) >= 2 and args[0].lower() == "pool":
                _append_reference(out, cmd, arg_index=1, kinds=pool_kinds)
        case _:
            return


def _walk_irules_commands(
    source: str,
    out: list[IrulesObjectReference],
    *,
    body_token: Token | None = None,
    rule_module: str | None = None,
) -> None:
    commands = segment_commands(source, body_token=body_token)
    for cmd in commands:
        _collect_command_references(cmd, out, rule_module=rule_module)

        body_indices = arg_indices_for_role(cmd.name, cmd.args, ArgRole.BODY)
        expr_indices = arg_indices_for_role(cmd.name, cmd.args, ArgRole.EXPR)
        recursed_cmd_tokens: set[tuple[int, int]] = set()
        for body_idx in body_indices:
            word_index = body_idx + 1
            if word_index >= len(cmd.argv):
                continue
            tok = cmd.argv[word_index]
            if tok.type not in (TokenType.STR, TokenType.CMD):
                continue
            if not tok.text.strip():
                continue
            _walk_irules_commands(tok.text, out, body_token=tok, rule_module=rule_module)
            if tok.type is TokenType.CMD:
                recursed_cmd_tokens.add((tok.start.offset, tok.end.offset))

        for expr_idx in expr_indices:
            word_index = expr_idx + 1
            if word_index >= len(cmd.argv):
                continue
            tok = cmd.argv[word_index]
            if tok.type not in (TokenType.STR, TokenType.ESC):
                continue
            base_offset, base_line, base_col = token_content_base(tok)
            lexer = TclLexer(
                tok.text,
                base_offset=base_offset,
                base_line=base_line,
                base_col=base_col,
            )
            while True:
                inner = lexer.get_token()
                if inner is None:
                    break
                if inner.type is not TokenType.CMD or not inner.text.strip():
                    continue
                key = (inner.start.offset, inner.end.offset)
                if key in recursed_cmd_tokens:
                    continue
                recursed_cmd_tokens.add(key)
                _walk_irules_commands(inner.text, out, body_token=inner, rule_module=rule_module)

        for tok in cmd.all_tokens:
            if tok.type is not TokenType.CMD:
                continue
            key = (tok.start.offset, tok.end.offset)
            if key in recursed_cmd_tokens:
                continue
            if not tok.text.strip():
                continue
            _walk_irules_commands(tok.text, out, body_token=tok, rule_module=rule_module)


def extract_irules_object_references(
    source: str,
    *,
    body_token: Token | None = None,
    rule_module: str | None = None,
) -> list[IrulesObjectReference]:
    """Extract BIG-IP object references from iRules source using Tcl parsing."""
    refs: list[IrulesObjectReference] = []
    _walk_irules_commands(source, refs, body_token=body_token, rule_module=rule_module)
    refs.sort(
        key=lambda ref: (
            ref.range.start.line,
            ref.range.start.character,
            ref.range.end.line,
            ref.range.end.character,
            ref.name,
        )
    )
    return refs
