"""Code actions provider -- quick fixes from diagnostics."""

from __future__ import annotations

import re
from textwrap import dedent

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, CodeFix, ProcDef
from core.commands.registry import REGISTRY
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import is_irules_dialect
from core.common.ip_utils import ipv4_to_ipv6_mapped, parse_ip
from core.common.lsp import to_lsp_range
from core.common.position import find_command_at_position, offset_at_position
from core.common.ranges import position_from_relative
from core.compiler.irules_flow import DATA_EVENT_REQUIREMENTS, find_irules_flow_warnings
from core.compiler.optimiser import demorgan_transform, invert_expression
from core.parsing.command_segmenter import segment_commands
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType
from core.refactoring import RefactoringResult
from core.refactoring._brace_expr import brace_expr
from core.refactoring._extract_datagroup import extract_to_datagroup
from core.refactoring._extract_variable import extract_variable
from core.refactoring._if_to_switch import if_to_switch
from core.refactoring._inline_variable import inline_variable
from core.refactoring._switch_to_dict import switch_to_dict

from .diagnostics import _check_comment_continuation, _check_trailing_whitespace
from .irules_context import find_enclosing_when_event
from .package_suggestions import rank_package_suggestions
from .symbol_resolution import find_word_at_position as _find_word_at_position

# Short names: d = Diagnostic.

_TAINT_VAR_RE = re.compile(r"Tainted variable \$(\w+)")
_DOUBLE_ENCODE_VAR_RE = re.compile(r"Variable \$(\w+) is already")
_IRULE_COLLECT_RE = re.compile(r"\b([A-Za-z0-9_]+)::collect\b")
_IRULE_PAYLOAD_RE = re.compile(r"\b([A-Za-z0-9_]+)::payload\b")
_SETUP_EVENT_BY_DATA_EVENT = {
    "CLIENT_DATA": "CLIENT_ACCEPTED",
    "SERVER_DATA": "SERVER_CONNECTED",
    "HTTP_REQUEST_DATA": "HTTP_REQUEST",
    "HTTP_RESPONSE_DATA": "HTTP_RESPONSE",
    "CLIENTSSL_DATA": "CLIENTSSL_HANDSHAKE",
    "SERVERSSL_DATA": "SERVERSSL_HANDSHAKE",
}
_SERVER_EVENT_PREFIXES = ("SERVER", "SERVERSSL")
_EXTRACT_PROC_BASE_NAME = "extracted_proc"
_PLAIN_VAR_RE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)\b")
_BRACED_VAR_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")


def _diag_code(value: object) -> str:
    if isinstance(value, str):
        return value
    return str(value) if value is not None else ""


def _kind_value(kind: types.CodeActionKind | str) -> str:
    if isinstance(kind, str):
        return kind
    return kind.value


def _context_allows_kind(
    context: types.CodeActionContext,
    kind: types.CodeActionKind | str,
) -> bool:
    if not context.only:
        return True
    kind_value = _kind_value(kind)
    for requested in context.only:
        requested_value = _kind_value(requested)
        if kind_value == requested_value or kind_value.startswith(f"{requested_value}."):
            return True
    return False


def get_code_actions(
    source: str,
    range_: types.Range,
    context: types.CodeActionContext,
    *,
    package_names: list[str] | None = None,
    lines: list[str] | None = None,
) -> list[types.CodeAction]:
    """Return code actions (quick fixes) for the given range and diagnostics.

    We match incoming LSP diagnostics (by code + range) against the analysis
    result to find the original Diagnostic objects that carry CodeFix data.
    """
    result = analyse(source)

    # Build a lookup: (code, start_line, start_char) -> list of CodeFix
    fix_lookup: dict[tuple[str, int, int], list[CodeFix]] = {}
    for d in result.diagnostics:
        if d.fixes:
            key = (d.code, d.range.start.line, d.range.start.character)
            fix_lookup[key] = list(d.fixes)

    # Also collect fixes from iRules flow warnings (IRULE5002, IRULE5004, etc.)
    for fw in find_irules_flow_warnings(source):
        if fw.fixes:
            key = (fw.code, fw.range.start.line, fw.range.start.character)
            fix_lookup.setdefault(key, []).extend(fw.fixes)

    # W112 trailing-whitespace fixes (source-level, not from analyser).
    for d in _check_trailing_whitespace(source, lines=lines):
        if d.fixes:
            key = (d.code, d.range.start.line, d.range.start.character)
            fix_lookup.setdefault(key, []).extend(d.fixes)

    # W115 comment-continuation fixes (source-level).
    for d in _check_comment_continuation(source, lines=lines):
        if d.fixes:
            key = (d.code, d.range.start.line, d.range.start.character)
            fix_lookup.setdefault(key, []).extend(d.fixes)

    actions: list[types.CodeAction] = []
    collect_action_keys: set[tuple[int, str]] = set()
    diagnostics = list(context.diagnostics or [])
    for lsp_diag in diagnostics:
        code = _diag_code(lsp_diag.code)

        # Handle optimiser diagnostics (O-codes) -- replacement comes
        # from the diagnostic's ``data`` field set by the diagnostics provider.
        if (
            code.startswith("O")
            and isinstance(lsp_diag.data, dict)
            and "replacement" in lsp_diag.data
            and not lsp_diag.data.get("hintOnly", False)
        ):
            group_edits_raw = lsp_diag.data.get("groupEdits")
            if isinstance(group_edits_raw, list) and group_edits_raw:
                # Grouped optimisation: apply all edits in the group.
                edits: list[types.TextEdit] = []
                for ge in group_edits_raw:
                    edits.append(
                        types.TextEdit(
                            range=types.Range(
                                start=types.Position(
                                    line=ge["startLine"],
                                    character=ge["startCharacter"],
                                ),
                                end=types.Position(
                                    line=ge["endLine"],
                                    character=ge["endCharacter"],
                                ),
                            ),
                            new_text=ge["replacement"],
                        )
                    )
            else:
                edits = [
                    types.TextEdit(
                        range=lsp_diag.range,
                        new_text=lsp_diag.data["replacement"],
                    )
                ]
            action = types.CodeAction(
                title=lsp_diag.message,
                kind=types.CodeActionKind.QuickFix,
                diagnostics=[lsp_diag],
                edit=types.WorkspaceEdit(
                    changes={"__current__": edits},
                ),
            )
            actions.append(action)
            continue

        for title, insert_line, snippet in _irules_collect_bootstrap_actions(source, lsp_diag):
            key = (insert_line, snippet)
            if key in collect_action_keys:
                continue
            collect_action_keys.add(key)
            edit = types.TextEdit(
                range=types.Range(
                    start=types.Position(line=insert_line, character=0),
                    end=types.Position(line=insert_line, character=0),
                ),
                new_text=snippet,
            )
            actions.append(
                types.CodeAction(
                    title=title,
                    kind=types.CodeActionKind.QuickFix,
                    diagnostics=[lsp_diag],
                    edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
                )
            )

        actions.extend(_catch_result_variable_actions(source, lsp_diag))
        actions.extend(_taint_quick_fix_actions(source, lsp_diag, lines=lines))

        # W120: offer to insert the missing ``package require``.
        if code == "W120":
            actions.extend(_missing_package_require_action(source, lsp_diag))

        key = (code, lsp_diag.range.start.line, lsp_diag.range.start.character)
        fixes = fix_lookup.get(key, [])
        for fix in fixes:
            edit = types.TextEdit(
                range=types.Range(
                    start=types.Position(
                        line=fix.range.start.line,
                        character=fix.range.start.character,
                    ),
                    end=types.Position(
                        line=fix.range.end.line,
                        character=fix.range.end.character + 1,
                    ),
                ),
                new_text=fix.new_text,
            )
            action = types.CodeAction(
                title=fix.description or f"Fix: {lsp_diag.message[:60]}",
                kind=types.CodeActionKind.QuickFix,
                diagnostics=[lsp_diag],
                edit=types.WorkspaceEdit(
                    changes={"__current__": [edit]},
                ),
            )
            actions.append(action)

    extract_action = _extract_proc_action(source, range_, analysis=result)
    if extract_action is not None:
        actions.append(extract_action)

    inline_action = _inline_proc_action(source, range_, analysis=result)
    if inline_action is not None:
        actions.append(inline_action)

    actions.extend(_expr_refactor_actions(source, range_))
    actions.extend(_ip_conversion_actions(source, range_))
    actions.extend(_package_require_actions(source, range_, context, package_names or []))

    # New refactoring actions.
    actions.extend(_new_refactor_actions(source, range_))

    profiles_action = _generate_profiles_action(source, result)
    if profiles_action is not None:
        actions.append(profiles_action)

    if context.only:
        return [
            action
            for action in actions
            if action.kind and _context_allows_kind(context, action.kind)
        ]
    return actions


def _catch_result_variable_actions(
    source: str,
    diag: types.Diagnostic,
) -> list[types.CodeAction]:
    if _diag_code(diag.code) != "W302":
        return []

    cmd = find_command_at_position(source, diag.range.start.line, diag.range.start.character)
    if cmd is None or not cmd.texts:
        return []
    if cmd.texts[0] != "catch" or len(cmd.texts) != 2:
        return []

    script_token = cmd.argv[1]
    insert_offset = script_token.end.offset + 1
    if script_token.type in (TokenType.STR, TokenType.CMD):
        insert_offset += 1
    insert_offset = max(0, min(insert_offset, len(source)))
    insert_pos = position_from_relative(
        source,
        insert_offset,
        base_line=0,
        base_col=0,
        base_offset=0,
    )

    if not cmd.texts[1].strip():
        return []

    actions: list[types.CodeAction] = []
    for title, suffix in (
        ("Add catch result variable", " result"),
        ("Add catch result + options variables", " result opts"),
    ):
        actions.append(
            types.CodeAction(
                title=title,
                kind=types.CodeActionKind.QuickFix,
                diagnostics=[diag],
                edit=types.WorkspaceEdit(
                    changes={
                        "__current__": [
                            types.TextEdit(
                                range=types.Range(
                                    start=types.Position(
                                        line=insert_pos.line,
                                        character=insert_pos.character,
                                    ),
                                    end=types.Position(
                                        line=insert_pos.line,
                                        character=insert_pos.character,
                                    ),
                                ),
                                new_text=suffix,
                            ),
                        ],
                    },
                ),
            )
        )
    return actions


def _word_at_position(source: str, line: int, character: int) -> str:
    return _find_word_at_position(source, line, character) or ""


def _package_insert_line(source: str, *, lines: list[str] | None = None) -> int:
    if lines is None:
        lines = source.split("\n")
    line = 0
    if lines and lines[0].startswith("#!"):
        line = 1
    while line < len(lines) and re.match(r"^\s*package\s+require\b", lines[line]):
        line += 1
    return line


_W120_PKG_RE = re.compile(r"`package require (\S+)`")


def _missing_package_require_action(
    source: str,
    diag: types.Diagnostic,
) -> list[types.CodeAction]:
    """Create a quick-fix that inserts ``package require <pkg>`` for W120."""
    match = _W120_PKG_RE.search(diag.message or "")
    if match is None:
        return []
    pkg = match.group(1)
    # Don't offer the fix if the import already exists (e.g. added since the
    # diagnostic was computed).
    if re.search(
        rf"^\s*package\s+require\s+{re.escape(pkg)}(?:\s|$)",
        source,
        flags=re.MULTILINE,
    ):
        return []
    insert_line = _package_insert_line(source)
    edit = types.TextEdit(
        range=types.Range(
            start=types.Position(line=insert_line, character=0),
            end=types.Position(line=insert_line, character=0),
        ),
        new_text=f"package require {pkg}\n",
    )
    return [
        types.CodeAction(
            title=f"Add 'package require {pkg}'",
            kind=types.CodeActionKind.QuickFix,
            diagnostics=[diag],
            edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
        )
    ]


def _package_require_actions(
    source: str,
    range_: types.Range,
    context: types.CodeActionContext,
    package_names: list[str],
) -> list[types.CodeAction]:
    if not package_names:
        return []

    word = _word_at_position(source, range_.start.line, range_.start.character)
    if not word:
        return []

    prefix = word.split("::", 1)[0].lower()
    if len(prefix) < 2:
        return []

    ranked = rank_package_suggestions(word, package_names, 5)
    if not ranked:
        return []

    insert_line = _package_insert_line(source)
    existing = source
    actions: list[types.CodeAction] = []
    diagnostics = list(context.diagnostics or [])
    for package_name in ranked:
        if re.search(
            rf"^\s*package\s+require\s+{re.escape(package_name)}(?:\s|$)",
            existing,
            flags=re.MULTILINE,
        ):
            continue
        edit = types.TextEdit(
            range=types.Range(
                start=types.Position(line=insert_line, character=0),
                end=types.Position(line=insert_line, character=0),
            ),
            new_text=f"package require {package_name}\n",
        )
        action = types.CodeAction(
            title=f"Add 'package require {package_name}'",
            kind=(
                types.CodeActionKind.QuickFix
                if diagnostics
                else types.CodeActionKind.RefactorRewrite
            ),
            diagnostics=diagnostics or None,
            edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
        )
        actions.append(action)

    return actions


def _event_from_message(message: str) -> str | None:
    match = re.search(r"'([A-Za-z0-9_]+)'\s+will never fire", message)
    if match:
        return match.group(1).upper()
    return None


def _payload_protocol_from_diagnostic(source: str, diag: types.Diagnostic) -> str | None:
    word = _word_at_position(source, diag.range.start.line, diag.range.start.character)
    match = _IRULE_PAYLOAD_RE.search(word)
    if match:
        return match.group(1).upper()

    match = _IRULE_PAYLOAD_RE.search(diag.message)
    if match:
        return match.group(1).upper()
    return None


def _setup_event_for_protocol(protocol: str, current_event: str | None) -> str:
    event = (current_event or "").upper()
    if event in _SETUP_EVENT_BY_DATA_EVENT:
        return _SETUP_EVENT_BY_DATA_EVENT[event]

    if protocol == "HTTP":
        return "HTTP_RESPONSE" if event.startswith("HTTP_RESPONSE") else "HTTP_REQUEST"
    if protocol == "SSL":
        return "SERVERSSL_HANDSHAKE" if event.startswith("SERVERSSL") else "CLIENTSSL_HANDSHAKE"
    if event.startswith(_SERVER_EVENT_PREFIXES):
        return "SERVER_CONNECTED"
    return "CLIENT_ACCEPTED"


def _collect_bootstrap_snippet(setup_event: str, protocol: str) -> str:
    return f"when {setup_event} priority 500 {{\n    {protocol}::collect\n}}\n\n"


def _irules_collect_bootstrap_actions(
    source: str,
    diag: types.Diagnostic,
) -> list[tuple[str, int, str]]:
    code = _diag_code(diag.code)
    if code not in {"IRULE1005", "IRULE1006"}:
        return []

    event_name, event_line = find_enclosing_when_event(source, diag.range.start.line)
    anchor_line = event_line

    protocols: list[str] = []
    if code == "IRULE1005":
        data_event = _word_at_position(source, diag.range.start.line, diag.range.start.character)
        if not data_event:
            data_event = _event_from_message(diag.message) or ""
        data_event = data_event.upper()
        if data_event:
            event_name = data_event
        protocols.extend(
            match.group(1).upper() for match in _IRULE_COLLECT_RE.finditer(diag.message)
        )
        if not protocols and data_event in DATA_EVENT_REQUIREMENTS:
            protocols.extend(DATA_EVENT_REQUIREMENTS[data_event][0])
    else:
        protocol = _payload_protocol_from_diagnostic(source, diag)
        if protocol:
            protocols.append(protocol)

    if not protocols:
        return []
    protocols = list(dict.fromkeys(protocols))

    actions: list[tuple[str, int, str]] = []
    for protocol in protocols:
        setup_event = _setup_event_for_protocol(protocol, event_name)
        snippet = _collect_bootstrap_snippet(setup_event, protocol)
        title = f"Add '{protocol}::collect' bootstrap in '{setup_event}'"
        actions.append((title, anchor_line, snippet))
    return actions


def _slice_lsp_range(source: str, range_: types.Range) -> str:
    start = offset_at_position(source, range_.start.line, range_.start.character)
    end = offset_at_position(source, range_.end.line, range_.end.character)
    if end < start:
        return ""
    return source[start:end]


def _line_indent(source: str, line: int, *, lines: list[str] | None = None) -> str:
    if lines is None:
        lines = source.split("\n")
    if line < 0 or line >= len(lines):
        return ""
    text = lines[line]
    return text[: len(text) - len(text.lstrip())]


def _dedent_block(text: str) -> str:
    lines = text.split("\n")
    non_empty = [line for line in lines if line.strip()]
    if not non_empty:
        return ""
    min_indent = min(len(line) - len(line.lstrip()) for line in non_empty)
    return "\n".join(line[min_indent:] if line.strip() else "" for line in lines)


def _next_extract_proc_name(analysis) -> str:
    existing = {proc.name for proc in analysis.all_procs.values()}
    if _EXTRACT_PROC_BASE_NAME not in existing:
        return _EXTRACT_PROC_BASE_NAME
    index = 2
    while True:
        name = f"{_EXTRACT_PROC_BASE_NAME}_{index}"
        if name not in existing:
            return name
        index += 1


def _selection_param_names(selection: str) -> list[str]:
    names: list[str] = []
    seen: set[str] = set()
    for match in _BRACED_VAR_RE.finditer(selection):
        name = match.group(1)
        if name in seen:
            continue
        seen.add(name)
        names.append(name)
    for match in _PLAIN_VAR_RE.finditer(selection):
        name = match.group(1)
        if name in seen:
            continue
        seen.add(name)
        names.append(name)
    return names


def _extract_proc_action(
    source: str,
    range_: types.Range,
    *,
    analysis,
) -> types.CodeAction | None:
    if range_.start.line == range_.end.line and range_.start.character == range_.end.character:
        return None

    selection = _slice_lsp_range(source, range_)
    if not selection.strip():
        return None

    proc_name = _next_extract_proc_name(analysis)
    params = _selection_param_names(selection)

    dedented = _dedent_block(selection).strip("\n")
    if not dedented.strip():
        return None
    indented_body = "\n".join(f"    {line}" if line else "" for line in dedented.split("\n"))
    param_list = " ".join(params)
    proc_text = f"proc {proc_name} {{{param_list}}} {{\n{indented_body}\n}}\n\n"

    call = proc_name
    if params:
        call = f"{call} {' '.join(f'${name}' for name in params)}"
    replacement = call
    if range_.start.character == 0:
        replacement = f"{_line_indent(source, range_.start.line)}{call}"
        if selection.endswith("\n"):
            replacement += "\n"

    insert_line = _package_insert_line(source)
    edits = [
        types.TextEdit(
            range=types.Range(
                start=types.Position(line=insert_line, character=0),
                end=types.Position(line=insert_line, character=0),
            ),
            new_text=proc_text,
        ),
        types.TextEdit(range=range_, new_text=replacement),
    ]

    # After the edit is applied the proc name sits at the insert line,
    # starting at column 5 (len("proc ")).  Attach a client-side command
    # so the editor can select the name and trigger a rename.
    name_start = len("proc ")
    name_end = name_start + len(proc_name)
    rename_command = types.Command(
        title="Rename extracted proc",
        command="tclLsp.renameSymbolAtPosition",
        arguments=[insert_line, name_start, name_end],
    )

    return types.CodeAction(
        title=f"Extract selection into proc '{proc_name}'",
        kind=types.CodeActionKind.RefactorExtract,
        edit=types.WorkspaceEdit(changes={"__current__": edits}),
        command=rename_command,
    )


def _proc_definition_for_call(analysis, call_name: str) -> tuple[str, ProcDef] | None:
    for qname, proc_def in analysis.all_procs.items():
        if call_name == proc_def.name or call_name == qname or call_name == f"::{proc_def.name}":
            return (qname, proc_def)
    return None


def _proc_body_from_source(source: str, proc_def: ProcDef) -> str | None:
    for cmd in segment_commands(source):
        if not cmd.texts or cmd.texts[0] != "proc" or len(cmd.texts) < 4:
            continue
        if cmd.texts[1] != proc_def.name:
            continue
        if cmd.range.start.line <= proc_def.name_range.start.line <= cmd.range.end.line:
            return cmd.texts[3]
    return None


def _inline_argument_map(proc_def: ProcDef, call_args: list[str]) -> dict[str, str] | None:
    if proc_def.params and proc_def.params[-1].name == "args":
        return None
    if len(call_args) > len(proc_def.params):
        return None

    mapping: dict[str, str] = {}
    for idx, param in enumerate(proc_def.params):
        if idx < len(call_args):
            mapping[param.name] = call_args[idx]
            continue
        if param.has_default:
            mapping[param.name] = param.default_value
            continue
        return None
    return mapping


def _substitute_proc_params(text: str, mapping: dict[str, str]) -> str:
    def replace_braced(match: re.Match[str]) -> str:
        name = match.group(1)
        return mapping.get(name, match.group(0))

    def replace_plain(match: re.Match[str]) -> str:
        name = match.group(1)
        return mapping.get(name, match.group(0))

    replaced = _BRACED_VAR_RE.sub(replace_braced, text)
    return _PLAIN_VAR_RE.sub(replace_plain, replaced)


def _inline_proc_action(
    source: str,
    range_: types.Range,
    *,
    analysis,
) -> types.CodeAction | None:
    cmd = find_command_at_position(source, range_.start.line, range_.start.character)
    if cmd is None or not cmd.texts:
        return None

    call_name = cmd.texts[0]
    # iRules ``call proc_name arg...`` — the proc name is texts[1].
    if call_name == "call" and len(cmd.texts) >= 2:
        call_name = cmd.texts[1]
        call_args = cmd.texts[2:]
    else:
        call_args = cmd.texts[1:]
    proc_pair = _proc_definition_for_call(analysis, call_name)
    if proc_pair is None:
        return None
    _qname, proc_def = proc_pair

    body_text = _proc_body_from_source(source, proc_def)
    if body_text is None:
        return None
    body_commands = segment_commands(body_text)
    if len(body_commands) != 1:
        return None
    body_cmd = body_commands[0]
    if not body_cmd.texts:
        return None
    if body_cmd.texts[0] in {
        "return",
        "break",
        "continue",
        "tailcall",
        "yield",
        "uplevel",
        "upvar",
        "global",
        "variable",
    }:
        return None

    arg_map = _inline_argument_map(proc_def, call_args)
    if arg_map is None:
        return None

    replacement = _substitute_proc_params(" ".join(body_cmd.texts), arg_map).strip()
    if not replacement:
        return None
    prefix = _line_indent(source, cmd.range.start.line)
    if prefix and not prefix.strip():
        replacement = f"{prefix}{replacement}"

    return types.CodeAction(
        title=f"Inline proc '{proc_def.name}'",
        kind=types.CodeActionKind.RefactorInline,
        edit=types.WorkspaceEdit(
            changes={
                "__current__": [
                    types.TextEdit(
                        range=to_lsp_range(cmd.range),
                        new_text=replacement,
                    ),
                ],
            },
        ),
    )


def _ip_conversion_actions(
    source: str,
    range_: types.Range,
) -> list[types.CodeAction]:
    """Offer IPv4\u2194IPv6-mapped conversion when the cursor is on an IP literal."""
    if range_.start.line != range_.end.line:
        return []

    lines = source.split("\n")
    if range_.start.line >= len(lines):
        return []
    line_text = lines[range_.start.line]

    # Extract word at cursor
    col = range_.start.character
    start = col
    while start > 0 and line_text[start - 1] not in " \t;{}[]\"'":
        start -= 1
    end = col
    while end < len(line_text) and line_text[end] not in " \t;{}[]\"'":
        end += 1
    word = line_text[start:end]
    if not word:
        return []

    info = parse_ip(word)
    if info is None:
        return []

    actions: list[types.CodeAction] = []
    word_range = types.Range(
        start=types.Position(line=range_.start.line, character=start),
        end=types.Position(line=range_.start.line, character=end),
    )

    # Preserve any /prefix suffix (e.g. 10.0.0.0/8 → ::ffff:10.0.0.0/8)
    suffix = ""
    if "/" in word:
        suffix = word[word.index("/") :]

    if info.version == 4:
        mapped = ipv4_to_ipv6_mapped(info.address)  # type: ignore[arg-type]
        new_text = mapped + suffix
        actions.append(
            types.CodeAction(
                title=f"Convert to IPv6-mapped \u2192 {new_text}",
                kind=types.CodeActionKind.RefactorRewrite,
                edit=types.WorkspaceEdit(
                    changes={"__current__": [types.TextEdit(range=word_range, new_text=new_text)]},
                ),
            )
        )
    elif info.version == 6 and info.ipv4_mapped is not None:
        v4 = str(info.ipv4_mapped)
        new_text = v4 + suffix
        actions.append(
            types.CodeAction(
                title=f"Convert to IPv4 \u2192 {new_text}",
                kind=types.CodeActionKind.RefactorRewrite,
                edit=types.WorkspaceEdit(
                    changes={"__current__": [types.TextEdit(range=word_range, new_text=new_text)]},
                ),
            )
        )

    return actions


def _expr_refactor_actions(
    source: str,
    range_: types.Range,
) -> list[types.CodeAction]:
    """Return De Morgan's and invert-expression refactoring actions for selections."""
    if range_.start.line == range_.end.line and range_.start.character == range_.end.character:
        return []

    selection = _slice_lsp_range(source, range_)
    if not selection.strip():
        return []

    actions: list[types.CodeAction] = []

    dm_result = demorgan_transform(selection)
    if dm_result is not None:
        title = f"De Morgan's law \u2192 {dm_result}"
        if len(title) > 80:
            title = "Apply De Morgan's law"
        actions.append(
            types.CodeAction(
                title=title,
                kind=types.CodeActionKind.RefactorRewrite,
                edit=types.WorkspaceEdit(
                    changes={"__current__": [types.TextEdit(range=range_, new_text=dm_result)]},
                ),
            )
        )

    inv_result = invert_expression(selection)
    if inv_result is not None:
        title = f"Invert expression \u2192 {inv_result}"
        if len(title) > 80:
            title = "Invert expression"
        actions.append(
            types.CodeAction(
                title=title,
                kind=types.CodeActionKind.RefactorRewrite,
                edit=types.WorkspaceEdit(
                    changes={"__current__": [types.TextEdit(range=range_, new_text=inv_result)]},
                ),
            )
        )

    return actions


# ── New refactoring actions ───────────────────────────────────────────

_REFACTOR_KIND_MAP: dict[str, types.CodeActionKind] = {
    "refactor.extract": types.CodeActionKind.RefactorExtract,
    "refactor.inline": types.CodeActionKind.RefactorInline,
    "refactor.rewrite": types.CodeActionKind.RefactorRewrite,
}


def _refactoring_to_code_action(
    result: RefactoringResult,
) -> types.CodeAction:
    """Convert a core :class:`RefactoringResult` to an LSP CodeAction."""
    edits = [
        types.TextEdit(
            range=types.Range(
                start=types.Position(line=e.start_line, character=e.start_character),
                end=types.Position(line=e.end_line, character=e.end_character),
            ),
            new_text=e.new_text,
        )
        for e in result.edits
    ]
    return types.CodeAction(
        title=result.title,
        kind=_REFACTOR_KIND_MAP.get(result.kind, types.CodeActionKind.RefactorRewrite),
        edit=types.WorkspaceEdit(changes={"__current__": edits}),
    )


def _datagroup_to_code_action(result) -> types.CodeAction:
    """Convert a :class:`DataGroupExtractionResult` to an LSP CodeAction.

    Only the iRule code rewrite is applied as a text edit.  The tmsh
    data-group definition is attached as structured data so tooling
    (MCP, AI, clipboard) can consume it without injecting comment
    blocks into the source file.
    """

    edits = []
    for e in result.edits:
        edits.append(
            types.TextEdit(
                range=types.Range(
                    start=types.Position(line=e.start_line, character=e.start_character),
                    end=types.Position(line=e.end_line, character=e.end_character),
                ),
                new_text=e.new_text,
            )
        )

    return types.CodeAction(
        title=result.title,
        kind=types.CodeActionKind.RefactorExtract,
        edit=types.WorkspaceEdit(changes={"__current__": edits}),
        data={"data_group_definition": result.data_group_tcl()},
    )


def _new_refactor_actions(
    source: str,
    range_: types.Range,
) -> list[types.CodeAction]:
    """Return refactoring actions from the new core refactoring engine."""
    actions: list[types.CodeAction] = []
    line = range_.start.line
    char = range_.start.character
    has_selection = (
        range_.start.line != range_.end.line or range_.start.character != range_.end.character
    )

    # Extract variable (requires a selection).
    if has_selection:
        ev = extract_variable(
            source,
            range_.start.line,
            range_.start.character,
            range_.end.line,
            range_.end.character,
        )
        if ev is not None:
            actions.append(_refactoring_to_code_action(ev))

    # Inline variable (cursor on a ``set`` command).
    iv = inline_variable(source, line, char)
    if iv is not None:
        actions.append(_refactoring_to_code_action(iv))

    # if/elseif chain → switch.
    its = if_to_switch(source, line, char)
    if its is not None:
        actions.append(_refactoring_to_code_action(its))

    # switch → dict lookup.
    std = switch_to_dict(source, line, char)
    if std is not None:
        actions.append(_refactoring_to_code_action(std))

    # Brace expr.
    be = brace_expr(source, line, char)
    if be is not None:
        actions.append(_refactoring_to_code_action(be))

    # Extract to data-group (iRules).
    if is_irules_dialect():
        dg = extract_to_datagroup(source, line, char)
        if dg is not None:
            actions.append(_datagroup_to_code_action(dg))

    return actions


# Taint quick-fix code actions

# Wrapping fixes: diagnostic code → (title, wrapper command)
_TAINT_WRAP_FIXES: dict[str, tuple[str, str]] = {
    "IRULE3001": ("Wrap ${var} with [html_encode]", "html_encode"),
    "IRULE3002": ("Wrap ${var} with [URI::encode]", "URI::encode"),
    "T103": ("Wrap ${var} with [regex::quote]", "regex::quote"),
}

# Encoder commands for T106 (double-encoding) code action.
_T106_ENCODERS = (
    "HTML::encode",
    "htmlencode",
    "html_escape",
    "html_encode",
    "URI::encode",
    "URI::encode_component",
    "URI::escape",
    "regex::quote",
    "regexp::quote",
    "re_quote",
    "regex_quote",
)

# Template proc definitions for helpers the code actions suggest.
# Keyed by proc name; value is the complete proc source (with trailing \n\n).
_HELPER_PROC_TEMPLATES: dict[str, str] = {
    "regex::quote": dedent("""\
        proc regex::quote {str} {
            regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&}
        }

    """),
    "html_encode": dedent("""\
        proc html_encode {str} {
            string map {& &amp; < &lt; > &gt; \\" &quot; ' &#39;} $str
        }

    """),
}


def _proc_defined_in_source(source: str, proc_name: str) -> bool:
    """Return True if *proc_name* is defined as a proc in *source*."""
    for cmd in segment_commands(source):
        if (
            cmd.texts
            and cmd.texts[0] == "proc"
            and len(cmd.texts) > 1
            and cmd.texts[1] == proc_name
        ):
            return True
    return False


def _find_var_in_line(
    line_text: str,
    var_name: str,
    *,
    start_char: int = 0,
) -> tuple[int, int] | None:
    """Find ``$var`` or ``${var}`` in *line_text* starting from *start_char*.

    Returns ``(start_col, end_col)`` of the full variable reference,
    or ``None`` if not found.
    """
    plain = f"${var_name}"
    braced = f"${{{var_name}}}"
    for pattern in (plain, braced):
        idx = line_text.find(pattern, start_char)
        if idx >= 0:
            # For plain $var, ensure the next char is not a word char
            # (so $raw doesn't match inside $rawdata).
            if pattern == plain:
                end = idx + len(pattern)
                if (
                    end < len(line_text)
                    and line_text[end]
                    in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"
                ):
                    # Try again after this position.
                    return _find_var_in_line(line_text, var_name, start_char=end)
            return (idx, idx + len(pattern))
    return None


def _taint_quick_fix_actions(
    source: str,
    diag: types.Diagnostic,
    *,
    lines: list[str] | None = None,
) -> list[types.CodeAction]:
    """Generate quick-fix code actions for taint diagnostics."""
    code = _diag_code(diag.code)
    actions: list[types.CodeAction] = []

    # Extract variable name from the diagnostic message.
    m = _TAINT_VAR_RE.search(diag.message)
    if m is None:
        m = _DOUBLE_ENCODE_VAR_RE.search(diag.message)
    if m is None:
        return actions
    var_name = m.group(1)

    if lines is None:
        lines = source.split("\n")
    diag_line = diag.range.start.line
    if diag_line >= len(lines):
        return actions
    line_text = lines[diag_line]

    # Find the variable reference in the source line.
    search_start = diag.range.start.character
    var_pos = _find_var_in_line(line_text, var_name, start_char=search_start)
    if var_pos is None:
        # Fall back to searching the whole line.
        var_pos = _find_var_in_line(line_text, var_name)
    if var_pos is None:
        return actions
    col_start, col_end = var_pos
    var_text = line_text[col_start:col_end]

    var_range = types.Range(
        start=types.Position(line=diag_line, character=col_start),
        end=types.Position(line=diag_line, character=col_end),
    )

    # Wrapping fixes (IRULE3001, IRULE3002, T103)
    if code in _TAINT_WRAP_FIXES:
        title_tmpl, wrapper = _TAINT_WRAP_FIXES[code]
        title = title_tmpl.replace("${var}", var_name)
        new_text = f"[{wrapper} {var_text}]"
        wrap_edit = types.TextEdit(range=var_range, new_text=new_text)
        edits: list[types.TextEdit] = [wrap_edit]

        # If the wrapper has a template and isn't already defined, also
        # insert the helper proc at the top of the file.
        template = _HELPER_PROC_TEMPLATES.get(wrapper)
        if template is not None and not _proc_defined_in_source(source, wrapper):
            insert_edit = types.TextEdit(
                range=types.Range(
                    start=types.Position(line=0, character=0),
                    end=types.Position(line=0, character=0),
                ),
                new_text=template,
            )
            edits.insert(0, insert_edit)

        actions.append(
            types.CodeAction(
                title=title,
                kind=types.CodeActionKind.QuickFix,
                diagnostics=[diag],
                edit=types.WorkspaceEdit(changes={"__current__": edits}),
            )
        )

    # T102: insert '--' before the variable
    if code == "T102":
        # Insert "-- " just before the variable reference.
        insert_pos = types.Range(
            start=types.Position(line=diag_line, character=col_start),
            end=types.Position(line=diag_line, character=col_start),
        )
        edit = types.TextEdit(range=insert_pos, new_text="-- ")
        actions.append(
            types.CodeAction(
                title=f"Insert '--' before ${var_name}",
                kind=types.CodeActionKind.QuickFix,
                diagnostics=[diag],
                edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
            )
        )

    # IRULE3004: no automatic fix (open redirect requires domain
    #     validation which can't be auto-generated)

    # T106: remove the redundant encoding wrapper
    if code == "T106":
        # Search for [encoder_name $var] or [encoder_name ${var}] in the line
        # and offer to replace with just $var.
        for encoder in _T106_ENCODERS:
            for vref in (f"${var_name}", f"${{{var_name}}}"):
                pattern = f"[{encoder} {vref}]"
                idx = line_text.find(pattern)
                if idx >= 0:
                    enc_range = types.Range(
                        start=types.Position(line=diag_line, character=idx),
                        end=types.Position(line=diag_line, character=idx + len(pattern)),
                    )
                    edit = types.TextEdit(range=enc_range, new_text=vref)
                    actions.append(
                        types.CodeAction(
                            title=f"Remove redundant {encoder}",
                            kind=types.CodeActionKind.QuickFix,
                            diagnostics=[diag],
                            edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
                        )
                    )
                    break
            else:
                continue
            break

    return actions


# Profile requirements header generation

_PROFILE_DIRECTIVE_RE = re.compile(r"profiles?\s*:\s*(.+)", re.IGNORECASE)


def _scan_profile_directive(source: str) -> tuple[frozenset[str], Token] | None:
    """Scan leading comment tokens for a ``# Profiles: HTTP, CLIENTSSL`` directive.

    Uses the Tcl lexer to locate the directive by token (like noqa) rather
    than regex over raw source.  Returns ``(profiles, comment_token)`` or
    ``None`` if no directive is found in the leading comments.
    """
    lexer = TclLexer(source)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL):
            continue
        if tok.type is not TokenType.COMMENT:
            break  # first non-comment content → stop scanning
        text = tok.text.lstrip("#").strip()
        m = _PROFILE_DIRECTIVE_RE.match(text)
        if m:
            profiles: set[str] = set()
            for part in re.split(r"[,\s]+", m.group(1)):
                if part:
                    profiles.add(part.upper())
            return frozenset(profiles), tok
    return None


def _compute_required_profiles(source: str, analysis: AnalysisResult) -> list[str]:
    """Compute required virtual-server profiles from events and commands.

    Returns a sorted list of canonical profile names.
    """
    profiles: set[str] = set()

    # From events
    for event_name in EVENT_REGISTRY.scan_file_events(source):
        props = EVENT_REGISTRY.get_props(event_name)
        if props is not None:
            profiles.update(props.implied_profiles)

    # From commands
    for invocation in analysis.command_invocations:
        spec = REGISTRY.get(invocation.name, "f5-irules")
        if spec is not None and spec.event_requires is not None and spec.event_requires.profiles:
            profiles.update(spec.event_requires.profiles)

    # Normalise: FASTHTTP is an alternative to HTTP; keep only HTTP when both appear.
    if "HTTP" in profiles and "FASTHTTP" in profiles:
        profiles.discard("FASTHTTP")

    return sorted(profiles)


def _generate_profiles_action(
    source: str,
    analysis: AnalysisResult,
) -> types.CodeAction | None:
    """Return a source action to insert or update the ``# Profiles:`` header.

    Returns ``None`` when the dialect is not iRules, when no profiles are
    needed, or when the existing directive already matches.
    """
    if not is_irules_dialect():
        return None

    required = _compute_required_profiles(source, analysis)
    if not required:
        return None

    required_set = frozenset(required)
    directive = _scan_profile_directive(source)
    new_text = f"# Profiles: {', '.join(required)}\n"

    if directive is not None:
        existing_profiles, tok = directive
        if existing_profiles == required_set:
            return None
        # Replace the existing directive line.
        edit = types.TextEdit(
            range=types.Range(
                start=types.Position(line=tok.start.line, character=0),
                end=types.Position(line=tok.start.line + 1, character=0),
            ),
            new_text=new_text,
        )
        title = f"Update profile requirements \u2192 {', '.join(required)}"
    else:
        # Insert at the top of the file.
        edit = types.TextEdit(
            range=types.Range(
                start=types.Position(line=0, character=0),
                end=types.Position(line=0, character=0),
            ),
            new_text=new_text,
        )
        title = f"Generate profile requirements header ({', '.join(required)})"

    return types.CodeAction(
        title=title,
        kind=types.CodeActionKind.Source,
        edit=types.WorkspaceEdit(changes={"__current__": [edit]}),
    )
