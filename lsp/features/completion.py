"""Completion provider -- $vars, commands, proc names, subcommands, switches."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef, Scope
from core.commands.registry import REGISTRY
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import (
    SIGNATURES,
    SubcommandSig,
)
from core.common.dialect import active_dialect
from core.formatting.config import FormatterConfig, IndentStyle

from .irules_context import find_enclosing_when_event
from .snippet_templates import SnippetContext, get_snippet_completions
from .symbol_resolution import (
    find_command_context_at_position,
    find_command_context_details_at_position,
    find_scope_at_line,
    find_variable_completion_prefix,
)


def _collect_vars_from_scope(scope: Scope) -> list[str]:
    """Collect variable names from scope and ancestors."""
    names: list[str] = []
    current: Scope | None = scope
    while current is not None:
        names.extend(current.variables.keys())
        current = current.parent
    return sorted(set(names))


def _usage_bucket(count: int) -> int:
    """Return a sort bucket (lower is better) for usage frequency."""
    if count >= 50:
        return 0
    if count >= 20:
        return 1
    if count >= 8:
        return 2
    if count >= 3:
        return 3
    if count >= 1:
        return 4
    return 5


def _event_bucket(cmd_name: str, dialect: str, current_event: str | None) -> int:
    """Return a sort bucket (lower is better) for event validity.

    Bucket 0: event-scoped valid (command has event metadata and passed).
    Bucket 1: event-neutral (no event requirements, always valid).
    Bucket 2: out-of-event (command has event metadata and failed).
    """
    if current_event is None or dialect != "f5-irules":
        return 1
    event_set = REGISTRY.commands_for_event(dialect, current_event)
    if cmd_name in event_set.out_of_event_commands:
        return 2
    if cmd_name in event_set.event_scoped_commands:
        return 0
    return 1


def _proc_usage_count(
    proc_name: str,
    *,
    qname: str | None,
    workspace_command_usage: dict[str, int],
    workspace_proc_usage: dict[str, int],
) -> int:
    """Best-effort usage count for proc ranking."""
    candidates = [
        workspace_command_usage.get(proc_name, 0),
        workspace_proc_usage.get(proc_name, 0),
    ]
    if qname:
        candidates.append(workspace_command_usage.get(qname, 0))
        candidates.append(workspace_proc_usage.get(qname, 0))
    return max(candidates)


def _builtin_sort_text(
    cmd_name: str,
    *,
    dialect: str,
    current_event: str | None,
    workspace_command_usage: dict[str, int],
) -> str:
    event_rank = _event_bucket(cmd_name, dialect, current_event)
    usage_rank = _usage_bucket(workspace_command_usage.get(cmd_name, 0))
    return f"B{event_rank}{usage_rank:02d}_{cmd_name}"


def _proc_sort_text(
    name: str,
    *,
    qname: str | None,
    tier: int,
    workspace_command_usage: dict[str, int],
    workspace_proc_usage: dict[str, int],
) -> str:
    usage_rank = _usage_bucket(
        _proc_usage_count(
            name,
            qname=qname,
            workspace_command_usage=workspace_command_usage,
            workspace_proc_usage=workspace_proc_usage,
        )
    )
    return f"A{tier}{usage_rank:02d}_{name}"


def _command_detail(cmd_name: str, required_package: str | None) -> str:
    """Return completion detail text showing command provenance.

    Shows ``tcllib (json)``, ``stdlib (http)``, ``Tk``, or ``built-in``.
    """
    if REGISTRY.is_tcllib_command(cmd_name):
        return f"tcllib ({REGISTRY.tcllib_package_for(cmd_name)})"
    if required_package is not None:
        if required_package == "Tk":
            return "Tk"
        return f"stdlib ({required_package})"
    return "built-in"


def _when_argument_values(
    source: str,
    line: int,
    character: int,
    *,
    dialect: str,
    arg_index: int,
) -> tuple:
    if arg_index < 0:
        return ()
    if arg_index == 0:
        return REGISTRY.argument_values("when", 0, dialect)
    if arg_index % 2 == 1:
        return REGISTRY.argument_values("when", 1, dialect)

    cmd, args, _current_word, _word_index = find_command_context_details_at_position(
        source, line, character
    )
    if cmd != "when":
        return ()
    prev_arg_index = arg_index - 1
    if prev_arg_index < len(args) and args[prev_arg_index] == "timing":
        return REGISTRY.argument_values("when", 2, dialect)
    return ()


def _get_trigger_context(
    source: str,
    line: int,
    character: int,
    dialect: str,
    *,
    lines: list[str] | None = None,
) -> tuple[str, str, str | None, int]:
    """Determine what triggered completion and the partial text.

    Returns (trigger, partial, command, arg_index):
        trigger: "$" for variable completion, "cmd" for command position,
                 "sub" for subcommand position, "switch" for switch,
                 "arg" for command-specific argument values
        partial: the text typed so far
        command: the command in the current command segment
        arg_index: argument index (0-based after command name), or -1 at command name
    """
    var_prefix = find_variable_completion_prefix(source, line, character, lines=lines)
    if var_prefix is not None:
        return ("$", var_prefix, None, -1)

    cmd, current_word, word_index = find_command_context_at_position(
        source, line, character, lines=lines
    )
    if cmd is None or word_index <= 0:
        return ("cmd", current_word, None, -1)

    arg_index = word_index - 1

    if current_word.startswith("-") and REGISTRY.switches(cmd, dialect):
        return ("switch", current_word, cmd, arg_index)

    sig = SIGNATURES.get(cmd)
    if isinstance(sig, SubcommandSig) and word_index == 1:
        return ("sub", current_word, cmd, arg_index)

    # iRules ``call proc_name`` — first argument is always a proc name.
    if cmd == "call" and word_index == 1 and dialect == "f5-irules":
        return ("call_target", current_word, cmd, arg_index)

    if REGISTRY.argument_values(cmd, arg_index, dialect):
        return ("arg", current_word, cmd, arg_index)

    # Subcommand-scoped argument values (e.g. "string is <class>").
    sig = SIGNATURES.get(cmd)
    if isinstance(sig, SubcommandSig) and word_index >= 2:
        _cmd_d, args_d, _, _ = find_command_context_details_at_position(source, line, character)
        if args_d:
            sub_arg_index = word_index - 2  # index within the subcommand
            sub_values = REGISTRY.subcommand_argument_values(cmd, args_d[0], sub_arg_index, dialect)
            if sub_values:
                return ("arg", current_word, cmd, arg_index)

    return ("cmd", current_word, cmd, arg_index)


def get_completions(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
    workspace_procs: list[str] | None = None,
    workspace_rule_init_vars: list[str] | None = None,
    workspace_command_usage: dict[str, int] | None = None,
    workspace_proc_usage: dict[str, int] | None = None,
    formatter_config: FormatterConfig | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.CompletionItem]:
    """Generate completion items for a position in source."""
    if analysis is None:
        analysis = analyse(source)

    command_usage = workspace_command_usage or {}
    proc_usage = workspace_proc_usage or {}

    dialect = active_dialect()
    active_packages = analysis.active_package_names()
    trigger, partial, context_cmd, context_arg_index = _get_trigger_context(
        source,
        line,
        character,
        dialect,
        lines=lines,
    )
    items: list[types.CompletionItem] = []

    if trigger == "$":
        # Variable completion
        scope = find_scope_at_line(analysis.global_scope, line)
        var_names = _collect_vars_from_scope(scope)
        for name in var_names:
            if partial and not name.startswith(partial.lstrip("{")):
                continue
            items.append(
                types.CompletionItem(
                    label=f"${name}",
                    kind=types.CompletionItemKind.Variable,
                    insert_text=name,
                )
            )
        # Cross-file RULE_INIT variables
        if workspace_rule_init_vars:
            existing = {item.label for item in items}
            for name in workspace_rule_init_vars:
                label = f"${name}"
                if label in existing:
                    continue
                if partial and not name.startswith(partial.lstrip("{")):
                    continue
                items.append(
                    types.CompletionItem(
                        label=label,
                        kind=types.CompletionItemKind.Variable,
                        insert_text=name,
                        detail="RULE_INIT (cross-file)",
                    )
                )

    elif trigger == "sub":
        # Subcommand completion
        if context_cmd:
            sig = SIGNATURES.get(context_cmd)
            if isinstance(sig, SubcommandSig):
                # Pre-fetch subcommand docs from arg_values[0] (subcommand position).
                sub_docs: dict[str, str] = {}
                cmd_spec = REGISTRY.get(context_cmd, dialect, active_packages=active_packages)
                if cmd_spec is not None:
                    for vs in cmd_spec.argument_values(0):
                        if vs.hover is not None:
                            sub_docs[vs.value] = vs.hover.summary
                for sub_name in sorted(sig.subcommands.keys()):
                    if partial and not sub_name.startswith(partial):
                        continue
                    items.append(
                        types.CompletionItem(
                            label=sub_name,
                            kind=types.CompletionItemKind.Function,
                            detail=f"{context_cmd} {sub_name}",
                            documentation=sub_docs.get(sub_name),
                        )
                    )

    elif trigger == "switch":
        # Switch/flag completion
        if context_cmd:
            cmd_spec = REGISTRY.get(context_cmd, dialect, active_packages=active_packages)
            for sw in REGISTRY.switches(context_cmd, dialect, active_packages=active_packages):
                if partial and not sw.startswith(partial):
                    continue
                doc: str | None = None
                if cmd_spec is not None:
                    opt = cmd_spec.option(sw)
                    if opt is not None:
                        doc = opt.detail or (opt.hover.summary if opt.hover else None)
                items.append(
                    types.CompletionItem(
                        label=sw,
                        kind=types.CompletionItemKind.Keyword,
                        documentation=doc,
                    )
                )

    elif trigger == "call_target":
        # iRules ``call`` — complete with proc names only.
        for qname, proc_def in analysis.all_procs.items():
            name = proc_def.name
            if partial and not name.startswith(partial):
                continue
            sig_str = _proc_signature_str(proc_def)
            items.append(
                types.CompletionItem(
                    label=name,
                    kind=types.CompletionItemKind.Function,
                    detail=sig_str,
                    documentation=proc_def.doc or None,
                    sort_text=f"0_{name}",
                )
            )
        if workspace_procs:
            existing = {item.label for item in items}
            for qname in workspace_procs:
                name = qname.rsplit("::", 1)[-1]
                if name in existing:
                    continue
                if partial and not name.startswith(partial):
                    continue
                items.append(
                    types.CompletionItem(
                        label=name,
                        kind=types.CompletionItemKind.Function,
                        detail=qname,
                        sort_text=f"1_{name}",
                    )
                )

    elif trigger == "arg":
        # Command-specific argument value completion.
        if context_cmd:
            value_specs = REGISTRY.argument_values(
                context_cmd, context_arg_index, dialect, active_packages=active_packages
            )
            if context_cmd == "when" and dialect == "f5-irules":
                value_specs = _when_argument_values(
                    source,
                    line,
                    character,
                    dialect=dialect,
                    arg_index=context_arg_index,
                )
            # Try subcommand-scoped values (e.g. "string is <class>").
            if not value_specs and context_arg_index >= 1:
                sig = SIGNATURES.get(context_cmd)
                if isinstance(sig, SubcommandSig):
                    _cmd_d, args_d, _, _ = find_command_context_details_at_position(
                        source,
                        line,
                        character,
                    )
                    if args_d:
                        sub_arg_index = context_arg_index - 1
                        value_specs = REGISTRY.subcommand_argument_values(
                            context_cmd,
                            args_d[0],
                            sub_arg_index,
                            dialect,
                            active_packages=active_packages,
                        )
            for value_spec in value_specs:
                if partial and not value_spec.value.startswith(partial):
                    continue
                items.append(
                    types.CompletionItem(
                        label=value_spec.value,
                        kind=types.CompletionItemKind.Keyword,
                        detail=value_spec.detail or context_cmd,
                        documentation=(value_spec.hover.summary if value_spec.hover else None),
                    )
                )

    else:
        # Command / proc name completion
        registry_commands = set(REGISTRY.command_names(dialect, active_packages=active_packages))
        known_registry_commands = REGISTRY.specs_by_name
        # Event-aware ranking: boost commands valid in the current event.
        current_event: str | None = None
        if dialect == "f5-irules":
            current_event, _ = find_enclosing_when_event(source, line)
        # Built-in commands — merge registry names (already package-filtered)
        # with SIGNATURES keys, but exclude SIGNATURES-only entries that are
        # package-gated and whose package is not active.
        all_sig_names = set(SIGNATURES.keys())
        if active_packages is not None:
            # Remove only those SIGNATURES entries that exist in the registry
            # with a required_package not in the active set.  Commands that
            # are *not* in the registry at all (e.g. tcllib commands added
            # only via SIGNATURES) pass through unfiltered.
            all_sig_names = {
                n
                for n in all_sig_names
                if n in registry_commands
                or n not in REGISTRY.specs_by_name
                or REGISTRY.get(n, dialect, active_packages=active_packages) is not None
            }
        for cmd_name in sorted(registry_commands.union(all_sig_names)):
            if partial and not cmd_name.startswith(partial):
                continue
            # Skip math operators from completions
            if cmd_name in ("+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="):
                continue
            cmd_spec = REGISTRY.get(cmd_name, dialect, active_packages=active_packages)
            if cmd_spec is None and cmd_name in known_registry_commands:
                continue
            cmd_doc = (
                cmd_spec.hover.summary
                if cmd_spec is not None and cmd_spec.hover is not None
                else None
            )
            req_pkg = cmd_spec.required_package if cmd_spec is not None else None
            items.append(
                types.CompletionItem(
                    label=cmd_name,
                    kind=types.CompletionItemKind.Function,
                    detail=_command_detail(cmd_name, req_pkg),
                    documentation=cmd_doc,
                    sort_text=_builtin_sort_text(
                        cmd_name,
                        dialect=dialect,
                        current_event=current_event,
                        workspace_command_usage=command_usage,
                    ),
                )
            )

        # User-defined procs from this file
        for qname, proc_def in analysis.all_procs.items():
            name = proc_def.name
            if partial and not name.startswith(partial):
                continue
            sig_str = _proc_signature_str(proc_def)
            items.append(
                types.CompletionItem(
                    label=name,
                    kind=types.CompletionItemKind.Function,
                    detail=sig_str,
                    documentation=proc_def.doc or None,
                    sort_text=_proc_sort_text(
                        name,
                        qname=qname,
                        tier=0,
                        workspace_command_usage=command_usage,
                        workspace_proc_usage=proc_usage,
                    ),
                )
            )

        # Workspace-wide procs
        if workspace_procs:
            existing = {item.label for item in items}
            for qname in workspace_procs:
                name = qname.rsplit("::", 1)[-1]
                if name in existing:
                    continue
                if partial and not name.startswith(partial):
                    continue
                items.append(
                    types.CompletionItem(
                        label=name,
                        kind=types.CompletionItemKind.Function,
                        detail=qname,
                        sort_text=_proc_sort_text(
                            name,
                            qname=qname,
                            tier=1,
                            workspace_command_usage=command_usage,
                            workspace_proc_usage=proc_usage,
                        ),
                    )
                )

        # Context-aware snippet templates
        if formatter_config is not None:
            scope = find_scope_at_line(analysis.global_scope, line)
            scope_vars = _collect_vars_from_scope(scope)
            snippet_ctx = SnippetContext(
                dialect=dialect,
                brace_style=formatter_config.brace_style,
                indent_unit=(
                    "\t"
                    if formatter_config.indent_style is IndentStyle.TABS
                    else " " * formatter_config.indent_size
                ),
                current_event=current_event,
                file_events=EVENT_REGISTRY.scan_file_events(source),
                scope_vars=scope_vars,
                partial=partial,
            )
            items.extend(get_snippet_completions(snippet_ctx))

    return items


def _proc_signature_str(proc_def: ProcDef) -> str:
    """Format a proc signature for display."""
    params = []
    for p in proc_def.params:
        if p.has_default:
            params.append(f"?{p.name}?")
        else:
            params.append(p.name)
    return f"proc {proc_def.name} {{{' '.join(params)}}}"
